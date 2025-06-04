//! gRPC client for CLI operations
//! 
//! Provides client connections to FO3 Wallet Core gRPC services

use tonic::transport::Channel;
use tonic::metadata::MetadataValue;
use tonic::{Request, Status};
use uuid::Uuid;
use tracing::{info, error};

// Import the generated gRPC client stubs
use fo3_wallet_api::proto::fo3::wallet::v1::{
    wallet_service_client::WalletServiceClient,
    kyc_service_client::KycServiceClient,
    card_service_client::CardServiceClient,
    fiat_gateway_service_client::FiatGatewayServiceClient,
    auth_service_client::AuthServiceClient,
    // Request/Response types
    CreateWalletRequest, CreateWalletResponse,
    ListWalletsRequest, ListWalletsResponse,
    GetWalletRequest, GetWalletResponse,
    GenerateAddressRequest, GenerateAddressResponse,
    GetBalanceRequest, GetBalanceResponse,
    SubmitKycRequest, SubmitKycResponse,
    ListKycSubmissionsRequest, ListKycSubmissionsResponse,
    GetKycStatusRequest, GetKycStatusResponse,
    ApproveKycRequest, ApproveKycResponse,
    RejectKycRequest, RejectKycResponse,
    CreateCardRequest, CreateCardResponse,
    ListCardsRequest, ListCardsResponse,
    GetCardRequest, GetCardResponse,
    ProcessCardTransactionRequest, ProcessCardTransactionResponse,
    FreezeCardRequest, FreezeCardResponse,
    LoginRequest, LoginResponse,
};

/// gRPC client configuration
#[derive(Debug, Clone)]
pub struct ClientConfig {
    pub server_url: String,
    pub auth_token: Option<String>,
    pub timeout_seconds: u64,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            server_url: "http://127.0.0.1:50051".to_string(),
            auth_token: None,
            timeout_seconds: 30,
        }
    }
}

impl ClientConfig {
    pub fn from_env() -> Self {
        Self {
            server_url: std::env::var("GRPC_SERVER_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:50051".to_string()),
            auth_token: std::env::var("AUTH_TOKEN").ok(),
            timeout_seconds: std::env::var("GRPC_TIMEOUT")
                .unwrap_or_else(|_| "30".to_string())
                .parse()
                .unwrap_or(30),
        }
    }
}

/// FO3 Wallet gRPC client
pub struct FO3Client {
    config: ClientConfig,
    channel: Channel,
}

impl FO3Client {
    /// Create a new client instance
    pub async fn new(config: ClientConfig) -> Result<Self, Box<dyn std::error::Error>> {
        info!("Connecting to FO3 Wallet Core at: {}", config.server_url);
        
        let channel = Channel::from_shared(config.server_url.clone())?
            .connect()
            .await?;

        Ok(Self { config, channel })
    }

    /// Add authentication headers to request
    fn add_auth_headers<T>(&self, mut request: Request<T>) -> Request<T> {
        if let Some(ref token) = self.config.auth_token {
            let auth_header = format!("Bearer {}", token);
            if let Ok(auth_value) = MetadataValue::try_from(auth_header) {
                request.metadata_mut().insert("authorization", auth_value);
            }
        }
        request
    }

    /// Create a new wallet
    pub async fn create_wallet(&self, name: String) -> Result<CreateWalletResponse, Status> {
        let mut client = WalletServiceClient::new(self.channel.clone());
        
        let request = self.add_auth_headers(Request::new(CreateWalletRequest {
            name,
            mnemonic: None, // Let the server generate
        }));

        let response = client.create_wallet(request).await?;
        Ok(response.into_inner())
    }

    /// List all wallets
    pub async fn list_wallets(&self) -> Result<ListWalletsResponse, Status> {
        let mut client = WalletServiceClient::new(self.channel.clone());
        
        let request = self.add_auth_headers(Request::new(ListWalletsRequest {
            page_size: 50,
            page_token: None,
        }));

        let response = client.list_wallets(request).await?;
        Ok(response.into_inner())
    }

    /// Get wallet details
    pub async fn get_wallet(&self, wallet_id: String) -> Result<GetWalletResponse, Status> {
        let mut client = WalletServiceClient::new(self.channel.clone());
        
        let request = self.add_auth_headers(Request::new(GetWalletRequest {
            wallet_id,
        }));

        let response = client.get_wallet(request).await?;
        Ok(response.into_inner())
    }

    /// Generate address for wallet
    pub async fn generate_address(&self, wallet_id: String, key_type: String) -> Result<GenerateAddressResponse, Status> {
        let mut client = WalletServiceClient::new(self.channel.clone());
        
        let request = self.add_auth_headers(Request::new(GenerateAddressRequest {
            wallet_id,
            key_type,
            derivation_path: None, // Use default
        }));

        let response = client.generate_address(request).await?;
        Ok(response.into_inner())
    }

