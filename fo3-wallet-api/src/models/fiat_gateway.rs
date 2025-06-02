//! Fiat Gateway data models and business logic

use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;
use rust_decimal::Decimal;

/// Payment provider types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaymentProvider {
    Ach,
    Visa,
    PayPal,
    Wire,
    Sepa,
}

impl From<PaymentProvider> for String {
    fn from(provider: PaymentProvider) -> Self {
        match provider {
            PaymentProvider::Ach => "ach".to_string(),
            PaymentProvider::Visa => "visa".to_string(),
            PaymentProvider::PayPal => "paypal".to_string(),
            PaymentProvider::Wire => "wire".to_string(),
            PaymentProvider::Sepa => "sepa".to_string(),
        }
    }
}

impl TryFrom<String> for PaymentProvider {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "ach" => Ok(PaymentProvider::Ach),
            "visa" => Ok(PaymentProvider::Visa),
            "paypal" => Ok(PaymentProvider::PayPal),
            "wire" => Ok(PaymentProvider::Wire),
            "sepa" => Ok(PaymentProvider::Sepa),
            _ => Err(format!("Invalid payment provider: {}", value)),
        }
    }
}

/// Account types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccountType {
    Checking,
    Savings,
    CreditCard,
    PayPal,
}

impl From<AccountType> for String {
    fn from(account_type: AccountType) -> Self {
        match account_type {
            AccountType::Checking => "checking".to_string(),
            AccountType::Savings => "savings".to_string(),
            AccountType::CreditCard => "credit_card".to_string(),
            AccountType::PayPal => "paypal".to_string(),
        }
    }
}

impl TryFrom<String> for AccountType {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "checking" => Ok(AccountType::Checking),
            "savings" => Ok(AccountType::Savings),
            "credit_card" => Ok(AccountType::CreditCard),
            "paypal" => Ok(AccountType::PayPal),
            _ => Err(format!("Invalid account type: {}", value)),
        }
    }
}

/// Transaction status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransactionStatus {
    Pending,
    Processing,
    Completed,
    Failed,
    Cancelled,
    RequiresApproval,
    Approved,
    Rejected,
}

impl From<TransactionStatus> for String {
    fn from(status: TransactionStatus) -> Self {
        match status {
            TransactionStatus::Pending => "pending".to_string(),
            TransactionStatus::Processing => "processing".to_string(),
            TransactionStatus::Completed => "completed".to_string(),
            TransactionStatus::Failed => "failed".to_string(),
            TransactionStatus::Cancelled => "cancelled".to_string(),
            TransactionStatus::RequiresApproval => "requires_approval".to_string(),
            TransactionStatus::Approved => "approved".to_string(),
            TransactionStatus::Rejected => "rejected".to_string(),
        }
    }
}

impl TryFrom<String> for TransactionStatus {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "pending" => Ok(TransactionStatus::Pending),
            "processing" => Ok(TransactionStatus::Processing),
            "completed" => Ok(TransactionStatus::Completed),
            "failed" => Ok(TransactionStatus::Failed),
            "cancelled" => Ok(TransactionStatus::Cancelled),
            "requires_approval" => Ok(TransactionStatus::RequiresApproval),
            "approved" => Ok(TransactionStatus::Approved),
            "rejected" => Ok(TransactionStatus::Rejected),
            _ => Err(format!("Invalid transaction status: {}", value)),
        }
    }
}

/// Transaction type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransactionType {
    Deposit,
    Withdrawal,
}

impl From<TransactionType> for String {
    fn from(tx_type: TransactionType) -> Self {
        match tx_type {
            TransactionType::Deposit => "deposit".to_string(),
            TransactionType::Withdrawal => "withdrawal".to_string(),
        }
    }
}

impl TryFrom<String> for TransactionType {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "deposit" => Ok(TransactionType::Deposit),
            "withdrawal" => Ok(TransactionType::Withdrawal),
            _ => Err(format!("Invalid transaction type: {}", value)),
        }
    }
}

