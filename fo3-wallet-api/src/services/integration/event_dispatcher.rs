//! Event Dispatcher for Real-time WebSocket Events
//! 
//! Manages event publishing and subscription for real-time notifications

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use tracing::{info, warn, error, instrument};

use crate::error::ServiceError;

/// Service event types for real-time notifications
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event_type", content = "data")]
pub enum ServiceEvent {
    /// Wallet events
    WalletCreated {
        wallet_id: String,
        name: String,
        created_at: DateTime<Utc>,
    },
    WalletUpdated {
        wallet_id: String,
        updated_at: DateTime<Utc>,
    },
    
    /// KYC events
    KycSubmitted {
        submission_id: String,
        wallet_id: String,
        submitted_at: DateTime<Utc>,
    },
    KycStatusChanged {
        submission_id: String,
        wallet_id: String,
        old_status: String,
        new_status: String,
        changed_at: DateTime<Utc>,
    },
    
    /// Card events
    CardCreated {
        card_id: String,
        user_id: String,
        currency: String,
        created_at: DateTime<Utc>,
    },
    CardTransactionProcessed {
        transaction_id: String,
        card_id: String,
        amount: String,
        merchant_name: String,
        status: String,
        processed_at: DateTime<Utc>,
    },
    CardStatusChanged {
        card_id: String,
        old_status: String,
        new_status: String,
        reason: Option<String>,
        changed_at: DateTime<Utc>,
    },
    
    /// Trading events
    TradingStrategyCreated {
        strategy_id: String,
        user_id: String,
        strategy_type: String,
        created_at: DateTime<Utc>,
    },
    TradeExecuted {
        trade_id: String,
        strategy_id: String,
        symbol: String,
        amount: String,
        price: String,
        executed_at: DateTime<Utc>,
    },
    
    /// DeFi events
    DefiPositionOpened {
        position_id: String,
        user_id: String,
        protocol: String,
        amount: String,
        apy: String,
        opened_at: DateTime<Utc>,
    },
    DefiRewardsClaimed {
        position_id: String,
        user_id: String,
        reward_amount: String,
        claimed_at: DateTime<Utc>,
    },
    
    /// System events
    SystemAlert {
        alert_type: String,
        message: String,
        severity: String,
        timestamp: DateTime<Utc>,
    },
    HealthCheckUpdate {
        service_name: String,
        status: String,
        timestamp: DateTime<Utc>,
    },
}

/// Event metadata for tracking and filtering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventMetadata {
    pub event_id: String,
    pub user_id: Option<String>,
    pub session_id: Option<String>,
    pub source_service: String,
    pub timestamp: DateTime<Utc>,
    pub correlation_id: Option<String>,
}

/// Complete event with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventWithMetadata {
    pub metadata: EventMetadata,
    pub event: ServiceEvent,
}

/// Event subscription filter
#[derive(Debug, Clone)]
pub struct EventFilter {
    pub user_id: Option<String>,
    pub event_types: Option<Vec<String>>,
    pub source_services: Option<Vec<String>>,
}

/// Event dispatcher for managing real-time events
pub struct EventDispatcher {
    /// Global event broadcaster
    global_sender: broadcast::Sender<EventWithMetadata>,
    
    /// User-specific event channels
    user_channels: Arc<RwLock<HashMap<String, broadcast::Sender<EventWithMetadata>>>>,
    
    /// Event statistics
    event_stats: Arc<RwLock<EventStats>>,
}

/// Event statistics for monitoring
#[derive(Debug, Default)]
struct EventStats {
    total_events: u64,
    events_by_type: HashMap<String, u64>,
    active_subscriptions: u64,
    last_event_time: Option<DateTime<Utc>>,
}

impl EventDispatcher {
    /// Create a new event dispatcher
    pub fn new() -> Self {
        let (global_sender, _) = broadcast::channel(1000);
        
        Self {
            global_sender,
            user_channels: Arc::new(RwLock::new(HashMap::new())),
            event_stats: Arc::new(RwLock::new(EventStats::default())),
        }
    }

    /// Publish an event to all subscribers
    #[instrument(skip(self))]
    pub async fn publish_event(
        &self,
        event: ServiceEvent,
        user_id: Option<String>,
        source_service: String,
        correlation_id: Option<String>,
    ) -> Result<(), ServiceError> {
        let event_with_metadata = EventWithMetadata {
            metadata: EventMetadata {
                event_id: Uuid::new_v4().to_string(),
                user_id: user_id.clone(),
                session_id: None, // Could be populated from request context
                source_service,
                timestamp: Utc::now(),
                correlation_id,
            },
            event: event.clone(),
        };

        // Update statistics
        self.update_event_stats(&event).await;

        // Publish to global channel
        if let Err(e) = self.global_sender.send(event_with_metadata.clone()) {
            warn!("Failed to publish to global channel: {}", e);
        }

        // Publish to user-specific channel if applicable
        if let Some(user_id) = user_id {
            self.publish_to_user_channel(&user_id, event_with_metadata).await?;
        }

        info!("Event published: {:?}", self.get_event_type_name(&event));
        Ok(())
    }

    /// Subscribe to events with optional filtering
    pub async fn subscribe(
        &self,
        filter: Option<EventFilter>,
    ) -> Result<broadcast::Receiver<EventWithMetadata>, ServiceError> {
        let receiver = self.global_sender.subscribe();
        
        // Update subscription count
        {
            let mut stats = self.event_stats.write().await;
            stats.active_subscriptions += 1;
        }

        info!("New event subscription created with filter: {:?}", filter);
        Ok(receiver)
    }

