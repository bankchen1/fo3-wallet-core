//! Fiat gateway authorization and compliance middleware

use std::sync::Arc;
use tonic::{Request, Status};
use uuid::Uuid;
use rust_decimal::Decimal;

use crate::middleware::auth::{AuthContext, AuthService};
use crate::middleware::kyc_guard::KycGuard;
use crate::models::fiat_gateway::{TransactionType, TransactionLimits, FiatTransaction};
use crate::state::AppState;
use crate::proto::fo3::wallet::v1::Permission;

/// Fiat gateway authorization guard
pub struct FiatGuard {
    state: Arc<AppState>,
    auth_service: Arc<AuthService>,
    kyc_guard: Arc<KycGuard>,
}

impl FiatGuard {
    pub fn new(
        state: Arc<AppState>,
        auth_service: Arc<AuthService>,
        kyc_guard: Arc<KycGuard>,
    ) -> Self {
        Self {
            state,
            auth_service,
            kyc_guard,
        }
    }

    /// Check if user can perform fiat deposits
    pub fn check_deposit_permission(&self, auth: &AuthContext) -> Result<(), Status> {
        self.auth_service.check_permission(auth, Permission::PermissionFiatDeposit)
    }

    /// Check if user can perform fiat withdrawals
    pub fn check_withdrawal_permission(&self, auth: &AuthContext) -> Result<(), Status> {
        self.auth_service.check_permission(auth, Permission::PermissionFiatWithdraw)
    }

    /// Check if user has admin permissions for fiat operations
    pub fn check_admin_permission(&self, auth: &AuthContext) -> Result<(), Status> {
        self.auth_service.check_permission(auth, Permission::PermissionFiatAdmin)
    }

    /// Validate transaction against user limits and KYC requirements
    pub async fn validate_transaction(
        &self,
        auth: &AuthContext,
        amount: Decimal,
        currency: &str,
        tx_type: TransactionType,
    ) -> Result<bool, Status> {
        let user_id = Uuid::parse_str(&auth.user_id)
            .map_err(|_| Status::internal("Invalid user ID format"))?;

        // Check KYC requirements for large transactions
        if amount >= Decimal::from(1000) {
            self.kyc_guard.require_kyc_approval(auth)?;
        }

        // Get transaction limits (mock implementation - in real app would query database)
        let limits = self.get_transaction_limits(user_id, currency).await?;

        // Calculate current usage (mock implementation)
        let daily_usage = self.calculate_daily_usage(user_id, tx_type, currency).await?;
        let monthly_usage = self.calculate_monthly_usage(user_id, tx_type, currency).await?;

        // Validate against limits
        limits.validate_transaction(amount, tx_type, daily_usage, monthly_usage)
            .map_err(|e| Status::failed_precondition(e))?;

        // Check if transaction requires approval
        let requires_approval = limits.requires_approval(amount);

        Ok(requires_approval)
    }

    /// Check AML (Anti-Money Laundering) compliance
    pub async fn check_aml_compliance(
        &self,
        auth: &AuthContext,
        amount: Decimal,
        currency: &str,
        tx_type: TransactionType,
    ) -> Result<(), Status> {
        let user_id = Uuid::parse_str(&auth.user_id)
            .map_err(|_| Status::internal("Invalid user ID format"))?;

        // Check for suspicious patterns
        if self.is_suspicious_pattern(user_id, amount, tx_type).await? {
            return Err(Status::failed_precondition(
                "Transaction flagged for manual review due to suspicious patterns"
            ));
        }

        // Check against sanctions lists (mock implementation)
        if self.is_sanctioned_user(user_id).await? {
            return Err(Status::permission_denied(
                "User is on sanctions list - transactions not permitted"
            ));
        }

        // Large transaction reporting threshold (e.g., $10,000)
        if amount >= Decimal::from(10000) {
            // In real implementation, this would trigger CTR (Currency Transaction Report)
            tracing::info!(
                user_id = %user_id,
                amount = %amount,
                currency = %currency,
                tx_type = ?tx_type,
                "Large transaction requires CTR reporting"
            );
        }

        Ok(())
    }