/// Bank account information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BankAccount {
    pub id: Uuid,
    pub user_id: Uuid,
    pub provider: PaymentProvider,
    pub account_type: AccountType,
    pub account_name: String,
    pub encrypted_account_number: String,
    pub masked_account_number: String, // Last 4 digits only
    pub routing_number: Option<String>,
    pub bank_name: Option<String>,
    pub currency: String,
    pub country: String,
    pub is_verified: bool,
    pub is_primary: bool,
    pub verification_method: Option<String>,
    pub verification_data: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub verified_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

impl BankAccount {
    /// Create a new bank account
    pub fn new(
        user_id: Uuid,
        provider: PaymentProvider,
        account_type: AccountType,
        account_name: String,
        encrypted_account_number: String,
        masked_account_number: String,
        routing_number: Option<String>,
        bank_name: Option<String>,
        currency: String,
        country: String,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            user_id,
            provider,
            account_type,
            account_name,
            encrypted_account_number,
            masked_account_number,
            routing_number,
            bank_name,
            currency,
            country,
            is_verified: false,
            is_primary: false,
            verification_method: None,
            verification_data: None,
            created_at: now,
            verified_at: None,
            updated_at: now,
            deleted_at: None,
        }
    }

    /// Mark account as verified
    pub fn mark_verified(&mut self, verification_method: String) {
        self.is_verified = true;
        self.verification_method = Some(verification_method);
        self.verified_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    /// Set as primary account
    pub fn set_primary(&mut self, is_primary: bool) {
        self.is_primary = is_primary;
        self.updated_at = Utc::now();
    }

    /// Soft delete account
    pub fn soft_delete(&mut self) {
        self.deleted_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    /// Check if account is deleted
    pub fn is_deleted(&self) -> bool {
        self.deleted_at.is_some()
    }

    /// Check if account can be used for transactions
    pub fn is_usable(&self) -> bool {
        self.is_verified && !self.is_deleted()
    }
}

/// Fiat transaction record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FiatTransaction {
    pub id: Uuid,
    pub user_id: Uuid,
    pub bank_account_id: Option<Uuid>,
    pub transaction_type: TransactionType,
    pub status: TransactionStatus,
    pub amount: Decimal,
    pub currency: String,
    pub fee_amount: Decimal,
    pub net_amount: Decimal,
    pub provider: PaymentProvider,
    pub external_transaction_id: Option<String>,
    pub reference_number: Option<String>,
    pub description: Option<String>,
    pub failure_reason: Option<String>,
    pub approval_notes: Option<String>,
    pub approver_id: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
}

impl FiatTransaction {
    /// Create a new fiat transaction
    pub fn new(
        user_id: Uuid,
        bank_account_id: Option<Uuid>,
        transaction_type: TransactionType,
        amount: Decimal,
        currency: String,
        provider: PaymentProvider,
        description: Option<String>,
    ) -> Self {
        let now = Utc::now();
        let fee_amount = Self::calculate_fee(amount, &provider, transaction_type);
        let net_amount = if transaction_type == TransactionType::Withdrawal {
            amount - fee_amount
        } else {
            amount
        };

        Self {
            id: Uuid::new_v4(),
            user_id,
            bank_account_id,
            transaction_type,
            status: TransactionStatus::Pending,
            amount,
            currency,
            fee_amount,
            net_amount,
            provider,
            external_transaction_id: None,
            reference_number: Some(Self::generate_reference_number()),
            description,
            failure_reason: None,
            approval_notes: None,
            approver_id: None,
            metadata: None,
            created_at: now,
            updated_at: now,
            completed_at: None,
            expires_at: Some(now + chrono::Duration::hours(24)), // 24 hour expiry
        }
    }

    /// Calculate transaction fee based on provider and type
    fn calculate_fee(amount: Decimal, provider: &PaymentProvider, tx_type: TransactionType) -> Decimal {
        match (provider, tx_type) {
            (PaymentProvider::Ach, TransactionType::Withdrawal) => Decimal::from(1), // $1 ACH fee
            (PaymentProvider::Wire, TransactionType::Withdrawal) => Decimal::from(25), // $25 wire fee
            (PaymentProvider::Visa, _) => amount * Decimal::from_str("0.029").unwrap(), // 2.9% for card
            (PaymentProvider::PayPal, _) => amount * Decimal::from_str("0.034").unwrap(), // 3.4% for PayPal
            _ => Decimal::ZERO,
        }
    }

