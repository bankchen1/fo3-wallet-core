//! Observability module with tracing, metrics, and monitoring

pub mod tracing;
pub mod metrics;
pub mod health;

use std::sync::Arc;
use opentelemetry::{global, KeyValue};
use opentelemetry_sdk::{
    trace::{self, Sampler},
    Resource,
};
use opentelemetry_jaeger::JaegerTraceExporter;
use prometheus::{Registry, Counter, Histogram, Gauge, Opts, HistogramOpts};
use anyhow::Result;

/// Observability configuration
#[derive(Debug, Clone)]
pub struct ObservabilityConfig {
    pub service_name: String,
    pub service_version: String,
    pub jaeger_endpoint: Option<String>,
    pub prometheus_enabled: bool,
    pub trace_sampling_ratio: f64,
}

impl Default for ObservabilityConfig {
    fn default() -> Self {
        Self {
            service_name: "fo3-wallet-api".to_string(),
            service_version: env!("CARGO_PKG_VERSION").to_string(),
            jaeger_endpoint: std::env::var("JAEGER_ENDPOINT").ok(),
            prometheus_enabled: std::env::var("PROMETHEUS_ENABLED")
                .unwrap_or_else(|_| "true".to_string())
                .parse()
                .unwrap_or(true),
            trace_sampling_ratio: std::env::var("TRACE_SAMPLING_RATIO")
                .unwrap_or_else(|_| "0.1".to_string())
                .parse()
                .unwrap_or(0.1),
        }
    }
}

/// Custom metrics for the FO3 Wallet API
pub struct WalletMetrics {
    pub registry: Registry,
    
    // Request metrics
    pub grpc_requests_total: Counter,
    pub grpc_request_duration: Histogram,
    pub grpc_errors_total: Counter,
    
    // Authentication metrics
    pub auth_attempts_total: Counter,
    pub auth_failures_total: Counter,
    pub active_sessions: Gauge,
    
    // Wallet metrics
    pub wallets_created_total: Counter,
    pub wallets_active: Gauge,
    pub addresses_derived_total: Counter,
    
    // Transaction metrics
    pub transactions_signed_total: Counter,
    pub transactions_broadcast_total: Counter,
    pub transaction_confirmations: Histogram,
    
    // DeFi metrics
    pub defi_swaps_total: Counter,
    pub defi_swap_volume: Histogram,
    pub defi_lending_operations_total: Counter,
    
    // WebSocket metrics
    pub websocket_connections_active: Gauge,
    pub websocket_messages_sent: Counter,
    pub websocket_messages_received: Counter,
    
    // Blockchain metrics
    pub blockchain_rpc_calls_total: Counter,
    pub blockchain_rpc_duration: Histogram,
    pub blockchain_rpc_errors_total: Counter,
}

impl WalletMetrics {
    pub fn new() -> Result<Self> {
        let registry = Registry::new();

        // Request metrics
        let grpc_requests_total = Counter::with_opts(Opts::new(
            "grpc_requests_total",
            "Total number of gRPC requests"
        ).const_labels([
            ("service", "fo3-wallet-api"),
        ].iter().cloned().collect()))?;

        let grpc_request_duration = Histogram::with_opts(HistogramOpts::new(
            "grpc_request_duration_seconds",
            "Duration of gRPC requests in seconds"
        ).buckets(vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0]))?;

        let grpc_errors_total = Counter::with_opts(Opts::new(
            "grpc_errors_total",
            "Total number of gRPC errors"
        ))?;

        // Authentication metrics
        let auth_attempts_total = Counter::with_opts(Opts::new(
            "auth_attempts_total",
            "Total number of authentication attempts"
        ))?;

        let auth_failures_total = Counter::with_opts(Opts::new(
            "auth_failures_total",
            "Total number of authentication failures"
        ))?;

        let active_sessions = Gauge::with_opts(Opts::new(
            "active_sessions",
            "Number of active user sessions"
        ))?;

        // Wallet metrics
        let wallets_created_total = Counter::with_opts(Opts::new(
            "wallets_created_total",
            "Total number of wallets created"
        ))?;

        let wallets_active = Gauge::with_opts(Opts::new(
            "wallets_active",
            "Number of active wallets"
        ))?;

        let addresses_derived_total = Counter::with_opts(Opts::new(
            "addresses_derived_total",
            "Total number of addresses derived"
        ))?;

        // Transaction metrics
        let transactions_signed_total = Counter::with_opts(Opts::new(
            "transactions_signed_total",
            "Total number of transactions signed"
        ))?;

        let transactions_broadcast_total = Counter::with_opts(Opts::new(
            "transactions_broadcast_total",
            "Total number of transactions broadcast"
        ))?;

        let transaction_confirmations = Histogram::with_opts(HistogramOpts::new(
            "transaction_confirmations",
            "Number of confirmations for transactions"
        ).buckets(vec![1.0, 3.0, 6.0, 12.0, 24.0, 50.0, 100.0]))?;

        // DeFi metrics
        let defi_swaps_total = Counter::with_opts(Opts::new(
            "defi_swaps_total",
            "Total number of DeFi swaps executed"
        ))?;

        let defi_swap_volume = Histogram::with_opts(HistogramOpts::new(
            "defi_swap_volume_usd",
            "Volume of DeFi swaps in USD"
        ).buckets(vec![1.0, 10.0, 100.0, 1000.0, 10000.0, 100000.0]))?;

        let defi_lending_operations_total = Counter::with_opts(Opts::new(
            "defi_lending_operations_total",
            "Total number of DeFi lending operations"
        ))?;

