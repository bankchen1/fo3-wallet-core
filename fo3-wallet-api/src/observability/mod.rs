//! Observability module with tracing, metrics, and monitoring

pub mod tracing;
pub mod metrics;
pub mod health;

use std::sync::Arc;
use std::collections::HashMap;
use opentelemetry::{global, KeyValue};
use opentelemetry_sdk::{
    trace::{self, Sampler},
    Resource,
};
use opentelemetry_jaeger::JaegerTraceExporter;
use prometheus::{Registry, Counter, Histogram, Gauge, Opts, HistogramOpts, CounterVec, HistogramVec, GaugeVec, TextEncoder, Encoder};
use anyhow::Result;
use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::get,
    Router,
};
use serde_json::{json, Value};
use tracing::{info, warn, error};

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

    // Enhanced business metrics for local validation
    pub kyc_submissions: CounterVec,
    pub card_operations: CounterVec,
    pub trading_operations: CounterVec,
    pub defi_operations: CounterVec,
    pub dapp_connections: CounterVec,
    pub notification_deliveries: CounterVec,
    pub ml_inferences: CounterVec,

    // Performance metrics
    pub response_times: HistogramVec,
    pub concurrent_operations: GaugeVec,

    // Security metrics
    pub authentication_attempts: CounterVec,
    pub security_violations: CounterVec,
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

        // Enhanced business metrics for local validation
        let kyc_submissions = CounterVec::new(
            Opts::new("kyc_submissions_total", "Total KYC submissions"),
            &["status", "document_type", "country"]
        )?;

        let card_operations = CounterVec::new(
            Opts::new("card_operations_total", "Total card operations"),
            &["operation", "currency", "status"]
        )?;

        let trading_operations = CounterVec::new(
            Opts::new("trading_operations_total", "Total trading operations"),
            &["operation", "strategy_type", "symbol", "status"]
        )?;

        let defi_operations = CounterVec::new(
            Opts::new("defi_operations_total", "Total DeFi operations"),
            &["operation", "protocol", "asset", "status"]
        )?;

        let dapp_connections = CounterVec::new(
            Opts::new("dapp_connections_total", "Total DApp connections"),
            &["dapp_url", "chain", "status"]
        )?;

        let notification_deliveries = CounterVec::new(
            Opts::new("notification_deliveries_total", "Total notification deliveries"),
            &["type", "channel", "status"]
        )?;

        let ml_inferences = CounterVec::new(
            Opts::new("ml_inferences_total", "Total ML model inferences"),
            &["model", "input_type", "status"]
        )?;

        // Performance metrics
        let response_times = HistogramVec::new(
            HistogramOpts::new("response_times_seconds", "Response times by service and operation")
                .buckets(vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.2, 0.5, 1.0]),
            &["service", "operation"]
        )?;

        let concurrent_operations = GaugeVec::new(
            Opts::new("concurrent_operations", "Number of concurrent operations by service"),
            &["service"]
        )?;

        // Security metrics
        let authentication_attempts = CounterVec::new(
            Opts::new("authentication_attempts_total", "Total authentication attempts"),
            &["method", "status", "user_agent"]
        )?;

        let security_violations = CounterVec::new(
            Opts::new("security_violations_total", "Total security violations"),
            &["type", "severity", "source"]
        )?;

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

        // Register enhanced metrics
        registry.register(Box::new(kyc_submissions.clone()))?;
        registry.register(Box::new(card_operations.clone()))?;
        registry.register(Box::new(trading_operations.clone()))?;
        registry.register(Box::new(defi_operations.clone()))?;
        registry.register(Box::new(dapp_connections.clone()))?;
        registry.register(Box::new(notification_deliveries.clone()))?;
        registry.register(Box::new(ml_inferences.clone()))?;
        registry.register(Box::new(response_times.clone()))?;
        registry.register(Box::new(concurrent_operations.clone()))?;
        registry.register(Box::new(authentication_attempts.clone()))?;
        registry.register(Box::new(security_violations.clone()))?;

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

            // Enhanced metrics
            kyc_submissions,
            card_operations,
            trading_operations,
            defi_operations,
            dapp_connections,
            notification_deliveries,
            ml_inferences,
            response_times,
            concurrent_operations,
            authentication_attempts,
            security_violations,
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

/// Create metrics endpoint handler with health checks
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

    async fn health_handler() -> Json<Value> {
        Json(json!({
            "status": "healthy",
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "service": "fo3-wallet-api",
            "version": env!("CARGO_PKG_VERSION"),
            "environment": "development"
        }))
    }

    async fn readiness_handler() -> Json<Value> {
        // TODO: Add actual readiness checks (database, redis, external services)
        Json(json!({
            "status": "ready",
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "checks": {
                "database": "ok",
                "redis": "ok",
                "external_apis": "ok",
                "ml_models": "ok"
            },
            "uptime_seconds": 0 // TODO: Calculate actual uptime
        }))
    }

    Router::new()
        .route("/metrics", get(metrics_handler))
        .route("/health", get(health_handler))
        .route("/ready", get(readiness_handler))
        .with_state(metrics)
}