    /// Generate a human-readable reference number
    fn generate_reference_number() -> String {
        format!("FO3-{}", Uuid::new_v4().to_string()[..8].to_uppercase())
    }

    /// Update transaction status
    pub fn update_status(&mut self, status: TransactionStatus, reason: Option<String>) {
        self.status = status;
        self.updated_at = Utc::now();
        
        if status == TransactionStatus::Completed {
            self.completed_at = Some(Utc::now());
        } else if matches!(status, TransactionStatus::Failed | TransactionStatus::Rejected) {
            self.failure_reason = reason;
        }
    }

    /// Approve transaction (admin action)
    pub fn approve(&mut self, approver_id: String, notes: Option<String>) {
        self.status = TransactionStatus::Approved;
        self.approver_id = Some(approver_id);
        self.approval_notes = notes;
        self.updated_at = Utc::now();
    }

    /// Reject transaction (admin action)
    pub fn reject(&mut self, approver_id: String, reason: String, notes: Option<String>) {
        self.status = TransactionStatus::Rejected;
        self.approver_id = Some(approver_id);
        self.failure_reason = Some(reason);
        self.approval_notes = notes;
        self.updated_at = Utc::now();
    }

    /// Cancel transaction (user action)
    pub fn cancel(&mut self, reason: Option<String>) -> Result<(), String> {
        if !self.can_be_cancelled() {
            return Err("Transaction cannot be cancelled in current status".to_string());
        }
        
        self.status = TransactionStatus::Cancelled;
        self.failure_reason = reason;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Check if transaction can be cancelled
    pub fn can_be_cancelled(&self) -> bool {
        matches!(self.status, TransactionStatus::Pending | TransactionStatus::RequiresApproval)
    }

    /// Check if transaction is in final state
    pub fn is_final(&self) -> bool {
        matches!(
            self.status,
            TransactionStatus::Completed | TransactionStatus::Failed | 
            TransactionStatus::Cancelled | TransactionStatus::Rejected
        )
    }

    /// Check if transaction is expired
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            Utc::now() > expires_at && !self.is_final()
        } else {
            false
        }
    }
}

/// Transaction limits for a user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionLimits {
    pub id: Uuid,
    pub user_id: Uuid,
    pub currency: String,
    pub daily_deposit_limit: Decimal,
    pub daily_withdrawal_limit: Decimal,
    pub monthly_deposit_limit: Decimal,
    pub monthly_withdrawal_limit: Decimal,
    pub single_transaction_limit: Decimal,
    pub requires_approval_above: Decimal,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub updated_by: Option<String>,
}

