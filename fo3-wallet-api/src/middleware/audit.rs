//! Audit logging middleware

use std::sync::Arc;
use std::time::Instant;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;
use tonic::{Request, Response, Status};

use crate::middleware::auth::AuthContext;
use crate::models::pii_protection::{PiiAnonymizer, PiiClassification, ComplianceAuditEntry};

/// Audit event types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditEventType {
    Authentication,
    Authorization,
    WalletCreated,
    WalletDeleted,
    TransactionSigned,
    TransactionBroadcast,
    ApiKeyCreated,
    ApiKeyRevoked,
    PermissionDenied,
    RateLimitExceeded,
    SecurityViolation,
    // KYC audit events
    KycSubmitted,
    KycApproved,
    KycRejected,
    KycDocumentUploaded,
    KycDocumentDeleted,
    KycStatusViewed,
    KycDataUpdated,
    // Fiat Gateway audit events
    FiatAccountBound,
    FiatAccountRemoved,
    FiatWithdrawalSubmitted,
    FiatWithdrawalApproved,
    FiatWithdrawalRejected,
    FiatWithdrawalCancelled,
    FiatDepositInitiated,
    FiatDepositCompleted,
    FiatLimitsUpdated,
    FiatSuspiciousActivity,
}

/// Audit log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogEntry {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub event_type: AuditEventType,
    pub user_id: Option<String>,
    pub username: Option<String>,
    pub method: String,
    pub resource: Option<String>,
    pub client_ip: Option<String>,
    pub user_agent: Option<String>,
    pub success: bool,
    pub error_message: Option<String>,
    pub request_id: String,
    pub duration_ms: Option<u64>,
    pub metadata: serde_json::Value,
}

impl AuditLogEntry {
    pub fn new(
        event_type: AuditEventType,
        method: String,
        success: bool,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            event_type,
            user_id: None,
            username: None,
            method,
            resource: None,
            client_ip: None,
            user_agent: None,
            success,
            error_message: None,
            request_id: Uuid::new_v4().to_string(),
            duration_ms: None,
            metadata: serde_json::Value::Object(serde_json::Map::new()),
        }
    }

    pub fn with_auth(mut self, auth: &AuthContext) -> Self {
        self.user_id = Some(auth.user_id.clone());
        self.username = Some(auth.username.clone());
        self
    }

    pub fn with_client_info(mut self, ip: Option<String>, user_agent: Option<String>) -> Self {
        self.client_ip = ip;
        self.user_agent = user_agent;
        self
    }

    pub fn with_error(mut self, error: &str) -> Self {
        self.error_message = Some(error.to_string());
        self.success = false;
        self
    }

    pub fn with_duration(mut self, duration: u64) -> Self {
        self.duration_ms = Some(duration);
        self
    }

    pub fn with_resource(mut self, resource: String) -> Self {
        self.resource = Some(resource);
        self
    }

    pub fn with_metadata(mut self, key: &str, value: serde_json::Value) -> Self {
        if let serde_json::Value::Object(ref mut map) = self.metadata {
            map.insert(key.to_string(), value);
        }
        self
    }
}

/// Audit logger service
pub struct AuditLogger {
    // In production, this would write to a database or log aggregation service
    _state: Arc<crate::state::AppState>,
}

impl AuditLogger {
    pub fn new(state: Arc<crate::state::AppState>) -> Self {
        Self { _state: state }
    }

    /// Log an audit event
    pub async fn log(&self, entry: AuditLogEntry) {
        // In production, this would:
        // 1. Write to database
        // 2. Send to log aggregation service (ELK, Splunk, etc.)
        // 3. Send alerts for security events
        
        // For now, log to structured logger
        match entry.event_type {
            AuditEventType::SecurityViolation | 
            AuditEventType::PermissionDenied |
            AuditEventType::RateLimitExceeded => {
                tracing::warn!(
                    audit_event = ?entry.event_type,
                    user_id = entry.user_id,
                    username = entry.username,
                    method = entry.method,
                    client_ip = entry.client_ip,
                    error = entry.error_message,
                    "Security audit event"
                );
            }
            _ => {
                tracing::info!(
                    audit_event = ?entry.event_type,
                    user_id = entry.user_id,
                    username = entry.username,
                    method = entry.method,
                    success = entry.success,
                    duration_ms = entry.duration_ms,
                    "Audit event"
                );
            }
        }

        // TODO: Store in database
        // self.store_in_database(&entry).await;
    }

