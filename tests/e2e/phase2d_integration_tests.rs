//! End-to-end integration tests for Phase 2D services
//! Tests inter-service communication and complete workflows

use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use uuid::Uuid;
use chrono::Utc;
use std::collections::HashMap;

use fo3_wallet_api::{
    proto::fo3::wallet::v1::{
        wallet_connect_service_client::WalletConnectServiceClient,
        d_app_signing_service_client::DAppSigningServiceClient,
        earn_service_client::EarnServiceClient,
        auth_service_client::AuthServiceClient,
        *,
    },
    state::AppState,
    services::{
        wallet_connect::WalletConnectServiceImpl,
        dapp_signing::DAppSigningServiceImpl,
        earn::EarnServiceImpl,
        auth::AuthServiceImpl,
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

use tonic::{transport::Server, Request, Response, Status};
use tonic::metadata::MetadataValue;
use tokio_stream::wrappers::TcpListenerStream;

/// Test configuration for Phase 2D integration tests
pub struct Phase2DTestConfig {
    pub server_url: String,
    pub test_timeout: Duration,
    pub admin_token: String,
    pub user_token: String,
}

impl Default for Phase2DTestConfig {
    fn default() -> Self {
        Self {
            server_url: "http://127.0.0.1:50051".to_string(),
            test_timeout: Duration::from_secs(30),
            admin_token: "admin_test_token".to_string(),
            user_token: "user_test_token".to_string(),
        }
    }
}

/// Test client for Phase 2D services
pub struct Phase2DTestClient {
    pub wallet_connect: WalletConnectServiceClient<tonic::transport::Channel>,
    pub dapp_signing: DAppSigningServiceClient<tonic::transport::Channel>,
    pub earn: EarnServiceClient<tonic::transport::Channel>,
    pub auth: AuthServiceClient<tonic::transport::Channel>,
    pub config: Phase2DTestConfig,
}

impl Phase2DTestClient {
    /// Create a new test client
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let config = Phase2DTestConfig::default();
        
        let channel = tonic::transport::Channel::from_shared(config.server_url.clone())?
            .connect()
            .await?;

        Ok(Self {
            wallet_connect: WalletConnectServiceClient::new(channel.clone()),
            dapp_signing: DAppSigningServiceClient::new(channel.clone()),
            earn: EarnServiceClient::new(channel.clone()),
            auth: AuthServiceClient::new(channel.clone()),
            config,
        })
    }

    /// Add authentication header to request
    pub fn add_auth_header<T>(&self, mut request: Request<T>, token: &str) -> Request<T> {
        request.metadata_mut().insert(
            "authorization",
            format!("Bearer {}", token).parse().unwrap(),
        );
        request
    }

    /// Create authenticated request with user token
    pub fn create_user_request<T>(&self, payload: T) -> Request<T> {
        self.add_auth_header(Request::new(payload), &self.config.user_token)
    }

    /// Create authenticated request with admin token
    pub fn create_admin_request<T>(&self, payload: T) -> Request<T> {
        self.add_auth_header(Request::new(payload), &self.config.admin_token)
    }
}

/// Start test server with all Phase 2D services
async fn start_test_server() -> (String, tokio::task::JoinHandle<()>) {
    let state = Arc::new(AppState::new());
    let auth_service = Arc::new(AuthService::new(state.clone()));
    let audit_logger = Arc::new(AuditLogger::new(state.clone()));
    let rate_limiter = Arc::new(RateLimiter::new());

    // Initialize repositories
    let wc_repository = Arc::new(InMemoryWalletConnectRepository::new());
    let signing_repository = Arc::new(InMemoryDAppSigningRepository::new());
    let earn_repository = Arc::new(InMemoryEarnRepository::new());

    // Initialize guards
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
    let auth_grpc_service = AuthServiceImpl::new(auth_service.clone(), audit_logger.clone());

    let addr = "127.0.0.1:0".parse().unwrap();
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = Server::builder()
        .add_service(wallet_connect_service_server::WalletConnectServiceServer::new(wc_service))
        .add_service(d_app_signing_service_server::DAppSigningServiceServer::new(signing_service))
        .add_service(earn_service_server::EarnServiceServer::new(earn_service))
        .serve_with_incoming(TcpListenerStream::new(listener));

    let handle = tokio::spawn(server);
    let server_url = format!("http://{}", addr);

    // Give the server a moment to start
    sleep(Duration::from_millis(100)).await;

    (server_url, handle)
}

#[tokio::test]
async fn test_complete_dapp_workflow() {
    println!("🔗 Testing Complete DApp Workflow");

    let (_server_url, _handle) = start_test_server().await;
    let mut client = Phase2DTestClient::new().await.expect("Failed to create test client");

    // 1. Create WalletConnect session
    let create_session_request = client.create_user_request(CreateSessionRequest {
        dapp_name: "Test DeFi DApp".to_string(),
        dapp_url: "https://test-defi-dapp.com".to_string(),
        dapp_icon: "https://test-defi-dapp.com/icon.png".to_string(),
        dapp_description: "Test DeFi DApp for yield farming".to_string(),
        required_chains: vec!["ethereum".to_string()],
        required_methods: vec!["eth_sendTransaction".to_string(), "personal_sign".to_string()],
        required_events: vec!["accountsChanged".to_string()],
        expiry_hours: 24,
        metadata: HashMap::new(),
    });

    let session_response = client.wallet_connect.create_session(create_session_request).await;
    assert!(session_response.is_ok(), "Session creation should succeed");
    let session_id = session_response.unwrap().into_inner().session_id;

    // 2. Approve WalletConnect session
    let approve_request = client.create_user_request(ApproveSessionRequest {
        session_id: session_id.clone(),
        approved_chains: vec!["ethereum".to_string()],
        approved_accounts: vec!["0x1234567890123456789012345678901234567890".to_string()],
        approved_methods: vec!["eth_sendTransaction".to_string()],
        approved_events: vec!["accountsChanged".to_string()],
    });

    let approve_response = client.wallet_connect.approve_session(approve_request).await;
    assert!(approve_response.is_ok(), "Session approval should succeed");

    // 3. Create signing request for DeFi transaction
    let signing_request = client.create_user_request(CreateSigningRequestRequest {
        session_id: session_id.clone(),
        dapp_url: "https://test-defi-dapp.com".to_string(),
        chain_id: "1".to_string(),
        account_address: "0x1234567890123456789012345678901234567890".to_string(),
        signing_method: "eth_sendTransaction".to_string(),
        transaction_data: r#"{"to":"0xA0b86a33E6417c8f2c8B758B2d7D2E89E2c8B758", "value":"0x0", "data":"0xa9059cbb..."}"#.to_string(),
        estimated_gas: "50000".to_string(),
        gas_price: "20000000000".to_string(),
        metadata: HashMap::new(),
    });

    let signing_response = client.dapp_signing.create_signing_request(signing_request).await;
    assert!(signing_response.is_ok(), "Signing request creation should succeed");
    let request_id = signing_response.unwrap().into_inner().request_id;

    // 4. Simulate transaction before signing
    let simulate_request = client.create_user_request(SimulateTransactionRequest {
        chain_id: "1".to_string(),
        from_address: "0x1234567890123456789012345678901234567890".to_string(),
        to_address: "0xA0b86a33E6417c8f2c8B758B2d7D2E89E2c8B758".to_string(),
        value: "0".to_string(),
        data: "0xa9059cbb...".to_string(),
        gas_limit: "50000".to_string(),
        gas_price: "20000000000".to_string(),
    });

    let simulate_response = client.dapp_signing.simulate_transaction(simulate_request).await;
    assert!(simulate_response.is_ok(), "Transaction simulation should succeed");

    // 5. Approve signing request
    let approve_signing_request = client.create_user_request(ApproveSigningRequestRequest {
        request_id: request_id.clone(),
        signature: "0x1234567890abcdef...".to_string(),
        transaction_hash: "0xabcdef1234567890...".to_string(),
        gas_used: "45000".to_string(),
        effective_gas_price: "20000000000".to_string(),
    });

    let approve_signing_response = client.dapp_signing.approve_signing_request(approve_signing_request).await;
    assert!(approve_signing_response.is_ok(), "Signing approval should succeed");

    println!("✅ Complete DApp workflow test passed");
}

#[tokio::test]
async fn test_earn_service_integration() {
    println!("💰 Testing EarnService Integration");

    let (_server_url, _handle) = start_test_server().await;
    let mut client = Phase2DTestClient::new().await.expect("Failed to create test client");

    // 1. Get yield products
    let products_request = client.create_user_request(GetYieldProductsRequest {
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
        page_size: 10,
        page_token: String::new(),
    });

    let products_response = client.earn.get_yield_products(products_request).await;
    assert!(products_response.is_ok(), "Get yield products should succeed");

    // 2. Calculate yield for a product
    let calculate_request = client.create_user_request(CalculateYieldRequest {
        product_id: Uuid::new_v4().to_string(),
        principal_amount: "1000.00".to_string(),
        duration_days: 365,
        compound_frequency: 12,
    });

    let calculate_response = client.earn.calculate_yield(calculate_request).await;
    // This might fail due to product not found, but should not crash
    assert!(calculate_response.is_ok() || calculate_response.unwrap_err().code() == tonic::Code::NotFound);

    // 3. Get portfolio summary
    let portfolio_request = client.create_user_request(GetPortfolioSummaryRequest {
        user_id: String::new(),
    });

    let portfolio_response = client.earn.get_portfolio_summary(portfolio_request).await;
    assert!(portfolio_response.is_ok(), "Get portfolio summary should succeed");

    // 4. Get earn analytics
    let analytics_request = client.create_user_request(GetEarnAnalyticsRequest {
        user_id: String::new(),
        start_date: (Utc::now() - chrono::Duration::days(30)).timestamp(),
        end_date: Utc::now().timestamp(),
    });

    let analytics_response = client.earn.get_earn_analytics(analytics_request).await;
    assert!(analytics_response.is_ok(), "Get earn analytics should succeed");

    println!("✅ EarnService integration test passed");
}

#[tokio::test]
async fn test_cross_service_analytics() {
    println!("📊 Testing Cross-Service Analytics");

    let (_server_url, _handle) = start_test_server().await;
    let mut client = Phase2DTestClient::new().await.expect("Failed to create test client");

    // 1. Get WalletConnect analytics
    let wc_analytics_request = client.create_user_request(GetSessionAnalyticsRequest {
        user_id: String::new(),
        start_date: (Utc::now() - chrono::Duration::days(30)).timestamp(),
        end_date: Utc::now().timestamp(),
        dapp_url: String::new(),
    });

    let wc_analytics_response = client.wallet_connect.get_session_analytics(wc_analytics_request).await;
    assert!(wc_analytics_response.is_ok(), "WalletConnect analytics should succeed");

    // 2. Get DApp signing analytics
    let signing_analytics_request = client.create_user_request(GetSigningAnalyticsRequest {
        user_id: String::new(),
        start_date: (Utc::now() - chrono::Duration::days(30)).timestamp(),
        end_date: Utc::now().timestamp(),
        chain_id: String::new(),
        dapp_url: String::new(),
    });

    let signing_analytics_response = client.dapp_signing.get_signing_analytics(signing_analytics_request).await;
    assert!(signing_analytics_response.is_ok(), "DApp signing analytics should succeed");

    // 3. Get earn analytics
    let earn_analytics_request = client.create_user_request(GetEarnAnalyticsRequest {
        user_id: String::new(),
        start_date: (Utc::now() - chrono::Duration::days(30)).timestamp(),
        end_date: Utc::now().timestamp(),
    });

    let earn_analytics_response = client.earn.get_earn_analytics(earn_analytics_request).await;
    assert!(earn_analytics_response.is_ok(), "Earn analytics should succeed");

    println!("✅ Cross-service analytics test passed");
}

#[tokio::test]
async fn test_security_and_rate_limiting() {
    println!("🔒 Testing Security and Rate Limiting");

    let (_server_url, _handle) = start_test_server().await;
    let mut client = Phase2DTestClient::new().await.expect("Failed to create test client");

    // Test authentication requirement
    let unauth_request = Request::new(GetYieldProductsRequest {
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
        page_size: 10,
        page_token: String::new(),
    });

    let unauth_response = client.earn.get_yield_products(unauth_request).await;
    assert!(unauth_response.is_err(), "Should require authentication");
    assert_eq!(unauth_response.unwrap_err().code(), tonic::Code::Unauthenticated);

    // Test rate limiting by making multiple rapid requests
    let mut success_count = 0;
    let mut rate_limited_count = 0;

    for _i in 0..20 {
        let request = client.create_user_request(ListSessionsRequest {
            status: 0,
            dapp_url: String::new(),
            page_size: 10,
            page_token: String::new(),
        });

        let response = client.wallet_connect.list_sessions(request).await;
        match response {
            Ok(_) => success_count += 1,
            Err(status) if status.code() == tonic::Code::ResourceExhausted => rate_limited_count += 1,
            Err(_) => {} // Other errors
        }
    }

    println!("Success: {}, Rate limited: {}", success_count, rate_limited_count);
    assert!(success_count > 0, "Some requests should succeed");

    println!("✅ Security and rate limiting test passed");
}

#[tokio::test]
async fn test_error_handling() {
    println!("⚠️ Testing Error Handling");

    let (_server_url, _handle) = start_test_server().await;
    let mut client = Phase2DTestClient::new().await.expect("Failed to create test client");

    // Test invalid UUID
    let invalid_request = client.create_user_request(GetSessionRequest {
        session_id: "invalid-uuid".to_string(),
    });

    let invalid_response = client.wallet_connect.get_session(invalid_request).await;
    assert!(invalid_response.is_err());
    assert_eq!(invalid_response.unwrap_err().code(), tonic::Code::InvalidArgument);

    // Test non-existent resource
    let not_found_request = client.create_user_request(GetSigningRequestRequest {
        request_id: Uuid::new_v4().to_string(),
    });

    let not_found_response = client.dapp_signing.get_signing_request(not_found_request).await;
    assert!(not_found_response.is_err());
    assert_eq!(not_found_response.unwrap_err().code(), tonic::Code::NotFound);

    println!("✅ Error handling test passed");
}
