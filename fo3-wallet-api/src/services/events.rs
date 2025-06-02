//! Event streaming service implementation

use std::sync::Arc;
use tonic::{Request, Response, Status};
use tokio_stream::wrappers::ReceiverStream;
use tokio::sync::mpsc;

use crate::proto::fo3::wallet::v1::{
    event_service_server::EventService,
    *,
};
use crate::websocket::WebSocketManager;
use crate::middleware::auth::{AuthService, AuthContext};

pub struct EventServiceImpl {
    websocket_manager: Arc<WebSocketManager>,
    auth_service: Arc<AuthService>,
}

impl EventServiceImpl {
    pub fn new(websocket_manager: Arc<WebSocketManager>, auth_service: Arc<AuthService>) -> Self {
        Self {
            websocket_manager,
            auth_service,
        }
    }

    /// Helper function to check authentication and get user context
    async fn get_auth_context(&self, request: &Request<()>) -> Result<AuthContext, Status> {
        self.auth_service.extract_auth(request).await
    }

    /// Convert proto event types to internal event types
    fn convert_event_types(&self, event_types: Vec<i32>) -> Vec<EventType> {
        event_types.into_iter()
            .filter_map(|et| EventType::try_from(et).ok())
            .collect()
    }
}

#[tonic::async_trait]
impl EventService for EventServiceImpl {
    type SubscribeWalletEventsStream = ReceiverStream<Result<WalletEvent, Status>>;

