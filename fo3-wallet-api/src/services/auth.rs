//! Authentication service implementation

use std::sync::Arc;
use tonic::{Request, Response, Status};
use uuid::Uuid;
use chrono::{DateTime, Utc, Duration};

use crate::proto::fo3::wallet::v1::{
    auth_service_server::AuthService,
    *,
};
use crate::middleware::{
    auth::{AuthService as AuthMiddleware, PasswordUtils, RateLimit as AuthRateLimit},
    audit::{AuditLogger, AuditEventType},
};

pub struct AuthServiceImpl {
    auth_service: Arc<AuthMiddleware>,
    audit_logger: Arc<AuditLogger>,
}

impl AuthServiceImpl {
    pub fn new(auth_service: Arc<AuthMiddleware>, audit_logger: Arc<AuditLogger>) -> Self {
        Self {
            auth_service,
            audit_logger,
        }
    }
}

#[tonic::async_trait]
impl AuthService for AuthServiceImpl {
    async fn login(
        &self,
        request: Request<LoginRequest>,
    ) -> Result<Response<LoginResponse>, Status> {
        let req = request.into_inner();
        let client_ip = request.metadata()
            .get("x-forwarded-for")
            .or_else(|| request.metadata().get("x-real-ip"))
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        // In a real implementation, validate credentials against database
        // For demo purposes, we'll use hardcoded credentials
        let valid_credentials = [
            ("admin", "admin123", UserRole::UserRoleAdmin),
            ("user", "user123", UserRole::UserRoleUser),
            ("viewer", "viewer123", UserRole::UserRoleViewer),
        ];

        let mut authenticated = false;
        let mut user_role = UserRole::UserRoleUser;

        for (username, password, role) in &valid_credentials {
            if req.username == *username && req.password == *password {
                authenticated = true;
                user_role = *role;
                break;
            }
        }

        if !authenticated {
            self.audit_logger.log_auth_event(
                &req.username,
                false,
                client_ip,
                Some("Invalid credentials"),
            ).await;

            return Err(Status::unauthenticated("Invalid username or password"));
        }

        // Generate user ID (in real implementation, this would come from database)
        let user_id = Uuid::new_v4().to_string();

        // Get permissions based on role
        let permissions = match user_role {
            UserRole::UserRoleSuperAdmin => vec![
                Permission::PermissionWalletRead,
                Permission::PermissionWalletWrite,
                Permission::PermissionTransactionRead,
                Permission::PermissionTransactionWrite,
                Permission::PermissionDefiRead,
                Permission::PermissionDefiWrite,
                Permission::PermissionSolanaRead,
                Permission::PermissionSolanaWrite,
                Permission::PermissionAdmin,
                Permission::PermissionKycSubmit,
                Permission::PermissionKycView,
                Permission::PermissionKycAdmin,
                Permission::PermissionFiatDeposit,
                Permission::PermissionFiatWithdraw,
                Permission::PermissionFiatAdmin,
                Permission::PermissionPricingRead,
                Permission::PermissionPricingAdmin,
                Permission::PermissionNotificationRead,
                Permission::PermissionNotificationAdmin,
            ],
            UserRole::UserRoleAdmin => vec![
                Permission::PermissionWalletRead,
                Permission::PermissionWalletWrite,
                Permission::PermissionTransactionRead,
                Permission::PermissionTransactionWrite,
                Permission::PermissionDefiRead,
                Permission::PermissionDefiWrite,
                Permission::PermissionSolanaRead,
                Permission::PermissionSolanaWrite,
                Permission::PermissionKycSubmit,
                Permission::PermissionKycView,
                Permission::PermissionKycAdmin,
                Permission::PermissionFiatDeposit,
                Permission::PermissionFiatWithdraw,
                Permission::PermissionFiatAdmin,
                Permission::PermissionPricingRead,
                Permission::PermissionPricingAdmin,
                Permission::PermissionNotificationRead,
                Permission::PermissionNotificationAdmin,
            ],
            UserRole::UserRoleUser => vec![
                Permission::PermissionWalletRead,
                Permission::PermissionWalletWrite,
                Permission::PermissionTransactionRead,
                Permission::PermissionTransactionWrite,
                Permission::PermissionDefiRead,
                Permission::PermissionDefiWrite,
                Permission::PermissionSolanaRead,
                Permission::PermissionSolanaWrite,
                Permission::PermissionKycSubmit,
                Permission::PermissionKycView,
                Permission::PermissionFiatDeposit,
                Permission::PermissionFiatWithdraw,
                Permission::PermissionPricingRead,
                Permission::PermissionNotificationRead,
            ],
            UserRole::UserRoleViewer => vec![
                Permission::PermissionWalletRead,
                Permission::PermissionTransactionRead,
                Permission::PermissionDefiRead,
                Permission::PermissionSolanaRead,
                Permission::PermissionKycView,
                Permission::PermissionPricingRead,
                Permission::PermissionNotificationRead,
            ],
            _ => vec![],
        };

        // Generate JWT token
        let access_token = self.auth_service.generate_jwt(
            &user_id,
            &req.username,
            user_role,
            permissions.clone(),
        )?;

        // Generate refresh token (simplified - in production, store in database)
        let refresh_token = Uuid::new_v4().to_string();

        // Calculate expiration
        let expires_at = (Utc::now() + Duration::hours(24)).timestamp();

        // Log successful authentication
        self.audit_logger.log_auth_event(
            &req.username,
            true,
            client_ip,
            None,
        ).await;

        let user = User {
            id: user_id,
            username: req.username.clone(),
            email: format!("{}@example.com", req.username), // Demo email
            role: user_role as i32,
            is_active: true,
            created_at: Utc::now().timestamp(),
            last_login: Utc::now().timestamp(),
            rate_limit: Some(RateLimit {
                requests_per_minute: 100,
                burst_limit: 20,
                daily_limit: 5000,
            }),
        };

        let response = LoginResponse {
            access_token,
            refresh_token,
            expires_at,
            user: Some(user),
        };

        Ok(Response::new(response))
    }

