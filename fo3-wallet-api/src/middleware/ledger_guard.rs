//! Ledger security guard middleware

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
use crate::models::ledger::{
    LedgerRepository, AccountType, TransactionStatus, JournalEntry, EntryType,
};
use crate::proto::fo3::wallet::v1::{Permission, UserRole};

/// Ledger security guard for validation and compliance enforcement
#[derive(Debug)]
pub struct LedgerGuard {
    auth_service: Arc<AuthService>,
    audit_logger: Arc<AuditLogger>,
    rate_limiter: Arc<RateLimiter>,
    ledger_repository: Arc<dyn LedgerRepository>,
}

impl LedgerGuard {
    pub fn new(
        auth_service: Arc<AuthService>,
        audit_logger: Arc<AuditLogger>,
        rate_limiter: Arc<RateLimiter>,
        ledger_repository: Arc<dyn LedgerRepository>,
    ) -> Self {
        Self {
            auth_service,
            audit_logger,
            rate_limiter,
            ledger_repository,
        }
    }

    /// Validate account creation
    pub async fn validate_account_creation<T>(
        &self,
        request: &Request<T>,
        account_code: &str,
        account_type: &AccountType,
        currency: &str,
    ) -> Result<AuthContext, Status> {
        // Extract and validate authentication
        let auth_context = self.auth_service.extract_auth(request).await?;

        // Check basic permissions
        self.auth_service.check_permission(&auth_context, Permission::PermissionLedgerManagement)?;

        // Rate limiting for account creation
        let rate_key = format!("account_creation:{}", auth_context.user_id);
        if !self.rate_limiter.check_rate_limit(&rate_key, 10, Duration::minutes(15)).await {
            self.audit_logger.log_security_event(
                &auth_context.user_id.to_string(),
                "account_creation_rate_limit",
                &format!("Rate limit exceeded for account creation: {}", account_code),
                request.remote_addr(),
            ).await;
            return Err(Status::resource_exhausted("Too many account creation attempts"));
        }

        // Validate account code format
        self.validate_account_code(account_code)?;

        // Check for duplicate account codes
        if let Ok(Some(_)) = self.ledger_repository.get_account_by_code(account_code).await {
            return Err(Status::already_exists(format!("Account code '{}' already exists", account_code)));
        }

        // Validate account type and currency combination
        self.validate_account_type_currency(account_type, currency)?;

        // Log the validation
        self.audit_logger.log_operation(
            &auth_context.user_id.to_string(),
            "account_creation_validation",
            &format!("Validated account creation: code={}, type={:?}, currency={}", account_code, account_type, currency),
            true,
            request.remote_addr(),
        ).await;

        Ok(auth_context)
    }

    /// Validate transaction recording
    pub async fn validate_transaction_recording<T>(
        &self,
        request: &Request<T>,
        transaction_type: &str,
        entries: &[JournalEntry],
        total_amount: &Decimal,
    ) -> Result<AuthContext, Status> {
        // Extract and validate authentication
        let auth_context = self.auth_service.extract_auth(request).await?;

        // Check basic permissions
        self.auth_service.check_permission(&auth_context, Permission::PermissionLedgerWrite)?;

        // Rate limiting for transaction recording
        let rate_key = format!("transaction_recording:{}", auth_context.user_id);
        if !self.rate_limiter.check_rate_limit(&rate_key, 50, Duration::minutes(5)).await {
            self.audit_logger.log_security_event(
                &auth_context.user_id.to_string(),
                "transaction_recording_rate_limit",
                &format!("Rate limit exceeded for transaction recording: {}", transaction_type),
                request.remote_addr(),
            ).await;
            return Err(Status::resource_exhausted("Too many transaction recording attempts"));
        }

        // Validate double-entry bookkeeping
        self.validate_double_entry_rules(entries)?;

        // Validate transaction amount
        self.validate_transaction_amount(total_amount)?;

        // Validate all accounts exist and are active
        self.validate_account_accessibility(entries).await?;

        // Check for suspicious patterns
        self.check_suspicious_transaction_patterns(&auth_context.user_id, entries, total_amount).await?;

        // Log the validation
        self.audit_logger.log_operation(
            &auth_context.user_id.to_string(),
            "transaction_recording_validation",
            &format!("Validated transaction recording: type={}, entries={}, amount={}", 
                transaction_type, entries.len(), total_amount),
            true,
            request.remote_addr(),
        ).await;

        Ok(auth_context)
    }