    /// Validate bank account ownership
    pub async fn validate_bank_account_access(
        &self,
        auth: &AuthContext,
        bank_account_id: Uuid,
    ) -> Result<(), Status> {
        let user_id = Uuid::parse_str(&auth.user_id)
            .map_err(|_| Status::internal("Invalid user ID format"))?;

        // Check if user owns the bank account (mock implementation)
        if !self.user_owns_bank_account(user_id, bank_account_id).await? {
            return Err(Status::permission_denied(
                "User does not have access to this bank account"
            ));
        }

        Ok(())
    }

    /// Check transaction velocity limits
    pub async fn check_velocity_limits(
        &self,
        auth: &AuthContext,
        amount: Decimal,
        tx_type: TransactionType,
    ) -> Result<(), Status> {
        let user_id = Uuid::parse_str(&auth.user_id)
            .map_err(|_| Status::internal("Invalid user ID format"))?;

        // Check transaction frequency (e.g., max 10 transactions per hour)
        let hourly_count = self.get_hourly_transaction_count(user_id, tx_type).await?;
        if hourly_count >= 10 {
            return Err(Status::resource_exhausted(
                "Transaction frequency limit exceeded. Please try again later."
            ));
        }

        // Check rapid succession transactions (e.g., no more than 3 in 5 minutes)
        let recent_count = self.get_recent_transaction_count(user_id, tx_type, 5).await?;
        if recent_count >= 3 {
            return Err(Status::resource_exhausted(
                "Too many transactions in quick succession. Please wait before trying again."
            ));
        }

        Ok(())
    }

    /// Validate jurisdiction compliance
    pub async fn check_jurisdiction_compliance(
        &self,
        auth: &AuthContext,
        amount: Decimal,
        currency: &str,
    ) -> Result<(), Status> {
        // Use existing KYC guard for jurisdiction checks
        self.kyc_guard.check_jurisdiction_compliance(auth, "US")?;

        // Additional fiat-specific jurisdiction checks
        if currency != "USD" && amount > Decimal::from(5000) {
            // Foreign currency transactions above $5k require additional verification
            return Err(Status::failed_precondition(
                "Foreign currency transactions above $5,000 require additional verification"
            ));
        }

        Ok(())
    }

    /// Check if user can cancel a transaction
    pub async fn can_cancel_transaction(
        &self,
        auth: &AuthContext,
        transaction_id: Uuid,
    ) -> Result<bool, Status> {
        let user_id = Uuid::parse_str(&auth.user_id)
            .map_err(|_| Status::internal("Invalid user ID format"))?;

        // Get transaction (mock implementation)
        let transaction = self.get_transaction(transaction_id).await?;

        // Check ownership
        if transaction.user_id != user_id {
            // Only admins can cancel other users' transactions
            self.check_admin_permission(auth)?;
        }

        // Check if transaction can be cancelled
        Ok(transaction.can_be_cancelled())
    }

    // Mock implementations for database operations
    // In real implementation, these would query the actual database

    async fn get_transaction_limits(&self, user_id: Uuid, currency: &str) -> Result<TransactionLimits, Status> {
        // Mock implementation - return default limits
        Ok(TransactionLimits::default_for_user(user_id, currency.to_string()))
    }

    async fn calculate_daily_usage(&self, user_id: Uuid, tx_type: TransactionType, currency: &str) -> Result<Decimal, Status> {
        // Mock implementation - return some usage
        Ok(Decimal::from(1000))
    }

    async fn calculate_monthly_usage(&self, user_id: Uuid, tx_type: TransactionType, currency: &str) -> Result<Decimal, Status> {
        // Mock implementation - return some usage
        Ok(Decimal::from(15000))
    }

    async fn is_suspicious_pattern(&self, user_id: Uuid, amount: Decimal, tx_type: TransactionType) -> Result<bool, Status> {
        // Mock implementation - flag round numbers above $9,999 as potentially suspicious
        Ok(amount > Decimal::from(9999) && amount.fract() == Decimal::ZERO)
    }