    async fn refresh_token(
        &self,
        request: Request<RefreshTokenRequest>,
    ) -> Result<Response<RefreshTokenResponse>, Status> {
        let req = request.into_inner();

        // In a real implementation, validate refresh token against database
        // For demo purposes, we'll generate new tokens
        let user_id = Uuid::new_v4().to_string();
        let username = "refreshed_user".to_string();
        let role = UserRole::UserRoleUser;
        let permissions = vec![
            Permission::PermissionWalletRead,
            Permission::PermissionWalletWrite,
        ];

        let access_token = self.auth_service.generate_jwt(
            &user_id,
            &username,
            role,
            permissions,
        )?;

        let new_refresh_token = Uuid::new_v4().to_string();
        let expires_at = (Utc::now() + Duration::hours(24)).timestamp();

        let response = RefreshTokenResponse {
            access_token,
            refresh_token: new_refresh_token,
            expires_at,
        };

        Ok(Response::new(response))
    }

    async fn logout(
        &self,
        request: Request<LogoutRequest>,
    ) -> Result<Response<LogoutResponse>, Status> {
        let _req = request.into_inner();

        // In a real implementation, invalidate the refresh token in database
        // For demo purposes, we'll just return success

        let response = LogoutResponse { success: true };
        Ok(Response::new(response))
    }

