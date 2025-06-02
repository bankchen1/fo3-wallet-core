//! Integration tests for the Notification service

use std::sync::Arc;
use tonic::{Request, Response, Status};
use uuid::Uuid;

use fo3_wallet_api::proto::fo3::wallet::v1::{
    notification_service_server::NotificationService,
    SendNotificationRequest, SendNotificationResponse,
    GetNotificationsRequest, GetNotificationsResponse,
    MarkAsReadRequest, MarkAsReadResponse,
    GetNotificationPreferencesRequest, GetNotificationPreferencesResponse,
    UpdateNotificationPreferencesRequest, UpdateNotificationPreferencesResponse,
    CreatePriceAlertRequest, CreatePriceAlertResponse,
    ListPriceAlertsRequest, ListPriceAlertsResponse,
    NotificationPreferences,
};
use fo3_wallet_api::services::notifications::NotificationServiceImpl;
use fo3_wallet_api::state::AppState;
use fo3_wallet_api::middleware::{
    auth::{AuthService, AuthContext, UserRole, Permission, AuthType},
    audit::AuditLogger,
};
use fo3_wallet_api::websocket::WebSocketManager;

fn create_test_auth_context() -> AuthContext {
    AuthContext {
        user_id: Uuid::new_v4().to_string(),
        username: "test_user".to_string(),
        role: UserRole::UserRoleUser,
        permissions: vec![Permission::PermissionNotificationRead],
        auth_type: AuthType::JWT("test_token".to_string()),
    }
}

fn create_admin_auth_context() -> AuthContext {
    AuthContext {
        user_id: Uuid::new_v4().to_string(),
        username: "admin_user".to_string(),
        role: UserRole::UserRoleAdmin,
        permissions: vec![
            Permission::PermissionNotificationRead,
            Permission::PermissionNotificationAdmin,
        ],
        auth_type: AuthType::JWT("admin_token".to_string()),
    }
}

async fn create_notification_service() -> NotificationServiceImpl {
    let state = Arc::new(AppState::new());
    let auth_service = Arc::new(AuthService::new(state.clone()));
    let audit_logger = Arc::new(AuditLogger::new(state.clone()));
    let websocket_manager = Arc::new(WebSocketManager::new(auth_service.clone()));

    NotificationServiceImpl::new(state, auth_service, audit_logger, websocket_manager)
}

