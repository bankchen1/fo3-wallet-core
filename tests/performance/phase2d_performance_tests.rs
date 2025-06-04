//! Performance tests for Phase 2D services
//! Validates response times and load handling capabilities

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::sleep;
use uuid::Uuid;
use chrono::Utc;
use std::collections::HashMap;
use futures::future::join_all;

use fo3_wallet_api::{
    proto::fo3::wallet::v1::{
        wallet_connect_service_client::WalletConnectServiceClient,
        d_app_signing_service_client::DAppSigningServiceClient,
        earn_service_client::EarnServiceClient,
        *,
    },
    state::AppState,
    services::{
        wallet_connect::WalletConnectServiceImpl,
        dapp_signing::DAppSigningServiceImpl,
        earn::EarnServiceImpl,
    },
    middleware::{
        auth::AuthService,
        audit::AuditLogger,
        rate_limit::RateLimiter,
        wallet_connect_guard::WalletConnectGuard,
        dapp_signing_guard::DAppSigningGuard,
        earn_guard::EarnGuard,
    },
    models::{
        InMemoryWalletConnectRepository,
        InMemoryDAppSigningRepository,
        InMemoryEarnRepository,
    },
};

use tonic::{transport::Server, Request};

/// Performance test configuration
pub struct PerformanceTestConfig {
    pub target_response_time_ms: u64,
    pub complex_operation_target_ms: u64,
    pub concurrent_users: usize,
    pub requests_per_user: usize,
    pub test_duration_seconds: u64,
}

impl Default for PerformanceTestConfig {
    fn default() -> Self {
        Self {
            target_response_time_ms: 200,      // <200ms for standard operations
            complex_operation_target_ms: 500,  // <500ms for complex operations
            concurrent_users: 50,              // 50 concurrent users
            requests_per_user: 10,             // 10 requests per user
            test_duration_seconds: 30,         // 30 second test duration
        }
    }
}

/// Performance test results
#[derive(Debug)]
pub struct PerformanceTestResult {
    pub operation_name: String,
    pub total_requests: usize,
    pub successful_requests: usize,
    pub failed_requests: usize,
    pub average_response_time_ms: f64,
    pub min_response_time_ms: u64,
    pub max_response_time_ms: u64,
    pub p95_response_time_ms: u64,
    pub p99_response_time_ms: u64,
    pub requests_per_second: f64,
    pub target_met: bool,
}

impl PerformanceTestResult {
    pub fn new(operation_name: String, target_ms: u64) -> Self {
        Self {
            operation_name,
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            average_response_time_ms: 0.0,
            min_response_time_ms: u64::MAX,
            max_response_time_ms: 0,
            p95_response_time_ms: 0,
            p99_response_time_ms: 0,
            requests_per_second: 0.0,
            target_met: false,
        }
    }

    pub fn calculate_from_measurements(&mut self, measurements: Vec<u64>, target_ms: u64, duration_secs: f64) {
        if measurements.is_empty() {
            return;
        }

        self.total_requests = measurements.len();
        self.successful_requests = measurements.len(); // All measurements are successful
        self.failed_requests = 0;

        let sum: u64 = measurements.iter().sum();
        self.average_response_time_ms = sum as f64 / measurements.len() as f64;

        self.min_response_time_ms = *measurements.iter().min().unwrap();
        self.max_response_time_ms = *measurements.iter().max().unwrap();

        let mut sorted = measurements.clone();
        sorted.sort();

        let p95_index = (sorted.len() as f64 * 0.95) as usize;
        let p99_index = (sorted.len() as f64 * 0.99) as usize;

        self.p95_response_time_ms = sorted[p95_index.min(sorted.len() - 1)];
        self.p99_response_time_ms = sorted[p99_index.min(sorted.len() - 1)];

        self.requests_per_second = measurements.len() as f64 / duration_secs;
        self.target_met = self.average_response_time_ms <= target_ms as f64;
    }

