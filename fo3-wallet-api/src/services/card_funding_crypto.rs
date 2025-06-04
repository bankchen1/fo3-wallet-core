//! Crypto funding method implementations for CardFundingService

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
    FundingTransaction, FundingTransactionStatus, FundingSourceType, CryptoCurrency,
};
use crate::models::notifications::NotificationType;

impl CardFundingService for CardFundingServiceImpl {
    /// Confirm cryptocurrency funding with transaction hash
    async fn confirm_crypto_funding(
        &self,
        request: Request<ConfirmCryptoFundingRequest>,
    ) -> Result<Response<ConfirmCryptoFundingResponse>, Status> {
        let req = request.get_ref();
        
        // Extract authentication
        let auth_context = self.auth_service.extract_auth(&request).await?;
        self.auth_service.check_permission(&auth_context, Permission::PermissionCardFunding)?;

        // Parse funding ID
        let funding_id = Uuid::parse_str(&req.funding_id)
            .map_err(|_| Status::invalid_argument("Invalid funding ID"))?;

        // Get existing funding transaction
        let mut transaction = self.funding_repository
            .get_funding_transaction_by_user(&auth_context.user_id, &funding_id)
            .await
            .map_err(|e| Status::internal(format!("Failed to get funding transaction: {}", e)))?
            .ok_or_else(|| Status::not_found("Funding transaction not found"))?;

        // Verify transaction is in pending state
        if transaction.status != FundingTransactionStatus::Pending {
            return Err(Status::failed_precondition("Funding transaction is not pending"));
        }

        // Verify transaction hasn't expired
        if let Some(expires_at) = transaction.expires_at {
            if Utc::now() > expires_at {
                transaction.status = FundingTransactionStatus::Failed;
                transaction.failure_reason = Some("Transaction expired".to_string());
                let _ = self.funding_repository.update_funding_transaction(&transaction).await;
                return Err(Status::deadline_exceeded("Funding transaction has expired"));
            }
        }

        // Update transaction with hash and set to processing
        transaction.external_transaction_id = Some(req.transaction_hash.clone());
        transaction.status = FundingTransactionStatus::Processing;
        transaction.updated_at = Utc::now();
        transaction.metadata.insert("transaction_hash".to_string(), req.transaction_hash.clone());

        // In a real implementation, this would:
        // 1. Validate the transaction hash on the blockchain
        // 2. Check confirmations
        // 3. Verify the amount and destination address
        // For now, we'll simulate immediate confirmation for demo purposes

        // Simulate blockchain confirmation (in real implementation, this would be async)
        let confirmation_status = "confirmed"; // Would be "pending", "confirmed", or "failed"
        
        if confirmation_status == "confirmed" {
            transaction.status = FundingTransactionStatus::Completed;
            transaction.completed_at = Some(Utc::now());
            transaction.metadata.insert("confirmations".to_string(), "12".to_string());
        }

        // Save updated transaction
        let updated_transaction = self.funding_repository
            .update_funding_transaction(&transaction)
            .await
            .map_err(|e| Status::internal(format!("Failed to update funding transaction: {}", e)))?;

        // Log the operation
        self.audit_logger.log_operation(
            &auth_context.user_id.to_string(),
            "confirm_crypto_funding",
            &format!("Confirmed crypto funding: {} with hash {}", funding_id, req.transaction_hash),
            true,
            request.remote_addr(),
        ).await;

        // Send notification based on status
        let (notification_type, title, message) = if updated_transaction.status == FundingTransactionStatus::Completed {
            (
                NotificationType::FundingCompleted,
                "Crypto Funding Completed",
                &format!("Your crypto funding of {} {} has been completed successfully!", 
                    updated_transaction.amount, updated_transaction.currency)
            )
        } else {
            (
                NotificationType::FundingProcessing,
                "Crypto Funding Processing",
                &format!("Your crypto funding of {} {} is being processed. We'll notify you when it's complete.", 
                    updated_transaction.amount, updated_transaction.currency)
            )
        };

        self.send_funding_notification(
            &auth_context.user_id,
            notification_type,
            title,
            message,
            HashMap::from([
                ("funding_id".to_string(), funding_id.to_string()),
                ("transaction_hash".to_string(), req.transaction_hash.clone()),
                ("status".to_string(), updated_transaction.status.to_string()),
            ]),
        ).await;

        Ok(Response::new(ConfirmCryptoFundingResponse {
            transaction: Some(self.funding_transaction_to_proto(&updated_transaction)),
            confirmation_status: confirmation_status.to_string(),
        }))
    }