    async fn subscribe_wallet_events(
        &self,
        request: Request<SubscribeWalletEventsRequest>,
    ) -> Result<Response<Self::SubscribeWalletEventsStream>, Status> {
        // Authenticate the request
        let auth_request = Request::from_parts(request.metadata().clone(), ());
        let auth_context = self.get_auth_context(&auth_request).await?;

        let req = request.into_inner();
        let (tx, rx) = mpsc::channel(100);

        // Get event receiver from WebSocket manager
        let mut event_receiver = self.websocket_manager.get_event_receiver();
        let wallet_ids = req.wallet_ids;
        let event_types = self.convert_event_types(req.event_types);
        let user_id = auth_context.user_id.clone();

        // Spawn task to filter and forward events
        tokio::spawn(async move {
            while let Ok(event) = event_receiver.recv().await {
                // Check if event is for this user
                if event.user_id != user_id {
                    continue;
                }

                // Check wallet ID filter
                if !wallet_ids.is_empty() && !wallet_ids.contains(&event.wallet_id) {
                    continue;
                }

                // Check event type filter
                if !event_types.is_empty() && !event_types.contains(&event.r#type()) {
                    continue;
                }

                // Extract wallet event if present
                if let Some(event_data) = &event.event_data {
                    if let crate::proto::fo3::wallet::v1::event::EventData::WalletEvent(wallet_event) = event_data {
                        if tx.send(Ok(wallet_event.clone())).await.is_err() {
                            break;
                        }
                    }
                }
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    type SubscribeTransactionEventsStream = ReceiverStream<Result<TransactionEvent, Status>>;

    async fn subscribe_transaction_events(
        &self,
        request: Request<SubscribeTransactionEventsRequest>,
    ) -> Result<Response<Self::SubscribeTransactionEventsStream>, Status> {
        // Authenticate the request
        let auth_request = Request::from_parts(request.metadata().clone(), ());
        let auth_context = self.get_auth_context(&auth_request).await?;

        let req = request.into_inner();
        let (tx, rx) = mpsc::channel(100);

        // Get event receiver from WebSocket manager
        let mut event_receiver = self.websocket_manager.get_event_receiver();
        let wallet_ids = req.wallet_ids;
        let event_types = self.convert_event_types(req.event_types);
        let user_id = auth_context.user_id.clone();

        // Spawn task to filter and forward events
        tokio::spawn(async move {
            while let Ok(event) = event_receiver.recv().await {
                // Check if event is for this user
                if event.user_id != user_id {
                    continue;
                }

                // Check wallet ID filter
                if !wallet_ids.is_empty() && !wallet_ids.contains(&event.wallet_id) {
                    continue;
                }

                // Check event type filter
                if !event_types.is_empty() && !event_types.contains(&event.r#type()) {
                    continue;
                }

                // Extract transaction event if present
                if let Some(event_data) = &event.event_data {
                    if let crate::proto::fo3::wallet::v1::event::EventData::TransactionEvent(tx_event) = event_data {
                        if tx.send(Ok(tx_event.clone())).await.is_err() {
                            break;
                        }
                    }
                }
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    type SubscribeDefiEventsStream = ReceiverStream<Result<DefiEvent, Status>>;

    async fn subscribe_defi_events(
        &self,
        request: Request<SubscribeDefiEventsRequest>,
    ) -> Result<Response<Self::SubscribeDefiEventsStream>, Status> {
        // Authenticate the request
        let auth_request = Request::from_parts(request.metadata().clone(), ());
        let auth_context = self.get_auth_context(&auth_request).await?;

        let req = request.into_inner();
        let (tx, rx) = mpsc::channel(100);

        // Get event receiver from WebSocket manager
        let mut event_receiver = self.websocket_manager.get_event_receiver();
        let wallet_ids = req.wallet_ids;
        let event_types = self.convert_event_types(req.event_types);
        let user_id = auth_context.user_id.clone();

        // Spawn task to filter and forward events
        tokio::spawn(async move {
            while let Ok(event) = event_receiver.recv().await {
                // Check if event is for this user
                if event.user_id != user_id {
                    continue;
                }

                // Check wallet ID filter
                if !wallet_ids.is_empty() && !wallet_ids.contains(&event.wallet_id) {
                    continue;
                }

                // Check event type filter
                if !event_types.is_empty() && !event_types.contains(&event.r#type()) {
                    continue;
                }

                // Extract DeFi event if present
                if let Some(event_data) = &event.event_data {
                    if let crate::proto::fo3::wallet::v1::event::EventData::DefiEvent(defi_event) = event_data {
                        if tx.send(Ok(defi_event.clone())).await.is_err() {
                            break;
                        }
                    }
                }
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    type SubscribeSolanaEventsStream = ReceiverStream<Result<SolanaEvent, Status>>;

    async fn subscribe_solana_events(
        &self,
        request: Request<SubscribeSolanaEventsRequest>,
    ) -> Result<Response<Self::SubscribeSolanaEventsStream>, Status> {
        // Authenticate the request
        let auth_request = Request::from_parts(request.metadata().clone(), ());
        let auth_context = self.get_auth_context(&auth_request).await?;

        let req = request.into_inner();
        let (tx, rx) = mpsc::channel(100);

        // Get event receiver from WebSocket manager
        let mut event_receiver = self.websocket_manager.get_event_receiver();
        let wallet_ids = req.wallet_ids;
        let event_types = self.convert_event_types(req.event_types);
        let user_id = auth_context.user_id.clone();

        // Spawn task to filter and forward events
        tokio::spawn(async move {
            while let Ok(event) = event_receiver.recv().await {
                // Check if event is for this user
                if event.user_id != user_id {
                    continue;
                }

                // Check wallet ID filter
                if !wallet_ids.is_empty() && !wallet_ids.contains(&event.wallet_id) {
                    continue;
                }

                // Check event type filter
                if !event_types.is_empty() && !event_types.contains(&event.r#type()) {
                    continue;
                }

                // Extract Solana event if present
                if let Some(event_data) = &event.event_data {
                    if let crate::proto::fo3::wallet::v1::event::EventData::SolanaEvent(solana_event) = event_data {
                        if tx.send(Ok(solana_event.clone())).await.is_err() {
                            break;
                        }
                    }
                }
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    type SubscribeAllEventsStream = ReceiverStream<Result<Event, Status>>;

    async fn subscribe_all_events(
        &self,
        request: Request<SubscribeAllEventsRequest>,
    ) -> Result<Response<Self::SubscribeAllEventsStream>, Status> {
        // Authenticate the request
        let auth_request = Request::from_parts(request.metadata().clone(), ());
        let auth_context = self.get_auth_context(&auth_request).await?;

        let req = request.into_inner();
        let (tx, rx) = mpsc::channel(100);

        // Get event receiver from WebSocket manager
        let mut event_receiver = self.websocket_manager.get_event_receiver();
        let wallet_ids = req.wallet_ids;
        let event_types = self.convert_event_types(req.event_types);
        let user_id = auth_context.user_id.clone();

        // Spawn task to filter and forward events
        tokio::spawn(async move {
            while let Ok(event) = event_receiver.recv().await {
                // Check if event is for this user
                if event.user_id != user_id {
                    continue;
                }

                // Check wallet ID filter
                if !wallet_ids.is_empty() && !wallet_ids.contains(&event.wallet_id) {
                    continue;
                }

                // Check event type filter
                if !event_types.is_empty() && !event_types.contains(&event.r#type()) {
                    continue;
                }

                if tx.send(Ok(event)).await.is_err() {
                    break;
                }
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }
}