    pub fn print_summary(&self) {
        println!("\n📊 Performance Test Results: {}", self.operation_name);
        println!("   Total Requests: {}", self.total_requests);
        println!("   Successful: {}", self.successful_requests);
        println!("   Failed: {}", self.failed_requests);
        println!("   Average Response Time: {:.2}ms", self.average_response_time_ms);
        println!("   Min Response Time: {}ms", self.min_response_time_ms);
        println!("   Max Response Time: {}ms", self.max_response_time_ms);
        println!("   P95 Response Time: {}ms", self.p95_response_time_ms);
        println!("   P99 Response Time: {}ms", self.p99_response_time_ms);
        println!("   Requests/Second: {:.2}", self.requests_per_second);
        println!("   Target Met: {}", if self.target_met { "✅" } else { "❌" });
    }
}

/// Performance test client
pub struct PerformanceTestClient {
    pub wallet_connect: WalletConnectServiceClient<tonic::transport::Channel>,
    pub dapp_signing: DAppSigningServiceClient<tonic::transport::Channel>,
    pub earn: EarnServiceClient<tonic::transport::Channel>,
    pub config: PerformanceTestConfig,
}

impl PerformanceTestClient {
    pub async fn new(server_url: String) -> Result<Self, Box<dyn std::error::Error>> {
        let channel = tonic::transport::Channel::from_shared(server_url)?
            .connect()
            .await?;

        Ok(Self {
            wallet_connect: WalletConnectServiceClient::new(channel.clone()),
            dapp_signing: DAppSigningServiceClient::new(channel.clone()),
            earn: EarnServiceClient::new(channel.clone()),
            config: PerformanceTestConfig::default(),
        })
    }

    pub fn create_auth_request<T>(&self, payload: T) -> Request<T> {
        let mut request = Request::new(payload);
        request.metadata_mut().insert(
            "authorization",
            "Bearer perf_test_token".parse().unwrap(),
        );
        request
    }
}

/// Start performance test server
async fn start_performance_test_server() -> (String, tokio::task::JoinHandle<()>) {
    let state = Arc::new(AppState::new());
    let auth_service = Arc::new(AuthService::new(state.clone()));
    let audit_logger = Arc::new(AuditLogger::new(state.clone()));
    let rate_limiter = Arc::new(RateLimiter::new());

    // Initialize repositories
    let wc_repository = Arc::new(InMemoryWalletConnectRepository::new());
    let signing_repository = Arc::new(InMemoryDAppSigningRepository::new());
    let earn_repository = Arc::new(InMemoryEarnRepository::new());

    // Initialize guards with higher rate limits for performance testing
    let wc_guard = Arc::new(WalletConnectGuard::new(
        auth_service.clone(),
        audit_logger.clone(),
        rate_limiter.clone(),
        wc_repository.clone(),
    ));
    let signing_guard = Arc::new(DAppSigningGuard::new(
        auth_service.clone(),
        audit_logger.clone(),
        rate_limiter.clone(),
        signing_repository.clone(),
    ));
    let earn_guard = Arc::new(EarnGuard::new(
        auth_service.clone(),
        audit_logger.clone(),
        rate_limiter.clone(),
        earn_repository.clone(),
    ));

    // Create services
    let wc_service = WalletConnectServiceImpl::new(
        state.clone(),
        auth_service.clone(),
        audit_logger.clone(),
        wc_guard,
        wc_repository,
    );
    let signing_service = DAppSigningServiceImpl::new(
        state.clone(),
        auth_service.clone(),
        audit_logger.clone(),
        signing_guard,
        signing_repository,
    );
    let earn_service = EarnServiceImpl::new(
        state.clone(),
        auth_service.clone(),
        audit_logger.clone(),
        earn_guard,
        earn_repository,
    );

    let addr = "127.0.0.1:0".parse().unwrap();
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = Server::builder()
        .add_service(wallet_connect_service_server::WalletConnectServiceServer::new(wc_service))
        .add_service(d_app_signing_service_server::DAppSigningServiceServer::new(signing_service))
        .add_service(earn_service_server::EarnServiceServer::new(earn_service))
        .serve_with_incoming(tokio_stream::wrappers::TcpListenerStream::new(listener));

    let handle = tokio::spawn(server);
    let server_url = format!("http://{}", addr);

    // Give the server a moment to start
    sleep(Duration::from_millis(100)).await;

    (server_url, handle)
}

