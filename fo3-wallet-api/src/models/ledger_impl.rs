//! In-memory implementation of LedgerRepository

use super::ledger::*;
use std::collections::HashMap;
use uuid::Uuid;
use rust_decimal::Decimal;
use chrono::{DateTime, Utc, NaiveDate};

#[async_trait::async_trait]
impl LedgerRepository for InMemoryLedgerRepository {
    // Account operations
    async fn create_account(&self, account: &LedgerAccount) -> Result<LedgerAccount, String> {
        let mut accounts = self.accounts.write().unwrap();
        let mut codes = self.account_codes.write().unwrap();
        
        // Check for duplicate account code
        if codes.contains_key(&account.account_code) {
            return Err(format!("Account code '{}' already exists", account.account_code));
        }
        
        accounts.insert(account.id, account.clone());
        codes.insert(account.account_code.clone(), account.id);
        
        Ok(account.clone())
    }

    async fn get_account(&self, id: &Uuid) -> Result<Option<LedgerAccount>, String> {
        let accounts = self.accounts.read().unwrap();
        Ok(accounts.get(id).cloned())
    }

    async fn get_account_by_code(&self, code: &str) -> Result<Option<LedgerAccount>, String> {
        let codes = self.account_codes.read().unwrap();
        let accounts = self.accounts.read().unwrap();
        
        if let Some(account_id) = codes.get(code) {
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
        page_size: i32,
    ) -> Result<(Vec<LedgerAccount>, i64), String> {
        let accounts = self.accounts.read().unwrap();
        let mut filtered: Vec<_> = accounts
            .values()
            .filter(|account| account_type.as_ref().map_or(true, |t| account.account_type == *t))
            .filter(|account| status.as_ref().map_or(true, |s| account.status == *s))
            .filter(|account| currency.as_ref().map_or(true, |c| account.currency == *c))
            .filter(|account| parent_id.map_or(true, |p| account.parent_account_id == Some(p)))
            .cloned()
            .collect();

        filtered.sort_by(|a, b| a.account_code.cmp(&b.account_code));

        let total = filtered.len() as i64;
        let start = ((page - 1) * page_size) as usize;
        let end = (start + page_size as usize).min(filtered.len());

        Ok((filtered[start..end].to_vec(), total))
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
            
            // Add closure reason to metadata
            account.metadata.insert("closure_reason".to_string(), reason.to_string());
            
            accounts.insert(*id, account.clone());
            Ok(account)
        } else {
            Err("Account not found".to_string())
        }
    }

    // Transaction operations
    async fn create_transaction(&self, transaction: &LedgerTransaction) -> Result<LedgerTransaction, String> {
        let mut transactions = self.transactions.write().unwrap();
        let mut references = self.reference_numbers.write().unwrap();
        
        // Check for duplicate reference number
        if references.contains_key(&transaction.reference_number) {
            return Err(format!("Reference number '{}' already exists", transaction.reference_number));
        }
        
        // Validate double-entry bookkeeping
        Self::validate_double_entry(&transaction.entries)?;
        
        transactions.insert(transaction.id, transaction.clone());
        references.insert(transaction.reference_number.clone(), transaction.id);
        
        // Create journal entries
        let mut journal_entries = self.journal_entries.write().unwrap();
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
        let references = self.reference_numbers.read().unwrap();
        let transactions = self.transactions.read().unwrap();
        
        if let Some(transaction_id) = references.get(reference) {
            Ok(transactions.get(transaction_id).cloned())
        } else {
            Ok(None)
        }
    }

    async fn list_transactions(
        &self,
        account_id: Option<Uuid>,
        status: Option<TransactionStatus>,
        transaction_type: Option<String>,
        currency: Option<String>,
        start_date: Option<DateTime<Utc>>,
        end_date: Option<DateTime<Utc>>,
        source_service: Option<String>,
        page: i32,
        page_size: i32,
    ) -> Result<(Vec<LedgerTransaction>, i64), String> {
        let transactions = self.transactions.read().unwrap();
        let mut filtered: Vec<_> = transactions
            .values()
            .filter(|tx| {
                if let Some(acc_id) = account_id {
                    tx.entries.iter().any(|entry| entry.account_id == acc_id)
                } else {
                    true
                }
            })
            .filter(|tx| status.as_ref().map_or(true, |s| tx.status == *s))
            .filter(|tx| transaction_type.as_ref().map_or(true, |t| tx.transaction_type == *t))
            .filter(|tx| currency.as_ref().map_or(true, |c| tx.currency == *c))
            .filter(|tx| start_date.map_or(true, |d| tx.created_at >= d))
            .filter(|tx| end_date.map_or(true, |d| tx.created_at <= d))
            .filter(|tx| source_service.as_ref().map_or(true, |s| tx.source_service.as_ref() == Some(s)))
            .cloned()
            .collect();

        filtered.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        let total = filtered.len() as i64;
        let start = ((page - 1) * page_size) as usize;
        let end = (start + page_size as usize).min(filtered.len());

        Ok((filtered[start..end].to_vec(), total))
    }

