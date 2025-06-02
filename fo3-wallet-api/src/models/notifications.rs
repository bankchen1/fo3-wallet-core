//! Notification data models and entities

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use rust_decimal::Decimal;
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Notification types supported by the system
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NotificationType {
    FiatTransaction,
    KycStatus,
    Security,
    PriceAlert,
    System,
    Card,
    Reward,
}

/// Notification priority levels
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NotificationPriority {
    Low,
    Normal,
    High,
    Urgent,
}

/// Delivery channels for notifications
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeliveryChannel {
    WebSocket,
    InApp,
    Email,
    Sms,
    Push,
}

/// Price alert condition types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PriceAlertCondition {
    Above,
    Below,
    ChangePercent,
}

/// Core notification entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub id: String,
    pub user_id: String,
    pub notification_type: NotificationType,
    pub priority: NotificationPriority,
    pub title: String,
    pub message: String,
    pub metadata: HashMap<String, String>,
    pub channels: Vec<DeliveryChannel>,
    pub is_read: bool,
    pub created_at: DateTime<Utc>,
    pub read_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub action_url: Option<String>,
    pub icon_url: Option<String>,
}

/// User notification preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationPreferences {
    pub user_id: String,
    pub fiat_transaction_enabled: bool,
    pub kyc_status_enabled: bool,
    pub security_alerts_enabled: bool,
    pub price_alerts_enabled: bool,
    pub system_announcements_enabled: bool,
    pub card_notifications_enabled: bool,
    pub reward_notifications_enabled: bool,
    pub preferred_channels: Vec<DeliveryChannel>,
    pub quiet_hours_enabled: bool,
    pub quiet_hours_start: u8, // Hour of day (0-23)
    pub quiet_hours_end: u8,   // Hour of day (0-23)
    pub timezone: String,
    pub updated_at: DateTime<Utc>,
}

/// Price alert configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceAlert {
    pub id: String,
    pub user_id: String,
    pub symbol: String,
    pub quote_currency: String,
    pub condition: PriceAlertCondition,
    pub threshold_value: Decimal,
    pub is_active: bool,
    pub is_repeating: bool,
    pub trigger_count: u32,
    pub max_triggers: u32, // 0 = unlimited
    pub created_at: DateTime<Utc>,
    pub last_triggered_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub note: Option<String>,
}

/// Notification metrics for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationMetrics {
    pub total_notifications_sent: u64,
    pub websocket_deliveries: u64,
    pub in_app_deliveries: u64,
    pub failed_deliveries: u64,
    pub delivery_success_rate: f64,
    pub active_websocket_connections: u64,
    pub active_price_alerts: u64,
    pub notifications_last_24h: u64,
    pub notifications_by_type: HashMap<String, u64>,
    pub notifications_by_priority: HashMap<String, u64>,
    pub average_delivery_time_ms: f64,
}

/// WebSocket subscription information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationSubscription {
    pub id: String,
    pub user_id: String,
    pub notification_types: Vec<NotificationType>,
    pub created_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub is_active: bool,
}

/// Notification delivery status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationDelivery {
    pub notification_id: String,
    pub channel: DeliveryChannel,
    pub status: DeliveryStatus,
    pub attempted_at: DateTime<Utc>,
    pub delivered_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub retry_count: u32,
}

/// Delivery status enumeration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeliveryStatus {
    Pending,
    Delivered,
    Failed,
    Retrying,
}

/// Event data for different notification types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationEventData {
    FiatTransaction {
        transaction_id: String,
        transaction_type: String, // "deposit" or "withdrawal"
        status: String,
        amount: Decimal,
        currency: String,
    },
    KycStatus {
        submission_id: String,
        status: String,
        rejection_reason: Option<String>,
        required_documents: Vec<String>,
    },
    Security {
        event_type: String, // "login", "password_change", "api_key_created", etc.
        ip_address: Option<String>,
        device_info: Option<String>,
        location: Option<String>,
    },
    PriceAlert {
        symbol: String,
        current_price: Decimal,
        threshold_price: Decimal,
        condition: PriceAlertCondition,
        change_percent: Option<Decimal>,
    },
    System {
        announcement_type: String, // "maintenance", "feature", "security", etc.
        severity: String,
        affected_services: Vec<String>,
    },
    Card {
        card_id: String,
        event_type: String, // "issued", "activated", "blocked", "transaction", etc.
        amount: Option<Decimal>,
        merchant: Option<String>,
    },
    Reward {
        reward_type: String, // "cashback", "referral", "deposit_bonus", etc.
        amount: Decimal,
        currency: String,
        source_transaction_id: Option<String>,
    },
}

