//! Additional CardFundingService method implementations

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
};
use crate::models::notifications::NotificationType;

impl CardFundingService for CardFundingServiceImpl {
    /// Get a funding source by ID
    async fn get_funding_source(
        &self,
        request: Request<GetFundingSourceRequest>,
    ) -> Result<Response<GetFundingSourceResponse>, Status> {
        let req = request.get_ref();
        
        // Extract authentication
        let auth_context = self.auth_service.extract_auth(&request).await?;
        self.auth_service.check_permission(&auth_context, Permission::PermissionCardFunding)?;

        // Parse funding source ID
        let source_id = Uuid::parse_str(&req.funding_source_id)
            .map_err(|_| Status::invalid_argument("Invalid funding source ID"))?;

        // Get funding source
        let source = self.funding_repository
            .get_funding_source_by_user(&auth_context.user_id, &source_id)
            .await
            .map_err(|e| Status::internal(format!("Failed to get funding source: {}", e)))?;

        match source {
            Some(source) => {
                Ok(Response::new(GetFundingSourceResponse {
                    funding_source: Some(self.funding_source_to_proto(&source)),
                }))
            }
            None => Err(Status::not_found("Funding source not found")),
        }
    }

    /// List user's funding sources
    async fn list_funding_sources(
        &self,
        request: Request<ListFundingSourcesRequest>,
    ) -> Result<Response<ListFundingSourcesResponse>, Status> {
        let req = request.get_ref();
        
        // Extract authentication
        let auth_context = self.auth_service.extract_auth(&request).await?;
        self.auth_service.check_permission(&auth_context, Permission::PermissionCardFunding)?;

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

        // Get funding sources
        let (sources, total_count) = self.funding_repository
            .list_funding_sources(&auth_context.user_id, source_type, status, page, page_size)
            .await
            .map_err(|e| Status::internal(format!("Failed to list funding sources: {}", e)))?;

        let proto_sources = sources.iter()
            .map(|source| self.funding_source_to_proto(source))
            .collect();

        Ok(Response::new(ListFundingSourcesResponse {
            funding_sources: proto_sources,
            total_count: total_count as i32,
            page,
            page_size,
        }))
    }

    /// Update a funding source
    async fn update_funding_source(
        &self,
        request: Request<UpdateFundingSourceRequest>,
    ) -> Result<Response<UpdateFundingSourceResponse>, Status> {
        let req = request.get_ref();
        
        // Extract authentication
        let auth_context = self.auth_service.extract_auth(&request).await?;
        self.auth_service.check_permission(&auth_context, Permission::PermissionCardFunding)?;

        // Parse funding source ID
        let source_id = Uuid::parse_str(&req.funding_source_id)
            .map_err(|_| Status::invalid_argument("Invalid funding source ID"))?;

        // Get existing funding source
        let mut source = self.funding_repository
            .get_funding_source_by_user(&auth_context.user_id, &source_id)
            .await
            .map_err(|e| Status::internal(format!("Failed to get funding source: {}", e)))?
            .ok_or_else(|| Status::not_found("Funding source not found"))?;

        // Update fields
        if !req.name.is_empty() {
            source.name = req.name.clone();
        }

        if let Some(limits) = &req.limits {
            source.limits.daily_limit = Decimal::from_str(&limits.daily_limit)
                .map_err(|_| Status::invalid_argument("Invalid daily limit"))?;
            source.limits.monthly_limit = Decimal::from_str(&limits.monthly_limit)
                .map_err(|_| Status::invalid_argument("Invalid monthly limit"))?;
            source.limits.per_transaction_limit = Decimal::from_str(&limits.per_transaction_limit)
                .map_err(|_| Status::invalid_argument("Invalid per transaction limit"))?;
            source.limits.minimum_amount = Decimal::from_str(&limits.minimum_amount)
                .map_err(|_| Status::invalid_argument("Invalid minimum amount"))?;
            source.limits.daily_transaction_count = limits.daily_transaction_count;
            source.limits.monthly_transaction_count = limits.monthly_transaction_count;
        }

        source.is_primary = req.is_primary;
        source.updated_at = Utc::now();

        // Save updated source
        let updated_source = self.funding_repository
            .update_funding_source(&source)
            .await
            .map_err(|e| Status::internal(format!("Failed to update funding source: {}", e)))?;

        // Log the operation
        self.audit_logger.log_operation(
            &auth_context.user_id.to_string(),
            "update_funding_source",
            &format!("Updated funding source: {}", updated_source.name),
            true,
            request.remote_addr(),
        ).await;

        Ok(Response::new(UpdateFundingSourceResponse {
            funding_source: Some(self.funding_source_to_proto(&updated_source)),
        }))
    }

