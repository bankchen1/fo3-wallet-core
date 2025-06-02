//! Authentication and authorization tests

use super::{TestClient, TestResults, run_test};
use fo3_wallet_api::proto::fo3::wallet::v1::{
    LoginRequest, CreateApiKeyRequest, Permission, RateLimit,
};

pub async fn run_auth_tests(client: &mut TestClient) -> TestResults {
    let mut results = TestResults::default();
    
    println!("🔐 Running Authentication Tests...");

    // Test 1: Valid login
    run_test!(results, "Valid Login", test_valid_login(client));

    // Test 2: Invalid login
    run_test!(results, "Invalid Login", test_invalid_login(client));

    // Test 3: JWT token validation
    run_test!(results, "JWT Token Validation", test_jwt_validation(client));

    // Test 4: API key creation
    run_test!(results, "API Key Creation", test_api_key_creation(client));

    // Test 5: API key authentication
    run_test!(results, "API Key Authentication", test_api_key_auth(client));

    // Test 6: Token refresh
    run_test!(results, "Token Refresh", test_token_refresh(client));

    // Test 7: Logout
    run_test!(results, "Logout", test_logout(client));

    results
}

async fn test_valid_login(client: &mut TestClient) -> Result<(), Box<dyn std::error::Error>> {
    let request = tonic::Request::new(LoginRequest {
        username: client.config.admin_username.clone(),
        password: client.config.admin_password.clone(),
        remember_me: false,
    });

    let response = client.auth.login(request).await?;
    let login_response = response.into_inner();

    assert!(!login_response.access_token.is_empty(), "Access token should not be empty");
    assert!(!login_response.refresh_token.is_empty(), "Refresh token should not be empty");
    assert!(login_response.expires_at > 0, "Expiration time should be set");
    assert!(login_response.user.is_some(), "User info should be present");

    Ok(())
}

async fn test_invalid_login(client: &mut TestClient) -> Result<(), Box<dyn std::error::Error>> {
    let request = tonic::Request::new(LoginRequest {
        username: "invalid_user".to_string(),
        password: "wrong_password".to_string(),
        remember_me: false,
    });

    let result = client.auth.login(request).await;
    
    match result {
        Err(status) => {
            assert_eq!(status.code(), tonic::Code::Unauthenticated);
            Ok(())
        }
        Ok(_) => Err("Login should have failed with invalid credentials".into()),
    }
}

async fn test_jwt_validation(client: &mut TestClient) -> Result<(), Box<dyn std::error::Error>> {
    // First, get a valid token
    let token = client.authenticate().await?;

    // Test accessing a protected endpoint with valid token
    let request = tonic::Request::new(
        fo3_wallet_api::proto::fo3::wallet::v1::ListWalletsRequest {
            page_size: 10,
            page_token: String::new(),
        }
    );
    let authenticated_request = client.add_auth_header(request, &token);

    let response = client.wallet.list_wallets(authenticated_request).await?;
    assert!(response.into_inner().wallets.len() >= 0); // Should not error

    // Test accessing with invalid token
    let request = tonic::Request::new(
        fo3_wallet_api::proto::fo3::wallet::v1::ListWalletsRequest {
            page_size: 10,
            page_token: String::new(),
        }
    );
    let invalid_request = client.add_auth_header(request, "invalid_token");

    let result = client.wallet.list_wallets(invalid_request).await;
    match result {
        Err(status) => {
            assert_eq!(status.code(), tonic::Code::Unauthenticated);
            Ok(())
        }
        Ok(_) => Err("Request should have failed with invalid token".into()),
    }
}

async fn test_api_key_creation(client: &mut TestClient) -> Result<(), Box<dyn std::error::Error>> {
    // Authenticate first
    let token = client.authenticate().await?;

    let request = tonic::Request::new(CreateApiKeyRequest {
        name: "test_api_key".to_string(),
        permissions: vec![
            Permission::PermissionWalletRead as i32,
            Permission::PermissionWalletWrite as i32,
        ],
        rate_limit: Some(RateLimit {
            requests_per_minute: 100,
            burst_limit: 20,
            daily_limit: 5000,
        }),
        expires_at: 0, // No expiration
    });
    let authenticated_request = client.add_auth_header(request, &token);

    let response = client.auth.create_api_key(authenticated_request).await?;
    let api_key_response = response.into_inner();

    assert!(api_key_response.api_key.is_some(), "API key should be present");
    assert!(!api_key_response.secret_key.is_empty(), "Secret key should not be empty");
    assert!(api_key_response.secret_key.starts_with("fo3_"), "Secret key should have correct prefix");

    Ok(())
}

async fn test_api_key_auth(client: &mut TestClient) -> Result<(), Box<dyn std::error::Error>> {
    // First create an API key
    let token = client.authenticate().await?;

    let request = tonic::Request::new(CreateApiKeyRequest {
        name: "test_auth_key".to_string(),
        permissions: vec![Permission::PermissionWalletRead as i32],
        rate_limit: Some(RateLimit {
            requests_per_minute: 100,
            burst_limit: 20,
            daily_limit: 5000,
        }),
        expires_at: 0,
    });
    let authenticated_request = client.add_auth_header(request, &token);

    let response = client.auth.create_api_key(authenticated_request).await?;
    let api_key_response = response.into_inner();
    let api_key = api_key_response.secret_key;

    // Test using API key for authentication
    let mut request = tonic::Request::new(
        fo3_wallet_api::proto::fo3::wallet::v1::ListWalletsRequest {
            page_size: 10,
            page_token: String::new(),
        }
    );
    request.metadata_mut().insert("x-api-key", api_key.parse().unwrap());

    let response = client.wallet.list_wallets(request).await?;
    assert!(response.into_inner().wallets.len() >= 0); // Should not error

    Ok(())
}

async fn test_token_refresh(client: &mut TestClient) -> Result<(), Box<dyn std::error::Error>> {
    // Login to get refresh token
    let request = tonic::Request::new(LoginRequest {
        username: client.config.admin_username.clone(),
        password: client.config.admin_password.clone(),
        remember_me: false,
    });

    let response = client.auth.login(request).await?;
    let login_response = response.into_inner();
    let refresh_token = login_response.refresh_token;

    // Use refresh token to get new access token
    let request = tonic::Request::new(
        fo3_wallet_api::proto::fo3::wallet::v1::RefreshTokenRequest {
            refresh_token,
        }
    );

    let response = client.auth.refresh_token(request).await?;
    let refresh_response = response.into_inner();

    assert!(!refresh_response.access_token.is_empty(), "New access token should not be empty");
    assert!(!refresh_response.refresh_token.is_empty(), "New refresh token should not be empty");
    assert!(refresh_response.expires_at > 0, "Expiration time should be set");

    Ok(())
}

async fn test_logout(client: &mut TestClient) -> Result<(), Box<dyn std::error::Error>> {
    // Login first
    let request = tonic::Request::new(LoginRequest {
        username: client.config.admin_username.clone(),
        password: client.config.admin_password.clone(),
        remember_me: false,
    });

    let response = client.auth.login(request).await?;
    let login_response = response.into_inner();
    let refresh_token = login_response.refresh_token;

    // Logout
    let request = tonic::Request::new(
        fo3_wallet_api::proto::fo3::wallet::v1::LogoutRequest {
            refresh_token,
        }
    );

    let response = client.auth.logout(request).await?;
    let logout_response = response.into_inner();

    assert!(logout_response.success, "Logout should be successful");

    Ok(())
}
