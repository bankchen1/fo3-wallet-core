//! Additional LedgerService method implementations

use super::ledger::LedgerServiceImpl;
use std::collections::HashMap;
use tonic::{Request, Response, Status};
use uuid::Uuid;
use rust_decimal::Decimal;
use chrono::Utc;

use crate::proto::fo3::wallet::v1::{
    ledger_service_server::LedgerService,
    *,
};
use crate::models::ledger::{
    LedgerTransaction, JournalEntry, TransactionStatus, JournalEntryStatus, EntryType,
};
use crate::models::notifications::NotificationType;

impl LedgerService for LedgerServiceImpl {
    /// Record a new transaction
    async fn record_transaction(
        &self,
        request: Request<RecordTransactionRequest>,
    ) -> Result<Response<RecordTransactionResponse>, Status> {
        let req = request.get_ref();
        
        // Parse and validate request
        let total_amount = Decimal::from_str(&req.total_amount)
            .map_err(|_| Status::invalid_argument("Invalid total amount"))?;

        // Parse journal entries
        let mut journal_entries = Vec::new();
        for (i, entry_req) in req.entries.iter().enumerate() {
            let account_id = Uuid::parse_str(&entry_req.account_id)
                .map_err(|_| Status::invalid_argument("Invalid account ID"))?;
            let entry_type = Self::proto_to_entry_type(entry_req.entry_type)?;
            let amount = Decimal::from_str(&entry_req.amount)
                .map_err(|_| Status::invalid_argument("Invalid entry amount"))?;

            let journal_entry = JournalEntry {
                id: Uuid::new_v4(),
                transaction_id: Uuid::new_v4(), // Will be updated with actual transaction ID
                account_id,
                entry_type,
                amount,
                currency: req.currency.clone(),
                description: entry_req.description.clone(),
                status: JournalEntryStatus::Draft,
                entry_sequence: i as i32 + 1,
                metadata: entry_req.metadata.clone(),
                created_at: Utc::now(),
                updated_at: Utc::now(),
                posted_at: None,
            };
            journal_entries.push(journal_entry);
        }

        // Validate request with security guard
        let auth_context = self.ledger_guard
            .validate_transaction_recording(&request, &req.transaction_type, &journal_entries, &total_amount)
            .await?;

        // Create transaction
        let transaction_id = Uuid::new_v4();
        let reference_number = if req.reference_number.is_empty() {
            Self::generate_reference_number()
        } else {
            req.reference_number.clone()
        };

        // Update journal entries with correct transaction ID
        for entry in &mut journal_entries {
            entry.transaction_id = transaction_id;
        }

        let transaction = LedgerTransaction {
            id: transaction_id,
            reference_number,
            status: TransactionStatus::Pending,
            transaction_type: req.transaction_type.clone(),
            description: req.description.clone(),
            currency: req.currency.clone(),
            total_amount,
            entries: journal_entries,
            source_service: if req.source_service.is_empty() { None } else { Some(req.source_service.clone()) },
            source_transaction_id: if req.source_transaction_id.is_empty() { None } else { Some(req.source_transaction_id.clone()) },
            metadata: req.metadata.clone(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            posted_at: None,
            reversed_at: None,
            reversal_reason: None,
            reversal_transaction_id: None,
        };

        // Save transaction
        let created_transaction = self.ledger_repository
            .create_transaction(&transaction)
            .await
            .map_err(|e| Status::internal(format!("Failed to create transaction: {}", e)))?;

        let mut posted = false;
        let mut validation_errors = Vec::new();

        // Auto-post if requested
        if req.auto_post {
            match self.ledger_repository.post_transaction(&transaction_id).await {
                Ok(posted_transaction) => {
                    posted = true;
                    // Update the created transaction with posted status
                    // In a real implementation, we'd return the updated transaction
                }
                Err(e) => {
                    validation_errors.push(format!("Failed to post transaction: {}", e));
                }
            }
        }

        // Record audit entry
        self.record_audit_entry(
            Some(created_transaction.id),
            None,
            "transaction_recorded",
            None,
            Some(serde_json::to_string(&created_transaction).unwrap_or_default()),
            &auth_context.user_id,
            request.remote_addr().map(|addr| addr.to_string()),
        ).await;

        // Log the operation
        self.audit_logger.log_operation(
            &auth_context.user_id.to_string(),
            "record_transaction",
            &format!("Recorded transaction: {} ({})", created_transaction.reference_number, created_transaction.transaction_type),
            true,
            request.remote_addr(),
        ).await;

        // Send notification
        self.send_ledger_notification(
            &auth_context.user_id,
            NotificationType::TransactionRecorded,
            "Transaction Recorded",
            &format!("Transaction '{}' has been recorded in the ledger.", created_transaction.reference_number),
            HashMap::from([
                ("transaction_id".to_string(), created_transaction.id.to_string()),
                ("reference_number".to_string(), created_transaction.reference_number.clone()),
                ("posted".to_string(), posted.to_string()),
            ]),
        ).await;

        Ok(Response::new(RecordTransactionResponse {
            transaction: Some(self.ledger_transaction_to_proto(&created_transaction)),
            posted,
            validation_errors,
        }))
    }

    /// Get a transaction by ID
    async fn get_transaction(
        &self,
        request: Request<GetTransactionRequest>,
    ) -> Result<Response<GetTransactionResponse>, Status> {
        let req = request.get_ref();
        
        // Extract authentication
        let auth_context = self.auth_service.extract_auth(&request).await?;
        self.auth_service.check_permission(&auth_context, Permission::PermissionLedgerRead)?;

        // Parse transaction ID
        let transaction_id = Uuid::parse_str(&req.transaction_id)
            .map_err(|_| Status::invalid_argument("Invalid transaction ID"))?;

        // Get transaction
        let transaction = self.ledger_repository
            .get_transaction(&transaction_id)
            .await
            .map_err(|e| Status::internal(format!("Failed to get transaction: {}", e)))?;

        match transaction {
            Some(transaction) => {
                Ok(Response::new(GetTransactionResponse {
                    transaction: Some(self.ledger_transaction_to_proto(&transaction)),
                }))
            }
            None => Err(Status::not_found("Transaction not found")),
        }
    }

    /// List transactions
    async fn list_transactions(
        &self,
        request: Request<ListTransactionsRequest>,
    ) -> Result<Response<ListTransactionsResponse>, Status> {
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

        let status = if req.status != 0 {
            Some(Self::proto_to_transaction_status(req.status)?)
        } else {
            None
        };

        let transaction_type = if req.transaction_type.is_empty() { None } else { Some(req.transaction_type.clone()) };
        let currency = if req.currency.is_empty() { None } else { Some(req.currency.clone()) };
        let source_service = if req.source_service.is_empty() { None } else { Some(req.source_service.clone()) };

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

        // Get transactions
        let (transactions, total_count) = self.ledger_repository
            .list_transactions(account_id, status, transaction_type, currency, start_date, end_date, source_service, page, page_size)
            .await
            .map_err(|e| Status::internal(format!("Failed to list transactions: {}", e)))?;

        let proto_transactions = transactions.iter()
            .map(|transaction| self.ledger_transaction_to_proto(transaction))
            .collect();

        Ok(Response::new(ListTransactionsResponse {
            transactions: proto_transactions,
            total_count: total_count as i32,
            page,
            page_size,
        }))
    }

    /// Reverse a transaction
    async fn reverse_transaction(
        &self,
        request: Request<ReverseTransactionRequest>,
    ) -> Result<Response<ReverseTransactionResponse>, Status> {
        let req = request.get_ref();
        
        // Parse transaction ID
        let transaction_id = Uuid::parse_str(&req.transaction_id)
            .map_err(|_| Status::invalid_argument("Invalid transaction ID"))?;

        // Validate request with security guard
        let auth_context = self.ledger_guard
            .validate_transaction_reversal(&request, &transaction_id, &req.reason)
            .await?;

        // Reverse transaction
        let (original_transaction, reversal_transaction) = self.ledger_repository
            .reverse_transaction(&transaction_id, &req.reason, &req.description)
            .await
            .map_err(|e| Status::internal(format!("Failed to reverse transaction: {}", e)))?;

        // Record audit entry
        self.record_audit_entry(
            Some(original_transaction.id),
            None,
            "transaction_reversed",
            Some(serde_json::to_string(&original_transaction).unwrap_or_default()),
            Some(serde_json::to_string(&reversal_transaction).unwrap_or_default()),
            &auth_context.user_id,
            request.remote_addr().map(|addr| addr.to_string()),
        ).await;

        // Log the operation
        self.audit_logger.log_operation(
            &auth_context.user_id.to_string(),
            "reverse_transaction",
            &format!("Reversed transaction: {} (reason: {})", original_transaction.reference_number, req.reason),
            true,
            request.remote_addr(),
        ).await;

        // Send notification
        self.send_ledger_notification(
            &auth_context.user_id,
            NotificationType::TransactionReversed,
            "Transaction Reversed",
            &format!("Transaction '{}' has been reversed.", original_transaction.reference_number),
            HashMap::from([
                ("original_transaction_id".to_string(), original_transaction.id.to_string()),
                ("reversal_transaction_id".to_string(), reversal_transaction.id.to_string()),
                ("reason".to_string(), req.reason.clone()),
            ]),
        ).await;

        Ok(Response::new(ReverseTransactionResponse {
            original_transaction: Some(self.ledger_transaction_to_proto(&original_transaction)),
            reversal_transaction: Some(self.ledger_transaction_to_proto(&reversal_transaction)),
            success: true,
        }))
    }

    /// Update a ledger account
    async fn update_ledger_account(
        &self,
        request: Request<UpdateLedgerAccountRequest>,
    ) -> Result<Response<UpdateLedgerAccountResponse>, Status> {
        let req = request.get_ref();

        // Parse account ID
        let account_id = Uuid::parse_str(&req.account_id)
            .map_err(|_| Status::invalid_argument("Invalid account ID"))?;

        // Validate request with security guard
        let auth_context = self.ledger_guard
            .validate_account_modification(&request, &account_id, &req.name, &req.description)
            .await?;

        // Get existing account
        let mut account = self.ledger_repository
            .get_account(&account_id)
            .await
            .map_err(|e| Status::internal(format!("Failed to get account: {}", e)))?
            .ok_or_else(|| Status::not_found("Account not found"))?;

        // Update fields
        if !req.name.is_empty() {
            account.name = req.name.clone();
        }
        if !req.description.is_empty() {
            account.description = Some(req.description.clone());
        }
        account.updated_at = Utc::now();

        // Save updated account
        let updated_account = self.ledger_repository
            .update_account(&account)
            .await
            .map_err(|e| Status::internal(format!("Failed to update account: {}", e)))?;

        // Record audit entry
        self.record_audit_entry(
            None,
            Some(account_id),
            "account_updated",
            Some(serde_json::to_string(&account).unwrap_or_default()),
            Some(serde_json::to_string(&updated_account).unwrap_or_default()),
            &auth_context.user_id,
            request.remote_addr().map(|addr| addr.to_string()),
        ).await;

        // Log the operation
        self.audit_logger.log_operation(
            &auth_context.user_id.to_string(),
            "update_ledger_account",
            &format!("Updated account: {} ({})", updated_account.name, updated_account.account_code),
            true,
            request.remote_addr(),
        ).await;

        Ok(Response::new(UpdateLedgerAccountResponse {
            account: Some(self.ledger_account_to_proto(&updated_account)),
            success: true,
        }))
    }

    /// Close a ledger account
    async fn close_ledger_account(
        &self,
        request: Request<CloseLedgerAccountRequest>,
    ) -> Result<Response<CloseLedgerAccountResponse>, Status> {
        let req = request.get_ref();

        // Parse account ID
        let account_id = Uuid::parse_str(&req.account_id)
            .map_err(|_| Status::invalid_argument("Invalid account ID"))?;

        // Validate request with security guard
        let auth_context = self.ledger_guard
            .validate_account_closure(&request, &account_id, &req.reason)
            .await?;

        // Close account
        let closed_account = self.ledger_repository
            .close_account(&account_id, &req.reason)
            .await
            .map_err(|e| Status::internal(format!("Failed to close account: {}", e)))?;

        // Record audit entry
        self.record_audit_entry(
            None,
            Some(account_id),
            "account_closed",
            None,
            Some(format!("Account closed. Reason: {}", req.reason)),
            &auth_context.user_id,
            request.remote_addr().map(|addr| addr.to_string()),
        ).await;

        // Log the operation
        self.audit_logger.log_operation(
            &auth_context.user_id.to_string(),
            "close_ledger_account",
            &format!("Closed account: {} (reason: {})", closed_account.name, req.reason),
            true,
            request.remote_addr(),
        ).await;

        // Send notification
        self.send_ledger_notification(
            &auth_context.user_id,
            NotificationType::AccountClosed,
            "Account Closed",
            &format!("Ledger account '{}' has been closed.", closed_account.name),
            HashMap::from([
                ("account_id".to_string(), account_id.to_string()),
                ("account_name".to_string(), closed_account.name.clone()),
                ("reason".to_string(), req.reason.clone()),
            ]),
        ).await;

        Ok(Response::new(CloseLedgerAccountResponse {
            account: Some(self.ledger_account_to_proto(&closed_account)),
            success: true,
        }))
    }

    /// Get account balance
    async fn get_account_balance(
        &self,
        request: Request<GetAccountBalanceRequest>,
    ) -> Result<Response<GetAccountBalanceResponse>, Status> {
        let req = request.get_ref();

        // Extract authentication
        let auth_context = self.auth_service.extract_auth(&request).await?;
        self.auth_service.check_permission(&auth_context, Permission::PermissionLedgerRead)?;

        // Parse account ID
        let account_id = Uuid::parse_str(&req.account_id)
            .map_err(|_| Status::invalid_argument("Invalid account ID"))?;

        // Parse as-of date
        let as_of_date = if req.as_of_date.is_empty() {
            Utc::now()
        } else {
            DateTime::parse_from_rfc3339(&req.as_of_date)
                .map_err(|_| Status::invalid_argument("Invalid as-of date"))?
                .with_timezone(&Utc)
        };

        // Get account balance
        let balance = self.ledger_repository
            .get_account_balance(&account_id, &as_of_date, req.include_pending)
            .await
            .map_err(|e| Status::internal(format!("Failed to get account balance: {}", e)))?;

        // Convert to proto
        let proto_balance = crate::proto::fo3::wallet::v1::AccountBalance {
            account_id: account_id.to_string(),
            current_balance: balance.current_balance.to_string(),
            pending_balance: balance.pending_balance.to_string(),
            available_balance: balance.available_balance.to_string(),
            currency: balance.currency,
            as_of_date: as_of_date.to_rfc3339(),
            last_transaction_date: balance.last_transaction_date.map(|dt| dt.to_rfc3339()).unwrap_or_default(),
        };

        Ok(Response::new(GetAccountBalanceResponse {
            balance: Some(proto_balance),
        }))
    }
}