    /// Validate journal entry posting
    pub async fn validate_journal_entry_posting<T>(
        &self,
        request: &Request<T>,
        entry_id: &Uuid,
    ) -> Result<AuthContext, Status> {
        // Extract and validate authentication
        let auth_context = self.auth_service.extract_auth(request).await?;

        // Check posting permissions
        self.auth_service.check_permission(&auth_context, Permission::PermissionLedgerPost)?;

        // Rate limiting for posting operations
        let rate_key = format!("journal_posting:{}", auth_context.user_id);
        if !self.rate_limiter.check_rate_limit(&rate_key, 100, Duration::minutes(5)).await {
            self.audit_logger.log_security_event(
                &auth_context.user_id.to_string(),
                "journal_posting_rate_limit",
                &format!("Rate limit exceeded for journal posting: {}", entry_id),
                request.remote_addr(),
            ).await;
            return Err(Status::resource_exhausted("Too many journal posting attempts"));
        }

        // Validate entry exists and is in draft status
        let entry = self.ledger_repository
            .get_journal_entry(entry_id)
            .await
            .map_err(|e| Status::internal(format!("Failed to get journal entry: {}", e)))?
            .ok_or_else(|| Status::not_found("Journal entry not found"))?;

        if entry.status != crate::models::ledger::JournalEntryStatus::Draft {
            return Err(Status::failed_precondition("Only draft entries can be posted"));
        }

        // Log the validation
        self.audit_logger.log_operation(
            &auth_context.user_id.to_string(),
            "journal_posting_validation",
            &format!("Validated journal entry posting: {}", entry_id),
            true,
            request.remote_addr(),
        ).await;

        Ok(auth_context)
    }

    /// Validate transaction reversal
    pub async fn validate_transaction_reversal<T>(
        &self,
        request: &Request<T>,
        transaction_id: &Uuid,
        reason: &str,
    ) -> Result<AuthContext, Status> {
        // Extract and validate authentication
        let auth_context = self.auth_service.extract_auth(request).await?;

        // Check reversal permissions (higher privilege required)
        self.auth_service.check_permission(&auth_context, Permission::PermissionLedgerReverse)?;

        // Enhanced rate limiting for reversals
        let rate_key = format!("transaction_reversal:{}", auth_context.user_id);
        if !self.rate_limiter.check_rate_limit(&rate_key, 5, Duration::minutes(15)).await {
            self.audit_logger.log_security_event(
                &auth_context.user_id.to_string(),
                "transaction_reversal_rate_limit",
                &format!("Rate limit exceeded for transaction reversal: {}", transaction_id),
                request.remote_addr(),
            ).await;
            return Err(Status::resource_exhausted("Too many transaction reversal attempts"));
        }

        // Validate transaction exists and can be reversed
        let transaction = self.ledger_repository
            .get_transaction(transaction_id)
            .await
            .map_err(|e| Status::internal(format!("Failed to get transaction: {}", e)))?
            .ok_or_else(|| Status::not_found("Transaction not found"))?;

        if transaction.status != TransactionStatus::Posted {
            return Err(Status::failed_precondition("Only posted transactions can be reversed"));
        }

        // Validate reversal reason
        if reason.trim().is_empty() {
            return Err(Status::invalid_argument("Reversal reason is required"));
        }

        if reason.len() < 10 {
            return Err(Status::invalid_argument("Reversal reason must be at least 10 characters"));
        }

        // Log the validation
        self.audit_logger.log_operation(
            &auth_context.user_id.to_string(),
            "transaction_reversal_validation",
            &format!("Validated transaction reversal: {} (reason: {})", transaction_id, reason),
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
        target_data: Option<&str>,
    ) -> Result<AuthContext, Status> {
        // Extract and validate authentication
        let auth_context = self.auth_service.extract_auth(request).await?;

        // Check admin permissions
        if auth_context.role != UserRole::UserRoleAdmin && auth_context.role != UserRole::UserRoleSuperAdmin {
            return Err(Status::permission_denied("Admin access required"));
        }

        // Enhanced rate limiting for admin operations
        let rate_key = format!("ledger_admin_operation:{}", auth_context.user_id);
        if !self.rate_limiter.check_rate_limit(&rate_key, 20, Duration::minutes(10)).await {
            self.audit_logger.log_security_event(
                &auth_context.user_id.to_string(),
                "ledger_admin_rate_limit",
                &format!("Rate limit exceeded for admin operation: {}", operation),
                request.remote_addr(),
            ).await;
            return Err(Status::resource_exhausted("Too many admin operation attempts"));
        }

        // Log admin operation
        let target_info = target_data.unwrap_or("N/A");
        self.audit_logger.log_operation(
            &auth_context.user_id.to_string(),
            "ledger_admin_operation",
            &format!("Admin operation: {} for: {}", operation, target_info),
            true,
            request.remote_addr(),
        ).await;

        Ok(auth_context)
    }

    /// Validate account code format
    fn validate_account_code(&self, account_code: &str) -> Result<(), Status> {
        if account_code.is_empty() {
            return Err(Status::invalid_argument("Account code cannot be empty"));
        }

        if account_code.len() < 3 || account_code.len() > 20 {
            return Err(Status::invalid_argument("Account code must be between 3 and 20 characters"));
        }

        // Account code should be alphanumeric with optional hyphens/underscores
        if !account_code.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
            return Err(Status::invalid_argument("Account code can only contain alphanumeric characters, hyphens, and underscores"));
        }

        Ok(())
    }

