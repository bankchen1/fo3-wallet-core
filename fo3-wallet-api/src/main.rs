//! FO3 Wallet API
//!
//! This is the secure gRPC API server for the FO3 multi-chain wallet and DeFi SDK.

mod services;
mod error;
mod state;
mod middleware;
mod ml;
mod tls;
mod websocket;
mod observability;
mod models;
mod storage;
mod database;
mod cache;

use std::net::SocketAddr;
use std::sync::Arc;

use tonic::{transport::Server, Request, Response, Status};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use axum::Router;

use fo3_wallet::{
    transaction::provider::{ProviderConfig, ProviderType},
};

use crate::state::AppState;
use crate::database::{initialize_database, ConnectionConfig as DatabaseConfig};
use crate::services::{
    wallet::WalletServiceImpl,
    transaction::TransactionServiceImpl,
    defi::DefiServiceImpl,
    health::HealthServiceImpl,
    auth::AuthServiceImpl,
    events::EventServiceImpl,
    kyc::KycServiceImpl,
    fiat_gateway::FiatGatewayServiceImpl,
    pricing::PricingServiceImpl,
    notifications::NotificationServiceImpl,
    cards::CardServiceImpl,
    spending_insights::SpendingInsightsServiceImpl,
    card_funding::CardFundingServiceImpl,
    ledger::LedgerServiceImpl,
    rewards::RewardsServiceImpl,
    referral::ReferralServiceImpl,
    earn::EarnServiceImpl,
    moonshot::MoonshotTradingServiceImpl,
    automated_trading::AutomatedTradingServiceImpl,
    market_intelligence::MarketIntelligenceServiceImpl,
    dapp_browser::DAppBrowserServiceImpl,
};
use crate::middleware::{
    auth::AuthService,
    rate_limit::RateLimiter,
    audit::AuditLogger,
    kyc_guard::KycGuard,
    fiat_guard::FiatGuard,
    pricing_guard::PricingGuard,
    card_guard::CardGuard,
    spending_guard::SpendingGuard,
    card_funding_guard::CardFundingGuard,
    ledger_guard::LedgerGuard,
    rewards_guard::RewardsGuard,
    referral_guard::ReferralGuard,
    earn_guard::EarnGuard,
    moonshot_guard::MoonshotGuard,
    trading_guard::TradingGuard,
};
use crate::websocket::{WebSocketManager, WebSocketServer};
use crate::observability::{ObservabilityConfig, init_observability};
use crate::tls::get_tls_config;

#[cfg(feature = "solana")]
use crate::services::solana::SolanaServiceImpl;

// Include the generated gRPC code
pub mod proto {
    pub mod fo3 {
        pub mod wallet {
            pub mod v1 {
                tonic::include_proto!("fo3.wallet.v1");
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize observability
    let observability_config = ObservabilityConfig::default();
    let metrics = init_observability(observability_config).await?;

    // Initialize tracing with OpenTelemetry
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info,tower_http=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_opentelemetry::layer())
        .init();

    tracing::info!("Starting FO3 Wallet Core API with enhanced security and observability");

    // Initialize database
    let database_config = DatabaseConfig::from_env();
    let database_pool = initialize_database(&database_config).await?;
    tracing::info!("Database initialized successfully");

    // Create application state with database pool
    let state = Arc::new(AppState::new(database_pool));

    // Initialize security services
    let auth_service = Arc::new(AuthService::new(state.clone()));
    let rate_limiter = Arc::new(RateLimiter::new());
    let audit_logger = Arc::new(AuditLogger::new(state.clone()));
    let kyc_guard = Arc::new(KycGuard::new(state.clone(), auth_service.clone()));
    let fiat_guard = Arc::new(FiatGuard::new(state.clone(), auth_service.clone(), kyc_guard.clone()));
    let pricing_guard = Arc::new(PricingGuard::new(state.clone(), auth_service.clone(), None));

    // Initialize WebSocket manager
    let websocket_manager = Arc::new(WebSocketManager::new(auth_service.clone()));

