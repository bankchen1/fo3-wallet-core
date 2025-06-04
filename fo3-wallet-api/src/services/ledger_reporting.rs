//! Financial reporting and reconciliation operations for LedgerService

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
    BalanceSheetSection, ReportType, AccountType,
};
use crate::models::notifications::NotificationType;

impl LedgerService for LedgerServiceImpl {
    /// Get balance sheet
    async fn get_balance_sheet(
        &self,
        request: Request<GetBalanceSheetRequest>,
    ) -> Result<Response<GetBalanceSheetResponse>, Status> {
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

        // Get balance sheet data
        let balance_sheet = self.ledger_repository
            .get_balance_sheet(&as_of_date, req.currency.clone())
            .await
            .map_err(|e| Status::internal(format!("Failed to get balance sheet: {}", e)))?;

        // Convert sections to proto
        let proto_sections = balance_sheet.sections.iter().map(|section| {
            crate::proto::fo3::wallet::v1::BalanceSheetSection {
                section_type: Self::account_type_to_proto(&section.section_type),
                section_name: section.section_name.clone(),
                items: section.items.iter().map(|item| {
                    crate::proto::fo3::wallet::v1::BalanceSheetItem {
                        account_id: item.account_id.to_string(),
                        account_code: item.account_code.clone(),
                        account_name: item.account_name.clone(),
                        balance: item.balance.to_string(),
                        currency: item.currency.clone(),
                    }
                }).collect(),
                total_balance: section.total_balance.to_string(),
            }
        }).collect();

        Ok(Response::new(GetBalanceSheetResponse {
            sections: proto_sections,
            total_assets: balance_sheet.total_assets.to_string(),
            total_liabilities: balance_sheet.total_liabilities.to_string(),
            total_equity: balance_sheet.total_equity.to_string(),
            as_of_date: as_of_date.to_rfc3339(),
            currency: balance_sheet.currency,
            is_balanced: balance_sheet.is_balanced,
        }))
    }

    /// Reconcile accounts
    async fn reconcile_accounts(
        &self,
        request: Request<ReconcileAccountsRequest>,
    ) -> Result<Response<ReconcileAccountsResponse>, Status> {
        let req = request.get_ref();
        
        // Validate request with security guard
        let auth_context = self.ledger_guard
            .validate_account_reconciliation(&request, &req.account_ids)
            .await?;

        // Parse account IDs
        let account_ids: Result<Vec<Uuid>, _> = req.account_ids.iter()
            .map(|id| Uuid::parse_str(id))
            .collect();
        let account_ids = account_ids
            .map_err(|_| Status::invalid_argument("Invalid account ID in list"))?;

        // Parse as-of date
        let as_of_date = if req.as_of_date.is_empty() {
            Utc::now()
        } else {
            DateTime::parse_from_rfc3339(&req.as_of_date)
                .map_err(|_| Status::invalid_argument("Invalid as-of date"))?
                .with_timezone(&Utc)
        };

        // Perform reconciliation
        let reconciliation_results = self.ledger_repository
            .reconcile_accounts(&account_ids, &as_of_date)
            .await
            .map_err(|e| Status::internal(format!("Failed to reconcile accounts: {}", e)))?;

        // Convert results to proto
        let proto_results = reconciliation_results.iter().map(|result| {
            crate::proto::fo3::wallet::v1::AccountReconciliation {
                account_id: result.account_id.to_string(),
                account_name: result.account_name.clone(),
                expected_balance: result.expected_balance.to_string(),
                actual_balance: result.actual_balance.to_string(),
                difference: result.difference.to_string(),
                is_reconciled: result.is_reconciled,
                issues: result.issues.iter().map(|issue| {
                    crate::proto::fo3::wallet::v1::ValidationIssue {
                        issue_type: issue.issue_type.clone(),
                        description: issue.description.clone(),
                        severity: issue.severity.clone(),
                        transaction_id: issue.transaction_id.map(|id| id.to_string()).unwrap_or_default(),
                        account_id: issue.account_id.map(|id| id.to_string()).unwrap_or_default(),
                    }
                }).collect(),
                last_reconciled_at: result.last_reconciled_at.map(|dt| dt.to_rfc3339()).unwrap_or_default(),
            }
        }).collect();

        // Calculate summary
        let total_accounts = reconciliation_results.len() as i32;
        let reconciled_accounts = reconciliation_results.iter().filter(|r| r.is_reconciled).count() as i32;
        let total_difference: Decimal = reconciliation_results.iter().map(|r| r.difference.abs()).sum();

        // Record audit entry
        self.record_audit_entry(
            None,
            None,
            "accounts_reconciled",
            None,
            Some(format!("Reconciled {} accounts as of {}", total_accounts, as_of_date.to_rfc3339())),
            &auth_context.user_id,
            request.remote_addr().map(|addr| addr.to_string()),
        ).await;

        // Log the operation
        self.audit_logger.log_operation(
            &auth_context.user_id.to_string(),
            "reconcile_accounts",
            &format!("Reconciled {} accounts: {}/{} balanced", 
                total_accounts, reconciled_accounts, total_accounts),
            true,
            request.remote_addr(),
        ).await;

        Ok(Response::new(ReconcileAccountsResponse {
            reconciliations: proto_results,
            total_accounts,
            reconciled_accounts,
            total_difference: total_difference.to_string(),
            as_of_date: as_of_date.to_rfc3339(),
        }))
    }

