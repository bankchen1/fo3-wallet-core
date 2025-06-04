//! Earn service security guard middleware

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
use crate::models::earn::{
    EarnRepository, YieldProductType, ProtocolType, RiskLevel, KeyType,
};
use crate::proto::fo3::wallet::v1::{Permission, UserRole};

/// Earn service security guard for validation and fraud prevention
#[derive(Debug)]
pub struct EarnGuard {
    auth_service: Arc<AuthService>,
    audit_logger: Arc<AuditLogger>,
    rate_limiter: Arc<RateLimiter>,
    earn_repository: Arc<dyn EarnRepository>,
}

impl EarnGuard {
    pub fn new(
        auth_service: Arc<AuthService>,
        audit_logger: Arc<AuditLogger>,
        rate_limiter: Arc<RateLimiter>,
        earn_repository: Arc<dyn EarnRepository>,
    ) -> Self {
        Self {
            auth_service,
            audit_logger,
            rate_limiter,
            earn_repository,
        }
    }

    /// Validate yield product access
    pub async fn validate_yield_product_access<T>(
        &self,
        request: &Request<T>,
        operation_type: &str,
    ) -> Result<AuthContext, Status> {
        // Extract and validate authentication
        let auth_context = self.auth_service.extract_auth(request).await?;

        // Rate limiting for yield product operations
        let rate_limit_key = format!("yield_products:{}:{}", operation_type, auth_context.user_id);
        let (rate_limit, duration) = match operation_type {
            "list" => (100, Duration::hours(1)), // 100 listings per hour
            "get" => (200, Duration::hours(1)), // 200 gets per hour
            "calculate" => (50, Duration::hours(1)), // 50 calculations per hour
            _ => (20, Duration::hours(1)), // Default rate limit
        };

        if !self.rate_limiter.check_rate_limit(&rate_limit_key, rate_limit, duration).await? {
            return Err(Status::resource_exhausted("Rate limit exceeded for yield product operations"));
        }

        Ok(auth_context)
    }

    /// Validate staking operation
    pub async fn validate_staking_operation<T>(
        &self,
        request: &Request<T>,
        operation_type: &str,
        amount: Option<&Decimal>,
        product_id: Option<&Uuid>,
    ) -> Result<AuthContext, Status> {
        // Extract and validate authentication
        let auth_context = self.auth_service.extract_auth(request).await?;

        // Check staking permissions
        if !self.auth_service.has_permission(&auth_context, Permission::ManageStaking).await? {
            return Err(Status::permission_denied("User does not have staking permissions"));
        }

        // Rate limiting for staking operations
        let rate_limit_key = format!("staking:{}:{}", operation_type, auth_context.user_id);
        let (rate_limit, duration) = match operation_type {
            "stake" => (10, Duration::hours(1)), // 10 stakes per hour
            "unstake" => (10, Duration::hours(1)), // 10 unstakes per hour
            "claim_rewards" => (20, Duration::hours(1)), // 20 claims per hour
            "list" => (50, Duration::hours(1)), // 50 listings per hour
            _ => (30, Duration::hours(1)), // Default rate limit
        };

        if !self.rate_limiter.check_rate_limit(&rate_limit_key, rate_limit, duration).await? {
            return Err(Status::resource_exhausted("Rate limit exceeded for staking operations"));
        }

        // Validate amount if provided
        if let Some(amount) = amount {
            self.validate_staking_amount(amount, operation_type)?;
        }

        // Validate product if provided
        if let Some(product_id) = product_id {
            self.validate_yield_product_for_staking(product_id).await?;
        }

        // Log staking operation attempt
        self.audit_logger.log_action(
            &auth_context.user_id,
            "staking_operation_attempt",
            &format!("Operation: {}, Amount: {:?}, Product: {:?}", 
                operation_type, amount, product_id),
            true,
            None,
        ).await;

        Ok(auth_context)
    }