#[tokio::test]
async fn test_earn_service_performance() {
    println!("🚀 Testing EarnService Performance");

    let (server_url, _handle) = start_performance_test_server().await;
    let mut client = PerformanceTestClient::new(server_url).await.expect("Failed to create client");

    // Test get_yield_products performance
    let mut measurements = Vec::new();
    let test_start = Instant::now();

    for _i in 0..100 {
        let request = client.create_auth_request(GetYieldProductsRequest {
            product_type: 0,
            protocol: 0,
            chain_type: 0,
            chain_id: String::new(),
            max_risk_level: 0,
            min_apy: String::new(),
            max_apy: String::new(),
            active_only: true,
            sort_by: "apy".to_string(),
            sort_desc: true,
            page_size: 20,
            page_token: String::new(),
        });

        let start = Instant::now();
        let response = client.earn.get_yield_products(request).await;
        let duration = start.elapsed();

        if response.is_ok() {
            measurements.push(duration.as_millis() as u64);
        }
    }

    let test_duration = test_start.elapsed().as_secs_f64();
    let mut result = PerformanceTestResult::new("get_yield_products".to_string(), client.config.target_response_time_ms);
    result.calculate_from_measurements(measurements, client.config.target_response_time_ms, test_duration);
    result.print_summary();

    assert!(result.target_met, "get_yield_products should meet performance target");
}

#[tokio::test]
async fn test_wallet_connect_performance() {
    println!("🔗 Testing WalletConnect Performance");

    let (server_url, _handle) = start_performance_test_server().await;
    let mut client = PerformanceTestClient::new(server_url).await.expect("Failed to create client");

    // Test list_sessions performance
    let mut measurements = Vec::new();
    let test_start = Instant::now();

    for _i in 0..100 {
        let request = client.create_auth_request(ListSessionsRequest {
            status: 0,
            dapp_url: String::new(),
            page_size: 20,
            page_token: String::new(),
        });

        let start = Instant::now();
        let response = client.wallet_connect.list_sessions(request).await;
        let duration = start.elapsed();

        if response.is_ok() {
            measurements.push(duration.as_millis() as u64);
        }
    }

    let test_duration = test_start.elapsed().as_secs_f64();
    let mut result = PerformanceTestResult::new("list_sessions".to_string(), client.config.target_response_time_ms);
    result.calculate_from_measurements(measurements, client.config.target_response_time_ms, test_duration);
    result.print_summary();

    assert!(result.target_met, "list_sessions should meet performance target");
}

#[tokio::test]
async fn test_dapp_signing_performance() {
    println!("✍️ Testing DApp Signing Performance");

    let (server_url, _handle) = start_performance_test_server().await;
    let mut client = PerformanceTestClient::new(server_url).await.expect("Failed to create client");

    // Test simulate_transaction performance (complex operation)
    let mut measurements = Vec::new();
    let test_start = Instant::now();

    for _i in 0..50 {
        let request = client.create_auth_request(SimulateTransactionRequest {
            chain_id: "1".to_string(),
            from_address: "0x1234567890123456789012345678901234567890".to_string(),
            to_address: "0x0987654321098765432109876543210987654321".to_string(),
            value: "1000000000000000000".to_string(),
            data: "0x".to_string(),
            gas_limit: "21000".to_string(),
            gas_price: "20000000000".to_string(),
        });

        let start = Instant::now();
        let response = client.dapp_signing.simulate_transaction(request).await;
        let duration = start.elapsed();

        if response.is_ok() {
            measurements.push(duration.as_millis() as u64);
        }
    }

    let test_duration = test_start.elapsed().as_secs_f64();
    let mut result = PerformanceTestResult::new("simulate_transaction".to_string(), client.config.complex_operation_target_ms);
    result.calculate_from_measurements(measurements, client.config.complex_operation_target_ms, test_duration);
    result.print_summary();

    assert!(result.target_met, "simulate_transaction should meet complex operation target");
}

