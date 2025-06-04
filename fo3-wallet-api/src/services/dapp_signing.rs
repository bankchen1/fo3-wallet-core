//! DApp signing service implementation

use std::sync::Arc;
use std::collections::HashMap;
use tonic::{Request, Response, Status};
use uuid::Uuid;
use rust_decimal::Decimal;
use chrono::{DateTime, Utc};

use crate::proto::fo3::wallet::v1::{
    d_app_signing_service_server::DAppSigningService,
    *,
};
use crate::state::AppState;
use crate::middleware::{
    auth::{AuthContext, AuthService},
    audit::AuditLogger,
    dapp_guard::DAppGuard,
};
use crate::models::dapp_signing::{
    SignatureResult, SimulationResult, ValidationResult, SigningHistoryEntry, SigningAnalytics,
    SignatureType, TransactionType, ValidationStatus, RiskLevel, KeyType as DAppKeyType,
    DAppSigningRepository,
};
use crate::models::notifications::{
    NotificationType, NotificationPriority, DeliveryChannel
};

/// DApp signing service implementation
#[derive(Debug)]
pub struct DAppSigningServiceImpl {
    state: Arc<AppState>,
    auth_service: Arc<AuthService>,
    audit_logger: Arc<AuditLogger>,
    dapp_guard: Arc<DAppGuard>,
    dapp_signing_repository: Arc<dyn DAppSigningRepository>,
}

impl DAppSigningServiceImpl {
    pub fn new(
        state: Arc<AppState>,
        auth_service: Arc<AuthService>,
        audit_logger: Arc<AuditLogger>,
        dapp_guard: Arc<DAppGuard>,
        dapp_signing_repository: Arc<dyn DAppSigningRepository>,
    ) -> Self {
        Self {
            state,
            auth_service,
            audit_logger,
            dapp_guard,
            dapp_signing_repository,
        }
    }

    /// Convert proto SignatureType to model SignatureType
    fn proto_to_model_signature_type(proto_type: i32) -> Result<SignatureType, Status> {
        match SignatureType::try_from(proto_type) {
            Ok(SignatureType::SignatureTypePersonalSign) => Ok(crate::models::dapp_signing::SignatureType::PersonalSign),
            Ok(SignatureType::SignatureTypeTypedDataV1) => Ok(crate::models::dapp_signing::SignatureType::TypedDataV1),
            Ok(SignatureType::SignatureTypeTypedDataV3) => Ok(crate::models::dapp_signing::SignatureType::TypedDataV3),
            Ok(SignatureType::SignatureTypeTypedDataV4) => Ok(crate::models::dapp_signing::SignatureType::TypedDataV4),
            Ok(SignatureType::SignatureTypeSolanaSignMessage) => Ok(crate::models::dapp_signing::SignatureType::SolanaSignMessage),
            Ok(SignatureType::SignatureTypeBitcoinSignMessage) => Ok(crate::models::dapp_signing::SignatureType::BitcoinSignMessage),
            _ => Err(Status::invalid_argument("Invalid signature type")),
        }
    }

    /// Convert model SignatureType to proto SignatureType
    fn model_to_proto_signature_type(model_type: crate::models::dapp_signing::SignatureType) -> SignatureType {
        match model_type {
            crate::models::dapp_signing::SignatureType::PersonalSign => SignatureType::SignatureTypePersonalSign,
            crate::models::dapp_signing::SignatureType::TypedDataV1 => SignatureType::SignatureTypeTypedDataV1,
            crate::models::dapp_signing::SignatureType::TypedDataV3 => SignatureType::SignatureTypeTypedDataV3,
            crate::models::dapp_signing::SignatureType::TypedDataV4 => SignatureType::SignatureTypeTypedDataV4,
            crate::models::dapp_signing::SignatureType::SolanaSignMessage => SignatureType::SignatureTypeSolanaSignMessage,
            crate::models::dapp_signing::SignatureType::BitcoinSignMessage => SignatureType::SignatureTypeBitcoinSignMessage,
        }
    }

    /// Convert proto TransactionType to model TransactionType
    fn proto_to_model_transaction_type(proto_type: i32) -> Result<TransactionType, Status> {
        match TransactionType::try_from(proto_type) {
            Ok(TransactionType::TransactionTypeTransfer) => Ok(crate::models::dapp_signing::TransactionType::Transfer),
            Ok(TransactionType::TransactionTypeContractCall) => Ok(crate::models::dapp_signing::TransactionType::ContractCall),
            Ok(TransactionType::TransactionTypeContractDeployment) => Ok(crate::models::dapp_signing::TransactionType::ContractDeployment),
            Ok(TransactionType::TransactionTypeTokenTransfer) => Ok(crate::models::dapp_signing::TransactionType::TokenTransfer),
            Ok(TransactionType::TransactionTypeNftTransfer) => Ok(crate::models::dapp_signing::TransactionType::NftTransfer),
            Ok(TransactionType::TransactionTypeDefiSwap) => Ok(crate::models::dapp_signing::TransactionType::DefiSwap),
            Ok(TransactionType::TransactionTypeDefiStake) => Ok(crate::models::dapp_signing::TransactionType::DefiStake),
            Ok(TransactionType::TransactionTypeDefiUnstake) => Ok(crate::models::dapp_signing::TransactionType::DefiUnstake),
            _ => Err(Status::invalid_argument("Invalid transaction type")),
        }
    }