    /// Get wallet balance
    pub async fn get_balance(&self, wallet_id: String) -> Result<GetBalanceResponse, Status> {
        let mut client = WalletServiceClient::new(self.channel.clone());
        
        let request = self.add_auth_headers(Request::new(GetBalanceRequest {
            wallet_id,
            key_type: None, // Get all balances
        }));

        let response = client.get_balance(request).await?;
        Ok(response.into_inner())
    }

    /// Submit KYC
    pub async fn submit_kyc(&self, user_id: String) -> Result<SubmitKycResponse, Status> {
        let mut client = KycServiceClient::new(self.channel.clone());
        
        // Create mock personal info for testing
        let request = self.add_auth_headers(Request::new(SubmitKycRequest {
            wallet_id: user_id,
            first_name: "Test".to_string(),
            last_name: "User".to_string(),
            date_of_birth: "1990-01-01".to_string(),
            nationality: "US".to_string(),
            country_of_residence: "US".to_string(),
            street_address: "123 Test St".to_string(),
            city: "Test City".to_string(),
            state_province: Some("CA".to_string()),
            postal_code: "12345".to_string(),
            address_country: "US".to_string(),
        }));

        let response = client.submit_kyc(request).await?;
        Ok(response.into_inner())
    }

    /// List KYC submissions
    pub async fn list_kyc_submissions(&self) -> Result<ListKycSubmissionsResponse, Status> {
        let mut client = KycServiceClient::new(self.channel.clone());
        
        let request = self.add_auth_headers(Request::new(ListKycSubmissionsRequest {
            page_size: 50,
            page_token: None,
            status_filter: None,
            wallet_id_filter: None,
        }));

        let response = client.list_kyc_submissions(request).await?;
        Ok(response.into_inner())
    }

    /// Get KYC status
    pub async fn get_kyc_status(&self, submission_id: String) -> Result<GetKycStatusResponse, Status> {
        let mut client = KycServiceClient::new(self.channel.clone());
        
        let request = self.add_auth_headers(Request::new(GetKycStatusRequest {
            submission_id,
        }));

        let response = client.get_kyc_status(request).await?;
        Ok(response.into_inner())
    }

    /// Approve KYC
    pub async fn approve_kyc(&self, submission_id: String) -> Result<ApproveKycResponse, Status> {
        let mut client = KycServiceClient::new(self.channel.clone());
        
        let request = self.add_auth_headers(Request::new(ApproveKycRequest {
            submission_id,
            reviewer_notes: Some("Approved via CLI".to_string()),
        }));

        let response = client.approve_kyc(request).await?;
        Ok(response.into_inner())
    }

    /// Reject KYC
    pub async fn reject_kyc(&self, submission_id: String, reason: String) -> Result<RejectKycResponse, Status> {
        let mut client = KycServiceClient::new(self.channel.clone());
        
        let request = self.add_auth_headers(Request::new(RejectKycRequest {
            submission_id,
            reason,
            reviewer_notes: Some("Rejected via CLI".to_string()),
        }));

        let response = client.reject_kyc(request).await?;
        Ok(response.into_inner())
    }

    /// Create a card
    pub async fn create_card(&self, user_id: String, currency: String) -> Result<CreateCardResponse, Status> {
        let mut client = CardServiceClient::new(self.channel.clone());
        
        let request = self.add_auth_headers(Request::new(CreateCardRequest {
            user_id,
            card_type: "virtual".to_string(),
            currency,
            daily_limit: Some("5000.00".to_string()),
            monthly_limit: Some("50000.00".to_string()),
            design_id: Some("default".to_string()),
        }));

        let response = client.create_card(request).await?;
        Ok(response.into_inner())
    }

    /// List cards for user
    pub async fn list_cards(&self, user_id: String) -> Result<ListCardsResponse, Status> {
        let mut client = CardServiceClient::new(self.channel.clone());
        
        let request = self.add_auth_headers(Request::new(ListCardsRequest {
            user_id,
            page_size: 50,
            page_token: None,
            status_filter: None,
        }));

        let response = client.list_cards(request).await?;
        Ok(response.into_inner())
    }

    /// Get card details
    pub async fn get_card(&self, card_id: String) -> Result<GetCardResponse, Status> {
        let mut client = CardServiceClient::new(self.channel.clone());
        
        let request = self.add_auth_headers(Request::new(GetCardRequest {
            card_id,
        }));

        let response = client.get_card(request).await?;
        Ok(response.into_inner())
    }

    /// Process card transaction
    pub async fn process_card_transaction(&self, card_id: String, amount: String, merchant: String) -> Result<ProcessCardTransactionResponse, Status> {
        let mut client = CardServiceClient::new(self.channel.clone());
        
        let request = self.add_auth_headers(Request::new(ProcessCardTransactionRequest {
            card_id,
            amount,
            currency: "USD".to_string(),
            merchant_name: merchant,
            merchant_category: Some("test".to_string()),
            description: Some("CLI test transaction".to_string()),
        }));

        let response = client.process_card_transaction(request).await?;
        Ok(response.into_inner())
    }