    // Create gRPC services with authentication
    let wallet_service = WalletServiceImpl::new(state.clone(), auth_service.clone(), audit_logger.clone());
    let transaction_service = TransactionServiceImpl::new(state.clone(), auth_service.clone(), audit_logger.clone());
    let defi_service = DefiServiceImpl::new(state.clone(), auth_service.clone(), audit_logger.clone());
    let health_service = HealthServiceImpl::new();
    let auth_grpc_service = AuthServiceImpl::new(auth_service.clone(), audit_logger.clone());
    let event_service = EventServiceImpl::new(websocket_manager.clone(), auth_service.clone());
    let kyc_service = KycServiceImpl::new(state.clone(), auth_service.clone(), audit_logger.clone());
    let fiat_gateway_service = FiatGatewayServiceImpl::new(state.clone(), auth_service.clone(), audit_logger.clone(), fiat_guard.clone());
    let pricing_service = PricingServiceImpl::new(
        state.clone(),
        auth_service.clone(),
        audit_logger.clone(),
        pricing_guard.clone()
    );
    let notification_service = NotificationServiceImpl::new(
        state.clone(),
        auth_service.clone(),
        audit_logger.clone(),
        websocket_manager.clone()
    );
    let card_service = CardServiceImpl::new(
        state.clone(),
        auth_service.clone(),
        audit_logger.clone()
    );
    let spending_insights_service = SpendingInsightsServiceImpl::new(
        state.clone(),
        auth_service.clone(),
        audit_logger.clone()
    );

    // Initialize card funding repository and guard
    let card_funding_repository = Arc::new(crate::models::InMemoryCardFundingRepository::new());
    let card_funding_guard = Arc::new(CardFundingGuard::new(
        auth_service.clone(),
        audit_logger.clone(),
        rate_limiter.clone(),
        card_funding_repository.clone()
    ));
    let card_funding_service = CardFundingServiceImpl::new(
        state.clone(),
        auth_service.clone(),
        audit_logger.clone(),
        card_funding_guard.clone(),
        card_funding_repository.clone()
    );

    // Initialize ledger repository and guard
    let ledger_repository = Arc::new(crate::models::InMemoryLedgerRepository::new());
    let ledger_guard = Arc::new(LedgerGuard::new(
        auth_service.clone(),
        audit_logger.clone(),
        rate_limiter.clone(),
        ledger_repository.clone()
    ));
    let ledger_service = LedgerServiceImpl::new(
        state.clone(),
        auth_service.clone(),
        audit_logger.clone(),
        ledger_guard.clone(),
        ledger_repository.clone()
    );

    // Initialize rewards repository and guard
    let rewards_repository = Arc::new(crate::models::InMemoryRewardsRepository::default());
    let rewards_guard = Arc::new(RewardsGuard::new(
        auth_service.clone(),
        audit_logger.clone(),
        rate_limiter.clone(),
        rewards_repository.clone()
    ));
    let rewards_service = RewardsServiceImpl::new(
        state.clone(),
        auth_service.clone(),
        audit_logger.clone(),
        rewards_guard.clone(),
        rewards_repository.clone()
    );

    // Initialize referral repository and guard
    let referral_repository = Arc::new(crate::models::InMemoryReferralRepository::default());
    let referral_guard = Arc::new(ReferralGuard::new(
        auth_service.clone(),
        audit_logger.clone(),
        rate_limiter.clone(),
        referral_repository.clone()
    ));
    let referral_service = ReferralServiceImpl::new(
        state.clone(),
        auth_service.clone(),
        audit_logger.clone(),
        referral_guard.clone(),
        referral_repository.clone()
    );

    // Initialize earn repository and guard
    let earn_repository = Arc::new(crate::models::InMemoryEarnRepository::new());
    let earn_guard = Arc::new(EarnGuard::new(
        auth_service.clone(),
        audit_logger.clone(),
        rate_limiter.clone(),
        earn_repository.clone()
    ));
    let earn_service = EarnServiceImpl::new(
        state.clone(),
        auth_service.clone(),
        audit_logger.clone(),
        earn_guard.clone(),
        earn_repository.clone()
    );

    // Initialize moonshot repository and guard
    let moonshot_repository = Arc::new(crate::models::InMemoryMoonshotRepository::new());
    moonshot_repository.initialize_mock_data().await?;
    let moonshot_guard = Arc::new(MoonshotGuard::new()?);
    let moonshot_service = MoonshotTradingServiceImpl::new(
        moonshot_repository,
        auth_service.clone(),
        audit_logger.clone(),
        rate_limiter.clone(),
        moonshot_guard
    );

