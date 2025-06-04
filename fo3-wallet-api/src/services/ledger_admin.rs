//! Admin and audit operations for LedgerService

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
    LedgerMetrics, AuditTrailEntry,
};
use crate::models::notifications::NotificationType;

impl LedgerService for LedgerServiceImpl {
    /// Get audit trail
    async fn get_audit_trail(
        &self,
        request: Request<GetAuditTrailRequest>,
    ) -> Result<Response<GetAuditTrailResponse>, Status> {
        let req = request.get_ref();
        
        // Extract authentication
        let auth_context = self.auth_service.extract_auth(&request).await?;
        self.auth_service.check_permission(&auth_context, Permission::PermissionLedgerAudit)?;

        // Parse filters
        let transaction_id = if req.transaction_id.is_empty() {
            None
        } else {
            Some(Uuid::parse_str(&req.transaction_id)
                .map_err(|_| Status::invalid_argument("Invalid transaction ID"))?)
        };

        let account_id = if req.account_id.is_empty() {
            None
        } else {
            Some(Uuid::parse_str(&req.account_id)
                .map_err(|_| Status::invalid_argument("Invalid account ID"))?)
        };

        let user_id = if req.user_id.is_empty() {
            None
        } else {
            Some(Uuid::parse_str(&req.user_id)
                .map_err(|_| Status::invalid_argument("Invalid user ID"))?)
        };

        let action = if req.action.is_empty() { None } else { Some(req.action.clone()) };

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

        // Get audit trail entries
        let (entries, total_count) = self.ledger_repository
            .get_audit_trail(transaction_id, account_id, user_id, action, start_date, end_date, page, page_size)
            .await
            .map_err(|e| Status::internal(format!("Failed to get audit trail: {}", e)))?;

        // Convert to proto
        let proto_entries = entries.iter().map(|entry| {
            crate::proto::fo3::wallet::v1::AuditTrailEntry {
                id: entry.id.to_string(),
                transaction_id: entry.transaction_id.map(|id| id.to_string()).unwrap_or_default(),
                account_id: entry.account_id.map(|id| id.to_string()).unwrap_or_default(),
                user_id: entry.user_id.to_string(),
                action: entry.action.clone(),
                old_value: entry.old_value.clone().unwrap_or_default(),
                new_value: entry.new_value.clone().unwrap_or_default(),
                ip_address: entry.ip_address.clone().unwrap_or_default(),
                user_agent: entry.user_agent.clone().unwrap_or_default(),
                created_at: entry.created_at.to_rfc3339(),
            }
        }).collect();

        Ok(Response::new(GetAuditTrailResponse {
            entries: proto_entries,
            total_count: total_count as i32,
            page,
            page_size,
        }))
    }

    /// Export ledger data
    async fn export_ledger_data(
        &self,
        request: Request<ExportLedgerDataRequest>,
    ) -> Result<Response<ExportLedgerDataResponse>, Status> {
        let req = request.get_ref();
        
        // Validate admin operation
        let auth_context = self.ledger_guard
            .validate_admin_operation(&request, "export_ledger_data")
            .await?;

        // Parse date range
        let start_date = DateTime::parse_from_rfc3339(&req.start_date)
            .map_err(|_| Status::invalid_argument("Invalid start date"))?
            .with_timezone(&Utc);
        let end_date = DateTime::parse_from_rfc3339(&req.end_date)
            .map_err(|_| Status::invalid_argument("Invalid end date"))?
            .with_timezone(&Utc);

        // Parse export format
        let export_format = req.format.to_lowercase();
        if !["csv", "json", "xlsx"].contains(&export_format.as_str()) {
            return Err(Status::invalid_argument("Unsupported export format"));
        }

        // Generate export data
        let export_data = self.ledger_repository
            .export_ledger_data(&start_date, &end_date, &export_format, req.include_audit_trail)
            .await
            .map_err(|e| Status::internal(format!("Failed to export ledger data: {}", e)))?;

        // Generate download URL (in real implementation, this would upload to cloud storage)
        let download_url = format!("https://exports.fo3wallet.com/ledger/export_{}_{}.{}", 
            start_date.format("%Y%m%d"), 
            end_date.format("%Y%m%d"), 
            export_format);

        // Record audit entry
        self.record_audit_entry(
            None,
            None,
            "ledger_data_exported",
            None,
            Some(format!("Exported ledger data from {} to {} in {} format", 
                start_date.to_rfc3339(), end_date.to_rfc3339(), export_format)),
            &auth_context.user_id,
            request.remote_addr().map(|addr| addr.to_string()),
        ).await;

        // Log the operation
        self.audit_logger.log_operation(
            &auth_context.user_id.to_string(),
            "export_ledger_data",
            &format!("Exported ledger data: {} records in {} format", 
                export_data.record_count, export_format),
            true,
            request.remote_addr(),
        ).await;

        Ok(Response::new(ExportLedgerDataResponse {
            download_url,
            file_size: export_data.file_size,
            record_count: export_data.record_count as i32,
            format: export_format,
            expires_at: (Utc::now() + chrono::Duration::hours(24)).to_rfc3339(),
        }))
    }