    async fn create_api_key(
        &self,
        request: Request<CreateApiKeyRequest>,
    ) -> Result<Response<CreateApiKeyResponse>, Status> {
        let req = request.into_inner();

        // Extract auth context (should be added by middleware)
        let auth_context = request.extensions().get::<crate::middleware::auth::AuthContext>()
            .ok_or_else(|| Status::unauthenticated("Authentication required"))?;

        // Check if user has permission to create API keys
        self.auth_service.check_permission(auth_context, Permission::PermissionAdmin)?;

        let rate_limit = AuthRateLimit {
            requests_per_minute: req.rate_limit.as_ref().map(|r| r.requests_per_minute).unwrap_or(100),
            burst_limit: req.rate_limit.as_ref().map(|r| r.burst_limit).unwrap_or(20),
            daily_limit: req.rate_limit.as_ref().map(|r| r.daily_limit).unwrap_or(5000),
        };

        let expires_at = if req.expires_at > 0 {
            Some(DateTime::from_timestamp(req.expires_at, 0).unwrap_or_else(|| Utc::now() + Duration::days(365)))
        } else {
            None
        };

        let permissions: Vec<crate::proto::fo3::wallet::v1::Permission> = req.permissions.into_iter()
            .filter_map(|p| crate::proto::fo3::wallet::v1::Permission::try_from(p).ok())
            .collect();

        let (key_prefix, secret_key) = self.auth_service.generate_api_key(
            &auth_context.user_id,
            &req.name,
            permissions.clone(),
            rate_limit.clone(),
            expires_at,
        ).await?;

        // Log API key creation
        self.audit_logger.log_wallet_event(
            auth_context,
            AuditEventType::ApiKeyCreated,
            &req.name,
            true,
        ).await;

        let api_key = ApiKey {
            id: Uuid::new_v4().to_string(),
            name: req.name,
            key_prefix,
            permissions: permissions.into_iter().map(|p| p as i32).collect(),
            rate_limit: Some(RateLimit {
                requests_per_minute: rate_limit.requests_per_minute,
                burst_limit: rate_limit.burst_limit,
                daily_limit: rate_limit.daily_limit,
            }),
            is_active: true,
            created_at: Utc::now().timestamp(),
            last_used: 0,
            expires_at: expires_at.map(|dt| dt.timestamp()).unwrap_or(0),
        };

        let response = CreateApiKeyResponse {
            api_key: Some(api_key),
            secret_key,
        };

        Ok(Response::new(response))
    }

    async fn list_api_keys(
        &self,
        request: Request<ListApiKeysRequest>,
    ) -> Result<Response<ListApiKeysResponse>, Status> {
        let _req = request.into_inner();

        // Extract auth context
        let _auth_context = request.extensions().get::<crate::middleware::auth::AuthContext>()
            .ok_or_else(|| Status::unauthenticated("Authentication required"))?;

        // In a real implementation, query database for user's API keys
        // For demo purposes, return empty list
        let response = ListApiKeysResponse {
            api_keys: vec![],
            next_page_token: String::new(),
        };

        Ok(Response::new(response))
    }

    async fn revoke_api_key(
        &self,
        request: Request<RevokeApiKeyRequest>,
    ) -> Result<Response<RevokeApiKeyResponse>, Status> {
        let req = request.into_inner();

        // Extract auth context
        let auth_context = request.extensions().get::<crate::middleware::auth::AuthContext>()
            .ok_or_else(|| Status::unauthenticated("Authentication required"))?;

        // Log API key revocation
        self.audit_logger.log_wallet_event(
            auth_context,
            AuditEventType::ApiKeyRevoked,
            &req.api_key_id,
            true,
        ).await;

        // In a real implementation, mark API key as inactive in database
        let response = RevokeApiKeyResponse { success: true };
        Ok(Response::new(response))
    }

    async fn rotate_api_key(
        &self,
        request: Request<RotateApiKeyRequest>,
    ) -> Result<Response<RotateApiKeyResponse>, Status> {
        let req = request.into_inner();

        // Extract auth context
        let auth_context = request.extensions().get::<crate::middleware::auth::AuthContext>()
            .ok_or_else(|| Status::unauthenticated("Authentication required"))?;

        // In a real implementation, generate new secret for existing API key
        let new_secret_key = format!("fo3_{}", Uuid::new_v4());

        let api_key = ApiKey {
            id: req.api_key_id,
            name: "Rotated Key".to_string(),
            key_prefix: new_secret_key[..12].to_string(),
            permissions: vec![],
            rate_limit: Some(RateLimit {
                requests_per_minute: 100,
                burst_limit: 20,
                daily_limit: 5000,
            }),
            is_active: true,
            created_at: Utc::now().timestamp(),
            last_used: 0,
            expires_at: 0,
        };

        let response = RotateApiKeyResponse {
            new_secret_key,
            api_key: Some(api_key),
        };

        Ok(Response::new(response))
    }
}