    /// Convert model TransactionType to proto TransactionType
    fn model_to_proto_transaction_type(model_type: crate::models::dapp_signing::TransactionType) -> TransactionType {
        match model_type {
            crate::models::dapp_signing::TransactionType::Transfer => TransactionType::TransactionTypeTransfer,
            crate::models::dapp_signing::TransactionType::ContractCall => TransactionType::TransactionTypeContractCall,
            crate::models::dapp_signing::TransactionType::ContractDeployment => TransactionType::TransactionTypeContractDeployment,
            crate::models::dapp_signing::TransactionType::TokenTransfer => TransactionType::TransactionTypeTokenTransfer,
            crate::models::dapp_signing::TransactionType::NftTransfer => TransactionType::TransactionTypeNftTransfer,
            crate::models::dapp_signing::TransactionType::DefiSwap => TransactionType::TransactionTypeDefiSwap,
            crate::models::dapp_signing::TransactionType::DefiStake => TransactionType::TransactionTypeDefiStake,
            crate::models::dapp_signing::TransactionType::DefiUnstake => TransactionType::TransactionTypeDefiUnstake,
        }
    }

    /// Convert proto KeyType to model KeyType
    fn proto_to_model_key_type(proto_type: i32) -> Result<DAppKeyType, Status> {
        match KeyType::try_from(proto_type) {
            Ok(KeyType::KeyTypeEthereum) => Ok(DAppKeyType::Ethereum),
            Ok(KeyType::KeyTypeBitcoin) => Ok(DAppKeyType::Bitcoin),
            Ok(KeyType::KeyTypeSolana) => Ok(DAppKeyType::Solana),
            _ => Err(Status::invalid_argument("Invalid key type")),
        }
    }

    /// Convert model KeyType to proto KeyType
    fn model_to_proto_key_type(model_type: DAppKeyType) -> KeyType {
        match model_type {
            DAppKeyType::Ethereum => KeyType::KeyTypeEthereum,
            DAppKeyType::Bitcoin => KeyType::KeyTypeBitcoin,
            DAppKeyType::Solana => KeyType::KeyTypeSolana,
        }
    }

    /// Convert model RiskLevel to proto RiskLevel
    fn model_to_proto_risk_level(model_level: crate::models::dapp_signing::RiskLevel) -> RiskLevel {
        match model_level {
            crate::models::dapp_signing::RiskLevel::Low => RiskLevel::RiskLevelLow,
            crate::models::dapp_signing::RiskLevel::Medium => RiskLevel::RiskLevelMedium,
            crate::models::dapp_signing::RiskLevel::High => RiskLevel::RiskLevelHigh,
            crate::models::dapp_signing::RiskLevel::Critical => RiskLevel::RiskLevelCritical,
        }
    }

    /// Convert model ValidationStatus to proto ValidationStatus
    fn model_to_proto_validation_status(model_status: crate::models::dapp_signing::ValidationStatus) -> ValidationStatus {
        match model_status {
            crate::models::dapp_signing::ValidationStatus::Valid => ValidationStatus::ValidationStatusValid,
            crate::models::dapp_signing::ValidationStatus::Invalid => ValidationStatus::ValidationStatusInvalid,
            crate::models::dapp_signing::ValidationStatus::Warning => ValidationStatus::ValidationStatusWarning,
            crate::models::dapp_signing::ValidationStatus::Blocked => ValidationStatus::ValidationStatusBlocked,
        }
    }

    /// Convert model SignatureResult to proto
    fn model_to_proto_signature_result(result: &SignatureResult) -> SignatureResult {
        SignatureResult {
            signature: result.signature.clone(),
            public_key: result.public_key.clone(),
            address: result.address.clone(),
            key_type: Self::model_to_proto_key_type(result.key_type) as i32,
            signature_type: Self::model_to_proto_signature_type(result.signature_type) as i32,
            signed_at: result.signed_at.timestamp(),
            transaction_hash: result.transaction_hash.clone().unwrap_or_default(),
            metadata: result.metadata.clone(),
        }
    }

    /// Convert model SimulationResult to proto
    fn model_to_proto_simulation_result(result: &SimulationResult) -> SimulationResult {
        SimulationResult {
            success: result.success,
            error_message: result.error_message.clone().unwrap_or_default(),
            gas_estimate: result.gas_estimate.clone(),
            gas_price: result.gas_price.clone(),
            total_fee: result.total_fee.clone(),
            state_changes: result.state_changes.clone(),
            events: result.events.clone(),
            risk_level: Self::model_to_proto_risk_level(result.risk_level) as i32,
            warnings: result.warnings.clone(),
            metadata: result.metadata.clone(),
        }
    }

