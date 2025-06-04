# LedgerService Implementation Guide

## Overview

The FO3 Wallet Core LedgerService provides a comprehensive double-entry bookkeeping system for all asset movements within the platform. This service ensures regulatory compliance, real-time balance reconciliation, and immutable transaction records while integrating seamlessly with existing services like CardService, FiatGateway, and CardFundingService.

## ✅ **Implementation Status: COMPLETE**

**All 22 gRPC methods have been fully implemented:**
- ✅ **Core ledger operations (5/5):** `CreateLedgerAccount`, `GetLedgerAccount`, `ListLedgerAccounts`, `UpdateLedgerAccount`, `CloseLedgerAccount`
- ✅ **Transaction recording operations (4/4):** `RecordTransaction`, `GetTransaction`, `ListTransactions`, `ReverseTransaction`
- ✅ **Journal entry operations (4/4):** `CreateJournalEntry`, `GetJournalEntry`, `ListJournalEntries`, `PostJournalEntry`
- ✅ **Balance and reconciliation operations (4/4):** `GetAccountBalance`, `GetTrialBalance`, `ReconcileAccounts`, `GetBalanceSheet`
- ✅ **Audit and compliance operations (4/4):** `GetAuditTrail`, `GenerateFinancialReport`, `ValidateBookkeeping`, `ExportLedgerData`
- ✅ **Admin operations (3/3):** `GetLedgerMetrics`, `PerformPeriodClose`, `BackupLedgerData`

**Additional Implementation Completed:**
- ✅ Complete in-memory repository implementation with all CRUD operations
- ✅ Double-entry bookkeeping validation and enforcement
- ✅ Real-time balance calculation and account reconciliation
- ✅ Comprehensive security middleware with audit logging
- ✅ Full test coverage with unit and integration tests
- ✅ Financial reporting and compliance features
- ✅ Transaction reversal and period close functionality
- ✅ Service integration with existing FO3 Wallet Core components

## 🏗️ Architecture

### Core Components

1. **LedgerService** (`fo3-wallet-api/src/services/ledger.rs`)
   - gRPC service implementation for all ledger operations
   - Double-entry bookkeeping enforcement
   - Real-time balance calculation and reconciliation
   - Integration with all FO3 Wallet services

2. **LedgerRepository** (`fo3-wallet-api/src/models/ledger.rs`)
   - Chart of accounts management
   - Transaction and journal entry data models
   - Repository pattern for data access
   - In-memory implementation for development

3. **LedgerGuard** (`fo3-wallet-api/src/middleware/ledger_guard.rs`)
   - Security validation and compliance enforcement
   - Double-entry bookkeeping validation
   - Rate limiting for ledger operations
   - Audit trail generation

4. **Database Schema** (`init.sql`)
   - Ledger accounts table with hierarchical structure
   - Transactions and journal entries with audit trail
   - Balance snapshots for performance optimization
   - Comprehensive indexing for query performance

## 🔧 Features

### Chart of Accounts

1. **Account Types**
   - **Assets**: Cash, receivables, inventory, equipment
   - **Liabilities**: Payables, loans, accrued expenses
   - **Equity**: Capital, retained earnings, owner's equity
   - **Revenue**: Service fees, interest income, commissions
   - **Expenses**: Operational costs, depreciation, taxes
   - **Contra Accounts**: Allowances, accumulated depreciation

2. **Account Management**
   - Hierarchical account structure with parent-child relationships
   - Flexible account coding system (alphanumeric with separators)
   - Multi-currency support with real-time conversion
   - System and manual account classifications

### Double-Entry Bookkeeping

1. **Transaction Recording**
   - Automatic double-entry validation (debits = credits)
   - Multi-entry transactions with unlimited journal entries
   - Real-time balance updates upon posting
   - Immutable transaction records for compliance

2. **Journal Entry Management**
   - Draft, posted, and reversed entry statuses
   - Sequence tracking within transactions
   - Detailed descriptions and metadata support
   - Automatic posting date assignment

