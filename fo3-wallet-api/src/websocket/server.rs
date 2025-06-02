//! WebSocket server implementation

use std::sync::Arc;
use std::net::SocketAddr;
use axum::{
    extract::{ws::{WebSocket, Message}, WebSocketUpgrade, State, Query},
    response::Response,
    routing::get,
    Router,
};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use tokio::sync::broadcast;
use uuid::Uuid;

use super::{WebSocketManager, WebSocketMessage, ConnectionInfo, ConnectionId, Subscription, SubscriptionRequest};
use crate::middleware::auth::AuthContext;

/// WebSocket server
pub struct WebSocketServer {
    manager: Arc<WebSocketManager>,
}

impl WebSocketServer {
    pub fn new(manager: Arc<WebSocketManager>) -> Self {
        Self { manager }
    }

    /// Create Axum router for WebSocket endpoints
    pub fn create_router(&self) -> Router {
        Router::new()
            .route("/ws", get(websocket_handler))
            .route("/ws/stats", get(websocket_stats_handler))
            .with_state(self.manager.clone())
    }

    /// Start WebSocket server on specified address
    pub async fn start(&self, addr: SocketAddr) -> Result<(), Box<dyn std::error::Error>> {
        let app = self.create_router();

        tracing::info!("Starting WebSocket server on {}", addr);

        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, app).await?;

        Ok(())
    }
}

/// Query parameters for WebSocket connection
#[derive(Deserialize)]
struct WebSocketQuery {
    token: Option<String>,
}

/// WebSocket upgrade handler
async fn websocket_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<WebSocketQuery>,
    State(manager): State<Arc<WebSocketManager>>,
) -> Response {
    ws.on_upgrade(move |socket| handle_websocket_connection(socket, manager, params.token))
}

/// Handle individual WebSocket connection
async fn handle_websocket_connection(
    socket: WebSocket,
    manager: Arc<WebSocketManager>,
    initial_token: Option<String>,
) {
    let connection_id = Uuid::new_v4().to_string();
    let (mut sender, mut receiver) = socket.split();
    
    // Authentication state
    let mut auth_context: Option<AuthContext> = None;
    let mut event_receiver: Option<broadcast::Receiver<crate::proto::fo3::wallet::v1::Event>> = None;

    // Try to authenticate with initial token if provided
    if let Some(token) = initial_token {
        match manager.authenticate_connection(&token).await {
            Ok(auth) => {
                auth_context = Some(auth.clone());
                event_receiver = Some(manager.get_event_receiver());
                
                let connection_info = ConnectionInfo {
                    id: connection_id.clone(),
                    user_id: auth.user_id.clone(),
                    subscriptions: vec![],
                    connected_at: chrono::Utc::now(),
                    last_ping: chrono::Utc::now(),
                };
                
                manager.add_connection(connection_info).await;
                
                // Send authentication success
                let auth_success = WebSocketMessage::Ack {
                    message_id: "auth_success".to_string(),
                };
                
                if let Ok(msg) = serde_json::to_string(&auth_success) {
                    let _ = sender.send(Message::Text(msg)).await;
                }
            }
            Err(e) => {
                let error_msg = WebSocketMessage::Error {
                    message: format!("Authentication failed: {}", e),
                    code: "AUTH_FAILED".to_string(),
                };
                
                if let Ok(msg) = serde_json::to_string(&error_msg) {
                    let _ = sender.send(Message::Text(msg)).await;
                }
                return;
            }
        }
    }

    // Spawn task to handle outgoing events
    let mut event_task = None;
    if let Some(mut rx) = event_receiver {
        let mut sender_clone = sender.clone();
        event_task = Some(tokio::spawn(async move {
            while let Ok(event) = rx.recv().await {
                let event_msg = WebSocketMessage::Event { event };
                
                if let Ok(msg) = serde_json::to_string(&event_msg) {
                    if sender_clone.send(Message::Text(msg)).await.is_err() {
                        break;
                    }
                }
            }
        }));
    }

    // Handle incoming messages
    while let Some(msg) = receiver.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                if let Err(e) = handle_websocket_message(
                    &text,
                    &connection_id,
                    &manager,
                    &mut auth_context,
                    &mut event_receiver,
                    &mut sender,
                ).await {
                    tracing::warn!("Error handling WebSocket message: {}", e);
                    
                    let error_msg = WebSocketMessage::Error {
                        message: e.to_string(),
                        code: "MESSAGE_ERROR".to_string(),
                    };
                    
                    if let Ok(msg) = serde_json::to_string(&error_msg) {
                        let _ = sender.send(Message::Text(msg)).await;
                    }
                }
            }
            Ok(Message::Binary(_)) => {
                // Binary messages not supported
                let error_msg = WebSocketMessage::Error {
                    message: "Binary messages not supported".to_string(),
                    code: "UNSUPPORTED_MESSAGE".to_string(),
                };
                
                if let Ok(msg) = serde_json::to_string(&error_msg) {
                    let _ = sender.send(Message::Text(msg)).await;
                }
            }
            Ok(Message::Ping(data)) => {
                // Respond to ping with pong
                let _ = sender.send(Message::Pong(data)).await;
                manager.update_ping(&connection_id).await;
            }
            Ok(Message::Pong(_)) => {
                // Update last ping time
                manager.update_ping(&connection_id).await;
            }
            Ok(Message::Close(_)) => {
                break;
            }
            Err(e) => {
                tracing::warn!("WebSocket error: {}", e);
                break;
            }
        }
    }

    // Clean up connection
    manager.remove_connection(&connection_id).await;
    
    if let Some(task) = event_task {
        task.abort();
    }
    
    tracing::info!("WebSocket connection closed: {}", connection_id);
}

