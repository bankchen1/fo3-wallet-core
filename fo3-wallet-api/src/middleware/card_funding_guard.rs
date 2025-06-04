//! Card funding security guard middleware

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
use crate::models::card_funding::{
    CardFundingRepository, FundingSourceType, FundingTransactionStatus,
    FundingLimits, FundingTransaction,
};
use crate::proto::fo3::wallet::v1::{Permission, UserRole};

/// Card funding security guard for validation and fraud prevention
#[derive(Debug)]
pub struct CardFundingGuard {
    auth_service: Arc<AuthService>,
    audit_logger: Arc<AuditLogger>,
    rate_limiter: Arc<RateLimiter>,
    funding_repository: Arc<dyn CardFundingRepository>,
}

impl CardFundingGuard {
    pub fn new(
        auth_service: Arc<AuthService>,
        audit_logger: Arc<AuditLogger>,
        rate_limiter: Arc<RateLimiter>,
        funding_repository: Arc<dyn CardFundingRepository>,
    ) -> Self {
        Self {
            auth_service,
            audit_logger,
            rate_limiter,
            funding_repository,
        }
    }

    /// Validate funding source creation
    pub async fn validate_funding_source_creation<T>(
        &self,
        request: &Request<T>,
        source_type: &FundingSourceType,
        amount_limit: &Decimal,
    ) -> Result<AuthContext, Status> {
        // Extract and validate authentication
        let auth_context = self.auth_service.extract_auth(request).await?;

        // Check basic permissions
        self.auth_service.check_permission(&auth_context, Permission::PermissionCardFunding)?;

        // Rate limiting for funding source creation
        let rate_key = format!("funding_source_creation:{}", auth_context.user_id);
        if !self.rate_limiter.check_rate_limit(&rate_key, 5, Duration::minutes(15)).await {
            self.audit_logger.log_security_event(
                &auth_context.user_id.to_string(),
                "funding_source_creation_rate_limit",
                &format!("Rate limit exceeded for funding source creation: {}", source_type),
                request.remote_addr(),
            ).await;
            return Err(Status::resource_exhausted("Too many funding source creation attempts"));
        }

        // Validate funding source type limits
        self.validate_funding_source_limits(&auth_context.user_id, source_type, amount_limit).await?;

        // Log the validation
        self.audit_logger.log_operation(
            &auth_context.user_id.to_string(),
            "funding_source_validation",
            &format!("Validated funding source creation: type={}, limit={}", source_type, amount_limit),
            true,
            request.remote_addr(),
        ).await;

        Ok(auth_context)
    }

    /// Validate funding transaction
    pub async fn validate_funding_transaction<T>(
        &self,
        request: &Request<T>,
        card_id: &Uuid,
        funding_source_id: &Uuid,
        amount: &Decimal,
        currency: &str,
    ) -> Result<AuthContext, Status> {
        // Extract and validate authentication
        let auth_context = self.auth_service.extract_auth(request).await?;

        // Check basic permissions
        self.auth_service.check_permission(&auth_context, Permission::PermissionCardFunding)?;

        // Rate limiting for funding transactions
        let rate_key = format!("funding_transaction:{}", auth_context.user_id);
        if !self.rate_limiter.check_rate_limit(&rate_key, 10, Duration::minutes(5)).await {
            self.audit_logger.log_security_event(
                &auth_context.user_id.to_string(),
                "funding_transaction_rate_limit",
                &format!("Rate limit exceeded for funding transactions: amount={}", amount),
                request.remote_addr(),
            ).await;
            return Err(Status::resource_exhausted("Too many funding transaction attempts"));
        }

        // Validate funding limits
        self.validate_funding_limits(&auth_context.user_id, amount, currency).await?;

        // Validate funding source ownership and status
        self.validate_funding_source_ownership(&auth_context.user_id, funding_source_id).await?;

        // Validate transaction amount
        self.validate_transaction_amount(amount, currency).await?;

        // Check for suspicious patterns
        self.check_suspicious_patterns(&auth_context.user_id, amount, currency).await?;

        // Log the validation
        self.audit_logger.log_operation(
            &auth_context.user_id.to_string(),
            "funding_transaction_validation",
            &format!("Validated funding transaction: card={}, source={}, amount={} {}", 
                card_id, funding_source_id, amount, currency),
            true,
            request.remote_addr(),
        ).await;

        Ok(auth_context)
    }

