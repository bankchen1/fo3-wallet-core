//! Integration tests for the Pricing service

use std::sync::Arc;
use tonic::{Request, Response, Status};
use uuid::Uuid;

use fo3_wallet_api::proto::fo3::wallet::v1::{
    pricing_service_server::PricingService,
    GetPriceRequest, GetPriceResponse,
    GetPriceBatchRequest, GetPriceBatchResponse,
    ListSupportedSymbolsRequest, ListSupportedSymbolsResponse,
    GetFiatRateRequest, GetFiatRateResponse,
};
use fo3_wallet_api::services::pricing::PricingServiceImpl;
use fo3_wallet_api::state::AppState;
use fo3_wallet_api::middleware::{
    auth::{AuthService, AuthContext, UserRole, Permission, AuthType},
    audit::AuditLogger,
    pricing_guard::PricingGuard,
};

fn create_test_auth_context() -> AuthContext {
    AuthContext {
        user_id: Uuid::new_v4().to_string(),
        username: "test_user".to_string(),
        role: UserRole::UserRoleUser,
        permissions: vec![Permission::PermissionPricingRead],
        auth_type: AuthType::JWT("test_token".to_string()),
    }
}

fn create_admin_auth_context() -> AuthContext {
    AuthContext {
        user_id: Uuid::new_v4().to_string(),
        username: "admin_user".to_string(),
        role: UserRole::UserRoleAdmin,
        permissions: vec![
            Permission::PermissionPricingRead,
            Permission::PermissionPricingAdmin,
        ],
        auth_type: AuthType::JWT("admin_token".to_string()),
    }
}

async fn create_pricing_service() -> PricingServiceImpl {
    let state = Arc::new(AppState::new());
    let auth_service = Arc::new(AuthService::new(state.clone()));
    let audit_logger = Arc::new(AuditLogger::new(state.clone()));
    let pricing_guard = Arc::new(PricingGuard::new(state.clone(), auth_service.clone(), None));

    PricingServiceImpl::new(state, auth_service, audit_logger, pricing_guard)
}

#[tokio::test]
async fn test_get_price_success() {
    let service = create_pricing_service().await;
    let auth_context = create_test_auth_context();

    let mut request = Request::new(GetPriceRequest {
        symbol: "BTC".to_string(),
        quote_currency: "USD".to_string(),
        chain: String::new(),
        contract_address: String::new(),
    });
    request.extensions_mut().insert(auth_context);

    let response = service.get_price(request).await;
    assert!(response.is_ok());

    let price_response = response.unwrap().into_inner();
    assert!(price_response.price.is_some());

    let price = price_response.price.unwrap();
    assert_eq!(price.symbol, "BTC");
    assert!(!price.price_usd.is_empty());
}

#[tokio::test]
async fn test_get_price_invalid_symbol() {
    let service = create_pricing_service().await;
    let auth_context = create_test_auth_context();

    let mut request = Request::new(GetPriceRequest {
        symbol: "INVALID@SYMBOL".to_string(),
        quote_currency: "USD".to_string(),
        chain: String::new(),
        contract_address: String::new(),
    });
    request.extensions_mut().insert(auth_context);

    let response = service.get_price(request).await;
    assert!(response.is_err());
    assert_eq!(response.unwrap_err().code(), tonic::Code::InvalidArgument);
}

#[tokio::test]
async fn test_get_price_batch_success() {
    let service = create_pricing_service().await;
    let auth_context = create_test_auth_context();

    let mut request = Request::new(GetPriceBatchRequest {
        symbols: vec!["BTC".to_string(), "ETH".to_string(), "USDT".to_string()],
        quote_currency: "USD".to_string(),
        include_metadata: true,
    });
    request.extensions_mut().insert(auth_context);

    let response = service.get_price_batch(request).await;
    assert!(response.is_ok());

    let batch_response = response.unwrap().into_inner();
    assert_eq!(batch_response.total_count, 3);
    assert!(batch_response.successful_count > 0);
    assert!(!batch_response.prices.is_empty());
}

#[tokio::test]
async fn test_get_price_batch_too_large() {
    let service = create_pricing_service().await;
    let auth_context = create_test_auth_context();

    // Create a batch that's too large (over 100 symbols)
    let large_symbols: Vec<String> = (0..101).map(|i| format!("TOKEN{}", i)).collect();

    let mut request = Request::new(GetPriceBatchRequest {
        symbols: large_symbols,
        quote_currency: "USD".to_string(),
        include_metadata: false,
    });
    request.extensions_mut().insert(auth_context);

    let response = service.get_price_batch(request).await;
    assert!(response.is_err());
    assert_eq!(response.unwrap_err().code(), tonic::Code::InvalidArgument);
}

