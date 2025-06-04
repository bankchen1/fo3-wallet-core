//! Ledger service implementation

use std::sync::Arc;
use std::collections::HashMap;
use tonic::{Request, Response, Status};
use uuid::Uuid;
use rust_decimal::Decimal;
use chrono::{DateTime, Utc};

use crate::proto::fo3::wallet::v1::{
    ledger_service_server::LedgerService,
    *,
};
use crate::state::AppState;
use crate::middleware::{
    auth::{AuthContext, AuthService},
    audit::AuditLogger,
    ledger_guard::LedgerGuard,
};
use crate::models::ledger::{
    LedgerAccount, LedgerTransaction, JournalEntry, AccountBalance, TrialBalanceEntry,
    BalanceSheetSection, FinancialReport, AuditTrailEntry, LedgerMetrics,
    AccountType, AccountStatus, TransactionStatus, JournalEntryStatus, EntryType, ReportType,
    LedgerRepository, AccountReconciliation, ValidationIssue,
};
use crate::models::notifications::{
    NotificationType, NotificationPriority, DeliveryChannel
};

/// Ledger service implementation
#[derive(Debug)]
pub struct LedgerServiceImpl {
    state: Arc<AppState>,
    auth_service: Arc<AuthService>,
    audit_logger: Arc<AuditLogger>,
    ledger_guard: Arc<LedgerGuard>,
    ledger_repository: Arc<dyn LedgerRepository>,
}

impl LedgerServiceImpl {
    pub fn new(
        state: Arc<AppState>,
        auth_service: Arc<AuthService>,
        audit_logger: Arc<AuditLogger>,
        ledger_guard: Arc<LedgerGuard>,
        ledger_repository: Arc<dyn LedgerRepository>,
    ) -> Self {
        Self {
            state,
            auth_service,
            audit_logger,
            ledger_guard,
            ledger_repository,
        }
    }

    /// Generate unique reference number for transactions
    fn generate_reference_number() -> String {
        format!("TXN{}", Uuid::new_v4().to_string().replace('-', "").to_uppercase()[..12].to_string())
    }

    /// Convert proto account type to model
    fn proto_to_account_type(proto_type: i32) -> Result<AccountType, Status> {
        match AccountType::try_from(proto_type) {
            Ok(account_type::AccountTypeAsset) => Ok(AccountType::Asset),
            Ok(account_type::AccountTypeLiability) => Ok(AccountType::Liability),
            Ok(account_type::AccountTypeEquity) => Ok(AccountType::Equity),
            Ok(account_type::AccountTypeRevenue) => Ok(AccountType::Revenue),
            Ok(account_type::AccountTypeExpense) => Ok(AccountType::Expense),
            Ok(account_type::AccountTypeContraAsset) => Ok(AccountType::ContraAsset),
            Ok(account_type::AccountTypeContraLiability) => Ok(AccountType::ContraLiability),
            Ok(account_type::AccountTypeContraEquity) => Ok(AccountType::ContraEquity),
            _ => Err(Status::invalid_argument("Invalid account type")),
        }
    }

    /// Convert model account type to proto
    fn account_type_to_proto(account_type: &AccountType) -> i32 {
        match account_type {
            AccountType::Asset => account_type::AccountTypeAsset as i32,
            AccountType::Liability => account_type::AccountTypeLiability as i32,
            AccountType::Equity => account_type::AccountTypeEquity as i32,
            AccountType::Revenue => account_type::AccountTypeRevenue as i32,
            AccountType::Expense => account_type::AccountTypeExpense as i32,
            AccountType::ContraAsset => account_type::AccountTypeContraAsset as i32,
            AccountType::ContraLiability => account_type::AccountTypeContraLiability as i32,
            AccountType::ContraEquity => account_type::AccountTypeContraEquity as i32,
        }
    }

    /// Convert proto account status to model
    fn proto_to_account_status(proto_status: i32) -> Result<AccountStatus, Status> {
        match AccountStatus::try_from(proto_status) {
            Ok(account_status::AccountStatusActive) => Ok(AccountStatus::Active),
            Ok(account_status::AccountStatusInactive) => Ok(AccountStatus::Inactive),
            Ok(account_status::AccountStatusClosed) => Ok(AccountStatus::Closed),
            Ok(account_status::AccountStatusSuspended) => Ok(AccountStatus::Suspended),
            _ => Err(Status::invalid_argument("Invalid account status")),
        }
    }

