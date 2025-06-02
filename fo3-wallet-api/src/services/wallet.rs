//! Wallet service implementation

use std::sync::Arc;
use tonic::{Request, Response, Status};

use fo3_wallet::{
    account::Wallet,
    crypto::keys::{KeyType as WalletKeyType, bitcoin::Network as BitcoinNetwork},
};

use crate::proto::fo3::wallet::v1::{
    wallet_service_server::WalletService,
    *,
};
use crate::state::AppState;
use crate::error::{wallet_error_to_status, string_error_to_status, not_found_error};

pub struct WalletServiceImpl {
    state: Arc<AppState>,
}

impl WalletServiceImpl {
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }
}

#[tonic::async_trait]
impl WalletService for WalletServiceImpl {
    async fn create_wallet(
        &self,
        request: Request<CreateWalletRequest>,
    ) -> Result<Response<CreateWalletResponse>, Status> {
        let req = request.into_inner();
        
        let (wallet, mnemonic) = Wallet::new(req.name)
            .map_err(wallet_error_to_status)?;

        self.state.add_wallet(wallet.clone())
            .map_err(string_error_to_status)?;

        let response = CreateWalletResponse {
            wallet: Some(wallet_to_proto(&wallet)),
            mnemonic,
        };

        Ok(Response::new(response))
    }

    async fn import_wallet(
        &self,
        request: Request<ImportWalletRequest>,
    ) -> Result<Response<ImportWalletResponse>, Status> {
        let req = request.into_inner();
        
        let wallet = Wallet::from_mnemonic(req.name, &req.mnemonic)
            .map_err(wallet_error_to_status)?;

        self.state.add_wallet(wallet.clone())
            .map_err(string_error_to_status)?;

        let response = ImportWalletResponse {
            wallet: Some(wallet_to_proto(&wallet)),
        };

        Ok(Response::new(response))
    }

    async fn get_wallet(
        &self,
        request: Request<GetWalletRequest>,
    ) -> Result<Response<GetWalletResponse>, Status> {
        let req = request.into_inner();
        
        let wallet = self.state.get_wallet(&req.wallet_id)
            .ok_or_else(|| not_found_error(&format!("Wallet not found: {}", req.wallet_id)))?;

        let response = GetWalletResponse {
            wallet: Some(wallet_to_proto(&wallet)),
        };

        Ok(Response::new(response))
    }

    async fn list_wallets(
        &self,
        _request: Request<ListWalletsRequest>,
    ) -> Result<Response<ListWalletsResponse>, Status> {
        let wallets = self.state.get_all_wallets();
        
        let response = ListWalletsResponse {
            wallets: wallets.into_iter().map(|w| wallet_to_proto(&w)).collect(),
            next_page_token: String::new(), // Simple implementation without pagination
        };

        Ok(Response::new(response))
    }

    async fn delete_wallet(
        &self,
        request: Request<DeleteWalletRequest>,
    ) -> Result<Response<DeleteWalletResponse>, Status> {
        let req = request.into_inner();
        
        let success = self.state.remove_wallet(&req.wallet_id);

        let response = DeleteWalletResponse { success };

        Ok(Response::new(response))
    }

    async fn derive_address(
        &self,
        request: Request<DeriveAddressRequest>,
    ) -> Result<Response<DeriveAddressResponse>, Status> {
        let req = request.into_inner();
        
        let wallet = self.state.get_wallet(&req.wallet_id)
            .ok_or_else(|| not_found_error(&format!("Wallet not found: {}", req.wallet_id)))?;

        let key_type = proto_to_wallet_key_type(req.key_type());
        let address_str = match key_type {
            WalletKeyType::Ethereum => wallet.get_ethereum_address(&req.derivation_path, None),
            WalletKeyType::Solana => wallet.get_solana_address(&req.derivation_path, None),
            WalletKeyType::Bitcoin => {
                let network = proto_to_bitcoin_network(req.bitcoin_network());
                wallet.get_bitcoin_address(&req.derivation_path, network, None)
            },
        }.map_err(wallet_error_to_status)?;

        let address = Address {
            address: address_str,
            key_type: req.key_type,
            derivation_path: req.derivation_path,
            bitcoin_network: req.bitcoin_network,
        };

        let response = DeriveAddressResponse {
            address: Some(address),
        };

        Ok(Response::new(response))
    }

    async fn get_addresses(
        &self,
        request: Request<GetAddressesRequest>,
    ) -> Result<Response<GetAddressesResponse>, Status> {
        let req = request.into_inner();
        
        let wallet = self.state.get_wallet(&req.wallet_id)
            .ok_or_else(|| not_found_error(&format!("Wallet not found: {}", req.wallet_id)))?;

        // For now, return empty addresses as we don't store them
        // In a real implementation, you'd store and retrieve addresses
        let response = GetAddressesResponse {
            addresses: vec![],
        };

        Ok(Response::new(response))
    }
}

// Helper functions to convert between proto and wallet types
fn wallet_to_proto(wallet: &Wallet) -> crate::proto::fo3::wallet::v1::Wallet {
    crate::proto::fo3::wallet::v1::Wallet {
        id: wallet.id().to_string(),
        name: wallet.name().to_string(),
        created_at: wallet.created_at().timestamp(),
        addresses: vec![], // Would be populated from storage in real implementation
    }
}

fn proto_to_wallet_key_type(key_type: KeyType) -> WalletKeyType {
    match key_type {
        KeyType::KeyTypeEthereum => WalletKeyType::Ethereum,
        KeyType::KeyTypeBitcoin => WalletKeyType::Bitcoin,
        KeyType::KeyTypeSolana => WalletKeyType::Solana,
        _ => WalletKeyType::Ethereum, // Default
    }
}

fn proto_to_bitcoin_network(network: BitcoinNetwork) -> fo3_wallet::crypto::keys::bitcoin::Network {
    match network {
        BitcoinNetwork::BitcoinNetworkMainnet => fo3_wallet::crypto::keys::bitcoin::Network::Bitcoin,
        BitcoinNetwork::BitcoinNetworkTestnet => fo3_wallet::crypto::keys::bitcoin::Network::Testnet,
        BitcoinNetwork::BitcoinNetworkRegtest => fo3_wallet::crypto::keys::bitcoin::Network::Regtest,
        _ => fo3_wallet::crypto::keys::bitcoin::Network::Bitcoin, // Default
    }
}