    /// Validate crypto funding transaction
    pub async fn validate_crypto_funding<T>(
        &self,
        request: &Request<T>,
        amount: &Decimal,
        currency: &str,
        network: &str,
    ) -> Result<AuthContext, Status> {
        // Extract and validate authentication
        let auth_context = self.auth_service.extract_auth(request).await?;

        // Check crypto funding permissions
        self.auth_service.check_permission(&auth_context, Permission::PermissionCardFunding)?;

        // Enhanced rate limiting for crypto funding
        let rate_key = format!("crypto_funding:{}", auth_context.user_id);
        if !self.rate_limiter.check_rate_limit(&rate_key, 3, Duration::minutes(10)).await {
            self.audit_logger.log_security_event(
                &auth_context.user_id.to_string(),
                "crypto_funding_rate_limit",
                &format!("Rate limit exceeded for crypto funding: {} on {}", currency, network),
                request.remote_addr(),
            ).await;
            return Err(Status::resource_exhausted("Too many crypto funding attempts"));
        }

        // Validate crypto funding limits (higher scrutiny)
        self.validate_crypto_funding_limits(&auth_context.user_id, amount, currency).await?;

        // Validate network and currency combination
        self.validate_crypto_network(currency, network).await?;

        // Log the validation
        self.audit_logger.log_operation(
            &auth_context.user_id.to_string(),
            "crypto_funding_validation",
            &format!("Validated crypto funding: amount={} {} on {}", amount, currency, network),
            true,
            request.remote_addr(),
        ).await;

        Ok(auth_context)
    }

    /// Validate admin operations
    pub async fn validate_admin_operation<T>(
        &self,
        request: &Request<T>,
        operation: &str,
        target_user_id: Option<&Uuid>,
    ) -> Result<AuthContext, Status> {
        // Extract and validate authentication
        let auth_context = self.auth_service.extract_auth(request).await?;

        // Check admin permissions
        if auth_context.role != UserRole::UserRoleAdmin && auth_context.role != UserRole::UserRoleSuperAdmin {
            return Err(Status::permission_denied("Admin access required"));
        }

        // Log admin operation
        let target_info = target_user_id.map(|id| id.to_string()).unwrap_or_else(|| "all".to_string());
        self.audit_logger.log_operation(
            &auth_context.user_id.to_string(),
            "admin_funding_operation",
            &format!("Admin operation: {} for user: {}", operation, target_info),
            true,
            request.remote_addr(),
        ).await;

        Ok(auth_context)
    }

    /// Validate funding source limits
    async fn validate_funding_source_limits(
        &self,
        user_id: &Uuid,
        source_type: &FundingSourceType,
        amount_limit: &Decimal,
    ) -> Result<(), Status> {
        // Get user's current funding sources
        let (sources, _) = self.funding_repository
            .list_funding_sources(user_id, Some(source_type.clone()), None, 1, 100)
            .await
            .map_err(|e| Status::internal(format!("Failed to check funding sources: {}", e)))?;

        // Limit number of funding sources per type
        let max_sources = match source_type {
            FundingSourceType::BankAccount => 5,
            FundingSourceType::CryptoWallet => 3,
            FundingSourceType::ExternalCard => 10,
            FundingSourceType::ACH => 5,
            FundingSourceType::FiatAccount => 3,
        };

        if sources.len() >= max_sources {
            return Err(Status::failed_precondition(
                format!("Maximum number of {} funding sources reached", source_type)
            ));
        }

        // Validate amount limits
        let max_limit = match source_type {
            FundingSourceType::CryptoWallet => Decimal::from(50000), // Higher scrutiny for crypto
            _ => Decimal::from(100000),
        };

        if *amount_limit > max_limit {
            return Err(Status::invalid_argument(
                format!("Funding limit exceeds maximum allowed: {}", max_limit)
            ));
        }

        Ok(())
    }

    /// Validate funding limits
    async fn validate_funding_limits(
        &self,
        user_id: &Uuid,
        amount: &Decimal,
        currency: &str,
    ) -> Result<(), Status> {
        // Get user's funding limits
        let limits = self.funding_repository
            .get_funding_limits(user_id)
            .await
            .map_err(|e| Status::internal(format!("Failed to get funding limits: {}", e)))?;

        let limits = limits.unwrap_or_else(|| FundingLimits::default());

        // Check per-transaction limit
        if *amount > limits.per_transaction_limit {
            return Err(Status::failed_precondition(
                format!("Amount exceeds per-transaction limit: {}", limits.per_transaction_limit)
            ));
        }

        // Check daily limit
        if limits.daily_used + amount > limits.daily_limit {
            return Err(Status::failed_precondition(
                format!("Amount would exceed daily limit: {}", limits.daily_limit)
            ));
        }

        // Check monthly limit
        if limits.monthly_used + amount > limits.monthly_limit {
            return Err(Status::failed_precondition(
                format!("Amount would exceed monthly limit: {}", limits.monthly_limit)
            ));
        }

        // Check transaction count limits
        if limits.daily_transactions_used >= limits.daily_transaction_count {
            return Err(Status::failed_precondition("Daily transaction count limit reached"));
        }

        if limits.monthly_transactions_used >= limits.monthly_transaction_count {
            return Err(Status::failed_precondition("Monthly transaction count limit reached"));
        }

        Ok(())
    }

