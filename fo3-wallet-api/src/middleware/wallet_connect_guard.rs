//! WalletConnect security guard middleware

use std::sync::Arc;
use tonic::{Request, Status};
use uuid::Uuid;
use chrono::{DateTime, Utc, Duration};

use crate::middleware::{
    auth::{AuthContext, AuthService},
    audit::AuditLogger,
    rate_limit::RateLimiter,
};
use crate::models::wallet_connect::{
    WalletConnectRepository, SessionStatus, RequestType, RequestStatus, KeyType,
};
use crate::proto::fo3::wallet::v1::{Permission, UserRole};

/// WalletConnect security guard for validation and fraud prevention
#[derive(Debug)]
pub struct WalletConnectGuard {
    auth_service: Arc<AuthService>,
    audit_logger: Arc<AuditLogger>,
    rate_limiter: Arc<RateLimiter>,
    wallet_connect_repository: Arc<dyn WalletConnectRepository>,
}

impl WalletConnectGuard {
    pub fn new(
        auth_service: Arc<AuthService>,
        audit_logger: Arc<AuditLogger>,
        rate_limiter: Arc<RateLimiter>,
        wallet_connect_repository: Arc<dyn WalletConnectRepository>,
    ) -> Self {
        Self {
            auth_service,
            audit_logger,
            rate_limiter,
            wallet_connect_repository,
        }
    }

    /// Validate session creation request
    pub async fn validate_session_creation<T>(
        &self,
        request: &Request<T>,
        dapp_url: &str,
        dapp_name: &str,
        supported_chains: &[KeyType],
    ) -> Result<AuthContext, Status> {
        // Extract and validate authentication
        let auth_context = self.auth_service.extract_auth(request).await?;

        // Rate limiting for session creation
        let rate_limit_key = format!("session_creation:{}", auth_context.user_id);
        if !self.rate_limiter.check_rate_limit(&rate_limit_key, 10, Duration::hours(1)).await? {
            return Err(Status::resource_exhausted("Rate limit exceeded for session creation"));
        }

        // Validate DApp URL
        self.validate_dapp_url(dapp_url)?;

        // Validate DApp name
        if dapp_name.is_empty() || dapp_name.len() > 100 {
            return Err(Status::invalid_argument("DApp name must be between 1 and 100 characters"));
        }

        // Validate supported chains
        if supported_chains.is_empty() {
            return Err(Status::invalid_argument("At least one supported chain must be specified"));
        }

        // Check for suspicious patterns
        self.check_session_creation_patterns(&auth_context.user_id, dapp_url).await?;

        // Log session creation attempt
        self.audit_logger.log_action(
            &auth_context.user_id.to_string(),
            "session_creation_attempt",
            &format!("DApp: {}, Chains: {:?}", dapp_name, supported_chains),
            true,
            None,
        ).await;

        Ok(auth_context)
    }

    /// Validate session access
    pub async fn validate_session_access<T>(
        &self,
        request: &Request<T>,
        session_id: &Uuid,
    ) -> Result<AuthContext, Status> {
        // Extract authentication
        let auth_context = self.auth_service.extract_auth(request).await?;

        // Get session
        let session = self.wallet_connect_repository.get_session(session_id).await
            .map_err(|e| Status::internal(format!("Failed to get session: {}", e)))?
            .ok_or_else(|| Status::not_found("Session not found"))?;

        // Check ownership or admin permissions
        if session.user_id != auth_context.user_id && 
           !self.auth_service.has_permission(&auth_context, Permission::ManageWalletConnect).await? {
            return Err(Status::permission_denied("Session does not belong to user"));
        }

        // Check session status
        if session.status == SessionStatus::Suspended {
            return Err(Status::failed_precondition("Session is suspended"));
        }

        if session.is_expired() {
            return Err(Status::failed_precondition("Session has expired"));
        }

        Ok(auth_context)
    }

