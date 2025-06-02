//! End-to-end tests for Fiat Gateway functionality

use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use uuid::Uuid;

use fo3_wallet_api::{
    proto::fo3::wallet::v1::{
        fiat_gateway_service_client::FiatGatewayServiceClient,
        auth_service_client::AuthServiceClient,
        *,
    },
    state::AppState,
    services::{
        fiat_gateway::FiatGatewayServiceImpl,
        auth::AuthServiceImpl,
    },
    middleware::{
        auth::AuthService,
        audit::AuditLogger,
        kyc_guard::KycGuard,
        fiat_guard::FiatGuard,
    },
};

use tonic::{transport::Server, Request, Response, Status};
use tonic::metadata::MetadataValue;

/// Test helper to create a test server
async fn create_test_server() -> (String, tokio::task::JoinHandle<()>) {
    let state = Arc::new(AppState::new());
    let auth_service = Arc::new(AuthService::new(state.clone()));
    let audit_logger = Arc::new(AuditLogger::new(state.clone()));
    let kyc_guard = Arc::new(KycGuard::new(state.clone(), auth_service.clone()));
    let fiat_guard = Arc::new(FiatGuard::new(state.clone(), auth_service.clone(), kyc_guard.clone()));

    let fiat_service = FiatGatewayServiceImpl::new(
        state.clone(),
        auth_service.clone(),
        audit_logger.clone(),
        fiat_guard.clone(),
    );
    let auth_grpc_service = AuthServiceImpl::new(auth_service.clone(), audit_logger.clone());

    let addr = "127.0.0.1:0".parse().unwrap();
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = Server::builder()
        .add_service(fiat_gateway_service_server::FiatGatewayServiceServer::new(fiat_service))
        .add_service(auth_service_server::AuthServiceServer::new(auth_grpc_service))
        .serve_with_incoming(tokio_stream::wrappers::TcpListenerStream::new(listener));

    let handle = tokio::spawn(server);
    let server_url = format!("http://{}", addr);

    // Give the server a moment to start
    sleep(Duration::from_millis(100)).await;

    (server_url, handle)
}

/// Test helper to authenticate and get JWT token
async fn authenticate_user(server_url: &str, username: &str, password: &str) -> String {
    let mut client = AuthServiceClient::connect(server_url.to_string()).await.unwrap();

    let request = Request::new(LoginRequest {
        username: username.to_string(),
        password: password.to_string(),
        remember_me: false,
    });

    let response = client.login(request).await.unwrap();
    response.into_inner().access_token
}

/// Test helper to create authenticated request
fn create_authenticated_request<T>(token: &str, message: T) -> Request<T> {
    let mut request = Request::new(message);
    let auth_header = MetadataValue::from_str(&format!("Bearer {}", token)).unwrap();
    request.metadata_mut().insert("authorization", auth_header);
    request
}

#[tokio::test]
async fn test_bank_account_binding_workflow() {
    let (server_url, _handle) = create_test_server().await;

    // Authenticate user
    let token = authenticate_user(&server_url, "testuser", "password123").await;
    let mut client = FiatGatewayServiceClient::connect(server_url).await.unwrap();

    // Test bank account binding
    let user_id = Uuid::new_v4().to_string();
    let bind_request = BindBankAccountRequest {
        user_id: user_id.clone(),
        provider: PaymentProvider::PaymentProviderAch as i32,
        account_type: AccountType::AccountTypeChecking as i32,
        account_name: "John Doe Checking".to_string(),
        account_number: "1234567890".to_string(),
        routing_number: "021000021".to_string(),
        bank_name: "Test Bank".to_string(),
        currency: "USD".to_string(),
        country: "US".to_string(),
        set_as_primary: true,
    };

    let request = create_authenticated_request(&token, bind_request);
    let response = client.bind_bank_account(request).await.unwrap();
    let bind_response = response.into_inner();

    assert!(bind_response.account.is_some());
    let account = bind_response.account.unwrap();
    assert_eq!(account.user_id, user_id);
    assert_eq!(account.account_name, "John Doe Checking");
    assert_eq!(account.masked_account_number, "****7890");
    assert!(!account.is_verified); // Should not be verified initially
    assert_eq!(bind_response.verification_method, "micro_deposits");
    assert_eq!(bind_response.verification_amounts.len(), 2);

    // Test getting bank accounts
    let get_request = GetBankAccountsRequest {
        user_id: user_id.clone(),
        verified_only: false,
    };

    let request = create_authenticated_request(&token, get_request);
    let response = client.get_bank_accounts(request).await.unwrap();
    let get_response = response.into_inner();

    assert_eq!(get_response.accounts.len(), 1);
    assert_eq!(get_response.accounts[0].id, account.id);
}