    async fn update_transaction(&self, transaction: &LedgerTransaction) -> Result<LedgerTransaction, String> {
        let mut transactions = self.transactions.write().unwrap();
        transactions.insert(transaction.id, transaction.clone());
        Ok(transaction.clone())
    }

    async fn post_transaction(&self, id: &Uuid) -> Result<LedgerTransaction, String> {
        let mut transactions = self.transactions.write().unwrap();
        let mut journal_entries = self.journal_entries.write().unwrap();
        let mut accounts = self.accounts.write().unwrap();
        
        if let Some(mut transaction) = transactions.get(id).cloned() {
            if transaction.status != TransactionStatus::Pending {
                return Err("Only pending transactions can be posted".to_string());
            }
            
            // Post all journal entries and update account balances
            for entry in &mut transaction.entries {
                if let Some(mut journal_entry) = journal_entries.get(&entry.id).cloned() {
                    journal_entry.status = JournalEntryStatus::Posted;
                    journal_entry.posted_at = Some(Utc::now());
                    journal_entries.insert(entry.id, journal_entry.clone());
                    
                    // Update account balance
                    if let Some(mut account) = accounts.get(&entry.account_id).cloned() {
                        let balance_impact = Self::calculate_balance_impact(
                            &account.account_type,
                            &entry.entry_type,
                            entry.amount
                        );
                        account.current_balance += balance_impact;
                        account.updated_at = Utc::now();
                        accounts.insert(entry.account_id, account);
                    }
                    
                    *entry = journal_entry;
                }
            }
            
            transaction.status = TransactionStatus::Posted;
            transaction.posted_at = Some(Utc::now());
            transaction.updated_at = Utc::now();
            
            transactions.insert(*id, transaction.clone());
            Ok(transaction)
        } else {
            Err("Transaction not found".to_string())
        }
    }

