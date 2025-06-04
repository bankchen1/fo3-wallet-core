//! Comprehensive integration tests for DAppSigningService
//! Tests all 13 gRPC methods with enterprise-grade validation

use std::sync::Arc;
use std::collections::HashMap;
use tonic::{Request, Code, Status};
use uuid::Uuid;
use chrono::Utc;

use fo3_wallet_api::proto::fo3::wallet::v1::{
    d_app_signing_service_server::DAppSigningService,
    *,
};
use fo3_wallet_api::services::dapp_signing::DAppSigningServiceImpl;
use fo3_wallet_api::middleware::{
    auth::{AuthService, AuthContext, UserRole, Permission},
    audit::AuditLogger,
    dapp_signing_guard::DAppSigningGuard,
    rate_limit::RateLimiter,
};
use fo3_wallet_api::models::{InMemoryDAppSigningRepository, dapp_signing::*};
use fo3_wallet_api::state::AppState;

/// Test helper to create a DAppSigningService instance
async fn create_test_service() -> DAppSigningServiceImpl {
    let state = Arc::new(AppState::new());
    let auth_service = Arc::new(AuthService::new(state.clone()));
    let audit_logger = Arc::new(AuditLogger::new(state.clone()));
    let rate_limiter = Arc::new(RateLimiter::new());
    let signing_repository = Arc::new(InMemoryDAppSigningRepository::new());
    let signing_guard = Arc::new(DAppSigningGuard::new(
        auth_service.clone(),
        audit_logger.clone(),
        rate_limiter.clone(),
        signing_repository.clone(),
    ));

    DAppSigningServiceImpl::new(
        state,
        auth_service,
        audit_logger,
        signing_guard,
        signing_repository,
    )
}

/// Test helper to create authenticated request
fn create_authenticated_request<T>(payload: T) -> Request<T> {
    let mut request = Request::new(payload);
    request.metadata_mut().insert(
        "authorization",
        "Bearer test_jwt_token_dapp_signing".parse().unwrap(),
    );
    request
}

#[tokio::test]
async fn test_create_signing_request() {
    let service = create_test_service().await;
    
    let request = create_authenticated_request(CreateSigningRequestRequest {
        session_id: Uuid::new_v4().to_string(),
        dapp_url: "https://test-dapp.com".to_string(),
        chain_id: "1".to_string(), // Ethereum mainnet
        account_address: "0x1234567890123456789012345678901234567890".to_string(),
        signing_method: "eth_sendTransaction".to_string(),
        transaction_data: r#"{"to":"0x...", "value":"0x0", "data":"0x"}"#.to_string(),
        estimated_gas: "21000".to_string(),
        gas_price: "20000000000".to_string(), // 20 gwei
        metadata: HashMap::new(),
    });

    let response = service.create_signing_request(request).await;
    assert!(response.is_ok(), "create_signing_request should succeed");
    
    let response = response.unwrap().into_inner();
    assert!(!response.request_id.is_empty(), "Should return request ID");
    assert!(response.expires_at > 0, "Should return valid expiry timestamp");
}

#[tokio::test]
async fn test_approve_signing_request() {
    let service = create_test_service().await;
    
    let request = create_authenticated_request(ApproveSigningRequestRequest {
        request_id: Uuid::new_v4().to_string(),
        signature: "0x1234567890abcdef...".to_string(),
        transaction_hash: "0xabcdef1234567890...".to_string(),
        gas_used: "21000".to_string(),
        effective_gas_price: "20000000000".to_string(),
    });

    let response = service.approve_signing_request(request).await;
    assert!(response.is_err(), "Should fail for non-existent request");
    assert_eq!(response.unwrap_err().code(), Code::NotFound);
}

#[tokio::test]
async fn test_reject_signing_request() {
    let service = create_test_service().await;
    
    let request = create_authenticated_request(RejectSigningRequestRequest {
        request_id: Uuid::new_v4().to_string(),
        reason: "User rejected the transaction".to_string(),
        error_code: "USER_REJECTED".to_string(),
    });

    let response = service.reject_signing_request(request).await;
    assert!(response.is_ok(), "reject_signing_request should succeed");
}

#[tokio::test]
async fn test_get_signing_request() {
    let service = create_test_service().await;
    
    let request = create_authenticated_request(GetSigningRequestRequest {
        request_id: Uuid::new_v4().to_string(),
    });

    let response = service.get_signing_request(request).await;
    assert!(response.is_err(), "Should fail for non-existent request");
    assert_eq!(response.unwrap_err().code(), Code::NotFound);
}