/// Notification repository trait for data access
#[async_trait::async_trait]
pub trait NotificationRepository: Send + Sync {
    /// Store a new notification
    async fn create_notification(&self, notification: &Notification) -> Result<(), String>;
    
    /// Get notifications for a user
    async fn get_user_notifications(
        &self,
        user_id: &str,
        type_filter: Option<&[NotificationType]>,
        unread_only: bool,
        limit: Option<u32>,
        offset: Option<u32>,
        since: Option<DateTime<Utc>>,
    ) -> Vec<Notification>;
    
    /// Mark notifications as read
    async fn mark_as_read(&self, user_id: &str, notification_ids: &[String]) -> Result<u32, String>;
    
    /// Delete a notification
    async fn delete_notification(&self, user_id: &str, notification_id: &str) -> Result<bool, String>;
    
    /// Get user notification preferences
    async fn get_user_preferences(&self, user_id: &str) -> Option<NotificationPreferences>;
    
    /// Update user notification preferences
    async fn update_user_preferences(&self, preferences: &NotificationPreferences) -> Result<(), String>;
    
    /// Create a price alert
    async fn create_price_alert(&self, alert: &PriceAlert) -> Result<(), String>;
    
    /// Get user price alerts
    async fn get_user_price_alerts(&self, user_id: &str, active_only: bool) -> Vec<PriceAlert>;
    
    /// Update a price alert
    async fn update_price_alert(&self, alert: &PriceAlert) -> Result<(), String>;
    
    /// Delete a price alert
    async fn delete_price_alert(&self, user_id: &str, alert_id: &str) -> Result<bool, String>;
    
    /// Get all active price alerts for monitoring
    async fn get_active_price_alerts(&self) -> Vec<PriceAlert>;
    
    /// Record notification delivery
    async fn record_delivery(&self, delivery: &NotificationDelivery) -> Result<(), String>;
    
    /// Get notification metrics
    async fn get_metrics(&self, start_time: Option<DateTime<Utc>>, end_time: Option<DateTime<Utc>>) -> NotificationMetrics;
    
    /// Clean up expired notifications
    async fn cleanup_expired_notifications(&self) -> Result<u32, String>;
}

impl Default for NotificationPreferences {
    fn default() -> Self {
        Self {
            user_id: String::new(),
            fiat_transaction_enabled: true,
            kyc_status_enabled: true,
            security_alerts_enabled: true,
            price_alerts_enabled: true,
            system_announcements_enabled: true,
            card_notifications_enabled: true,
            reward_notifications_enabled: true,
            preferred_channels: vec![DeliveryChannel::WebSocket, DeliveryChannel::InApp],
            quiet_hours_enabled: false,
            quiet_hours_start: 22, // 10 PM
            quiet_hours_end: 8,    // 8 AM
            timezone: "UTC".to_string(),
            updated_at: Utc::now(),
        }
    }
}

impl Notification {
    pub fn new(
        user_id: String,
        notification_type: NotificationType,
        priority: NotificationPriority,
        title: String,
        message: String,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            user_id,
            notification_type,
            priority,
            title,
            message,
            metadata: HashMap::new(),
            channels: vec![DeliveryChannel::WebSocket, DeliveryChannel::InApp],
            is_read: false,
            created_at: Utc::now(),
            read_at: None,
            expires_at: None,
            action_url: None,
            icon_url: None,
        }
    }

    pub fn with_metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = metadata;
        self
    }

    pub fn with_channels(mut self, channels: Vec<DeliveryChannel>) -> Self {
        self.channels = channels;
        self
    }

    pub fn with_expiration(mut self, expires_at: DateTime<Utc>) -> Self {
        self.expires_at = Some(expires_at);
        self
    }

    pub fn with_action_url(mut self, action_url: String) -> Self {
        self.action_url = Some(action_url);
        self
    }

    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            Utc::now() > expires_at
        } else {
            false
        }
    }
}

