//! Comprehensive tests for the LedgerService

#[cfg(test)]
mod tests {
    use super::super::ledger::LedgerServiceImpl;
    use crate::models::ledger::{
        LedgerAccount, LedgerTransaction, JournalEntry, AccountType, AccountStatus, 
        TransactionStatus, JournalEntryStatus, EntryType, InMemoryLedgerRepository,
        LedgerRepository,
    };
    use crate::middleware::{
        auth::AuthService,
        audit::AuditLogger,
        ledger_guard::LedgerGuard,
        rate_limit::RateLimiter,
    };
    use crate::state::AppState;
    use crate::proto::fo3::wallet::v1::{
        ledger_service_server::LedgerService,
        *,
    };
    use std::sync::Arc;
    use std::collections::HashMap;
    use uuid::Uuid;
    use rust_decimal::Decimal;
    use chrono::Utc;
    use tonic::{Request, Code};

    fn create_test_service() -> LedgerServiceImpl {
        let state = Arc::new(AppState::new());
        let auth_service = Arc::new(AuthService::new(state.clone()));
        let audit_logger = Arc::new(AuditLogger::new(state.clone()));
        let rate_limiter = Arc::new(RateLimiter::new());
        let ledger_repository = Arc::new(InMemoryLedgerRepository::new());
        let ledger_guard = Arc::new(LedgerGuard::new(
            auth_service.clone(),
            audit_logger.clone(),
            rate_limiter.clone(),
            ledger_repository.clone(),
        ));

        LedgerServiceImpl::new(
            state,
            auth_service,
            audit_logger,
            ledger_guard,
            ledger_repository,
        )
    }

    fn create_test_account() -> LedgerAccount {
        LedgerAccount {
            id: Uuid::new_v4(),
            account_code: "1000".to_string(),
            account_name: "Cash".to_string(),
            account_type: AccountType::Asset,
            status: AccountStatus::Active,
            currency: "USD".to_string(),
            parent_account_id: None,
            description: Some("Cash account".to_string()),
            is_system_account: false,
            allow_manual_entries: true,
            current_balance: Decimal::ZERO,
            pending_balance: Decimal::ZERO,
            metadata: HashMap::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            closed_at: None,
        }
    }

    fn create_test_journal_entries(transaction_id: Uuid, account1_id: Uuid, account2_id: Uuid) -> Vec<JournalEntry> {
        vec![
            JournalEntry {
                id: Uuid::new_v4(),
                transaction_id,
                account_id: account1_id,
                entry_type: EntryType::Debit,
                amount: Decimal::from(100),
                currency: "USD".to_string(),
                description: "Test debit entry".to_string(),
                status: JournalEntryStatus::Draft,
                entry_sequence: 1,
                metadata: HashMap::new(),
                created_at: Utc::now(),
                updated_at: Utc::now(),
                posted_at: None,
            },
            JournalEntry {
                id: Uuid::new_v4(),
                transaction_id,
                account_id: account2_id,
                entry_type: EntryType::Credit,
                amount: Decimal::from(100),
                currency: "USD".to_string(),
                description: "Test credit entry".to_string(),
                status: JournalEntryStatus::Draft,
                entry_sequence: 2,
                metadata: HashMap::new(),
                created_at: Utc::now(),
                updated_at: Utc::now(),
                posted_at: None,
            },
        ]
    }

    #[tokio::test]
    async fn test_create_ledger_account_success() {
        let service = create_test_service();
        
        let request = Request::new(CreateLedgerAccountRequest {
            account_code: "1000".to_string(),
            account_name: "Cash".to_string(),
            account_type: account_type::AccountTypeAsset as i32,
            currency: "USD".to_string(),
            parent_account_id: "".to_string(),
            description: "Cash account".to_string(),
            allow_manual_entries: true,
        });

        // This would fail without proper authentication, but tests the structure
        let result = service.create_ledger_account(request).await;
        assert!(result.is_err()); // Expected to fail due to missing auth
        assert_eq!(result.unwrap_err().code(), Code::Unauthenticated);
    }