    /// Convert model ValidationResult to proto
    fn model_to_proto_validation_result(result: &ValidationResult) -> ValidationResult {
        ValidationResult {
            status: Self::model_to_proto_validation_status(result.status) as i32,
            risk_level: Self::model_to_proto_risk_level(result.risk_level) as i32,
            warnings: result.warnings.clone(),
            errors: result.errors.clone(),
            is_whitelisted: result.is_whitelisted,
            is_blacklisted: result.is_blacklisted,
            risk_score: result.risk_score.to_string(),
            metadata: result.metadata.clone(),
        }
    }

    /// Send notification to user
    async fn send_notification(
        &self,
        user_id: &Uuid,
        notification_type: NotificationType,
        title: &str,
        message: &str,
        metadata: Option<HashMap<String, String>>,
    ) {
        if let Err(e) = self.state.notification_service.send_notification(
            user_id,
            notification_type,
            NotificationPriority::High, // Signing operations are high priority
            title,
            message,
            vec![DeliveryChannel::Push, DeliveryChannel::InApp],
            metadata,
        ).await {
            tracing::warn!("Failed to send notification: {}", e);
        }
    }

    /// Simulate transaction for security validation
    async fn simulate_transaction_internal(
        &self,
        from_address: &str,
        to_address: &str,
        amount: &str,
        data: Option<&str>,
        key_type: DAppKeyType,
        chain_id: &str,
    ) -> Result<SimulationResult, Status> {
        // In a real implementation, this would call actual blockchain simulation services
        // For now, we'll create a mock simulation based on transaction characteristics
        
        let amount_decimal = amount.parse::<Decimal>()
            .map_err(|_| Status::invalid_argument("Invalid amount format"))?;

        // Determine risk level based on amount and transaction type
        let risk_level = if amount_decimal > Decimal::from(100_000) {
            crate::models::dapp_signing::RiskLevel::Critical
        } else if amount_decimal > Decimal::from(10_000) {
            crate::models::dapp_signing::RiskLevel::High
        } else if amount_decimal > Decimal::from(1_000) {
            crate::models::dapp_signing::RiskLevel::Medium
        } else {
            crate::models::dapp_signing::RiskLevel::Low
        };

        // Estimate gas based on chain type and transaction complexity
        let (gas_estimate, gas_price) = match key_type {
            DAppKeyType::Ethereum => {
                let base_gas = if data.is_some() { "100000" } else { "21000" };
                (base_gas.to_string(), "20000000000".to_string()) // 20 gwei
            }
            DAppKeyType::Solana => {
                ("5000".to_string(), "5000".to_string()) // 5000 lamports
            }
            DAppKeyType::Bitcoin => {
                ("250".to_string(), "10".to_string()) // 250 bytes, 10 sat/byte
            }
        };

        // Calculate total fee
        let gas_estimate_num = gas_estimate.parse::<u64>().unwrap_or(0);
        let gas_price_num = gas_price.parse::<u64>().unwrap_or(0);
        let total_fee = (gas_estimate_num * gas_price_num).to_string();

        let simulation_result = SimulationResult::success(
            gas_estimate,
            gas_price,
            total_fee,
            risk_level,
        );

        // Store simulation result
        self.dapp_signing_repository
            .create_simulation_result(&simulation_result)
            .await
            .map_err(|e| Status::internal(format!("Failed to store simulation result: {}", e)))?;

        Ok(simulation_result)
    }
}

#[tonic::async_trait]
impl DAppSigningService for DAppSigningServiceImpl {
    /// Sign a message
    async fn sign_message(
        &self,
        request: Request<SignMessageRequest>,
    ) -> Result<Response<SignMessageResponse>, Status> {
        let req = request.get_ref();

        // Parse session ID
        let session_id = Uuid::parse_str(&req.session_id)
            .map_err(|_| Status::invalid_argument("Invalid session ID"))?;

        // Convert types
        let signature_type = Self::proto_to_model_signature_type(req.signature_type)?;
        let key_type = Self::proto_to_model_key_type(req.key_type)?;

        // Validate message signing request
        let auth_context = self.dapp_guard
            .validate_message_signing(&request, &session_id, &req.address, &req.message, signature_type, key_type)
            .await?;

        // Parse user ID
        let user_id = Uuid::parse_str(&auth_context.user_id)
            .map_err(|_| Status::invalid_argument("Invalid user ID"))?;

        // In a real implementation, this would call the actual signing service
        // For now, we'll create a mock signature
        let mock_signature = format!("0x{}", hex::encode(format!("signature_{}_{}", req.message, req.address)));
        let mock_public_key = format!("0x{}", hex::encode(format!("pubkey_{}", req.address)));

        // Create signature result
        let signature_result = SignatureResult::new(
            mock_signature,
            mock_public_key,
            req.address.clone(),
            key_type,
            signature_type,
            None, // No transaction hash for message signing
        );

        // Store signature result
        let stored_result = self.dapp_signing_repository
            .create_signature_result(&signature_result)
            .await
            .map_err(|e| Status::internal(format!("Failed to store signature result: {}", e)))?;

        // Create history entry
        let mut history_entry = SigningHistoryEntry::new(
            user_id,
            session_id,
            req.metadata.get("dapp_url").cloned().unwrap_or_default(),
            signature_type,
            key_type,
            req.chain_id.clone(),
            req.address.clone(),
            true, // success
            crate::models::dapp_signing::RiskLevel::Low, // Message signing is generally low risk
        );
        history_entry.metadata = req.metadata.clone();

        // Store history entry
        self.dapp_signing_repository
            .create_history_entry(&history_entry)
            .await
            .map_err(|e| Status::internal(format!("Failed to store history entry: {}", e)))?;

        // Send notification to user
        self.send_notification(
            &user_id,
            NotificationType::TransactionSigned,
            "Message Signed",
            &format!("Message signed successfully for {}", req.address),
            Some([
                ("session_id".to_string(), session_id.to_string()),
                ("signature_type".to_string(), format!("{:?}", signature_type)),
                ("address".to_string(), req.address.clone()),
            ].into()),
        ).await;

        // Log message signing
        self.audit_logger.log_action(
            &user_id.to_string(),
            "message_signed",
            &format!("Session: {}, Address: {}, Type: {:?}", session_id, req.address, signature_type),
            true,
            None,
        ).await;

        let response = SignMessageResponse {
            result: Some(Self::model_to_proto_signature_result(&stored_result)),
        };

        Ok(Response::new(response))
    }

