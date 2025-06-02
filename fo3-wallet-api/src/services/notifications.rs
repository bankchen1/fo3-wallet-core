//! Notification service implementation

use std::sync::Arc;
use std::collections::HashMap;
use std::str::FromStr;
use tonic::{Request, Response, Status};
use uuid::Uuid;
use rust_decimal::Decimal;
use chrono::{DateTime, Utc};

use crate::proto::fo3::wallet::v1::{
    notification_service_server::NotificationService,
    *,
};
use crate::state::AppState;
use crate::middleware::{
    auth::{AuthContext, AuthService},
    audit::AuditLogger,
};
use crate::models::notifications::{
    Notification, NotificationPreferences, PriceAlert, NotificationMetrics,
    NotificationType, NotificationPriority, DeliveryChannel, PriceAlertCondition,
    NotificationRepository, InMemoryNotificationRepository, NotificationEventData,
    NotificationDelivery, DeliveryStatus,
};
use crate::websocket::WebSocketManager;

/// Notification service implementation
pub struct NotificationServiceImpl {
    state: Arc<AppState>,
    auth_service: Arc<AuthService>,
    audit_logger: Arc<AuditLogger>,
    repository: Arc<dyn NotificationRepository>,
    websocket_manager: Arc<WebSocketManager>,
}

impl NotificationServiceImpl {
    pub fn new(
        state: Arc<AppState>,
        auth_service: Arc<AuthService>,
        audit_logger: Arc<AuditLogger>,
        websocket_manager: Arc<WebSocketManager>,
    ) -> Self {
        let repository = Arc::new(InMemoryNotificationRepository::new());

        Self {
            state,
            auth_service,
            audit_logger,
            repository,
            websocket_manager,
        }
    }

    /// Send notification via WebSocket if user is connected
    async fn send_websocket_notification(&self, notification: &Notification) -> bool {
        if notification.channels.contains(&DeliveryChannel::WebSocket) {
            // Create WebSocket message
            let message = serde_json::json!({
                "type": "notification",
                "data": {
                    "id": notification.id,
                    "type": format!("{:?}", notification.notification_type),
                    "priority": format!("{:?}", notification.priority),
                    "title": notification.title,
                    "message": notification.message,
                    "metadata": notification.metadata,
                    "created_at": notification.created_at.timestamp(),
                    "action_url": notification.action_url,
                    "icon_url": notification.icon_url,
                }
            });

            if let Ok(message_str) = serde_json::to_string(&message) {
                return self.websocket_manager.send_to_user(&notification.user_id, &message_str).await;
            }
        }
        false
    }

    /// Record notification delivery attempt
    async fn record_delivery(&self, notification_id: &str, channel: DeliveryChannel, success: bool) {
        let delivery = NotificationDelivery {
            notification_id: notification_id.to_string(),
            channel,
            status: if success { DeliveryStatus::Delivered } else { DeliveryStatus::Failed },
            attempted_at: Utc::now(),
            delivered_at: if success { Some(Utc::now()) } else { None },
            error_message: if success { None } else { Some("Delivery failed".to_string()) },
            retry_count: 0,
        };

        let _ = self.repository.record_delivery(&delivery).await;
    }

    /// Check if user should receive notification based on preferences
    async fn should_send_notification(&self, user_id: &str, notification_type: &NotificationType) -> bool {
        if let Some(preferences) = self.repository.get_user_preferences(user_id).await {
            match notification_type {
                NotificationType::FiatTransaction => preferences.fiat_transaction_enabled,
                NotificationType::KycStatus => preferences.kyc_status_enabled,
                NotificationType::Security => preferences.security_alerts_enabled,
                NotificationType::PriceAlert => preferences.price_alerts_enabled,
                NotificationType::System => preferences.system_announcements_enabled,
                NotificationType::Card => preferences.card_notifications_enabled,
                NotificationType::Reward => preferences.reward_notifications_enabled,
            }
        } else {
            // Default to enabled if no preferences set
            true
        }
    }

    /// Check if notification should be sent during quiet hours
    async fn is_quiet_hours(&self, user_id: &str) -> bool {
        if let Some(preferences) = self.repository.get_user_preferences(user_id).await {
            if preferences.quiet_hours_enabled {
                let now = Utc::now();
                // This is a simplified check - in production, we'd use the user's timezone
                let current_hour = now.hour() as u8;
                
                if preferences.quiet_hours_start <= preferences.quiet_hours_end {
                    // Normal range (e.g., 22:00 to 08:00 next day)
                    current_hour >= preferences.quiet_hours_start && current_hour < preferences.quiet_hours_end
                } else {
                    // Overnight range (e.g., 22:00 to 08:00 next day)
                    current_hour >= preferences.quiet_hours_start || current_hour < preferences.quiet_hours_end
                }
            } else {
                false
            }
        } else {
            false
        }
    }