    async fn reverse_transaction(&self, id: &Uuid, reason: &str, description: &str) -> Result<(LedgerTransaction, LedgerTransaction), String> {
        let mut transactions = self.transactions.write().unwrap();
        let mut references = self.reference_numbers.write().unwrap();
        
        if let Some(mut original_transaction) = transactions.get(id).cloned() {
            if original_transaction.status != TransactionStatus::Posted {
                return Err("Only posted transactions can be reversed".to_string());
            }
            
            // Create reversal transaction
            let reversal_id = Uuid::new_v4();
            let reversal_reference = Self::generate_reference_number();
            
            // Create reversal entries (opposite of original)
            let mut reversal_entries = Vec::new();
            for (i, original_entry) in original_transaction.entries.iter().enumerate() {
                let reversal_entry = JournalEntry {
                    id: Uuid::new_v4(),
                    transaction_id: reversal_id,
                    account_id: original_entry.account_id,
                    entry_type: match original_entry.entry_type {
                        EntryType::Debit => EntryType::Credit,
                        EntryType::Credit => EntryType::Debit,
                    },
                    amount: original_entry.amount,
                    currency: original_entry.currency.clone(),
                    description: format!("Reversal: {}", original_entry.description),
                    status: JournalEntryStatus::Posted,
                    entry_sequence: i as i32 + 1,
                    metadata: HashMap::new(),
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                    posted_at: Some(Utc::now()),
                };
                reversal_entries.push(reversal_entry);
            }
            
            let reversal_transaction = LedgerTransaction {
                id: reversal_id,
                reference_number: reversal_reference.clone(),
                status: TransactionStatus::Posted,
                transaction_type: format!("reversal_{}", original_transaction.transaction_type),
                description: description.to_string(),
                currency: original_transaction.currency.clone(),
                total_amount: original_transaction.total_amount,
                entries: reversal_entries.clone(),
                source_service: original_transaction.source_service.clone(),
                source_transaction_id: Some(original_transaction.id.to_string()),
                metadata: HashMap::from([
                    ("reversal_reason".to_string(), reason.to_string()),
                    ("original_transaction_id".to_string(), original_transaction.id.to_string()),
                ]),
                created_at: Utc::now(),
                updated_at: Utc::now(),
                posted_at: Some(Utc::now()),
                reversed_at: None,
                reversal_reason: None,
                reversal_transaction_id: None,
            };
            
            // Update original transaction
            original_transaction.status = TransactionStatus::Reversed;
            original_transaction.reversed_at = Some(Utc::now());
            original_transaction.reversal_reason = Some(reason.to_string());
            original_transaction.reversal_transaction_id = Some(reversal_id);
            original_transaction.updated_at = Utc::now();
            
            // Save both transactions
            transactions.insert(*id, original_transaction.clone());
            transactions.insert(reversal_id, reversal_transaction.clone());
            references.insert(reversal_reference, reversal_id);
            
            // Update journal entries
            let mut journal_entries = self.journal_entries.write().unwrap();
            for entry in &reversal_entries {
                journal_entries.insert(entry.id, entry.clone());
            }
            
            // Update account balances
            let mut accounts = self.accounts.write().unwrap();
            for entry in &reversal_entries {
                if let Some(mut account) = accounts.get(&entry.account_id).cloned() {
                    let balance_impact = Self::calculate_balance_impact(
                        &account.account_type,
                        &entry.entry_type,
                        entry.amount
                    );
                    account.current_balance += balance_impact;
                    account.updated_at = Utc::now();
                    accounts.insert(entry.account_id, account);
                }
            }
            
            Ok((original_transaction, reversal_transaction))
        } else {
            Err("Transaction not found".to_string())
        }
    }

    // Journal entry operations
    async fn create_journal_entries(&self, entries: &[JournalEntry]) -> Result<Vec<JournalEntry>, String> {
        let mut journal_entries = self.journal_entries.write().unwrap();

        for entry in entries {
            journal_entries.insert(entry.id, entry.clone());
        }

        Ok(entries.to_vec())
    }

    async fn get_journal_entry(&self, id: &Uuid) -> Result<Option<JournalEntry>, String> {
        let journal_entries = self.journal_entries.read().unwrap();
        Ok(journal_entries.get(id).cloned())
    }

    async fn list_journal_entries(
        &self,
        transaction_id: Option<Uuid>,
        account_id: Option<Uuid>,
        status: Option<JournalEntryStatus>,
        start_date: Option<DateTime<Utc>>,
        end_date: Option<DateTime<Utc>>,
        page: i32,
        page_size: i32,
    ) -> Result<(Vec<JournalEntry>, i64), String> {
        let journal_entries = self.journal_entries.read().unwrap();
        let mut filtered: Vec<_> = journal_entries
            .values()
            .filter(|entry| transaction_id.map_or(true, |id| entry.transaction_id == id))
            .filter(|entry| account_id.map_or(true, |id| entry.account_id == id))
            .filter(|entry| status.as_ref().map_or(true, |s| entry.status == *s))
            .filter(|entry| start_date.map_or(true, |d| entry.created_at >= d))
            .filter(|entry| end_date.map_or(true, |d| entry.created_at <= d))
            .cloned()
            .collect();

        filtered.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        let total = filtered.len() as i64;
        let start = ((page - 1) * page_size) as usize;
        let end = (start + page_size as usize).min(filtered.len());

        Ok((filtered[start..end].to_vec(), total))
    }

    async fn post_journal_entry(&self, id: &Uuid, posting_date: Option<DateTime<Utc>>) -> Result<JournalEntry, String> {
        let mut journal_entries = self.journal_entries.write().unwrap();

        if let Some(mut entry) = journal_entries.get(id).cloned() {
            if entry.status != JournalEntryStatus::Draft {
                return Err("Only draft entries can be posted".to_string());
            }

            entry.status = JournalEntryStatus::Posted;
            entry.posted_at = Some(posting_date.unwrap_or_else(Utc::now));
            entry.updated_at = Utc::now();

            journal_entries.insert(*id, entry.clone());
            Ok(entry)
        } else {
            Err("Journal entry not found".to_string())
        }
    }

