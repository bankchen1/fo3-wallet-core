//! Ledger data models for double-entry bookkeeping

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use rust_decimal::Decimal;
use chrono::{DateTime, Utc, NaiveDate};

/// Account types in the chart of accounts
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccountType {
    Asset,
    Liability,
    Equity,
    Revenue,
    Expense,
    ContraAsset,
    ContraLiability,
    ContraEquity,
}

impl std::fmt::Display for AccountType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AccountType::Asset => write!(f, "asset"),
            AccountType::Liability => write!(f, "liability"),
            AccountType::Equity => write!(f, "equity"),
            AccountType::Revenue => write!(f, "revenue"),
            AccountType::Expense => write!(f, "expense"),
            AccountType::ContraAsset => write!(f, "contra_asset"),
            AccountType::ContraLiability => write!(f, "contra_liability"),
            AccountType::ContraEquity => write!(f, "contra_equity"),
        }
    }
}

impl std::str::FromStr for AccountType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "asset" => Ok(AccountType::Asset),
            "liability" => Ok(AccountType::Liability),
            "equity" => Ok(AccountType::Equity),
            "revenue" => Ok(AccountType::Revenue),
            "expense" => Ok(AccountType::Expense),
            "contra_asset" => Ok(AccountType::ContraAsset),
            "contra_liability" => Ok(AccountType::ContraLiability),
            "contra_equity" => Ok(AccountType::ContraEquity),
            _ => Err(format!("Invalid account type: {}", s)),
        }
    }
}

/// Account status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccountStatus {
    Active,
    Inactive,
    Closed,
    Suspended,
}

impl std::fmt::Display for AccountStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AccountStatus::Active => write!(f, "active"),
            AccountStatus::Inactive => write!(f, "inactive"),
            AccountStatus::Closed => write!(f, "closed"),
            AccountStatus::Suspended => write!(f, "suspended"),
        }
    }
}

impl std::str::FromStr for AccountStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "active" => Ok(AccountStatus::Active),
            "inactive" => Ok(AccountStatus::Inactive),
            "closed" => Ok(AccountStatus::Closed),
            "suspended" => Ok(AccountStatus::Suspended),
            _ => Err(format!("Invalid account status: {}", s)),
        }
    }
}

/// Transaction status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransactionStatus {
    Pending,
    Posted,
    Reversed,
    Failed,
}

impl std::fmt::Display for TransactionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransactionStatus::Pending => write!(f, "pending"),
            TransactionStatus::Posted => write!(f, "posted"),
            TransactionStatus::Reversed => write!(f, "reversed"),
            TransactionStatus::Failed => write!(f, "failed"),
        }
    }
}

impl std::str::FromStr for TransactionStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pending" => Ok(TransactionStatus::Pending),
            "posted" => Ok(TransactionStatus::Posted),
            "reversed" => Ok(TransactionStatus::Reversed),
            "failed" => Ok(TransactionStatus::Failed),
            _ => Err(format!("Invalid transaction status: {}", s)),
        }
    }
}

/// Journal entry status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum JournalEntryStatus {
    Draft,
    Posted,
    Reversed,
}

impl std::fmt::Display for JournalEntryStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JournalEntryStatus::Draft => write!(f, "draft"),
            JournalEntryStatus::Posted => write!(f, "posted"),
            JournalEntryStatus::Reversed => write!(f, "reversed"),
        }
    }
}

impl std::str::FromStr for JournalEntryStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "draft" => Ok(JournalEntryStatus::Draft),
            "posted" => Ok(JournalEntryStatus::Posted),
            "reversed" => Ok(JournalEntryStatus::Reversed),
            _ => Err(format!("Invalid journal entry status: {}", s)),
        }
    }
}

/// Entry type (debit or credit)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntryType {
    Debit,
    Credit,
}

impl std::fmt::Display for EntryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EntryType::Debit => write!(f, "debit"),
            EntryType::Credit => write!(f, "credit"),
        }
    }
}

impl std::str::FromStr for EntryType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "debit" => Ok(EntryType::Debit),
            "credit" => Ok(EntryType::Credit),
            _ => Err(format!("Invalid entry type: {}", s)),
        }
    }
}

/// Financial report types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReportType {
    BalanceSheet,
    IncomeStatement,
    CashFlow,
    TrialBalance,
    GeneralLedger,
}