    /// Get ledger metrics
    async fn get_ledger_metrics(
        &self,
        request: Request<GetLedgerMetricsRequest>,
    ) -> Result<Response<GetLedgerMetricsResponse>, Status> {
        let req = request.get_ref();
        
        // Extract authentication
        let auth_context = self.auth_service.extract_auth(&request).await?;
        self.auth_service.check_permission(&auth_context, Permission::PermissionLedgerRead)?;

        // Parse date range
        let start_date = if req.start_date.is_empty() {
            Utc::now() - chrono::Duration::days(30) // Default to last 30 days
        } else {
            DateTime::parse_from_rfc3339(&req.start_date)
                .map_err(|_| Status::invalid_argument("Invalid start date"))?
                .with_timezone(&Utc)
        };

        let end_date = if req.end_date.is_empty() {
            Utc::now()
        } else {
            DateTime::parse_from_rfc3339(&req.end_date)
                .map_err(|_| Status::invalid_argument("Invalid end date"))?
                .with_timezone(&Utc)
        };

        // Get ledger metrics
        let metrics = self.ledger_repository
            .get_ledger_metrics(&start_date, &end_date)
            .await
            .map_err(|e| Status::internal(format!("Failed to get ledger metrics: {}", e)))?;

        // Convert to proto
        let proto_metrics = crate::proto::fo3::wallet::v1::LedgerMetrics {
            total_accounts: metrics.total_accounts as i32,
            active_accounts: metrics.active_accounts as i32,
            total_transactions: metrics.total_transactions as i32,
            total_journal_entries: metrics.total_journal_entries as i32,
            total_transaction_volume: metrics.total_transaction_volume.to_string(),
            average_transaction_size: metrics.average_transaction_size.to_string(),
            transactions_by_type: metrics.transactions_by_type.into_iter().collect(),
            accounts_by_type: metrics.accounts_by_type.into_iter().map(|(k, v)| {
                (Self::account_type_to_proto(&k), v as i32)
            }).collect(),
            balance_sheet_total: metrics.balance_sheet_total.to_string(),
            trial_balance_status: metrics.trial_balance_status,
            last_reconciliation_date: metrics.last_reconciliation_date.map(|dt| dt.to_rfc3339()).unwrap_or_default(),
            period_start: start_date.to_rfc3339(),
            period_end: end_date.to_rfc3339(),
        };

        Ok(Response::new(GetLedgerMetricsResponse {
            metrics: Some(proto_metrics),
        }))
    }