    /// Verify a message signature
    async fn verify_message(
        &self,
        request: Request<VerifyMessageRequest>,
    ) -> Result<Response<VerifyMessageResponse>, Status> {
        let req = request.get_ref();

        // Extract authentication
        let _auth_context = self.auth_service.extract_auth(&request).await?;

        // Convert types
        let signature_type = Self::proto_to_model_signature_type(req.signature_type)?;
        let key_type = Self::proto_to_model_key_type(req.key_type)?;

        // In a real implementation, this would call actual signature verification
        // For now, we'll do a mock verification based on our mock signature format
        let expected_signature = format!("0x{}", hex::encode(format!("signature_{}_{}", req.message, req.address)));
        let is_valid = req.signature == expected_signature;

        let recovered_address = if is_valid {
            req.address.clone()
        } else {
            String::new()
        };

        let response = VerifyMessageResponse {
            is_valid,
            recovered_address,
        };

        Ok(Response::new(response))
    }

    /// Sign a transaction
    async fn sign_transaction(
        &self,
        request: Request<SignTransactionRequest>,
    ) -> Result<Response<SignTransactionResponse>, Status> {
        let req = request.get_ref();

        // Parse session ID
        let session_id = Uuid::parse_str(&req.session_id)
            .map_err(|_| Status::invalid_argument("Invalid session ID"))?;

        // Convert types
        let key_type = Self::proto_to_model_key_type(req.key_type)?;
        let transaction_type = Self::proto_to_model_transaction_type(req.transaction_type)?;

        // Parse amount
        let amount = req.amount.parse::<Decimal>()
            .map_err(|_| Status::invalid_argument("Invalid amount format"))?;

        // Validate transaction signing request
        let auth_context = self.dapp_guard
            .validate_transaction_signing(
                &request,
                &session_id,
                &req.from_address,
                &req.to_address,
                &amount,
                key_type,
                &req.chain_id,
                transaction_type,
            )
            .await?;

        // Parse user ID
        let user_id = Uuid::parse_str(&auth_context.user_id)
            .map_err(|_| Status::invalid_argument("Invalid user ID"))?;

        // Simulate transaction if requested
        let simulation_result = if req.simulate_first {
            Some(self.simulate_transaction_internal(
                &req.from_address,
                &req.to_address,
                &req.amount,
                if req.data.is_empty() { None } else { Some(&req.data) },
                key_type,
                &req.chain_id,
            ).await?)
        } else {
            None
        };

        // Validate transaction
        let validation_result = ValidationResult::valid(
            crate::models::dapp_signing::RiskLevel::Medium,
            Decimal::from(25), // Risk score out of 100
        );

        // Store validation result
        self.dapp_signing_repository
            .create_validation_result(&validation_result)
            .await
            .map_err(|e| Status::internal(format!("Failed to store validation result: {}", e)))?;

        // In a real implementation, this would call the actual transaction signing service
        let mock_signature = format!("0x{}", hex::encode(format!("tx_signature_{}_{}", req.from_address, req.to_address)));
        let mock_public_key = format!("0x{}", hex::encode(format!("pubkey_{}", req.from_address)));
        let mock_tx_hash = format!("0x{}", hex::encode(format!("tx_hash_{}_{}", req.from_address, chrono::Utc::now().timestamp())));

        // Create signature result
        let signature_result = SignatureResult::new(
            mock_signature,
            mock_public_key,
            req.from_address.clone(),
            key_type,
            crate::models::dapp_signing::SignatureType::PersonalSign, // Default for transactions
            Some(mock_tx_hash.clone()),
        );

        // Store signature result
        let stored_result = self.dapp_signing_repository
            .create_signature_result(&signature_result)
            .await
            .map_err(|e| Status::internal(format!("Failed to store signature result: {}", e)))?;

        // Create history entry
        let mut history_entry = SigningHistoryEntry::new(
            user_id,
            session_id,
            req.metadata.get("dapp_url").cloned().unwrap_or_default(),
            crate::models::dapp_signing::SignatureType::PersonalSign,
            key_type,
            req.chain_id.clone(),
            req.from_address.clone(),
            true, // success
            validation_result.risk_level,
        );
        history_entry.transaction_type = Some(transaction_type);
        history_entry.amount = Some(amount);
        history_entry.recipient = Some(req.to_address.clone());
        history_entry.metadata = req.metadata.clone();

        // Store history entry
        self.dapp_signing_repository
            .create_history_entry(&history_entry)
            .await
            .map_err(|e| Status::internal(format!("Failed to store history entry: {}", e)))?;

        // Send notification to user
        self.send_notification(
            &user_id,
            NotificationType::TransactionSigned,
            "Transaction Signed",
            &format!("Transaction signed: {} {} to {}", req.amount, "tokens", req.to_address),
            Some([
                ("session_id".to_string(), session_id.to_string()),
                ("transaction_hash".to_string(), mock_tx_hash),
                ("amount".to_string(), req.amount.clone()),
                ("recipient".to_string(), req.to_address.clone()),
            ].into()),
        ).await;

        // Log transaction signing
        self.audit_logger.log_action(
            &user_id.to_string(),
            "transaction_signed",
            &format!("Session: {}, From: {}, To: {}, Amount: {}, Type: {:?}",
                session_id, req.from_address, req.to_address, req.amount, transaction_type),
            true,
            None,
        ).await;

        let response = SignTransactionResponse {
            result: Some(Self::model_to_proto_signature_result(&stored_result)),
            simulation: simulation_result.map(|s| Self::model_to_proto_simulation_result(&s)),
            validation: Some(Self::model_to_proto_validation_result(&validation_result)),
        };

        Ok(Response::new(response))
    }

