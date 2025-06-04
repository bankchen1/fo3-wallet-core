//! DApp signing security guard middleware

use std::sync::Arc;
use tonic::{Request, Status};
use uuid::Uuid;
use rust_decimal::Decimal;
use chrono::{DateTime, Utc, Duration};

use crate::middleware::{
    auth::{AuthContext, AuthService},
    audit::AuditLogger,
    rate_limit::RateLimiter,
};
use crate::models::dapp_signing::{
    DAppSigningRepository, SignatureType, TransactionType, KeyType, RiskLevel,
};
use crate::proto::fo3::wallet::v1::{Permission, UserRole};

/// DApp signing security guard for validation and fraud prevention
#[derive(Debug)]
pub struct DAppGuard {
    auth_service: Arc<AuthService>,
    audit_logger: Arc<AuditLogger>,
    rate_limiter: Arc<RateLimiter>,
    dapp_signing_repository: Arc<dyn DAppSigningRepository>,
}

impl DAppGuard {
    pub fn new(
        auth_service: Arc<AuthService>,
        audit_logger: Arc<AuditLogger>,
        rate_limiter: Arc<RateLimiter>,
        dapp_signing_repository: Arc<dyn DAppSigningRepository>,
    ) -> Self {
        Self {
            auth_service,
            audit_logger,
            rate_limiter,
            dapp_signing_repository,
        }
    }

    /// Validate message signing request
    pub async fn validate_message_signing<T>(
        &self,
        request: &Request<T>,
        session_id: &Uuid,
        address: &str,
        message: &str,
        signature_type: SignatureType,
        key_type: KeyType,
    ) -> Result<AuthContext, Status> {
        // Extract and validate authentication
        let auth_context = self.auth_service.extract_auth(request).await?;

        // Rate limiting for message signing
        let rate_limit_key = format!("message_signing:{}:{}", auth_context.user_id, session_id);
        if !self.rate_limiter.check_rate_limit(&rate_limit_key, 20, Duration::minutes(10)).await? {
            return Err(Status::resource_exhausted("Rate limit exceeded for message signing"));
        }

        // Validate message content
        self.validate_message_content(message)?;

        // Validate address format
        self.validate_address_format(address, key_type)?;

        // Check signing permissions
        self.validate_signing_permissions(&auth_context, address).await?;

        // Check for suspicious signing patterns
        self.check_signing_patterns(&auth_context.user_id, session_id, None).await?;

        // Log signing attempt
        self.audit_logger.log_action(
            &auth_context.user_id.to_string(),
            "message_signing_attempt",
            &format!("Session: {}, Address: {}, Type: {:?}", session_id, address, signature_type),
            true,
            None,
        ).await;

        Ok(auth_context)
    }

    /// Validate transaction signing request
    pub async fn validate_transaction_signing<T>(
        &self,
        request: &Request<T>,
        session_id: &Uuid,
        from_address: &str,
        to_address: &str,
        amount: &Decimal,
        key_type: KeyType,
        chain_id: &str,
        transaction_type: TransactionType,
    ) -> Result<AuthContext, Status> {
        // Extract and validate authentication
        let auth_context = self.auth_service.extract_auth(request).await?;

        // Rate limiting for transaction signing
        let rate_limit_key = format!("transaction_signing:{}:{}", auth_context.user_id, session_id);
        let (rate_limit, duration) = match transaction_type {
            TransactionType::Transfer | TransactionType::TokenTransfer => (10, Duration::minutes(10)),
            TransactionType::DefiSwap | TransactionType::DefiStake => (5, Duration::minutes(10)),
            TransactionType::ContractCall | TransactionType::ContractDeployment => (3, Duration::minutes(10)),
            _ => (15, Duration::minutes(10)),
        };

        if !self.rate_limiter.check_rate_limit(&rate_limit_key, rate_limit, duration).await? {
            return Err(Status::resource_exhausted("Rate limit exceeded for transaction signing"));
        }

        // Validate addresses
        self.validate_address_format(from_address, key_type)?;
        self.validate_address_format(to_address, key_type)?;

        // Validate amount
        self.validate_transaction_amount(amount, transaction_type)?;

        // Check transaction limits
        self.validate_transaction_limits(&auth_context.user_id, amount, key_type, chain_id, transaction_type).await?;

        // Check signing permissions
        self.validate_signing_permissions(&auth_context, from_address).await?;

        // Check for suspicious transaction patterns
        self.check_signing_patterns(&auth_context.user_id, session_id, Some(transaction_type)).await?;

        // Validate recipient address (blacklist check)
        self.validate_recipient_address(to_address, key_type).await?;

        // Log transaction signing attempt
        self.audit_logger.log_action(
            &auth_context.user_id.to_string(),
            "transaction_signing_attempt",
            &format!("Session: {}, From: {}, To: {}, Amount: {}, Type: {:?}", 
                session_id, from_address, to_address, amount, transaction_type),
            true,
            None,
        ).await;

        Ok(auth_context)
    }