    /// Convert internal notification to proto
    fn notification_to_proto(&self, notification: &Notification) -> crate::proto::fo3::wallet::v1::Notification {
        crate::proto::fo3::wallet::v1::Notification {
            id: notification.id.clone(),
            user_id: notification.user_id.clone(),
            r#type: match notification.notification_type {
                NotificationType::FiatTransaction => 1,
                NotificationType::KycStatus => 2,
                NotificationType::Security => 3,
                NotificationType::PriceAlert => 4,
                NotificationType::System => 5,
                NotificationType::Card => 6,
                NotificationType::Reward => 7,
            },
            priority: match notification.priority {
                NotificationPriority::Low => 1,
                NotificationPriority::Normal => 2,
                NotificationPriority::High => 3,
                NotificationPriority::Urgent => 4,
            },
            title: notification.title.clone(),
            message: notification.message.clone(),
            metadata: notification.metadata.clone(),
            channels: notification.channels.iter().map(|c| match c {
                DeliveryChannel::WebSocket => 1,
                DeliveryChannel::InApp => 2,
                DeliveryChannel::Email => 3,
                DeliveryChannel::Sms => 4,
                DeliveryChannel::Push => 5,
            }).collect(),
            is_read: notification.is_read,
            created_at: notification.created_at.timestamp(),
            read_at: notification.read_at.map(|t| t.timestamp()).unwrap_or(0),
            expires_at: notification.expires_at.map(|t| t.timestamp()).unwrap_or(0),
            action_url: notification.action_url.clone().unwrap_or_default(),
            icon_url: notification.icon_url.clone().unwrap_or_default(),
        }
    }

    /// Convert internal price alert to proto
    fn price_alert_to_proto(&self, alert: &PriceAlert) -> crate::proto::fo3::wallet::v1::PriceAlert {
        crate::proto::fo3::wallet::v1::PriceAlert {
            id: alert.id.clone(),
            user_id: alert.user_id.clone(),
            symbol: alert.symbol.clone(),
            quote_currency: alert.quote_currency.clone(),
            condition: match alert.condition {
                PriceAlertCondition::Above => 1,
                PriceAlertCondition::Below => 2,
                PriceAlertCondition::ChangePercent => 3,
            },
            threshold_value: alert.threshold_value.to_string(),
            is_active: alert.is_active,
            is_repeating: alert.is_repeating,
            trigger_count: alert.trigger_count,
            max_triggers: alert.max_triggers,
            created_at: alert.created_at.timestamp(),
            last_triggered_at: alert.last_triggered_at.map(|t| t.timestamp()).unwrap_or(0),
            expires_at: alert.expires_at.map(|t| t.timestamp()).unwrap_or(0),
            note: alert.note.clone().unwrap_or_default(),
        }
    }

    /// Convert internal preferences to proto
    fn preferences_to_proto(&self, prefs: &NotificationPreferences) -> crate::proto::fo3::wallet::v1::NotificationPreferences {
        crate::proto::fo3::wallet::v1::NotificationPreferences {
            user_id: prefs.user_id.clone(),
            fiat_transaction_enabled: prefs.fiat_transaction_enabled,
            kyc_status_enabled: prefs.kyc_status_enabled,
            security_alerts_enabled: prefs.security_alerts_enabled,
            price_alerts_enabled: prefs.price_alerts_enabled,
            system_announcements_enabled: prefs.system_announcements_enabled,
            card_notifications_enabled: prefs.card_notifications_enabled,
            reward_notifications_enabled: prefs.reward_notifications_enabled,
            preferred_channels: prefs.preferred_channels.iter().map(|c| match c {
                DeliveryChannel::WebSocket => 1,
                DeliveryChannel::InApp => 2,
                DeliveryChannel::Email => 3,
                DeliveryChannel::Sms => 4,
                DeliveryChannel::Push => 5,
            }).collect(),
            quiet_hours_enabled: prefs.quiet_hours_enabled,
            quiet_hours_start: prefs.quiet_hours_start as i32,
            quiet_hours_end: prefs.quiet_hours_end as i32,
            timezone: prefs.timezone.clone(),
            updated_at: prefs.updated_at.timestamp(),
        }
    }

    /// Convert internal metrics to proto
    fn metrics_to_proto(&self, metrics: &NotificationMetrics) -> crate::proto::fo3::wallet::v1::NotificationMetrics {
        crate::proto::fo3::wallet::v1::NotificationMetrics {
            total_notifications_sent: metrics.total_notifications_sent,
            websocket_deliveries: metrics.websocket_deliveries,
            in_app_deliveries: metrics.in_app_deliveries,
            failed_deliveries: metrics.failed_deliveries,
            delivery_success_rate: metrics.delivery_success_rate,
            active_websocket_connections: metrics.active_websocket_connections,
            active_price_alerts: metrics.active_price_alerts,
            notifications_last_24h: metrics.notifications_last_24h,
            notifications_by_type: metrics.notifications_by_type.clone(),
            notifications_by_priority: metrics.notifications_by_priority.clone(),
            average_delivery_time_ms: metrics.average_delivery_time_ms,
        }
    }

    /// Create notification from event data
    pub async fn create_notification_from_event(
        &self,
        user_id: &str,
        event_data: NotificationEventData,
    ) -> Result<Notification, String> {
        let (notification_type, priority, title, message, metadata) = match event_data {
            NotificationEventData::FiatTransaction { transaction_id, transaction_type, status, amount, currency } => {
                let title = format!("{} {}", 
                    transaction_type.chars().next().unwrap().to_uppercase().collect::<String>() + &transaction_type[1..],
                    match status.as_str() {
                        "completed" => "Completed",
                        "approved" => "Approved",
                        "rejected" => "Rejected",
                        "pending" => "Pending",
                        _ => "Updated",
                    }
                );
                let message = format!("Your {} of {} {} has been {}", transaction_type, amount, currency, status);
                let mut metadata = HashMap::new();
                metadata.insert("transaction_id".to_string(), transaction_id);
                metadata.insert("transaction_type".to_string(), transaction_type);
                metadata.insert("status".to_string(), status.clone());
                metadata.insert("amount".to_string(), amount.to_string());
                metadata.insert("currency".to_string(), currency);
                
                let priority = match status.as_str() {
                    "rejected" => NotificationPriority::High,
                    "completed" | "approved" => NotificationPriority::Normal,
                    _ => NotificationPriority::Low,
                };
                
                (NotificationType::FiatTransaction, priority, title, message, metadata)
            },
            NotificationEventData::KycStatus { submission_id, status, rejection_reason, required_documents } => {
                let title = format!("KYC Verification {}", 
                    match status.as_str() {
                        "approved" => "Approved",
                        "rejected" => "Rejected",
                        "requires_update" => "Requires Update",
                        _ => "Updated",
                    }
                );
                let message = if status == "approved" {
                    "Your identity verification has been approved. You can now access all features.".to_string()
                } else if status == "rejected" {
                    format!("Your identity verification was rejected. Reason: {}", 
                        rejection_reason.unwrap_or_else(|| "Please contact support".to_string()))
                } else if status == "requires_update" {
                    format!("Additional documents required: {}", required_documents.join(", "))
                } else {
                    "Your KYC status has been updated.".to_string()
                };
                
                let mut metadata = HashMap::new();
                metadata.insert("submission_id".to_string(), submission_id);
                metadata.insert("status".to_string(), status.clone());
                if let Some(reason) = rejection_reason {
                    metadata.insert("rejection_reason".to_string(), reason);
                }
                if !required_documents.is_empty() {
                    metadata.insert("required_documents".to_string(), required_documents.join(","));
                }
                
                let priority = match status.as_str() {
                    "approved" => NotificationPriority::High,
                    "rejected" => NotificationPriority::High,
                    "requires_update" => NotificationPriority::Normal,
                    _ => NotificationPriority::Low,
                };
                
                (NotificationType::KycStatus, priority, title, message, metadata)
            },
            NotificationEventData::Security { event_type, ip_address, device_info, location } => {
                let title = match event_type.as_str() {
                    "login" => "New Login Detected",
                    "password_change" => "Password Changed",
                    "api_key_created" => "New API Key Created",
                    "api_key_revoked" => "API Key Revoked",
                    _ => "Security Event",
                };
                let message = match event_type.as_str() {
                    "login" => format!("New login from {}", 
                        location.as_deref().unwrap_or_else(|| ip_address.as_deref().unwrap_or("unknown location"))),
                    "password_change" => "Your password has been successfully changed.".to_string(),
                    "api_key_created" => "A new API key has been created for your account.".to_string(),
                    "api_key_revoked" => "An API key has been revoked from your account.".to_string(),
                    _ => "A security event occurred on your account.".to_string(),
                };
                
                let mut metadata = HashMap::new();
                metadata.insert("event_type".to_string(), event_type);
                if let Some(ip) = ip_address {
                    metadata.insert("ip_address".to_string(), ip);
                }
                if let Some(device) = device_info {
                    metadata.insert("device_info".to_string(), device);
                }
                if let Some(loc) = location {
                    metadata.insert("location".to_string(), loc);
                }
                
                (NotificationType::Security, NotificationPriority::High, title.to_string(), message, metadata)
            },
            NotificationEventData::PriceAlert { symbol, current_price, threshold_price, condition, change_percent } => {
                let condition_text = match condition {
                    PriceAlertCondition::Above => "above",
                    PriceAlertCondition::Below => "below",
                    PriceAlertCondition::ChangePercent => "changed by",
                };
                let title = format!("{} Price Alert", symbol);
                let message = if let Some(change) = change_percent {
                    format!("{} price has changed by {}% (current: ${})", symbol, change, current_price)
                } else {
                    format!("{} price is now {} ${} (threshold: ${})", symbol, condition_text, current_price, threshold_price)
                };
                
                let mut metadata = HashMap::new();
                metadata.insert("symbol".to_string(), symbol);
                metadata.insert("current_price".to_string(), current_price.to_string());
                metadata.insert("threshold_price".to_string(), threshold_price.to_string());
                metadata.insert("condition".to_string(), format!("{:?}", condition));
                if let Some(change) = change_percent {
                    metadata.insert("change_percent".to_string(), change.to_string());
                }
                
                (NotificationType::PriceAlert, NotificationPriority::Normal, title, message, metadata)
            },
            NotificationEventData::System { announcement_type, severity, affected_services } => {
                let title = match announcement_type.as_str() {
                    "maintenance" => "Scheduled Maintenance",
                    "feature" => "New Feature Available",
                    "security" => "Security Update",
                    _ => "System Announcement",
                };
                let message = match announcement_type.as_str() {
                    "maintenance" => format!("Scheduled maintenance affecting: {}", affected_services.join(", ")),
                    "feature" => "New features have been added to your account.".to_string(),
                    "security" => "Important security updates have been applied.".to_string(),
                    _ => "System announcement".to_string(),
                };
                
                let mut metadata = HashMap::new();
                metadata.insert("announcement_type".to_string(), announcement_type);
                metadata.insert("severity".to_string(), severity.clone());
                metadata.insert("affected_services".to_string(), affected_services.join(","));
                
                let priority = match severity.as_str() {
                    "critical" => NotificationPriority::Urgent,
                    "high" => NotificationPriority::High,
                    "medium" => NotificationPriority::Normal,
                    _ => NotificationPriority::Low,
                };
                
                (NotificationType::System, priority, title.to_string(), message, metadata)
            },
            NotificationEventData::Card { card_id, event_type, amount, merchant } => {
                let title = match event_type.as_str() {
                    "issued" => "Card Issued",
                    "activated" => "Card Activated",
                    "blocked" => "Card Blocked",
                    "transaction" => "Card Transaction",
                    _ => "Card Update",
                };
                let message = match event_type.as_str() {
                    "issued" => "Your FO3 Cash Card has been issued.".to_string(),
                    "activated" => "Your FO3 Cash Card has been activated.".to_string(),
                    "blocked" => "Your FO3 Cash Card has been blocked for security.".to_string(),
                    "transaction" => format!("Card transaction: {} at {}", 
                        amount.map(|a| format!("${}", a)).unwrap_or_else(|| "Amount".to_string()),
                        merchant.as_deref().unwrap_or("merchant")),
                    _ => "Your card status has been updated.".to_string(),
                };
                
                let mut metadata = HashMap::new();
                metadata.insert("card_id".to_string(), card_id);
                metadata.insert("event_type".to_string(), event_type.clone());
                if let Some(amt) = amount {
                    metadata.insert("amount".to_string(), amt.to_string());
                }
                if let Some(merch) = merchant {
                    metadata.insert("merchant".to_string(), merch);
                }
                
                let priority = match event_type.as_str() {
                    "blocked" => NotificationPriority::High,
                    "transaction" => NotificationPriority::Normal,
                    _ => NotificationPriority::Low,
                };
                
                (NotificationType::Card, priority, title.to_string(), message, metadata)
            },
            NotificationEventData::Reward { reward_type, amount, currency, source_transaction_id } => {
                let title = match reward_type.as_str() {
                    "cashback" => "Cashback Earned",
                    "referral" => "Referral Bonus",
                    "deposit_bonus" => "Deposit Bonus",
                    _ => "Reward Earned",
                };
                let message = format!("You've earned {} {} {}", amount, currency, 
                    match reward_type.as_str() {
                        "cashback" => "in cashback",
                        "referral" => "referral bonus",
                        "deposit_bonus" => "deposit bonus",
                        _ => "reward",
                    }
                );
                
                let mut metadata = HashMap::new();
                metadata.insert("reward_type".to_string(), reward_type);
                metadata.insert("amount".to_string(), amount.to_string());
                metadata.insert("currency".to_string(), currency);
                if let Some(tx_id) = source_transaction_id {
                    metadata.insert("source_transaction_id".to_string(), tx_id);
                }
                
                (NotificationType::Reward, NotificationPriority::Normal, title.to_string(), message, metadata)
            },
        };

        let notification = Notification::new(
            user_id.to_string(),
            notification_type,
            priority,
            title,
            message,
        ).with_metadata(metadata);

        Ok(notification)
    }
}