        // WebSocket metrics
        let websocket_connections_active = Gauge::with_opts(Opts::new(
            "websocket_connections_active",
            "Number of active WebSocket connections"
        ))?;

        let websocket_messages_sent = Counter::with_opts(Opts::new(
            "websocket_messages_sent_total",
            "Total number of WebSocket messages sent"
        ))?;

        let websocket_messages_received = Counter::with_opts(Opts::new(
            "websocket_messages_received_total",
            "Total number of WebSocket messages received"
        ))?;

        // Blockchain metrics
        let blockchain_rpc_calls_total = Counter::with_opts(Opts::new(
            "blockchain_rpc_calls_total",
            "Total number of blockchain RPC calls"
        ))?;

        let blockchain_rpc_duration = Histogram::with_opts(HistogramOpts::new(
            "blockchain_rpc_duration_seconds",
            "Duration of blockchain RPC calls in seconds"
        ).buckets(vec![0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0]))?;

        let blockchain_rpc_errors_total = Counter::with_opts(Opts::new(
            "blockchain_rpc_errors_total",
            "Total number of blockchain RPC errors"
        ))?;

        // Register all metrics
        registry.register(Box::new(grpc_requests_total.clone()))?;
        registry.register(Box::new(grpc_request_duration.clone()))?;
        registry.register(Box::new(grpc_errors_total.clone()))?;
        registry.register(Box::new(auth_attempts_total.clone()))?;
        registry.register(Box::new(auth_failures_total.clone()))?;
        registry.register(Box::new(active_sessions.clone()))?;
        registry.register(Box::new(wallets_created_total.clone()))?;
        registry.register(Box::new(wallets_active.clone()))?;
        registry.register(Box::new(addresses_derived_total.clone()))?;
        registry.register(Box::new(transactions_signed_total.clone()))?;
        registry.register(Box::new(transactions_broadcast_total.clone()))?;
        registry.register(Box::new(transaction_confirmations.clone()))?;
        registry.register(Box::new(defi_swaps_total.clone()))?;
        registry.register(Box::new(defi_swap_volume.clone()))?;
        registry.register(Box::new(defi_lending_operations_total.clone()))?;
        registry.register(Box::new(websocket_connections_active.clone()))?;
        registry.register(Box::new(websocket_messages_sent.clone()))?;
        registry.register(Box::new(websocket_messages_received.clone()))?;
        registry.register(Box::new(blockchain_rpc_calls_total.clone()))?;
        registry.register(Box::new(blockchain_rpc_duration.clone()))?;
        registry.register(Box::new(blockchain_rpc_errors_total.clone()))?;

        Ok(Self {
            registry,
            grpc_requests_total,
            grpc_request_duration,
            grpc_errors_total,
            auth_attempts_total,
            auth_failures_total,
            active_sessions,
            wallets_created_total,
            wallets_active,
            addresses_derived_total,
            transactions_signed_total,
            transactions_broadcast_total,
            transaction_confirmations,
            defi_swaps_total,
            defi_swap_volume,
            defi_lending_operations_total,
            websocket_connections_active,
            websocket_messages_sent,
            websocket_messages_received,
            blockchain_rpc_calls_total,
            blockchain_rpc_duration,
            blockchain_rpc_errors_total,
        })
    }
}

/// Initialize observability stack
pub async fn init_observability(config: ObservabilityConfig) -> Result<Arc<WalletMetrics>> {
    // Initialize tracing
    init_tracing(&config).await?;
    
    // Initialize metrics
    let metrics = Arc::new(WalletMetrics::new()?);
    
    tracing::info!(
        service_name = %config.service_name,
        service_version = %config.service_version,
        jaeger_enabled = config.jaeger_endpoint.is_some(),
        prometheus_enabled = config.prometheus_enabled,
        "Observability initialized"
    );
    
    Ok(metrics)
}

/// Initialize distributed tracing
async fn init_tracing(config: &ObservabilityConfig) -> Result<()> {
    // Create resource with service information
    let resource = Resource::new(vec![
        KeyValue::new("service.name", config.service_name.clone()),
        KeyValue::new("service.version", config.service_version.clone()),
    ]);

    // Configure tracer
    let mut tracer_builder = trace::TracerProvider::builder()
        .with_resource(resource)
        .with_sampler(Sampler::TraceIdRatioBased(config.trace_sampling_ratio));

    // Add Jaeger exporter if configured
    if let Some(jaeger_endpoint) = &config.jaeger_endpoint {
        let jaeger_exporter = JaegerTraceExporter::builder()
            .with_endpoint(jaeger_endpoint)
            .build()?;
        
        tracer_builder = tracer_builder.with_batch_exporter(jaeger_exporter, trace::BatchConfig::default());
    }

    let tracer_provider = tracer_builder.build();
    global::set_tracer_provider(tracer_provider);

    Ok(())
}

/// Create metrics endpoint handler
pub fn create_metrics_handler(metrics: Arc<WalletMetrics>) -> axum::Router {
    use axum::{routing::get, Router};
    use prometheus::{Encoder, TextEncoder};

    async fn metrics_handler(
        axum::extract::State(metrics): axum::extract::State<Arc<WalletMetrics>>,
    ) -> Result<String, axum::http::StatusCode> {
        let encoder = TextEncoder::new();
        let metric_families = metrics.registry.gather();
        
        encoder.encode_to_string(&metric_families)
            .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)
    }

    Router::new()
        .route("/metrics", get(metrics_handler))
        .with_state(metrics)
}