    /// Convert model account status to proto
    fn account_status_to_proto(status: &AccountStatus) -> i32 {
        match status {
            AccountStatus::Active => account_status::AccountStatusActive as i32,
            AccountStatus::Inactive => account_status::AccountStatusInactive as i32,
            AccountStatus::Closed => account_status::AccountStatusClosed as i32,
            AccountStatus::Suspended => account_status::AccountStatusSuspended as i32,
        }
    }

    /// Convert proto transaction status to model
    fn proto_to_transaction_status(proto_status: i32) -> Result<TransactionStatus, Status> {
        match TransactionStatus::try_from(proto_status) {
            Ok(transaction_status::TransactionStatusPending) => Ok(TransactionStatus::Pending),
            Ok(transaction_status::TransactionStatusPosted) => Ok(TransactionStatus::Posted),
            Ok(transaction_status::TransactionStatusReversed) => Ok(TransactionStatus::Reversed),
            Ok(transaction_status::TransactionStatusFailed) => Ok(TransactionStatus::Failed),
            _ => Err(Status::invalid_argument("Invalid transaction status")),
        }
    }

    /// Convert model transaction status to proto
    fn transaction_status_to_proto(status: &TransactionStatus) -> i32 {
        match status {
            TransactionStatus::Pending => transaction_status::TransactionStatusPending as i32,
            TransactionStatus::Posted => transaction_status::TransactionStatusPosted as i32,
            TransactionStatus::Reversed => transaction_status::TransactionStatusReversed as i32,
            TransactionStatus::Failed => transaction_status::TransactionStatusFailed as i32,
        }
    }

    /// Convert proto entry type to model
    fn proto_to_entry_type(proto_type: i32) -> Result<EntryType, Status> {
        match EntryType::try_from(proto_type) {
            Ok(entry_type::EntryTypeDebit) => Ok(EntryType::Debit),
            Ok(entry_type::EntryTypeCredit) => Ok(EntryType::Credit),
            _ => Err(Status::invalid_argument("Invalid entry type")),
        }
    }

    /// Convert model entry type to proto
    fn entry_type_to_proto(entry_type: &EntryType) -> i32 {
        match entry_type {
            EntryType::Debit => entry_type::EntryTypeDebit as i32,
            EntryType::Credit => entry_type::EntryTypeCredit as i32,
        }
    }

    /// Convert proto journal entry status to model
    fn proto_to_journal_entry_status(proto_status: i32) -> Result<JournalEntryStatus, Status> {
        match JournalEntryStatus::try_from(proto_status) {
            Ok(journal_entry_status::JournalEntryStatusDraft) => Ok(JournalEntryStatus::Draft),
            Ok(journal_entry_status::JournalEntryStatusPosted) => Ok(JournalEntryStatus::Posted),
            Ok(journal_entry_status::JournalEntryStatusReversed) => Ok(JournalEntryStatus::Reversed),
            _ => Err(Status::invalid_argument("Invalid journal entry status")),
        }
    }

    /// Convert model journal entry status to proto
    fn journal_entry_status_to_proto(status: &JournalEntryStatus) -> i32 {
        match status {
            JournalEntryStatus::Draft => journal_entry_status::JournalEntryStatusDraft as i32,
            JournalEntryStatus::Posted => journal_entry_status::JournalEntryStatusPosted as i32,
            JournalEntryStatus::Reversed => journal_entry_status::JournalEntryStatusReversed as i32,
        }
    }

    /// Convert model ledger account to proto
    fn ledger_account_to_proto(&self, account: &LedgerAccount) -> crate::proto::fo3::wallet::v1::LedgerAccount {
        crate::proto::fo3::wallet::v1::LedgerAccount {
            id: account.id.to_string(),
            account_code: account.account_code.clone(),
            account_name: account.account_name.clone(),
            account_type: Self::account_type_to_proto(&account.account_type),
            status: Self::account_status_to_proto(&account.status),
            currency: account.currency.clone(),
            parent_account_id: account.parent_account_id.map(|id| id.to_string()).unwrap_or_default(),
            description: account.description.clone().unwrap_or_default(),
            is_system_account: account.is_system_account,
            allow_manual_entries: account.allow_manual_entries,
            current_balance: account.current_balance.to_string(),
            pending_balance: account.pending_balance.to_string(),
            metadata: account.metadata.clone(),
            created_at: account.created_at.to_rfc3339(),
            updated_at: account.updated_at.to_rfc3339(),
            closed_at: account.closed_at.map(|dt| dt.to_rfc3339()).unwrap_or_default(),
        }
    }