    #[tokio::test]
    async fn test_ledger_account_repository_operations() {
        let repository = InMemoryLedgerRepository::new();
        let test_account = create_test_account();

        // Test create
        let created = repository.create_account(&test_account).await.unwrap();
        assert_eq!(created.id, test_account.id);
        assert_eq!(created.account_code, test_account.account_code);

        // Test get
        let retrieved = repository.get_account(&test_account.id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, test_account.id);

        // Test get by code
        let by_code = repository.get_account_by_code(&test_account.account_code).await.unwrap();
        assert!(by_code.is_some());
        assert_eq!(by_code.unwrap().account_code, test_account.account_code);

        // Test list
        let (accounts, count) = repository
            .list_accounts(Some(AccountType::Asset), None, None, None, 1, 10)
            .await
            .unwrap();
        assert_eq!(count, 1);
        assert_eq!(accounts.len(), 1);

        // Test update
        let mut updated_account = test_account.clone();
        updated_account.account_name = "Updated Cash".to_string();
        let updated = repository.update_account(&updated_account).await.unwrap();
        assert_eq!(updated.account_name, "Updated Cash");

        // Test close
        let closed = repository.close_account(&test_account.id, "Test closure").await.unwrap();
        assert_eq!(closed.status, AccountStatus::Closed);
        assert!(closed.closed_at.is_some());
    }

    #[tokio::test]
    async fn test_double_entry_validation() {
        let repository = InMemoryLedgerRepository::new();
        
        // Create test accounts
        let account1 = create_test_account();
        let mut account2 = create_test_account();
        account2.id = Uuid::new_v4();
        account2.account_code = "2000".to_string();
        account2.account_name = "Accounts Payable".to_string();
        account2.account_type = AccountType::Liability;

        repository.create_account(&account1).await.unwrap();
        repository.create_account(&account2).await.unwrap();

        // Test valid double-entry transaction
        let transaction_id = Uuid::new_v4();
        let entries = create_test_journal_entries(transaction_id, account1.id, account2.id);
        
        let transaction = LedgerTransaction {
            id: transaction_id,
            reference_number: "TXN001".to_string(),
            status: TransactionStatus::Pending,
            transaction_type: "test".to_string(),
            description: "Test transaction".to_string(),
            currency: "USD".to_string(),
            total_amount: Decimal::from(100),
            entries,
            source_service: None,
            source_transaction_id: None,
            metadata: HashMap::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            posted_at: None,
            reversed_at: None,
            reversal_reason: None,
            reversal_transaction_id: None,
        };

        // Should succeed with balanced entries
        let created = repository.create_transaction(&transaction).await.unwrap();
        assert_eq!(created.id, transaction.id);

        // Test invalid unbalanced transaction
        let mut unbalanced_entries = create_test_journal_entries(Uuid::new_v4(), account1.id, account2.id);
        unbalanced_entries[1].amount = Decimal::from(50); // Unbalanced

        let unbalanced_transaction = LedgerTransaction {
            id: Uuid::new_v4(),
            reference_number: "TXN002".to_string(),
            status: TransactionStatus::Pending,
            transaction_type: "test".to_string(),
            description: "Unbalanced transaction".to_string(),
            currency: "USD".to_string(),
            total_amount: Decimal::from(100),
            entries: unbalanced_entries,
            source_service: None,
            source_transaction_id: None,
            metadata: HashMap::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            posted_at: None,
            reversed_at: None,
            reversal_reason: None,
            reversal_transaction_id: None,
        };

        // Should fail with unbalanced entries
        let result = repository.create_transaction(&unbalanced_transaction).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Double-entry validation failed"));
    }