#[tokio::test]
async fn test_bank_account_verification() {
    let (server_url, _handle) = create_test_server().await;

    let token = authenticate_user(&server_url, "testuser", "password123").await;
    let mut client = FiatGatewayServiceClient::connect(server_url).await.unwrap();

    // First bind an account
    let user_id = Uuid::new_v4().to_string();
    let bind_request = BindBankAccountRequest {
        user_id: user_id.clone(),
        provider: PaymentProvider::PaymentProviderAch as i32,
        account_type: AccountType::AccountTypeChecking as i32,
        account_name: "Test Account".to_string(),
        account_number: "9876543210".to_string(),
        routing_number: "021000021".to_string(),
        bank_name: "Test Bank".to_string(),
        currency: "USD".to_string(),
        country: "US".to_string(),
        set_as_primary: false,
    };

    let request = create_authenticated_request(&token, bind_request);
    let response = client.bind_bank_account(request).await.unwrap();
    let account = response.into_inner().account.unwrap();

    // Test verification with correct amounts
    let verify_request = VerifyBankAccountRequest {
        user_id: user_id.clone(),
        account_id: account.id.clone(),
        verification_amounts: vec!["0.12".to_string(), "0.34".to_string()],
    };

    let request = create_authenticated_request(&token, verify_request);
    let response = client.verify_bank_account(request).await.unwrap();
    let verify_response = response.into_inner();

    assert!(verify_response.verified);
    assert_eq!(verify_response.message, "Bank account verified successfully");
    assert!(verify_response.account.unwrap().is_verified);

    // Test verification with incorrect amounts
    let verify_request_wrong = VerifyBankAccountRequest {
        user_id: user_id.clone(),
        account_id: account.id.clone(),
        verification_amounts: vec!["0.11".to_string(), "0.22".to_string()],
    };

    let request = create_authenticated_request(&token, verify_request_wrong);
    let response = client.verify_bank_account(request).await.unwrap();
    let verify_response = response.into_inner();

    assert!(!verify_response.verified);
    assert_eq!(verify_response.message, "Verification failed - incorrect amounts");
}