#[async_trait::async_trait]
impl NotificationService for NotificationServiceImpl {
    async fn send_notification(
        &self,
        request: Request<SendNotificationRequest>,
    ) -> Result<Response<SendNotificationResponse>, Status> {
        let auth_context = request.extensions().get::<AuthContext>()
            .ok_or_else(|| Status::unauthenticated("Authentication required"))?;

        let req = request.into_inner();

        // Validate user_id (admin can send to any user, regular users can only send to themselves)
        if auth_context.role != crate::middleware::auth::UserRole::UserRoleAdmin && auth_context.user_id != req.user_id {
            return Err(Status::permission_denied("Cannot send notifications to other users"));
        }

        // Convert proto types to internal types
        let notification_type = match req.r#type {
            1 => NotificationType::FiatTransaction,
            2 => NotificationType::KycStatus,
            3 => NotificationType::Security,
            4 => NotificationType::PriceAlert,
            5 => NotificationType::System,
            6 => NotificationType::Card,
            7 => NotificationType::Reward,
            _ => return Err(Status::invalid_argument("Invalid notification type")),
        };

        let priority = match req.priority {
            1 => NotificationPriority::Low,
            2 => NotificationPriority::Normal,
            3 => NotificationPriority::High,
            4 => NotificationPriority::Urgent,
            _ => NotificationPriority::Normal,
        };

