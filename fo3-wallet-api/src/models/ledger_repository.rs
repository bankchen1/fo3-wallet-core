//! Additional repository implementation methods for LedgerRepository

use super::ledger::*;
use std::collections::HashMap;
use uuid::Uuid;
use rust_decimal::Decimal;
use chrono::{DateTime, Utc, NaiveDate};

impl InMemoryLedgerRepository {
    // Continue transaction operations
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
        page_size: i32
    ) -> Result<(Vec<LedgerTransaction>, i64), String> {
        let transactions = self.transactions.read().unwrap();
        let mut filtered_transactions: Vec<LedgerTransaction> = transactions
            .values()
            .filter(|tx| {
                // Filter by account_id (check if any journal entry matches)
                account_id.map_or(true, |aid| tx.entries.iter().any(|e| e.account_id == aid)) &&
                status.as_ref().map_or(true, |s| tx.status == *s) &&
                transaction_type.as_ref().map_or(true, |t| tx.transaction_type == *t) &&
                currency.as_ref().map_or(true, |c| tx.currency == *c) &&
                start_date.map_or(true, |sd| tx.created_at >= sd) &&
                end_date.map_or(true, |ed| tx.created_at <= ed) &&
                source_service.as_ref().map_or(true, |ss| tx.source_service.as_ref() == Some(ss))
            })
            .cloned()
            .collect();

        // Sort by created_at descending
        filtered_transactions.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        let total_count = filtered_transactions.len() as i64;
        let start = ((page - 1) * page_size) as usize;
        let end = (start + page_size as usize).min(filtered_transactions.len());
        
        let paginated_transactions = if start < filtered_transactions.len() {
            filtered_transactions[start..end].to_vec()
        } else {
            Vec::new()
        };

        Ok((paginated_transactions, total_count))
    }

    async fn update_transaction(&self, transaction: &LedgerTransaction) -> Result<LedgerTransaction, String> {
        let mut transactions = self.transactions.write().unwrap();
        let mut journal_entries = self.journal_entries.write().unwrap();
        
        // Update transaction
        transactions.insert(transaction.id, transaction.clone());
        
        // Update journal entries
        for entry in &transaction.entries {
            journal_entries.insert(entry.id, entry.clone());
        }
        
        Ok(transaction.clone())
    }

    async fn post_transaction(&self, id: &Uuid) -> Result<LedgerTransaction, String> {
        let mut transactions = self.transactions.write().unwrap();
        let mut journal_entries = self.journal_entries.write().unwrap();
        let mut accounts = self.accounts.write().unwrap();
        
        if let Some(mut transaction) = transactions.get(id).cloned() {
            // Validate double-entry before posting
            Self::validate_double_entry(&transaction.entries)?;
            
            // Update transaction status
            transaction.status = TransactionStatus::Posted;
            transaction.posted_at = Some(Utc::now());
            transaction.updated_at = Utc::now();
            
            // Update journal entries and account balances
            for mut entry in &mut transaction.entries {
                entry.status = JournalEntryStatus::Posted;
                entry.posted_at = Some(Utc::now());
                entry.updated_at = Utc::now();
                
                // Update account balance
                if let Some(mut account) = accounts.get(&entry.account_id).cloned() {
                    let balance_impact = Self::calculate_balance_impact(&account.account_type, &entry.entry_type, entry.amount);
                    account.current_balance += balance_impact;
                    account.updated_at = Utc::now();
                    accounts.insert(entry.account_id, account);
                }
                
                journal_entries.insert(entry.id, entry.clone());
            }
            
            transactions.insert(*id, transaction.clone());
            Ok(transaction)
        } else {
            Err("Transaction not found".to_string())
        }
    }

    async fn reverse_transaction(&self, id: &Uuid, reason: &str, description: &str) -> Result<(LedgerTransaction, LedgerTransaction), String> {
        let mut transactions = self.transactions.write().unwrap();
        let mut journal_entries = self.journal_entries.write().unwrap();
        let mut accounts = self.accounts.write().unwrap();
        
        if let Some(mut original_transaction) = transactions.get(id).cloned() {
            // Check if transaction can be reversed
            if original_transaction.status != TransactionStatus::Posted {
                return Err("Only posted transactions can be reversed".to_string());
            }
            
            // Create reversal transaction
            let reversal_id = Uuid::new_v4();
            let mut reversal_entries = Vec::new();
            
            for original_entry in &original_transaction.entries {
                // Reverse the entry type
                let reversed_entry_type = match original_entry.entry_type {
                    EntryType::Debit => EntryType::Credit,
                    EntryType::Credit => EntryType::Debit,
                };
                
                let reversal_entry = JournalEntry {
                    id: Uuid::new_v4(),
                    transaction_id: reversal_id,
                    account_id: original_entry.account_id,
                    entry_type: reversed_entry_type,
                    amount: original_entry.amount,
                    currency: original_entry.currency.clone(),
                    description: format!("Reversal of: {}", original_entry.description),
                    status: JournalEntryStatus::Posted,
                    entry_sequence: original_entry.entry_sequence,
                    metadata: original_entry.metadata.clone(),
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                    posted_at: Some(Utc::now()),
                };
                
                // Update account balance
                if let Some(mut account) = accounts.get(&reversal_entry.account_id).cloned() {
                    let balance_impact = Self::calculate_balance_impact(&account.account_type, &reversal_entry.entry_type, reversal_entry.amount);
                    account.current_balance += balance_impact;
                    account.updated_at = Utc::now();
                    accounts.insert(reversal_entry.account_id, account);
                }
                
                journal_entries.insert(reversal_entry.id, reversal_entry.clone());
                reversal_entries.push(reversal_entry);
            }
            
            let reversal_transaction = LedgerTransaction {
                id: reversal_id,
                reference_number: format!("REV-{}", original_transaction.reference_number),
                status: TransactionStatus::Posted,
                transaction_type: format!("reversal_{}", original_transaction.transaction_type),
                description: description.to_string(),
                currency: original_transaction.currency.clone(),
                total_amount: original_transaction.total_amount,
                entries: reversal_entries,
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
            
            // Store both transactions
            transactions.insert(original_transaction.id, original_transaction.clone());
            transactions.insert(reversal_transaction.id, reversal_transaction.clone());
            
            Ok((original_transaction, reversal_transaction))
        } else {
            Err("Transaction not found".to_string())
        }
    }

    // Journal entry operations
    async fn create_journal_entries(&self, entries: &[JournalEntry]) -> Result<Vec<JournalEntry>, String> {
        let mut journal_entries = self.journal_entries.write().unwrap();
        let mut created_entries = Vec::new();
        
        for entry in entries {
            journal_entries.insert(entry.id, entry.clone());
            created_entries.push(entry.clone());
        }
        
        Ok(created_entries)
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
        page_size: i32
    ) -> Result<(Vec<JournalEntry>, i64), String> {
        let journal_entries = self.journal_entries.read().unwrap();
        let mut filtered_entries: Vec<JournalEntry> = journal_entries
            .values()
            .filter(|entry| {
                transaction_id.map_or(true, |tid| entry.transaction_id == tid) &&
                account_id.map_or(true, |aid| entry.account_id == aid) &&
                status.as_ref().map_or(true, |s| entry.status == *s) &&
                start_date.map_or(true, |sd| entry.created_at >= sd) &&
                end_date.map_or(true, |ed| entry.created_at <= ed)
            })
            .cloned()
            .collect();

        // Sort by created_at descending
        filtered_entries.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        let total_count = filtered_entries.len() as i64;
        let start = ((page - 1) * page_size) as usize;
        let end = (start + page_size as usize).min(filtered_entries.len());
        
        let paginated_entries = if start < filtered_entries.len() {
            filtered_entries[start..end].to_vec()
        } else {
            Vec::new()
        };

        Ok((paginated_entries, total_count))
    }

    async fn post_journal_entry(&self, id: &Uuid, posting_date: Option<DateTime<Utc>>) -> Result<JournalEntry, String> {
        let mut journal_entries = self.journal_entries.write().unwrap();
        let mut accounts = self.accounts.write().unwrap();
        
        if let Some(mut entry) = journal_entries.get(id).cloned() {
            // Update entry status
            entry.status = JournalEntryStatus::Posted;
            entry.posted_at = posting_date.or_else(|| Some(Utc::now()));
            entry.updated_at = Utc::now();
            
            // Update account balance
            if let Some(mut account) = accounts.get(&entry.account_id).cloned() {
                let balance_impact = Self::calculate_balance_impact(&account.account_type, &entry.entry_type, entry.amount);
                account.current_balance += balance_impact;
                account.updated_at = Utc::now();
                accounts.insert(entry.account_id, account);
            }
            
            journal_entries.insert(*id, entry.clone());
            Ok(entry)
        } else {
            Err("Journal entry not found".to_string())
        }
    }
}