    // Initialize ML model manager
    let ml_config = crate::ml::MLConfig::default();
    let model_manager = Arc::new(crate::ml::ModelManager::new(ml_config));

    // Load ML models
    if let Err(e) = model_manager.load_model("sentiment_v1", "/app/models/sentiment").await {
        tracing::warn!("Failed to load sentiment model: {}", e);
    }
    if let Err(e) = model_manager.load_model("yield_predictor_v1", "/app/models/yield").await {
        tracing::warn!("Failed to load yield predictor model: {}", e);
    }

    // Initialize trading guard and automated trading service
    let trading_guard = Arc::new(TradingGuard::new(
        auth_service.clone(),
        audit_logger.clone(),
        rate_limiter.clone()
    ));
    let automated_trading_service = AutomatedTradingServiceImpl::new(
        auth_service.clone(),
        audit_logger.clone(),
        trading_guard.clone(),
        model_manager.clone()
    );

    // Initialize market intelligence service with ML integration
    let market_intelligence_service = MarketIntelligenceServiceImpl::new(
        auth_service.clone(),
        audit_logger.clone(),
        rate_limiter.clone(),
        model_manager.clone()
    );

    // Initialize DApp browser service
    let dapp_browser_service = DAppBrowserServiceImpl::new(
        auth_service.clone(),
        audit_logger.clone(),
        rate_limiter.clone()
    );

    // Get server addresses
    let grpc_addr: SocketAddr = std::env::var("GRPC_LISTEN_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:50051".to_string())
        .parse()?;

    let websocket_addr: SocketAddr = std::env::var("WEBSOCKET_LISTEN_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:8080".to_string())
        .parse()?;

    let metrics_addr: SocketAddr = std::env::var("METRICS_LISTEN_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:9090".to_string())
        .parse()?;

    // Configure TLS if enabled
    let tls_config = get_tls_config()?;

    // Build the gRPC server
    let mut server_builder = if let Some(tls) = tls_config {
        tracing::info!("TLS enabled for gRPC server");
        Server::builder().tls_config(tls.server_config)?
    } else {
        tracing::warn!("TLS disabled - not recommended for production");
        Server::builder()
    };

    // Add services with authentication and rate limiting
    server_builder = server_builder
        .add_service(
            tonic_web::enable(
                proto::fo3::wallet::v1::wallet_service_server::WalletServiceServer::new(wallet_service)
            )
        )
        .add_service(
            tonic_web::enable(
                proto::fo3::wallet::v1::transaction_service_server::TransactionServiceServer::new(transaction_service)
            )
        )
        .add_service(
            tonic_web::enable(
                proto::fo3::wallet::v1::defi_service_server::DefiServiceServer::new(defi_service)
            )
        )
        .add_service(
            proto::fo3::wallet::v1::health_service_server::HealthServiceServer::new(health_service)
        )
        .add_service(
            proto::fo3::wallet::v1::auth_service_server::AuthServiceServer::new(auth_grpc_service)
        )
        .add_service(
            proto::fo3::wallet::v1::event_service_server::EventServiceServer::new(event_service)
        )
        .add_service(
            tonic_web::enable(
                proto::fo3::wallet::v1::kyc_service_server::KycServiceServer::new(kyc_service)
            )
        )
        .add_service(
            tonic_web::enable(
                proto::fo3::wallet::v1::fiat_gateway_service_server::FiatGatewayServiceServer::new(fiat_gateway_service)
            )
        )
        .add_service(
            tonic_web::enable(
                proto::fo3::wallet::v1::pricing_service_server::PricingServiceServer::new(pricing_service)
            )
        )
        .add_service(
            tonic_web::enable(
                proto::fo3::wallet::v1::notification_service_server::NotificationServiceServer::new(notification_service)
            )
        )
        .add_service(
            tonic_web::enable(
                proto::fo3::wallet::v1::card_service_server::CardServiceServer::new(card_service)
            )
        )
        .add_service(
            tonic_web::enable(
                proto::fo3::wallet::v1::spending_insights_service_server::SpendingInsightsServiceServer::new(spending_insights_service)
            )
        )
        .add_service(
            tonic_web::enable(
                proto::fo3::wallet::v1::card_funding_service_server::CardFundingServiceServer::new(card_funding_service)
            )
        )
        .add_service(
            tonic_web::enable(
                proto::fo3::wallet::v1::ledger_service_server::LedgerServiceServer::new(ledger_service)
            )
        )
        .add_service(
            tonic_web::enable(
                proto::fo3::wallet::v1::rewards_service_server::RewardsServiceServer::new(rewards_service)
            )
        )
        .add_service(
            tonic_web::enable(
                proto::fo3::wallet::v1::referral_service_server::ReferralServiceServer::new(referral_service)
            )
        )
        .add_service(
            tonic_web::enable(
                proto::fo3::wallet::v1::earn_service_server::EarnServiceServer::new(earn_service)
            )
        )
        .add_service(
            tonic_web::enable(
                proto::fo3::wallet::v1::moonshot_trading_service_server::MoonshotTradingServiceServer::new(moonshot_service)
            )
        )
        .add_service(
            tonic_web::enable(
                proto::fo3::wallet::v1::automated_trading_service_server::AutomatedTradingServiceServer::new(automated_trading_service)
            )
        )
        .add_service(
            tonic_web::enable(
                proto::fo3::wallet::v1::market_intelligence_service_server::MarketIntelligenceServiceServer::new(market_intelligence_service)
            )
        )
        .add_service(
            tonic_web::enable(
                proto::fo3::wallet::v1::d_app_browser_service_server::DAppBrowserServiceServer::new(dapp_browser_service)
            )
        );