#[tokio::test]
async fn test_withdrawal_workflow() {
    let (server_url, _handle) = create_test_server().await;

    let token = authenticate_user(&server_url, "testuser", "password123").await;
    let mut client = FiatGatewayServiceClient::connect(server_url).await.unwrap();

    // First bind and verify an account
    let user_id = Uuid::new_v4().to_string();
    let bind_request = BindBankAccountRequest {
        user_id: user_id.clone(),
        provider: PaymentProvider::PaymentProviderAch as i32,
        account_type: AccountType::AccountTypeChecking as i32,
        account_name: "Withdrawal Test Account".to_string(),
        account_number: "1111222233".to_string(),
        routing_number: "021000021".to_string(),
        bank_name: "Test Bank".to_string(),
        currency: "USD".to_string(),
        country: "US".to_string(),
        set_as_primary: true,
    };

    let request = create_authenticated_request(&token, bind_request);
    let response = client.bind_bank_account(request).await.unwrap();
    let account = response.into_inner().account.unwrap();

    // Verify the account
    let verify_request = VerifyBankAccountRequest {
        user_id: user_id.clone(),
        account_id: account.id.clone(),
        verification_amounts: vec!["0.12".to_string(), "0.34".to_string()],
    };

    let request = create_authenticated_request(&token, verify_request);
    client.verify_bank_account(request).await.unwrap();

    // Submit withdrawal
    let withdrawal_request = SubmitWithdrawalRequest {
        user_id: user_id.clone(),
        bank_account_id: account.id.clone(),
        amount: "500.00".to_string(),
        currency: "USD".to_string(),
        description: "Test withdrawal".to_string(),
    };

    let request = create_authenticated_request(&token, withdrawal_request);
    let response = client.submit_withdrawal(request).await.unwrap();
    let withdrawal_response = response.into_inner();

    assert!(withdrawal_response.transaction.is_some());
    let transaction = withdrawal_response.transaction.unwrap();
    assert_eq!(transaction.user_id, user_id);
    assert_eq!(transaction.amount, "500.00");
    assert_eq!(transaction.currency, "USD");
    assert_eq!(transaction.r#type, TransactionType::TransactionTypeWithdrawal as i32);
    assert!(!transaction.reference_number.is_empty());

    // Test getting withdrawal status
    let status_request = GetWithdrawalStatusRequest {
        user_id: user_id.clone(),
        transaction_id: transaction.id.clone(),
    };

    let request = create_authenticated_request(&token, status_request);
    let response = client.get_withdrawal_status(request).await.unwrap();
    let status_response = response.into_inner();

    assert!(status_response.transaction.is_some());
    assert_eq!(status_response.transaction.unwrap().id, transaction.id);
}

#[tokio::test]
async fn test_withdrawal_validation_and_limits() {
    let (server_url, _handle) = create_test_server().await;

    let token = authenticate_user(&server_url, "testuser", "password123").await;
    let mut client = FiatGatewayServiceClient::connect(server_url).await.unwrap();

    let user_id = Uuid::new_v4().to_string();

    // Test withdrawal without verified account
    let withdrawal_request = SubmitWithdrawalRequest {
        user_id: user_id.clone(),
        bank_account_id: Uuid::new_v4().to_string(),
        amount: "100.00".to_string(),
        currency: "USD".to_string(),
        description: "Test".to_string(),
    };

    let request = create_authenticated_request(&token, withdrawal_request);
    let result = client.submit_withdrawal(request).await;

    // Should fail because bank account doesn't exist
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert_eq!(error.code(), tonic::Code::NotFound);

    // Test invalid amount
    let invalid_withdrawal = SubmitWithdrawalRequest {
        user_id: user_id.clone(),
        bank_account_id: Uuid::new_v4().to_string(),
        amount: "-100.00".to_string(),
        currency: "USD".to_string(),
        description: "Invalid amount".to_string(),
    };

    let request = create_authenticated_request(&token, invalid_withdrawal);
    let result = client.submit_withdrawal(request).await;

    assert!(result.is_err());
    let error = result.unwrap_err();
    assert_eq!(error.code(), tonic::Code::InvalidArgument);
}

#[tokio::test]
async fn test_cross_user_access_prevention() {
    let (server_url, _handle) = create_test_server().await;

    let user1_token = authenticate_user(&server_url, "user1", "password123").await;
    let user2_token = authenticate_user(&server_url, "user2", "password123").await;
    let mut client = FiatGatewayServiceClient::connect(server_url).await.unwrap();

    // User 1 binds an account
    let user1_id = Uuid::new_v4().to_string();
    let bind_request = BindBankAccountRequest {
        user_id: user1_id.clone(),
        provider: PaymentProvider::PaymentProviderAch as i32,
        account_type: AccountType::AccountTypeChecking as i32,
        account_name: "User1 Account".to_string(),
        account_number: "1111111111".to_string(),
        routing_number: "021000021".to_string(),
        bank_name: "Test Bank".to_string(),
        currency: "USD".to_string(),
        country: "US".to_string(),
        set_as_primary: false,
    };

    let request = create_authenticated_request(&user1_token, bind_request);
    let response = client.bind_bank_account(request).await.unwrap();
    let account = response.into_inner().account.unwrap();

    // User 2 tries to access User 1's accounts
    let get_request = GetBankAccountsRequest {
        user_id: user1_id.clone(),
        verified_only: false,
    };

    let request = create_authenticated_request(&user2_token, get_request);
    let result = client.get_bank_accounts(request).await;

    // Should fail with permission denied
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert_eq!(error.code(), tonic::Code::PermissionDenied);

    // User 2 tries to submit withdrawal for User 1
    let withdrawal_request = SubmitWithdrawalRequest {
        user_id: user1_id.clone(),
        bank_account_id: account.id.clone(),
        amount: "100.00".to_string(),
        currency: "USD".to_string(),
        description: "Unauthorized withdrawal".to_string(),
    };

    let request = create_authenticated_request(&user2_token, withdrawal_request);
    let result = client.submit_withdrawal(request).await;

    assert!(result.is_err());
    let error = result.unwrap_err();
    assert_eq!(error.code(), tonic::Code::PermissionDenied);
}

#[tokio::test]
async fn test_bank_account_removal() {
    let (server_url, _handle) = create_test_server().await;

    let token = authenticate_user(&server_url, "testuser", "password123").await;
    let mut client = FiatGatewayServiceClient::connect(server_url).await.unwrap();

    // Bind an account
    let user_id = Uuid::new_v4().to_string();
    let bind_request = BindBankAccountRequest {
        user_id: user_id.clone(),
        provider: PaymentProvider::PaymentProviderVisa as i32,
        account_type: AccountType::AccountTypeCreditCard as i32,
        account_name: "Test Card".to_string(),
        account_number: "4111111111111111".to_string(),
        routing_number: "".to_string(),
        bank_name: "Test Bank".to_string(),
        currency: "USD".to_string(),
        country: "US".to_string(),
        set_as_primary: false,
    };

    let request = create_authenticated_request(&token, bind_request);
    let response = client.bind_bank_account(request).await.unwrap();
    let account = response.into_inner().account.unwrap();

    // Remove the account
    let remove_request = RemoveBankAccountRequest {
        user_id: user_id.clone(),
        account_id: account.id.clone(),
    };

    let request = create_authenticated_request(&token, remove_request);
    let response = client.remove_bank_account(request).await.unwrap();
    let remove_response = response.into_inner();

    assert!(remove_response.success);
    assert_eq!(remove_response.message, "Bank account removed successfully");

    // Verify account is no longer accessible
    let get_request = GetBankAccountsRequest {
        user_id: user_id.clone(),
        verified_only: false,
    };

    let request = create_authenticated_request(&token, get_request);
    let response = client.get_bank_accounts(request).await.unwrap();
    let get_response = response.into_inner();

    // Should return empty list (account is soft deleted)
    assert_eq!(get_response.accounts.len(), 0);
}