impl TransactionLimits {
    /// Create default limits for a user
    pub fn default_for_user(user_id: Uuid, currency: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            user_id,
            currency,
            daily_deposit_limit: Decimal::from(10000), // $10,000
            daily_withdrawal_limit: Decimal::from(10000), // $10,000
            monthly_deposit_limit: Decimal::from(100000), // $100,000
            monthly_withdrawal_limit: Decimal::from(100000), // $100,000
            single_transaction_limit: Decimal::from(50000), // $50,000
            requires_approval_above: Decimal::from(10000), // $10,000
            created_at: now,
            updated_at: now,
            updated_by: None,
        }
    }

    /// Update limits (admin action)
    pub fn update_limits(
        &mut self,
        daily_deposit: Option<Decimal>,
        daily_withdrawal: Option<Decimal>,
        monthly_deposit: Option<Decimal>,
        monthly_withdrawal: Option<Decimal>,
        single_transaction: Option<Decimal>,
        approval_threshold: Option<Decimal>,
        updated_by: String,
    ) {
        if let Some(limit) = daily_deposit {
            self.daily_deposit_limit = limit;
        }
        if let Some(limit) = daily_withdrawal {
            self.daily_withdrawal_limit = limit;
        }
        if let Some(limit) = monthly_deposit {
            self.monthly_deposit_limit = limit;
        }
        if let Some(limit) = monthly_withdrawal {
            self.monthly_withdrawal_limit = limit;
        }
        if let Some(limit) = single_transaction {
            self.single_transaction_limit = limit;
        }
        if let Some(threshold) = approval_threshold {
            self.requires_approval_above = threshold;
        }

        self.updated_by = Some(updated_by);
        self.updated_at = Utc::now();
    }

    /// Check if transaction amount requires approval
    pub fn requires_approval(&self, amount: Decimal) -> bool {
        amount > self.requires_approval_above
    }

    /// Validate transaction against limits
    pub fn validate_transaction(
        &self,
        amount: Decimal,
        tx_type: TransactionType,
        daily_usage: Decimal,
        monthly_usage: Decimal,
    ) -> Result<(), String> {
        // Check single transaction limit
        if amount > self.single_transaction_limit {
            return Err(format!(
                "Transaction amount {} exceeds single transaction limit of {}",
                amount, self.single_transaction_limit
            ));
        }

        // Check daily limits
        let daily_limit = match tx_type {
            TransactionType::Deposit => self.daily_deposit_limit,
            TransactionType::Withdrawal => self.daily_withdrawal_limit,
        };

        if daily_usage + amount > daily_limit {
            return Err(format!(
                "Transaction would exceed daily {} limit of {} (current usage: {})",
                tx_type.to_string(), daily_limit, daily_usage
            ));
        }

        // Check monthly limits
        let monthly_limit = match tx_type {
            TransactionType::Deposit => self.monthly_deposit_limit,
            TransactionType::Withdrawal => self.monthly_withdrawal_limit,
        };

        if monthly_usage + amount > monthly_limit {
            return Err(format!(
                "Transaction would exceed monthly {} limit of {} (current usage: {})",
                tx_type.to_string(), monthly_limit, monthly_usage
            ));
        }

        Ok(())
    }
}

/// Fiat gateway repository trait for database operations
pub trait FiatGatewayRepository {
    type Error;

    // Bank account operations
    async fn create_bank_account(&self, account: &BankAccount) -> Result<(), Self::Error>;
    async fn get_bank_account_by_id(&self, id: Uuid) -> Result<Option<BankAccount>, Self::Error>;
    async fn get_bank_accounts_by_user(&self, user_id: Uuid, verified_only: bool) -> Result<Vec<BankAccount>, Self::Error>;
    async fn update_bank_account(&self, account: &BankAccount) -> Result<(), Self::Error>;
    async fn delete_bank_account(&self, id: Uuid) -> Result<(), Self::Error>;

    // Transaction operations
    async fn create_transaction(&self, transaction: &FiatTransaction) -> Result<(), Self::Error>;
    async fn get_transaction_by_id(&self, id: Uuid) -> Result<Option<FiatTransaction>, Self::Error>;
    async fn get_transaction_by_external_id(&self, external_id: &str) -> Result<Option<FiatTransaction>, Self::Error>;
    async fn update_transaction(&self, transaction: &FiatTransaction) -> Result<(), Self::Error>;
    async fn list_transactions(
        &self,
        user_id: Option<Uuid>,
        tx_type: Option<TransactionType>,
        status: Option<TransactionStatus>,
        page_size: i32,
        page_token: Option<String>,
        start_date: Option<DateTime<Utc>>,
        end_date: Option<DateTime<Utc>>,
    ) -> Result<(Vec<FiatTransaction>, Option<String>), Self::Error>;

    // Limits operations
    async fn get_transaction_limits(&self, user_id: Uuid, currency: &str) -> Result<Option<TransactionLimits>, Self::Error>;
    async fn create_transaction_limits(&self, limits: &TransactionLimits) -> Result<(), Self::Error>;
    async fn update_transaction_limits(&self, limits: &TransactionLimits) -> Result<(), Self::Error>;

    // Usage calculations
    async fn get_daily_usage(&self, user_id: Uuid, tx_type: TransactionType, currency: &str) -> Result<Decimal, Self::Error>;
    async fn get_monthly_usage(&self, user_id: Uuid, tx_type: TransactionType, currency: &str) -> Result<Decimal, Self::Error>;
}