    // Add Solana service if feature is enabled
    #[cfg(feature = "solana")]
    {
        let solana_service = SolanaServiceImpl::new(state.clone(), auth_service.clone(), audit_logger.clone());
        server_builder = server_builder.add_service(
            tonic_web::enable(
                proto::fo3::wallet::v1::solana_service_server::SolanaServiceServer::new(solana_service)
            )
        );
    }

    // Start background tasks
    let rate_limiter_cleanup = rate_limiter.clone();
    tokio::spawn(async move {
        crate::middleware::rate_limit::cleanup_rate_limit_buckets(rate_limiter_cleanup).await;
    });

    let websocket_cleanup = websocket_manager.clone();
    tokio::spawn(async move {
        crate::websocket::cleanup_stale_connections_task(websocket_cleanup).await;
    });

    // Start WebSocket server
    let websocket_server = WebSocketServer::new(websocket_manager.clone());
    let websocket_handle = {
        let server = websocket_server;
        tokio::spawn(async move {
            if let Err(e) = server.start(websocket_addr).await {
                tracing::error!("WebSocket server error: {}", e);
            }
        })
    };

    // Start metrics server
    let metrics_handle = {
        let metrics_router = crate::observability::create_metrics_handler(metrics.clone());
        let metrics_app = Router::new().nest("/", metrics_router);

        tokio::spawn(async move {
            let listener = tokio::net::TcpListener::bind(metrics_addr).await.unwrap();
            tracing::info!("Metrics server listening on {}", metrics_addr);
            axum::serve(listener, metrics_app).await.unwrap();
        })
    };

    tracing::info!("Starting secure gRPC server on {}", grpc_addr);
    tracing::info!("WebSocket server starting on {}", websocket_addr);
    tracing::info!("Metrics server starting on {}", metrics_addr);

    // Start the gRPC server
    let grpc_handle = tokio::spawn(async move {
        if let Err(e) = server_builder.serve(grpc_addr).await {
            tracing::error!("gRPC server error: {}", e);
        }
    });

    // Wait for all servers
    tokio::select! {
        _ = grpc_handle => tracing::info!("gRPC server stopped"),
        _ = websocket_handle => tracing::info!("WebSocket server stopped"),
        _ = metrics_handle => tracing::info!("Metrics server stopped"),
        _ = tokio::signal::ctrl_c() => {
            tracing::info!("Received shutdown signal");
        }
    }

    // Cleanup
    opentelemetry::global::shutdown_tracer_provider();
    tracing::info!("FO3 Wallet Core API shutdown complete");

    Ok(())
}


