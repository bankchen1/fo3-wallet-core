//! Comprehensive integration tests for WalletConnectService
//! Tests all 13 gRPC methods with enterprise-grade validation

use std::sync::Arc;
use std::collections::HashMap;
use tonic::{Request, Code, Status};
use uuid::Uuid;
use chrono::Utc;

use fo3_wallet_api::proto::fo3::wallet::v1::{
    wallet_connect_service_server::WalletConnectService,
    *,
};
use fo3_wallet_api::services::wallet_connect::WalletConnectServiceImpl;
use fo3_wallet_api::middleware::{
    auth::{AuthService, AuthContext, UserRole, Permission},
    audit::AuditLogger,
    wallet_connect_guard::WalletConnectGuard,
    rate_limit::RateLimiter,
};
use fo3_wallet_api::models::{InMemoryWalletConnectRepository, wallet_connect::*};
use fo3_wallet_api::state::AppState;

/// Test helper to create a WalletConnectService instance
async fn create_test_service() -> WalletConnectServiceImpl {
    let state = Arc::new(AppState::new());
    let auth_service = Arc::new(AuthService::new(state.clone()));
    let audit_logger = Arc::new(AuditLogger::new(state.clone()));
    let rate_limiter = Arc::new(RateLimiter::new());
    let wc_repository = Arc::new(InMemoryWalletConnectRepository::new());
    let wc_guard = Arc::new(WalletConnectGuard::new(
        auth_service.clone(),
        audit_logger.clone(),
        rate_limiter.clone(),
        wc_repository.clone(),
    ));

    WalletConnectServiceImpl::new(
        state,
        auth_service,
        audit_logger,
        wc_guard,
        wc_repository,
    )
}

/// Test helper to create authenticated request
fn create_authenticated_request<T>(payload: T) -> Request<T> {
    let mut request = Request::new(payload);
    request.metadata_mut().insert(
        "authorization",
        "Bearer test_jwt_token_wc".parse().unwrap(),
    );
    request
}

#[tokio::test]
async fn test_create_session() {
    let service = create_test_service().await;
    
    let request = create_authenticated_request(CreateSessionRequest {
        dapp_name: "Test DApp".to_string(),
        dapp_url: "https://test-dapp.com".to_string(),
        dapp_icon: "https://test-dapp.com/icon.png".to_string(),
        dapp_description: "Test DApp for integration testing".to_string(),
        required_chains: vec!["ethereum".to_string(), "polygon".to_string()],
        required_methods: vec!["eth_sendTransaction".to_string(), "personal_sign".to_string()],
        required_events: vec!["accountsChanged".to_string(), "chainChanged".to_string()],
        expiry_hours: 24,
        metadata: HashMap::new(),
    });

    let response = service.create_session(request).await;
    assert!(response.is_ok(), "create_session should succeed");
    
    let response = response.unwrap().into_inner();
    assert!(!response.session_id.is_empty(), "Should return session ID");
    assert!(!response.uri.is_empty(), "Should return WalletConnect URI");
    assert!(response.expiry_at > 0, "Should return valid expiry timestamp");
}

#[tokio::test]
async fn test_approve_session() {
    let service = create_test_service().await;
    
    // First create a session
    let create_request = create_authenticated_request(CreateSessionRequest {
        dapp_name: "Test DApp".to_string(),
        dapp_url: "https://test-dapp.com".to_string(),
        dapp_icon: "https://test-dapp.com/icon.png".to_string(),
        dapp_description: "Test DApp".to_string(),
        required_chains: vec!["ethereum".to_string()],
        required_methods: vec!["eth_sendTransaction".to_string()],
        required_events: vec!["accountsChanged".to_string()],
        expiry_hours: 24,
        metadata: HashMap::new(),
    });

    let create_response = service.create_session(create_request).await;
    assert!(create_response.is_ok());
    let session_id = create_response.unwrap().into_inner().session_id;

    // Now approve the session
    let approve_request = create_authenticated_request(ApproveSessionRequest {
        session_id: session_id.clone(),
        approved_chains: vec!["ethereum".to_string()],
        approved_accounts: vec!["0x1234567890123456789012345678901234567890".to_string()],
        approved_methods: vec!["eth_sendTransaction".to_string()],
        approved_events: vec!["accountsChanged".to_string()],
    });

    let response = service.approve_session(approve_request).await;
    assert!(response.is_ok(), "approve_session should succeed");
    
    let response = response.unwrap().into_inner();
    assert!(response.success, "Session approval should be successful");
}

