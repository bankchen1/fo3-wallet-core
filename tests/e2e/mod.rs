//! End-to-end tests for FO3 Wallet Core

pub mod auth_tests;
pub mod wallet_tests;
pub mod transaction_tests;
pub mod defi_tests;
pub mod websocket_tests;
pub mod security_tests;
pub mod kyc_tests;
pub mod fiat_gateway_tests;

use std::sync::Arc;
use std::time::Duration;
use tonic::transport::Channel;
use tokio::time::timeout;

use fo3_wallet_api::proto::fo3::wallet::v1::{
    wallet_service_client::WalletServiceClient,
    transaction_service_client::TransactionServiceClient,
    defi_service_client::DefiServiceClient,
    auth_service_client::AuthServiceClient,
    health_service_client::HealthServiceClient,
    event_service_client::EventServiceClient,
    kyc_service_client::KycServiceClient,
    fiat_gateway_service_client::FiatGatewayServiceClient,
};

/// Test configuration
pub struct TestConfig {
    pub grpc_endpoint: String,
    pub websocket_endpoint: String,
    pub test_timeout: Duration,
    pub admin_username: String,
    pub admin_password: String,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            grpc_endpoint: std::env::var("TEST_GRPC_ENDPOINT")
                .unwrap_or_else(|_| "http://localhost:50051".to_string()),
            websocket_endpoint: std::env::var("TEST_WEBSOCKET_ENDPOINT")
                .unwrap_or_else(|_| "ws://localhost:8080/ws".to_string()),
            test_timeout: Duration::from_secs(30),
            admin_username: "admin".to_string(),
            admin_password: "admin123".to_string(),
        }
    }
}

/// Test client wrapper with all gRPC clients
pub struct TestClient {
    pub wallet: WalletServiceClient<Channel>,
    pub transaction: TransactionServiceClient<Channel>,
    pub defi: DefiServiceClient<Channel>,
    pub auth: AuthServiceClient<Channel>,
    pub health: HealthServiceClient<Channel>,
    pub events: EventServiceClient<Channel>,
    pub kyc: KycServiceClient<Channel>,
    pub fiat_gateway: FiatGatewayServiceClient<Channel>,
    pub config: TestConfig,
}

impl TestClient {
    /// Create a new test client
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let config = TestConfig::default();
        
        // Wait for service to be ready
        Self::wait_for_service(&config.grpc_endpoint).await?;
        
        let channel = Channel::from_shared(config.grpc_endpoint.clone())?
            .connect()
            .await?;

        Ok(Self {
            wallet: WalletServiceClient::new(channel.clone()),
            transaction: TransactionServiceClient::new(channel.clone()),
            defi: DefiServiceClient::new(channel.clone()),
            auth: AuthServiceClient::new(channel.clone()),
            health: HealthServiceClient::new(channel.clone()),
            events: EventServiceClient::new(channel.clone()),
            kyc: KycServiceClient::new(channel.clone()),
            fiat_gateway: FiatGatewayServiceClient::new(channel.clone()),
            config,
        })
    }

    /// Wait for the gRPC service to be ready
    async fn wait_for_service(endpoint: &str) -> Result<(), Box<dyn std::error::Error>> {
        let max_attempts = 30;
        let mut attempts = 0;

        while attempts < max_attempts {
            match Channel::from_shared(endpoint.to_string())?.connect().await {
                Ok(channel) => {
                    let mut health_client = HealthServiceClient::new(channel);
                    
                    match timeout(
                        Duration::from_secs(5),
                        health_client.check(tonic::Request::new(
                            fo3_wallet_api::proto::fo3::wallet::v1::HealthCheckRequest {
                                service: String::new(),
                            }
                        ))
                    ).await {
                        Ok(Ok(_)) => {
                            println!("Service is ready!");
                            return Ok(());
                        }
                        _ => {}
                    }
                }
                Err(_) => {}
            }

            attempts += 1;
            tokio::time::sleep(Duration::from_secs(2)).await;
            println!("Waiting for service... attempt {}/{}", attempts, max_attempts);
        }

        Err("Service did not become ready in time".into())
    }

    /// Authenticate and get access token
    pub async fn authenticate(&mut self) -> Result<String, Box<dyn std::error::Error>> {
        use fo3_wallet_api::proto::fo3::wallet::v1::LoginRequest;

        let request = tonic::Request::new(LoginRequest {
            username: self.config.admin_username.clone(),
            password: self.config.admin_password.clone(),
            remember_me: false,
        });

        let response = self.auth.login(request).await?;
        let login_response = response.into_inner();

        Ok(login_response.access_token)
    }

    /// Add authentication header to request
    pub fn add_auth_header<T>(
        &self,
        mut request: tonic::Request<T>,
        token: &str,
    ) -> tonic::Request<T> {
        let auth_header = format!("Bearer {}", token);
        request.metadata_mut().insert(
            "authorization",
            auth_header.parse().unwrap(),
        );
        request
    }
}

/// Test result aggregator
#[derive(Debug, Default)]
pub struct TestResults {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub failures: Vec<String>,
}

impl TestResults {
    pub fn add_result(&mut self, test_name: &str, success: bool, error: Option<String>) {
        self.total += 1;
        if success {
            self.passed += 1;
            println!("✅ {}", test_name);
        } else {
            self.failed += 1;
            let error_msg = error.unwrap_or_else(|| "Unknown error".to_string());
            println!("❌ {}: {}", test_name, error_msg);
            self.failures.push(format!("{}: {}", test_name, error_msg));
        }
    }

    pub fn success_rate(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            (self.passed as f64 / self.total as f64) * 100.0
        }
    }

    pub fn print_summary(&self) {
        println!("\n📊 Test Summary:");
        println!("Total tests: {}", self.total);
        println!("Passed: {}", self.passed);
        println!("Failed: {}", self.failed);
        println!("Success rate: {:.1}%", self.success_rate());

        if !self.failures.is_empty() {
            println!("\n❌ Failures:");
            for failure in &self.failures {
                println!("  - {}", failure);
            }
        }
    }
}

/// Macro for running tests with error handling
#[macro_export]
macro_rules! run_test {
    ($results:expr, $test_name:expr, $test_fn:expr) => {
        match $test_fn.await {
            Ok(_) => $results.add_result($test_name, true, None),
            Err(e) => $results.add_result($test_name, false, Some(e.to_string())),
        }
    };
}

/// Helper function to generate random test data
pub fn generate_test_wallet_name() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let suffix: u32 = rng.gen_range(1000..9999);
    format!("test_wallet_{}", suffix)
}
