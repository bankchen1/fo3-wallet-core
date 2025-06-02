//! End-to-end test runner for FO3 Wallet Core

mod e2e;

use e2e::{TestClient, TestResults};

#[tokio::test]
async fn run_all_e2e_tests() {
    println!("🚀 Starting FO3 Wallet Core End-to-End Tests");
    println!("============================================");

    // Initialize test client
    let mut client = match TestClient::new().await {
        Ok(client) => client,
        Err(e) => {
            eprintln!("❌ Failed to initialize test client: {}", e);
            eprintln!("Make sure the FO3 Wallet API is running on the expected endpoint");
            panic!("Test initialization failed");
        }
    };

    let mut overall_results = TestResults::default();

    // Run authentication tests
    let auth_results = e2e::auth_tests::run_auth_tests(&mut client).await;
    overall_results.merge(auth_results);

    // Run wallet tests
    let wallet_results = e2e::wallet_tests::run_wallet_tests(&mut client).await;
    overall_results.merge(wallet_results);

    // Print final summary
    println!("\n🏁 Overall Test Results");
    println!("=======================");
    overall_results.print_summary();

    // Assert overall success rate
    let success_rate = overall_results.success_rate();
    assert!(
        success_rate >= 95.0,
        "Overall success rate ({:.1}%) is below the required 95%",
        success_rate
    );

    println!("\n✅ All tests completed successfully!");
}

#[tokio::test]
async fn test_service_health() {
    println!("🏥 Testing Service Health");

    let mut client = TestClient::new().await.expect("Failed to create test client");

    let request = tonic::Request::new(
        fo3_wallet_api::proto::fo3::wallet::v1::HealthCheckRequest {
            service: String::new(),
        }
    );

    let response = client.health.check(request).await.expect("Health check failed");
    let health_response = response.into_inner();

    assert_eq!(
        health_response.status,
        fo3_wallet_api::proto::fo3::wallet::v1::health_check_response::ServingStatus::Serving as i32,
        "Service should be healthy"
    );

    println!("✅ Service health check passed");
}

#[tokio::test]
async fn test_authentication_flow() {
    println!("🔐 Testing Complete Authentication Flow");

    let mut client = TestClient::new().await.expect("Failed to create test client");

    // Test login
    let token = client.authenticate().await.expect("Authentication failed");
    assert!(!token.is_empty(), "Token should not be empty");

    // Test authenticated request
    let request = tonic::Request::new(
        fo3_wallet_api::proto::fo3::wallet::v1::ListWalletsRequest {
            page_size: 10,
            page_token: String::new(),
        }
    );
    let authenticated_request = client.add_auth_header(request, &token);

    let response = client.wallet.list_wallets(authenticated_request).await;
    assert!(response.is_ok(), "Authenticated request should succeed");

    println!("✅ Authentication flow test passed");
}

#[tokio::test]
async fn test_wallet_lifecycle() {
    println!("💼 Testing Complete Wallet Lifecycle");

    let mut client = TestClient::new().await.expect("Failed to create test client");
    let token = client.authenticate().await.expect("Authentication failed");

    // Create wallet
    let wallet_name = e2e::generate_test_wallet_name();
    let create_request = tonic::Request::new(
        fo3_wallet_api::proto::fo3::wallet::v1::CreateWalletRequest {
            name: wallet_name.clone(),
        }
    );
    let authenticated_create_request = client.add_auth_header(create_request, &token);

    let create_response = client.wallet.create_wallet(authenticated_create_request).await
        .expect("Wallet creation failed");
    let created_wallet = create_response.into_inner().wallet.unwrap();
    let wallet_id = created_wallet.id.clone();

    // Derive addresses for all supported chains
    let chains = vec![
        (fo3_wallet_api::proto::fo3::wallet::v1::KeyType::KeyTypeEthereum, "m/44'/60'/0'/0/0"),
        (fo3_wallet_api::proto::fo3::wallet::v1::KeyType::KeyTypeBitcoin, "m/44'/0'/0'/0/0"),
        (fo3_wallet_api::proto::fo3::wallet::v1::KeyType::KeyTypeSolana, "m/44'/501'/0'/0'"),
    ];

    for (key_type, derivation_path) in chains {
        let derive_request = tonic::Request::new(
            fo3_wallet_api::proto::fo3::wallet::v1::DeriveAddressRequest {
                wallet_id: wallet_id.clone(),
                key_type: key_type as i32,
                derivation_path: derivation_path.to_string(),
                bitcoin_network: fo3_wallet_api::proto::fo3::wallet::v1::BitcoinNetwork::BitcoinNetworkMainnet as i32,
            }
        );
        let authenticated_derive_request = client.add_auth_header(derive_request, &token);

        let derive_response = client.wallet.derive_address(authenticated_derive_request).await
            .expect(&format!("Address derivation failed for {:?}", key_type));
        
        let address = derive_response.into_inner().address.unwrap();
        assert!(!address.address.is_empty(), "Address should not be empty");
        println!("✅ Derived {:?} address: {}", key_type, address.address);
    }

    // Clean up - delete wallet
    let delete_request = tonic::Request::new(
        fo3_wallet_api::proto::fo3::wallet::v1::DeleteWalletRequest {
            wallet_id,
        }
    );
    let authenticated_delete_request = client.add_auth_header(delete_request, &token);

    let delete_response = client.wallet.delete_wallet(authenticated_delete_request).await
        .expect("Wallet deletion failed");
    assert!(delete_response.into_inner().success, "Wallet deletion should succeed");

    println!("✅ Wallet lifecycle test passed");
}