    /// Validate funding source ownership
    async fn validate_funding_source_ownership(
        &self,
        user_id: &Uuid,
        funding_source_id: &Uuid,
    ) -> Result<(), Status> {
        let source = self.funding_repository
            .get_funding_source_by_user(user_id, funding_source_id)
            .await
            .map_err(|e| Status::internal(format!("Failed to get funding source: {}", e)))?;

        match source {
            Some(source) => {
                if source.user_id != *user_id {
                    return Err(Status::permission_denied("Funding source not owned by user"));
                }
                if !source.is_verified {
                    return Err(Status::failed_precondition("Funding source not verified"));
                }
                Ok(())
            }
            None => Err(Status::not_found("Funding source not found")),
        }
    }

    /// Validate transaction amount
    async fn validate_transaction_amount(&self, amount: &Decimal, currency: &str) -> Result<(), Status> {
        // Minimum amount validation
        let min_amount = Decimal::from(1); // $1 minimum
        if *amount < min_amount {
            return Err(Status::invalid_argument(
                format!("Amount below minimum: {}", min_amount)
            ));
        }

        // Maximum amount validation (per transaction)
        let max_amount = match currency {
            "USD" | "EUR" | "GBP" => Decimal::from(25000),
            _ => Decimal::from(10000), // Lower limits for other currencies
        };

        if *amount > max_amount {
            return Err(Status::invalid_argument(
                format!("Amount exceeds maximum: {}", max_amount)
            ));
        }

        Ok(())
    }

    /// Check for suspicious funding patterns
    async fn check_suspicious_patterns(
        &self,
        user_id: &Uuid,
        amount: &Decimal,
        currency: &str,
    ) -> Result<(), Status> {
        // Get recent transactions for pattern analysis
        let now = Utc::now();
        let one_hour_ago = now - Duration::hours(1);
        
        let (recent_transactions, _) = self.funding_repository
            .list_funding_transactions(user_id, None, None, Some(FundingTransactionStatus::Completed), 1, 50)
            .await
            .map_err(|e| Status::internal(format!("Failed to get recent transactions: {}", e)))?;

        // Check for rapid successive transactions
        let recent_count = recent_transactions
            .iter()
            .filter(|tx| tx.created_at > one_hour_ago)
            .count();

        if recent_count >= 5 {
            return Err(Status::failed_precondition(
                "Too many recent funding transactions detected"
            ));
        }

        // Check for unusual amount patterns
        let recent_total: Decimal = recent_transactions
            .iter()
            .filter(|tx| tx.created_at > one_hour_ago && tx.currency == currency)
            .map(|tx| tx.amount)
            .sum();

        if recent_total > Decimal::from(50000) {
            return Err(Status::failed_precondition(
                "Unusual funding volume detected in recent transactions"
            ));
        }

        Ok(())
    }

    /// Validate crypto funding limits (enhanced)
    async fn validate_crypto_funding_limits(
        &self,
        user_id: &Uuid,
        amount: &Decimal,
        currency: &str,
    ) -> Result<(), Status> {
        // Enhanced limits for crypto funding
        let daily_crypto_limit = Decimal::from(10000); // $10k daily for crypto
        let monthly_crypto_limit = Decimal::from(100000); // $100k monthly for crypto

        // Get recent crypto funding volume
        let now = Utc::now();
        let daily_volume = self.funding_repository
            .get_user_funding_volume(user_id, &(now - Duration::days(1)), &now)
            .await
            .map_err(|e| Status::internal(format!("Failed to get funding volume: {}", e)))?;

        let monthly_volume = self.funding_repository
            .get_user_funding_volume(user_id, &(now - Duration::days(30)), &now)
            .await
            .map_err(|e| Status::internal(format!("Failed to get funding volume: {}", e)))?;

        if daily_volume + amount > daily_crypto_limit {
            return Err(Status::failed_precondition(
                format!("Crypto funding would exceed daily limit: {}", daily_crypto_limit)
            ));
        }

        if monthly_volume + amount > monthly_crypto_limit {
            return Err(Status::failed_precondition(
                format!("Crypto funding would exceed monthly limit: {}", monthly_crypto_limit)
            ));
        }

        Ok(())
    }

    /// Validate crypto network and currency combination
    async fn validate_crypto_network(&self, currency: &str, network: &str) -> Result<(), Status> {
        let valid_combinations = match currency {
            "USDT" => vec!["ethereum", "tron", "bsc", "polygon"],
            "USDC" => vec!["ethereum", "polygon", "avalanche", "solana"],
            "DAI" => vec!["ethereum", "polygon"],
            "BUSD" => vec!["bsc", "ethereum"],
            _ => return Err(Status::invalid_argument("Unsupported cryptocurrency")),
        };

        if !valid_combinations.contains(&network) {
            return Err(Status::invalid_argument(
                format!("Invalid network {} for currency {}", network, currency)
            ));
        }

        Ok(())
    }
}