    /// Freeze/unfreeze card
    pub async fn freeze_card(&self, card_id: String, freeze: bool) -> Result<FreezeCardResponse, Status> {
        let mut client = CardServiceClient::new(self.channel.clone());

        let request = self.add_auth_headers(Request::new(FreezeCardRequest {
            card_id,
            freeze,
            reason: if freeze { Some("CLI freeze".to_string()) } else { None },
        }));

        let response = client.freeze_card(request).await?;
        Ok(response.into_inner())
    }

    /// Create trading strategy
    pub async fn create_trading_strategy(&self, name: String, strategy_type: String) -> Result<String, Status> {
        info!("Creating trading strategy: {} (type: {})", name, strategy_type);
        // Mock implementation for now - would connect to actual trading service
        Ok(format!("strategy_{}", uuid::Uuid::new_v4()))
    }

    /// List trading strategies
    pub async fn list_trading_strategies(&self) -> Result<Vec<String>, Status> {
        info!("Listing trading strategies");
        // Mock implementation - would connect to actual trading service
        Ok(vec![
            "momentum_strategy_1".to_string(),
            "arbitrage_strategy_2".to_string(),
            "dca_strategy_3".to_string(),
        ])
    }

    /// Execute trade
    pub async fn execute_trade(&self, strategy_id: String, symbol: String, amount: String) -> Result<String, Status> {
        info!("Executing trade: {} {} with strategy {}", amount, symbol, strategy_id);
        // Mock implementation - would connect to actual trading service
        Ok(format!("trade_{}", uuid::Uuid::new_v4()))
    }

    /// Get DeFi positions
    pub async fn get_defi_positions(&self, wallet_id: String) -> Result<Vec<String>, Status> {
        info!("Getting DeFi positions for wallet: {}", wallet_id);
        // Mock implementation - would connect to actual DeFi service
        Ok(vec![
            "uniswap_v3_position_1".to_string(),
            "aave_lending_position_2".to_string(),
            "compound_staking_position_3".to_string(),
        ])
    }

    /// Stake tokens
    pub async fn stake_tokens(&self, wallet_id: String, protocol: String, amount: String, token: String) -> Result<String, Status> {
        info!("Staking {} {} on {} for wallet {}", amount, token, protocol, wallet_id);
        // Mock implementation - would connect to actual DeFi service
        Ok(format!("stake_{}", uuid::Uuid::new_v4()))
    }

    /// Connect DApp
    pub async fn connect_dapp(&self, wallet_id: String, dapp_url: String) -> Result<String, Status> {
        info!("Connecting wallet {} to DApp: {}", wallet_id, dapp_url);
        // Mock implementation - would connect to actual DApp service
        Ok(format!("session_{}", uuid::Uuid::new_v4()))
    }

    /// Sign DApp transaction
    pub async fn sign_dapp_transaction(&self, session_id: String, transaction_data: String) -> Result<String, Status> {
        info!("Signing DApp transaction for session: {}", session_id);
        // Mock implementation - would connect to actual DApp signing service
        Ok(format!("signature_{}", uuid::Uuid::new_v4()))
    }

    /// Create bank account
    pub async fn create_bank_account(&self, user_id: String, account_type: String, currency: String) -> Result<String, Status> {
        info!("Creating {} bank account for user {} (currency: {})", account_type, user_id, currency);
        // Mock implementation - would connect to actual fiat gateway service
        Ok(format!("account_{}", uuid::Uuid::new_v4()))
    }

    /// Process fiat deposit
    pub async fn process_fiat_deposit(&self, account_id: String, amount: String, currency: String) -> Result<String, Status> {
        info!("Processing fiat deposit: {} {} to account {}", amount, currency, account_id);
        // Mock implementation - would connect to actual fiat gateway service
        Ok(format!("deposit_{}", uuid::Uuid::new_v4()))
    }

    /// Process fiat withdrawal
    pub async fn process_fiat_withdrawal(&self, account_id: String, amount: String, currency: String) -> Result<String, Status> {
        info!("Processing fiat withdrawal: {} {} from account {}", amount, currency, account_id);
        // Mock implementation - would connect to actual fiat gateway service
        Ok(format!("withdrawal_{}", uuid::Uuid::new_v4()))
    }

    /// Get price data
    pub async fn get_price(&self, symbol: String) -> Result<String, Status> {
        info!("Getting price for symbol: {}", symbol);
        // Mock implementation - would connect to actual pricing service
        match symbol.as_str() {
            "BTC" => Ok("45000.00".to_string()),
            "ETH" => Ok("3000.00".to_string()),
            "SOL" => Ok("100.00".to_string()),
            _ => Ok("1.00".to_string()),
        }
    }

    /// Send notification
    pub async fn send_notification(&self, user_id: String, message: String, notification_type: String) -> Result<String, Status> {
        info!("Sending {} notification to user {}: {}", notification_type, user_id, message);
        // Mock implementation - would connect to actual notification service
        Ok(format!("notification_{}", uuid::Uuid::new_v4()))
    }
}