### Balance Management

1. **Real-Time Balances**
   - Current balance (posted transactions only)
   - Pending balance (including unposted transactions)
   - Available balance calculations
   - Multi-currency balance tracking

2. **Trial Balance**
   - Real-time trial balance generation
   - Debit/credit balance classification by account type
   - Zero balance inclusion/exclusion options
   - Currency and date filtering

### Financial Reporting

1. **Balance Sheet**
   - Assets, liabilities, and equity sections
   - Hierarchical account grouping
   - Multi-currency consolidation
   - As-of-date reporting

2. **Audit Trail**
   - Complete transaction history
   - User action tracking
   - IP address and timestamp logging
   - Compliance-ready audit reports

### Security & Compliance

1. **Access Control**
   - Role-based permissions (read, write, post, reverse, admin)
   - Operation-specific rate limiting
   - Enhanced security for reversals and admin operations
   - Comprehensive audit logging

2. **Data Integrity**
   - Immutable transaction records
   - Double-entry validation enforcement
   - Balance reconciliation checks
   - Automated bookkeeping validation

## 📊 API Endpoints

### Account Management

```protobuf
// Create a new ledger account
rpc CreateLedgerAccount(CreateLedgerAccountRequest) returns (CreateLedgerAccountResponse);

// Get account details
rpc GetLedgerAccount(GetLedgerAccountRequest) returns (GetLedgerAccountResponse);

// List accounts with filtering
rpc ListLedgerAccounts(ListLedgerAccountsRequest) returns (ListLedgerAccountsResponse);

// Update account properties
rpc UpdateLedgerAccount(UpdateLedgerAccountRequest) returns (UpdateLedgerAccountResponse);

// Close an account
rpc CloseLedgerAccount(CloseLedgerAccountRequest) returns (CloseLedgerAccountResponse);
```

### Transaction Management

```protobuf
// Record a new transaction
rpc RecordTransaction(RecordTransactionRequest) returns (RecordTransactionResponse);

// Get transaction details
rpc GetTransaction(GetTransactionRequest) returns (GetTransactionResponse);

// List transactions with filtering
rpc ListTransactions(ListTransactionsRequest) returns (ListTransactionsResponse);

// Reverse a posted transaction
rpc ReverseTransaction(ReverseTransactionRequest) returns (ReverseTransactionResponse);
```

### Journal Entry Operations

```protobuf
// Create journal entries
rpc CreateJournalEntry(CreateJournalEntryRequest) returns (CreateJournalEntryResponse);

// Get journal entry details
rpc GetJournalEntry(GetJournalEntryRequest) returns (GetJournalEntryResponse);

// List journal entries
rpc ListJournalEntries(ListJournalEntriesRequest) returns (ListJournalEntriesResponse);

// Post a journal entry
rpc PostJournalEntry(PostJournalEntryRequest) returns (PostJournalEntryResponse);
```

### Balance & Reconciliation

```protobuf
// Get account balance
rpc GetAccountBalance(GetAccountBalanceRequest) returns (GetAccountBalanceResponse);

// Generate trial balance
rpc GetTrialBalance(GetTrialBalanceRequest) returns (GetTrialBalanceResponse);

// Reconcile accounts
rpc ReconcileAccounts(ReconcileAccountsRequest) returns (ReconcileAccountsResponse);

// Generate balance sheet
rpc GetBalanceSheet(GetBalanceSheetRequest) returns (GetBalanceSheetResponse);
```

### Audit & Compliance

```protobuf
// Get audit trail
rpc GetAuditTrail(GetAuditTrailRequest) returns (GetAuditTrailResponse);

// Generate financial reports
rpc GenerateFinancialReport(GenerateFinancialReportRequest) returns (GenerateFinancialReportResponse);

// Validate bookkeeping integrity
rpc ValidateBookkeeping(ValidateBookkeepingRequest) returns (ValidateBookkeepingResponse);

// Export ledger data
rpc ExportLedgerData(ExportLedgerDataRequest) returns (ExportLedgerDataResponse);
```

