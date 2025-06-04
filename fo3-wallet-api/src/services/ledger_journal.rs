//! Journal entry operations for LedgerService

use super::ledger::LedgerServiceImpl;
use std::collections::HashMap;
use tonic::{Request, Response, Status};
use uuid::Uuid;
use rust_decimal::Decimal;
use chrono::{DateTime, Utc};

use crate::proto::fo3::wallet::v1::{
    ledger_service_server::LedgerService,
    *,
};
use crate::models::ledger::{
    JournalEntry, JournalEntryStatus, EntryType,
};
use crate::models::notifications::NotificationType;

impl LedgerService for LedgerServiceImpl {
    /// Create a journal entry
    async fn create_journal_entry(
        &self,
        request: Request<CreateJournalEntryRequest>,
    ) -> Result<Response<CreateJournalEntryResponse>, Status> {
        let req = request.get_ref();
        
        // Parse request parameters
        let account_id = Uuid::parse_str(&req.account_id)
            .map_err(|_| Status::invalid_argument("Invalid account ID"))?;
        let transaction_id = Uuid::parse_str(&req.transaction_id)
            .map_err(|_| Status::invalid_argument("Invalid transaction ID"))?;
        let entry_type = Self::proto_to_entry_type(req.entry_type)?;
        let amount = Decimal::from_str(&req.amount)
            .map_err(|_| Status::invalid_argument("Invalid amount"))?;

        // Validate request with security guard
        let auth_context = self.ledger_guard
            .validate_journal_entry_creation(&request, &account_id, &transaction_id, &amount)
            .await?;

        // Create journal entry
        let journal_entry = JournalEntry {
            id: Uuid::new_v4(),
            transaction_id,
            account_id,
            entry_type,
            amount,
            currency: req.currency.clone(),
            description: req.description.clone(),
            status: JournalEntryStatus::Draft,
            entry_sequence: req.entry_sequence,
            metadata: req.metadata.clone(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            posted_at: None,
        };

        // Save journal entry
        let created_entry = self.ledger_repository
            .create_journal_entry(&journal_entry)
            .await
            .map_err(|e| Status::internal(format!("Failed to create journal entry: {}", e)))?;

        // Record audit entry
        self.record_audit_entry(
            Some(transaction_id),
            Some(account_id),
            "journal_entry_created",
            None,
            Some(serde_json::to_string(&created_entry).unwrap_or_default()),
            &auth_context.user_id,
            request.remote_addr().map(|addr| addr.to_string()),
        ).await;

        // Log the operation
        self.audit_logger.log_operation(
            &auth_context.user_id.to_string(),
            "create_journal_entry",
            &format!("Created journal entry: {} {} for account {}", 
                entry_type, amount, account_id),
            true,
            request.remote_addr(),
        ).await;

        Ok(Response::new(CreateJournalEntryResponse {
            entry: Some(self.journal_entry_to_proto(&created_entry)),
        }))
    }

    /// Get a journal entry by ID
    async fn get_journal_entry(
        &self,
        request: Request<GetJournalEntryRequest>,
    ) -> Result<Response<GetJournalEntryResponse>, Status> {
        let req = request.get_ref();
        
        // Extract authentication
        let auth_context = self.auth_service.extract_auth(&request).await?;
        self.auth_service.check_permission(&auth_context, Permission::PermissionLedgerRead)?;

        // Parse entry ID
        let entry_id = Uuid::parse_str(&req.entry_id)
            .map_err(|_| Status::invalid_argument("Invalid entry ID"))?;

        // Get journal entry
        let entry = self.ledger_repository
            .get_journal_entry(&entry_id)
            .await
            .map_err(|e| Status::internal(format!("Failed to get journal entry: {}", e)))?;

        match entry {
            Some(entry) => {
                Ok(Response::new(GetJournalEntryResponse {
                    entry: Some(self.journal_entry_to_proto(&entry)),
                }))
            }
            None => Err(Status::not_found("Journal entry not found")),
        }
    }

    /// List journal entries
    async fn list_journal_entries(
        &self,
        request: Request<ListJournalEntriesRequest>,
    ) -> Result<Response<ListJournalEntriesResponse>, Status> {
        let req = request.get_ref();
        
        // Extract authentication
        let auth_context = self.auth_service.extract_auth(&request).await?;
        self.auth_service.check_permission(&auth_context, Permission::PermissionLedgerRead)?;

        // Parse filters
        let account_id = if req.account_id.is_empty() {
            None
        } else {
            Some(Uuid::parse_str(&req.account_id)
                .map_err(|_| Status::invalid_argument("Invalid account ID"))?)
        };

        let transaction_id = if req.transaction_id.is_empty() {
            None
        } else {
            Some(Uuid::parse_str(&req.transaction_id)
                .map_err(|_| Status::invalid_argument("Invalid transaction ID"))?)
        };

        let status = if req.status != 0 {
            Some(Self::proto_to_journal_entry_status(req.status)?)
        } else {
            None
        };

        let entry_type = if req.entry_type != 0 {
            Some(Self::proto_to_entry_type(req.entry_type)?)
        } else {
            None
        };

        let start_date = if req.start_date.is_empty() {
            None
        } else {
            Some(DateTime::parse_from_rfc3339(&req.start_date)
                .map_err(|_| Status::invalid_argument("Invalid start date"))?
                .with_timezone(&Utc))
        };

        let end_date = if req.end_date.is_empty() {
            None
        } else {
            Some(DateTime::parse_from_rfc3339(&req.end_date)
                .map_err(|_| Status::invalid_argument("Invalid end date"))?
                .with_timezone(&Utc))
        };

        let page = if req.page > 0 { req.page } else { 1 };
        let page_size = if req.page_size > 0 && req.page_size <= 100 { req.page_size } else { 20 };

        // Get journal entries
        let (entries, total_count) = self.ledger_repository
            .list_journal_entries(account_id, transaction_id, status, entry_type, start_date, end_date, page, page_size)
            .await
            .map_err(|e| Status::internal(format!("Failed to list journal entries: {}", e)))?;

        let proto_entries = entries.iter()
            .map(|entry| self.journal_entry_to_proto(entry))
            .collect();

        Ok(Response::new(ListJournalEntriesResponse {
            entries: proto_entries,
            total_count: total_count as i32,
            page,
            page_size,
        }))
    }

    /// Post a journal entry
    async fn post_journal_entry(
        &self,
        request: Request<PostJournalEntryRequest>,
    ) -> Result<Response<PostJournalEntryResponse>, Status> {
        let req = request.get_ref();
        
        // Parse entry ID
        let entry_id = Uuid::parse_str(&req.entry_id)
            .map_err(|_| Status::invalid_argument("Invalid entry ID"))?;

        // Validate request with security guard
        let auth_context = self.ledger_guard
            .validate_journal_entry_posting(&request, &entry_id)
            .await?;

        // Post journal entry
        let posted_entry = self.ledger_repository
            .post_journal_entry(&entry_id)
            .await
            .map_err(|e| Status::internal(format!("Failed to post journal entry: {}", e)))?;

        // Record audit entry
        self.record_audit_entry(
            Some(posted_entry.transaction_id),
            Some(posted_entry.account_id),
            "journal_entry_posted",
            None,
            Some(serde_json::to_string(&posted_entry).unwrap_or_default()),
            &auth_context.user_id,
            request.remote_addr().map(|addr| addr.to_string()),
        ).await;

        // Log the operation
        self.audit_logger.log_operation(
            &auth_context.user_id.to_string(),
            "post_journal_entry",
            &format!("Posted journal entry: {} {} for account {}", 
                posted_entry.entry_type, posted_entry.amount, posted_entry.account_id),
            true,
            request.remote_addr(),
        ).await;

        // Send notification
        self.send_ledger_notification(
            &auth_context.user_id,
            NotificationType::JournalEntryPosted,
            "Journal Entry Posted",
            &format!("Journal entry for {} {} has been posted.", 
                posted_entry.amount, posted_entry.currency),
            HashMap::from([
                ("entry_id".to_string(), posted_entry.id.to_string()),
                ("transaction_id".to_string(), posted_entry.transaction_id.to_string()),
                ("account_id".to_string(), posted_entry.account_id.to_string()),
                ("amount".to_string(), posted_entry.amount.to_string()),
            ]),
        ).await;

        Ok(Response::new(PostJournalEntryResponse {
            entry: Some(self.journal_entry_to_proto(&posted_entry)),
            success: true,
        }))
    }

    /// Get trial balance
    async fn get_trial_balance(
        &self,
        request: Request<GetTrialBalanceRequest>,
    ) -> Result<Response<GetTrialBalanceResponse>, Status> {
        let req = request.get_ref();
        
        // Extract authentication
        let auth_context = self.auth_service.extract_auth(&request).await?;
        self.auth_service.check_permission(&auth_context, Permission::PermissionLedgerRead)?;

        // Parse as-of date
        let as_of_date = if req.as_of_date.is_empty() {
            Utc::now()
        } else {
            DateTime::parse_from_rfc3339(&req.as_of_date)
                .map_err(|_| Status::invalid_argument("Invalid as-of date"))?
                .with_timezone(&Utc)
        };

        // Get trial balance
        let trial_balance_entries = self.ledger_repository
            .get_trial_balance(&as_of_date, req.include_zero_balances, req.currency.clone())
            .await
            .map_err(|e| Status::internal(format!("Failed to get trial balance: {}", e)))?;

        // Convert to proto
        let proto_entries = trial_balance_entries.iter().map(|entry| {
            crate::proto::fo3::wallet::v1::TrialBalanceEntry {
                account_id: entry.account_id.to_string(),
                account_code: entry.account_code.clone(),
                account_name: entry.account_name.clone(),
                account_type: Self::account_type_to_proto(&entry.account_type),
                debit_balance: entry.debit_balance.to_string(),
                credit_balance: entry.credit_balance.to_string(),
                currency: entry.currency.clone(),
            }
        }).collect();

        // Calculate totals
        let total_debits: Decimal = trial_balance_entries.iter().map(|e| e.debit_balance).sum();
        let total_credits: Decimal = trial_balance_entries.iter().map(|e| e.credit_balance).sum();
        let is_balanced = total_debits == total_credits;

        Ok(Response::new(GetTrialBalanceResponse {
            entries: proto_entries,
            total_debits: total_debits.to_string(),
            total_credits: total_credits.to_string(),
            is_balanced,
            as_of_date: as_of_date.to_rfc3339(),
        }))
    }
}