impl std::fmt::Display for ReportType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReportType::BalanceSheet => write!(f, "balance_sheet"),
            ReportType::IncomeStatement => write!(f, "income_statement"),
            ReportType::CashFlow => write!(f, "cash_flow"),
            ReportType::TrialBalance => write!(f, "trial_balance"),
            ReportType::GeneralLedger => write!(f, "general_ledger"),
        }
    }
}

/// Ledger account entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerAccount {
    pub id: Uuid,
    pub account_code: String,
    pub account_name: String,
    pub account_type: AccountType,
    pub status: AccountStatus,
    pub currency: String,
    pub parent_account_id: Option<Uuid>,
    pub description: Option<String>,
    pub is_system_account: bool,
    pub allow_manual_entries: bool,
    pub current_balance: Decimal,
    pub pending_balance: Decimal,
    pub metadata: HashMap<String, String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
}

/// Ledger transaction entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerTransaction {
    pub id: Uuid,
    pub reference_number: String,
    pub status: TransactionStatus,
    pub transaction_type: String,
    pub description: String,
    pub currency: String,
    pub total_amount: Decimal,
    pub entries: Vec<JournalEntry>,
    pub source_service: Option<String>,
    pub source_transaction_id: Option<String>,
    pub metadata: HashMap<String, String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub posted_at: Option<DateTime<Utc>>,
    pub reversed_at: Option<DateTime<Utc>>,
    pub reversal_reason: Option<String>,
    pub reversal_transaction_id: Option<Uuid>,
}

/// Journal entry entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalEntry {
    pub id: Uuid,
    pub transaction_id: Uuid,
    pub account_id: Uuid,
    pub entry_type: EntryType,
    pub amount: Decimal,
    pub currency: String,
    pub description: String,
    pub status: JournalEntryStatus,
    pub entry_sequence: i32,
    pub metadata: HashMap<String, String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub posted_at: Option<DateTime<Utc>>,
}

/// Account balance information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountBalance {
    pub account_id: Uuid,
    pub account_code: String,
    pub account_name: String,
    pub account_type: AccountType,
    pub currency: String,
    pub current_balance: Decimal,
    pub pending_balance: Decimal,
    pub available_balance: Decimal,
    pub last_transaction_date: Option<DateTime<Utc>>,
    pub transaction_count: i64,
}

/// Trial balance entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrialBalanceEntry {
    pub account_id: Uuid,
    pub account_code: String,
    pub account_name: String,
    pub account_type: AccountType,
    pub debit_balance: Decimal,
    pub credit_balance: Decimal,
    pub net_balance: Decimal,
}

/// Balance sheet item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceSheetItem {
    pub account_id: Uuid,
    pub account_code: String,
    pub account_name: String,
    pub balance: Decimal,
    pub sub_items: Vec<BalanceSheetItem>,
}

/// Balance sheet section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceSheetSection {
    pub section_name: String,
    pub items: Vec<BalanceSheetItem>,
    pub section_total: Decimal,
}

/// Financial report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinancialReport {
    pub id: Uuid,
    pub report_type: ReportType,
    pub title: String,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub currency: String,
    pub sections: Vec<BalanceSheetSection>,
    pub summary: HashMap<String, String>,
    pub generated_at: DateTime<Utc>,
    pub generated_by: Uuid,
}

/// Audit trail entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditTrailEntry {
    pub id: Uuid,
    pub transaction_id: Option<Uuid>,
    pub account_id: Option<Uuid>,
    pub action: String,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
    pub user_id: Option<Uuid>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub metadata: HashMap<String, String>,
    pub timestamp: DateTime<Utc>,
}

/// Account reconciliation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountReconciliation {
    pub account_id: Uuid,
    pub expected_balance: Decimal,
    pub actual_balance: Decimal,
    pub difference: Decimal,
    pub balanced: bool,
    pub issues: Vec<String>,
}

/// Validation issue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationIssue {
    pub issue_type: String,
    pub description: String,
    pub account_id: Option<Uuid>,
    pub transaction_id: Option<Uuid>,
    pub severity: String,
    pub fixed: bool,
    pub fix_description: Option<String>,
}

/// Ledger metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerMetrics {
    pub total_accounts: i64,
    pub active_accounts: i64,
    pub total_transactions: i64,
    pub pending_transactions: i64,
    pub total_assets: Decimal,
    pub total_liabilities: Decimal,
    pub total_equity: Decimal,
    pub books_balanced: bool,
    pub last_reconciliation: Option<DateTime<Utc>>,
    pub currency_balances: HashMap<String, Decimal>,
}