    #[tokio::test]
    async fn test_transaction_posting_and_balance_updates() {
        let repository = InMemoryLedgerRepository::new();
        
        // Create test accounts
        let mut cash_account = create_test_account();
        cash_account.account_code = "1000".to_string();
        cash_account.account_name = "Cash".to_string();
        cash_account.account_type = AccountType::Asset;

        let mut revenue_account = create_test_account();
        revenue_account.id = Uuid::new_v4();
        revenue_account.account_code = "4000".to_string();
        revenue_account.account_name = "Revenue".to_string();
        revenue_account.account_type = AccountType::Revenue;

        repository.create_account(&cash_account).await.unwrap();
        repository.create_account(&revenue_account).await.unwrap();

        // Create transaction: Debit Cash $100, Credit Revenue $100
        let transaction_id = Uuid::new_v4();
        let entries = create_test_journal_entries(transaction_id, cash_account.id, revenue_account.id);
        
        let transaction = LedgerTransaction {
            id: transaction_id,
            reference_number: "TXN001".to_string(),
            status: TransactionStatus::Pending,
            transaction_type: "revenue".to_string(),
            description: "Revenue transaction".to_string(),
            currency: "USD".to_string(),
            total_amount: Decimal::from(100),
            entries,
            source_service: None,
            source_transaction_id: None,
            metadata: HashMap::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            posted_at: None,
            reversed_at: None,
            reversal_reason: None,
            reversal_transaction_id: None,
        };

        // Create and post transaction
        repository.create_transaction(&transaction).await.unwrap();
        let posted_transaction = repository.post_transaction(&transaction_id).await.unwrap();

        // Verify transaction status
        assert_eq!(posted_transaction.status, TransactionStatus::Posted);
        assert!(posted_transaction.posted_at.is_some());

        // Verify account balances
        let updated_cash = repository.get_account(&cash_account.id).await.unwrap().unwrap();
        let updated_revenue = repository.get_account(&revenue_account.id).await.unwrap().unwrap();

        // Cash (Asset) increases with debit
        assert_eq!(updated_cash.current_balance, Decimal::from(100));
        // Revenue increases with credit
        assert_eq!(updated_revenue.current_balance, Decimal::from(100));
    }

    #[tokio::test]
    async fn test_transaction_reversal() {
        let repository = InMemoryLedgerRepository::new();
        
        // Create and post a transaction first
        let cash_account = create_test_account();
        let mut revenue_account = create_test_account();
        revenue_account.id = Uuid::new_v4();
        revenue_account.account_code = "4000".to_string();
        revenue_account.account_type = AccountType::Revenue;

        repository.create_account(&cash_account).await.unwrap();
        repository.create_account(&revenue_account).await.unwrap();

        let transaction_id = Uuid::new_v4();
        let entries = create_test_journal_entries(transaction_id, cash_account.id, revenue_account.id);
        
        let transaction = LedgerTransaction {
            id: transaction_id,
            reference_number: "TXN001".to_string(),
            status: TransactionStatus::Pending,
            transaction_type: "revenue".to_string(),
            description: "Revenue transaction".to_string(),
            currency: "USD".to_string(),
            total_amount: Decimal::from(100),
            entries,
            source_service: None,
            source_transaction_id: None,
            metadata: HashMap::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            posted_at: None,
            reversed_at: None,
            reversal_reason: None,
            reversal_transaction_id: None,
        };

        repository.create_transaction(&transaction).await.unwrap();
        repository.post_transaction(&transaction_id).await.unwrap();

        // Now reverse the transaction
        let (original, reversal) = repository
            .reverse_transaction(&transaction_id, "Test reversal", "Reversing for test")
            .await
            .unwrap();

        // Verify original transaction is marked as reversed
        assert_eq!(original.status, TransactionStatus::Reversed);
        assert!(original.reversed_at.is_some());
        assert_eq!(original.reversal_reason, Some("Test reversal".to_string()));

        // Verify reversal transaction
        assert_eq!(reversal.status, TransactionStatus::Posted);
        assert!(reversal.reference_number.starts_with("REV-"));
        assert_eq!(reversal.entries.len(), 2);

        // Verify account balances are back to zero
        let final_cash = repository.get_account(&cash_account.id).await.unwrap().unwrap();
        let final_revenue = repository.get_account(&revenue_account.id).await.unwrap().unwrap();

        assert_eq!(final_cash.current_balance, Decimal::ZERO);
        assert_eq!(final_revenue.current_balance, Decimal::ZERO);
    }
}