    /// Simulate a transaction
    async fn simulate_transaction(
        &self,
        request: Request<SimulateTransactionRequest>,
    ) -> Result<Response<SimulateTransactionResponse>, Status> {
        let req = request.get_ref();

        // Extract authentication
        let _auth_context = self.auth_service.extract_auth(&request).await?;

        // Convert key type
        let key_type = Self::proto_to_model_key_type(req.key_type)?;

        // Simulate transaction
        let simulation_result = self.simulate_transaction_internal(
            &req.from_address,
            &req.to_address,
            &req.amount,
            if req.data.is_empty() { None } else { Some(&req.data) },
            key_type,
            &req.chain_id,
        ).await?;

        let response = SimulateTransactionResponse {
            result: Some(Self::model_to_proto_simulation_result(&simulation_result)),
        };

        Ok(Response::new(response))
    }

    /// Estimate gas for a transaction
    async fn estimate_gas(
        &self,
        request: Request<EstimateGasRequest>,
    ) -> Result<Response<EstimateGasResponse>, Status> {
        let req = request.get_ref();

        // Extract authentication
        let _auth_context = self.auth_service.extract_auth(&request).await?;

        // Convert key type
        let key_type = Self::proto_to_model_key_type(req.key_type)?;

        // Estimate gas based on chain type and transaction complexity
        let (gas_estimate, gas_price) = match key_type {
            DAppKeyType::Ethereum => {
                let base_gas = if req.data.is_empty() { "21000" } else { "100000" };
                (base_gas.to_string(), "20000000000".to_string()) // 20 gwei
            }
            DAppKeyType::Solana => {
                ("5000".to_string(), "5000".to_string()) // 5000 lamports
            }
            DAppKeyType::Bitcoin => {
                ("250".to_string(), "10".to_string()) // 250 bytes, 10 sat/byte
            }
        };

        // Calculate total fee
        let gas_estimate_num = gas_estimate.parse::<u64>().unwrap_or(0);
        let gas_price_num = gas_price.parse::<u64>().unwrap_or(0);
        let total_fee = (gas_estimate_num * gas_price_num).to_string();

        let response = EstimateGasResponse {
            gas_estimate,
            gas_price,
            total_fee,
        };

        Ok(Response::new(response))
    }

    /// Batch sign transactions
    async fn batch_sign_transactions(
        &self,
        request: Request<BatchSignTransactionsRequest>,
    ) -> Result<Response<BatchSignTransactionsResponse>, Status> {
        let req = request.get_ref();

        // Parse session ID
        let session_id = Uuid::parse_str(&req.session_id)
            .map_err(|_| Status::invalid_argument("Invalid session ID"))?;

        // Validate batch signing request
        let _auth_context = self.dapp_guard
            .validate_batch_signing(&request, &session_id, req.transactions.len(), "transactions")
            .await?;

        let mut results = Vec::new();
        let mut successful_count = 0;
        let mut failed_count = 0;

        for transaction_req in &req.transactions {
            // Create individual request for each transaction
            let individual_request = Request::new(transaction_req.clone());

            match self.sign_transaction(individual_request).await {
                Ok(response) => {
                    results.push(response.into_inner());
                    successful_count += 1;
                }
                Err(status) => {
                    // Create error response
                    let error_result = SignTransactionResponse {
                        result: None,
                        simulation: None,
                        validation: Some(ValidationResult {
                            status: ValidationStatus::ValidationStatusInvalid as i32,
                            risk_level: RiskLevel::RiskLevelCritical as i32,
                            warnings: vec![],
                            errors: vec![status.message().to_string()],
                            is_whitelisted: false,
                            is_blacklisted: false,
                            risk_score: "100".to_string(),
                            metadata: HashMap::new(),
                        }),
                    };
                    results.push(error_result);
                    failed_count += 1;

                    // If fail_on_first_error is true, stop processing
                    if req.fail_on_first_error {
                        break;
                    }
                }
            }
        }

        let response = BatchSignTransactionsResponse {
            results,
            successful_count,
            failed_count,
        };

        Ok(Response::new(response))
    }

