//! Admin and external payment method implementations for CardFundingService

use super::card_funding::CardFundingServiceImpl;
use std::collections::HashMap;
use tonic::{Request, Response, Status};
use uuid::Uuid;
use rust_decimal::Decimal;
use chrono::Utc;

use crate::proto::fo3::wallet::v1::{
    card_funding_service_server::CardFundingService,
    *,
};
use crate::models::card_funding::{
    FundingTransaction, FundingTransactionStatus, FundingSourceType,
    FundingMetrics, FundingSourceMetrics, CurrencyMetrics,
};
use crate::models::notifications::NotificationType;

impl CardFundingService for CardFundingServiceImpl {
    /// Initiate external card funding
    async fn initiate_card_funding(
        &self,
        request: Request<InitiateCardFundingRequest>,
    ) -> Result<Response<InitiateCardFundingResponse>, Status> {
        let req = request.get_ref();
        
        // Parse request parameters
        let card_id = Uuid::parse_str(&req.card_id)
            .map_err(|_| Status::invalid_argument("Invalid card ID"))?;
        let external_card_id = Uuid::parse_str(&req.external_card_id)
            .map_err(|_| Status::invalid_argument("Invalid external card ID"))?;
        let amount = Decimal::from_str(&req.amount)
            .map_err(|_| Status::invalid_argument("Invalid amount"))?;

        // Validate external card funding request
        let auth_context = self.funding_guard
            .validate_funding_transaction(&request, &card_id, &external_card_id, &amount, "USD")
            .await?;

        // Calculate fees for external card funding (higher due to interchange)
        let fee_calculation = self.calculate_funding_fees(&FundingSourceType::ExternalCard, &amount, "USD");

        // Create external card funding transaction
        let funding_transaction = FundingTransaction {
            id: Uuid::new_v4(),
            user_id: auth_context.user_id,
            card_id,
            funding_source_id: external_card_id,
            status: FundingTransactionStatus::Pending, // Requires 3DS authorization
            amount,
            currency: "USD".to_string(),
            fee_amount: fee_calculation.fee_amount,
            fee_percentage: fee_calculation.fee_percentage,
            exchange_rate: None,
            net_amount: fee_calculation.net_amount,
            reference_number: Self::generate_reference_number(),
            external_transaction_id: None,
            description: Some(req.description.clone()),
            failure_reason: None,
            metadata: HashMap::from([
                ("funding_type".to_string(), "external_card".to_string()),
                ("external_card_id".to_string(), external_card_id.to_string()),
                ("requires_3ds".to_string(), "true".to_string()),
            ]),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            completed_at: None,
            expires_at: Some(Utc::now() + chrono::Duration::minutes(15)), // 15-minute expiration for 3DS
        };

        // Save transaction
        let created_transaction = self.funding_repository
            .create_funding_transaction(&funding_transaction)
            .await
            .map_err(|e| Status::internal(format!("Failed to create card funding: {}", e)))?;

        // Generate 3DS authorization URL (mock implementation)
        let authorization_url = format!("https://3ds.fo3wallet.com/authorize?transaction_id={}&amount={}&currency=USD", 
            created_transaction.id, amount);

        // Log the operation
        self.audit_logger.log_operation(
            &auth_context.user_id.to_string(),
            "initiate_card_funding",
            &format!("Initiated external card funding: {} USD from card {} to card {}", 
                amount, external_card_id, card_id),
            true,
            request.remote_addr(),
        ).await;

        // Send notification
        self.send_funding_notification(
            &auth_context.user_id,
            NotificationType::FundingInitiated,
            "Card Funding Initiated",
            &format!("Your card funding of ${} requires authorization. Please complete the 3D Secure verification.", 
                amount),
            HashMap::from([
                ("transaction_id".to_string(), created_transaction.id.to_string()),
                ("authorization_url".to_string(), authorization_url.clone()),
                ("expires_at".to_string(), created_transaction.expires_at.unwrap().to_rfc3339()),
            ]),
        ).await;

        Ok(Response::new(InitiateCardFundingResponse {
            transaction: Some(self.funding_transaction_to_proto(&created_transaction)),
            fee_calculation: Some(self.fee_calculation_to_proto(&fee_calculation)),
            authorization_url,
        }))
    }