        let channels: Vec<DeliveryChannel> = req.channels.iter().filter_map(|&c| match c {
            1 => Some(DeliveryChannel::WebSocket),
            2 => Some(DeliveryChannel::InApp),
            3 => Some(DeliveryChannel::Email),
            4 => Some(DeliveryChannel::Sms),
            5 => Some(DeliveryChannel::Push),
            _ => None,
        }).collect();

        // Check if user should receive this type of notification
        if !self.should_send_notification(&req.user_id, &notification_type).await {
            return Err(Status::permission_denied("User has disabled this notification type"));
        }

        // Check quiet hours for non-urgent notifications
        if priority != NotificationPriority::Urgent && self.is_quiet_hours(&req.user_id).await {
            return Err(Status::failed_precondition("Notification blocked due to quiet hours"));
        }

        // Create notification
        let mut notification = Notification::new(
            req.user_id,
            notification_type,
            priority,
            req.title,
            req.message,
        ).with_metadata(req.metadata)
         .with_channels(if channels.is_empty() { vec![DeliveryChannel::WebSocket, DeliveryChannel::InApp] } else { channels });

        if req.expires_at > 0 {
            notification = notification.with_expiration(
                DateTime::from_timestamp(req.expires_at, 0)
                    .ok_or_else(|| Status::invalid_argument("Invalid expiration time"))?
            );
        }

        if !req.action_url.is_empty() {
            notification = notification.with_action_url(req.action_url);
        }

        // Store notification
        self.repository.create_notification(&notification).await
            .map_err(|e| Status::internal(format!("Failed to create notification: {}", e)))?;

        // Attempt delivery
        let mut delivered = false;
        let mut failed_channels = Vec::new();

