//! Transaction service implementation

use std::sync::Arc;
use tonic::{Request, Response, Status};

use fo3_wallet::{
    crypto::keys::KeyType as WalletKeyType,
    transaction::{TransactionRequest, provider::ProviderFactory},
};

use crate::proto::fo3::wallet::v1::{
    transaction_service_server::TransactionService,
    *,
};
use crate::state::AppState;
use crate::error::{wallet_error_to_status, invalid_argument_error};

pub struct TransactionServiceImpl {
    state: Arc<AppState>,
}

impl TransactionServiceImpl {
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }
}

#[tonic::async_trait]
impl TransactionService for TransactionServiceImpl {
    async fn send_transaction(
        &self,
        request: Request<SendTransactionRequest>,
    ) -> Result<Response<SendTransactionResponse>, Status> {
        let req = request.into_inner();
        
        let key_type = proto_to_wallet_key_type(req.key_type());
        let provider_config = match key_type {
            WalletKeyType::Ethereum => self.state.provider_config.clone(),
            WalletKeyType::Bitcoin => self.state.get_bitcoin_config(),
            #[cfg(feature = "solana")]
            WalletKeyType::Solana => self.state.get_solana_config(),
            #[cfg(not(feature = "solana"))]
            WalletKeyType::Solana => return Err(invalid_argument_error("Solana support not enabled")),
        };

        let provider = ProviderFactory::create_provider(key_type, provider_config)
            .map_err(wallet_error_to_status)?;

        let tx_request = TransactionRequest {
            key_type,
            from: req.from_address,
            to: req.to_address,
            value: req.value,
            gas_price: req.gas_price,
            gas_limit: req.gas_limit,
            nonce: req.nonce,
            data: Some(serde_json::json!({
                "private_key": req.private_key,
                "data": req.data
            })),
        };

        let hash = provider.send_transaction(&tx_request)
            .map_err(wallet_error_to_status)?;

        let status = provider.get_transaction_status(&hash)
            .map_err(wallet_error_to_status)?;

        let response = SendTransactionResponse {
            transaction_hash: hash,
            status: wallet_status_to_proto(status) as i32,
        };

        Ok(Response::new(response))
    }

    async fn get_transaction(
        &self,
        request: Request<GetTransactionRequest>,
    ) -> Result<Response<GetTransactionResponse>, Status> {
        let req = request.into_inner();
        
        let key_type = proto_to_wallet_key_type(req.key_type());
        let provider_config = match key_type {
            WalletKeyType::Ethereum => self.state.provider_config.clone(),
            WalletKeyType::Bitcoin => self.state.get_bitcoin_config(),
            #[cfg(feature = "solana")]
            WalletKeyType::Solana => self.state.get_solana_config(),
            #[cfg(not(feature = "solana"))]
            WalletKeyType::Solana => return Err(invalid_argument_error("Solana support not enabled")),
        };

        let provider = ProviderFactory::create_provider(key_type, provider_config)
            .map_err(wallet_error_to_status)?;

        let transaction = provider.get_transaction(&req.transaction_hash)
            .map_err(wallet_error_to_status)?;

        // Convert transaction to proto format
        let proto_transaction = Transaction {
            hash: req.transaction_hash,
            key_type: req.key_type,
            from_address: String::new(), // Would be filled from transaction data
            to_address: String::new(),   // Would be filled from transaction data
            value: String::new(),        // Would be filled from transaction data
            gas_price: String::new(),
            gas_limit: String::new(),
            nonce: String::new(),
            data: vec![],
            status: TransactionStatus::TransactionStatusConfirmed as i32, // Default
            timestamp: 0,
            block_hash: String::new(),
            block_number: 0,
            transaction_index: 0,
            gas_used: String::new(),
            fee: String::new(),
        };

        let response = GetTransactionResponse {
            transaction: Some(proto_transaction),
        };

        Ok(Response::new(response))
    }