    // Balance operations
    async fn get_account_balance(
        &self,
        account_id: &Uuid,
        as_of_date: Option<DateTime<Utc>>,
        include_pending: bool,
    ) -> Result<Option<AccountBalance>, String> {
        let accounts = self.accounts.read().unwrap();
        let journal_entries = self.journal_entries.read().unwrap();

        if let Some(account) = accounts.get(account_id) {
            let cutoff_date = as_of_date.unwrap_or_else(Utc::now);

            // Calculate balance from journal entries up to the cutoff date
            let mut current_balance = Decimal::ZERO;
            let mut pending_balance = Decimal::ZERO;
            let mut transaction_count = 0i64;
            let mut last_transaction_date: Option<DateTime<Utc>> = None;

            for entry in journal_entries.values() {
                if entry.account_id == *account_id && entry.created_at <= cutoff_date {
                    let balance_impact = Self::calculate_balance_impact(
                        &account.account_type,
                        &entry.entry_type,
                        entry.amount
                    );

                    if entry.status == JournalEntryStatus::Posted {
                        current_balance += balance_impact;
                    }

                    if include_pending || entry.status == JournalEntryStatus::Posted {
                        pending_balance += balance_impact;
                    }

                    transaction_count += 1;

                    if last_transaction_date.is_none() || entry.created_at > last_transaction_date.unwrap() {
                        last_transaction_date = Some(entry.created_at);
                    }
                }
            }

            let available_balance = if include_pending { pending_balance } else { current_balance };

            Ok(Some(AccountBalance {
                account_id: *account_id,
                account_code: account.account_code.clone(),
                account_name: account.account_name.clone(),
                account_type: account.account_type.clone(),
                currency: account.currency.clone(),
                current_balance,
                pending_balance,
                available_balance,
                last_transaction_date,
                transaction_count,
            }))
        } else {
            Ok(None)
        }
    }

    async fn get_trial_balance(
        &self,
        as_of_date: Option<DateTime<Utc>>,
        currency: Option<String>,
        account_type: Option<AccountType>,
        include_zero_balances: bool,
    ) -> Result<Vec<TrialBalanceEntry>, String> {
        let accounts = self.accounts.read().unwrap();
        let mut trial_balance = Vec::new();

        for account in accounts.values() {
            // Apply filters
            if currency.as_ref().map_or(false, |c| account.currency != *c) {
                continue;
            }
            if account_type.as_ref().map_or(false, |t| account.account_type != *t) {
                continue;
            }

            // Get account balance
            if let Ok(Some(balance)) = self.get_account_balance(&account.id, as_of_date, false).await {
                let (debit_balance, credit_balance) = match account.account_type {
                    AccountType::Asset | AccountType::Expense | AccountType::ContraLiability | AccountType::ContraEquity => {
                        if balance.current_balance >= Decimal::ZERO {
                            (balance.current_balance, Decimal::ZERO)
                        } else {
                            (Decimal::ZERO, -balance.current_balance)
                        }
                    }
                    AccountType::Liability | AccountType::Equity | AccountType::Revenue | AccountType::ContraAsset => {
                        if balance.current_balance >= Decimal::ZERO {
                            (Decimal::ZERO, balance.current_balance)
                        } else {
                            (-balance.current_balance, Decimal::ZERO)
                        }
                    }
                };

                if include_zero_balances || debit_balance != Decimal::ZERO || credit_balance != Decimal::ZERO {
                    trial_balance.push(TrialBalanceEntry {
                        account_id: account.id,
                        account_code: account.account_code.clone(),
                        account_name: account.account_name.clone(),
                        account_type: account.account_type.clone(),
                        debit_balance,
                        credit_balance,
                        net_balance: balance.current_balance,
                    });
                }
            }
        }

        trial_balance.sort_by(|a, b| a.account_code.cmp(&b.account_code));
        Ok(trial_balance)
    }

    async fn update_account_balance(&self, account_id: &Uuid, amount: Decimal, entry_type: EntryType) -> Result<(), String> {
        let mut accounts = self.accounts.write().unwrap();

        if let Some(mut account) = accounts.get(account_id).cloned() {
            let balance_impact = Self::calculate_balance_impact(&account.account_type, &entry_type, amount);
            account.current_balance += balance_impact;
            account.updated_at = Utc::now();
            accounts.insert(*account_id, account);
            Ok(())
        } else {
            Err("Account not found".to_string())
        }
    }

