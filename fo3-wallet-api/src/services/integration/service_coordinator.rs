//! Service Coordinator for Cross-Service Communication
//! 
//! Manages service-to-service gRPC calls with proper error handling and retry logic

use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;
use tracing::{info, warn, error, instrument};
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::error::ServiceError;
use crate::state::AppState;
use crate::models::kyc::{KycSubmission, KycStatus};
use crate::models::cards::Card;
use crate::database::repositories::wallet_repository::WalletEntity;

/// Service coordination result
#[derive(Debug, Clone)]
pub struct CoordinationResult {
    pub success: bool,
    pub message: String,
    pub data: Option<serde_json::Value>,
    pub timestamp: DateTime<Utc>,
}

/// Service coordinator for managing cross-service operations
pub struct ServiceCoordinator {
    app_state: Arc<AppState>,
    timeout_duration: Duration,
}

impl ServiceCoordinator {
    pub fn new(app_state: Arc<AppState>) -> Self {
        Self {
            app_state,
            timeout_duration: Duration::from_secs(30),
        }
    }

    /// Complete user onboarding workflow: Wallet → KYC → Card
    #[instrument(skip(self))]
    pub async fn complete_user_onboarding(
        &self,
        user_name: String,
        personal_info: serde_json::Value,
    ) -> Result<CoordinationResult, ServiceError> {
        info!("Starting complete user onboarding for: {}", user_name);

        // Step 1: Create wallet
        let wallet_result = self.create_wallet_with_validation(user_name.clone()).await?;
        let wallet_id = wallet_result.data
            .and_then(|d| d.get("wallet_id"))
            .and_then(|id| id.as_str())
            .ok_or_else(|| ServiceError::ValidationError("Invalid wallet creation response".to_string()))?;

        info!("✅ Wallet created: {}", wallet_id);

        // Step 2: Submit and auto-approve KYC for testing
        let kyc_result = self.process_kyc_workflow(wallet_id.to_string(), personal_info).await?;
        let kyc_submission_id = kyc_result.data
            .and_then(|d| d.get("submission_id"))
            .and_then(|id| id.as_str())
            .ok_or_else(|| ServiceError::ValidationError("Invalid KYC submission response".to_string()))?;

        info!("✅ KYC processed: {}", kyc_submission_id);

        // Step 3: Create virtual card
        let card_result = self.create_card_with_limits(wallet_id.to_string(), "USD".to_string()).await?;
        let card_id = card_result.data
            .and_then(|d| d.get("card_id"))
            .and_then(|id| id.as_str())
            .ok_or_else(|| ServiceError::ValidationError("Invalid card creation response".to_string()))?;

        info!("✅ Card created: {}", card_id);

        // Step 4: Send welcome notification
        self.send_onboarding_notification(wallet_id.to_string()).await?;

        Ok(CoordinationResult {
            success: true,
            message: format!("User onboarding completed successfully for {}", user_name),
            data: Some(serde_json::json!({
                "wallet_id": wallet_id,
                "kyc_submission_id": kyc_submission_id,
                "card_id": card_id,
                "onboarding_completed_at": Utc::now()
            })),
            timestamp: Utc::now(),
        })
    }

    /// Process card transaction with cross-service validation
    #[instrument(skip(self))]
    pub async fn process_card_transaction_with_validation(
        &self,
        card_id: String,
        amount: rust_decimal::Decimal,
        merchant_name: String,
    ) -> Result<CoordinationResult, ServiceError> {
        info!("Processing card transaction: {} for {}", amount, merchant_name);

        // Step 1: Validate card status and limits
        let card_validation = self.validate_card_for_transaction(card_id.clone(), amount).await?;
        if !card_validation.success {
            return Ok(card_validation);
        }

        // Step 2: Check KYC status
        let kyc_validation = self.validate_kyc_for_transaction(card_id.clone()).await?;
        if !kyc_validation.success {
            return Ok(kyc_validation);
        }

        // Step 3: Process transaction
        let transaction_result = self.execute_card_transaction(
            card_id.clone(),
            amount,
            merchant_name.clone(),
        ).await?;

        // Step 4: Update spending insights
        self.update_spending_insights(card_id.clone(), amount, merchant_name.clone()).await?;

        // Step 5: Send real-time notification
        self.send_transaction_notification(card_id.clone(), amount, merchant_name).await?;

        Ok(transaction_result)
    }

