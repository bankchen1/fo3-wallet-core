//! WebSocket server for real-time notifications

pub mod server;
pub mod connection;
pub mod events;

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};
use uuid::Uuid;

use crate::proto::fo3::wallet::v1::{Event, EventType};
use crate::middleware::auth::AuthContext;

/// WebSocket connection identifier
pub type ConnectionId = String;

/// WebSocket connection information
#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    pub id: ConnectionId,
    pub user_id: String,
    pub subscriptions: Vec<Subscription>,
    pub connected_at: chrono::DateTime<chrono::Utc>,
    pub last_ping: chrono::DateTime<chrono::Utc>,
}

/// Event subscription
#[derive(Debug, Clone)]
pub struct Subscription {
    pub event_types: Vec<EventType>,
    pub wallet_ids: Vec<String>,
    pub filters: HashMap<String, String>,
}

/// WebSocket message types
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type")]
pub enum WebSocketMessage {
    #[serde(rename = "auth")]
    Auth {
        token: String,
    },
    #[serde(rename = "subscribe")]
    Subscribe {
        subscription: SubscriptionRequest,
    },
    #[serde(rename = "unsubscribe")]
    Unsubscribe {
        subscription_id: String,
    },
    #[serde(rename = "ping")]
    Ping {
        timestamp: i64,
    },
    #[serde(rename = "pong")]
    Pong {
        timestamp: i64,
    },
    #[serde(rename = "event")]
    Event {
        event: Event,
    },
    #[serde(rename = "error")]
    Error {
        message: String,
        code: String,
    },
    #[serde(rename = "ack")]
    Ack {
        message_id: String,
    },
}

/// Subscription request
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SubscriptionRequest {
    pub id: String,
    pub event_types: Vec<String>,
    pub wallet_ids: Vec<String>,
    pub filters: HashMap<String, String>,
}

/// WebSocket manager for handling connections and event distribution
pub struct WebSocketManager {
    connections: Arc<RwLock<HashMap<ConnectionId, ConnectionInfo>>>,
    event_sender: broadcast::Sender<Event>,
    auth_service: Arc<crate::middleware::auth::AuthService>,
}