#[tokio::test]
async fn test_send_notification_success() {
    let service = create_notification_service().await;
    let auth_context = create_test_auth_context();

    let mut request = Request::new(SendNotificationRequest {
        user_id: auth_context.user_id.clone(),
        r#type: 5, // System notification
        priority: 2, // Normal priority
        title: "Test Notification".to_string(),
        message: "This is a test notification".to_string(),
        metadata: std::collections::HashMap::new(),
        channels: vec![1, 2], // WebSocket and InApp
        expires_at: 0,
        action_url: String::new(),
        icon_url: String::new(),
    });
    request.extensions_mut().insert(auth_context);

    let response = service.send_notification(request).await;
    assert!(response.is_ok());

    let notification_response = response.unwrap().into_inner();
    assert!(notification_response.notification.is_some());
    assert!(notification_response.delivered);

    let notification = notification_response.notification.unwrap();
    assert_eq!(notification.title, "Test Notification");
    assert_eq!(notification.message, "This is a test notification");
    assert_eq!(notification.r#type, 5);
    assert_eq!(notification.priority, 2);
}

#[tokio::test]
async fn test_send_notification_permission_denied() {
    let service = create_notification_service().await;
    let auth_context = create_test_auth_context();

    let mut request = Request::new(SendNotificationRequest {
        user_id: "other_user".to_string(), // Different user
        r#type: 5,
        priority: 2,
        title: "Test Notification".to_string(),
        message: "This is a test notification".to_string(),
        metadata: std::collections::HashMap::new(),
        channels: vec![1, 2],
        expires_at: 0,
        action_url: String::new(),
        icon_url: String::new(),
    });
    request.extensions_mut().insert(auth_context);

    let response = service.send_notification(request).await;
    assert!(response.is_err());
    assert_eq!(response.unwrap_err().code(), tonic::Code::PermissionDenied);
}

#[tokio::test]
async fn test_get_notifications_success() {
    let service = create_notification_service().await;
    let auth_context = create_test_auth_context();

    // First, send a notification
    let mut send_request = Request::new(SendNotificationRequest {
        user_id: auth_context.user_id.clone(),
        r#type: 3, // Security notification
        priority: 3, // High priority
        title: "Security Alert".to_string(),
        message: "Login from new device".to_string(),
        metadata: std::collections::HashMap::new(),
        channels: vec![1, 2],
        expires_at: 0,
        action_url: String::new(),
        icon_url: String::new(),
    });
    send_request.extensions_mut().insert(auth_context.clone());

    let send_response = service.send_notification(send_request).await;
    assert!(send_response.is_ok());

    // Now get notifications
    let mut get_request = Request::new(GetNotificationsRequest {
        user_id: auth_context.user_id.clone(),
        type_filter: vec![],
        unread_only: false,
        page_size: 10,
        page_token: String::new(),
        since_timestamp: 0,
    });
    get_request.extensions_mut().insert(auth_context);

    let response = service.get_notifications(get_request).await;
    assert!(response.is_ok());

    let notifications_response = response.unwrap().into_inner();
    assert!(!notifications_response.notifications.is_empty());
    assert_eq!(notifications_response.total_count, 1);
    assert_eq!(notifications_response.unread_count, 1);

    let notification = &notifications_response.notifications[0];
    assert_eq!(notification.title, "Security Alert");
    assert_eq!(notification.r#type, 3);
    assert!(!notification.is_read);
}

#[tokio::test]
async fn test_mark_as_read_success() {
    let service = create_notification_service().await;
    let auth_context = create_test_auth_context();

    // Send a notification first
    let mut send_request = Request::new(SendNotificationRequest {
        user_id: auth_context.user_id.clone(),
        r#type: 1, // Fiat transaction
        priority: 2,
        title: "Transaction Complete".to_string(),
        message: "Your deposit has been processed".to_string(),
        metadata: std::collections::HashMap::new(),
        channels: vec![1, 2],
        expires_at: 0,
        action_url: String::new(),
        icon_url: String::new(),
    });
    send_request.extensions_mut().insert(auth_context.clone());

    let send_response = service.send_notification(send_request).await;
    assert!(send_response.is_ok());

    // Mark all notifications as read
    let mut mark_request = Request::new(MarkAsReadRequest {
        user_id: auth_context.user_id.clone(),
        notification_ids: vec![], // Empty = mark all as read
    });
    mark_request.extensions_mut().insert(auth_context);

    let response = service.mark_as_read(mark_request).await;
    assert!(response.is_ok());

    let mark_response = response.unwrap().into_inner();
    assert_eq!(mark_response.marked_count, 1);
}

#[tokio::test]
async fn test_notification_preferences() {
    let service = create_notification_service().await;
    let auth_context = create_test_auth_context();

    // Get default preferences
    let mut get_request = Request::new(GetNotificationPreferencesRequest {
        user_id: auth_context.user_id.clone(),
    });
    get_request.extensions_mut().insert(auth_context.clone());

    let response = service.get_notification_preferences(get_request).await;
    assert!(response.is_ok());

    let prefs_response = response.unwrap().into_inner();
    assert!(prefs_response.preferences.is_some());

    let preferences = prefs_response.preferences.unwrap();
    assert!(preferences.fiat_transaction_enabled);
    assert!(preferences.security_alerts_enabled);

    // Update preferences
    let updated_prefs = NotificationPreferences {
        user_id: auth_context.user_id.clone(),
        fiat_transaction_enabled: false, // Disable fiat notifications
        kyc_status_enabled: true,
        security_alerts_enabled: true,
        price_alerts_enabled: false, // Disable price alerts
        system_announcements_enabled: true,
        card_notifications_enabled: true,
        reward_notifications_enabled: true,
        preferred_channels: vec![2], // Only in-app
        quiet_hours_enabled: true,
        quiet_hours_start: 22,
        quiet_hours_end: 8,
        timezone: "UTC".to_string(),
        updated_at: chrono::Utc::now().timestamp(),
    };

    let mut update_request = Request::new(UpdateNotificationPreferencesRequest {
        user_id: auth_context.user_id.clone(),
        preferences: Some(updated_prefs),
    });
    update_request.extensions_mut().insert(auth_context);

    let response = service.update_notification_preferences(update_request).await;
    assert!(response.is_ok());

    let update_response = response.unwrap().into_inner();
    assert!(update_response.preferences.is_some());

    let updated = update_response.preferences.unwrap();
    assert!(!updated.fiat_transaction_enabled);
    assert!(!updated.price_alerts_enabled);
    assert!(updated.quiet_hours_enabled);
    assert_eq!(updated.preferred_channels, vec![2]);
}

#[tokio::test]
async fn test_price_alerts() {
    let service = create_notification_service().await;
    let auth_context = create_test_auth_context();

    // Create a price alert
    let mut create_request = Request::new(CreatePriceAlertRequest {
        user_id: auth_context.user_id.clone(),
        symbol: "BTC".to_string(),
        quote_currency: "USD".to_string(),
        condition: 1, // Above
        threshold_value: "50000.00".to_string(),
        is_repeating: false,
        max_triggers: 1,
        expires_at: 0,
        note: "BTC price alert".to_string(),
    });
    create_request.extensions_mut().insert(auth_context.clone());

    let response = service.create_price_alert(create_request).await;
    assert!(response.is_ok());

    let create_response = response.unwrap().into_inner();
    assert!(create_response.alert.is_some());

    let alert = create_response.alert.unwrap();
    assert_eq!(alert.symbol, "BTC");
    assert_eq!(alert.threshold_value, "50000.00");
    assert_eq!(alert.condition, 1);
    assert!(alert.is_active);

    // List price alerts
    let mut list_request = Request::new(ListPriceAlertsRequest {
        user_id: auth_context.user_id.clone(),
        active_only: true,
        page_size: 10,
        page_token: String::new(),
    });
    list_request.extensions_mut().insert(auth_context);

    let response = service.list_price_alerts(list_request).await;
    assert!(response.is_ok());

    let list_response = response.unwrap().into_inner();
    assert_eq!(list_response.total_count, 1);
    assert!(!list_response.alerts.is_empty());

    let listed_alert = &list_response.alerts[0];
    assert_eq!(listed_alert.symbol, "BTC");
    assert_eq!(listed_alert.threshold_value, "50000.00");
}

#[tokio::test]
async fn test_authentication_required() {
    let service = create_notification_service().await;

    // Request without authentication context
    let request = Request::new(GetNotificationsRequest {
        user_id: "test_user".to_string(),
        type_filter: vec![],
        unread_only: false,
        page_size: 10,
        page_token: String::new(),
        since_timestamp: 0,
    });

    let response = service.get_notifications(request).await;
    assert!(response.is_err());
    assert_eq!(response.unwrap_err().code(), tonic::Code::Unauthenticated);
}

#[tokio::test]
async fn test_invalid_notification_type() {
    let service = create_notification_service().await;
    let auth_context = create_test_auth_context();

    let mut request = Request::new(SendNotificationRequest {
        user_id: auth_context.user_id.clone(),
        r#type: 99, // Invalid type
        priority: 2,
        title: "Test".to_string(),
        message: "Test".to_string(),
        metadata: std::collections::HashMap::new(),
        channels: vec![1],
        expires_at: 0,
        action_url: String::new(),
        icon_url: String::new(),
    });
    request.extensions_mut().insert(auth_context);

    let response = service.send_notification(request).await;
    assert!(response.is_err());
    assert_eq!(response.unwrap_err().code(), tonic::Code::InvalidArgument);
}

#[tokio::test]
async fn test_price_alert_validation() {
    let service = create_notification_service().await;
    let auth_context = create_test_auth_context();

    // Test invalid threshold value
    let mut request = Request::new(CreatePriceAlertRequest {
        user_id: auth_context.user_id.clone(),
        symbol: "BTC".to_string(),
        quote_currency: "USD".to_string(),
        condition: 1,
        threshold_value: "invalid_number".to_string(),
        is_repeating: false,
        max_triggers: 1,
        expires_at: 0,
        note: String::new(),
    });
    request.extensions_mut().insert(auth_context.clone());

    let response = service.create_price_alert(request).await;
    assert!(response.is_err());
    assert_eq!(response.unwrap_err().code(), tonic::Code::InvalidArgument);

    // Test negative threshold value
    let mut request = Request::new(CreatePriceAlertRequest {
        user_id: auth_context.user_id.clone(),
        symbol: "BTC".to_string(),
        quote_currency: "USD".to_string(),
        condition: 1,
        threshold_value: "-100.00".to_string(),
        is_repeating: false,
        max_triggers: 1,
        expires_at: 0,
        note: String::new(),
    });
    request.extensions_mut().insert(auth_context);

    let response = service.create_price_alert(request).await;
    assert!(response.is_err());
    assert_eq!(response.unwrap_err().code(), tonic::Code::InvalidArgument);
}
