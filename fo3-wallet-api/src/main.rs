//! FO3 Wallet API
//!
//! This is the REST API server for the FO3 multi-chain wallet and DeFi SDK.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::{
    routing::{get, post},
    Router,
    extract::{Extension, Json, Path},
    http::StatusCode,
};
use serde::{Serialize, Deserialize};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use fo3_wallet::{
    account::Wallet,
    crypto::keys::KeyType,
    transaction::{TransactionRequest, TransactionStatus, provider::{ProviderConfig, ProviderType, ProviderFactory}},
    defi::{Token, SwapRequest, LendingRequest, StakingRequest},
    error::{Error as WalletError},
};

// Application state
struct AppState {
    // In a real application, this would be a database
    wallets: std::sync::RwLock<std::collections::HashMap<String, Wallet>>,
    // Provider configuration
    provider_config: ProviderConfig,
}

impl AppState {
    fn new() -> Self {
        let provider_config = ProviderConfig {
            provider_type: ProviderType::Http,
            url: "https://mainnet.infura.io/v3/your-api-key".to_string(),
            api_key: None,
            timeout: Some(30),
        };

        Self {
            wallets: std::sync::RwLock::new(std::collections::HashMap::new()),
            provider_config,
        }
    }

    fn add_wallet(&self, wallet: Wallet) -> std::result::Result<(), String> {
        let id = wallet.id().to_string();
        let mut wallets = self.wallets.write().unwrap();
        if wallets.contains_key(&id) {
            return Err("Wallet already exists".to_string());
        }
        wallets.insert(id, wallet);
        Ok(())
    }

    fn get_wallet(&self, id: &str) -> Option<Wallet> {
        self.wallets.read().unwrap().get(id).cloned()
    }

    fn get_all_wallets(&self) -> Vec<Wallet> {
        self.wallets.read().unwrap().values().cloned().collect()
    }
}

// API error type
#[derive(thiserror::Error, Debug)]
enum ApiError {
    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Bad request: {0}")]
    #[allow(dead_code)]
    BadRequest(String),

    #[error("Internal server error: {0}")]
    InternalServerError(String),

    #[error("Wallet error: {0}")]
    Wallet(#[from] WalletError),
}

impl axum::response::IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let (status, error_message) = match &self {
            Self::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            Self::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            Self::InternalServerError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            Self::Wallet(err) => (StatusCode::BAD_REQUEST, &err.to_string()),
        };

        let body = Json(serde_json::json!({
            "error": {
                "message": error_message,
                "code": status.as_u16()
            }
        }));

        (status, body).into_response()
    }
}

type Result<T> = std::result::Result<T, ApiError>;

// Request and response types
#[derive(Debug, Deserialize)]
struct CreateWalletRequest {
    name: String,
}