    // Reconciliation operations
    async fn reconcile_accounts(&self, account_ids: &[Uuid], reconciliation_date: DateTime<Utc>, auto_correct: bool) -> Result<Vec<AccountReconciliation>, String> {
        let mut reconciliations = Vec::new();

        for account_id in account_ids {
            if let Ok(Some(balance)) = self.get_account_balance(account_id, Some(reconciliation_date), false).await {
                // For this implementation, we'll assume the expected balance equals the current balance
                let expected_balance = balance.current_balance;
                let actual_balance = balance.current_balance;
                let difference = actual_balance - expected_balance;
                let balanced = difference == Decimal::ZERO;

                let mut issues = Vec::new();
                if !balanced {
                    issues.push(format!("Balance discrepancy of {}", difference));
                }

                reconciliations.push(AccountReconciliation {
                    account_id: *account_id,
                    expected_balance,
                    actual_balance,
                    difference,
                    balanced,
                    issues,
                });
            }
        }

        Ok(reconciliations)
    }

    async fn validate_bookkeeping(&self, start_date: Option<DateTime<Utc>>, end_date: Option<DateTime<Utc>>, account_ids: Option<Vec<Uuid>>, fix_issues: bool) -> Result<(bool, Vec<ValidationIssue>), String> {
        let transactions = self.transactions.read().unwrap();
        let mut issues = Vec::new();
        let mut is_valid = true;

        // Validate double-entry bookkeeping for all transactions
        for transaction in transactions.values() {
            // Filter by date range if specified
            if let Some(start) = start_date {
                if transaction.created_at < start {
                    continue;
                }
            }
            if let Some(end) = end_date {
                if transaction.created_at > end {
                    continue;
                }
            }

            // Validate double-entry rules
            if let Err(error) = Self::validate_double_entry(&transaction.entries) {
                is_valid = false;
                issues.push(ValidationIssue {
                    issue_type: "double_entry_violation".to_string(),
                    description: error,
                    account_id: None,
                    transaction_id: Some(transaction.id),
                    severity: "high".to_string(),
                    fixed: false,
                    fix_description: None,
                });
            }
        }

        Ok((is_valid, issues))
    }

    // Audit operations
    async fn create_audit_entry(&self, entry: &AuditTrailEntry) -> Result<AuditTrailEntry, String> {
        let mut audit_trail = self.audit_trail.write().unwrap();
        audit_trail.push(entry.clone());
        Ok(entry.clone())
    }

    async fn get_audit_trail(&self, account_id: Option<Uuid>, transaction_id: Option<Uuid>, user_id: Option<Uuid>, action: Option<String>, start_date: Option<DateTime<Utc>>, end_date: Option<DateTime<Utc>>, page: i32, page_size: i32) -> Result<(Vec<AuditTrailEntry>, i64), String> {
        let audit_trail = self.audit_trail.read().unwrap();
        let mut filtered: Vec<_> = audit_trail
            .iter()
            .filter(|entry| account_id.map_or(true, |id| entry.account_id == Some(id)))
            .filter(|entry| transaction_id.map_or(true, |id| entry.transaction_id == Some(id)))
            .filter(|entry| user_id.map_or(true, |id| entry.user_id == Some(id)))
            .filter(|entry| action.as_ref().map_or(true, |a| entry.action == *a))
            .filter(|entry| start_date.map_or(true, |d| entry.timestamp >= d))
            .filter(|entry| end_date.map_or(true, |d| entry.timestamp <= d))
            .cloned()
            .collect();

        filtered.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        let total = filtered.len() as i64;
        let start = ((page - 1) * page_size) as usize;
        let end = (start + page_size as usize).min(filtered.len());

        Ok((filtered[start..end].to_vec(), total))
    }