    /// Remove a funding source
    async fn remove_funding_source(
        &self,
        request: Request<RemoveFundingSourceRequest>,
    ) -> Result<Response<RemoveFundingSourceResponse>, Status> {
        let req = request.get_ref();
        
        // Extract authentication
        let auth_context = self.auth_service.extract_auth(&request).await?;
        self.auth_service.check_permission(&auth_context, Permission::PermissionCardFunding)?;

        // Parse funding source ID
        let source_id = Uuid::parse_str(&req.funding_source_id)
            .map_err(|_| Status::invalid_argument("Invalid funding source ID"))?;

        // Verify ownership
        let source = self.funding_repository
            .get_funding_source_by_user(&auth_context.user_id, &source_id)
            .await
            .map_err(|e| Status::internal(format!("Failed to get funding source: {}", e)))?
            .ok_or_else(|| Status::not_found("Funding source not found"))?;

        // Remove funding source
        let success = self.funding_repository
            .delete_funding_source(&source_id)
            .await
            .map_err(|e| Status::internal(format!("Failed to remove funding source: {}", e)))?;

        if success {
            // Log the operation
            self.audit_logger.log_operation(
                &auth_context.user_id.to_string(),
                "remove_funding_source",
                &format!("Removed funding source: {} (reason: {})", source.name, req.reason),
                true,
                request.remote_addr(),
            ).await;

            // Send notification
            self.send_funding_notification(
                &auth_context.user_id,
                NotificationType::FundingSourceRemoved,
                "Funding Source Removed",
                &format!("Your {} funding source '{}' has been removed.", 
                    source.source_type, source.name),
                HashMap::from([
                    ("funding_source_id".to_string(), source.id.to_string()),
                    ("reason".to_string(), req.reason.clone()),
                ]),
            ).await;

            Ok(Response::new(RemoveFundingSourceResponse {
                success: true,
                message: "Funding source removed successfully".to_string(),
            }))
        } else {
            Ok(Response::new(RemoveFundingSourceResponse {
                success: false,
                message: "Failed to remove funding source".to_string(),
            }))
        }
    }