#[derive(Debug, Serialize)]
struct WalletResponse {
    wallet: Wallet,
    mnemonic: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ImportWalletRequest {
    name: String,
    mnemonic: String,
}

#[derive(Debug, Deserialize)]
struct DeriveAddressRequest {
    wallet_id: String,
    key_type: KeyType,
    path: String,
}

#[derive(Debug, Serialize)]
struct AddressResponse {
    address: String,
    key_type: KeyType,
    path: String,
}

#[derive(Debug, Serialize)]
struct TransactionResponse {
    hash: String,
    status: TransactionStatus,
}

// API handlers
async fn create_wallet(
    Extension(state): Extension<Arc<AppState>>,
    Json(request): Json<CreateWalletRequest>,
) -> Result<(StatusCode, Json<WalletResponse>)> {
    let (wallet, mnemonic) = Wallet::new(request.name)
        .map_err(ApiError::Wallet)?;

    state.add_wallet(wallet.clone())
        .map_err(|e| ApiError::InternalServerError(e))?;

    Ok((
        StatusCode::CREATED,
        Json(WalletResponse {
            wallet,
            mnemonic: Some(mnemonic),
        }),
    ))
}

async fn import_wallet(
    Extension(state): Extension<Arc<AppState>>,
    Json(request): Json<ImportWalletRequest>,
) -> Result<(StatusCode, Json<WalletResponse>)> {
    let wallet = Wallet::from_mnemonic(request.name, &request.mnemonic)
        .map_err(ApiError::Wallet)?;

    state.add_wallet(wallet.clone())
        .map_err(|e| ApiError::InternalServerError(e))?;

    Ok((
        StatusCode::CREATED,
        Json(WalletResponse {
            wallet,
            mnemonic: None,
        }),
    ))
}

async fn get_wallet(
    Extension(state): Extension<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<WalletResponse>> {
    let wallet = state.get_wallet(&id)
        .ok_or_else(|| ApiError::NotFound(format!("Wallet not found: {}", id)))?;

    Ok(Json(WalletResponse {
        wallet,
        mnemonic: None,
    }))
}

async fn get_all_wallets(
    Extension(state): Extension<Arc<AppState>>,
) -> Result<Json<Vec<Wallet>>> {
    let wallets = state.get_all_wallets();
    Ok(Json(wallets))
}

async fn derive_address(
    Extension(state): Extension<Arc<AppState>>,
    Json(request): Json<DeriveAddressRequest>,
) -> Result<Json<AddressResponse>> {
    let wallet = state.get_wallet(&request.wallet_id)
        .ok_or_else(|| ApiError::NotFound(format!("Wallet not found: {}", request.wallet_id)))?;

    let address = match request.key_type {
        KeyType::Ethereum => wallet.get_ethereum_address(&request.path, None),
        KeyType::Solana => wallet.get_solana_address(&request.path, None),
        KeyType::Bitcoin => wallet.get_bitcoin_address(&request.path, fo3_wallet::crypto::keys::bitcoin::Network::Bitcoin, None),
    }.map_err(ApiError::Wallet)?;

    Ok(Json(AddressResponse {
        address,
        key_type: request.key_type,
        path: request.path,
    }))
}

async fn send_transaction(
    Extension(state): Extension<Arc<AppState>>,
    Json(request): Json<TransactionRequest>,
) -> Result<Json<TransactionResponse>> {
    let provider = ProviderFactory::create_provider(request.key_type, state.provider_config.clone())
        .map_err(|e| ApiError::Wallet(e))?;

    let hash = provider.send_transaction(&request)
        .map_err(|e| ApiError::Wallet(e))?;

    let status = provider.get_transaction_status(&hash)
        .map_err(|e| ApiError::Wallet(e))?;

    Ok(Json(TransactionResponse {
        hash,
        status,
    }))
}

async fn get_transaction(
    Extension(state): Extension<Arc<AppState>>,
    Path((key_type, hash)): Path<(KeyType, String)>,
) -> Result<Json<serde_json::Value>> {
    let provider = ProviderFactory::create_provider(key_type, state.provider_config.clone())
        .map_err(|e| ApiError::Wallet(e))?;

    let transaction = provider.get_transaction(&hash)
        .map_err(|e| ApiError::Wallet(e))?;

    Ok(Json(serde_json::to_value(transaction).unwrap()))
}

async fn swap_tokens(
    Extension(state): Extension<Arc<AppState>>,
    Json(request): Json<SwapRequest>,
) -> Result<Json<serde_json::Value>> {
    let result = fo3_wallet::defi::swap_tokens(&request, &state.provider_config)
        .map_err(|e| ApiError::Wallet(e))?;

    Ok(Json(serde_json::to_value(result).unwrap()))
}

async fn get_supported_tokens(
    Extension(state): Extension<Arc<AppState>>,
    Path(key_type): Path<KeyType>,
) -> Result<Json<Vec<Token>>> {
    let tokens = fo3_wallet::defi::get_supported_tokens(key_type, &state.provider_config)
        .map_err(|e| ApiError::Wallet(e))?;

    Ok(Json(tokens))
}

async fn execute_lending(
    Extension(state): Extension<Arc<AppState>>,
    Json(request): Json<LendingRequest>,
) -> Result<Json<serde_json::Value>> {
    let result = fo3_wallet::defi::execute_lending(&request, &state.provider_config)
        .map_err(|e| ApiError::Wallet(e))?;

    Ok(Json(serde_json::to_value(result).unwrap()))
}

async fn execute_staking(
    Extension(state): Extension<Arc<AppState>>,
    Json(request): Json<StakingRequest>,
) -> Result<Json<serde_json::Value>> {
    let result = fo3_wallet::defi::execute_staking(&request, &state.provider_config)
        .map_err(|e| ApiError::Wallet(e))?;

    Ok(Json(serde_json::to_value(result).unwrap()))
}

async fn health_check() -> &'static str {
    "OK"
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info,tower_http=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Create application state
    let state = Arc::new(AppState::new());

    // Build our application with routes
    let app = Router::new()
        .route("/health", get(health_check))
        // Wallet routes
        .route("/wallets", get(get_all_wallets))
        .route("/wallets", post(create_wallet))
        .route("/wallets/:id", get(get_wallet))
        .route("/wallets/import", post(import_wallet))
        .route("/wallets/derive-address", post(derive_address))
        // Transaction routes
        .route("/transactions", post(send_transaction))
        .route("/transactions/:key_type/:hash", get(get_transaction))
        // DeFi routes
        .route("/defi/tokens/:key_type", get(get_supported_tokens))
        .route("/defi/swap", post(swap_tokens))
        .route("/defi/lending", post(execute_lending))
        .route("/defi/staking", post(execute_staking))
        .layer(Extension(state));

    // Run the server
    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    tracing::info!("Listening on {}", addr);
    // let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}