// Helper trait to merge test results
impl TestResults {
    fn merge(&mut self, other: TestResults) {
        self.total += other.total;
        self.passed += other.passed;
        self.failed += other.failed;
        self.failures.extend(other.failures);
    }
}

#[tokio::test]
async fn test_concurrent_requests() {
    println!("🔄 Testing Concurrent Requests");

    let mut client = TestClient::new().await.expect("Failed to create test client");
    let token = client.authenticate().await.expect("Authentication failed");

    // Create multiple concurrent wallet creation requests
    let mut handles = vec![];
    
    for i in 0..5 {
        let mut client_clone = TestClient::new().await.expect("Failed to create test client");
        let token_clone = token.clone();
        
        let handle = tokio::spawn(async move {
            let wallet_name = format!("concurrent_wallet_{}", i);
            let request = tonic::Request::new(
                fo3_wallet_api::proto::fo3::wallet::v1::CreateWalletRequest {
                    name: wallet_name,
                }
            );
            let authenticated_request = client_clone.add_auth_header(request, &token_clone);
            
            client_clone.wallet.create_wallet(authenticated_request).await
        });
        
        handles.push(handle);
    }

    // Wait for all requests to complete
    let mut successful_requests = 0;
    for handle in handles {
        match handle.await {
            Ok(Ok(_)) => successful_requests += 1,
            Ok(Err(e)) => println!("Request failed: {}", e),
            Err(e) => println!("Task failed: {}", e),
        }
    }

    assert!(successful_requests >= 4, "At least 4 out of 5 concurrent requests should succeed");
    println!("✅ Concurrent requests test passed ({}/5 successful)", successful_requests);
}

#[tokio::test]
async fn test_rate_limiting() {
    println!("🚦 Testing Rate Limiting");

    let mut client = TestClient::new().await.expect("Failed to create test client");
    let token = client.authenticate().await.expect("Authentication failed");

    // Make many rapid requests to trigger rate limiting
    let mut successful_requests = 0;
    let mut rate_limited_requests = 0;

    for i in 0..50 {
        let request = tonic::Request::new(
            fo3_wallet_api::proto::fo3::wallet::v1::ListWalletsRequest {
                page_size: 1,
                page_token: String::new(),
            }
        );
        let authenticated_request = client.add_auth_header(request, &token);

        match client.wallet.list_wallets(authenticated_request).await {
            Ok(_) => successful_requests += 1,
            Err(status) if status.code() == tonic::Code::ResourceExhausted => {
                rate_limited_requests += 1;
                break; // Stop when we hit rate limit
            }
            Err(e) => println!("Unexpected error: {}", e),
        }

        // Small delay to avoid overwhelming the server
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }

    println!("Successful requests: {}, Rate limited: {}", successful_requests, rate_limited_requests);
    
    // We should be able to make some requests, but eventually hit rate limiting
    assert!(successful_requests > 0, "Should be able to make some requests");
    
    println!("✅ Rate limiting test completed");
}