    /// Validate account type and currency combination
    fn validate_account_type_currency(&self, account_type: &AccountType, currency: &str) -> Result<(), Status> {
        // Validate currency format
        if currency.len() != 3 {
            return Err(Status::invalid_argument("Currency must be a 3-letter code"));
        }

        if !currency.chars().all(|c| c.is_ascii_uppercase()) {
            return Err(Status::invalid_argument("Currency must be uppercase"));
        }

        // Validate supported currencies
        let supported_currencies = ["USD", "EUR", "GBP", "CAD", "AUD", "JPY", "CHF", "CNY"];
        if !supported_currencies.contains(&currency) {
            return Err(Status::invalid_argument(format!("Unsupported currency: {}", currency)));
        }

        Ok(())
    }

    /// Validate double-entry bookkeeping rules
    fn validate_double_entry_rules(&self, entries: &[JournalEntry]) -> Result<(), Status> {
        if entries.is_empty() {
            return Err(Status::invalid_argument("Transaction must have at least one journal entry"));
        }

        if entries.len() < 2 {
            return Err(Status::invalid_argument("Double-entry bookkeeping requires at least two entries"));
        }

        let mut debit_total = Decimal::ZERO;
        let mut credit_total = Decimal::ZERO;

        for entry in entries {
            if entry.amount <= Decimal::ZERO {
                return Err(Status::invalid_argument("Journal entry amounts must be positive"));
            }

            match entry.entry_type {
                EntryType::Debit => debit_total += entry.amount,
                EntryType::Credit => credit_total += entry.amount,
            }
        }

        if debit_total != credit_total {
            return Err(Status::invalid_argument(
                format!("Double-entry validation failed: debits ({}) != credits ({})", debit_total, credit_total)
            ));
        }

        Ok(())
    }

    /// Validate transaction amount
    fn validate_transaction_amount(&self, amount: &Decimal) -> Result<(), Status> {
        if *amount <= Decimal::ZERO {
            return Err(Status::invalid_argument("Transaction amount must be positive"));
        }

        // Maximum transaction amount for security
        let max_amount = Decimal::from(1_000_000); // $1M limit
        if *amount > max_amount {
            return Err(Status::invalid_argument(
                format!("Transaction amount exceeds maximum allowed: {}", max_amount)
            ));
        }

        Ok(())
    }

    /// Validate account accessibility
    async fn validate_account_accessibility(&self, entries: &[JournalEntry]) -> Result<(), Status> {
        for entry in entries {
            let account = self.ledger_repository
                .get_account(&entry.account_id)
                .await
                .map_err(|e| Status::internal(format!("Failed to get account: {}", e)))?
                .ok_or_else(|| Status::not_found(format!("Account not found: {}", entry.account_id)))?;

            if account.status != crate::models::ledger::AccountStatus::Active {
                return Err(Status::failed_precondition(
                    format!("Account {} is not active: {:?}", account.account_code, account.status)
                ));
            }

            // Check if manual entries are allowed for this account
            if !account.allow_manual_entries && !account.is_system_account {
                return Err(Status::failed_precondition(
                    format!("Manual entries not allowed for account: {}", account.account_code)
                ));
            }
        }

        Ok(())
    }

    /// Check for suspicious transaction patterns
    async fn check_suspicious_transaction_patterns(
        &self,
        user_id: &Uuid,
        entries: &[JournalEntry],
        total_amount: &Decimal,
    ) -> Result<(), Status> {
        // Check for unusually large transactions
        let large_transaction_threshold = Decimal::from(100_000); // $100k
        if *total_amount > large_transaction_threshold {
            // Log for review but don't block
            // In a real implementation, this might trigger additional approval workflows
        }

        // Check for rapid successive large transactions
        let now = Utc::now();
        let one_hour_ago = now - Duration::hours(1);
        
        // Get recent transactions for this user (simplified check)
        if let Ok((recent_transactions, _)) = self.ledger_repository
            .list_transactions(None, None, None, None, Some(one_hour_ago), Some(now), None, 1, 100)
            .await
        {
            let recent_large_count = recent_transactions
                .iter()
                .filter(|tx| tx.total_amount > Decimal::from(50_000))
                .count();

            if recent_large_count >= 5 {
                return Err(Status::failed_precondition(
                    "Too many large transactions in recent period - manual review required"
                ));
            }
        }

        Ok(())
    }
}