/// Account balance snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountBalanceSnapshot {
    pub id: Uuid,
    pub account_id: Uuid,
    pub balance_date: NaiveDate,
    pub opening_balance: Decimal,
    pub closing_balance: Decimal,
    pub debit_total: Decimal,
    pub credit_total: Decimal,
    pub transaction_count: i32,
    pub currency: String,
    pub created_at: DateTime<Utc>,
}

/// Repository trait for ledger operations
#[async_trait::async_trait]
pub trait LedgerRepository: Send + Sync {
    // Account operations
    async fn create_account(&self, account: &LedgerAccount) -> Result<LedgerAccount, String>;
    async fn get_account(&self, id: &Uuid) -> Result<Option<LedgerAccount>, String>;
    async fn get_account_by_code(&self, code: &str) -> Result<Option<LedgerAccount>, String>;
    async fn list_accounts(&self, account_type: Option<AccountType>, status: Option<AccountStatus>, currency: Option<String>, parent_id: Option<Uuid>, page: i32, page_size: i32) -> Result<(Vec<LedgerAccount>, i64), String>;
    async fn update_account(&self, account: &LedgerAccount) -> Result<LedgerAccount, String>;
    async fn close_account(&self, id: &Uuid, reason: &str) -> Result<LedgerAccount, String>;

    // Transaction operations
    async fn create_transaction(&self, transaction: &LedgerTransaction) -> Result<LedgerTransaction, String>;
    async fn get_transaction(&self, id: &Uuid) -> Result<Option<LedgerTransaction>, String>;
    async fn get_transaction_by_reference(&self, reference: &str) -> Result<Option<LedgerTransaction>, String>;
    async fn list_transactions(&self, account_id: Option<Uuid>, status: Option<TransactionStatus>, transaction_type: Option<String>, currency: Option<String>, start_date: Option<DateTime<Utc>>, end_date: Option<DateTime<Utc>>, source_service: Option<String>, page: i32, page_size: i32) -> Result<(Vec<LedgerTransaction>, i64), String>;
    async fn update_transaction(&self, transaction: &LedgerTransaction) -> Result<LedgerTransaction, String>;
    async fn post_transaction(&self, id: &Uuid) -> Result<LedgerTransaction, String>;
    async fn reverse_transaction(&self, id: &Uuid, reason: &str, description: &str) -> Result<(LedgerTransaction, LedgerTransaction), String>;

    // Journal entry operations
    async fn create_journal_entries(&self, entries: &[JournalEntry]) -> Result<Vec<JournalEntry>, String>;
    async fn get_journal_entry(&self, id: &Uuid) -> Result<Option<JournalEntry>, String>;
    async fn list_journal_entries(&self, transaction_id: Option<Uuid>, account_id: Option<Uuid>, status: Option<JournalEntryStatus>, start_date: Option<DateTime<Utc>>, end_date: Option<DateTime<Utc>>, page: i32, page_size: i32) -> Result<(Vec<JournalEntry>, i64), String>;
    async fn post_journal_entry(&self, id: &Uuid, posting_date: Option<DateTime<Utc>>) -> Result<JournalEntry, String>;

    // Balance operations
    async fn get_account_balance(&self, account_id: &Uuid, as_of_date: Option<DateTime<Utc>>, include_pending: bool) -> Result<Option<AccountBalance>, String>;
    async fn get_trial_balance(&self, as_of_date: Option<DateTime<Utc>>, currency: Option<String>, account_type: Option<AccountType>, include_zero_balances: bool) -> Result<Vec<TrialBalanceEntry>, String>;
    async fn update_account_balance(&self, account_id: &Uuid, amount: Decimal, entry_type: EntryType) -> Result<(), String>;

    // Reconciliation operations
    async fn reconcile_accounts(&self, account_ids: &[Uuid], reconciliation_date: DateTime<Utc>, auto_correct: bool) -> Result<Vec<AccountReconciliation>, String>;
    async fn validate_bookkeeping(&self, start_date: Option<DateTime<Utc>>, end_date: Option<DateTime<Utc>>, account_ids: Option<Vec<Uuid>>, fix_issues: bool) -> Result<(bool, Vec<ValidationIssue>), String>;

    // Audit operations
    async fn create_audit_entry(&self, entry: &AuditTrailEntry) -> Result<AuditTrailEntry, String>;
    async fn get_audit_trail(&self, account_id: Option<Uuid>, transaction_id: Option<Uuid>, user_id: Option<Uuid>, action: Option<String>, start_date: Option<DateTime<Utc>>, end_date: Option<DateTime<Utc>>, page: i32, page_size: i32) -> Result<(Vec<AuditTrailEntry>, i64), String>;

