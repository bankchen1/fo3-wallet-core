//! Integration tests for Spending Insights service

use std::sync::Arc;
use tonic::{Request, Response, Status};
use uuid::Uuid;
use rust_decimal::Decimal;

use fo3_wallet_api::state::AppState;
use fo3_wallet_api::services::spending_insights::SpendingInsightsServiceImpl;
use fo3_wallet_api::middleware::{
    auth::{AuthService, AuthContext, UserRole, Permission},
    audit::AuditLogger,
};
use fo3_wallet_api::proto::fo3::wallet::v1::{
    spending_insights_service_server::SpendingInsightsService,
    *,
};
use fo3_wallet_api::models::spending_insights::{Budget, SpendingAlert, TimePeriod, BudgetStatus, AlertType};

/// Create a test spending insights service
async fn create_spending_insights_service() -> SpendingInsightsServiceImpl {
    let state = Arc::new(AppState::new());
    let auth_service = Arc::new(AuthService::new());
    let audit_logger = Arc::new(AuditLogger::new());
    
    SpendingInsightsServiceImpl::new(state, auth_service, audit_logger)
}

/// Create a test auth context
fn create_test_auth_context() -> AuthContext {
    AuthContext {
        user_id: Uuid::new_v4().to_string(),
        username: "test_user".to_string(),
        role: UserRole::UserRoleUser,
        permissions: vec![
            Permission::PermissionSpendingRead,
            Permission::PermissionSpendingAdmin,
        ],
        auth_type: fo3_wallet_api::middleware::auth::AuthType::JWT("test_token".to_string()),
    }
}

#[tokio::test]
async fn test_get_spending_summary_success() {
    let service = create_spending_insights_service().await;
    let auth_context = create_test_auth_context();

    let mut request = Request::new(GetSpendingSummaryRequest {
        period: 3, // Monthly
        start_date: 0,
        end_date: 0,
        currency: "USD".to_string(),
        card_ids: vec![],
    });

    request.extensions_mut().insert(auth_context);

    let response = service.get_spending_summary(request).await;
    assert!(response.is_ok());

    let summary_response = response.unwrap().into_inner();
    assert!(!summary_response.total_spent.is_empty());
    assert_eq!(summary_response.currency, "USD");
    assert!(summary_response.transaction_count >= 0);
    assert!(!summary_response.period_label.is_empty());
    assert!(!summary_response.categories.is_empty());
}

#[tokio::test]
async fn test_get_category_breakdown_success() {
    let service = create_spending_insights_service().await;
    let auth_context = create_test_auth_context();

    let mut request = Request::new(GetCategoryBreakdownRequest {
        period: 3, // Monthly
        start_date: 0,
        end_date: 0,
        currency: "USD".to_string(),
        include_subcategories: false,
    });

    request.extensions_mut().insert(auth_context);

    let response = service.get_category_breakdown(request).await;
    assert!(response.is_ok());

    let breakdown_response = response.unwrap().into_inner();
    assert!(!breakdown_response.categories.is_empty());
    assert!(!breakdown_response.total_amount.is_empty());
    assert_eq!(breakdown_response.currency, "USD");
    assert!(!breakdown_response.period_label.is_empty());
}

#[tokio::test]
async fn test_create_budget_success() {
    let service = create_spending_insights_service().await;
    let auth_context = create_test_auth_context();

    let mut request = Request::new(CreateBudgetRequest {
        category: "restaurant".to_string(),
        amount: "500.00".to_string(),
        currency: "USD".to_string(),
        period: 3, // Monthly
        alert_thresholds: vec!["80.0".to_string(), "100.0".to_string()],
        is_active: true,
    });

    request.extensions_mut().insert(auth_context);

    let response = service.create_budget(request).await;
    assert!(response.is_ok());

    let budget_response = response.unwrap().into_inner();
    assert!(budget_response.success);
    assert!(budget_response.budget.is_some());

    let budget = budget_response.budget.unwrap();
    assert_eq!(budget.category, "restaurant");
    assert_eq!(budget.amount, "500.00");
    assert_eq!(budget.currency, "USD");
    assert_eq!(budget.period, 3);
    assert!(budget.is_active);
}