    /// Get cryptocurrency funding status
    async fn get_crypto_funding_status(
        &self,
        request: Request<GetCryptoFundingStatusRequest>,
    ) -> Result<Response<GetCryptoFundingStatusResponse>, Status> {
        let req = request.get_ref();
        
        // Extract authentication
        let auth_context = self.auth_service.extract_auth(&request).await?;
        self.auth_service.check_permission(&auth_context, Permission::PermissionCardFunding)?;

        // Parse funding ID
        let funding_id = Uuid::parse_str(&req.funding_id)
            .map_err(|_| Status::invalid_argument("Invalid funding ID"))?;

        // Get funding transaction
        let transaction = self.funding_repository
            .get_funding_transaction_by_user(&auth_context.user_id, &funding_id)
            .await
            .map_err(|e| Status::internal(format!("Failed to get funding transaction: {}", e)))?
            .ok_or_else(|| Status::not_found("Funding transaction not found"))?;

        // Extract crypto details from metadata
        let crypto_currency_str = transaction.metadata.get("crypto_currency")
            .ok_or_else(|| Status::internal("Missing crypto currency in transaction metadata"))?;
        let network = transaction.metadata.get("network")
            .ok_or_else(|| Status::internal("Missing network in transaction metadata"))?;
        let deposit_address = transaction.metadata.get("deposit_address")
            .ok_or_else(|| Status::internal("Missing deposit address in transaction metadata"))?;
        let required_confirmations = transaction.metadata.get("required_confirmations")
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(6);
        let current_confirmations = transaction.metadata.get("confirmations")
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(0);

        // Parse crypto currency
        let crypto_currency = match crypto_currency_str.as_str() {
            "USDT" => CryptoCurrency::USDT,
            "USDC" => CryptoCurrency::USDC,
            "DAI" => CryptoCurrency::DAI,
            "BUSD" => CryptoCurrency::BUSD,
            _ => return Err(Status::internal("Invalid crypto currency in metadata")),
        };

        // Create crypto funding details
        let crypto_details = crate::proto::fo3::wallet::v1::CryptoFundingDetails {
            currency: Self::crypto_currency_to_proto(&crypto_currency),
            network: network.clone(),
            deposit_address: deposit_address.clone(),
            required_confirmations: required_confirmations.to_string(),
            current_confirmations: current_confirmations.to_string(),
            transaction_hash: transaction.external_transaction_id.clone().unwrap_or_default(),
            exchange_rate: transaction.exchange_rate.map(|r| r.to_string()).unwrap_or_else(|| "1.0".to_string()),
            expires_at: transaction.expires_at.map(|dt| dt.to_rfc3339()).unwrap_or_default(),
        };

        Ok(Response::new(GetCryptoFundingStatusResponse {
            funding_details: Some(crypto_details),
            transaction: Some(self.funding_transaction_to_proto(&transaction)),
        }))
    }

    /// Initiate ACH funding
    async fn initiate_ach_funding(
        &self,
        request: Request<InitiateAchFundingRequest>,
    ) -> Result<Response<InitiateAchFundingResponse>, Status> {
        let req = request.get_ref();
        
        // Parse request parameters
        let card_id = Uuid::parse_str(&req.card_id)
            .map_err(|_| Status::invalid_argument("Invalid card ID"))?;
        let bank_account_id = Uuid::parse_str(&req.bank_account_id)
            .map_err(|_| Status::invalid_argument("Invalid bank account ID"))?;
        let amount = Decimal::from_str(&req.amount)
            .map_err(|_| Status::invalid_argument("Invalid amount"))?;

        // Validate ACH funding request
        let auth_context = self.funding_guard
            .validate_funding_transaction(&request, &card_id, &bank_account_id, &amount, "USD")
            .await?;

        // Calculate fees (higher for same-day ACH)
        let base_fee_percentage = if req.same_day {
            Decimal::from_str("0.015").unwrap() // 1.5% for same-day ACH
        } else {
            Decimal::from_str("0.005").unwrap() // 0.5% for standard ACH
        };

        let fee_amount = amount * base_fee_percentage;
        let net_amount = amount - fee_amount;

        let fee_calculation = crate::models::card_funding::FeeCalculation {
            base_amount: amount,
            fee_percentage: base_fee_percentage,
            fee_amount,
            net_amount,
            exchange_rate: None,
            exchange_fee: None,
            total_fee: fee_amount,
            fee_breakdown: vec![
                crate::models::card_funding::FeeBreakdown {
                    fee_type: if req.same_day { "same_day_ach_fee" } else { "standard_ach_fee" }.to_string(),
                    amount: fee_amount,
                    description: if req.same_day {
                        "Same-day ACH processing fee".to_string()
                    } else {
                        "Standard ACH processing fee".to_string()
                    },
                }
            ],
        };

        // Create ACH funding transaction
        let funding_transaction = FundingTransaction {
            id: Uuid::new_v4(),
            user_id: auth_context.user_id,
            card_id,
            funding_source_id: bank_account_id,
            status: FundingTransactionStatus::Processing,
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
                ("funding_type".to_string(), "ach".to_string()),
                ("same_day".to_string(), req.same_day.to_string()),
                ("bank_account_id".to_string(), bank_account_id.to_string()),
            ]),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            completed_at: None,
            expires_at: None,
        };

        // Save transaction
        let created_transaction = self.funding_repository
            .create_funding_transaction(&funding_transaction)
            .await
            .map_err(|e| Status::internal(format!("Failed to create ACH funding: {}", e)))?;

        // Estimate completion time
        let estimated_completion = if req.same_day {
            "Same business day"
        } else {
            "1-3 business days"
        }.to_string();

        // Log the operation
        self.audit_logger.log_operation(
            &auth_context.user_id.to_string(),
            "initiate_ach_funding",
            &format!("Initiated ACH funding: {} USD from bank account {} to card {}", 
                amount, bank_account_id, card_id),
            true,
            request.remote_addr(),
        ).await;

        // Send notification
        self.send_funding_notification(
            &auth_context.user_id,
            NotificationType::FundingInitiated,
            "ACH Funding Initiated",
            &format!("Your ACH funding of ${} has been initiated and will complete in {}.", 
                amount, estimated_completion),
            HashMap::from([
                ("transaction_id".to_string(), created_transaction.id.to_string()),
                ("amount".to_string(), amount.to_string()),
                ("estimated_completion".to_string(), estimated_completion.clone()),
            ]),
        ).await;

        Ok(Response::new(InitiateAchFundingResponse {
            transaction: Some(self.funding_transaction_to_proto(&created_transaction)),
            fee_calculation: Some(self.fee_calculation_to_proto(&fee_calculation)),
            estimated_completion,
        }))
    }
}