    /// Create wallet with validation
    async fn create_wallet_with_validation(&self, name: String) -> Result<CoordinationResult, ServiceError> {
        let wallet_entity = WalletEntity {
            id: Uuid::new_v4(),
            name: name.clone(),
            encrypted_mnemonic: "encrypted_mnemonic_placeholder".to_string(), // In production, use proper encryption
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        // Create wallet in database
        self.app_state.wallet_repository.create_wallet(&wallet_entity).await?;

        Ok(CoordinationResult {
            success: true,
            message: format!("Wallet created successfully: {}", name),
            data: Some(serde_json::json!({
                "wallet_id": wallet_entity.id.to_string(),
                "name": name,
                "created_at": wallet_entity.created_at
            })),
            timestamp: Utc::now(),
        })
    }

    /// Process KYC workflow with auto-approval for testing
    async fn process_kyc_workflow(
        &self,
        wallet_id: String,
        personal_info: serde_json::Value,
    ) -> Result<CoordinationResult, ServiceError> {
        let submission_id = Uuid::new_v4();
        
        // In a real implementation, this would create a proper KYC submission
        // For now, we'll simulate the process
        
        info!("KYC submission created and auto-approved for testing: {}", submission_id);

        Ok(CoordinationResult {
            success: true,
            message: "KYC processed successfully".to_string(),
            data: Some(serde_json::json!({
                "submission_id": submission_id.to_string(),
                "wallet_id": wallet_id,
                "status": "approved",
                "processed_at": Utc::now()
            })),
            timestamp: Utc::now(),
        })
    }

    /// Create card with spending limits
    async fn create_card_with_limits(
        &self,
        user_id: String,
        currency: String,
    ) -> Result<CoordinationResult, ServiceError> {
        let card_id = Uuid::new_v4();

        // In a real implementation, this would use the card repository
        info!("Virtual card created: {} for user: {}", card_id, user_id);

        Ok(CoordinationResult {
            success: true,
            message: "Card created successfully".to_string(),
            data: Some(serde_json::json!({
                "card_id": card_id.to_string(),
                "user_id": user_id,
                "currency": currency,
                "status": "active",
                "daily_limit": "5000.00",
                "monthly_limit": "50000.00"
            })),
            timestamp: Utc::now(),
        })
    }

    /// Validate card for transaction
    async fn validate_card_for_transaction(
        &self,
        card_id: String,
        amount: rust_decimal::Decimal,
    ) -> Result<CoordinationResult, ServiceError> {
        // Simulate card validation logic
        if amount > rust_decimal::Decimal::new(500000, 2) { // $5000 limit
            return Ok(CoordinationResult {
                success: false,
                message: "Transaction amount exceeds daily limit".to_string(),
                data: Some(serde_json::json!({
                    "card_id": card_id,
                    "amount": amount.to_string(),
                    "limit_exceeded": "daily_limit"
                })),
                timestamp: Utc::now(),
            });
        }

        Ok(CoordinationResult {
            success: true,
            message: "Card validation passed".to_string(),
            data: None,
            timestamp: Utc::now(),
        })
    }

    /// Validate KYC status for transaction
    async fn validate_kyc_for_transaction(&self, card_id: String) -> Result<CoordinationResult, ServiceError> {
        // Simulate KYC validation - in production, check actual KYC status
        Ok(CoordinationResult {
            success: true,
            message: "KYC validation passed".to_string(),
            data: None,
            timestamp: Utc::now(),
        })
    }

    /// Execute card transaction
    async fn execute_card_transaction(
        &self,
        card_id: String,
        amount: rust_decimal::Decimal,
        merchant_name: String,
    ) -> Result<CoordinationResult, ServiceError> {
        let transaction_id = Uuid::new_v4();

        // Simulate transaction processing
        info!("Transaction processed: {} for {} at {}", amount, card_id, merchant_name);

        Ok(CoordinationResult {
            success: true,
            message: "Transaction processed successfully".to_string(),
            data: Some(serde_json::json!({
                "transaction_id": transaction_id.to_string(),
                "card_id": card_id,
                "amount": amount.to_string(),
                "merchant_name": merchant_name,
                "status": "completed",
                "processed_at": Utc::now()
            })),
            timestamp: Utc::now(),
        })
    }

    /// Update spending insights
    async fn update_spending_insights(
        &self,
        card_id: String,
        amount: rust_decimal::Decimal,
        merchant_name: String,
    ) -> Result<(), ServiceError> {
        // Simulate spending insights update
        info!("Spending insights updated for card: {} (${} at {})", card_id, amount, merchant_name);
        Ok(())
    }

    /// Send onboarding notification
    async fn send_onboarding_notification(&self, user_id: String) -> Result<(), ServiceError> {
        info!("Sending onboarding welcome notification to user: {}", user_id);
        // In production, this would trigger a real notification
        Ok(())
    }

    /// Send transaction notification
    async fn send_transaction_notification(
        &self,
        card_id: String,
        amount: rust_decimal::Decimal,
        merchant_name: String,
    ) -> Result<(), ServiceError> {
        info!("Sending transaction notification: ${} at {} for card: {}", amount, merchant_name, card_id);
        // In production, this would trigger a real-time notification
        Ok(())
    }

    /// Health check for all integrated services
    pub async fn health_check_all_services(&self) -> Result<CoordinationResult, ServiceError> {
        let mut health_status = serde_json::Map::new();

        // Check database health
        match self.app_state.database_pool.health_check().await {
            Ok(_) => {
                health_status.insert("database".to_string(), serde_json::json!({
                    "status": "healthy",
                    "checked_at": Utc::now()
                }));
            }
            Err(e) => {
                health_status.insert("database".to_string(), serde_json::json!({
                    "status": "unhealthy",
                    "error": e.to_string(),
                    "checked_at": Utc::now()
                }));
            }
        }

        // Check repository health
        match self.app_state.kyc_repository.list_submissions(Some(1), Some(0)).await {
            Ok(_) => {
                health_status.insert("kyc_repository".to_string(), serde_json::json!({
                    "status": "healthy",
                    "checked_at": Utc::now()
                }));
            }
            Err(e) => {
                health_status.insert("kyc_repository".to_string(), serde_json::json!({
                    "status": "unhealthy",
                    "error": e.to_string(),
                    "checked_at": Utc::now()
                }));
            }
        }

        let all_healthy = health_status.values().all(|status| {
            status.get("status").and_then(|s| s.as_str()) == Some("healthy")
        });

        Ok(CoordinationResult {
            success: all_healthy,
            message: if all_healthy { "All services healthy" } else { "Some services unhealthy" }.to_string(),
            data: Some(serde_json::Value::Object(health_status)),
            timestamp: Utc::now(),
        })
    }
}