    async fn sign_transaction(
        &self,
        request: Request<SignTransactionRequest>,
    ) -> Result<Response<SignTransactionResponse>, Status> {
        let req = request.into_inner();
        
        let key_type = proto_to_wallet_key_type(req.key_type());
        let provider_config = match key_type {
            WalletKeyType::Ethereum => self.state.provider_config.clone(),
            WalletKeyType::Bitcoin => self.state.get_bitcoin_config(),
            #[cfg(feature = "solana")]
            WalletKeyType::Solana => self.state.get_solana_config(),
            #[cfg(not(feature = "solana"))]
            WalletKeyType::Solana => return Err(invalid_argument_error("Solana support not enabled")),
        };

        let provider = ProviderFactory::create_provider(key_type, provider_config)
            .map_err(wallet_error_to_status)?;

        let tx_request = TransactionRequest {
            key_type,
            from: req.from_address,
            to: req.to_address,
            value: req.value,
            gas_price: req.gas_price,
            gas_limit: req.gas_limit,
            nonce: req.nonce,
            data: Some(serde_json::json!({
                "private_key": req.private_key,
                "data": req.data
            })),
        };

        let signed_tx = provider.sign_transaction(&tx_request)
            .map_err(wallet_error_to_status)?;

        let response = SignTransactionResponse {
            signed_transaction: signed_tx,
        };

        Ok(Response::new(response))
    }

    async fn broadcast_transaction(
        &self,
        request: Request<BroadcastTransactionRequest>,
    ) -> Result<Response<BroadcastTransactionResponse>, Status> {
        let req = request.into_inner();
        
        let key_type = proto_to_wallet_key_type(req.key_type());
        let provider_config = match key_type {
            WalletKeyType::Ethereum => self.state.provider_config.clone(),
            WalletKeyType::Bitcoin => self.state.get_bitcoin_config(),
            #[cfg(feature = "solana")]
            WalletKeyType::Solana => self.state.get_solana_config(),
            #[cfg(not(feature = "solana"))]
            WalletKeyType::Solana => return Err(invalid_argument_error("Solana support not enabled")),
        };

        let provider = ProviderFactory::create_provider(key_type, provider_config)
            .map_err(wallet_error_to_status)?;

        let hash = provider.broadcast_transaction(&req.signed_transaction)
            .map_err(wallet_error_to_status)?;

        let status = provider.get_transaction_status(&hash)
            .map_err(wallet_error_to_status)?;

        let response = BroadcastTransactionResponse {
            transaction_hash: hash,
            status: wallet_status_to_proto(status) as i32,
        };

        Ok(Response::new(response))
    }

    async fn get_transaction_history(
        &self,
        _request: Request<GetTransactionHistoryRequest>,
    ) -> Result<Response<GetTransactionHistoryResponse>, Status> {
        // Simple implementation - would query blockchain in real implementation
        let response = GetTransactionHistoryResponse {
            transactions: vec![],
            next_page_token: String::new(),
        };

        Ok(Response::new(response))
    }
}

// Helper functions
fn proto_to_wallet_key_type(key_type: KeyType) -> WalletKeyType {
    match key_type {
        KeyType::KeyTypeEthereum => WalletKeyType::Ethereum,
        KeyType::KeyTypeBitcoin => WalletKeyType::Bitcoin,
        KeyType::KeyTypeSolana => WalletKeyType::Solana,
        _ => WalletKeyType::Ethereum, // Default
    }
}

fn wallet_status_to_proto(status: fo3_wallet::transaction::TransactionStatus) -> TransactionStatus {
    match status {
        fo3_wallet::transaction::TransactionStatus::Pending => TransactionStatus::TransactionStatusPending,
        fo3_wallet::transaction::TransactionStatus::Confirmed => TransactionStatus::TransactionStatusConfirmed,
        fo3_wallet::transaction::TransactionStatus::Failed => TransactionStatus::TransactionStatusFailed,
    }
}