    /// Batch sign messages
    async fn batch_sign_messages(
        &self,
        request: Request<BatchSignMessagesRequest>,
    ) -> Result<Response<BatchSignMessagesResponse>, Status> {
        let req = request.get_ref();

        // Parse session ID
        let session_id = Uuid::parse_str(&req.session_id)
            .map_err(|_| Status::invalid_argument("Invalid session ID"))?;

        // Validate batch signing request
        let _auth_context = self.dapp_guard
            .validate_batch_signing(&request, &session_id, req.messages.len(), "messages")
            .await?;

        let mut results = Vec::new();
        let mut successful_count = 0;
        let mut failed_count = 0;

        for message_req in &req.messages {
            // Create individual request for each message
            let individual_request = Request::new(message_req.clone());

            match self.sign_message(individual_request).await {
                Ok(response) => {
                    results.push(response.into_inner());
                    successful_count += 1;
                }
                Err(_status) => {
                    // Create error response
                    let error_result = SignMessageResponse {
                        result: None,
                    };
                    results.push(error_result);
                    failed_count += 1;

                    // If fail_on_first_error is true, stop processing
                    if req.fail_on_first_error {
                        break;
                    }
                }
            }
        }

        let response = BatchSignMessagesResponse {
            results,
            successful_count,
            failed_count,
        };

        Ok(Response::new(response))
    }

    /// Validate a transaction
    async fn validate_transaction(
        &self,
        request: Request<ValidateTransactionRequest>,
    ) -> Result<Response<ValidateTransactionResponse>, Status> {
        let req = request.get_ref();

        // Extract authentication
        let _auth_context = self.auth_service.extract_auth(&request).await?;

        // Convert types
        let key_type = Self::proto_to_model_key_type(req.key_type)?;
        let transaction_type = Self::proto_to_model_transaction_type(req.transaction_type)?;

        // Parse amount
        let amount = req.amount.parse::<Decimal>()
            .map_err(|_| Status::invalid_argument("Invalid amount format"))?;

        // Perform validation logic
        let mut warnings = Vec::new();
        let mut errors = Vec::new();
        let mut risk_score = Decimal::from(0);

        // Amount validation
        if amount <= Decimal::ZERO {
            errors.push("Amount must be positive".to_string());
            risk_score += Decimal::from(50);
        } else if amount > Decimal::from(1_000_000) {
            warnings.push("Large transaction amount".to_string());
            risk_score += Decimal::from(30);
        }

        // Address validation
        if req.from_address == req.to_address {
            warnings.push("Sending to same address".to_string());
            risk_score += Decimal::from(10);
        }

        // Transaction type specific validation
        match transaction_type {
            crate::models::dapp_signing::TransactionType::ContractCall => {
                if req.data.is_empty() {
                    warnings.push("Contract call without data".to_string());
                    risk_score += Decimal::from(20);
                }
            }
            crate::models::dapp_signing::TransactionType::DefiSwap => {
                if amount > Decimal::from(100_000) {
                    warnings.push("Large DeFi swap amount".to_string());
                    risk_score += Decimal::from(25);
                }
            }
            _ => {}
        }

        // Determine validation status and risk level
        let (status, risk_level) = if !errors.is_empty() {
            (crate::models::dapp_signing::ValidationStatus::Invalid, crate::models::dapp_signing::RiskLevel::Critical)
        } else if risk_score > Decimal::from(50) {
            (crate::models::dapp_signing::ValidationStatus::Warning, crate::models::dapp_signing::RiskLevel::High)
        } else if risk_score > Decimal::from(25) {
            (crate::models::dapp_signing::ValidationStatus::Valid, crate::models::dapp_signing::RiskLevel::Medium)
        } else {
            (crate::models::dapp_signing::ValidationStatus::Valid, crate::models::dapp_signing::RiskLevel::Low)
        };

        let validation_result = ValidationResult {
            status,
            risk_level,
            warnings,
            errors,
            is_whitelisted: false, // Would check against whitelist in real implementation
            is_blacklisted: false, // Would check against blacklist in real implementation
            risk_score,
            metadata: HashMap::new(),
        };

        // Store validation result
        self.dapp_signing_repository
            .create_validation_result(&validation_result)
            .await
            .map_err(|e| Status::internal(format!("Failed to store validation result: {}", e)))?;

        let response = ValidateTransactionResponse {
            result: Some(Self::model_to_proto_validation_result(&validation_result)),
        };

        Ok(Response::new(response))
    }