#[tokio::test]
async fn test_get_fiat_rate_success() {
    let service = create_pricing_service().await;
    let auth_context = create_test_auth_context();

    let mut request = Request::new(GetFiatRateRequest {
        from_currency: "USD".to_string(),
        to_currency: "EUR".to_string(),
    });
    request.extensions_mut().insert(auth_context);

    let response = service.get_fiat_rate(request).await;
    assert!(response.is_ok());

    let rate_response = response.unwrap().into_inner();
    assert!(rate_response.rate.is_some());

    let rate = rate_response.rate.unwrap();
    assert_eq!(rate.from_currency, "USD");
    assert_eq!(rate.to_currency, "EUR");
    assert!(!rate.rate.is_empty());
}

#[tokio::test]
async fn test_list_supported_symbols_success() {
    let service = create_pricing_service().await;
    let auth_context = create_test_auth_context();

    let mut request = Request::new(ListSupportedSymbolsRequest {
        type_filter: 0, // No filter
        chain_filter: String::new(),
        active_only: true,
        page_size: 50,
        page_token: String::new(),
    });
    request.extensions_mut().insert(auth_context);

    let response = service.list_supported_symbols(request).await;
    assert!(response.is_ok());

    let symbols_response = response.unwrap().into_inner();
    assert!(symbols_response.total_count > 0);
    assert!(!symbols_response.assets.is_empty());

    // Check that we have some expected assets
    let symbols: Vec<String> = symbols_response.assets.iter().map(|a| a.symbol.clone()).collect();
    assert!(symbols.contains(&"BTC".to_string()));
    assert!(symbols.contains(&"ETH".to_string()));
    assert!(symbols.contains(&"USD".to_string()));
}

#[tokio::test]
async fn test_list_supported_symbols_with_filter() {
    let service = create_pricing_service().await;
    let auth_context = create_test_auth_context();

    let mut request = Request::new(ListSupportedSymbolsRequest {
        type_filter: 1, // Cryptocurrency filter
        chain_filter: String::new(),
        active_only: true,
        page_size: 50,
        page_token: String::new(),
    });
    request.extensions_mut().insert(auth_context);

    let response = service.list_supported_symbols(request).await;
    assert!(response.is_ok());

    let symbols_response = response.unwrap().into_inner();
    assert!(symbols_response.total_count > 0);

    // All returned assets should be cryptocurrencies
    for asset in symbols_response.assets {
        assert_eq!(asset.r#type, 1); // AssetType::Cryptocurrency
    }
}

#[tokio::test]
async fn test_authentication_required() {
    let service = create_pricing_service().await;

    // Request without authentication context
    let request = Request::new(GetPriceRequest {
        symbol: "BTC".to_string(),
        quote_currency: "USD".to_string(),
        chain: String::new(),
        contract_address: String::new(),
    });

    let response = service.get_price(request).await;
    assert!(response.is_err());
    assert_eq!(response.unwrap_err().code(), tonic::Code::Unauthenticated);
}

#[tokio::test]
async fn test_invalid_quote_currency() {
    let service = create_pricing_service().await;
    let auth_context = create_test_auth_context();

    let mut request = Request::new(GetPriceRequest {
        symbol: "BTC".to_string(),
        quote_currency: "INVALID".to_string(),
        chain: String::new(),
        contract_address: String::new(),
    });
    request.extensions_mut().insert(auth_context);

    let response = service.get_price(request).await;
    assert!(response.is_err());
    assert_eq!(response.unwrap_err().code(), tonic::Code::InvalidArgument);
}

#[tokio::test]
async fn test_empty_symbol() {
    let service = create_pricing_service().await;
    let auth_context = create_test_auth_context();

    let mut request = Request::new(GetPriceRequest {
        symbol: String::new(),
        quote_currency: "USD".to_string(),
        chain: String::new(),
        contract_address: String::new(),
    });
    request.extensions_mut().insert(auth_context);

    let response = service.get_price(request).await;
    assert!(response.is_err());
    assert_eq!(response.unwrap_err().code(), tonic::Code::InvalidArgument);
}

#[tokio::test]
async fn test_pricing_caching() {
    let service = create_pricing_service().await;
    let auth_context = create_test_auth_context();

    // First request
    let mut request1 = Request::new(GetPriceRequest {
        symbol: "BTC".to_string(),
        quote_currency: "USD".to_string(),
        chain: String::new(),
        contract_address: String::new(),
    });
    request1.extensions_mut().insert(auth_context.clone());

    let response1 = service.get_price(request1).await;
    assert!(response1.is_ok());

    // Second request (should hit cache)
    let mut request2 = Request::new(GetPriceRequest {
        symbol: "BTC".to_string(),
        quote_currency: "USD".to_string(),
        chain: String::new(),
        contract_address: String::new(),
    });
    request2.extensions_mut().insert(auth_context);

    let response2 = service.get_price(request2).await;
    assert!(response2.is_ok());

    // Both responses should have the same price (from cache)
    let price1 = response1.unwrap().into_inner().price.unwrap();
    let price2 = response2.unwrap().into_inner().price.unwrap();
    assert_eq!(price1.price_usd, price2.price_usd);
}