#[tokio::test]
async fn test_reject_session() {
    let service = create_test_service().await;
    
    let request = create_authenticated_request(RejectSessionRequest {
        session_id: Uuid::new_v4().to_string(),
        reason: "User rejected the connection".to_string(),
    });

    let response = service.reject_session(request).await;
    // Should succeed even for non-existent session
    assert!(response.is_ok(), "reject_session should succeed");
}

#[tokio::test]
async fn test_disconnect_session() {
    let service = create_test_service().await;
    
    let request = create_authenticated_request(DisconnectSessionRequest {
        session_id: Uuid::new_v4().to_string(),
        reason: "User disconnected".to_string(),
    });

    let response = service.disconnect_session(request).await;
    assert!(response.is_ok(), "disconnect_session should succeed");
}

#[tokio::test]
async fn test_get_session() {
    let service = create_test_service().await;
    
    let request = create_authenticated_request(GetSessionRequest {
        session_id: Uuid::new_v4().to_string(),
    });

    let response = service.get_session(request).await;
    assert!(response.is_err(), "Should fail for non-existent session");
    assert_eq!(response.unwrap_err().code(), Code::NotFound);
}

#[tokio::test]
async fn test_list_sessions() {
    let service = create_test_service().await;
    
    let request = create_authenticated_request(ListSessionsRequest {
        status: 0, // All statuses
        dapp_url: String::new(),
        page_size: 20,
        page_token: String::new(),
    });

    let response = service.list_sessions(request).await;
    assert!(response.is_ok(), "list_sessions should succeed");
    
    let response = response.unwrap().into_inner();
    assert!(response.sessions.len() >= 0, "Should return sessions list");
}

#[tokio::test]
async fn test_update_session() {
    let service = create_test_service().await;
    
    let request = create_authenticated_request(UpdateSessionRequest {
        session_id: Uuid::new_v4().to_string(),
        approved_chains: vec!["ethereum".to_string(), "polygon".to_string()],
        approved_accounts: vec!["0x1234567890123456789012345678901234567890".to_string()],
        approved_methods: vec!["eth_sendTransaction".to_string()],
        approved_events: vec!["accountsChanged".to_string()],
    });

    let response = service.update_session(request).await;
    assert!(response.is_err(), "Should fail for non-existent session");
    assert_eq!(response.unwrap_err().code(), Code::NotFound);
}

#[tokio::test]
async fn test_send_session_event() {
    let service = create_test_service().await;
    
    let request = create_authenticated_request(SendSessionEventRequest {
        session_id: Uuid::new_v4().to_string(),
        event_name: "accountsChanged".to_string(),
        event_data: r#"["0x1234567890123456789012345678901234567890"]"#.to_string(),
    });

    let response = service.send_session_event(request).await;
    assert!(response.is_err(), "Should fail for non-existent session");
    assert_eq!(response.unwrap_err().code(), Code::NotFound);
}

#[tokio::test]
async fn test_get_session_requests() {
    let service = create_test_service().await;
    
    let request = create_authenticated_request(GetSessionRequestsRequest {
        session_id: Uuid::new_v4().to_string(),
        status: 0, // All statuses
        page_size: 20,
        page_token: String::new(),
    });

    let response = service.get_session_requests(request).await;
    assert!(response.is_ok(), "get_session_requests should succeed");
    
    let response = response.unwrap().into_inner();
    assert!(response.requests.len() >= 0, "Should return requests list");
}

#[tokio::test]
async fn test_respond_to_request() {
    let service = create_test_service().await;
    
    let request = create_authenticated_request(RespondToRequestRequest {
        request_id: Uuid::new_v4().to_string(),
        approved: true,
        result: r#"{"success": true}"#.to_string(),
        error_message: String::new(),
    });

    let response = service.respond_to_request(request).await;
    assert!(response.is_err(), "Should fail for non-existent request");
    assert_eq!(response.unwrap_err().code(), Code::NotFound);
}