    /// Validate lending operation
    pub async fn validate_lending_operation<T>(
        &self,
        request: &Request<T>,
        operation_type: &str,
        amount: Option<&Decimal>,
        product_id: Option<&Uuid>,
    ) -> Result<AuthContext, Status> {
        // Extract and validate authentication
        let auth_context = self.auth_service.extract_auth(request).await?;

        // Check lending permissions
        if !self.auth_service.has_permission(&auth_context, Permission::ManageLending).await? {
            return Err(Status::permission_denied("User does not have lending permissions"));
        }

        // Rate limiting for lending operations
        let rate_limit_key = format!("lending:{}:{}", operation_type, auth_context.user_id);
        let (rate_limit, duration) = match operation_type {
            "supply" => (15, Duration::hours(1)), // 15 supplies per hour
            "withdraw" => (15, Duration::hours(1)), // 15 withdrawals per hour
            "list" => (50, Duration::hours(1)), // 50 listings per hour
            _ => (30, Duration::hours(1)), // Default rate limit
        };

        if !self.rate_limiter.check_rate_limit(&rate_limit_key, rate_limit, duration).await? {
            return Err(Status::resource_exhausted("Rate limit exceeded for lending operations"));
        }

        // Validate amount if provided
        if let Some(amount) = amount {
            self.validate_lending_amount(amount, operation_type)?;
        }

        // Validate product if provided
        if let Some(product_id) = product_id {
            self.validate_yield_product_for_lending(product_id).await?;
        }

        // Log lending operation attempt
        self.audit_logger.log_action(
            &auth_context.user_id,
            "lending_operation_attempt",
            &format!("Operation: {}, Amount: {:?}, Product: {:?}", 
                operation_type, amount, product_id),
            true,
            None,
        ).await;

        Ok(auth_context)
    }