    /// Check transaction limits
    async fn check_transaction_limits(
        &self,
        request: Request<CheckTransactionLimitsRequest>,
    ) -> Result<Response<CheckTransactionLimitsResponse>, Status> {
        let req = request.get_ref();

        // Extract authentication
        let auth_context = self.auth_service.extract_auth(&request).await?;

        // Parse user ID
        let user_id = Uuid::parse_str(&auth_context.user_id)
            .map_err(|_| Status::invalid_argument("Invalid user ID"))?;

        // Convert types
        let key_type = Self::proto_to_model_key_type(req.key_type)?;
        let transaction_type = Self::proto_to_model_transaction_type(req.transaction_type)?;

        // Parse amount
        let amount = req.amount.parse::<Decimal>()
            .map_err(|_| Status::invalid_argument("Invalid amount format"))?;

        // Set default time window if not provided (24 hours)
        let time_window_hours = if req.time_window_hours > 0 {
            req.time_window_hours
        } else {
            24
        };

        // Check transaction limits
        let (within_limits, daily_limit, daily_used, daily_remaining) =
            self.dapp_signing_repository.check_transaction_limits(
                &user_id,
                &amount,
                key_type,
                &req.chain_id,
                transaction_type,
                time_window_hours,
            ).await
            .map_err(|e| Status::internal(format!("Failed to check transaction limits: {}", e)))?;

        // Check for violations
        let mut violations = Vec::new();
        if !within_limits {
            if daily_remaining < amount {
                violations.push(format!("Daily limit exceeded. Remaining: {}", daily_remaining));
            }
        }

        let response = CheckTransactionLimitsResponse {
            within_limits,
            daily_limit: daily_limit.to_string(),
            daily_used: daily_used.to_string(),
            daily_remaining: daily_remaining.to_string(),
            transaction_limit: "50000".to_string(), // Mock transaction limit
            violations,
        };

        Ok(Response::new(response))
    }

    /// Get signing history
    async fn get_signing_history(
        &self,
        request: Request<GetSigningHistoryRequest>,
    ) -> Result<Response<GetSigningHistoryResponse>, Status> {
        let req = request.get_ref();

        // Extract authentication and validate analytics access
        let auth_context = self.dapp_guard
            .validate_analytics_access(&request, None)
            .await?;

        // Parse user ID
        let user_id = if req.user_id.is_empty() {
            Some(Uuid::parse_str(&auth_context.user_id)
                .map_err(|_| Status::invalid_argument("Invalid user ID"))?)
        } else {
            let target_user_id = Uuid::parse_str(&req.user_id)
                .map_err(|_| Status::invalid_argument("Invalid target user ID"))?;

            if target_user_id != Uuid::parse_str(&auth_context.user_id).unwrap() &&
               !self.auth_service.has_permission(&auth_context, Permission::ViewAnalytics).await? {
                return Err(Status::permission_denied("Can only view your own signing history"));
            }

            Some(target_user_id)
        };

        // Parse optional filters
        let session_id = if req.session_id.is_empty() {
            None
        } else {
            Some(Uuid::parse_str(&req.session_id)
                .map_err(|_| Status::invalid_argument("Invalid session ID"))?)
        };

        let key_type = if req.key_type != 0 {
            Some(Self::proto_to_model_key_type(req.key_type)?)
        } else {
            None
        };

        let transaction_type = if req.transaction_type != 0 {
            Some(Self::proto_to_model_transaction_type(req.transaction_type)?)
        } else {
            None
        };

        let start_date = if req.start_date > 0 {
            Some(DateTime::from_timestamp(req.start_date, 0)
                .ok_or_else(|| Status::invalid_argument("Invalid start_date timestamp"))?)
        } else {
            None
        };

        let end_date = if req.end_date > 0 {
            Some(DateTime::from_timestamp(req.end_date, 0)
                .ok_or_else(|| Status::invalid_argument("Invalid end_date timestamp"))?)
        } else {
            None
        };

        let dapp_url = if req.dapp_url.is_empty() {
            None
        } else {
            Some(req.dapp_url.clone())
        };

        // Set pagination defaults
        let page_size = if req.page_size > 0 && req.page_size <= 100 {
            req.page_size
        } else {
            20
        };

        let page = if req.page_token.is_empty() {
            1
        } else {
            req.page_token.parse::<i32>()
                .map_err(|_| Status::invalid_argument("Invalid page token"))?
        };

        // Get signing history from repository
        let (entries, total_count) = self.dapp_signing_repository
            .get_signing_history(
                user_id,
                session_id,
                dapp_url,
                key_type,
                transaction_type,
                start_date,
                end_date,
                page,
                page_size,
            )
            .await
            .map_err(|e| Status::internal(format!("Failed to get signing history: {}", e)))?;

        // Convert to proto
        let proto_entries: Vec<SigningHistoryEntry> = entries.iter()
            .map(|entry| SigningHistoryEntry {
                entry_id: entry.entry_id.to_string(),
                user_id: entry.user_id.to_string(),
                session_id: entry.session_id.to_string(),
                dapp_url: entry.dapp_url.clone(),
                signature_type: Self::model_to_proto_signature_type(entry.signature_type) as i32,
                transaction_type: entry.transaction_type.map(|tt| Self::model_to_proto_transaction_type(tt) as i32).unwrap_or(0),
                key_type: Self::model_to_proto_key_type(entry.key_type) as i32,
                chain_id: entry.chain_id.clone(),
                address: entry.address.clone(),
                amount: entry.amount.map(|a| a.to_string()).unwrap_or_default(),
                recipient: entry.recipient.clone().unwrap_or_default(),
                contract_address: entry.contract_address.clone().unwrap_or_default(),
                success: entry.success,
                error_message: entry.error_message.clone().unwrap_or_default(),
                risk_level: Self::model_to_proto_risk_level(entry.risk_level) as i32,
                signed_at: entry.signed_at.timestamp(),
                metadata: entry.metadata.clone(),
            })
            .collect();

        // Generate next page token
        let next_page_token = if (page * page_size) < total_count as i32 {
            (page + 1).to_string()
        } else {
            String::new()
        };

        let response = GetSigningHistoryResponse {
            entries: proto_entries,
            next_page_token,
            total_count: total_count as i32,
        };

        Ok(Response::new(response))
    }