        // WebSocket delivery
        if notification.channels.contains(&DeliveryChannel::WebSocket) {
            let ws_success = self.send_websocket_notification(&notification).await;
            self.record_delivery(&notification.id, DeliveryChannel::WebSocket, ws_success).await;
            if ws_success {
                delivered = true;
            } else {
                failed_channels.push("websocket".to_string());
            }
        }

        // In-app delivery (always succeeds as it's stored in repository)
        if notification.channels.contains(&DeliveryChannel::InApp) {
            self.record_delivery(&notification.id, DeliveryChannel::InApp, true).await;
            delivered = true;
        }

        // Audit log
        self.audit_logger.log_action(
            &auth_context.user_id,
            "notifications.send_notification",
            &format!("target_user={}, type={:?}, delivered={}", notification.user_id, notification.notification_type, delivered),
            true,
            None,
        ).await;

        Ok(Response::new(SendNotificationResponse {
            notification: Some(self.notification_to_proto(&notification)),
            delivered,
            failed_channels,
        }))
    }

    async fn get_notifications(
        &self,
        request: Request<GetNotificationsRequest>,
    ) -> Result<Response<GetNotificationsResponse>, Status> {
        let auth_context = request.extensions().get::<AuthContext>()
            .ok_or_else(|| Status::unauthenticated("Authentication required"))?;

        let req = request.into_inner();

        // Users can only get their own notifications
        if auth_context.user_id != req.user_id {
            return Err(Status::permission_denied("Cannot access other users' notifications"));
        }

        // Convert type filter
        let type_filter: Option<Vec<NotificationType>> = if req.type_filter.is_empty() {
            None
        } else {
            Some(req.type_filter.iter().filter_map(|&t| match t {
                1 => Some(NotificationType::FiatTransaction),
                2 => Some(NotificationType::KycStatus),
                3 => Some(NotificationType::Security),
                4 => Some(NotificationType::PriceAlert),
                5 => Some(NotificationType::System),
                6 => Some(NotificationType::Card),
                7 => Some(NotificationType::Reward),
                _ => None,
            }).collect())
        };

        let since = if req.since_timestamp > 0 {
            DateTime::from_timestamp(req.since_timestamp, 0)
        } else {
            None
        };

        // Get notifications
        let notifications = self.repository.get_user_notifications(
            &req.user_id,
            type_filter.as_deref(),
            req.unread_only,
            if req.page_size > 0 { Some(req.page_size as u32) } else { None },
            None, // offset - would be calculated from page_token in production
            since,
        ).await;

        let proto_notifications: Vec<_> = notifications.iter()
            .map(|n| self.notification_to_proto(n))
            .collect();

        let unread_count = if req.unread_only {
            proto_notifications.len() as i32
        } else {
            self.repository.get_user_notifications(
                &req.user_id,
                type_filter.as_deref(),
                true, // unread_only
                None,
                None,
                since,
            ).await.len() as i32
        };

        // Audit log
        self.audit_logger.log_action(
            &auth_context.user_id,
            "notifications.get_notifications",
            &format!("count={}, unread_only={}", proto_notifications.len(), req.unread_only),
            true,
            None,
        ).await;

        Ok(Response::new(GetNotificationsResponse {
            notifications: proto_notifications,
            next_page_token: String::new(), // Simplified pagination
            total_count: notifications.len() as i32,
            unread_count,
        }))
    }

    async fn mark_as_read(
        &self,
        request: Request<MarkAsReadRequest>,
    ) -> Result<Response<MarkAsReadResponse>, Status> {
        let auth_context = request.extensions().get::<AuthContext>()
            .ok_or_else(|| Status::unauthenticated("Authentication required"))?;

        let req = request.into_inner();

        // Users can only mark their own notifications as read
        if auth_context.user_id != req.user_id {
            return Err(Status::permission_denied("Cannot mark other users' notifications as read"));
        }

        let marked_count = self.repository.mark_as_read(&req.user_id, &req.notification_ids).await
            .map_err(|e| Status::internal(format!("Failed to mark notifications as read: {}", e)))?;

        // Audit log
        self.audit_logger.log_action(
            &auth_context.user_id,
            "notifications.mark_as_read",
            &format!("marked_count={}", marked_count),
            true,
            None,
        ).await;

        Ok(Response::new(MarkAsReadResponse {
            marked_count: marked_count as i32,
        }))
    }

    async fn delete_notification(
        &self,
        request: Request<DeleteNotificationRequest>,
    ) -> Result<Response<DeleteNotificationResponse>, Status> {
        let auth_context = request.extensions().get::<AuthContext>()
            .ok_or_else(|| Status::unauthenticated("Authentication required"))?;

        let req = request.into_inner();

        // Users can only delete their own notifications
        if auth_context.user_id != req.user_id {
            return Err(Status::permission_denied("Cannot delete other users' notifications"));
        }

        let success = self.repository.delete_notification(&req.user_id, &req.notification_id).await
            .map_err(|e| Status::internal(format!("Failed to delete notification: {}", e)))?;

        // Audit log
        self.audit_logger.log_action(
            &auth_context.user_id,
            "notifications.delete_notification",
            &format!("notification_id={}, success={}", req.notification_id, success),
            true,
            None,
        ).await;

        Ok(Response::new(DeleteNotificationResponse { success }))
    }

    async fn get_notification_preferences(
        &self,
        request: Request<GetNotificationPreferencesRequest>,
    ) -> Result<Response<GetNotificationPreferencesResponse>, Status> {
        let auth_context = request.extensions().get::<AuthContext>()
            .ok_or_else(|| Status::unauthenticated("Authentication required"))?;

        let req = request.into_inner();

        // Users can only get their own preferences
        if auth_context.user_id != req.user_id {
            return Err(Status::permission_denied("Cannot access other users' preferences"));
        }

        let preferences = self.repository.get_user_preferences(&req.user_id).await
            .unwrap_or_else(|| {
                let mut default_prefs = NotificationPreferences::default();
                default_prefs.user_id = req.user_id.clone();
                default_prefs
            });

        // Audit log
        self.audit_logger.log_action(
            &auth_context.user_id,
            "notifications.get_notification_preferences",
            "preferences_retrieved",
            true,
            None,
        ).await;

        Ok(Response::new(GetNotificationPreferencesResponse {
            preferences: Some(self.preferences_to_proto(&preferences)),
        }))
    }

    async fn update_notification_preferences(
        &self,
        request: Request<UpdateNotificationPreferencesRequest>,
    ) -> Result<Response<UpdateNotificationPreferencesResponse>, Status> {
        let auth_context = request.extensions().get::<AuthContext>()
            .ok_or_else(|| Status::unauthenticated("Authentication required"))?;

        let req = request.into_inner();

        // Users can only update their own preferences
        if auth_context.user_id != req.user_id {
            return Err(Status::permission_denied("Cannot update other users' preferences"));
        }

        let proto_prefs = req.preferences
            .ok_or_else(|| Status::invalid_argument("Preferences are required"))?;

        // Convert proto preferences to internal format
        let preferred_channels: Vec<DeliveryChannel> = proto_prefs.preferred_channels.iter().filter_map(|&c| match c {
            1 => Some(DeliveryChannel::WebSocket),
            2 => Some(DeliveryChannel::InApp),
            3 => Some(DeliveryChannel::Email),
            4 => Some(DeliveryChannel::Sms),
            5 => Some(DeliveryChannel::Push),
            _ => None,
        }).collect();

        let preferences = NotificationPreferences {
            user_id: req.user_id,
            fiat_transaction_enabled: proto_prefs.fiat_transaction_enabled,
            kyc_status_enabled: proto_prefs.kyc_status_enabled,
            security_alerts_enabled: proto_prefs.security_alerts_enabled,
            price_alerts_enabled: proto_prefs.price_alerts_enabled,
            system_announcements_enabled: proto_prefs.system_announcements_enabled,
            card_notifications_enabled: proto_prefs.card_notifications_enabled,
            reward_notifications_enabled: proto_prefs.reward_notifications_enabled,
            preferred_channels,
            quiet_hours_enabled: proto_prefs.quiet_hours_enabled,
            quiet_hours_start: proto_prefs.quiet_hours_start as u8,
            quiet_hours_end: proto_prefs.quiet_hours_end as u8,
            timezone: proto_prefs.timezone,
            updated_at: Utc::now(),
        };

        self.repository.update_user_preferences(&preferences).await
            .map_err(|e| Status::internal(format!("Failed to update preferences: {}", e)))?;

        // Audit log
        self.audit_logger.log_action(
            &auth_context.user_id,
            "notifications.update_notification_preferences",
            "preferences_updated",
            true,
            None,
        ).await;

        Ok(Response::new(UpdateNotificationPreferencesResponse {
            preferences: Some(self.preferences_to_proto(&preferences)),
        }))
    }

    async fn create_price_alert(
        &self,
        request: Request<CreatePriceAlertRequest>,
    ) -> Result<Response<CreatePriceAlertResponse>, Status> {
        let auth_context = request.extensions().get::<AuthContext>()
            .ok_or_else(|| Status::unauthenticated("Authentication required"))?;

        let req = request.into_inner();

        // Users can only create alerts for themselves
        if auth_context.user_id != req.user_id {
            return Err(Status::permission_denied("Cannot create price alerts for other users"));
        }

        // Validate threshold value
        let threshold_value = Decimal::from_str_exact(&req.threshold_value)
            .map_err(|_| Status::invalid_argument("Invalid threshold value"))?;

        if threshold_value <= Decimal::ZERO {
            return Err(Status::invalid_argument("Threshold value must be positive"));
        }

        // Convert condition
        let condition = match req.condition {
            1 => PriceAlertCondition::Above,
            2 => PriceAlertCondition::Below,
            3 => PriceAlertCondition::ChangePercent,
            _ => return Err(Status::invalid_argument("Invalid price alert condition")),
        };

        let mut alert = PriceAlert::new(
            req.user_id,
            req.symbol,
            req.quote_currency,
            condition,
            threshold_value,
        );

        alert.is_repeating = req.is_repeating;
        alert.max_triggers = req.max_triggers;

        if req.expires_at > 0 {
            alert.expires_at = DateTime::from_timestamp(req.expires_at, 0);
        }

        if !req.note.is_empty() {
            alert.note = Some(req.note);
        }

        self.repository.create_price_alert(&alert).await
            .map_err(|e| Status::internal(format!("Failed to create price alert: {}", e)))?;

        // Audit log
        self.audit_logger.log_action(
            &auth_context.user_id,
            "notifications.create_price_alert",
            &format!("symbol={}, condition={:?}, threshold={}", alert.symbol, alert.condition, alert.threshold_value),
            true,
            None,
        ).await;

        Ok(Response::new(CreatePriceAlertResponse {
            alert: Some(self.price_alert_to_proto(&alert)),
        }))
    }

    async fn list_price_alerts(
        &self,
        request: Request<ListPriceAlertsRequest>,
    ) -> Result<Response<ListPriceAlertsResponse>, Status> {
        let auth_context = request.extensions().get::<AuthContext>()
            .ok_or_else(|| Status::unauthenticated("Authentication required"))?;

        let req = request.into_inner();

        // Users can only list their own alerts
        if auth_context.user_id != req.user_id {
            return Err(Status::permission_denied("Cannot access other users' price alerts"));
        }

        let alerts = self.repository.get_user_price_alerts(&req.user_id, req.active_only).await;
        let proto_alerts: Vec<_> = alerts.iter().map(|a| self.price_alert_to_proto(a)).collect();

        // Audit log
        self.audit_logger.log_action(
            &auth_context.user_id,
            "notifications.list_price_alerts",
            &format!("count={}, active_only={}", proto_alerts.len(), req.active_only),
            true,
            None,
        ).await;

        Ok(Response::new(ListPriceAlertsResponse {
            alerts: proto_alerts,
            next_page_token: String::new(), // Simplified pagination
            total_count: alerts.len() as i32,
        }))
    }

    async fn update_price_alert(
        &self,
        request: Request<UpdatePriceAlertRequest>,
    ) -> Result<Response<UpdatePriceAlertResponse>, Status> {
        let auth_context = request.extensions().get::<AuthContext>()
            .ok_or_else(|| Status::unauthenticated("Authentication required"))?;

        let req = request.into_inner();

        // Users can only update their own alerts
        if auth_context.user_id != req.user_id {
            return Err(Status::permission_denied("Cannot update other users' price alerts"));
        }

        let proto_alert = req.alert
            .ok_or_else(|| Status::invalid_argument("Alert data is required"))?;

        // Validate threshold value
        let threshold_value = Decimal::from_str_exact(&proto_alert.threshold_value)
            .map_err(|_| Status::invalid_argument("Invalid threshold value"))?;

        // Convert condition
        let condition = match proto_alert.condition {
            1 => PriceAlertCondition::Above,
            2 => PriceAlertCondition::Below,
            3 => PriceAlertCondition::ChangePercent,
            _ => return Err(Status::invalid_argument("Invalid price alert condition")),
        };

        let alert = PriceAlert {
            id: req.alert_id,
            user_id: req.user_id,
            symbol: proto_alert.symbol,
            quote_currency: proto_alert.quote_currency,
            condition,
            threshold_value,
            is_active: proto_alert.is_active,
            is_repeating: proto_alert.is_repeating,
            trigger_count: proto_alert.trigger_count,
            max_triggers: proto_alert.max_triggers,
            created_at: DateTime::from_timestamp(proto_alert.created_at, 0).unwrap_or_else(Utc::now),
            last_triggered_at: if proto_alert.last_triggered_at > 0 {
                DateTime::from_timestamp(proto_alert.last_triggered_at, 0)
            } else {
                None
            },
            expires_at: if proto_alert.expires_at > 0 {
                DateTime::from_timestamp(proto_alert.expires_at, 0)
            } else {
                None
            },
            note: if proto_alert.note.is_empty() { None } else { Some(proto_alert.note) },
        };

        self.repository.update_price_alert(&alert).await
            .map_err(|e| Status::internal(format!("Failed to update price alert: {}", e)))?;

        // Audit log
        self.audit_logger.log_action(
            &auth_context.user_id,
            "notifications.update_price_alert",
            &format!("alert_id={}, active={}", alert.id, alert.is_active),
            true,
            None,
        ).await;

        Ok(Response::new(UpdatePriceAlertResponse {
            alert: Some(self.price_alert_to_proto(&alert)),
        }))
    }

    async fn delete_price_alert(
        &self,
        request: Request<DeletePriceAlertRequest>,
    ) -> Result<Response<DeletePriceAlertResponse>, Status> {
        let auth_context = request.extensions().get::<AuthContext>()
            .ok_or_else(|| Status::unauthenticated("Authentication required"))?;

        let req = request.into_inner();

        // Users can only delete their own alerts
        if auth_context.user_id != req.user_id {
            return Err(Status::permission_denied("Cannot delete other users' price alerts"));
        }

        let success = self.repository.delete_price_alert(&req.user_id, &req.alert_id).await
            .map_err(|e| Status::internal(format!("Failed to delete price alert: {}", e)))?;

        // Audit log
        self.audit_logger.log_action(
            &auth_context.user_id,
            "notifications.delete_price_alert",
            &format!("alert_id={}, success={}", req.alert_id, success),
            true,
            None,
        ).await;

        Ok(Response::new(DeletePriceAlertResponse { success }))
    }

    async fn subscribe_to_notifications(
        &self,
        request: Request<SubscribeToNotificationsRequest>,
    ) -> Result<Response<SubscribeToNotificationsResponse>, Status> {
        let auth_context = request.extensions().get::<AuthContext>()
            .ok_or_else(|| Status::unauthenticated("Authentication required"))?;

        let req = request.into_inner();

        // Users can only subscribe for themselves
        if auth_context.user_id != req.user_id {
            return Err(Status::permission_denied("Cannot subscribe for other users"));
        }

        // In a real implementation, this would register the WebSocket connection
        // For now, we'll just return a subscription ID
        let subscription_id = Uuid::new_v4().to_string();

        // Audit log
        self.audit_logger.log_action(
            &auth_context.user_id,
            "notifications.subscribe_to_notifications",
            &format!("subscription_id={}", subscription_id),
            true,
            None,
        ).await;

        Ok(Response::new(SubscribeToNotificationsResponse {
            success: true,
            subscription_id,
        }))
    }

    async fn unsubscribe_from_notifications(
        &self,
        request: Request<UnsubscribeFromNotificationsRequest>,
    ) -> Result<Response<UnsubscribeFromNotificationsResponse>, Status> {
        let auth_context = request.extensions().get::<AuthContext>()
            .ok_or_else(|| Status::unauthenticated("Authentication required"))?;

        let req = request.into_inner();

        // Users can only unsubscribe for themselves
        if auth_context.user_id != req.user_id {
            return Err(Status::permission_denied("Cannot unsubscribe for other users"));
        }

        // In a real implementation, this would close the WebSocket connection
        // For now, we'll just return success

        // Audit log
        self.audit_logger.log_action(
            &auth_context.user_id,
            "notifications.unsubscribe_from_notifications",
            &format!("subscription_id={}", req.subscription_id),
            true,
            None,
        ).await;

        Ok(Response::new(UnsubscribeFromNotificationsResponse {
            success: true,
        }))
    }

    async fn broadcast_notification(
        &self,
        request: Request<BroadcastNotificationRequest>,
    ) -> Result<Response<BroadcastNotificationResponse>, Status> {
        let auth_context = request.extensions().get::<AuthContext>()
            .ok_or_else(|| Status::unauthenticated("Authentication required"))?;

        // Check admin permissions
        self.auth_service.check_permission(auth_context, crate::middleware::auth::Permission::PermissionNotificationAdmin)?;

        let req = request.into_inner();

        // Convert proto types
        let notification_type = match req.r#type {
            1 => NotificationType::FiatTransaction,
            2 => NotificationType::KycStatus,
            3 => NotificationType::Security,
            4 => NotificationType::PriceAlert,
            5 => NotificationType::System,
            6 => NotificationType::Card,
            7 => NotificationType::Reward,
            _ => return Err(Status::invalid_argument("Invalid notification type")),
        };

        let priority = match req.priority {
            1 => NotificationPriority::Low,
            2 => NotificationPriority::Normal,
            3 => NotificationPriority::High,
            4 => NotificationPriority::Urgent,
            _ => NotificationPriority::Normal,
        };

        let channels: Vec<DeliveryChannel> = req.channels.iter().filter_map(|&c| match c {
            1 => Some(DeliveryChannel::WebSocket),
            2 => Some(DeliveryChannel::InApp),
            3 => Some(DeliveryChannel::Email),
            4 => Some(DeliveryChannel::Sms),
            5 => Some(DeliveryChannel::Push),
            _ => None,
        }).collect();

        // Determine target users
        let target_users = if req.target_user_ids.is_empty() {
            // Broadcast to all users - in a real implementation, we'd get this from a user repository
            vec!["all_users".to_string()] // Placeholder
        } else {
            req.target_user_ids
        };

        let mut notifications_sent = 0;
        let mut successful_deliveries = 0;
        let mut failed_deliveries = 0;

        // Send notification to each target user
        for user_id in &target_users {
            if user_id == "all_users" {
                // Skip placeholder - in real implementation, iterate through all users
                continue;
            }

            // Check if user should receive this type of notification
            if !self.should_send_notification(user_id, &notification_type).await {
                continue;
            }

            // Create notification
            let mut notification = Notification::new(
                user_id.clone(),
                notification_type.clone(),
                priority.clone(),
                req.title.clone(),
                req.message.clone(),
            ).with_metadata(req.metadata.clone())
             .with_channels(if channels.is_empty() { vec![DeliveryChannel::WebSocket, DeliveryChannel::InApp] } else { channels.clone() });

            if req.expires_at > 0 {
                if let Some(expires_at) = DateTime::from_timestamp(req.expires_at, 0) {
                    notification = notification.with_expiration(expires_at);
                }
            }

            // Store notification
            if let Ok(()) = self.repository.create_notification(&notification).await {
                notifications_sent += 1;

                // Attempt delivery
                if notification.channels.contains(&DeliveryChannel::WebSocket) {
                    if self.send_websocket_notification(&notification).await {
                        successful_deliveries += 1;
                    } else {
                        failed_deliveries += 1;
                    }
                }

                if notification.channels.contains(&DeliveryChannel::InApp) {
                    successful_deliveries += 1;
                }
            } else {
                failed_deliveries += 1;
            }
        }

        // Audit log
        self.audit_logger.log_action(
            &auth_context.user_id,
            "notifications.broadcast_notification",
            &format!("type={:?}, sent={}, successful={}, failed={}", notification_type, notifications_sent, successful_deliveries, failed_deliveries),
            true,
            None,
        ).await;

        Ok(Response::new(BroadcastNotificationResponse {
            notifications_sent,
            successful_deliveries,
            failed_deliveries,
        }))
    }

    async fn get_notification_metrics(
        &self,
        request: Request<GetNotificationMetricsRequest>,
    ) -> Result<Response<GetNotificationMetricsResponse>, Status> {
        let auth_context = request.extensions().get::<AuthContext>()
            .ok_or_else(|| Status::unauthenticated("Authentication required"))?;

        // Check admin permissions
        self.auth_service.check_permission(auth_context, crate::middleware::auth::Permission::PermissionNotificationAdmin)?;

        let req = request.into_inner();

        let start_time = if req.start_time > 0 {
            DateTime::from_timestamp(req.start_time, 0)
        } else {
            None
        };

        let end_time = if req.end_time > 0 {
            DateTime::from_timestamp(req.end_time, 0)
        } else {
            None
        };

        let metrics = self.repository.get_metrics(start_time, end_time).await;

        // Audit log
        self.audit_logger.log_action(
            &auth_context.user_id,
            "notifications.get_notification_metrics",
            "metrics_retrieved",
            true,
            None,
        ).await;

        Ok(Response::new(GetNotificationMetricsResponse {
            metrics: Some(self.metrics_to_proto(&metrics)),
        }))
    }
}
