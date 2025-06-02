//! FO3 Wallet API
//!
//! This is the secure gRPC API server for the FO3 multi-chain wallet and DeFi SDK.

mod services;
mod error;
mod state;
mod middleware;
mod tls;
mod websocket;
mod observability;
mod models;
mod storage;

use std::net::SocketAddr;
use std::sync::Arc;

use tonic::{transport::Server, Request, Response, Status};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use axum::Router;

use fo3_wallet::{
    transaction::provider::{ProviderConfig, ProviderType},
};

use crate::state::AppState;
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

    // Create application state
    let state = Arc::new(AppState::new());

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