    /// Validate session request handling
    pub async fn validate_session_request<T>(
        &self,
        request: &Request<T>,
        session_id: &Uuid,
        request_type: RequestType,
        method: &str,
        params: &str,
    ) -> Result<AuthContext, Status> {
        // Validate session access first
        let auth_context = self.validate_session_access(request, session_id).await?;

        // Rate limiting for session requests
        let rate_limit_key = format!("session_requests:{}:{}", session_id, auth_context.user_id);
        let (rate_limit, duration) = match request_type {
            RequestType::SignMessage => (20, Duration::minutes(10)), // 20 message signs per 10 minutes
            RequestType::SignTransaction => (10, Duration::minutes(10)), // 10 transaction signs per 10 minutes
            RequestType::SendTransaction => (5, Duration::minutes(10)), // 5 sends per 10 minutes
            _ => (30, Duration::minutes(10)), // 30 other requests per 10 minutes
        };

        if !self.rate_limiter.check_rate_limit(&rate_limit_key, rate_limit, duration).await? {
            return Err(Status::resource_exhausted("Rate limit exceeded for session requests"));
        }

        // Validate method and params
        self.validate_request_method_and_params(request_type, method, params)?;

        // Check for suspicious request patterns
        self.check_request_patterns(&auth_context.user_id, session_id, request_type).await?;

        // Log request attempt
        self.audit_logger.log_action(
            &auth_context.user_id.to_string(),
            "session_request_attempt",
            &format!("Session: {}, Type: {:?}, Method: {}", session_id, request_type, method),
            true,
            None,
        ).await;

        Ok(auth_context)
    }

    /// Validate session analytics access
    pub async fn validate_analytics_access<T>(
        &self,
        request: &Request<T>,
        target_user_id: Option<Uuid>,
    ) -> Result<AuthContext, Status> {
        // Extract authentication
        let auth_context = self.auth_service.extract_auth(request).await?;

        // Check if user can access analytics for themselves or has admin permissions
        if let Some(target_id) = target_user_id {
            if target_id != auth_context.user_id && 
               !self.auth_service.has_permission(&auth_context, Permission::ViewAnalytics).await? {
                return Err(Status::permission_denied("Can only view your own analytics"));
            }
        }

        // Rate limiting for analytics access
        let rate_limit_key = format!("analytics_access:{}", auth_context.user_id);
        if !self.rate_limiter.check_rate_limit(&rate_limit_key, 50, Duration::hours(1)).await? {
            return Err(Status::resource_exhausted("Rate limit exceeded for analytics access"));
        }

        Ok(auth_context)
    }

    /// Validate administrative access
    pub async fn validate_administrative_access<T>(
        &self,
        request: &Request<T>,
        operation_type: &str,
    ) -> Result<AuthContext, Status> {
        // Extract authentication
        let auth_context = self.auth_service.extract_auth(request).await?;

        // Check admin permissions
        self.auth_service.check_permission(&auth_context, Permission::ManageWalletConnect)?;

        // Rate limiting based on operation type
        let (rate_limit, duration) = match operation_type {
            "flag_suspicious_session" => (20, Duration::hours(1)), // 20 flags per hour
            "bulk_operations" => (5, Duration::hours(1)), // 5 bulk operations per hour
            "system_analytics" => (30, Duration::hours(1)), // 30 system analytics per hour
            _ => (10, Duration::hours(1)), // Default rate limit
        };

        let rate_limit_key = format!("admin_{}:{}", operation_type, auth_context.user_id);
        if !self.rate_limiter.check_rate_limit(&rate_limit_key, rate_limit, duration).await? {
            return Err(Status::resource_exhausted("Rate limit exceeded for administrative operations"));
        }

        // Log administrative action
        self.audit_logger.log_action(
            &auth_context.user_id.to_string(),
            "admin_operation",
            &format!("Operation: {}", operation_type),
            true,
            None,
        ).await;

        Ok(auth_context)
    }