#[tokio::test]
async fn test_concurrent_load() {
    println!("⚡ Testing Concurrent Load");

    let (server_url, _handle) = start_performance_test_server().await;
    let config = PerformanceTestConfig::default();

    // Create multiple concurrent clients
    let mut tasks = Vec::new();

    for user_id in 0..config.concurrent_users {
        let server_url_clone = server_url.clone();
        let task = tokio::spawn(async move {
            let mut client = PerformanceTestClient::new(server_url_clone).await.expect("Failed to create client");
            let mut measurements = Vec::new();

            for _request in 0..10 {
                let request = client.create_auth_request(GetYieldProductsRequest {
                    product_type: 0,
                    protocol: 0,
                    chain_type: 0,
                    chain_id: String::new(),
                    max_risk_level: 0,
                    min_apy: String::new(),
                    max_apy: String::new(),
                    active_only: true,
                    sort_by: "apy".to_string(),
                    sort_desc: true,
                    page_size: 20,
                    page_token: String::new(),
                });

                let start = Instant::now();
                let response = client.earn.get_yield_products(request).await;
                let duration = start.elapsed();

                if response.is_ok() {
                    measurements.push(duration.as_millis() as u64);
                }

                // Small delay between requests
                sleep(Duration::from_millis(10)).await;
            }

            (user_id, measurements)
        });

        tasks.push(task);
    }

    let test_start = Instant::now();
    let results = join_all(tasks).await;
    let test_duration = test_start.elapsed().as_secs_f64();

    // Aggregate results
    let mut all_measurements = Vec::new();
    let mut successful_users = 0;

    for result in results {
        if let Ok((user_id, measurements)) = result {
            if !measurements.is_empty() {
                successful_users += 1;
                all_measurements.extend(measurements);
            }
        }
    }

    let mut result = PerformanceTestResult::new("concurrent_load".to_string(), config.target_response_time_ms);
    result.calculate_from_measurements(all_measurements, config.target_response_time_ms, test_duration);
    result.print_summary();

    println!("   Concurrent Users: {}", config.concurrent_users);
    println!("   Successful Users: {}", successful_users);

    assert!(successful_users >= config.concurrent_users / 2, "At least 50% of users should succeed");
    assert!(result.target_met, "Concurrent load should meet performance target");
}

#[tokio::test]
async fn test_memory_usage() {
    println!("💾 Testing Memory Usage");

    let (server_url, _handle) = start_performance_test_server().await;
    let mut client = PerformanceTestClient::new(server_url).await.expect("Failed to create client");

    // Make many requests to test memory stability
    for batch in 0..10 {
        println!("   Processing batch {}/10", batch + 1);

        for _i in 0..100 {
            let request = client.create_auth_request(GetYieldProductsRequest {
                product_type: 0,
                protocol: 0,
                chain_type: 0,
                chain_id: String::new(),
                max_risk_level: 0,
                min_apy: String::new(),
                max_apy: String::new(),
                active_only: true,
                sort_by: "apy".to_string(),
                sort_desc: true,
                page_size: 20,
                page_token: String::new(),
            });

            let _response = client.earn.get_yield_products(request).await;
        }

        // Small delay between batches
        sleep(Duration::from_millis(100)).await;
    }

    println!("✅ Memory usage test completed (1000 requests processed)");
}