### Admin Operations

```protobuf
// Get ledger metrics
rpc GetLedgerMetrics(GetLedgerMetricsRequest) returns (GetLedgerMetricsResponse);

// Perform period close
rpc PerformPeriodClose(PerformPeriodCloseRequest) returns (PerformPeriodCloseResponse);

// Backup ledger data
rpc BackupLedgerData(BackupLedgerDataRequest) returns (BackupLedgerDataResponse);
```

## 🔒 Security Implementation

### Authentication & Authorization

```rust
// JWT-based authentication with RBAC
let auth_context = self.auth_service.extract_auth(&request).await?;
self.auth_service.check_permission(&auth_context, Permission::PermissionLedgerWrite)?;

// Enhanced validation for transaction recording
let auth_context = self.ledger_guard
    .validate_transaction_recording(&request, &transaction_type, &entries, &total_amount)
    .await?;
```

### Rate Limiting

```rust
// Account creation: 10 attempts per 15 minutes
let rate_key = format!("account_creation:{}", auth_context.user_id);
if !self.rate_limiter.check_rate_limit(&rate_key, 10, Duration::minutes(15)).await {
    return Err(Status::resource_exhausted("Too many account creation attempts"));
}

// Transaction recording: 50 attempts per 5 minutes
let rate_key = format!("transaction_recording:{}", auth_context.user_id);
if !self.rate_limiter.check_rate_limit(&rate_key, 50, Duration::minutes(5)).await {
    return Err(Status::resource_exhausted("Too many transaction recording attempts"));
}

// Transaction reversal: 5 attempts per 15 minutes (enhanced security)
let rate_key = format!("transaction_reversal:{}", auth_context.user_id);
if !self.rate_limiter.check_rate_limit(&rate_key, 5, Duration::minutes(15)).await {
    return Err(Status::resource_exhausted("Too many transaction reversal attempts"));
}
```

### Double-Entry Validation

```rust
fn validate_double_entry_rules(&self, entries: &[JournalEntry]) -> Result<(), Status> {
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
```

## 💰 Integration with FO3 Services

### CardService Integration

```rust
// Record card transaction in ledger
pub async fn record_card_transaction(
    &self,
    card_id: &Uuid,
    amount: Decimal,
    merchant: &str,
    transaction_type: &str,
) -> Result<LedgerTransaction, String> {
    let entries = vec![
        JournalEntry {
            account_id: self.get_card_asset_account(card_id).await?,
            entry_type: EntryType::Credit, // Decrease card balance
            amount,
            description: format!("Card payment to {}", merchant),
            // ... other fields
        },
        JournalEntry {
            account_id: self.get_merchant_expense_account().await?,
            entry_type: EntryType::Debit, // Increase expenses
            amount,
            description: format!("Payment to {}", merchant),
            // ... other fields
        },
    ];

    self.record_transaction(&LedgerTransaction {
        transaction_type: transaction_type.to_string(),
        description: format!("Card transaction: {}", merchant),
        entries,
        source_service: Some("CardService".to_string()),
        source_transaction_id: Some(card_transaction_id.to_string()),
        // ... other fields
    }).await
}
```

### CardFundingService Integration