    /// Get signing analytics
    async fn get_signing_analytics(
        &self,
        request: Request<GetSigningAnalyticsRequest>,
    ) -> Result<Response<GetSigningAnalyticsResponse>, Status> {
        let req = request.get_ref();

        // Parse user ID and validate analytics access
        let target_user_id = if req.user_id.is_empty() {
            None
        } else {
            Some(Uuid::parse_str(&req.user_id)
                .map_err(|_| Status::invalid_argument("Invalid user ID"))?)
        };

        let _auth_context = self.dapp_guard
            .validate_analytics_access(&request, target_user_id)
            .await?;

        // Parse date range
        let start_date = if req.start_date > 0 {
            Some(DateTime::from_timestamp(req.start_date, 0)
                .ok_or_else(|| Status::invalid_argument("Invalid start_date timestamp"))?)
        } else {
            None
        };

        let end_date = if req.end_date > 0 {
            Some(DateTime::from_timestamp(req.end_date, 0)
                .ok_or_else(|| Status::invalid_argument("Invalid end_date timestamp"))?)
        } else {
            None
        };

        // Get analytics from repository
        let analytics = self.dapp_signing_repository
            .get_signing_analytics(target_user_id, start_date, end_date)
            .await
            .map_err(|e| Status::internal(format!("Failed to get signing analytics: {}", e)))?;

        // Convert to proto
        let proto_analytics = SigningAnalytics {
            user_id: analytics.user_id.to_string(),
            total_signatures: analytics.total_signatures,
            successful_signatures: analytics.successful_signatures,
            failed_signatures: analytics.failed_signatures,
            total_transactions: analytics.total_transactions,
            successful_transactions: analytics.successful_transactions,
            total_value_signed: analytics.total_value_signed.to_string(),
            most_used_chains: analytics.most_used_chains.iter()
                .map(|&kt| Self::model_to_proto_key_type(kt) as i32)
                .collect(),
            most_used_types: analytics.most_used_types.iter()
                .map(|&tt| Self::model_to_proto_transaction_type(tt) as i32)
                .collect(),
            top_dapps: analytics.top_dapps,
            signature_type_counts: analytics.signature_type_counts,
            average_transaction_value: analytics.average_transaction_value,
            average_risk_level: Self::model_to_proto_risk_level(analytics.average_risk_level) as i32,
            last_activity_at: analytics.last_activity_at.timestamp(),
        };

        let response = GetSigningAnalyticsResponse {
            analytics: Some(proto_analytics),
        };

        Ok(Response::new(response))
    }

    /// Flag suspicious activity
    async fn flag_suspicious_activity(
        &self,
        request: Request<FlagSuspiciousActivityRequest>,
    ) -> Result<Response<FlagSuspiciousActivityResponse>, Status> {
        let req = request.get_ref();

        // Parse user ID
        let user_id = Uuid::parse_str(&req.user_id)
            .map_err(|_| Status::invalid_argument("Invalid user ID"))?;

        // Parse optional session ID
        let session_id = if req.session_id.is_empty() {
            None
        } else {
            Some(Uuid::parse_str(&req.session_id)
                .map_err(|_| Status::invalid_argument("Invalid session ID"))?)
        };

        // Validate administrative access
        let _auth_context = self.dapp_guard
            .validate_administrative_access(&request, "flag_suspicious_activity")
            .await?;

        // Flag the activity
        let investigation_id = self.dapp_signing_repository
            .flag_suspicious_activity(&user_id, session_id, &req.reason, &req.evidence)
            .await
            .map_err(|e| Status::internal(format!("Failed to flag suspicious activity: {}", e)))?;

        // Send notification to user if auto_suspend is enabled
        if req.auto_suspend {
            self.send_notification(
                &user_id,
                NotificationType::SecurityAlert,
                "Account Suspended",
                &format!("Your account has been suspended due to suspicious activity: {}", req.reason),
                Some([
                    ("investigation_id".to_string(), investigation_id.clone()),
                    ("reason".to_string(), req.reason.clone()),
                ].into()),
            ).await;
        }

        let response = FlagSuspiciousActivityResponse {
            success: true,
            investigation_id,
        };

        Ok(Response::new(response))
    }
}