    /// Log authentication event
    pub async fn log_auth_event(&self, username: &str, success: bool, client_ip: Option<String>, error: Option<&str>) {
        let mut entry = AuditLogEntry::new(
            AuditEventType::Authentication,
            "authenticate".to_string(),
            success,
        );

        entry.username = Some(username.to_string());
        entry.client_ip = client_ip;

        if let Some(err) = error {
            entry = entry.with_error(err);
        }

        self.log(entry).await;
    }

    /// Log authorization event
    pub async fn log_authz_event(&self, auth: &AuthContext, method: &str, resource: Option<String>, success: bool, error: Option<&str>) {
        let mut entry = AuditLogEntry::new(
            AuditEventType::Authorization,
            method.to_string(),
            success,
        )
        .with_auth(auth);

        if let Some(res) = resource {
            entry = entry.with_resource(res);
        }

        if let Some(err) = error {
            entry = entry.with_error(err);
        }

        self.log(entry).await;
    }

    /// Log wallet operation
    pub async fn log_wallet_event(&self, auth: &AuthContext, event_type: AuditEventType, wallet_id: &str, success: bool) {
        let entry = AuditLogEntry::new(
            event_type,
            "wallet_operation".to_string(),
            success,
        )
        .with_auth(auth)
        .with_resource(wallet_id.to_string());

        self.log(entry).await;
    }

    /// Log transaction event
    pub async fn log_transaction_event(&self, auth: &AuthContext, event_type: AuditEventType, tx_hash: &str, success: bool) {
        let entry = AuditLogEntry::new(
            event_type,
            "transaction_operation".to_string(),
            success,
        )
        .with_auth(auth)
        .with_resource(tx_hash.to_string());

        self.log(entry).await;
    }

    /// Log security violation
    pub async fn log_security_violation(&self, description: &str, client_ip: Option<String>, user_id: Option<String>) {
        let mut entry = AuditLogEntry::new(
            AuditEventType::SecurityViolation,
            "security_check".to_string(),
            false,
        )
        .with_error(description);

        entry.client_ip = client_ip;
        entry.user_id = user_id;

        self.log(entry).await;
    }

    /// Log KYC submission event
    pub async fn log_kyc_submission(&self, auth: &AuthContext, kyc_id: &str, success: bool) {
        let entry = AuditLogEntry::new(
            AuditEventType::KycSubmitted,
            "kyc_submit".to_string(),
            success,
        )
        .with_auth(auth)
        .with_resource(kyc_id.to_string());

        self.log(entry).await;
    }

    /// Log KYC approval event
    pub async fn log_kyc_approval(&self, auth: &AuthContext, kyc_id: &str, reviewer_notes: Option<&str>) {
        let mut entry = AuditLogEntry::new(
            AuditEventType::KycApproved,
            "kyc_approve".to_string(),
            true,
        )
        .with_auth(auth)
        .with_resource(kyc_id.to_string());

        if let Some(notes) = reviewer_notes {
            entry.metadata = serde_json::json!({ "reviewer_notes": notes });
        }

        self.log(entry).await;
    }

    /// Log KYC rejection event
    pub async fn log_kyc_rejection(&self, auth: &AuthContext, kyc_id: &str, reason: &str) {
        let entry = AuditLogEntry::new(
            AuditEventType::KycRejected,
            "kyc_reject".to_string(),
            true,
        )
        .with_auth(auth)
        .with_resource(kyc_id.to_string())
        .with_error(reason);

        self.log(entry).await;
    }

    /// Log KYC document upload event
    pub async fn log_kyc_document_upload(&self, auth: &AuthContext, kyc_id: &str, document_type: &str, success: bool) {
        let entry = AuditLogEntry::new(
            AuditEventType::KycDocumentUploaded,
            "kyc_document_upload".to_string(),
            success,
        )
        .with_auth(auth)
        .with_resource(kyc_id.to_string());

        let mut entry = entry;
        entry.metadata = serde_json::json!({ "document_type": document_type });

        self.log(entry).await;
    }