```rust
// Record funding transaction in ledger
pub async fn record_funding_transaction(
    &self,
    funding_transaction: &FundingTransaction,
) -> Result<LedgerTransaction, String> {
    let entries = vec![
        JournalEntry {
            account_id: self.get_card_asset_account(&funding_transaction.card_id).await?,
            entry_type: EntryType::Debit, // Increase card balance
            amount: funding_transaction.net_amount,
            description: "Card funding".to_string(),
            // ... other fields
        },
        JournalEntry {
            account_id: self.get_funding_source_account(&funding_transaction.funding_source_id).await?,
            entry_type: EntryType::Credit, // Decrease funding source
            amount: funding_transaction.amount,
            description: "Funding source debit".to_string(),
            // ... other fields
        },
        JournalEntry {
            account_id: self.get_fee_revenue_account().await?,
            entry_type: EntryType::Credit, // Record fee revenue
            amount: funding_transaction.fee_amount,
            description: "Funding fee revenue".to_string(),
            // ... other fields
        },
    ];

    self.record_transaction(&LedgerTransaction {
        transaction_type: "card_funding".to_string(),
        description: "Card funding transaction".to_string(),
        entries,
        source_service: Some("CardFundingService".to_string()),
        source_transaction_id: Some(funding_transaction.id.to_string()),
        // ... other fields
    }).await
}
```

### FiatGateway Integration

```rust
// Record fiat deposit/withdrawal in ledger
pub async fn record_fiat_transaction(
    &self,
    fiat_transaction: &FiatTransaction,
) -> Result<LedgerTransaction, String> {
    let (debit_account, credit_account) = match fiat_transaction.transaction_type {
        FiatTransactionType::Deposit => {
            (self.get_cash_account().await?, self.get_customer_deposits_account().await?)
        }
        FiatTransactionType::Withdrawal => {
            (self.get_customer_deposits_account().await?, self.get_cash_account().await?)
        }
    };

    let entries = vec![
        JournalEntry {
            account_id: debit_account,
            entry_type: EntryType::Debit,
            amount: fiat_transaction.amount,
            description: format!("{:?} transaction", fiat_transaction.transaction_type),
            // ... other fields
        },
        JournalEntry {
            account_id: credit_account,
            entry_type: EntryType::Credit,
            amount: fiat_transaction.amount,
            description: format!("{:?} transaction", fiat_transaction.transaction_type),
            // ... other fields
        },
    ];

    self.record_transaction(&LedgerTransaction {
        transaction_type: format!("fiat_{:?}", fiat_transaction.transaction_type).to_lowercase(),
        description: format!("Fiat {:?}", fiat_transaction.transaction_type),
        entries,
        source_service: Some("FiatGateway".to_string()),
        source_transaction_id: Some(fiat_transaction.id.to_string()),
        // ... other fields
    }).await
}
```

## 📈 Chart of Accounts Structure

### Standard Account Codes

```
1000-1999: Assets
  1001: Cash and Cash Equivalents
  1100: Customer Card Balances
  1200: Accounts Receivable
  1300: Prepaid Expenses
  1400: Equipment and Software
  1500: Accumulated Depreciation (Contra-Asset)

2000-2999: Liabilities
  2001: Accounts Payable
  2100: Customer Deposits
  2200: Accrued Expenses
  2300: Deferred Revenue
  2400: Short-term Debt
  2500: Long-term Debt

3000-3999: Equity
  3001: Share Capital
  3100: Retained Earnings
  3200: Current Year Earnings

4000-4999: Revenue
  4001: Card Transaction Fees
  4100: Funding Fees
  4200: Interchange Revenue
  4300: Interest Income
  4400: Other Revenue

5000-5999: Expenses
  5001: Payment Processing Costs
  5100: Technology Expenses
  5200: Personnel Costs
  5300: Marketing Expenses
  5400: Compliance Costs
  5500: General & Administrative
```

## 🧪 Testing

### Unit Tests

```bash
# Run ledger service tests
cargo test ledger_test

# Run specific test categories
cargo test test_double_entry_validation
cargo test test_balance_calculation
cargo test test_trial_balance_generation
```

### Integration Tests

```bash
# Run full integration test suite
cargo test --test ledger_test

# Test with different account types
cargo test test_create_asset_account
cargo test test_create_liability_account
cargo test test_record_double_entry_transaction
```

### Performance Tests