    /// Get funding status
    async fn get_funding_status(
        &self,
        request: Request<GetFundingStatusRequest>,
    ) -> Result<Response<GetFundingStatusResponse>, Status> {
        let req = request.get_ref();
        
        // Extract authentication
        let auth_context = self.auth_service.extract_auth(&request).await?;
        self.auth_service.check_permission(&auth_context, Permission::PermissionCardFunding)?;

        // Parse transaction ID
        let transaction_id = Uuid::parse_str(&req.transaction_id)
            .map_err(|_| Status::invalid_argument("Invalid transaction ID"))?;

        // Get funding transaction
        let transaction = self.funding_repository
            .get_funding_transaction_by_user(&auth_context.user_id, &transaction_id)
            .await
            .map_err(|e| Status::internal(format!("Failed to get funding transaction: {}", e)))?
            .ok_or_else(|| Status::not_found("Funding transaction not found"))?;

        // Generate status details based on transaction status and type
        let status_details = match transaction.status {
            FundingTransactionStatus::Pending => {
                if transaction.metadata.get("requires_3ds") == Some(&"true".to_string()) {
                    "Waiting for 3D Secure authorization"
                } else {
                    "Transaction is pending processing"
                }
            }
            FundingTransactionStatus::Processing => {
                match transaction.metadata.get("funding_type").map(|s| s.as_str()) {
                    Some("ach") => "ACH transfer is being processed by the bank",
                    Some("crypto") => "Waiting for blockchain confirmations",
                    Some("external_card") => "Card payment is being processed",
                    _ => "Transaction is being processed",
                }
            }
            FundingTransactionStatus::Completed => "Transaction completed successfully",
            FundingTransactionStatus::Failed => {
                transaction.failure_reason.as_deref().unwrap_or("Transaction failed")
            }
            FundingTransactionStatus::Cancelled => "Transaction was cancelled",
            FundingTransactionStatus::Refunded => "Transaction was refunded",
        }.to_string();

        Ok(Response::new(GetFundingStatusResponse {
            transaction: Some(self.funding_transaction_to_proto(&transaction)),
            status_details,
        }))
    }