#[tokio::test]
async fn test_create_budget_invalid_amount_fails() {
    let service = create_spending_insights_service().await;
    let auth_context = create_test_auth_context();

    let mut request = Request::new(CreateBudgetRequest {
        category: "restaurant".to_string(),
        amount: "-100.00".to_string(), // Invalid negative amount
        currency: "USD".to_string(),
        period: 3,
        alert_thresholds: vec!["80.0".to_string()],
        is_active: true,
    });

    request.extensions_mut().insert(auth_context);

    let response = service.create_budget(request).await;
    assert!(response.is_err());
    
    let error = response.unwrap_err();
    assert_eq!(error.code(), tonic::Code::InvalidArgument);
}

#[tokio::test]
async fn test_get_budgets_success() {
    let service = create_spending_insights_service().await;
    let auth_context = create_test_auth_context();

    // First create a budget
    let mut create_request = Request::new(CreateBudgetRequest {
        category: "grocery".to_string(),
        amount: "400.00".to_string(),
        currency: "USD".to_string(),
        period: 3,
        alert_thresholds: vec!["90.0".to_string()],
        is_active: true,
    });
    create_request.extensions_mut().insert(auth_context.clone());

    let _create_response = service.create_budget(create_request).await.unwrap();

    // Now get budgets
    let mut get_request = Request::new(GetBudgetsRequest {
        category: String::new(),
        period: 0,
        active_only: false,
    });
    get_request.extensions_mut().insert(auth_context);

    let response = service.get_budgets(get_request).await;
    assert!(response.is_ok());

    let budgets_response = response.unwrap().into_inner();
    assert!(budgets_response.total_count > 0);
    assert!(!budgets_response.budgets.is_empty());
    
    // Check that our created budget is in the list
    let grocery_budget = budgets_response.budgets.iter()
        .find(|b| b.category == "grocery");
    assert!(grocery_budget.is_some());
}