    /// Convert model journal entry to proto
    fn journal_entry_to_proto(&self, entry: &JournalEntry) -> crate::proto::fo3::wallet::v1::JournalEntry {
        crate::proto::fo3::wallet::v1::JournalEntry {
            id: entry.id.to_string(),
            transaction_id: entry.transaction_id.to_string(),
            account_id: entry.account_id.to_string(),
            entry_type: Self::entry_type_to_proto(&entry.entry_type),
            amount: entry.amount.to_string(),
            currency: entry.currency.clone(),
            description: entry.description.clone(),
            status: Self::journal_entry_status_to_proto(&entry.status),
            entry_sequence: entry.entry_sequence,
            metadata: entry.metadata.clone(),
            created_at: entry.created_at.to_rfc3339(),
            posted_at: entry.posted_at.map(|dt| dt.to_rfc3339()).unwrap_or_default(),
        }
    }

    /// Convert model ledger transaction to proto
    fn ledger_transaction_to_proto(&self, transaction: &LedgerTransaction) -> crate::proto::fo3::wallet::v1::LedgerTransaction {
        crate::proto::fo3::wallet::v1::LedgerTransaction {
            id: transaction.id.to_string(),
            reference_number: transaction.reference_number.clone(),
            status: Self::transaction_status_to_proto(&transaction.status),
            transaction_type: transaction.transaction_type.clone(),
            description: transaction.description.clone(),
            currency: transaction.currency.clone(),
            total_amount: transaction.total_amount.to_string(),
            entries: transaction.entries.iter().map(|e| self.journal_entry_to_proto(e)).collect(),
            source_service: transaction.source_service.clone().unwrap_or_default(),
            source_transaction_id: transaction.source_transaction_id.clone().unwrap_or_default(),
            metadata: transaction.metadata.clone(),
            created_at: transaction.created_at.to_rfc3339(),
            posted_at: transaction.posted_at.map(|dt| dt.to_rfc3339()).unwrap_or_default(),
            reversed_at: transaction.reversed_at.map(|dt| dt.to_rfc3339()).unwrap_or_default(),
            reversal_reason: transaction.reversal_reason.clone().unwrap_or_default(),
        }
    }

    /// Send ledger notification
    async fn send_ledger_notification(
        &self,
        user_id: &Uuid,
        notification_type: NotificationType,
        title: &str,
        message: &str,
        metadata: HashMap<String, String>,
    ) {
        if let Some(notification_service) = &self.state.notification_service {
            let _ = notification_service.send_notification(
                user_id,
                notification_type,
                NotificationPriority::Medium,
                title,
                message,
                Some(metadata),
                vec![DeliveryChannel::Push, DeliveryChannel::Email],
            ).await;
        }
    }

    /// Record ledger audit entry
    async fn record_audit_entry(
        &self,
        transaction_id: Option<Uuid>,
        account_id: Option<Uuid>,
        action: &str,
        old_value: Option<String>,
        new_value: Option<String>,
        user_id: &Uuid,
        ip_address: Option<String>,
    ) {
        let audit_entry = AuditTrailEntry {
            id: Uuid::new_v4(),
            transaction_id,
            account_id,
            action: action.to_string(),
            old_value,
            new_value,
            user_id: Some(*user_id),
            ip_address,
            user_agent: None,
            metadata: HashMap::new(),
            timestamp: Utc::now(),
        };

        let _ = self.ledger_repository.create_audit_entry(&audit_entry).await;
    }
}