    /// Get user funding sources (admin operation)
    async fn get_user_funding_sources(
        &self,
        request: Request<GetUserFundingSourcesRequest>,
    ) -> Result<Response<GetUserFundingSourcesResponse>, Status> {
        let req = request.get_ref();
        
        // Validate admin operation
        let target_user_id = Uuid::parse_str(&req.user_id)
            .map_err(|_| Status::invalid_argument("Invalid user ID"))?;
        
        let _auth_context = self.funding_guard
            .validate_admin_operation(&request, "get_user_funding_sources", Some(&target_user_id))
            .await?;

        // Parse filters
        let source_type = if req.r#type != 0 {
            Some(Self::proto_to_funding_source_type(req.r#type)?)
        } else {
            None
        };

        let status = if req.status != 0 {
            Some(Self::proto_to_funding_source_status(req.status)?)
        } else {
            None
        };

        let page = if req.page > 0 { req.page } else { 1 };
        let page_size = if req.page_size > 0 && req.page_size <= 100 { req.page_size } else { 20 };

        // Get user's funding sources
        let (sources, total_count) = self.funding_repository
            .list_funding_sources(&target_user_id, source_type, status, page, page_size)
            .await
            .map_err(|e| Status::internal(format!("Failed to get user funding sources: {}", e)))?;

        let proto_sources = sources.iter()
            .map(|source| self.funding_source_to_proto(source))
            .collect();

        Ok(Response::new(GetUserFundingSourcesResponse {
            funding_sources: proto_sources,
            total_count: total_count as i32,
            page,
            page_size,
        }))
    }

    /// Get funding metrics (admin operation)
    async fn get_funding_metrics(
        &self,
        request: Request<GetFundingMetricsRequest>,
    ) -> Result<Response<GetFundingMetricsResponse>, Status> {
        let req = request.get_ref();
        
        // Validate admin operation
        let _auth_context = self.funding_guard
            .validate_admin_operation(&request, "get_funding_metrics", None)
            .await?;

        // Parse date range
        let start_date = chrono::DateTime::parse_from_rfc3339(&req.start_date)
            .map_err(|_| Status::invalid_argument("Invalid start date"))?
            .with_timezone(&Utc);
        let end_date = chrono::DateTime::parse_from_rfc3339(&req.end_date)
            .map_err(|_| Status::invalid_argument("Invalid end date"))?
            .with_timezone(&Utc);

        // Parse optional filters
        let source_type = if req.source_type != 0 {
            Some(Self::proto_to_funding_source_type(req.source_type)?)
        } else {
            None
        };

        let currency = if req.currency.is_empty() {
            None
        } else {
            Some(req.currency.clone())
        };

        // Get funding metrics
        let metrics = self.funding_repository
            .get_funding_metrics(&start_date, &end_date, source_type, currency)
            .await
            .map_err(|e| Status::internal(format!("Failed to get funding metrics: {}", e)))?;

        // Convert to proto
        let proto_metrics = crate::proto::fo3::wallet::v1::FundingMetrics {
            total_volume: metrics.total_volume.to_string(),
            total_fees: metrics.total_fees.to_string(),
            total_transactions: metrics.total_transactions as i32,
            average_transaction_size: metrics.average_transaction_size.to_string(),
            by_source: metrics.by_source.iter().map(|sm| crate::proto::fo3::wallet::v1::FundingSourceMetrics {
                r#type: Self::funding_source_type_to_proto(&sm.source_type),
                volume: sm.volume.to_string(),
                fees: sm.fees.to_string(),
                transaction_count: sm.transaction_count as i32,
                success_rate: sm.success_rate.to_string(),
            }).collect(),
            by_currency: metrics.by_currency.iter().map(|cm| crate::proto::fo3::wallet::v1::CurrencyMetrics {
                currency: cm.currency.clone(),
                volume: cm.volume.to_string(),
                fees: cm.fees.to_string(),
                transaction_count: cm.transaction_count as i32,
            }).collect(),
            success_rate: metrics.success_rate.to_string(),
        };

        Ok(Response::new(GetFundingMetricsResponse {
            metrics: Some(proto_metrics),
        }))
    }

    /// Update user funding limits (admin operation)
    async fn update_funding_limits(
        &self,
        request: Request<UpdateFundingLimitsRequest>,
    ) -> Result<Response<UpdateFundingLimitsResponse>, Status> {
        let req = request.get_ref();
        
        // Validate admin operation
        let target_user_id = Uuid::parse_str(&req.user_id)
            .map_err(|_| Status::invalid_argument("Invalid user ID"))?;
        
        let auth_context = self.funding_guard
            .validate_admin_operation(&request, "update_funding_limits", Some(&target_user_id))
            .await?;

        // Parse new limits
        let limits_proto = req.limits.as_ref()
            .ok_or_else(|| Status::invalid_argument("Funding limits required"))?;

        // Get existing limits or create new ones
        let mut limits = self.funding_repository
            .get_funding_limits(&target_user_id)
            .await
            .map_err(|e| Status::internal(format!("Failed to get funding limits: {}", e)))?
            .unwrap_or_else(|| {
                use crate::models::card_funding::FundingLimits;
                let mut default_limits = FundingLimits::default();
                default_limits.user_id = target_user_id;
                default_limits
            });

        // Update limits
        limits.daily_limit = Decimal::from_str(&limits_proto.daily_limit)
            .map_err(|_| Status::invalid_argument("Invalid daily limit"))?;
        limits.monthly_limit = Decimal::from_str(&limits_proto.monthly_limit)
            .map_err(|_| Status::invalid_argument("Invalid monthly limit"))?;
        limits.yearly_limit = Decimal::from_str(&limits_proto.yearly_limit)
            .map_err(|_| Status::invalid_argument("Invalid yearly limit"))?;
        limits.per_transaction_limit = Decimal::from_str(&limits_proto.per_transaction_limit)
            .map_err(|_| Status::invalid_argument("Invalid per transaction limit"))?;
        limits.daily_transaction_count = limits_proto.daily_transaction_count;
        limits.monthly_transaction_count = limits_proto.monthly_transaction_count;
        limits.updated_at = Utc::now();

        // Save updated limits
        let updated_limits = if limits.id == Uuid::new_v4() { // Check if it's a new default
            self.funding_repository
                .create_funding_limits(&limits)
                .await
                .map_err(|e| Status::internal(format!("Failed to create funding limits: {}", e)))?
        } else {
            self.funding_repository
                .update_funding_limits(&limits)
                .await
                .map_err(|e| Status::internal(format!("Failed to update funding limits: {}", e)))?
        };

        // Log the operation
        self.audit_logger.log_operation(
            &auth_context.user_id.to_string(),
            "update_funding_limits",
            &format!("Updated funding limits for user {}: daily={}, monthly={}, yearly={}, reason: {}", 
                target_user_id, updated_limits.daily_limit, updated_limits.monthly_limit, 
                updated_limits.yearly_limit, req.reason),
            true,
            request.remote_addr(),
        ).await;

        // Convert to proto
        let proto_limits = crate::proto::fo3::wallet::v1::FundingLimits {
            daily_limit: updated_limits.daily_limit.to_string(),
            monthly_limit: updated_limits.monthly_limit.to_string(),
            yearly_limit: updated_limits.yearly_limit.to_string(),
            per_transaction_limit: updated_limits.per_transaction_limit.to_string(),
            daily_used: updated_limits.daily_used.to_string(),
            monthly_used: updated_limits.monthly_used.to_string(),
            yearly_used: updated_limits.yearly_used.to_string(),
            daily_transaction_count: updated_limits.daily_transaction_count,
            monthly_transaction_count: updated_limits.monthly_transaction_count,
            daily_transactions_used: updated_limits.daily_transactions_used,
            monthly_transactions_used: updated_limits.monthly_transactions_used,
        };

        Ok(Response::new(UpdateFundingLimitsResponse {
            limits: Some(proto_limits),
            success: true,
            message: "Funding limits updated successfully".to_string(),
        }))
    }
}