    /// Perform period close
    async fn perform_period_close(
        &self,
        request: Request<PerformPeriodCloseRequest>,
    ) -> Result<Response<PerformPeriodCloseResponse>, Status> {
        let req = request.get_ref();
        
        // Validate admin operation
        let auth_context = self.ledger_guard
            .validate_admin_operation(&request, "perform_period_close")
            .await?;

        // Parse period end date
        let period_end_date = DateTime::parse_from_rfc3339(&req.period_end_date)
            .map_err(|_| Status::invalid_argument("Invalid period end date"))?
            .with_timezone(&Utc);

        // Perform period close
        let close_result = self.ledger_repository
            .perform_period_close(&period_end_date, req.close_type.clone())
            .await
            .map_err(|e| Status::internal(format!("Failed to perform period close: {}", e)))?;

        // Record audit entry
        self.record_audit_entry(
            None,
            None,
            "period_close_performed",
            None,
            Some(format!("Period close performed for {} (type: {})", 
                period_end_date.to_rfc3339(), req.close_type)),
            &auth_context.user_id,
            request.remote_addr().map(|addr| addr.to_string()),
        ).await;

        // Log the operation
        self.audit_logger.log_operation(
            &auth_context.user_id.to_string(),
            "perform_period_close",
            &format!("Period close completed for {}: {} accounts processed", 
                period_end_date.format("%Y-%m-%d"), close_result.accounts_processed),
            true,
            request.remote_addr(),
        ).await;

        // Send notification
        self.send_ledger_notification(
            &auth_context.user_id,
            NotificationType::PeriodClosed,
            "Period Close Completed",
            &format!("Period close for {} has been completed successfully.", 
                period_end_date.format("%Y-%m-%d")),
            HashMap::from([
                ("period_end_date".to_string(), period_end_date.to_rfc3339()),
                ("accounts_processed".to_string(), close_result.accounts_processed.to_string()),
                ("close_type".to_string(), req.close_type.clone()),
            ]),
        ).await;

        Ok(Response::new(PerformPeriodCloseResponse {
            success: close_result.success,
            accounts_processed: close_result.accounts_processed as i32,
            closing_entries_created: close_result.closing_entries_created as i32,
            period_end_date: period_end_date.to_rfc3339(),
            close_type: req.close_type.clone(),
            message: close_result.message,
        }))
    }

    /// Backup ledger data
    async fn backup_ledger_data(
        &self,
        request: Request<BackupLedgerDataRequest>,
    ) -> Result<Response<BackupLedgerDataResponse>, Status> {
        let req = request.get_ref();
        
        // Validate admin operation
        let auth_context = self.ledger_guard
            .validate_admin_operation(&request, "backup_ledger_data")
            .await?;

        // Perform backup
        let backup_result = self.ledger_repository
            .backup_ledger_data(req.backup_type.clone(), req.include_audit_trail)
            .await
            .map_err(|e| Status::internal(format!("Failed to backup ledger data: {}", e)))?;

        // Generate backup location (in real implementation, this would be cloud storage)
        let backup_location = format!("s3://fo3-backups/ledger/backup_{}_{}.tar.gz", 
            req.backup_type, Utc::now().format("%Y%m%d_%H%M%S"));

        // Record audit entry
        self.record_audit_entry(
            None,
            None,
            "ledger_backup_created",
            None,
            Some(format!("Ledger backup created: {} (type: {})", 
                backup_location, req.backup_type)),
            &auth_context.user_id,
            request.remote_addr().map(|addr| addr.to_string()),
        ).await;

        // Log the operation
        self.audit_logger.log_operation(
            &auth_context.user_id.to_string(),
            "backup_ledger_data",
            &format!("Ledger backup completed: {} MB, {} records", 
                backup_result.backup_size_mb, backup_result.record_count),
            true,
            request.remote_addr(),
        ).await;

        Ok(Response::new(BackupLedgerDataResponse {
            backup_id: backup_result.backup_id,
            backup_location,
            backup_size_mb: backup_result.backup_size_mb as i32,
            record_count: backup_result.record_count as i32,
            backup_type: req.backup_type.clone(),
            created_at: Utc::now().to_rfc3339(),
            success: backup_result.success,
        }))
    }
}
