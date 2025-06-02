//! End-to-end tests for KYC functionality

use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use uuid::Uuid;
use chrono::NaiveDate;

use fo3_wallet_api::{
    proto::fo3::wallet::v1::{
        kyc_service_client::KycServiceClient,
        auth_service_client::AuthServiceClient,
        *,
    },
    state::AppState,
    services::{
        kyc::KycServiceImpl,
        auth::AuthServiceImpl,
    },
    middleware::{
        auth::AuthService,
        audit::AuditLogger,
        kyc_guard::KycGuard,
    },
    models::kyc::{KycStatus, DocumentType},
};

use tonic::{transport::Server, Request, Response, Status};
use tonic::metadata::MetadataValue;

/// Test helper to create a test server
async fn create_test_server() -> (String, tokio::task::JoinHandle<()>) {
    let state = Arc::new(AppState::new());
    let auth_service = Arc::new(AuthService::new(state.clone()));
    let audit_logger = Arc::new(AuditLogger::new(state.clone()));
    let kyc_guard = Arc::new(KycGuard::new(state.clone(), auth_service.clone()));

    let kyc_service = KycServiceImpl::new(state.clone(), auth_service.clone(), audit_logger.clone());
    let auth_grpc_service = AuthServiceImpl::new(auth_service.clone(), audit_logger.clone());

    let addr = "127.0.0.1:0".parse().unwrap();
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = Server::builder()
        .add_service(kyc_service_server::KycServiceServer::new(kyc_service))
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
async fn test_kyc_submission_workflow() {
    let (server_url, _handle) = create_test_server().await;

    // Authenticate user
    let token = authenticate_user(&server_url, "testuser", "password123").await;
    let mut client = KycServiceClient::connect(server_url).await.unwrap();

    // Test KYC submission
    let wallet_id = Uuid::new_v4().to_string();
    let personal_info = PersonalInfo {
        first_name: "John".to_string(),
        last_name: "Doe".to_string(),
        date_of_birth: "1990-01-01".to_string(),
        nationality: "US".to_string(),
        country_of_residence: "US".to_string(),
        address: Some(Address {
            street_address: "123 Main St".to_string(),
            city: "New York".to_string(),
            state_province: "NY".to_string(),
            postal_code: "10001".to_string(),
            country: "US".to_string(),
        }),
    };

    let submit_request = SubmitKycRequest {
        wallet_id: wallet_id.clone(),
        personal_info: Some(personal_info),
        document_ids: vec![],
    };

    let request = create_authenticated_request(&token, submit_request);
    let response = client.submit_kyc(request).await.unwrap();
    let submission = response.into_inner().submission.unwrap();

    assert_eq!(submission.wallet_id, wallet_id);
    assert_eq!(submission.status, KycStatus::KycStatusPending as i32);
    assert_eq!(submission.personal_info.as_ref().unwrap().first_name, "John");

    // Test getting KYC status
    let status_request = GetKycStatusRequest {
        wallet_id: wallet_id.clone(),
    };

    let request = create_authenticated_request(&token, status_request);
    let response = client.get_kyc_status(request).await.unwrap();
    let status_submission = response.into_inner().submission.unwrap();

    assert_eq!(status_submission.id, submission.id);
    assert_eq!(status_submission.status, KycStatus::KycStatusPending as i32);
}

#[tokio::test]
async fn test_kyc_admin_operations() {
    let (server_url, _handle) = create_test_server().await;

    // Authenticate admin user
    let admin_token = authenticate_user(&server_url, "admin", "admin123").await;
    let user_token = authenticate_user(&server_url, "testuser", "password123").await;
    let mut client = KycServiceClient::connect(server_url).await.unwrap();

    // Submit KYC as regular user
    let wallet_id = Uuid::new_v4().to_string();
    let personal_info = PersonalInfo {
        first_name: "Jane".to_string(),
        last_name: "Smith".to_string(),
        date_of_birth: "1985-05-15".to_string(),
        nationality: "CA".to_string(),
        country_of_residence: "CA".to_string(),
        address: Some(Address {
            street_address: "456 Maple Ave".to_string(),
            city: "Toronto".to_string(),
            state_province: "ON".to_string(),
            postal_code: "M5V 3A8".to_string(),
            country: "CA".to_string(),
        }),
    };

    let submit_request = SubmitKycRequest {
        wallet_id: wallet_id.clone(),
        personal_info: Some(personal_info),
        document_ids: vec![],
    };

    let request = create_authenticated_request(&user_token, submit_request);
    let response = client.submit_kyc(request).await.unwrap();
    let submission = response.into_inner().submission.unwrap();
    let submission_id = submission.id.clone();

    // Test admin listing submissions
    let list_request = ListKycSubmissionsRequest {
        page_size: 10,
        page_token: String::new(),
        status_filter: 0,
        wallet_id_filter: String::new(),
    };

    let request = create_authenticated_request(&admin_token, list_request);
    let response = client.list_kyc_submissions(request).await.unwrap();
    let list_response = response.into_inner();

    assert!(!list_response.submissions.is_empty());
    assert!(list_response.submissions.iter().any(|s| s.id == submission_id));

    // Test admin approval
    let approve_request = ApproveKycRequest {
        submission_id: submission_id.clone(),
        reviewer_notes: "Documents verified successfully".to_string(),
    };

    let request = create_authenticated_request(&admin_token, approve_request);
    let response = client.approve_kyc(request).await.unwrap();
    let approved_submission = response.into_inner().submission.unwrap();

    assert_eq!(approved_submission.status, KycStatus::KycStatusApproved as i32);
    assert_eq!(approved_submission.reviewer_notes, "Documents verified successfully");
    assert!(!approved_submission.reviewer_id.is_empty());
}

#[tokio::test]
async fn test_kyc_rejection_workflow() {
    let (server_url, _handle) = create_test_server().await;

    let admin_token = authenticate_user(&server_url, "admin", "admin123").await;
    let user_token = authenticate_user(&server_url, "testuser", "password123").await;
    let mut client = KycServiceClient::connect(server_url).await.unwrap();

    // Submit KYC
    let wallet_id = Uuid::new_v4().to_string();
    let personal_info = PersonalInfo {
        first_name: "Bob".to_string(),
        last_name: "Johnson".to_string(),
        date_of_birth: "1975-12-25".to_string(),
        nationality: "UK".to_string(),
        country_of_residence: "UK".to_string(),
        address: Some(Address {
            street_address: "789 Oak Road".to_string(),
            city: "London".to_string(),
            state_province: String::new(),
            postal_code: "SW1A 1AA".to_string(),
            country: "UK".to_string(),
        }),
    };

    let submit_request = SubmitKycRequest {
        wallet_id: wallet_id.clone(),
        personal_info: Some(personal_info),
        document_ids: vec![],
    };

    let request = create_authenticated_request(&user_token, submit_request);
    let response = client.submit_kyc(request).await.unwrap();
    let submission = response.into_inner().submission.unwrap();
    let submission_id = submission.id.clone();

    // Test admin rejection
    let reject_request = RejectKycRequest {
        submission_id: submission_id.clone(),
        rejection_reason: "Document quality insufficient".to_string(),
        reviewer_notes: "Please provide clearer images".to_string(),
    };

    let request = create_authenticated_request(&admin_token, reject_request);
    let response = client.reject_kyc(request).await.unwrap();
    let rejected_submission = response.into_inner().submission.unwrap();

    assert_eq!(rejected_submission.status, KycStatus::KycStatusRejected as i32);
    assert_eq!(rejected_submission.rejection_reason, "Document quality insufficient");
    assert_eq!(rejected_submission.reviewer_notes, "Please provide clearer images");
}

#[tokio::test]
async fn test_kyc_document_update() {
    let (server_url, _handle) = create_test_server().await;

    let user_token = authenticate_user(&server_url, "testuser", "password123").await;
    let mut client = KycServiceClient::connect(server_url).await.unwrap();

    // Submit initial KYC
    let wallet_id = Uuid::new_v4().to_string();
    let personal_info = PersonalInfo {
        first_name: "Alice".to_string(),
        last_name: "Brown".to_string(),
        date_of_birth: "1992-08-10".to_string(),
        nationality: "AU".to_string(),
        country_of_residence: "AU".to_string(),
        address: Some(Address {
            street_address: "321 Beach Blvd".to_string(),
            city: "Sydney".to_string(),
            state_province: "NSW".to_string(),
            postal_code: "2000".to_string(),
            country: "AU".to_string(),
        }),
    };

    let submit_request = SubmitKycRequest {
        wallet_id: wallet_id.clone(),
        personal_info: Some(personal_info),
        document_ids: vec![],
    };

    let request = create_authenticated_request(&user_token, submit_request);
    let response = client.submit_kyc(request).await.unwrap();
    let submission = response.into_inner().submission.unwrap();
    let submission_id = submission.id.clone();

    // Test document update
    let new_doc_id = Uuid::new_v4().to_string();
    let update_request = UpdateKycDocumentsRequest {
        submission_id: submission_id.clone(),
        document_ids: vec![new_doc_id.clone()],
        remove_document_ids: vec![],
    };

    let request = create_authenticated_request(&user_token, update_request);
    let response = client.update_kyc_documents(request).await.unwrap();
    let updated_submission = response.into_inner().submission.unwrap();

    assert!(!updated_submission.documents.is_empty());
    assert!(updated_submission.documents.iter().any(|d| d.id == new_doc_id));
}

#[tokio::test]
async fn test_kyc_permission_checks() {
    let (server_url, _handle) = create_test_server().await;

    let user_token = authenticate_user(&server_url, "testuser", "password123").await;
    let mut client = KycServiceClient::connect(server_url).await.unwrap();

    // Test that regular user cannot list all submissions
    let list_request = ListKycSubmissionsRequest {
        page_size: 10,
        page_token: String::new(),
        status_filter: 0,
        wallet_id_filter: String::new(),
    };

    let request = create_authenticated_request(&user_token, list_request);
    let result = client.list_kyc_submissions(request).await;

    // Should fail with permission denied
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert_eq!(error.code(), tonic::Code::PermissionDenied);
}

#[tokio::test]
async fn test_kyc_duplicate_submission() {
    let (server_url, _handle) = create_test_server().await;

    let user_token = authenticate_user(&server_url, "testuser", "password123").await;
    let mut client = KycServiceClient::connect(server_url).await.unwrap();

    let wallet_id = Uuid::new_v4().to_string();
    let personal_info = PersonalInfo {
        first_name: "Charlie".to_string(),
        last_name: "Wilson".to_string(),
        date_of_birth: "1988-03-20".to_string(),
        nationality: "DE".to_string(),
        country_of_residence: "DE".to_string(),
        address: Some(Address {
            street_address: "Unter den Linden 1".to_string(),
            city: "Berlin".to_string(),
            state_province: String::new(),
            postal_code: "10117".to_string(),
            country: "DE".to_string(),
        }),
    };

    // First submission should succeed
    let submit_request = SubmitKycRequest {
        wallet_id: wallet_id.clone(),
        personal_info: Some(personal_info.clone()),
        document_ids: vec![],
    };

    let request = create_authenticated_request(&user_token, submit_request);
    let response = client.submit_kyc(request).await;
    assert!(response.is_ok());

    // Second submission should fail
    let submit_request2 = SubmitKycRequest {
        wallet_id: wallet_id.clone(),
        personal_info: Some(personal_info),
        document_ids: vec![],
    };

    let request2 = create_authenticated_request(&user_token, submit_request2);
    let result = client.submit_kyc(request2).await;

    assert!(result.is_err());
    let error = result.unwrap_err();
    assert_eq!(error.code(), tonic::Code::AlreadyExists);
}

#[tokio::test]
async fn test_kyc_security_validation() {
    let (server_url, _handle) = create_test_server().await;

    let user_token = authenticate_user(&server_url, "testuser", "password123").await;
    let mut client = KycServiceClient::connect(server_url).await.unwrap();

    // Test invalid date format
    let wallet_id = Uuid::new_v4().to_string();
    let personal_info = PersonalInfo {
        first_name: "Test".to_string(),
        last_name: "User".to_string(),
        date_of_birth: "invalid-date".to_string(), // Invalid format
        nationality: "US".to_string(),
        country_of_residence: "US".to_string(),
        address: Some(Address {
            street_address: "123 Test St".to_string(),
            city: "Test City".to_string(),
            state_province: "TS".to_string(),
            postal_code: "12345".to_string(),
            country: "US".to_string(),
        }),
    };

    let submit_request = SubmitKycRequest {
        wallet_id: wallet_id.clone(),
        personal_info: Some(personal_info),
        document_ids: vec![],
    };

    let request = create_authenticated_request(&user_token, submit_request);
    let result = client.submit_kyc(request).await;

    assert!(result.is_err());
    let error = result.unwrap_err();
    assert_eq!(error.code(), tonic::Code::InvalidArgument);
}

#[tokio::test]
async fn test_kyc_cross_user_access_prevention() {
    let (server_url, _handle) = create_test_server().await;

    let user1_token = authenticate_user(&server_url, "user1", "password123").await;
    let user2_token = authenticate_user(&server_url, "user2", "password123").await;
    let mut client = KycServiceClient::connect(server_url).await.unwrap();

    // User 1 submits KYC
    let wallet_id = Uuid::new_v4().to_string();
    let personal_info = PersonalInfo {
        first_name: "User".to_string(),
        last_name: "One".to_string(),
        date_of_birth: "1990-01-01".to_string(),
        nationality: "US".to_string(),
        country_of_residence: "US".to_string(),
        address: Some(Address {
            street_address: "123 User1 St".to_string(),
            city: "User1 City".to_string(),
            state_province: "U1".to_string(),
            postal_code: "11111".to_string(),
            country: "US".to_string(),
        }),
    };

    let submit_request = SubmitKycRequest {
        wallet_id: wallet_id.clone(),
        personal_info: Some(personal_info),
        document_ids: vec![],
    };

    let request = create_authenticated_request(&user1_token, submit_request);
    let response = client.submit_kyc(request).await.unwrap();
    let submission = response.into_inner().submission.unwrap();

    // User 2 tries to access User 1's KYC status
    let status_request = GetKycStatusRequest {
        wallet_id: wallet_id.clone(),
    };

    let request = create_authenticated_request(&user2_token, status_request);
    let result = client.get_kyc_status(request).await;

    // Should fail with permission denied
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert_eq!(error.code(), tonic::Code::PermissionDenied);
}

#[tokio::test]
async fn test_kyc_performance_document_upload() {
    let (server_url, _handle) = create_test_server().await;

    let user_token = authenticate_user(&server_url, "testuser", "password123").await;
    let mut client = KycServiceClient::connect(server_url).await.unwrap();

    let start_time = std::time::Instant::now();

    // Simulate multiple document uploads
    let mut upload_times = Vec::new();
    for i in 0..10 {
        let upload_start = std::time::Instant::now();

        // Create a mock upload request
        let upload_request = UploadDocumentRequest {
            data: Some(upload_document_request::Data::Metadata(DocumentMetadata {
                r#type: DocumentType::DocumentTypeGovernmentId as i32,
                filename: format!("test_document_{}.pdf", i),
                content_type: "application/pdf".to_string(),
                size_bytes: 1024 * 1024, // 1MB
                wallet_id: Uuid::new_v4().to_string(),
            })),
        };

        let request = create_authenticated_request(&user_token, upload_request);
        let result = client.upload_document(tokio_stream::once(Ok(request))).await;

        let upload_duration = upload_start.elapsed();
        upload_times.push(upload_duration);

        // For now, we expect this to work (placeholder implementation)
        assert!(result.is_ok());
    }

    let total_duration = start_time.elapsed();
    let avg_upload_time = upload_times.iter().sum::<std::time::Duration>() / upload_times.len() as u32;

    // Performance assertions
    assert!(total_duration < std::time::Duration::from_secs(30), "Total upload time should be under 30 seconds");
    assert!(avg_upload_time < std::time::Duration::from_secs(2), "Average upload time should be under 2 seconds");

    println!("Performance metrics:");
    println!("Total time for 10 uploads: {:?}", total_duration);
    println!("Average upload time: {:?}", avg_upload_time);
}

#[tokio::test]
async fn test_kyc_state_transitions() {
    let (server_url, _handle) = create_test_server().await;

    let admin_token = authenticate_user(&server_url, "admin", "admin123").await;
    let user_token = authenticate_user(&server_url, "testuser", "password123").await;
    let mut client = KycServiceClient::connect(server_url).await.unwrap();

    // Submit KYC
    let wallet_id = Uuid::new_v4().to_string();
    let personal_info = PersonalInfo {
        first_name: "State".to_string(),
        last_name: "Transition".to_string(),
        date_of_birth: "1985-06-15".to_string(),
        nationality: "CA".to_string(),
        country_of_residence: "CA".to_string(),
        address: Some(Address {
            street_address: "456 State St".to_string(),
            city: "Transition City".to_string(),
            state_province: "ST".to_string(),
            postal_code: "54321".to_string(),
            country: "CA".to_string(),
        }),
    };

    let submit_request = SubmitKycRequest {
        wallet_id: wallet_id.clone(),
        personal_info: Some(personal_info),
        document_ids: vec![],
    };

    let request = create_authenticated_request(&user_token, submit_request);
    let response = client.submit_kyc(request).await.unwrap();
    let submission = response.into_inner().submission.unwrap();
    let submission_id = submission.id.clone();

    // Verify initial state is Pending
    assert_eq!(submission.status, KycStatus::KycStatusPending as i32);

    // Admin approves
    let approve_request = ApproveKycRequest {
        submission_id: submission_id.clone(),
        reviewer_notes: "All documents verified".to_string(),
    };

    let request = create_authenticated_request(&admin_token, approve_request);
    let response = client.approve_kyc(request).await.unwrap();
    let approved_submission = response.into_inner().submission.unwrap();

    // Verify state changed to Approved
    assert_eq!(approved_submission.status, KycStatus::KycStatusApproved as i32);
    assert!(!approved_submission.reviewer_id.is_empty());
    assert!(approved_submission.reviewed_at > 0);

    // Try to approve again (should fail)
    let approve_again_request = ApproveKycRequest {
        submission_id: submission_id.clone(),
        reviewer_notes: "Trying again".to_string(),
    };

    let request = create_authenticated_request(&admin_token, approve_again_request);
    let result = client.approve_kyc(request).await;

    assert!(result.is_err());
    let error = result.unwrap_err();
    assert_eq!(error.code(), tonic::Code::FailedPrecondition);
}

#[tokio::test]
async fn test_kyc_pagination_and_filtering() {
    let (server_url, _handle) = create_test_server().await;

    let admin_token = authenticate_user(&server_url, "admin", "admin123").await;
    let user_token = authenticate_user(&server_url, "testuser", "password123").await;
    let mut client = KycServiceClient::connect(server_url).await.unwrap();

    // Create multiple KYC submissions
    let mut submission_ids = Vec::new();
    for i in 0..5 {
        let wallet_id = Uuid::new_v4().to_string();
        let personal_info = PersonalInfo {
            first_name: format!("User{}", i),
            last_name: "Test".to_string(),
            date_of_birth: "1990-01-01".to_string(),
            nationality: "US".to_string(),
            country_of_residence: "US".to_string(),
            address: Some(Address {
                street_address: format!("{} Test St", i),
                city: "Test City".to_string(),
                state_province: "TS".to_string(),
                postal_code: "12345".to_string(),
                country: "US".to_string(),
            }),
        };

        let submit_request = SubmitKycRequest {
            wallet_id: wallet_id.clone(),
            personal_info: Some(personal_info),
            document_ids: vec![],
        };

        let request = create_authenticated_request(&user_token, submit_request);
        let response = client.submit_kyc(request).await.unwrap();
        let submission = response.into_inner().submission.unwrap();
        submission_ids.push(submission.id);
    }

    // Test pagination
    let list_request = ListKycSubmissionsRequest {
        page_size: 2,
        page_token: String::new(),
        status_filter: 0,
        wallet_id_filter: String::new(),
    };

    let request = create_authenticated_request(&admin_token, list_request);
    let response = client.list_kyc_submissions(request).await.unwrap();
    let list_response = response.into_inner();

    assert_eq!(list_response.submissions.len(), 2);
    assert!(!list_response.next_page_token.is_empty());
    assert!(list_response.total_count >= 5);

    // Test filtering by status
    let filtered_request = ListKycSubmissionsRequest {
        page_size: 10,
        page_token: String::new(),
        status_filter: KycStatus::KycStatusPending as i32,
        wallet_id_filter: String::new(),
    };

    let request = create_authenticated_request(&admin_token, filtered_request);
    let response = client.list_kyc_submissions(request).await.unwrap();
    let filtered_response = response.into_inner();

    // All returned submissions should be pending
    for submission in &filtered_response.submissions {
        assert_eq!(submission.status, KycStatus::KycStatusPending as i32);
    }
}