    /// Fund a card from a funding source
    async fn fund_card(
        &self,
        request: Request<FundCardRequest>,
    ) -> Result<Response<FundCardResponse>, Status> {
        let req = request.get_ref();

        // Parse request parameters
        let card_id = Uuid::parse_str(&req.card_id)
            .map_err(|_| Status::invalid_argument("Invalid card ID"))?;
        let funding_source_id = Uuid::parse_str(&req.funding_source_id)
            .map_err(|_| Status::invalid_argument("Invalid funding source ID"))?;
        let amount = Decimal::from_str(&req.amount)
            .map_err(|_| Status::invalid_argument("Invalid amount"))?;

        // Validate request with security guard
        let auth_context = self.funding_guard
            .validate_funding_transaction(&request, &card_id, &funding_source_id, &amount, &req.currency)
            .await?;

        // Get funding source to determine fee structure
        let funding_source = self.funding_repository
            .get_funding_source_by_user(&auth_context.user_id, &funding_source_id)
            .await
            .map_err(|e| Status::internal(format!("Failed to get funding source: {}", e)))?
            .ok_or_else(|| Status::not_found("Funding source not found"))?;

        // Calculate fees
        let fee_calculation = self.calculate_funding_fees(&funding_source.source_type, &amount, &req.currency);

        // Check if user accepts fees
        if !req.accept_fees {
            return Err(Status::failed_precondition("Fee acceptance required"));
        }

        // Create funding transaction
        let funding_transaction = FundingTransaction {
            id: Uuid::new_v4(),
            user_id: auth_context.user_id,
            card_id,
            funding_source_id,
            status: FundingTransactionStatus::Pending,
            amount,
            currency: req.currency.clone(),
            fee_amount: fee_calculation.fee_amount,
            fee_percentage: fee_calculation.fee_percentage,
            exchange_rate: fee_calculation.exchange_rate,
            net_amount: fee_calculation.net_amount,
            reference_number: Self::generate_reference_number(),
            external_transaction_id: None,
            description: Some(req.description.clone()),
            failure_reason: None,
            metadata: HashMap::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            completed_at: None,
            expires_at: None,
        };

        // Save transaction
        let created_transaction = self.funding_repository
            .create_funding_transaction(&funding_transaction)
            .await
            .map_err(|e| Status::internal(format!("Failed to create funding transaction: {}", e)))?;

        // Log the operation
        self.audit_logger.log_operation(
            &auth_context.user_id.to_string(),
            "fund_card",
            &format!("Initiated card funding: {} {} from {} to card {}",
                amount, req.currency, funding_source.name, card_id),
            true,
            request.remote_addr(),
        ).await;

        // Send notification
        self.send_funding_notification(
            &auth_context.user_id,
            NotificationType::FundingInitiated,
            "Card Funding Initiated",
            &format!("Your card funding of {} {} has been initiated and is being processed.",
                amount, req.currency),
            HashMap::from([
                ("transaction_id".to_string(), created_transaction.id.to_string()),
                ("amount".to_string(), amount.to_string()),
                ("currency".to_string(), req.currency.clone()),
            ]),
        ).await;

        // In a real implementation, this would trigger the actual funding process
        // For now, we'll simulate immediate completion for non-crypto sources
        let new_card_balance = match funding_source.source_type {
            FundingSourceType::FiatAccount => {
                // Immediate completion for fiat accounts
                let mut completed_tx = created_transaction.clone();
                completed_tx.status = FundingTransactionStatus::Completed;
                completed_tx.completed_at = Some(Utc::now());

                let _ = self.funding_repository.update_funding_transaction(&completed_tx).await;

                // Would update card balance in CardService
                "1000.00".to_string() // Placeholder
            }
            _ => {
                // Other sources require processing time
                "0.00".to_string() // Balance not updated yet
            }
        };

        Ok(Response::new(FundCardResponse {
            transaction: Some(self.funding_transaction_to_proto(&created_transaction)),
            fee_calculation: Some(self.fee_calculation_to_proto(&fee_calculation)),
            new_card_balance,
        }))
    }

    /// Get funding transaction history
    async fn get_funding_history(
        &self,
        request: Request<GetFundingHistoryRequest>,
    ) -> Result<Response<GetFundingHistoryResponse>, Status> {
        let req = request.get_ref();

        // Extract authentication
        let auth_context = self.auth_service.extract_auth(&request).await?;
        self.auth_service.check_permission(&auth_context, Permission::PermissionCardFunding)?;

        // Parse optional filters
        let card_id = if req.card_id.is_empty() {
            None
        } else {
            Some(Uuid::parse_str(&req.card_id)
                .map_err(|_| Status::invalid_argument("Invalid card ID"))?)
        };

        let funding_source_id = if req.funding_source_id.is_empty() {
            None
        } else {
            Some(Uuid::parse_str(&req.funding_source_id)
                .map_err(|_| Status::invalid_argument("Invalid funding source ID"))?)
        };

        let status = if req.status != 0 {
            Some(Self::proto_to_funding_transaction_status(req.status)?)
        } else {
            None
        };

        let page = if req.page > 0 { req.page } else { 1 };
        let page_size = if req.page_size > 0 && req.page_size <= 100 { req.page_size } else { 20 };

        // Get funding transactions
        let (transactions, total_count) = self.funding_repository
            .list_funding_transactions(&auth_context.user_id, card_id, funding_source_id, status, page, page_size)
            .await
            .map_err(|e| Status::internal(format!("Failed to get funding history: {}", e)))?;

        let proto_transactions = transactions.iter()
            .map(|tx| self.funding_transaction_to_proto(tx))
            .collect();

        Ok(Response::new(GetFundingHistoryResponse {
            transactions: proto_transactions,
            total_count: total_count as i32,
            page,
            page_size,
        }))
    }