    /// Validate batch signing request
    pub async fn validate_batch_signing<T>(
        &self,
        request: &Request<T>,
        session_id: &Uuid,
        batch_size: usize,
        operation_type: &str,
    ) -> Result<AuthContext, Status> {
        // Extract authentication
        let auth_context = self.auth_service.extract_auth(request).await?;

        // Validate batch size
        let max_batch_size = match operation_type {
            "transactions" => 5, // Max 5 transactions per batch
            "messages" => 10, // Max 10 messages per batch
            _ => 3, // Conservative default
        };

        if batch_size > max_batch_size {
            return Err(Status::invalid_argument(
                format!("Batch size {} exceeds maximum of {}", batch_size, max_batch_size)
            ));
        }

        // Rate limiting for batch operations
        let rate_limit_key = format!("batch_signing:{}:{}", auth_context.user_id, session_id);
        if !self.rate_limiter.check_rate_limit(&rate_limit_key, 3, Duration::minutes(10)).await? {
            return Err(Status::resource_exhausted("Rate limit exceeded for batch signing"));
        }

        // Log batch signing attempt
        self.audit_logger.log_action(
            &auth_context.user_id.to_string(),
            "batch_signing_attempt",
            &format!("Session: {}, Type: {}, Size: {}", session_id, operation_type, batch_size),
            true,
            None,
        ).await;

        Ok(auth_context)
    }

    /// Validate analytics access
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
        let rate_limit_key = format!("signing_analytics_access:{}", auth_context.user_id);
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
        self.auth_service.check_permission(&auth_context, Permission::ManageSigning)?;

        // Rate limiting based on operation type
        let (rate_limit, duration) = match operation_type {
            "flag_suspicious_activity" => (20, Duration::hours(1)), // 20 flags per hour
            "system_analytics" => (30, Duration::hours(1)), // 30 system analytics per hour
            "transaction_limits" => (100, Duration::hours(1)), // 100 limit checks per hour
            _ => (10, Duration::hours(1)), // Default rate limit
        };

        let rate_limit_key = format!("signing_admin_{}:{}", operation_type, auth_context.user_id);
        if !self.rate_limiter.check_rate_limit(&rate_limit_key, rate_limit, duration).await? {
            return Err(Status::resource_exhausted("Rate limit exceeded for administrative operations"));
        }

        // Log administrative action
        self.audit_logger.log_action(
            &auth_context.user_id.to_string(),
            "signing_admin_operation",
            &format!("Operation: {}", operation_type),
            true,
            None,
        ).await;