impl WebSocketManager {
    pub fn new(auth_service: Arc<crate::middleware::auth::AuthService>) -> Self {
        let (event_sender, _) = broadcast::channel(1000);
        
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            event_sender,
            auth_service,
        }
    }

    /// Add a new WebSocket connection
    pub async fn add_connection(&self, connection_info: ConnectionInfo) {
        let mut connections = self.connections.write().await;
        connections.insert(connection_info.id.clone(), connection_info);
    }

    /// Remove a WebSocket connection
    pub async fn remove_connection(&self, connection_id: &ConnectionId) {
        let mut connections = self.connections.write().await;
        connections.remove(connection_id);
    }

    /// Update connection's last ping time
    pub async fn update_ping(&self, connection_id: &ConnectionId) {
        let mut connections = self.connections.write().await;
        if let Some(conn) = connections.get_mut(connection_id) {
            conn.last_ping = chrono::Utc::now();
        }
    }

    /// Add subscription to a connection
    pub async fn add_subscription(&self, connection_id: &ConnectionId, subscription: Subscription) {
        let mut connections = self.connections.write().await;
        if let Some(conn) = connections.get_mut(connection_id) {
            conn.subscriptions.push(subscription);
        }
    }

    /// Remove subscription from a connection
    pub async fn remove_subscription(&self, connection_id: &ConnectionId, subscription_id: &str) {
        let mut connections = self.connections.write().await;
        if let Some(conn) = connections.get_mut(connection_id) {
            conn.subscriptions.retain(|sub| {
                // In a real implementation, subscriptions would have IDs
                // For now, we'll use a simple filter
                true
            });
        }
    }

    /// Broadcast event to all matching connections
    pub async fn broadcast_event(&self, event: Event) {
        let connections = self.connections.read().await;

        for (connection_id, conn_info) in connections.iter() {
            if self.should_send_event_to_connection(&event, conn_info) {
                // Send event to specific connection
                if let Err(e) = self.event_sender.send(event.clone()) {
                    tracing::warn!("Failed to send event to connection {}: {}", connection_id, e);
                }
            }
        }
    }

    /// Send message to a specific user
    pub async fn send_to_user(&self, user_id: &str, message: &str) -> bool {
        let connections = self.connections.read().await;

        for (connection_id, conn_info) in connections.iter() {
            if conn_info.user_id == user_id {
                // In a real implementation, this would send the message through the WebSocket
                // For now, we'll just log it and return success
                tracing::info!("Sending WebSocket message to user {} (connection {}): {}", user_id, connection_id, message);
                return true;
            }
        }

        false
    }

    /// Check if an event should be sent to a specific connection
    fn should_send_event_to_connection(&self, event: &Event, conn_info: &ConnectionInfo) -> bool {
        // Check if the event is for this user
        if event.user_id != conn_info.user_id {
            return false;
        }

        // Check subscriptions
        for subscription in &conn_info.subscriptions {
            // Check event type filter
            if !subscription.event_types.is_empty() && !subscription.event_types.contains(&event.r#type()) {
                continue;
            }

            // Check wallet ID filter
            if !subscription.wallet_ids.is_empty() && !subscription.wallet_ids.contains(&event.wallet_id) {
                continue;
            }

            // If we get here, the event matches this subscription
            return true;
        }

        false
    }

    /// Get event receiver for a connection
    pub fn get_event_receiver(&self) -> broadcast::Receiver<Event> {
        self.event_sender.subscribe()
    }

    /// Authenticate WebSocket connection
    pub async fn authenticate_connection(&self, token: &str) -> Result<AuthContext, String> {
        self.auth_service.validate_jwt(token)
            .map(|claims| AuthContext {
                user_id: claims.sub,
                username: claims.username,
                role: crate::proto::fo3::wallet::v1::UserRole::try_from(claims.role)
                    .unwrap_or(crate::proto::fo3::wallet::v1::UserRole::UserRoleUser),
                permissions: claims.permissions.into_iter()
                    .filter_map(|p| crate::proto::fo3::wallet::v1::Permission::try_from(p).ok())
                    .collect(),
                auth_type: crate::middleware::auth::AuthType::JWT(token.to_string()),
            })
            .map_err(|e| e.message().to_string())
    }

    /// Get connection statistics
    pub async fn get_stats(&self) -> WebSocketStats {
        let connections = self.connections.read().await;
        let total_connections = connections.len();
        let mut users = std::collections::HashSet::new();
        let mut total_subscriptions = 0;

        for conn in connections.values() {
            users.insert(&conn.user_id);
            total_subscriptions += conn.subscriptions.len();
        }

        WebSocketStats {
            total_connections,
            unique_users: users.len(),
            total_subscriptions,
        }
    }

    /// Clean up stale connections
    pub async fn cleanup_stale_connections(&self) {
        let mut connections = self.connections.write().await;
        let now = chrono::Utc::now();
        let stale_threshold = chrono::Duration::minutes(5);

        connections.retain(|connection_id, conn_info| {
            let is_stale = now.signed_duration_since(conn_info.last_ping) > stale_threshold;
            if is_stale {
                tracing::info!("Removing stale WebSocket connection: {}", connection_id);
            }
            !is_stale
        });
    }
}

/// WebSocket connection statistics
#[derive(Debug, serde::Serialize)]
pub struct WebSocketStats {
    pub total_connections: usize,
    pub unique_users: usize,
    pub total_subscriptions: usize,
}

/// Background task to clean up stale connections
pub async fn cleanup_stale_connections_task(manager: Arc<WebSocketManager>) {
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60)); // Check every minute
    
    loop {
        interval.tick().await;
        manager.cleanup_stale_connections().await;
    }
}