    /// Validate vault operation
    pub async fn validate_vault_operation<T>(
        &self,
        request: &Request<T>,
        operation_type: &str,
        amount: Option<&Decimal>,
        product_id: Option<&Uuid>,
    ) -> Result<AuthContext, Status> {
        // Extract and validate authentication
        let auth_context = self.auth_service.extract_auth(request).await?;

        // Check vault permissions
        if !self.auth_service.has_permission(&auth_context, Permission::ManageVaults).await? {
            return Err(Status::permission_denied("User does not have vault permissions"));
        }

        // Rate limiting for vault operations
        let rate_limit_key = format!("vault:{}:{}", operation_type, auth_context.user_id);
        let (rate_limit, duration) = match operation_type {
            "deposit" => (10, Duration::hours(1)), // 10 deposits per hour
            "withdraw" => (10, Duration::hours(1)), // 10 withdrawals per hour
            "list" => (50, Duration::hours(1)), // 50 listings per hour
            _ => (30, Duration::hours(1)), // Default rate limit
        };

        if !self.rate_limiter.check_rate_limit(&rate_limit_key, rate_limit, duration).await? {
            return Err(Status::resource_exhausted("Rate limit exceeded for vault operations"));
        }

        // Validate amount if provided
        if let Some(amount) = amount {
            self.validate_vault_amount(amount, operation_type)?;
        }

        // Validate product if provided
        if let Some(product_id) = product_id {
            self.validate_yield_product_for_vault(product_id).await?;
        }

        // Log vault operation attempt
        self.audit_logger.log_action(
            &auth_context.user_id,
            "vault_operation_attempt",
            &format!("Operation: {}, Amount: {:?}, Product: {:?}", 
                operation_type, amount, product_id),
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
        analytics_type: &str,
    ) -> Result<AuthContext, Status> {
        // Extract authentication
        let auth_context = self.auth_service.extract_auth(request).await?;

        // Check if user can access analytics for themselves or has admin permissions
        if let Some(target_id) = target_user_id {
            if target_id != Uuid::parse_str(&auth_context.user_id).unwrap() &&
               !self.auth_service.has_permission(&auth_context, Permission::ViewAnalytics).await? {
                return Err(Status::permission_denied("Can only view your own analytics"));
            }
        }

        // Rate limiting for analytics access
        let rate_limit_key = format!("earn_analytics:{}:{}", analytics_type, auth_context.user_id);
        let (rate_limit, duration) = match analytics_type {
            "portfolio_summary" => (30, Duration::hours(1)), // 30 summaries per hour
            "yield_chart" => (50, Duration::hours(1)), // 50 charts per hour
            "earn_analytics" => (20, Duration::hours(1)), // 20 analytics per hour
            _ => (40, Duration::hours(1)), // Default rate limit
        };

        if !self.rate_limiter.check_rate_limit(&rate_limit_key, rate_limit, duration).await? {
            return Err(Status::resource_exhausted("Rate limit exceeded for analytics access"));
        }

        Ok(auth_context)
    }

    /// Validate risk and optimization access
    pub async fn validate_risk_optimization_access<T>(
        &self,
        request: &Request<T>,
        operation_type: &str,
    ) -> Result<AuthContext, Status> {
        // Extract authentication
        let auth_context = self.auth_service.extract_auth(request).await?;

        // Check risk management permissions
        if !self.auth_service.has_permission(&auth_context, Permission::ViewRiskAnalytics).await? {
            return Err(Status::permission_denied("User does not have risk analytics permissions"));
        }

        // Rate limiting for risk and optimization operations
        let rate_limit_key = format!("risk_optimization:{}:{}", operation_type, auth_context.user_id);
        let (rate_limit, duration) = match operation_type {
            "assess_risk" => (20, Duration::hours(1)), // 20 assessments per hour
            "optimize_portfolio" => (10, Duration::hours(1)), // 10 optimizations per hour
            _ => (15, Duration::hours(1)), // Default rate limit
        };

        if !self.rate_limiter.check_rate_limit(&rate_limit_key, rate_limit, duration).await? {
            return Err(Status::resource_exhausted("Rate limit exceeded for risk/optimization operations"));
        }

        // Log risk/optimization operation attempt
        self.audit_logger.log_action(
            &auth_context.user_id,
            "risk_optimization_attempt",
            &format!("Operation: {}", operation_type),
            true,
            None,
        ).await;

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
        self.auth_service.check_permission(&auth_context, Permission::ManageEarnProducts)?;

        // Rate limiting based on operation type
        let (rate_limit, duration) = match operation_type {
            "create_product" => (5, Duration::hours(1)), // 5 product creations per hour
            "update_product" => (20, Duration::hours(1)), // 20 product updates per hour
            "system_analytics" => (30, Duration::hours(1)), // 30 system analytics per hour
            _ => (10, Duration::hours(1)), // Default rate limit
        };

        let rate_limit_key = format!("earn_admin_{}:{}", operation_type, auth_context.user_id);
        if !self.rate_limiter.check_rate_limit(&rate_limit_key, rate_limit, duration).await? {
            return Err(Status::resource_exhausted("Rate limit exceeded for administrative operations"));
        }

        // Log administrative action
        self.audit_logger.log_action(
            &auth_context.user_id,
            "earn_admin_operation",
            &format!("Operation: {}", operation_type),
            true,
            None,
        ).await;

        Ok(auth_context)
    }

    /// Validate staking amount
    fn validate_staking_amount(&self, amount: &Decimal, operation_type: &str) -> Result<(), Status> {
        if *amount <= Decimal::ZERO {
            return Err(Status::invalid_argument("Staking amount must be positive"));
        }

        // Set maximum amounts based on operation type
        let max_amount = match operation_type {
            "stake" => Decimal::from(1_000_000), // $1M max stake
            "unstake" => Decimal::from(10_000_000), // $10M max unstake (can unstake large positions)
            _ => Decimal::from(500_000), // $500K default
        };

        if *amount > max_amount {
            return Err(Status::invalid_argument(
                format!("Staking amount {} exceeds maximum of {}", amount, max_amount)
            ));
        }

        Ok(())
    }

    /// Validate lending amount
    fn validate_lending_amount(&self, amount: &Decimal, operation_type: &str) -> Result<(), Status> {
        if *amount <= Decimal::ZERO {
            return Err(Status::invalid_argument("Lending amount must be positive"));
        }

        // Set maximum amounts based on operation type
        let max_amount = match operation_type {
            "supply" => Decimal::from(2_000_000), // $2M max supply
            "withdraw" => Decimal::from(10_000_000), // $10M max withdraw
            _ => Decimal::from(1_000_000), // $1M default
        };

        if *amount > max_amount {
            return Err(Status::invalid_argument(
                format!("Lending amount {} exceeds maximum of {}", amount, max_amount)
            ));
        }

        Ok(())
    }

    /// Validate vault amount
    fn validate_vault_amount(&self, amount: &Decimal, operation_type: &str) -> Result<(), Status> {
        if *amount <= Decimal::ZERO {
            return Err(Status::invalid_argument("Vault amount must be positive"));
        }

        // Set maximum amounts based on operation type
        let max_amount = match operation_type {
            "deposit" => Decimal::from(5_000_000), // $5M max deposit
            "withdraw" => Decimal::from(10_000_000), // $10M max withdraw
            _ => Decimal::from(2_000_000), // $2M default
        };

        if *amount > max_amount {
            return Err(Status::invalid_argument(
                format!("Vault amount {} exceeds maximum of {}", amount, max_amount)
            ));
        }

        Ok(())
    }

    /// Validate yield product for staking
    async fn validate_yield_product_for_staking(&self, product_id: &Uuid) -> Result<(), Status> {
        let product = self.earn_repository.get_yield_product(product_id).await
            .map_err(|e| Status::internal(format!("Failed to get yield product: {}", e)))?
            .ok_or_else(|| Status::not_found("Yield product not found"))?;

        if product.product_type != YieldProductType::Staking {
            return Err(Status::invalid_argument("Product is not a staking product"));
        }

        if !product.is_active {
            return Err(Status::failed_precondition("Staking product is not active"));
        }

        Ok(())
    }

    /// Validate yield product for lending
    async fn validate_yield_product_for_lending(&self, product_id: &Uuid) -> Result<(), Status> {
        let product = self.earn_repository.get_yield_product(product_id).await
            .map_err(|e| Status::internal(format!("Failed to get yield product: {}", e)))?
            .ok_or_else(|| Status::not_found("Yield product not found"))?;

        if product.product_type != YieldProductType::Lending {
            return Err(Status::invalid_argument("Product is not a lending product"));
        }

        if !product.is_active {
            return Err(Status::failed_precondition("Lending product is not active"));
        }

        Ok(())
    }

    /// Validate yield product for vault
    async fn validate_yield_product_for_vault(&self, product_id: &Uuid) -> Result<(), Status> {
        let product = self.earn_repository.get_yield_product(product_id).await
            .map_err(|e| Status::internal(format!("Failed to get yield product: {}", e)))?
            .ok_or_else(|| Status::not_found("Yield product not found"))?;

        if product.product_type != YieldProductType::Vault {
            return Err(Status::invalid_argument("Product is not a vault product"));
        }

        if !product.is_active {
            return Err(Status::failed_precondition("Vault product is not active"));
        }

        Ok(())
    }

    /// Validate lending operation
    pub async fn validate_lending_operation<T>(
        &self,
        request: &Request<T>,
        operation_type: &str,
    ) -> Result<AuthContext, Status> {
        // Extract authentication
        let auth_context = self.auth_service.extract_auth(request).await?;

        // Check lending permissions
        if !self.auth_service.has_permission(&auth_context, Permission::UseLendingProducts).await? {
            return Err(Status::permission_denied("User does not have lending permissions"));
        }

        // Rate limiting for lending operations
        let rate_limit_key = format!("lending_{}:{}", operation_type, auth_context.user_id);
        let (rate_limit, duration) = match operation_type {
            "supply" => (20, Duration::hours(1)), // 20 supplies per hour
            "withdraw" => (30, Duration::hours(1)), // 30 withdrawals per hour
            "list" => (100, Duration::hours(1)), // 100 list operations per hour
            _ => (25, Duration::hours(1)), // Default rate limit
        };

        if !self.rate_limiter.check_rate_limit(&rate_limit_key, rate_limit, duration).await? {
            return Err(Status::resource_exhausted("Rate limit exceeded for lending operations"));
        }

        // Log lending operation attempt
        self.audit_logger.log_action(
            &auth_context.user_id,
            "lending_operation_attempt",
            &format!("Operation: {}", operation_type),
            true,
            None,
        ).await;

        Ok(auth_context)
    }

    /// Validate vault operation
    pub async fn validate_vault_operation<T>(
        &self,
        request: &Request<T>,
        operation_type: &str,
    ) -> Result<AuthContext, Status> {
        // Extract authentication
        let auth_context = self.auth_service.extract_auth(request).await?;

        // Check vault permissions
        if !self.auth_service.has_permission(&auth_context, Permission::UseVaultProducts).await? {
            return Err(Status::permission_denied("User does not have vault permissions"));
        }

        // Rate limiting for vault operations
        let rate_limit_key = format!("vault_{}:{}", operation_type, auth_context.user_id);
        let (rate_limit, duration) = match operation_type {
            "deposit" => (15, Duration::hours(1)), // 15 deposits per hour
            "withdraw" => (25, Duration::hours(1)), // 25 withdrawals per hour
            "list" => (100, Duration::hours(1)), // 100 list operations per hour
            _ => (20, Duration::hours(1)), // Default rate limit
        };

        if !self.rate_limiter.check_rate_limit(&rate_limit_key, rate_limit, duration).await? {
            return Err(Status::resource_exhausted("Rate limit exceeded for vault operations"));
        }

        // Log vault operation attempt
        self.audit_logger.log_action(
            &auth_context.user_id,
            "vault_operation_attempt",
            &format!("Operation: {}", operation_type),
            true,
            None,
        ).await;

        Ok(auth_context)
    }

    /// Validate risk assessment operation
    pub async fn validate_risk_assessment<T>(
        &self,
        request: &Request<T>,
        operation_type: &str,
    ) -> Result<AuthContext, Status> {
        // Extract authentication
        let auth_context = self.auth_service.extract_auth(request).await?;

        // Check risk assessment permissions
        if !self.auth_service.has_permission(&auth_context, Permission::ViewRiskAnalytics).await? {
            return Err(Status::permission_denied("User does not have risk assessment permissions"));
        }

        // Rate limiting for risk assessment operations
        let rate_limit_key = format!("risk_{}:{}", operation_type, auth_context.user_id);
        let (rate_limit, duration) = match operation_type {
            "assess" => (15, Duration::hours(1)), // 15 assessments per hour
            "optimize" => (10, Duration::hours(1)), // 10 optimizations per hour
            _ => (12, Duration::hours(1)), // Default rate limit
        };

        if !self.rate_limiter.check_rate_limit(&rate_limit_key, rate_limit, duration).await? {
            return Err(Status::resource_exhausted("Rate limit exceeded for risk assessment operations"));
        }

        // Log risk assessment operation attempt
        self.audit_logger.log_action(
            &auth_context.user_id,
            "risk_assessment_attempt",
            &format!("Operation: {}", operation_type),
            true,
            None,
        ).await;

        Ok(auth_context)
    }

    /// Validate analytics access with simplified signature
    pub async fn validate_analytics_access_simple<T>(
        &self,
        request: &Request<T>,
        analytics_type: &str,
    ) -> Result<AuthContext, Status> {
        self.validate_analytics_access(request, None, analytics_type).await
    }
}