        Ok(auth_context)
    }

    /// Validate message content for security
    fn validate_message_content(&self, message: &str) -> Result<(), Status> {
        if message.is_empty() {
            return Err(Status::invalid_argument("Message cannot be empty"));
        }

        // Prevent excessively long messages
        if message.len() > 10_000 {
            return Err(Status::invalid_argument("Message too long"));
        }

        // Check for suspicious content patterns
        let suspicious_patterns = [
            "private key", "seed phrase", "mnemonic", "password", "secret",
            "phishing", "scam", "fake", "malicious"
        ];

        let message_lower = message.to_lowercase();
        if suspicious_patterns.iter().any(|pattern| message_lower.contains(pattern)) {
            return Err(Status::invalid_argument("Message contains suspicious content"));
        }

        Ok(())
    }

    /// Validate address format based on key type
    fn validate_address_format(&self, address: &str, key_type: KeyType) -> Result<(), Status> {
        if address.is_empty() {
            return Err(Status::invalid_argument("Address cannot be empty"));
        }

        match key_type {
            KeyType::Ethereum => {
                if !address.starts_with("0x") || address.len() != 42 {
                    return Err(Status::invalid_argument("Invalid Ethereum address format"));
                }
            }
            KeyType::Bitcoin => {
                if address.len() < 26 || address.len() > 62 {
                    return Err(Status::invalid_argument("Invalid Bitcoin address format"));
                }
            }
            KeyType::Solana => {
                if address.len() < 32 || address.len() > 44 {
                    return Err(Status::invalid_argument("Invalid Solana address format"));
                }
            }
        }

        Ok(())
    }

    /// Validate transaction amount
    fn validate_transaction_amount(&self, amount: &Decimal, transaction_type: TransactionType) -> Result<(), Status> {
        if *amount <= Decimal::ZERO {
            return Err(Status::invalid_argument("Transaction amount must be positive"));
        }

        // Set maximum amounts based on transaction type (in USD equivalent)
        let max_amount = match transaction_type {
            TransactionType::Transfer => Decimal::from(100_000), // $100,000
            TransactionType::TokenTransfer => Decimal::from(50_000), // $50,000
            TransactionType::DefiSwap => Decimal::from(200_000), // $200,000
            TransactionType::DefiStake => Decimal::from(500_000), // $500,000
            TransactionType::ContractCall => Decimal::from(10_000), // $10,000
            TransactionType::ContractDeployment => Decimal::from(1_000), // $1,000
            _ => Decimal::from(25_000), // $25,000 default
        };

        if *amount > max_amount {
            return Err(Status::invalid_argument(
                format!("Transaction amount {} exceeds maximum of {}", amount, max_amount)
            ));
        }

        Ok(())
    }

    /// Validate transaction limits
    async fn validate_transaction_limits(
        &self,
        user_id: &Uuid,
        amount: &Decimal,
        key_type: KeyType,
        chain_id: &str,
        transaction_type: TransactionType,
    ) -> Result<(), Status> {
        let (within_limits, daily_limit, daily_used, daily_remaining) = 
            self.dapp_signing_repository.check_transaction_limits(
                user_id,
                amount,
                key_type,
                chain_id,
                transaction_type,
                24, // 24-hour window
            ).await
            .map_err(|e| Status::internal(format!("Failed to check transaction limits: {}", e)))?;

        if !within_limits {
            return Err(Status::resource_exhausted(
                format!("Transaction limits exceeded. Daily limit: {}, Used: {}, Remaining: {}", 
                    daily_limit, daily_used, daily_remaining)
            ));
        }

        Ok(())
    }

    /// Validate signing permissions
    async fn validate_signing_permissions(&self, auth_context: &AuthContext, address: &str) -> Result<(), Status> {
        // For now, basic validation - in production, this would check if the user owns the address
        // This could integrate with wallet service to verify address ownership
        
        // Check if user has signing permissions
        if !self.auth_service.has_permission(auth_context, Permission::SignTransactions).await? {
            return Err(Status::permission_denied("User does not have signing permissions"));
        }

        Ok(())
    }

    /// Validate recipient address (blacklist check)
    async fn validate_recipient_address(&self, address: &str, key_type: KeyType) -> Result<(), Status> {
        // Known malicious addresses (simplified - in production this would be a comprehensive database)
        let blacklisted_addresses = [
            "0x0000000000000000000000000000000000000000", // Ethereum null address
            "0xdead000000000000000000000000000000000000", // Common burn address
        ];

        if blacklisted_addresses.contains(&address) {
            return Err(Status::invalid_argument("Recipient address is blacklisted"));
        }

        Ok(())
    }

    /// Check for suspicious signing patterns
    async fn check_signing_patterns(
        &self,
        user_id: &Uuid,
        session_id: &Uuid,
        transaction_type: Option<TransactionType>,
    ) -> Result<(), Status> {
        let end_date = Utc::now();
        let start_date = end_date - Duration::minutes(5);

        // Get recent signing history
        if let Ok((entries, _)) = self.dapp_signing_repository.get_signing_history(
            Some(*user_id),
            Some(*session_id),
            None,
            None,
            transaction_type,
            Some(start_date),
            Some(end_date),
            1,
            100,
        ).await {
            // Check for rapid-fire signing
            if entries.len() > 10 {
                return Err(Status::resource_exhausted("Too many signing operations in short time"));
            }

            // Check for failed attempts pattern
            let failed_attempts = entries.iter().filter(|e| !e.success).count();
            if failed_attempts > 5 {
                return Err(Status::resource_exhausted("Too many failed signing attempts"));
            }
        }

        Ok(())
    }
}