/// Handle individual WebSocket message
async fn handle_websocket_message(
    text: &str,
    connection_id: &ConnectionId,
    manager: &Arc<WebSocketManager>,
    auth_context: &mut Option<AuthContext>,
    event_receiver: &mut Option<broadcast::Receiver<crate::proto::fo3::wallet::v1::Event>>,
    sender: &mut futures_util::stream::SplitSink<WebSocket, Message>,
) -> Result<(), Box<dyn std::error::Error>> {
    let message: WebSocketMessage = serde_json::from_str(text)?;

    match message {
        WebSocketMessage::Auth { token } => {
            match manager.authenticate_connection(&token).await {
                Ok(auth) => {
                    *auth_context = Some(auth.clone());
                    *event_receiver = Some(manager.get_event_receiver());
                    
                    let connection_info = ConnectionInfo {
                        id: connection_id.clone(),
                        user_id: auth.user_id.clone(),
                        subscriptions: vec![],
                        connected_at: chrono::Utc::now(),
                        last_ping: chrono::Utc::now(),
                    };
                    
                    manager.add_connection(connection_info).await;
                    
                    let success_msg = WebSocketMessage::Ack {
                        message_id: "auth_success".to_string(),
                    };
                    
                    let msg = serde_json::to_string(&success_msg)?;
                    sender.send(Message::Text(msg)).await?;
                }
                Err(e) => {
                    let error_msg = WebSocketMessage::Error {
                        message: format!("Authentication failed: {}", e),
                        code: "AUTH_FAILED".to_string(),
                    };
                    
                    let msg = serde_json::to_string(&error_msg)?;
                    sender.send(Message::Text(msg)).await?;
                }
            }
        }
        WebSocketMessage::Subscribe { subscription } => {
            if auth_context.is_none() {
                let error_msg = WebSocketMessage::Error {
                    message: "Not authenticated".to_string(),
                    code: "NOT_AUTHENTICATED".to_string(),
                };
                
                let msg = serde_json::to_string(&error_msg)?;
                sender.send(Message::Text(msg)).await?;
                return Ok(());
            }

            let sub = Subscription {
                event_types: subscription.event_types.into_iter()
                    .filter_map(|et| match et.as_str() {
                        "wallet_created" => Some(crate::proto::fo3::wallet::v1::EventType::EventTypeWalletCreated),
                        "wallet_updated" => Some(crate::proto::fo3::wallet::v1::EventType::EventTypeWalletUpdated),
                        "balance_changed" => Some(crate::proto::fo3::wallet::v1::EventType::EventTypeBalanceChanged),
                        "transaction_pending" => Some(crate::proto::fo3::wallet::v1::EventType::EventTypeTransactionPending),
                        "transaction_confirmed" => Some(crate::proto::fo3::wallet::v1::EventType::EventTypeTransactionConfirmed),
                        "transaction_failed" => Some(crate::proto::fo3::wallet::v1::EventType::EventTypeTransactionFailed),
                        _ => None,
                    })
                    .collect(),
                wallet_ids: subscription.wallet_ids,
                filters: subscription.filters,
            };

            manager.add_subscription(connection_id, sub).await;

            let ack_msg = WebSocketMessage::Ack {
                message_id: subscription.id,
            };
            
            let msg = serde_json::to_string(&ack_msg)?;
            sender.send(Message::Text(msg)).await?;
        }
        WebSocketMessage::Unsubscribe { subscription_id } => {
            manager.remove_subscription(connection_id, &subscription_id).await;
            
            let ack_msg = WebSocketMessage::Ack {
                message_id: subscription_id,
            };
            
            let msg = serde_json::to_string(&ack_msg)?;
            sender.send(Message::Text(msg)).await?;
        }
        WebSocketMessage::Ping { timestamp } => {
            manager.update_ping(connection_id).await;
            
            let pong_msg = WebSocketMessage::Pong { timestamp };
            let msg = serde_json::to_string(&pong_msg)?;
            sender.send(Message::Text(msg)).await?;
        }
        _ => {
            // Other message types are not expected from client
            let error_msg = WebSocketMessage::Error {
                message: "Unexpected message type".to_string(),
                code: "UNEXPECTED_MESSAGE".to_string(),
            };
            
            let msg = serde_json::to_string(&error_msg)?;
            sender.send(Message::Text(msg)).await?;
        }
    }

    Ok(())
}

/// WebSocket statistics handler
async fn websocket_stats_handler(
    State(manager): State<Arc<WebSocketManager>>,
) -> axum::Json<super::WebSocketStats> {
    let stats = manager.get_stats().await;
    axum::Json(stats)
}