    async fn is_sanctioned_user(&self, user_id: Uuid) -> Result<bool, Status> {
        // Mock implementation - no users are sanctioned in this demo
        Ok(false)
    }

    async fn user_owns_bank_account(&self, user_id: Uuid, bank_account_id: Uuid) -> Result<bool, Status> {
        // Mock implementation - assume user owns the account
        Ok(true)
    }

    async fn get_hourly_transaction_count(&self, user_id: Uuid, tx_type: TransactionType) -> Result<i32, Status> {
        // Mock implementation
        Ok(2)
    }

    async fn get_recent_transaction_count(&self, user_id: Uuid, tx_type: TransactionType, minutes: i32) -> Result<i32, Status> {
        // Mock implementation
        Ok(1)
    }

    async fn get_transaction(&self, transaction_id: Uuid) -> Result<FiatTransaction, Status> {
        // Mock implementation - create a dummy transaction
        Ok(FiatTransaction::new(
            Uuid::new_v4(),
            Some(Uuid::new_v4()),
            TransactionType::Withdrawal,
            Decimal::from(1000),
            "USD".to_string(),
            crate::models::fiat_gateway::PaymentProvider::Ach,
            Some("Test transaction".to_string()),
        ))
    }
}

/// Fiat interceptor for gRPC services
pub struct FiatInterceptor {
    fiat_guard: Arc<FiatGuard>,
    require_kyc: bool,
}

impl FiatInterceptor {
    pub fn new(fiat_guard: Arc<FiatGuard>, require_kyc: bool) -> Self {
        Self {
            fiat_guard,
            require_kyc,
        }
    }

    pub async fn intercept<T>(&self, mut request: Request<T>) -> Result<Request<T>, Status> {
        if self.require_kyc {
            let auth_context = request.extensions().get::<AuthContext>()
                .ok_or_else(|| Status::unauthenticated("Authentication required"))?;

            // Basic KYC check for fiat operations
            self.fiat_guard.kyc_guard.require_kyc_approval(auth_context)?;
        }

        Ok(request)
    }
}

/// Risk scoring for fiat transactions
pub struct FiatRiskScorer {
    fiat_guard: Arc<FiatGuard>,
}

impl FiatRiskScorer {
    pub fn new(fiat_guard: Arc<FiatGuard>) -> Self {
        Self { fiat_guard }
    }

    /// Calculate risk score for a transaction
    pub async fn calculate_risk_score(
        &self,
        auth: &AuthContext,
        amount: Decimal,
        tx_type: TransactionType,
        currency: &str,
    ) -> Result<f64, Status> {
        let mut risk_score = 0.0;

        // Base risk by transaction type
        risk_score += match tx_type {
            TransactionType::Withdrawal => 0.3, // Withdrawals are riskier
            TransactionType::Deposit => 0.1,
        };

        // Amount-based risk
        if amount > Decimal::from(10000) {
            risk_score += 0.4;
        } else if amount > Decimal::from(5000) {
            risk_score += 0.2;
        }

        // Currency risk
        if currency != "USD" {
            risk_score += 0.2;
        }

        // KYC status risk
        let kyc_risk = self.fiat_guard.kyc_guard.calculate_risk_score(auth).await
            .unwrap_or(0.8);
        risk_score += kyc_risk * 0.3;

        // Time-based risk (transactions outside business hours)
        let now = chrono::Utc::now();
        let hour = now.hour();
        if hour < 6 || hour > 22 {
            risk_score += 0.1;
        }

        Ok(risk_score.min(1.0))
    }

    /// Determine if transaction requires manual review
    pub async fn requires_manual_review(
        &self,
        auth: &AuthContext,
        amount: Decimal,
        tx_type: TransactionType,
        currency: &str,
    ) -> Result<bool, Status> {
        let risk_score = self.calculate_risk_score(auth, amount, tx_type, currency).await?;
        
        // High risk threshold for manual review
        Ok(risk_score > 0.7)
    }
}
