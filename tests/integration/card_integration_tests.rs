//! Integration tests for Card service

use std::sync::Arc;
use tonic::{Request, Response, Status};
use uuid::Uuid;
use rust_decimal::Decimal;

use fo3_wallet_api::state::AppState;
use fo3_wallet_api::services::cards::CardServiceImpl;
use fo3_wallet_api::middleware::{
    auth::{AuthService, AuthContext, UserRole, Permission},
    audit::AuditLogger,
};
use fo3_wallet_api::proto::fo3::wallet::v1::{
    card_service_server::CardService,
    *,
};
use fo3_wallet_api::models::cards::{Card, CardLimits, CardStatus, CardType};
use fo3_wallet_api::models::kyc::{KycSubmission, KycStatus, PersonalInfo, Address, DocumentType};

/// Create a test card service
async fn create_card_service() -> CardServiceImpl {
    let state = Arc::new(AppState::new());
    let auth_service = Arc::new(AuthService::new());
    let audit_logger = Arc::new(AuditLogger::new());
    
    CardServiceImpl::new(state, auth_service, audit_logger)
}

/// Create a test auth context
fn create_test_auth_context() -> AuthContext {
    AuthContext {
        user_id: Uuid::new_v4().to_string(),
        username: "test_user".to_string(),
        role: UserRole::UserRoleUser,
        permissions: vec![
            Permission::PermissionCardRead,
            Permission::PermissionCardAdmin,
        ],
        auth_type: fo3_wallet_api::middleware::auth::AuthType::JWT("test_token".to_string()),
    }
}

/// Create a test KYC submission for the user
async fn create_test_kyc_submission(service: &CardServiceImpl, user_id: Uuid) {
    let kyc_submission = KycSubmission {
        id: Uuid::new_v4(),
        wallet_id: user_id,
        status: KycStatus::Approved,
        personal_info: PersonalInfo {
            first_name: "John".to_string(),
            last_name: "Doe".to_string(),
            date_of_birth: chrono::NaiveDate::from_ymd_opt(1990, 1, 1).unwrap(),
            nationality: "US".to_string(),
            address: Address {
                street_address: "123 Main St".to_string(),
                city: "New York".to_string(),
                state_province: Some("NY".to_string()),
                postal_code: "10001".to_string(),
                country: "US".to_string(),
            },
        },
        documents: Vec::new(),
        submitted_at: chrono::Utc::now(),
        reviewed_at: Some(chrono::Utc::now()),
        reviewer_id: Some("admin".to_string()),
        reviewer_notes: Some("Approved".to_string()),
        rejection_reason: None,
        updated_at: chrono::Utc::now(),
    };

    // Add KYC submission to state
    service.state.kyc_submissions.write().unwrap()
        .insert(kyc_submission.id.to_string(), kyc_submission);
}