#[tokio::test]
async fn test_create_spending_alert_success() {
    let service = create_spending_insights_service().await;
    let auth_context = create_test_auth_context();

    let mut request = Request::new(CreateSpendingAlertRequest {
        r#type: 4, // LargeTransaction
        category: String::new(),
        merchant: String::new(),
        threshold_amount: "1000.00".to_string(),
        currency: "USD".to_string(),
        custom_message: "Alert for large transactions".to_string(),
    });

    request.extensions_mut().insert(auth_context);

    let response = service.create_spending_alert(request).await;
    assert!(response.is_ok());

    let alert_response = response.unwrap().into_inner();
    assert!(alert_response.success);
    assert!(alert_response.alert.is_some());

    let alert = alert_response.alert.unwrap();
    assert_eq!(alert.r#type, 4);
    assert_eq!(alert.threshold_amount, "1000.00");
    assert_eq!(alert.currency, "USD");
    assert!(alert.is_active);
}

#[tokio::test]
async fn test_get_top_merchants_success() {
    let service = create_spending_insights_service().await;
    let auth_context = create_test_auth_context();

    let mut request = Request::new(GetTopMerchantsRequest {
        period: 3, // Monthly
        start_date: 0,
        end_date: 0,
        category: String::new(),
        limit: 5,
    });

    request.extensions_mut().insert(auth_context);

    let response = service.get_top_merchants(request).await;
    assert!(response.is_ok());

    let merchants_response = response.unwrap().into_inner();
    assert!(!merchants_response.merchants.is_empty());
    assert!(!merchants_response.total_amount.is_empty());
    assert_eq!(merchants_response.currency, "USD");
    assert!(!merchants_response.period_label.is_empty());

    // Check merchant data structure
    let first_merchant = &merchants_response.merchants[0];
    assert!(!first_merchant.merchant_name.is_empty());
    assert!(!first_merchant.category.is_empty());
    assert!(!first_merchant.total_amount.is_empty());
    assert!(first_merchant.transaction_count > 0);
    assert!(first_merchant.frequency_score >= 0.0 && first_merchant.frequency_score <= 1.0);
}

#[tokio::test]
async fn test_get_platform_insights_admin_success() {
    let service = create_spending_insights_service().await;
    let mut auth_context = create_test_auth_context();
    auth_context.role = UserRole::UserRoleAdmin;

    let mut request = Request::new(GetPlatformInsightsRequest {
        period: 3, // Monthly
        start_date: 0,
        end_date: 0,
        include_trends: true,
    });

    request.extensions_mut().insert(auth_context);

    let response = service.get_platform_insights(request).await;
    assert!(response.is_ok());

    let insights_response = response.unwrap().into_inner();
    assert!(insights_response.insights.is_some());
    assert!(!insights_response.key_metrics.is_empty());
    assert!(!insights_response.growth_insights.is_empty());

    let insights = insights_response.insights.unwrap();
    assert!(insights.total_users > 0);
    assert!(!insights.total_volume.is_empty());
    assert!(!insights.currency.is_empty());
    assert!(insights.total_transactions > 0);
    assert!(!insights.top_categories.is_empty());
}

#[tokio::test]
async fn test_get_platform_insights_non_admin_fails() {
    let service = create_spending_insights_service().await;
    let auth_context = create_test_auth_context(); // Regular user, not admin

    let mut request = Request::new(GetPlatformInsightsRequest {
        period: 3,
        start_date: 0,
        end_date: 0,
        include_trends: false,
    });

    request.extensions_mut().insert(auth_context);

    let response = service.get_platform_insights(request).await;
    assert!(response.is_err());
    
    let error = response.unwrap_err();
    assert_eq!(error.code(), tonic::Code::PermissionDenied);
}

#[tokio::test]
async fn test_invalid_time_period_fails() {
    let service = create_spending_insights_service().await;
    let auth_context = create_test_auth_context();

    let mut request = Request::new(GetSpendingSummaryRequest {
        period: 999, // Invalid period
        start_date: 0,
        end_date: 0,
        currency: "USD".to_string(),
        card_ids: vec![],
    });

    request.extensions_mut().insert(auth_context);

    let response = service.get_spending_summary(request).await;
    assert!(response.is_err());
    
    let error = response.unwrap_err();
    assert_eq!(error.code(), tonic::Code::InvalidArgument);
}

#[tokio::test]
async fn test_invalid_currency_fails() {
    let service = create_spending_insights_service().await;
    let auth_context = create_test_auth_context();

    let mut request = Request::new(CreateBudgetRequest {
        category: "restaurant".to_string(),
        amount: "500.00".to_string(),
        currency: "INVALID".to_string(), // Invalid currency
        period: 3,
        alert_thresholds: vec!["80.0".to_string()],
        is_active: true,
    });

    request.extensions_mut().insert(auth_context);

    let response = service.create_budget(request).await;
    assert!(response.is_err());
    
    let error = response.unwrap_err();
    assert_eq!(error.code(), tonic::Code::InvalidArgument);
}

#[tokio::test]
async fn test_unimplemented_methods_return_unimplemented() {
    let service = create_spending_insights_service().await;
    let auth_context = create_test_auth_context();

    // Test one of the unimplemented methods
    let mut request = Request::new(GetSpendingTrendsRequest {
        period: 3,
        start_date: 0,
        end_date: 0,
        category: String::new(),
        merchant: String::new(),
    });

    request.extensions_mut().insert(auth_context);

    let response = service.get_spending_trends(request).await;
    assert!(response.is_err());
    
    let error = response.unwrap_err();
    assert_eq!(error.code(), tonic::Code::Unimplemented);
}