    /// Generate financial report
    async fn generate_financial_report(
        &self,
        request: Request<GenerateFinancialReportRequest>,
    ) -> Result<Response<GenerateFinancialReportResponse>, Status> {
        let req = request.get_ref();
        
        // Extract authentication
        let auth_context = self.auth_service.extract_auth(&request).await?;
        self.auth_service.check_permission(&auth_context, Permission::PermissionLedgerRead)?;

        // Parse report type
        let report_type = Self::proto_to_report_type(req.report_type)?;

        // Parse date range
        let start_date = DateTime::parse_from_rfc3339(&req.start_date)
            .map_err(|_| Status::invalid_argument("Invalid start date"))?
            .with_timezone(&Utc);
        let end_date = DateTime::parse_from_rfc3339(&req.end_date)
            .map_err(|_| Status::invalid_argument("Invalid end date"))?
            .with_timezone(&Utc);

        // Generate report
        let report = self.ledger_repository
            .generate_financial_report(&report_type, &start_date, &end_date, req.currency.clone())
            .await
            .map_err(|e| Status::internal(format!("Failed to generate financial report: {}", e)))?;

        // Convert to proto
        let proto_report = crate::proto::fo3::wallet::v1::FinancialReport {
            report_type: Self::report_type_to_proto(&report.report_type),
            title: report.title,
            currency: report.currency,
            start_date: report.start_date.to_rfc3339(),
            end_date: report.end_date.to_rfc3339(),
            sections: report.sections.iter().map(|section| {
                crate::proto::fo3::wallet::v1::BalanceSheetSection {
                    section_type: Self::account_type_to_proto(&section.section_type),
                    section_name: section.section_name.clone(),
                    items: section.items.iter().map(|item| {
                        crate::proto::fo3::wallet::v1::BalanceSheetItem {
                            account_id: item.account_id.to_string(),
                            account_code: item.account_code.clone(),
                            account_name: item.account_name.clone(),
                            balance: item.balance.to_string(),
                            currency: item.currency.clone(),
                        }
                    }).collect(),
                    total_balance: section.total_balance.to_string(),
                }
            }).collect(),
            summary: report.summary,
            generated_at: report.generated_at.to_rfc3339(),
        };

        // Log the operation
        self.audit_logger.log_operation(
            &auth_context.user_id.to_string(),
            "generate_financial_report",
            &format!("Generated {} report for period {} to {}", 
                report_type, start_date.format("%Y-%m-%d"), end_date.format("%Y-%m-%d")),
            true,
            request.remote_addr(),
        ).await;

        Ok(Response::new(GenerateFinancialReportResponse {
            report: Some(proto_report),
        }))
    }

    /// Validate bookkeeping
    async fn validate_bookkeeping(
        &self,
        request: Request<ValidateBookkeepingRequest>,
    ) -> Result<Response<ValidateBookkeepingResponse>, Status> {
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

        // Perform validation
        let validation_results = self.ledger_repository
            .validate_bookkeeping(&as_of_date, req.check_double_entry, req.check_balance_integrity)
            .await
            .map_err(|e| Status::internal(format!("Failed to validate bookkeeping: {}", e)))?;

        // Convert issues to proto
        let proto_issues = validation_results.iter().map(|issue| {
            crate::proto::fo3::wallet::v1::ValidationIssue {
                issue_type: issue.issue_type.clone(),
                description: issue.description.clone(),
                severity: issue.severity.clone(),
                transaction_id: issue.transaction_id.map(|id| id.to_string()).unwrap_or_default(),
                account_id: issue.account_id.map(|id| id.to_string()).unwrap_or_default(),
            }
        }).collect();

        let is_valid = validation_results.is_empty();
        let critical_issues = validation_results.iter().filter(|i| i.severity == "critical").count() as i32;
        let warning_issues = validation_results.iter().filter(|i| i.severity == "warning").count() as i32;

        // Log the operation
        self.audit_logger.log_operation(
            &auth_context.user_id.to_string(),
            "validate_bookkeeping",
            &format!("Bookkeeping validation: {} issues found ({} critical, {} warnings)", 
                validation_results.len(), critical_issues, warning_issues),
            true,
            request.remote_addr(),
        ).await;

        Ok(Response::new(ValidateBookkeepingResponse {
            is_valid,
            issues: proto_issues,
            total_issues: validation_results.len() as i32,
            critical_issues,
            warning_issues,
            validated_at: as_of_date.to_rfc3339(),
        }))
    }
}