    /// Log KYC status view event
    pub async fn log_kyc_status_view(&self, auth: &AuthContext, kyc_id: &str) {
        let entry = AuditLogEntry::new(
            AuditEventType::KycStatusViewed,
            "kyc_status_view".to_string(),
            true,
        )
        .with_auth(auth)
        .with_resource(kyc_id.to_string());

        self.log(entry).await;
    }

    /// Log compliance audit entry with PII protection
    pub async fn log_compliance_event(&self, compliance_entry: ComplianceAuditEntry) {
        let mut audit_entry = AuditLogEntry::new(
            AuditEventType::KycDataUpdated,
            compliance_entry.operation.clone(),
            true,
        );

        // Mask sensitive data in user ID
        audit_entry.user_id = Some(PiiAnonymizer::mask_sensitive_data(
            &compliance_entry.user_id,
            PiiClassification::Internal,
        ));

        audit_entry.metadata = serde_json::json!({
            "compliance_id": compliance_entry.id.to_string(),
            "data_type": compliance_entry.data_type,
            "legal_basis": compliance_entry.legal_basis,
            "purpose": compliance_entry.purpose,
            "retention_period_days": compliance_entry.retention_period,
            "processor_id": PiiAnonymizer::mask_sensitive_data(
                &compliance_entry.processor_id,
                PiiClassification::Internal,
            ),
        });

        self.log(audit_entry).await;
    }

    /// Log data retention event
    pub async fn log_data_retention_event(&self, user_id: &str, data_type: &str, action: &str, success: bool) {
        let mut entry = AuditLogEntry::new(
            AuditEventType::SecurityViolation, // Reusing for data retention
            format!("data_retention_{}", action).to_string(),
            success,
        );

        entry.user_id = Some(PiiAnonymizer::mask_sensitive_data(user_id, PiiClassification::Internal));
        entry.metadata = serde_json::json!({
            "data_type": data_type,
            "retention_action": action,
            "compliance_requirement": "GDPR/CCPA data retention policy",
        });

        self.log(entry).await;
    }

    /// Log PII access event with enhanced tracking
    pub async fn log_pii_access(&self, auth: &AuthContext, data_type: &str, purpose: &str, legal_basis: &str) {
        let mut entry = AuditLogEntry::new(
            AuditEventType::KycStatusViewed, // Reusing for PII access
            "pii_access".to_string(),
            true,
        )
        .with_auth(auth);

        entry.metadata = serde_json::json!({
            "data_type": data_type,
            "access_purpose": purpose,
            "legal_basis": legal_basis,
            "gdpr_article": self.determine_gdpr_article(legal_basis),
        });

        self.log(entry).await;
    }

    /// Determine GDPR article based on legal basis
    fn determine_gdpr_article(&self, legal_basis: &str) -> String {
        match legal_basis.to_lowercase().as_str() {
            s if s.contains("consent") => "Article 6(1)(a) - Consent".to_string(),
            s if s.contains("contract") => "Article 6(1)(b) - Contract".to_string(),
            s if s.contains("legal obligation") => "Article 6(1)(c) - Legal obligation".to_string(),
            s if s.contains("vital interests") => "Article 6(1)(d) - Vital interests".to_string(),
            s if s.contains("public task") => "Article 6(1)(e) - Public task".to_string(),
            s if s.contains("legitimate interest") => "Article 6(1)(f) - Legitimate interests".to_string(),
            _ => "Not specified".to_string(),
        }
    }

    /// Log fiat account binding event
    pub async fn log_fiat_account_bound(&self, auth: &AuthContext, account_id: &str) {
        let entry = AuditLogEntry::new(
            AuditEventType::FiatAccountBound,
            "bind_bank_account".to_string(),
            true,
        )
        .with_auth(auth)
        .with_resource(account_id.to_string());

        self.log(entry).await;
    }

    /// Log fiat withdrawal submission
    pub async fn log_fiat_withdrawal_submitted(&self, auth: &AuthContext, transaction_id: &str, amount: &str, currency: &str) {
        let mut entry = AuditLogEntry::new(
            AuditEventType::FiatWithdrawalSubmitted,
            "submit_withdrawal".to_string(),
            true,
        )
        .with_auth(auth)
        .with_resource(transaction_id.to_string());

        entry.metadata = serde_json::json!({
            "amount": amount,
            "currency": currency,
            "transaction_type": "withdrawal"
        });

        self.log(entry).await;
    }