impl PriceAlert {
    pub fn new(
        user_id: String,
        symbol: String,
        quote_currency: String,
        condition: PriceAlertCondition,
        threshold_value: Decimal,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            user_id,
            symbol,
            quote_currency,
            condition,
            threshold_value,
            is_active: true,
            is_repeating: false,
            trigger_count: 0,
            max_triggers: 1,
            created_at: Utc::now(),
            last_triggered_at: None,
            expires_at: None,
            note: None,
        }
    }

    pub fn should_trigger(&self, current_price: Decimal) -> bool {
        if !self.is_active {
            return false;
        }

        if let Some(expires_at) = self.expires_at {
            if Utc::now() > expires_at {
                return false;
            }
        }

        if self.max_triggers > 0 && self.trigger_count >= self.max_triggers {
            return false;
        }

        match self.condition {
            PriceAlertCondition::Above => current_price > self.threshold_value,
            PriceAlertCondition::Below => current_price < self.threshold_value,
            PriceAlertCondition::ChangePercent => {
                // This would require historical price data to calculate percentage change
                // For now, we'll return false and implement this later
                false
            }
        }
    }

    pub fn trigger(&mut self) {
        self.trigger_count += 1;
        self.last_triggered_at = Some(Utc::now());
        
        if !self.is_repeating && self.trigger_count >= self.max_triggers {
            self.is_active = false;
        }
    }
}

/// In-memory notification repository implementation
pub struct InMemoryNotificationRepository {
    notifications: std::sync::RwLock<HashMap<String, Notification>>,
    user_notifications: std::sync::RwLock<HashMap<String, Vec<String>>>, // user_id -> notification_ids
    preferences: std::sync::RwLock<HashMap<String, NotificationPreferences>>,
    price_alerts: std::sync::RwLock<HashMap<String, PriceAlert>>,
    user_price_alerts: std::sync::RwLock<HashMap<String, Vec<String>>>, // user_id -> alert_ids
    deliveries: std::sync::RwLock<HashMap<String, Vec<NotificationDelivery>>>, // notification_id -> deliveries
    metrics: std::sync::RwLock<NotificationMetrics>,
}

impl InMemoryNotificationRepository {
    pub fn new() -> Self {
        let metrics = NotificationMetrics {
            total_notifications_sent: 0,
            websocket_deliveries: 0,
            in_app_deliveries: 0,
            failed_deliveries: 0,
            delivery_success_rate: 0.0,
            active_websocket_connections: 0,
            active_price_alerts: 0,
            notifications_last_24h: 0,
            notifications_by_type: HashMap::new(),
            notifications_by_priority: HashMap::new(),
            average_delivery_time_ms: 0.0,
        };

        Self {
            notifications: std::sync::RwLock::new(HashMap::new()),
            user_notifications: std::sync::RwLock::new(HashMap::new()),
            preferences: std::sync::RwLock::new(HashMap::new()),
            price_alerts: std::sync::RwLock::new(HashMap::new()),
            user_price_alerts: std::sync::RwLock::new(HashMap::new()),
            deliveries: std::sync::RwLock::new(HashMap::new()),
            metrics: std::sync::RwLock::new(metrics),
        }
    }
}

impl Default for InMemoryNotificationRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl NotificationRepository for InMemoryNotificationRepository {
    async fn create_notification(&self, notification: &Notification) -> Result<(), String> {
        let mut notifications = self.notifications.write().unwrap();
        let mut user_notifications = self.user_notifications.write().unwrap();
        let mut metrics = self.metrics.write().unwrap();

        notifications.insert(notification.id.clone(), notification.clone());

        let user_notif_list = user_notifications.entry(notification.user_id.clone()).or_insert_with(Vec::new);
        user_notif_list.push(notification.id.clone());

        // Update metrics
        metrics.total_notifications_sent += 1;
        let type_key = format!("{:?}", notification.notification_type);
        let type_count = metrics.notifications_by_type.get(&type_key).unwrap_or(&0) + 1;
        metrics.notifications_by_type.insert(type_key, type_count);

        let priority_key = format!("{:?}", notification.priority);
        let priority_count = metrics.notifications_by_priority.get(&priority_key).unwrap_or(&0) + 1;
        metrics.notifications_by_priority.insert(priority_key, priority_count);

        // Check if notification is within last 24 hours
        let twenty_four_hours_ago = Utc::now() - chrono::Duration::hours(24);
        if notification.created_at > twenty_four_hours_ago {
            metrics.notifications_last_24h += 1;
        }

        Ok(())
    }