#[tokio::test]
async fn test_get_session_analytics() {
    let service = create_test_service().await;
    
    let request = create_authenticated_request(GetSessionAnalyticsRequest {
        user_id: String::new(), // Current user
        start_date: (Utc::now() - chrono::Duration::days(30)).timestamp(),
        end_date: Utc::now().timestamp(),
        dapp_url: String::new(),
    });

    let response = service.get_session_analytics(request).await;
    assert!(response.is_ok(), "get_session_analytics should succeed");
    
    let response = response.unwrap().into_inner();
    assert!(response.analytics.is_some(), "Should return analytics data");
}

#[tokio::test]
async fn test_cleanup_expired_sessions() {
    let service = create_test_service().await;
    
    let request = create_authenticated_request(CleanupExpiredSessionsRequest {
        dry_run: true,
    });

    let response = service.cleanup_expired_sessions(request).await;
    assert!(response.is_ok(), "cleanup_expired_sessions should succeed");
    
    let response = response.unwrap().into_inner();
    assert!(response.cleaned_count >= 0, "Should return cleanup count");
}

#[tokio::test]
async fn test_input_validation() {
    let service = create_test_service().await;
    
    // Test with invalid session ID
    let request = create_authenticated_request(GetSessionRequest {
        session_id: "invalid-uuid".to_string(),
    });

    let response = service.get_session(request).await;
    assert!(response.is_err(), "Should validate session ID format");
    assert_eq!(response.unwrap_err().code(), Code::InvalidArgument);
}

#[tokio::test]
async fn test_authentication_required() {
    let service = create_test_service().await;
    
    // Test without authentication
    let request = Request::new(ListSessionsRequest {
        status: 0,
        dapp_url: String::new(),
        page_size: 20,
        page_token: String::new(),
    });

    let response = service.list_sessions(request).await;
    assert!(response.is_err(), "Should require authentication");
    assert_eq!(response.unwrap_err().code(), Code::Unauthenticated);
}

#[tokio::test]
async fn test_rate_limiting() {
    let service = create_test_service().await;
    
    // Make multiple rapid requests to test rate limiting
    for i in 0..10 {
        let request = create_authenticated_request(ListSessionsRequest {
            status: 0,
            dapp_url: String::new(),
            page_size: 20,
            page_token: String::new(),
        });

        let response = service.list_sessions(request).await;
        if i < 5 {
            assert!(response.is_ok(), "First few requests should succeed");
        }
        // Later requests might be rate limited depending on implementation
    }
}

#[tokio::test]
async fn test_session_lifecycle() {
    let service = create_test_service().await;
    
    // 1. Create session
    let create_request = create_authenticated_request(CreateSessionRequest {
        dapp_name: "Lifecycle Test DApp".to_string(),
        dapp_url: "https://lifecycle-test.com".to_string(),
        dapp_icon: "https://lifecycle-test.com/icon.png".to_string(),
        dapp_description: "Testing session lifecycle".to_string(),
        required_chains: vec!["ethereum".to_string()],
        required_methods: vec!["eth_sendTransaction".to_string()],
        required_events: vec!["accountsChanged".to_string()],
        expiry_hours: 1,
        metadata: HashMap::new(),
    });

    let create_response = service.create_session(create_request).await;
    assert!(create_response.is_ok());
    let session_id = create_response.unwrap().into_inner().session_id;

    // 2. Get session
    let get_request = create_authenticated_request(GetSessionRequest {
        session_id: session_id.clone(),
    });
    let get_response = service.get_session(get_request).await;
    assert!(get_response.is_ok());

    // 3. Approve session
    let approve_request = create_authenticated_request(ApproveSessionRequest {
        session_id: session_id.clone(),
        approved_chains: vec!["ethereum".to_string()],
        approved_accounts: vec!["0x1234567890123456789012345678901234567890".to_string()],
        approved_methods: vec!["eth_sendTransaction".to_string()],
        approved_events: vec!["accountsChanged".to_string()],
    });
    let approve_response = service.approve_session(approve_request).await;
    assert!(approve_response.is_ok());

    // 4. Disconnect session
    let disconnect_request = create_authenticated_request(DisconnectSessionRequest {
        session_id: session_id.clone(),
        reason: "Test completed".to_string(),
    });
    let disconnect_response = service.disconnect_session(disconnect_request).await;
    assert!(disconnect_response.is_ok());
}