    /// Estimate funding fees
    async fn estimate_funding_fee(
        &self,
        request: Request<EstimateFundingFeeRequest>,
    ) -> Result<Response<EstimateFundingFeeResponse>, Status> {
        let req = request.get_ref();

        // Extract authentication
        let auth_context = self.auth_service.extract_auth(&request).await?;
        self.auth_service.check_permission(&auth_context, Permission::PermissionCardFunding)?;

        // Parse request parameters
        let funding_source_id = Uuid::parse_str(&req.funding_source_id)
            .map_err(|_| Status::invalid_argument("Invalid funding source ID"))?;
        let amount = Decimal::from_str(&req.amount)
            .map_err(|_| Status::invalid_argument("Invalid amount"))?;

        // Get funding source
        let funding_source = self.funding_repository
            .get_funding_source_by_user(&auth_context.user_id, &funding_source_id)
            .await
            .map_err(|e| Status::internal(format!("Failed to get funding source: {}", e)))?
            .ok_or_else(|| Status::not_found("Funding source not found"))?;

        // Calculate fees
        let fee_calculation = self.calculate_funding_fees(&funding_source.source_type, &amount, &req.currency);

        // Estimate completion time based on source type
        let estimated_completion_time = match funding_source.source_type {
            FundingSourceType::FiatAccount => "Instant",
            FundingSourceType::ExternalCard => "1-5 minutes",
            FundingSourceType::ACH => "1-3 business days",
            FundingSourceType::BankAccount => "1-3 business days",
            FundingSourceType::CryptoWallet => "10-60 minutes",
        }.to_string();

        Ok(Response::new(EstimateFundingFeeResponse {
            fee_calculation: Some(self.fee_calculation_to_proto(&fee_calculation)),
            estimated_completion_time,
        }))
    }

    /// Get user funding limits
    async fn get_funding_limits(
        &self,
        request: Request<GetFundingLimitsRequest>,
    ) -> Result<Response<GetFundingLimitsResponse>, Status> {
        let req = request.get_ref();

        // Extract authentication
        let auth_context = self.auth_service.extract_auth(&request).await?;

        // Determine target user (admin can query other users)
        let target_user_id = if req.user_id.is_empty() {
            // User querying their own limits
            self.auth_service.check_permission(&auth_context, Permission::PermissionCardFunding)?;
            auth_context.user_id
        } else {
            // Admin querying another user's limits
            self.funding_guard.validate_admin_operation(&request, "get_funding_limits", None).await?;
            Uuid::parse_str(&req.user_id)
                .map_err(|_| Status::invalid_argument("Invalid user ID"))?
        };

        // Get funding limits
        let limits = self.funding_repository
            .get_funding_limits(&target_user_id)
            .await
            .map_err(|e| Status::internal(format!("Failed to get funding limits: {}", e)))?;

        let limits = limits.unwrap_or_else(|| {
            use crate::models::card_funding::FundingLimits;
            let mut default_limits = FundingLimits::default();
            default_limits.user_id = target_user_id;
            default_limits
        });

        // Convert to proto
        let proto_limits = crate::proto::fo3::wallet::v1::FundingLimits {
            daily_limit: limits.daily_limit.to_string(),
            monthly_limit: limits.monthly_limit.to_string(),
            yearly_limit: limits.yearly_limit.to_string(),
            per_transaction_limit: limits.per_transaction_limit.to_string(),
            daily_used: limits.daily_used.to_string(),
            monthly_used: limits.monthly_used.to_string(),
            yearly_used: limits.yearly_used.to_string(),
            daily_transaction_count: limits.daily_transaction_count,
            monthly_transaction_count: limits.monthly_transaction_count,
            daily_transactions_used: limits.daily_transactions_used,
            monthly_transactions_used: limits.monthly_transactions_used,
        };

        Ok(Response::new(GetFundingLimitsResponse {
            limits: Some(proto_limits),
        }))
    }