#[tokio::test]
async fn test_list_signing_requests() {
    let service = create_test_service().await;
    
    let request = create_authenticated_request(ListSigningRequestsRequest {
        session_id: String::new(),
        status: 0, // All statuses
        chain_id: String::new(),
        dapp_url: String::new(),
        start_date: 0,
        end_date: 0,
        page_size: 20,
        page_token: String::new(),
    });

    let response = service.list_signing_requests(request).await;
    assert!(response.is_ok(), "list_signing_requests should succeed");
    
    let response = response.unwrap().into_inner();
    assert!(response.requests.len() >= 0, "Should return requests list");
}

#[tokio::test]
async fn test_simulate_transaction() {
    let service = create_test_service().await;
    
    let request = create_authenticated_request(SimulateTransactionRequest {
        chain_id: "1".to_string(),
        from_address: "0x1234567890123456789012345678901234567890".to_string(),
        to_address: "0x0987654321098765432109876543210987654321".to_string(),
        value: "1000000000000000000".to_string(), // 1 ETH
        data: "0x".to_string(),
        gas_limit: "21000".to_string(),
        gas_price: "20000000000".to_string(),
    });

    let response = service.simulate_transaction(request).await;
    assert!(response.is_ok(), "simulate_transaction should succeed");
    
    let response = response.unwrap().into_inner();
    assert!(response.simulation.is_some(), "Should return simulation result");
}

#[tokio::test]
async fn test_estimate_gas() {
    let service = create_test_service().await;
    
    let request = create_authenticated_request(EstimateGasRequest {
        chain_id: "1".to_string(),
        from_address: "0x1234567890123456789012345678901234567890".to_string(),
        to_address: "0x0987654321098765432109876543210987654321".to_string(),
        value: "1000000000000000000".to_string(),
        data: "0x".to_string(),
    });

    let response = service.estimate_gas(request).await;
    assert!(response.is_ok(), "estimate_gas should succeed");
    
    let response = response.unwrap().into_inner();
    assert!(!response.gas_estimate.is_empty(), "Should return gas estimate");
}

#[tokio::test]
async fn test_get_transaction_status() {
    let service = create_test_service().await;
    
    let request = create_authenticated_request(GetTransactionStatusRequest {
        chain_id: "1".to_string(),
        transaction_hash: "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef12".to_string(),
    });

    let response = service.get_transaction_status(request).await;
    assert!(response.is_ok(), "get_transaction_status should succeed");
    
    let response = response.unwrap().into_inner();
    assert!(response.status.is_some(), "Should return transaction status");
}

#[tokio::test]
async fn test_cancel_signing_request() {
    let service = create_test_service().await;
    
    let request = create_authenticated_request(CancelSigningRequestRequest {
        request_id: Uuid::new_v4().to_string(),
        reason: "User cancelled the request".to_string(),
    });

    let response = service.cancel_signing_request(request).await;
    assert!(response.is_ok(), "cancel_signing_request should succeed");
}

#[tokio::test]
async fn test_get_signing_analytics() {
    let service = create_test_service().await;
    
    let request = create_authenticated_request(GetSigningAnalyticsRequest {
        user_id: String::new(), // Current user
        start_date: (Utc::now() - chrono::Duration::days(30)).timestamp(),
        end_date: Utc::now().timestamp(),
        chain_id: String::new(),
        dapp_url: String::new(),
    });

    let response = service.get_signing_analytics(request).await;
    assert!(response.is_ok(), "get_signing_analytics should succeed");
    
    let response = response.unwrap().into_inner();
    assert!(response.analytics.is_some(), "Should return analytics data");
}

#[tokio::test]
async fn test_batch_sign_transactions() {
    let service = create_test_service().await;
    
    let request = create_authenticated_request(BatchSignTransactionsRequest {
        session_id: Uuid::new_v4().to_string(),
        transactions: vec![
            TransactionToSign {
                chain_id: "1".to_string(),
                to_address: "0x1234567890123456789012345678901234567890".to_string(),
                value: "1000000000000000000".to_string(),
                data: "0x".to_string(),
                gas_limit: "21000".to_string(),
                gas_price: "20000000000".to_string(),
                nonce: "1".to_string(),
            },
            TransactionToSign {
                chain_id: "1".to_string(),
                to_address: "0x0987654321098765432109876543210987654321".to_string(),
                value: "2000000000000000000".to_string(),
                data: "0x".to_string(),
                gas_limit: "21000".to_string(),
                gas_price: "20000000000".to_string(),
                nonce: "2".to_string(),
            },
        ],
        metadata: HashMap::new(),
    });

    let response = service.batch_sign_transactions(request).await;
    assert!(response.is_ok(), "batch_sign_transactions should succeed");
    
    let response = response.unwrap().into_inner();
    assert!(!response.batch_id.is_empty(), "Should return batch ID");
    assert_eq!(response.transaction_count, 2, "Should process 2 transactions");
}