#[tokio::test]
async fn test_issue_virtual_card_success() {
    let service = create_card_service().await;
    let auth_context = create_test_auth_context();
    let user_id = Uuid::parse_str(&auth_context.user_id).unwrap();

    // Create approved KYC submission
    create_test_kyc_submission(&service, user_id).await;

    let mut request = Request::new(IssueCardRequest {
        cardholder_name: "John Doe".to_string(),
        currency: "USD".to_string(),
        limits: Some(CardLimits {
            daily_limit: "1000.00".to_string(),
            monthly_limit: "10000.00".to_string(),
            per_transaction_limit: "500.00".to_string(),
            atm_daily_limit: "200.00".to_string(),
            transaction_count_daily: 20,
            transaction_count_monthly: 200,
        }),
        design_id: "premium".to_string(),
        linked_account_id: String::new(),
        is_primary: true,
    });

    request.extensions_mut().insert(auth_context);

    let response = service.issue_virtual_card(request).await;
    assert!(response.is_ok());

    let issue_response = response.unwrap().into_inner();
    assert!(issue_response.card.is_some());
    assert!(!issue_response.full_card_number.is_empty());
    assert!(!issue_response.cvv.is_empty());
    assert!(!issue_response.pin.is_empty());

    let card = issue_response.card.unwrap();
    assert_eq!(card.cardholder_name, "John Doe");
    assert_eq!(card.currency, "USD");
    assert_eq!(card.status, 1); // Active
    assert_eq!(card.r#type, 1); // Virtual
    assert!(card.is_primary);
}

#[tokio::test]
async fn test_issue_card_without_kyc_fails() {
    let service = create_card_service().await;
    let auth_context = create_test_auth_context();

    let mut request = Request::new(IssueCardRequest {
        cardholder_name: "John Doe".to_string(),
        currency: "USD".to_string(),
        limits: None,
        design_id: "default".to_string(),
        linked_account_id: String::new(),
        is_primary: false,
    });

    request.extensions_mut().insert(auth_context);

    let response = service.issue_virtual_card(request).await;
    assert!(response.is_err());
    
    let error = response.unwrap_err();
    assert_eq!(error.code(), tonic::Code::FailedPrecondition);
    assert!(error.message().contains("KYC verification required"));
}

#[tokio::test]
async fn test_get_card_success() {
    let service = create_card_service().await;
    let auth_context = create_test_auth_context();
    let user_id = Uuid::parse_str(&auth_context.user_id).unwrap();

    // Create approved KYC submission
    create_test_kyc_submission(&service, user_id).await;

    // First, issue a card
    let mut issue_request = Request::new(IssueCardRequest {
        cardholder_name: "John Doe".to_string(),
        currency: "USD".to_string(),
        limits: None,
        design_id: "default".to_string(),
        linked_account_id: String::new(),
        is_primary: false,
    });
    issue_request.extensions_mut().insert(auth_context.clone());

    let issue_response = service.issue_virtual_card(issue_request).await.unwrap();
    let card_id = issue_response.into_inner().card.unwrap().id;

    // Now get the card
    let mut get_request = Request::new(GetCardRequest {
        card_id: card_id.clone(),
    });
    get_request.extensions_mut().insert(auth_context);

    let response = service.get_card(get_request).await;
    assert!(response.is_ok());

    let get_response = response.unwrap().into_inner();
    assert!(get_response.card.is_some());
    
    let card = get_response.card.unwrap();
    assert_eq!(card.id, card_id);
    assert_eq!(card.cardholder_name, "John Doe");
}

#[tokio::test]
async fn test_list_cards_success() {
    let service = create_card_service().await;
    let auth_context = create_test_auth_context();
    let user_id = Uuid::parse_str(&auth_context.user_id).unwrap();

    // Create approved KYC submission
    create_test_kyc_submission(&service, user_id).await;

    // Issue multiple cards
    for i in 0..3 {
        let mut issue_request = Request::new(IssueCardRequest {
            cardholder_name: format!("Card {}", i),
            currency: "USD".to_string(),
            limits: None,
            design_id: "default".to_string(),
            linked_account_id: String::new(),
            is_primary: i == 0,
        });
        issue_request.extensions_mut().insert(auth_context.clone());

        let _response = service.issue_virtual_card(issue_request).await.unwrap();
    }

    // List cards
    let mut list_request = Request::new(ListCardsRequest {
        status: 0, // All statuses
        r#type: 0, // All types
        page_size: 10,
        page_token: String::new(),
    });
    list_request.extensions_mut().insert(auth_context);

    let response = service.list_cards(list_request).await;
    assert!(response.is_ok());

    let list_response = response.unwrap().into_inner();
    assert_eq!(list_response.cards.len(), 3);
    assert_eq!(list_response.total_count, 3);
    
    // Check that one card is primary
    let primary_cards: Vec<_> = list_response.cards.iter()
        .filter(|card| card.is_primary)
        .collect();
    assert_eq!(primary_cards.len(), 1);
}

#[tokio::test]
async fn test_freeze_card_success() {
    let service = create_card_service().await;
    let auth_context = create_test_auth_context();
    let user_id = Uuid::parse_str(&auth_context.user_id).unwrap();

    // Create approved KYC submission
    create_test_kyc_submission(&service, user_id).await;

    // Issue a card
    let mut issue_request = Request::new(IssueCardRequest {
        cardholder_name: "John Doe".to_string(),
        currency: "USD".to_string(),
        limits: None,
        design_id: "default".to_string(),
        linked_account_id: String::new(),
        is_primary: false,
    });
    issue_request.extensions_mut().insert(auth_context.clone());

    let issue_response = service.issue_virtual_card(issue_request).await.unwrap();
    let card_id = issue_response.into_inner().card.unwrap().id;

    // Freeze the card
    let mut freeze_request = Request::new(FreezeCardRequest {
        card_id: card_id.clone(),
        reason: "Lost card".to_string(),
        require_2fa: false,
        verification_code: String::new(),
    });
    freeze_request.extensions_mut().insert(auth_context);

    let response = service.freeze_card(freeze_request).await;
    assert!(response.is_ok());

    let freeze_response = response.unwrap().into_inner();
    assert!(freeze_response.success);
    assert!(freeze_response.card.is_some());
    
    let card = freeze_response.card.unwrap();
    assert_eq!(card.status, 2); // Frozen
    assert_eq!(card.frozen_reason, "Lost card");
}

#[tokio::test]
async fn test_simulate_transaction_success() {
    let service = create_card_service().await;
    let auth_context = create_test_auth_context();
    let user_id = Uuid::parse_str(&auth_context.user_id).unwrap();

    // Create approved KYC submission
    create_test_kyc_submission(&service, user_id).await;

    // Issue a card
    let mut issue_request = Request::new(IssueCardRequest {
        cardholder_name: "John Doe".to_string(),
        currency: "USD".to_string(),
        limits: None,
        design_id: "default".to_string(),
        linked_account_id: String::new(),
        is_primary: false,
    });
    issue_request.extensions_mut().insert(auth_context.clone());

    let issue_response = service.issue_virtual_card(issue_request).await.unwrap();
    let card_id = issue_response.into_inner().card.unwrap().id;

    // Top up the card first
    let mut topup_request = Request::new(TopUpCardRequest {
        card_id: card_id.clone(),
        amount: "100.00".to_string(),
        currency: "USD".to_string(),
        funding_source_id: String::new(),
    });
    topup_request.extensions_mut().insert(auth_context.clone());

    let _topup_response = service.top_up_card(topup_request).await.unwrap();

    // Simulate a transaction
    let mut simulate_request = Request::new(SimulateTransactionRequest {
        card_id: card_id.clone(),
        amount: "50.00".to_string(),
        currency: "USD".to_string(),
        merchant: Some(MerchantInfo {
            name: "Test Store".to_string(),
            category: "Retail".to_string(),
            category_code: 4, // Retail
            location: "New York, NY".to_string(),
            country: "US".to_string(),
            mcc: "5411".to_string(),
        }),
        description: "Test purchase".to_string(),
        is_authorization_only: false,
    });
    simulate_request.extensions_mut().insert(auth_context);

    let response = service.simulate_transaction(simulate_request).await;
    assert!(response.is_ok());

    let simulate_response = response.unwrap().into_inner();
    assert!(simulate_response.approved);
    assert!(simulate_response.transaction.is_some());
    assert!(!simulate_response.authorization_code.is_empty());
    
    let transaction = simulate_response.transaction.unwrap();
    assert_eq!(transaction.amount, "50.00");
    assert_eq!(transaction.currency, "USD");
    assert_eq!(transaction.status, 2); // Approved
}

#[tokio::test]
async fn test_top_up_card_success() {
    let service = create_card_service().await;
    let auth_context = create_test_auth_context();
    let user_id = Uuid::parse_str(&auth_context.user_id).unwrap();

    // Create approved KYC submission
    create_test_kyc_submission(&service, user_id).await;

    // Issue a card
    let mut issue_request = Request::new(IssueCardRequest {
        cardholder_name: "John Doe".to_string(),
        currency: "USD".to_string(),
        limits: None,
        design_id: "default".to_string(),
        linked_account_id: String::new(),
        is_primary: false,
    });
    issue_request.extensions_mut().insert(auth_context.clone());

    let issue_response = service.issue_virtual_card(issue_request).await.unwrap();
    let card_id = issue_response.into_inner().card.unwrap().id;

    // Top up the card
    let mut topup_request = Request::new(TopUpCardRequest {
        card_id: card_id.clone(),
        amount: "250.00".to_string(),
        currency: "USD".to_string(),
        funding_source_id: String::new(),
    });
    topup_request.extensions_mut().insert(auth_context);

    let response = service.top_up_card(topup_request).await;
    assert!(response.is_ok());

    let topup_response = response.unwrap().into_inner();
    assert!(topup_response.success);
    assert!(topup_response.card.is_some());
    assert!(topup_response.transaction.is_some());
    
    let card = topup_response.card.unwrap();
    assert_eq!(card.balance, "250.00");
    
    let transaction = topup_response.transaction.unwrap();
    assert_eq!(transaction.amount, "250.00");
    assert_eq!(transaction.r#type, 4); // TopUp
    assert_eq!(transaction.status, 5); // Settled
}

#[tokio::test]
async fn test_get_card_metrics_admin() {
    let service = create_card_service().await;
    let mut auth_context = create_test_auth_context();
    auth_context.role = UserRole::UserRoleAdmin;

    let mut request = Request::new(GetCardMetricsRequest {
        start_date: 0,
        end_date: 0,
        currency: String::new(),
    });
    request.extensions_mut().insert(auth_context);

    let response = service.get_card_metrics(request).await;
    assert!(response.is_ok());

    let metrics_response = response.unwrap().into_inner();
    assert!(metrics_response.metrics.is_some());
    
    let metrics = metrics_response.metrics.unwrap();
    assert_eq!(metrics.total_cards_issued, 0); // No cards issued yet
    assert_eq!(metrics.active_cards, 0);
    assert_eq!(metrics.total_transactions, 0);
}