    /// Initiate cryptocurrency funding
    async fn initiate_crypto_funding(
        &self,
        request: Request<InitiateCryptoFundingRequest>,
    ) -> Result<Response<InitiateCryptoFundingResponse>, Status> {
        let req = request.get_ref();

        // Parse request parameters
        let card_id = Uuid::parse_str(&req.card_id)
            .map_err(|_| Status::invalid_argument("Invalid card ID"))?;
        let amount = Decimal::from_str(&req.amount)
            .map_err(|_| Status::invalid_argument("Invalid amount"))?;
        let currency = Self::proto_to_crypto_currency(req.currency as i32)?;

        // Validate crypto funding request
        let auth_context = self.funding_guard
            .validate_crypto_funding(&request, &amount, &currency.to_string(), &req.network)
            .await?;

        // Generate unique funding ID
        let funding_id = Uuid::new_v4();

        // Generate deposit address (in real implementation, this would call blockchain service)
        let deposit_address = format!("{}_{}",
            match currency {
                crate::models::card_funding::CryptoCurrency::USDT => "0x1234567890abcdef",
                crate::models::card_funding::CryptoCurrency::USDC => "0xabcdef1234567890",
                crate::models::card_funding::CryptoCurrency::DAI => "0x567890abcdef1234",
                crate::models::card_funding::CryptoCurrency::BUSD => "0xdef1234567890abc",
            },
            funding_id.to_string()[..8].to_uppercase()
        );

        // Calculate fees for crypto funding
        let fee_calculation = self.calculate_funding_fees(
            &FundingSourceType::CryptoWallet,
            &amount,
            &currency.to_string()
        );

        // Create crypto funding details
        let expires_at = Utc::now() + chrono::Duration::hours(2); // 2-hour expiration
        let crypto_details = crate::models::card_funding::CryptoFundingDetails {
            currency: currency.clone(),
            network: req.network.clone(),
            deposit_address: deposit_address.clone(),
            required_confirmations: match req.network.as_str() {
                "ethereum" => 12,
                "bsc" => 15,
                "polygon" => 20,
                "tron" => 19,
                _ => 6,
            },
            current_confirmations: 0,
            transaction_hash: None,
            exchange_rate: Decimal::ONE, // Would fetch from pricing service
            expires_at,
        };

        // Create pending funding transaction
        let funding_transaction = FundingTransaction {
            id: funding_id,
            user_id: auth_context.user_id,
            card_id,
            funding_source_id: Uuid::new_v4(), // Temporary crypto source
            status: FundingTransactionStatus::Pending,
            amount,
            currency: currency.to_string(),
            fee_amount: fee_calculation.fee_amount,
            fee_percentage: fee_calculation.fee_percentage,
            exchange_rate: Some(crypto_details.exchange_rate),
            net_amount: fee_calculation.net_amount,
            reference_number: Self::generate_reference_number(),
            external_transaction_id: None,
            description: Some(format!("Crypto funding: {} {} via {}", amount, currency, req.network)),
            failure_reason: None,
            metadata: HashMap::from([
                ("crypto_currency".to_string(), currency.to_string()),
                ("network".to_string(), req.network.clone()),
                ("deposit_address".to_string(), deposit_address.clone()),
                ("required_confirmations".to_string(), crypto_details.required_confirmations.to_string()),
            ]),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            completed_at: None,
            expires_at: Some(expires_at),
        };

        // Save transaction
        let created_transaction = self.funding_repository
            .create_funding_transaction(&funding_transaction)
            .await
            .map_err(|e| Status::internal(format!("Failed to create crypto funding: {}", e)))?;

        // Convert crypto details to proto
        let proto_crypto_details = crate::proto::fo3::wallet::v1::CryptoFundingDetails {
            currency: Self::crypto_currency_to_proto(&currency),
            network: req.network.clone(),
            deposit_address,
            required_confirmations: crypto_details.required_confirmations.to_string(),
            current_confirmations: crypto_details.current_confirmations.to_string(),
            transaction_hash: "".to_string(),
            exchange_rate: crypto_details.exchange_rate.to_string(),
            expires_at: expires_at.to_rfc3339(),
        };

        // Log the operation
        self.audit_logger.log_operation(
            &auth_context.user_id.to_string(),
            "initiate_crypto_funding",
            &format!("Initiated crypto funding: {} {} via {} to card {}",
                amount, currency, req.network, card_id),
            true,
            request.remote_addr(),
        ).await;

        // Send notification
        self.send_funding_notification(
            &auth_context.user_id,
            NotificationType::FundingInitiated,
            "Crypto Funding Initiated",
            &format!("Your crypto funding of {} {} has been initiated. Please send funds to the provided address.",
                amount, currency),
            HashMap::from([
                ("funding_id".to_string(), funding_id.to_string()),
                ("deposit_address".to_string(), proto_crypto_details.deposit_address.clone()),
                ("expires_at".to_string(), expires_at.to_rfc3339()),
            ]),
        ).await;

        Ok(Response::new(InitiateCryptoFundingResponse {
            funding_id: funding_id.to_string(),
            funding_details: Some(proto_crypto_details),
            fee_calculation: Some(self.fee_calculation_to_proto(&fee_calculation)),
            expires_at: expires_at.to_rfc3339(),
        }))
    }
}