    /// Log fiat withdrawal approval
    pub async fn log_fiat_withdrawal_approved(&self, auth: &AuthContext, transaction_id: &str, notes: Option<&str>) {
        let mut entry = AuditLogEntry::new(
            AuditEventType::FiatWithdrawalApproved,
            "approve_withdrawal".to_string(),
            true,
        )
        .with_auth(auth)
        .with_resource(transaction_id.to_string());

        if let Some(notes) = notes {
            entry.metadata = serde_json::json!({
                "approval_notes": notes
            });
        }

        self.log(entry).await;
    }

    /// Log fiat withdrawal rejection
    pub async fn log_fiat_withdrawal_rejected(&self, auth: &AuthContext, transaction_id: &str, reason: &str) {
        let mut entry = AuditLogEntry::new(
            AuditEventType::FiatWithdrawalRejected,
            "reject_withdrawal".to_string(),
            true,
        )
        .with_auth(auth)
        .with_resource(transaction_id.to_string());

        entry.metadata = serde_json::json!({
            "rejection_reason": reason
        });

        self.log(entry).await;
    }

    /// Log suspicious fiat activity
    pub async fn log_fiat_suspicious_activity(&self, auth: &AuthContext, transaction_id: &str, reason: &str, risk_score: f64) {
        let mut entry = AuditLogEntry::new(
            AuditEventType::FiatSuspiciousActivity,
            "suspicious_activity_detected".to_string(),
            true,
        )
        .with_auth(auth)
        .with_resource(transaction_id.to_string());

        entry.metadata = serde_json::json!({
            "reason": reason,
            "risk_score": risk_score,
            "requires_manual_review": true
        });

        self.log(entry).await;
    }
}

/// Audit interceptor for gRPC services
pub struct AuditInterceptor {
    audit_logger: Arc<AuditLogger>,
}

impl AuditInterceptor {
    pub fn new(audit_logger: Arc<AuditLogger>) -> Self {
        Self { audit_logger }
    }

    pub async fn intercept<T>(&self, request: Request<T>) -> Result<Request<T>, Status> {
        let start_time = Instant::now();
        let method = request.uri().path().to_string();
        
        // Extract client info
        let client_ip = request
            .metadata()
            .get("x-forwarded-for")
            .or_else(|| request.metadata().get("x-real-ip"))
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        let user_agent = request
            .metadata()
            .get("user-agent")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        // Add request timing and client info to extensions
        let mut request = request;
        request.extensions_mut().insert(start_time);
        request.extensions_mut().insert(AuditContext {
            method: method.clone(),
            client_ip,
            user_agent,
            audit_logger: self.audit_logger.clone(),
        });

        Ok(request)
    }
}

/// Audit context stored in request extensions
#[derive(Clone)]
pub struct AuditContext {
    pub method: String,
    pub client_ip: Option<String>,
    pub user_agent: Option<String>,
    pub audit_logger: Arc<AuditLogger>,
}

/// Helper function to log request completion
pub async fn log_request_completion<T>(
    request: &Request<T>,
    response: &Result<Response<impl std::fmt::Debug>, Status>,
) {
    if let (Some(start_time), Some(audit_ctx), auth_opt) = (
        request.extensions().get::<Instant>(),
        request.extensions().get::<AuditContext>(),
        request.extensions().get::<AuthContext>(),
    ) {
        let duration = start_time.elapsed().as_millis() as u64;
        let success = response.is_ok();

        let mut entry = AuditLogEntry::new(
            AuditEventType::Authorization, // Generic operation
            audit_ctx.method.clone(),
            success,
        )
        .with_duration(duration)
        .with_client_info(audit_ctx.client_ip.clone(), audit_ctx.user_agent.clone());

        if let Some(auth) = auth_opt {
            entry = entry.with_auth(auth);
        }

        if let Err(status) = response {
            entry = entry.with_error(&status.message());
        }

        audit_ctx.audit_logger.log(entry).await;
    }
}

/// Macro to easily add audit logging to service methods
#[macro_export]
macro_rules! audit_service_call {
    ($request:expr, $response:expr) => {
        crate::middleware::audit::log_request_completion(&$request, &$response).await;
    };
}