    /// Validate DApp URL format and security
    fn validate_dapp_url(&self, url: &str) -> Result<(), Status> {
        if url.is_empty() {
            return Err(Status::invalid_argument("DApp URL cannot be empty"));
        }

        // Basic URL validation
        if !url.starts_with("https://") && !url.starts_with("http://") {
            return Err(Status::invalid_argument("DApp URL must be a valid HTTP/HTTPS URL"));
        }

        // Check for suspicious domains
        let suspicious_domains = [
            "localhost", "127.0.0.1", "0.0.0.0", "192.168.", "10.0.", "172.16.",
            "phishing", "scam", "fake", "malicious"
        ];

        let url_lower = url.to_lowercase();
        if suspicious_domains.iter().any(|domain| url_lower.contains(domain)) {
            return Err(Status::invalid_argument("DApp URL appears to be suspicious"));
        }

        Ok(())
    }

    /// Validate request method and parameters
    fn validate_request_method_and_params(
        &self,
        request_type: RequestType,
        method: &str,
        params: &str,
    ) -> Result<(), Status> {
        // Validate method name
        if method.is_empty() || method.len() > 100 {
            return Err(Status::invalid_argument("Method name must be between 1 and 100 characters"));
        }

        // Validate params size (prevent DoS)
        if params.len() > 10_000 {
            return Err(Status::invalid_argument("Request parameters too large"));
        }

        // Basic JSON validation for params
        if !params.is_empty() {
            serde_json::from_str::<serde_json::Value>(params)
                .map_err(|_| Status::invalid_argument("Invalid JSON in request parameters"))?;
        }

        // Validate method against request type
        match request_type {
            RequestType::SignMessage => {
                if !method.starts_with("personal_sign") && !method.starts_with("eth_sign") {
                    return Err(Status::invalid_argument("Invalid method for message signing"));
                }
            }
            RequestType::SignTransaction => {
                if !method.starts_with("eth_sendTransaction") && !method.starts_with("eth_signTransaction") {
                    return Err(Status::invalid_argument("Invalid method for transaction signing"));
                }
            }
            _ => {} // Other types are more flexible
        }

        Ok(())
    }

    /// Check for suspicious session creation patterns
    async fn check_session_creation_patterns(&self, user_id: &Uuid, dapp_url: &str) -> Result<(), Status> {
        let end_date = Utc::now();
        let start_date = end_date - Duration::hours(1);

        // Get recent sessions for this user
        if let Ok((sessions, _)) = self.wallet_connect_repository.list_sessions(
            Some(*user_id),
            None,
            None,
            None,
            Some(start_date),
            Some(end_date),
            1,
            100,
        ).await {
            // Check for too many sessions in short time
            if sessions.len() > 10 {
                return Err(Status::resource_exhausted("Too many sessions created in the last hour"));
            }

            // Check for repeated attempts with same DApp
            let same_dapp_count = sessions.iter()
                .filter(|s| s.dapp_url == dapp_url)
                .count();

            if same_dapp_count > 3 {
                return Err(Status::resource_exhausted("Too many sessions with the same DApp"));
            }
        }

        Ok(())
    }

    /// Check for suspicious request patterns
    async fn check_request_patterns(
        &self,
        user_id: &Uuid,
        session_id: &Uuid,
        request_type: RequestType,
    ) -> Result<(), Status> {
        let end_date = Utc::now();
        let start_date = end_date - Duration::minutes(5);

        // Get recent requests for this session
        if let Ok((requests, _)) = self.wallet_connect_repository.list_requests(
            Some(*session_id),
            Some(*user_id),
            None,
            Some(request_type),
            1,
            100,
        ).await {
            // Check for rapid-fire requests
            let recent_requests = requests.iter()
                .filter(|r| r.created_at >= start_date)
                .count();

            let max_requests = match request_type {
                RequestType::SignTransaction | RequestType::SendTransaction => 3, // Max 3 transactions in 5 minutes
                RequestType::SignMessage => 10, // Max 10 message signs in 5 minutes
                _ => 20, // Max 20 other requests in 5 minutes
            };

            if recent_requests > max_requests {
                return Err(Status::resource_exhausted("Too many requests in short time period"));
            }
        }

        Ok(())
    }
}