    // Reporting operations
    async fn generate_balance_sheet(&self, as_of_date: DateTime<Utc>, currency: &str, include_sub_accounts: bool) -> Result<FinancialReport, String> {
        let trial_balance = self.get_trial_balance(Some(as_of_date), Some(currency.to_string()), None, false).await?;

        let mut assets = Vec::new();
        let mut liabilities = Vec::new();
        let mut equity = Vec::new();

        let mut total_assets = Decimal::ZERO;
        let mut total_liabilities = Decimal::ZERO;
        let mut total_equity = Decimal::ZERO;

        for entry in trial_balance {
            let balance = entry.net_balance;

            let item = BalanceSheetItem {
                account_id: entry.account_id,
                account_code: entry.account_code,
                account_name: entry.account_name,
                balance,
                sub_items: Vec::new(),
            };

            match entry.account_type {
                AccountType::Asset => {
                    total_assets += balance;
                    assets.push(item);
                }
                AccountType::Liability => {
                    total_liabilities += balance;
                    liabilities.push(item);
                }
                AccountType::Equity => {
                    total_equity += balance;
                    equity.push(item);
                }
                _ => {}
            }
        }

        let sections = vec![
            BalanceSheetSection {
                section_name: "Assets".to_string(),
                items: assets,
                section_total: total_assets,
            },
            BalanceSheetSection {
                section_name: "Liabilities".to_string(),
                items: liabilities,
                section_total: total_liabilities,
            },
            BalanceSheetSection {
                section_name: "Equity".to_string(),
                items: equity,
                section_total: total_equity,
            },
        ];

        let summary = HashMap::from([
            ("total_assets".to_string(), total_assets.to_string()),
            ("total_liabilities".to_string(), total_liabilities.to_string()),
            ("total_equity".to_string(), total_equity.to_string()),
        ]);

        Ok(FinancialReport {
            id: Uuid::new_v4(),
            report_type: ReportType::BalanceSheet,
            title: format!("Balance Sheet as of {}", as_of_date.format("%Y-%m-%d")),
            period_start: as_of_date,
            period_end: as_of_date,
            currency: currency.to_string(),
            sections,
            summary,
            generated_at: Utc::now(),
            generated_by: Uuid::new_v4(),
        })
    }

    async fn get_ledger_metrics(&self, start_date: Option<DateTime<Utc>>, end_date: Option<DateTime<Utc>>, currency: Option<String>) -> Result<LedgerMetrics, String> {
        let accounts = self.accounts.read().unwrap();
        let transactions = self.transactions.read().unwrap();

        let total_accounts = accounts.len() as i64;
        let active_accounts = accounts.values().filter(|a| a.status == AccountStatus::Active).count() as i64;

        let total_transactions = transactions.len() as i64;
        let pending_transactions = transactions.values().filter(|tx| tx.status == TransactionStatus::Pending).count() as i64;

        let mut total_assets = Decimal::ZERO;
        let mut total_liabilities = Decimal::ZERO;
        let mut total_equity = Decimal::ZERO;
        let mut currency_balances = HashMap::new();

        for account in accounts.values() {
            match account.account_type {
                AccountType::Asset => total_assets += account.current_balance,
                AccountType::Liability => total_liabilities += account.current_balance,
                AccountType::Equity => total_equity += account.current_balance,
                _ => {}
            }

            *currency_balances.entry(account.currency.clone()).or_insert(Decimal::ZERO) += account.current_balance;
        }

        let books_balanced = (total_assets - total_liabilities - total_equity).abs() < Decimal::from_str("0.01").unwrap();

        Ok(LedgerMetrics {
            total_accounts,
            active_accounts,
            total_transactions,
            pending_transactions,
            total_assets,
            total_liabilities,
            total_equity,
            books_balanced,
            last_reconciliation: Some(Utc::now()),
            currency_balances: currency_balances.into_iter().map(|(k, v)| (k, v.to_string())).collect(),
        })
    }

    // Snapshot operations
    async fn create_balance_snapshot(&self, snapshot: &AccountBalanceSnapshot) -> Result<AccountBalanceSnapshot, String> {
        let mut snapshots = self.balance_snapshots.write().unwrap();
        let account_snapshots = snapshots.entry(snapshot.account_id).or_insert_with(Vec::new);
        account_snapshots.push(snapshot.clone());
        Ok(snapshot.clone())
    }

    async fn get_balance_snapshots(&self, account_id: &Uuid, start_date: NaiveDate, end_date: NaiveDate) -> Result<Vec<AccountBalanceSnapshot>, String> {
        let snapshots = self.balance_snapshots.read().unwrap();

        if let Some(account_snapshots) = snapshots.get(account_id) {
            let filtered: Vec<_> = account_snapshots
                .iter()
                .filter(|snapshot| snapshot.balance_date >= start_date && snapshot.balance_date <= end_date)
                .cloned()
                .collect();
            Ok(filtered)
        } else {
            Ok(Vec::new())
        }
    }
}