    /// Subscribe to user-specific events
    pub async fn subscribe_user_events(
        &self,
        user_id: String,
    ) -> Result<broadcast::Receiver<EventWithMetadata>, ServiceError> {
        let mut channels = self.user_channels.write().await;
        
        let sender = channels.entry(user_id.clone()).or_insert_with(|| {
            let (sender, _) = broadcast::channel(100);
            sender
        });

        let receiver = sender.subscribe();
        
        info!("User-specific subscription created for user: {}", user_id);
        Ok(receiver)
    }

    /// Publish event to user-specific channel
    async fn publish_to_user_channel(
        &self,
        user_id: &str,
        event: EventWithMetadata,
    ) -> Result<(), ServiceError> {
        let channels = self.user_channels.read().await;
        
        if let Some(sender) = channels.get(user_id) {
            if let Err(e) = sender.send(event) {
                warn!("Failed to publish to user channel {}: {}", user_id, e);
            }
        }

        Ok(())
    }

    /// Update event statistics
    async fn update_event_stats(&self, event: &ServiceEvent) {
        let mut stats = self.event_stats.write().await;
        
        stats.total_events += 1;
        stats.last_event_time = Some(Utc::now());
        
        let event_type = self.get_event_type_name(event);
        *stats.events_by_type.entry(event_type).or_insert(0) += 1;
    }

    /// Get event type name for statistics
    fn get_event_type_name(&self, event: &ServiceEvent) -> String {
        match event {
            ServiceEvent::WalletCreated { .. } => "wallet_created".to_string(),
            ServiceEvent::WalletUpdated { .. } => "wallet_updated".to_string(),
            ServiceEvent::KycSubmitted { .. } => "kyc_submitted".to_string(),
            ServiceEvent::KycStatusChanged { .. } => "kyc_status_changed".to_string(),
            ServiceEvent::CardCreated { .. } => "card_created".to_string(),
            ServiceEvent::CardTransactionProcessed { .. } => "card_transaction_processed".to_string(),
            ServiceEvent::CardStatusChanged { .. } => "card_status_changed".to_string(),
            ServiceEvent::TradingStrategyCreated { .. } => "trading_strategy_created".to_string(),
            ServiceEvent::TradeExecuted { .. } => "trade_executed".to_string(),
            ServiceEvent::DefiPositionOpened { .. } => "defi_position_opened".to_string(),
            ServiceEvent::DefiRewardsClaimed { .. } => "defi_rewards_claimed".to_string(),
            ServiceEvent::SystemAlert { .. } => "system_alert".to_string(),
            ServiceEvent::HealthCheckUpdate { .. } => "health_check_update".to_string(),
        }
    }

    /// Get event statistics
    pub async fn get_stats(&self) -> EventStats {
        self.event_stats.read().await.clone()
    }

    /// Cleanup inactive user channels
    pub async fn cleanup_inactive_channels(&self) {
        let mut channels = self.user_channels.write().await;
        
        // Remove channels with no active receivers
        channels.retain(|user_id, sender| {
            let has_receivers = sender.receiver_count() > 0;
            if !has_receivers {
                info!("Cleaning up inactive channel for user: {}", user_id);
            }
            has_receivers
        });
    }

    /// Publish wallet events
    pub async fn publish_wallet_created(
        &self,
        wallet_id: String,
        name: String,
        user_id: Option<String>,
    ) -> Result<(), ServiceError> {
        self.publish_event(
            ServiceEvent::WalletCreated {
                wallet_id,
                name,
                created_at: Utc::now(),
            },
            user_id,
            "wallet_service".to_string(),
            None,
        ).await
    }

    /// Publish KYC events
    pub async fn publish_kyc_status_changed(
        &self,
        submission_id: String,
        wallet_id: String,
        old_status: String,
        new_status: String,
        user_id: Option<String>,
    ) -> Result<(), ServiceError> {
        self.publish_event(
            ServiceEvent::KycStatusChanged {
                submission_id,
                wallet_id,
                old_status,
                new_status,
                changed_at: Utc::now(),
            },
            user_id,
            "kyc_service".to_string(),
            None,
        ).await
    }

    /// Publish card transaction events
    pub async fn publish_card_transaction(
        &self,
        transaction_id: String,
        card_id: String,
        amount: String,
        merchant_name: String,
        status: String,
        user_id: Option<String>,
    ) -> Result<(), ServiceError> {
        self.publish_event(
            ServiceEvent::CardTransactionProcessed {
                transaction_id,
                card_id,
                amount,
                merchant_name,
                status,
                processed_at: Utc::now(),
            },
            user_id,
            "card_service".to_string(),
            None,
        ).await
    }

    /// Publish system alerts
    pub async fn publish_system_alert(
        &self,
        alert_type: String,
        message: String,
        severity: String,
    ) -> Result<(), ServiceError> {
        self.publish_event(
            ServiceEvent::SystemAlert {
                alert_type,
                message,
                severity,
                timestamp: Utc::now(),
            },
            None,
            "system".to_string(),
            None,
        ).await
    }
}

impl Clone for EventStats {
    fn clone(&self) -> Self {
        Self {
            total_events: self.total_events,
            events_by_type: self.events_by_type.clone(),
            active_subscriptions: self.active_subscriptions,
            last_event_time: self.last_event_time,
        }
    }
}