#[tonic::async_trait]
impl LedgerService for LedgerServiceImpl {
    /// Create a new ledger account
    async fn create_ledger_account(
        &self,
        request: Request<CreateLedgerAccountRequest>,
    ) -> Result<Response<CreateLedgerAccountResponse>, Status> {
        let req = request.get_ref();

        // Parse and validate request
        let account_type = Self::proto_to_account_type(req.account_type)?;

        // Validate request with security guard
        let auth_context = self.ledger_guard
            .validate_account_creation(&request, &req.account_code, &account_type, &req.currency)
            .await?;

        // Create ledger account
        let account = LedgerAccount {
            id: Uuid::new_v4(),
            account_code: req.account_code.clone(),
            account_name: req.account_name.clone(),
            account_type,
            status: AccountStatus::Active,
            currency: req.currency.clone(),
            parent_account_id: if req.parent_account_id.is_empty() {
                None
            } else {
                Some(Uuid::parse_str(&req.parent_account_id)
                    .map_err(|_| Status::invalid_argument("Invalid parent account ID"))?)
            },
            description: if req.description.is_empty() { None } else { Some(req.description.clone()) },
            is_system_account: false,
            allow_manual_entries: req.allow_manual_entries,
            current_balance: Decimal::ZERO,
            pending_balance: Decimal::ZERO,
            metadata: req.metadata.clone(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            closed_at: None,
        };

        // Save to repository
        let created_account = self.ledger_repository
            .create_account(&account)
            .await
            .map_err(|e| Status::internal(format!("Failed to create account: {}", e)))?;

        // Record audit entry
        self.record_audit_entry(
            None,
            Some(created_account.id),
            "account_created",
            None,
            Some(serde_json::to_string(&created_account).unwrap_or_default()),
            &auth_context.user_id,
            request.remote_addr().map(|addr| addr.to_string()),
        ).await;

        // Log the operation
        self.audit_logger.log_operation(
            &auth_context.user_id.to_string(),
            "create_ledger_account",
            &format!("Created ledger account: {} ({})", created_account.account_code, created_account.account_name),
            true,
            request.remote_addr(),
        ).await;

        // Send notification
        self.send_ledger_notification(
            &auth_context.user_id,
            NotificationType::AccountCreated,
            "Ledger Account Created",
            &format!("Ledger account '{}' has been created successfully.", created_account.account_name),
            HashMap::from([
                ("account_id".to_string(), created_account.id.to_string()),
                ("account_code".to_string(), created_account.account_code.clone()),
            ]),
        ).await;

        Ok(Response::new(CreateLedgerAccountResponse {
            account: Some(self.ledger_account_to_proto(&created_account)),
        }))
    }

    /// Get a ledger account by ID
    async fn get_ledger_account(
        &self,
        request: Request<GetLedgerAccountRequest>,
    ) -> Result<Response<GetLedgerAccountResponse>, Status> {
        let req = request.get_ref();

        // Extract authentication
        let auth_context = self.auth_service.extract_auth(&request).await?;
        self.auth_service.check_permission(&auth_context, Permission::PermissionLedgerRead)?;

        // Parse account ID
        let account_id = Uuid::parse_str(&req.account_id)
            .map_err(|_| Status::invalid_argument("Invalid account ID"))?;

        // Get account
        let account = self.ledger_repository
            .get_account(&account_id)
            .await
            .map_err(|e| Status::internal(format!("Failed to get account: {}", e)))?;

        match account {
            Some(account) => {
                Ok(Response::new(GetLedgerAccountResponse {
                    account: Some(self.ledger_account_to_proto(&account)),
                }))
            }
            None => Err(Status::not_found("Account not found")),
        }
    }

    /// List ledger accounts
    async fn list_ledger_accounts(
        &self,
        request: Request<ListLedgerAccountsRequest>,
    ) -> Result<Response<ListLedgerAccountsResponse>, Status> {
        let req = request.get_ref();

        // Extract authentication
        let auth_context = self.auth_service.extract_auth(&request).await?;
        self.auth_service.check_permission(&auth_context, Permission::PermissionLedgerRead)?;

        // Parse filters
        let account_type = if req.account_type != 0 {
            Some(Self::proto_to_account_type(req.account_type)?)
        } else {
            None
        };

        let status = if req.status != 0 {
            Some(Self::proto_to_account_status(req.status)?)
        } else {
            None
        };

        let currency = if req.currency.is_empty() { None } else { Some(req.currency.clone()) };

        let parent_account_id = if req.parent_account_id.is_empty() {
            None
        } else {
            Some(Uuid::parse_str(&req.parent_account_id)
                .map_err(|_| Status::invalid_argument("Invalid parent account ID"))?)
        };

        let page = if req.page > 0 { req.page } else { 1 };
        let page_size = if req.page_size > 0 && req.page_size <= 100 { req.page_size } else { 20 };

        // Get accounts
        let (accounts, total_count) = self.ledger_repository
            .list_accounts(account_type, status, currency, parent_account_id, page, page_size)
            .await
            .map_err(|e| Status::internal(format!("Failed to list accounts: {}", e)))?;

        let proto_accounts = accounts.iter()
            .map(|account| self.ledger_account_to_proto(account))
            .collect();

        Ok(Response::new(ListLedgerAccountsResponse {
            accounts: proto_accounts,
            total_count: total_count as i32,
            page,
            page_size,
        }))
    }