    async fn get_user_notifications(
        &self,
        user_id: &str,
        type_filter: Option<&[NotificationType]>,
        unread_only: bool,
        limit: Option<u32>,
        offset: Option<u32>,
        since: Option<DateTime<Utc>>,
    ) -> Vec<Notification> {
        let notifications = self.notifications.read().unwrap();
        let user_notifications = self.user_notifications.read().unwrap();

        if let Some(notification_ids) = user_notifications.get(user_id) {
            let mut user_notifs: Vec<_> = notification_ids.iter()
                .filter_map(|id| notifications.get(id))
                .filter(|notif| {
                    // Apply filters
                    if unread_only && notif.is_read {
                        return false;
                    }

                    if let Some(types) = type_filter {
                        if !types.contains(&notif.notification_type) {
                            return false;
                        }
                    }

                    if let Some(since_time) = since {
                        if notif.created_at <= since_time {
                            return false;
                        }
                    }

                    // Check if notification is expired
                    if notif.is_expired() {
                        return false;
                    }

                    true
                })
                .cloned()
                .collect();

            // Sort by creation time (newest first)
            user_notifs.sort_by(|a, b| b.created_at.cmp(&a.created_at));

            // Apply pagination
            let start = offset.unwrap_or(0) as usize;
            let end = if let Some(limit) = limit {
                std::cmp::min(start + limit as usize, user_notifs.len())
            } else {
                user_notifs.len()
            };

            if start < user_notifs.len() {
                user_notifs[start..end].to_vec()
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        }
    }

    async fn mark_as_read(&self, user_id: &str, notification_ids: &[String]) -> Result<u32, String> {
        let mut notifications = self.notifications.write().unwrap();
        let user_notifications = self.user_notifications.read().unwrap();
        let mut marked_count = 0;

        if let Some(user_notif_ids) = user_notifications.get(user_id) {
            let ids_to_mark: Vec<_> = if notification_ids.is_empty() {
                // Mark all user notifications as read
                user_notif_ids.clone()
            } else {
                // Mark only specified notifications that belong to the user
                notification_ids.iter()
                    .filter(|id| user_notif_ids.contains(id))
                    .cloned()
                    .collect()
            };

            for id in ids_to_mark {
                if let Some(notification) = notifications.get_mut(&id) {
                    if !notification.is_read {
                        notification.is_read = true;
                        notification.read_at = Some(Utc::now());
                        marked_count += 1;
                    }
                }
            }
        }

        Ok(marked_count)
    }

    async fn delete_notification(&self, user_id: &str, notification_id: &str) -> Result<bool, String> {
        let mut notifications = self.notifications.write().unwrap();
        let mut user_notifications = self.user_notifications.write().unwrap();

        if let Some(user_notif_list) = user_notifications.get_mut(user_id) {
            if let Some(pos) = user_notif_list.iter().position(|id| id == notification_id) {
                user_notif_list.remove(pos);
                notifications.remove(notification_id);
                return Ok(true);
            }
        }

        Ok(false)
    }

    async fn get_user_preferences(&self, user_id: &str) -> Option<NotificationPreferences> {
        let preferences = self.preferences.read().unwrap();
        preferences.get(user_id).cloned()
    }

    async fn update_user_preferences(&self, preferences: &NotificationPreferences) -> Result<(), String> {
        let mut prefs = self.preferences.write().unwrap();
        prefs.insert(preferences.user_id.clone(), preferences.clone());
        Ok(())
    }

    async fn create_price_alert(&self, alert: &PriceAlert) -> Result<(), String> {
        let mut price_alerts = self.price_alerts.write().unwrap();
        let mut user_price_alerts = self.user_price_alerts.write().unwrap();
        let mut metrics = self.metrics.write().unwrap();

        price_alerts.insert(alert.id.clone(), alert.clone());

        let user_alert_list = user_price_alerts.entry(alert.user_id.clone()).or_insert_with(Vec::new);
        user_alert_list.push(alert.id.clone());

        if alert.is_active {
            metrics.active_price_alerts += 1;
        }

        Ok(())
    }

    async fn get_user_price_alerts(&self, user_id: &str, active_only: bool) -> Vec<PriceAlert> {
        let price_alerts = self.price_alerts.read().unwrap();
        let user_price_alerts = self.user_price_alerts.read().unwrap();

        if let Some(alert_ids) = user_price_alerts.get(user_id) {
            alert_ids.iter()
                .filter_map(|id| price_alerts.get(id))
                .filter(|alert| !active_only || alert.is_active)
                .cloned()
                .collect()
        } else {
            Vec::new()
        }
    }

    async fn update_price_alert(&self, alert: &PriceAlert) -> Result<(), String> {
        let mut price_alerts = self.price_alerts.write().unwrap();
        let mut metrics = self.metrics.write().unwrap();

        if let Some(existing_alert) = price_alerts.get(&alert.id) {
            let was_active = existing_alert.is_active;
            let is_active = alert.is_active;

            price_alerts.insert(alert.id.clone(), alert.clone());

            // Update metrics
            if was_active && !is_active {
                metrics.active_price_alerts = metrics.active_price_alerts.saturating_sub(1);
            } else if !was_active && is_active {
                metrics.active_price_alerts += 1;
            }

            Ok(())
        } else {
            Err("Price alert not found".to_string())
        }
    }

    async fn delete_price_alert(&self, user_id: &str, alert_id: &str) -> Result<bool, String> {
        let mut price_alerts = self.price_alerts.write().unwrap();
        let mut user_price_alerts = self.user_price_alerts.write().unwrap();
        let mut metrics = self.metrics.write().unwrap();

        if let Some(user_alert_list) = user_price_alerts.get_mut(user_id) {
            if let Some(pos) = user_alert_list.iter().position(|id| id == alert_id) {
                user_alert_list.remove(pos);

                if let Some(alert) = price_alerts.remove(alert_id) {
                    if alert.is_active {
                        metrics.active_price_alerts = metrics.active_price_alerts.saturating_sub(1);
                    }
                }

                return Ok(true);
            }
        }

        Ok(false)
    }

    async fn get_active_price_alerts(&self) -> Vec<PriceAlert> {
        let price_alerts = self.price_alerts.read().unwrap();
        price_alerts.values()
            .filter(|alert| alert.is_active)
            .cloned()
            .collect()
    }

    async fn record_delivery(&self, delivery: &NotificationDelivery) -> Result<(), String> {
        let mut deliveries = self.deliveries.write().unwrap();
        let mut metrics = self.metrics.write().unwrap();

        let delivery_list = deliveries.entry(delivery.notification_id.clone()).or_insert_with(Vec::new);
        delivery_list.push(delivery.clone());

        // Update metrics
        match delivery.status {
            DeliveryStatus::Delivered => {
                match delivery.channel {
                    DeliveryChannel::WebSocket => metrics.websocket_deliveries += 1,
                    DeliveryChannel::InApp => metrics.in_app_deliveries += 1,
                    _ => {}
                }
            }
            DeliveryStatus::Failed => {
                metrics.failed_deliveries += 1;
            }
            _ => {}
        }

        // Recalculate delivery success rate
        let total_deliveries = metrics.websocket_deliveries + metrics.in_app_deliveries + metrics.failed_deliveries;
        if total_deliveries > 0 {
            let successful_deliveries = metrics.websocket_deliveries + metrics.in_app_deliveries;
            metrics.delivery_success_rate = successful_deliveries as f64 / total_deliveries as f64;
        }

        Ok(())
    }

    async fn get_metrics(&self, _start_time: Option<DateTime<Utc>>, _end_time: Option<DateTime<Utc>>) -> NotificationMetrics {
        let metrics = self.metrics.read().unwrap();
        metrics.clone()
    }

    async fn cleanup_expired_notifications(&self) -> Result<u32, String> {
        let mut notifications = self.notifications.write().unwrap();
        let mut user_notifications = self.user_notifications.write().unwrap();
        let now = Utc::now();
        let mut cleaned_count = 0;

        // Find expired notifications
        let expired_ids: Vec<_> = notifications.iter()
            .filter(|(_, notif)| {
                if let Some(expires_at) = notif.expires_at {
                    now > expires_at
                } else {
                    false
                }
            })
            .map(|(id, _)| id.clone())
            .collect();

        // Remove expired notifications
        for id in expired_ids {
            if let Some(notification) = notifications.remove(&id) {
                // Remove from user notification lists
                if let Some(user_list) = user_notifications.get_mut(&notification.user_id) {
                    user_list.retain(|notif_id| notif_id != &id);
                }
                cleaned_count += 1;
            }
        }

        Ok(cleaned_count)
    }
}