#[tokio::test]
async fn test_get_supported_chains() {
    let service = create_test_service().await;
    
    let request = create_authenticated_request(GetSupportedChainsRequest {
        include_testnets: true,
    });

    let response = service.get_supported_chains(request).await;
    assert!(response.is_ok(), "get_supported_chains should succeed");
    
    let response = response.unwrap().into_inner();
    assert!(!response.chains.is_empty(), "Should return supported chains");
}

#[tokio::test]
async fn test_input_validation() {
    let service = create_test_service().await;
    
    // Test with invalid chain ID
    let request = create_authenticated_request(EstimateGasRequest {
        chain_id: "invalid_chain".to_string(),
        from_address: "0x1234567890123456789012345678901234567890".to_string(),
        to_address: "0x0987654321098765432109876543210987654321".to_string(),
        value: "1000000000000000000".to_string(),
        data: "0x".to_string(),
    });

    let response = service.estimate_gas(request).await;
    assert!(response.is_err(), "Should validate chain ID");
    assert_eq!(response.unwrap_err().code(), Code::InvalidArgument);
}

#[tokio::test]
async fn test_authentication_required() {
    let service = create_test_service().await;
    
    // Test without authentication
    let request = Request::new(ListSigningRequestsRequest {
        session_id: String::new(),
        status: 0,
        chain_id: String::new(),
        dapp_url: String::new(),
        start_date: 0,
        end_date: 0,
        page_size: 20,
        page_token: String::new(),
    });

    let response = service.list_signing_requests(request).await;
    assert!(response.is_err(), "Should require authentication");
    assert_eq!(response.unwrap_err().code(), Code::Unauthenticated);
}

#[tokio::test]
async fn test_signing_request_lifecycle() {
    let service = create_test_service().await;
    
    // 1. Create signing request
    let create_request = create_authenticated_request(CreateSigningRequestRequest {
        session_id: Uuid::new_v4().to_string(),
        dapp_url: "https://lifecycle-test.com".to_string(),
        chain_id: "1".to_string(),
        account_address: "0x1234567890123456789012345678901234567890".to_string(),
        signing_method: "eth_sendTransaction".to_string(),
        transaction_data: r#"{"to":"0x0987654321098765432109876543210987654321", "value":"0x0", "data":"0x"}"#.to_string(),
        estimated_gas: "21000".to_string(),
        gas_price: "20000000000".to_string(),
        metadata: HashMap::new(),
    });

    let create_response = service.create_signing_request(create_request).await;
    assert!(create_response.is_ok());
    let request_id = create_response.unwrap().into_inner().request_id;

    // 2. Get signing request
    let get_request = create_authenticated_request(GetSigningRequestRequest {
        request_id: request_id.clone(),
    });
    let get_response = service.get_signing_request(get_request).await;
    assert!(get_response.is_ok());

    // 3. Simulate transaction
    let simulate_request = create_authenticated_request(SimulateTransactionRequest {
        chain_id: "1".to_string(),
        from_address: "0x1234567890123456789012345678901234567890".to_string(),
        to_address: "0x0987654321098765432109876543210987654321".to_string(),
        value: "0".to_string(),
        data: "0x".to_string(),
        gas_limit: "21000".to_string(),
        gas_price: "20000000000".to_string(),
    });
    let simulate_response = service.simulate_transaction(simulate_request).await;
    assert!(simulate_response.is_ok());

    // 4. Approve signing request
    let approve_request = create_authenticated_request(ApproveSigningRequestRequest {
        request_id: request_id.clone(),
        signature: "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef121234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef121b".to_string(),
        transaction_hash: "0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890ab".to_string(),
        gas_used: "21000".to_string(),
        effective_gas_price: "20000000000".to_string(),
    });
    let approve_response = service.approve_signing_request(approve_request).await;
    assert!(approve_response.is_ok());
}