```bash
# Load testing with multiple concurrent transactions
cargo test test_concurrent_transaction_recording

# Balance calculation performance
cargo test test_large_volume_balance_calculation
```

## 📊 Monitoring & Analytics

### Key Metrics

1. **Transaction Volume Metrics**
   - Total transactions processed
   - Transactions by type and source service
   - Average transaction size
   - Processing times

2. **Balance Integrity Metrics**
   - Trial balance validation status
   - Account reconciliation results
   - Double-entry compliance rate
   - Balance calculation performance

3. **Compliance Metrics**
   - Audit trail completeness
   - Transaction immutability verification
   - Regulatory report generation times
   - Data export success rates

### Observability

```rust
// Prometheus metrics integration
use prometheus::{Counter, Histogram, Gauge};

lazy_static! {
    static ref LEDGER_TRANSACTIONS_TOTAL: Counter = Counter::new(
        "ledger_transactions_total", "Total number of ledger transactions"
    ).unwrap();
    
    static ref BALANCE_CALCULATION_DURATION: Histogram = Histogram::new(
        "balance_calculation_duration_seconds", "Time spent calculating balances"
    ).unwrap();
    
    static ref TRIAL_BALANCE_STATUS: Gauge = Gauge::new(
        "trial_balance_balanced", "Whether the trial balance is balanced (1=balanced, 0=unbalanced)"
    ).unwrap();
}
```

## 🚀 Deployment

### Environment Configuration

```bash
# Enable ledger service
ENABLE_LEDGER_SERVICE=true

# Double-entry validation
ENFORCE_DOUBLE_ENTRY=true
ALLOW_UNBALANCED_TRANSACTIONS=false

# Performance settings
LEDGER_BATCH_SIZE=1000
BALANCE_CALCULATION_CACHE_TTL=300

# Compliance settings
AUDIT_TRAIL_RETENTION_DAYS=2555  # 7 years
IMMUTABLE_TRANSACTIONS=true
REQUIRE_REVERSAL_REASON=true
```

### Database Migration

```sql
-- Run the ledger service database migration
psql -d fo3_wallet -f init.sql

-- Verify ledger tables created
\dt ledger*
\dt journal*
\dt account*
```

### Service Health Checks

```bash
# Check service status
grpcurl -plaintext localhost:50051 fo3.wallet.v1.HealthService/Check

# Test ledger service endpoints
grpcurl -plaintext -d '{"page": 1, "page_size": 10}' \
  localhost:50051 fo3.wallet.v1.LedgerService/ListLedgerAccounts

# Validate trial balance
grpcurl -plaintext -d '{"as_of_date": "2024-12-31T23:59:59Z", "currency": "USD"}' \
  localhost:50051 fo3.wallet.v1.LedgerService/GetTrialBalance
```

## 📋 Best Practices

### Double-Entry Bookkeeping

1. **Always validate double-entry rules** before posting transactions
2. **Use descriptive account codes** following a consistent numbering scheme
3. **Maintain detailed transaction descriptions** for audit purposes
4. **Implement proper account hierarchies** for financial reporting
5. **Regular trial balance validation** to ensure books are balanced

### Performance Optimization

1. **Use balance snapshots** for frequently accessed account balances
2. **Implement proper database indexing** for query optimization
3. **Cache trial balance results** for reporting performance
4. **Batch process large volumes** of transactions when possible
5. **Optimize journal entry queries** with proper filtering

### Compliance & Security

1. **Maintain immutable transaction records** for regulatory compliance
2. **Implement comprehensive audit trails** for all operations
3. **Use proper access controls** for sensitive ledger operations
4. **Regular backup and recovery testing** for data protection
5. **Monitor for suspicious transaction patterns** and anomalies

This LedgerService implementation provides a robust, compliant, and scalable foundation for double-entry bookkeeping in the FO3 Wallet ecosystem, ensuring accurate financial record-keeping and regulatory compliance while supporting real-time balance reconciliation across all platform services.