    // Reporting operations
    async fn generate_balance_sheet(&self, as_of_date: DateTime<Utc>, currency: &str, include_sub_accounts: bool) -> Result<FinancialReport, String>;
    async fn get_ledger_metrics(&self, start_date: Option<DateTime<Utc>>, end_date: Option<DateTime<Utc>>, currency: Option<String>) -> Result<LedgerMetrics, String>;

    // Snapshot operations
    async fn create_balance_snapshot(&self, snapshot: &AccountBalanceSnapshot) -> Result<AccountBalanceSnapshot, String>;
    async fn get_balance_snapshots(&self, account_id: &Uuid, start_date: NaiveDate, end_date: NaiveDate) -> Result<Vec<AccountBalanceSnapshot>, String>;
}

/// In-memory implementation for development and testing
#[derive(Debug, Default)]
pub struct InMemoryLedgerRepository {
    accounts: std::sync::RwLock<HashMap<Uuid, LedgerAccount>>,
    transactions: std::sync::RwLock<HashMap<Uuid, LedgerTransaction>>,
    journal_entries: std::sync::RwLock<HashMap<Uuid, JournalEntry>>,
    audit_trail: std::sync::RwLock<Vec<AuditTrailEntry>>,
    balance_snapshots: std::sync::RwLock<HashMap<Uuid, Vec<AccountBalanceSnapshot>>>,
    account_codes: std::sync::RwLock<HashMap<String, Uuid>>,
    reference_numbers: std::sync::RwLock<HashMap<String, Uuid>>,
}

impl InMemoryLedgerRepository {
    pub fn new() -> Self {
        Self::default()
    }

    /// Generate unique reference number for transactions
    fn generate_reference_number() -> String {
        format!("TXN{}", Uuid::new_v4().to_string().replace('-', "").to_uppercase()[..12].to_string())
    }

    /// Validate double-entry bookkeeping rules
    fn validate_double_entry(entries: &[JournalEntry]) -> Result<(), String> {
        if entries.is_empty() {
            return Err("Transaction must have at least one journal entry".to_string());
        }

        let mut debit_total = Decimal::ZERO;
        let mut credit_total = Decimal::ZERO;

        for entry in entries {
            match entry.entry_type {
                EntryType::Debit => debit_total += entry.amount,
                EntryType::Credit => credit_total += entry.amount,
            }
        }

        if debit_total != credit_total {
            return Err(format!(
                "Double-entry validation failed: debits ({}) != credits ({})",
                debit_total, credit_total
            ));
        }

        Ok(())
    }

    /// Calculate account balance based on account type and entry type
    fn calculate_balance_impact(account_type: &AccountType, entry_type: &EntryType, amount: Decimal) -> Decimal {
        match (account_type, entry_type) {
            // Assets increase with debits, decrease with credits
            (AccountType::Asset, EntryType::Debit) => amount,
            (AccountType::Asset, EntryType::Credit) => -amount,

            // Liabilities increase with credits, decrease with debits
            (AccountType::Liability, EntryType::Credit) => amount,
            (AccountType::Liability, EntryType::Debit) => -amount,

            // Equity increases with credits, decreases with debits
            (AccountType::Equity, EntryType::Credit) => amount,
            (AccountType::Equity, EntryType::Debit) => -amount,

            // Revenue increases with credits, decreases with debits
            (AccountType::Revenue, EntryType::Credit) => amount,
            (AccountType::Revenue, EntryType::Debit) => -amount,

            // Expenses increase with debits, decrease with credits
            (AccountType::Expense, EntryType::Debit) => amount,
            (AccountType::Expense, EntryType::Credit) => -amount,

            // Contra accounts work opposite to their base type
            (AccountType::ContraAsset, EntryType::Credit) => amount,
            (AccountType::ContraAsset, EntryType::Debit) => -amount,
            (AccountType::ContraLiability, EntryType::Debit) => amount,
            (AccountType::ContraLiability, EntryType::Credit) => -amount,
            (AccountType::ContraEquity, EntryType::Debit) => amount,
            (AccountType::ContraEquity, EntryType::Credit) => -amount,
        }
    }
}

#[async_trait::async_trait]
impl LedgerRepository for InMemoryLedgerRepository {
    // Account operations
    async fn create_account(&self, account: &LedgerAccount) -> Result<LedgerAccount, String> {
        let mut accounts = self.accounts.write().unwrap();
        let mut account_codes = self.account_codes.write().unwrap();

        // Check for duplicate account code
        if account_codes.contains_key(&account.account_code) {
            return Err(format!("Account code '{}' already exists", account.account_code));
        }

        accounts.insert(account.id, account.clone());
        account_codes.insert(account.account_code.clone(), account.id);
        Ok(account.clone())
    }

    async fn get_account(&self, id: &Uuid) -> Result<Option<LedgerAccount>, String> {
        let accounts = self.accounts.read().unwrap();
        Ok(accounts.get(id).cloned())
    }

    async fn get_account_by_code(&self, code: &str) -> Result<Option<LedgerAccount>, String> {
        let account_codes = self.account_codes.read().unwrap();
        let accounts = self.accounts.read().unwrap();

        if let Some(account_id) = account_codes.get(code) {
            Ok(accounts.get(account_id).cloned())
        } else {
            Ok(None)
        }
    }

    async fn list_accounts(
        &self,
        account_type: Option<AccountType>,
        status: Option<AccountStatus>,
        currency: Option<String>,
        parent_id: Option<Uuid>,
        page: i32,
        page_size: i32
    ) -> Result<(Vec<LedgerAccount>, i64), String> {
        let accounts = self.accounts.read().unwrap();
        let mut filtered_accounts: Vec<LedgerAccount> = accounts
            .values()
            .filter(|account| {
                account_type.as_ref().map_or(true, |t| account.account_type == *t) &&
                status.as_ref().map_or(true, |s| account.status == *s) &&
                currency.as_ref().map_or(true, |c| account.currency == *c) &&
                parent_id.map_or(true, |p| account.parent_account_id == Some(p))
            })
            .cloned()
            .collect();

        // Sort by account code
        filtered_accounts.sort_by(|a, b| a.account_code.cmp(&b.account_code));

        let total_count = filtered_accounts.len() as i64;
        let start = ((page - 1) * page_size) as usize;
        let end = (start + page_size as usize).min(filtered_accounts.len());

        let paginated_accounts = if start < filtered_accounts.len() {
            filtered_accounts[start..end].to_vec()
        } else {
            Vec::new()
        };

        Ok((paginated_accounts, total_count))
    }

    async fn update_account(&self, account: &LedgerAccount) -> Result<LedgerAccount, String> {
        let mut accounts = self.accounts.write().unwrap();
        accounts.insert(account.id, account.clone());
        Ok(account.clone())
    }

    async fn close_account(&self, id: &Uuid, reason: &str) -> Result<LedgerAccount, String> {
        let mut accounts = self.accounts.write().unwrap();

        if let Some(mut account) = accounts.get(id).cloned() {
            account.status = AccountStatus::Closed;
            account.closed_at = Some(Utc::now());
            account.updated_at = Utc::now();
            account.metadata.insert("closure_reason".to_string(), reason.to_string());

            accounts.insert(*id, account.clone());
            Ok(account)
        } else {
            Err("Account not found".to_string())
        }
    }

    // Transaction operations
    async fn create_transaction(&self, transaction: &LedgerTransaction) -> Result<LedgerTransaction, String> {
        // Validate double-entry bookkeeping
        Self::validate_double_entry(&transaction.entries)?;

        let mut transactions = self.transactions.write().unwrap();
        let mut reference_numbers = self.reference_numbers.write().unwrap();
        let mut journal_entries = self.journal_entries.write().unwrap();

        // Check for duplicate reference number
        if reference_numbers.contains_key(&transaction.reference_number) {
            return Err(format!("Reference number '{}' already exists", transaction.reference_number));
        }

        // Store transaction
        transactions.insert(transaction.id, transaction.clone());
        reference_numbers.insert(transaction.reference_number.clone(), transaction.id);

        // Store journal entries
        for entry in &transaction.entries {
            journal_entries.insert(entry.id, entry.clone());
        }

        Ok(transaction.clone())
    }

    async fn get_transaction(&self, id: &Uuid) -> Result<Option<LedgerTransaction>, String> {
        let transactions = self.transactions.read().unwrap();
        Ok(transactions.get(id).cloned())
    }

    async fn get_transaction_by_reference(&self, reference: &str) -> Result<Option<LedgerTransaction>, String> {
        let reference_numbers = self.reference_numbers.read().unwrap();
        let transactions = self.transactions.read().unwrap();

        if let Some(transaction_id) = reference_numbers.get(reference) {
            Ok(transactions.get(transaction_id).cloned())
        } else {
            Ok(None)
        }
    }
}
