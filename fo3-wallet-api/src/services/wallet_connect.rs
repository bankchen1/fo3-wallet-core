//! WalletConnect service implementation

use std::sync::Arc;
use std::collections::HashMap;
use tonic::{Request, Response, Status};
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::proto::fo3::wallet::v1::{
    wallet_connect_service_server::WalletConnectService,
    *,
};
use crate::state::AppState;
use crate::middleware::{
    auth::{AuthContext, AuthService},
    audit::AuditLogger,
    wallet_connect_guard::WalletConnectGuard,
};
use crate::models::wallet_connect::{
    WalletConnectSession, DAppInfo, SessionRequest, SessionAnalytics,
    SessionStatus, RequestType, RequestStatus, KeyType as WCKeyType,
    WalletConnectRepository,
};
use crate::models::notifications::{
    NotificationType, NotificationPriority, DeliveryChannel
};

/// WalletConnect service implementation
#[derive(Debug)]
pub struct WalletConnectServiceImpl {
    state: Arc<AppState>,
    auth_service: Arc<AuthService>,
    audit_logger: Arc<AuditLogger>,
    wallet_connect_guard: Arc<WalletConnectGuard>,
    wallet_connect_repository: Arc<dyn WalletConnectRepository>,
}

impl WalletConnectServiceImpl {
    pub fn new(
        state: Arc<AppState>,
        auth_service: Arc<AuthService>,
        audit_logger: Arc<AuditLogger>,
        wallet_connect_guard: Arc<WalletConnectGuard>,
        wallet_connect_repository: Arc<dyn WalletConnectRepository>,
    ) -> Self {
        Self {
            state,
            auth_service,
            audit_logger,
            wallet_connect_guard,
            wallet_connect_repository,
        }
    }

    /// Convert proto KeyType to model KeyType
    fn proto_to_model_key_type(proto_type: i32) -> Result<WCKeyType, Status> {
        match KeyType::try_from(proto_type) {
            Ok(KeyType::KeyTypeEthereum) => Ok(WCKeyType::Ethereum),
            Ok(KeyType::KeyTypeBitcoin) => Ok(WCKeyType::Bitcoin),
            Ok(KeyType::KeyTypeSolana) => Ok(WCKeyType::Solana),
            _ => Err(Status::invalid_argument("Invalid key type")),
        }
    }

    /// Convert model KeyType to proto KeyType
    fn model_to_proto_key_type(model_type: WCKeyType) -> KeyType {
        match model_type {
            WCKeyType::Ethereum => KeyType::KeyTypeEthereum,
            WCKeyType::Bitcoin => KeyType::KeyTypeBitcoin,
            WCKeyType::Solana => KeyType::KeyTypeSolana,
        }
    }

    /// Convert proto SessionStatus to model SessionStatus
    fn proto_to_model_session_status(proto_status: i32) -> Result<SessionStatus, Status> {
        match SessionStatus::try_from(proto_status) {
            Ok(SessionStatus::SessionStatusPending) => Ok(crate::models::wallet_connect::SessionStatus::Pending),
            Ok(SessionStatus::SessionStatusActive) => Ok(crate::models::wallet_connect::SessionStatus::Active),
            Ok(SessionStatus::SessionStatusExpired) => Ok(crate::models::wallet_connect::SessionStatus::Expired),
            Ok(SessionStatus::SessionStatusTerminated) => Ok(crate::models::wallet_connect::SessionStatus::Terminated),
            Ok(SessionStatus::SessionStatusSuspended) => Ok(crate::models::wallet_connect::SessionStatus::Suspended),
            _ => Err(Status::invalid_argument("Invalid session status")),
        }
    }

    /// Convert model SessionStatus to proto SessionStatus
    fn model_to_proto_session_status(model_status: crate::models::wallet_connect::SessionStatus) -> SessionStatus {
        match model_status {
            crate::models::wallet_connect::SessionStatus::Pending => SessionStatus::SessionStatusPending,
            crate::models::wallet_connect::SessionStatus::Active => SessionStatus::SessionStatusActive,
            crate::models::wallet_connect::SessionStatus::Expired => SessionStatus::SessionStatusExpired,
            crate::models::wallet_connect::SessionStatus::Terminated => SessionStatus::SessionStatusTerminated,
            crate::models::wallet_connect::SessionStatus::Suspended => SessionStatus::SessionStatusSuspended,
        }
    }

    /// Convert proto RequestType to model RequestType
    fn proto_to_model_request_type(proto_type: i32) -> Result<RequestType, Status> {
        match RequestType::try_from(proto_type) {
            Ok(RequestType::RequestTypeSignMessage) => Ok(crate::models::wallet_connect::RequestType::SignMessage),
            Ok(RequestType::RequestTypeSignTransaction) => Ok(crate::models::wallet_connect::RequestType::SignTransaction),
            Ok(RequestType::RequestTypeSendTransaction) => Ok(crate::models::wallet_connect::RequestType::SendTransaction),
            Ok(RequestType::RequestTypeSwitchChain) => Ok(crate::models::wallet_connect::RequestType::SwitchChain),
            Ok(RequestType::RequestTypeAddChain) => Ok(crate::models::wallet_connect::RequestType::AddChain),
            Ok(RequestType::RequestTypeWatchAsset) => Ok(crate::models::wallet_connect::RequestType::WatchAsset),
            _ => Err(Status::invalid_argument("Invalid request type")),
        }
    }

    /// Convert model RequestType to proto RequestType
    fn model_to_proto_request_type(model_type: crate::models::wallet_connect::RequestType) -> RequestType {
        match model_type {
            crate::models::wallet_connect::RequestType::SignMessage => RequestType::RequestTypeSignMessage,
            crate::models::wallet_connect::RequestType::SignTransaction => RequestType::RequestTypeSignTransaction,
            crate::models::wallet_connect::RequestType::SendTransaction => RequestType::RequestTypeSendTransaction,
            crate::models::wallet_connect::RequestType::SwitchChain => RequestType::RequestTypeSwitchChain,
            crate::models::wallet_connect::RequestType::AddChain => RequestType::RequestTypeAddChain,
            crate::models::wallet_connect::RequestType::WatchAsset => RequestType::RequestTypeWatchAsset,
        }
    }

    /// Convert model WalletConnectSession to proto
    fn model_to_proto_session(session: &WalletConnectSession) -> WalletConnectSession {
        WalletConnectSession {
            session_id: session.session_id.to_string(),
            user_id: session.user_id.to_string(),
            dapp_url: session.dapp_url.clone(),
            dapp_name: session.dapp_name.clone(),
            dapp_description: session.dapp_description.clone(),
            dapp_icons: session.dapp_icons.clone(),
            supported_chains: session.supported_chains.iter()
                .map(|&kt| Self::model_to_proto_key_type(kt) as i32)
                .collect(),
            accounts: session.accounts.clone(),
            status: Self::model_to_proto_session_status(session.status) as i32,
            created_at: session.created_at.timestamp(),
            updated_at: session.updated_at.timestamp(),
            expires_at: session.expires_at.timestamp(),
            bridge_url: session.bridge_url.clone(),
            key: session.key.clone(),
            peer_id: session.peer_id.clone(),
            metadata: session.metadata.clone(),
        }
    }

    /// Convert model DAppInfo to proto
    fn model_to_proto_dapp_info(dapp_info: &DAppInfo) -> DAppInfo {
        DAppInfo {
            url: dapp_info.url.clone(),
            name: dapp_info.name.clone(),
            description: dapp_info.description.clone(),
            icons: dapp_info.icons.clone(),
            version: dapp_info.version.clone(),
            supported_chains: dapp_info.supported_chains.iter()
                .map(|&kt| Self::model_to_proto_key_type(kt) as i32)
                .collect(),
            metadata: dapp_info.metadata.clone(),
            first_connected_at: dapp_info.first_connected_at.timestamp(),
            last_connected_at: dapp_info.last_connected_at.timestamp(),
            connection_count: dapp_info.connection_count,
            is_trusted: dapp_info.is_trusted,
            is_flagged: dapp_info.is_flagged,
        }
    }

    /// Convert model SessionRequest to proto
    fn model_to_proto_session_request(request: &SessionRequest) -> SessionRequest {
        SessionRequest {
            request_id: request.request_id.to_string(),
            session_id: request.session_id.to_string(),
            user_id: request.user_id.to_string(),
            request_type: Self::model_to_proto_request_type(request.request_type) as i32,
            status: match request.status {
                crate::models::wallet_connect::RequestStatus::Pending => RequestStatus::RequestStatusPending as i32,
                crate::models::wallet_connect::RequestStatus::Approved => RequestStatus::RequestStatusApproved as i32,
                crate::models::wallet_connect::RequestStatus::Rejected => RequestStatus::RequestStatusRejected as i32,
                crate::models::wallet_connect::RequestStatus::Expired => RequestStatus::RequestStatusExpired as i32,
            },
            method: request.method.clone(),
            params: request.params.clone(),
            result: request.result.clone().unwrap_or_default(),
            error_message: request.error_message.clone().unwrap_or_default(),
            created_at: request.created_at.timestamp(),
            updated_at: request.updated_at.timestamp(),
            expires_at: request.expires_at.timestamp(),
            chain_type: Self::model_to_proto_key_type(request.chain_type) as i32,
            chain_id: request.chain_id.clone(),
            metadata: request.metadata.clone(),
        }
    }

    /// Send notification to user
    async fn send_notification(
        &self,
        user_id: &Uuid,
        notification_type: NotificationType,
        title: &str,
        message: &str,
        metadata: Option<HashMap<String, String>>,
    ) {
        if let Err(e) = self.state.notification_service.send_notification(
            user_id,
            notification_type,
            NotificationPriority::Medium,
            title,
            message,
            vec![DeliveryChannel::Push, DeliveryChannel::InApp],
            metadata,
        ).await {
            tracing::warn!("Failed to send notification: {}", e);
        }
    }
}

#[tonic::async_trait]
impl WalletConnectService for WalletConnectServiceImpl {
    /// Create a new WalletConnect session
    async fn create_session(
        &self,
        request: Request<CreateSessionRequest>,
    ) -> Result<Response<CreateSessionResponse>, Status> {
        let req = request.get_ref();

        // Parse supported chains
        let supported_chains: Result<Vec<WCKeyType>, Status> = req.supported_chains.iter()
            .map(|&chain| Self::proto_to_model_key_type(chain))
            .collect();
        let supported_chains = supported_chains?;

        // Validate request with security guard
        let auth_context = self.wallet_connect_guard
            .validate_session_creation(&request, &req.dapp_url, &req.dapp_name, &supported_chains)
            .await?;

        // Parse user ID
        let user_id = Uuid::parse_str(&auth_context.user_id)
            .map_err(|_| Status::invalid_argument("Invalid user ID"))?;

        // Set default expiration if not provided (24 hours)
        let expires_in_seconds = if req.expires_in_seconds > 0 {
            req.expires_in_seconds
        } else {
            86400 // 24 hours
        };

        // Create new session
        let session = WalletConnectSession::new(
            user_id,
            req.dapp_url.clone(),
            req.dapp_name.clone(),
            req.dapp_description.clone(),
            req.dapp_icons.clone(),
            supported_chains,
            req.bridge_url.clone(),
            expires_in_seconds,
        );

        // Save session to repository
        let created_session = self.wallet_connect_repository
            .create_session(&session)
            .await
            .map_err(|e| Status::internal(format!("Failed to create session: {}", e)))?;

        // Generate connection URI (WalletConnect format)
        let connection_uri = format!(
            "wc:{}@1?bridge={}&key={}",
            created_session.session_id,
            urlencoding::encode(&created_session.bridge_url),
            created_session.key
        );

        // Send notification to user
        self.send_notification(
            &user_id,
            NotificationType::WalletConnectSession,
            "New DApp Connection",
            &format!("New connection request from {}", req.dapp_name),
            Some([("session_id".to_string(), created_session.session_id.to_string())].into()),
        ).await;

        // Log session creation
        self.audit_logger.log_action(
            &user_id.to_string(),
            "session_created",
            &format!("DApp: {}, Session: {}", req.dapp_name, created_session.session_id),
            true,
            None,
        ).await;

        let response = CreateSessionResponse {
            session: Some(Self::model_to_proto_session(&created_session)),
            connection_uri,
        };

        Ok(Response::new(response))
    }

    /// Get a WalletConnect session by ID
    async fn get_session(
        &self,
        request: Request<GetSessionRequest>,
    ) -> Result<Response<GetSessionResponse>, Status> {
        let req = request.get_ref();

        // Parse session ID
        let session_id = Uuid::parse_str(&req.session_id)
            .map_err(|_| Status::invalid_argument("Invalid session ID"))?;

        // Validate session access
        let _auth_context = self.wallet_connect_guard
            .validate_session_access(&request, &session_id)
            .await?;

        // Get session from repository
        let session = self.wallet_connect_repository
            .get_session(&session_id)
            .await
            .map_err(|e| Status::internal(format!("Failed to get session: {}", e)))?
            .ok_or_else(|| Status::not_found("Session not found"))?;

        let response = GetSessionResponse {
            session: Some(Self::model_to_proto_session(&session)),
        };

        Ok(Response::new(response))
    }

    /// List WalletConnect sessions
    async fn list_sessions(
        &self,
        request: Request<ListSessionsRequest>,
    ) -> Result<Response<ListSessionsResponse>, Status> {
        let req = request.get_ref();

        // Extract authentication
        let auth_context = self.auth_service.extract_auth(&request).await?;

        // Parse optional filters
        let user_id = if req.user_id.is_empty() {
            Some(Uuid::parse_str(&auth_context.user_id)
                .map_err(|_| Status::invalid_argument("Invalid user ID"))?)
        } else {
            // Check if user can list sessions for other users
            let target_user_id = Uuid::parse_str(&req.user_id)
                .map_err(|_| Status::invalid_argument("Invalid target user ID"))?;

            if target_user_id != Uuid::parse_str(&auth_context.user_id).unwrap() &&
               !self.auth_service.has_permission(&auth_context, Permission::ManageWalletConnect).await? {
                return Err(Status::permission_denied("Can only list your own sessions"));
            }

            Some(target_user_id)
        };

        let status = if req.status != 0 {
            Some(Self::proto_to_model_session_status(req.status)?)
        } else {
            None
        };

        let chain_type = if req.chain_type != 0 {
            Some(Self::proto_to_model_key_type(req.chain_type)?)
        } else {
            None
        };

        let created_after = if req.created_after > 0 {
            Some(DateTime::from_timestamp(req.created_after, 0)
                .ok_or_else(|| Status::invalid_argument("Invalid created_after timestamp"))?)
        } else {
            None
        };

        let created_before = if req.created_before > 0 {
            Some(DateTime::from_timestamp(req.created_before, 0)
                .ok_or_else(|| Status::invalid_argument("Invalid created_before timestamp"))?)
        } else {
            None
        };

        let dapp_url = if req.dapp_url.is_empty() {
            None
        } else {
            Some(req.dapp_url.clone())
        };

        // Set pagination defaults
        let page_size = if req.page_size > 0 && req.page_size <= 100 {
            req.page_size
        } else {
            20
        };

        let page = if req.page_token.is_empty() {
            1
        } else {
            req.page_token.parse::<i32>()
                .map_err(|_| Status::invalid_argument("Invalid page token"))?
        };

        // Get sessions from repository
        let (sessions, total_count) = self.wallet_connect_repository
            .list_sessions(
                user_id,
                status,
                dapp_url,
                chain_type,
                created_after,
                created_before,
                page,
                page_size,
            )
            .await
            .map_err(|e| Status::internal(format!("Failed to list sessions: {}", e)))?;

        // Convert to proto
        let proto_sessions: Vec<WalletConnectSession> = sessions.iter()
            .map(Self::model_to_proto_session)
            .collect();

        // Generate next page token
        let next_page_token = if (page * page_size) < total_count as i32 {
            (page + 1).to_string()
        } else {
            String::new()
        };

        let response = ListSessionsResponse {
            sessions: proto_sessions,
            next_page_token,
            total_count: total_count as i32,
        };

        Ok(Response::new(response))
    }

    /// Update a WalletConnect session
    async fn update_session(
        &self,
        request: Request<UpdateSessionRequest>,
    ) -> Result<Response<UpdateSessionResponse>, Status> {
        let req = request.get_ref();

        // Parse session ID
        let session_id = Uuid::parse_str(&req.session_id)
            .map_err(|_| Status::invalid_argument("Invalid session ID"))?;

        // Validate session access
        let _auth_context = self.wallet_connect_guard
            .validate_session_access(&request, &session_id)
            .await?;

        // Get existing session
        let mut session = self.wallet_connect_repository
            .get_session(&session_id)
            .await
            .map_err(|e| Status::internal(format!("Failed to get session: {}", e)))?
            .ok_or_else(|| Status::not_found("Session not found"))?;

        // Update fields if provided
        if req.status != 0 {
            session.status = Self::proto_to_model_session_status(req.status)?;
        }

        if !req.accounts.is_empty() {
            session.accounts = req.accounts.clone();
        }

        if !req.supported_chains.is_empty() {
            let supported_chains: Result<Vec<WCKeyType>, Status> = req.supported_chains.iter()
                .map(|&chain| Self::proto_to_model_key_type(chain))
                .collect();
            session.supported_chains = supported_chains?;
        }

        if req.expires_at > 0 {
            session.expires_at = DateTime::from_timestamp(req.expires_at, 0)
                .ok_or_else(|| Status::invalid_argument("Invalid expires_at timestamp"))?;
        }

        // Update metadata
        for (key, value) in &req.metadata {
            session.metadata.insert(key.clone(), value.clone());
        }

        session.updated_at = Utc::now();

        // Save updated session
        let updated_session = self.wallet_connect_repository
            .update_session(&session)
            .await
            .map_err(|e| Status::internal(format!("Failed to update session: {}", e)))?;

        // Log session update
        self.audit_logger.log_action(
            &session.user_id.to_string(),
            "session_updated",
            &format!("Session: {}", session_id),
            true,
            None,
        ).await;

        let response = UpdateSessionResponse {
            session: Some(Self::model_to_proto_session(&updated_session)),
        };

        Ok(Response::new(response))
    }

    /// Delete a WalletConnect session
    async fn delete_session(
        &self,
        request: Request<DeleteSessionRequest>,
    ) -> Result<Response<DeleteSessionResponse>, Status> {
        let req = request.get_ref();

        // Parse session ID
        let session_id = Uuid::parse_str(&req.session_id)
            .map_err(|_| Status::invalid_argument("Invalid session ID"))?;

        // Validate session access
        let _auth_context = self.wallet_connect_guard
            .validate_session_access(&request, &session_id)
            .await?;

        // Get session for logging
        let session = self.wallet_connect_repository
            .get_session(&session_id)
            .await
            .map_err(|e| Status::internal(format!("Failed to get session: {}", e)))?;

        // Delete session
        let success = self.wallet_connect_repository
            .delete_session(&session_id)
            .await
            .map_err(|e| Status::internal(format!("Failed to delete session: {}", e)))?;

        if let Some(session) = session {
            // Send notification to user
            self.send_notification(
                &session.user_id,
                NotificationType::WalletConnectSession,
                "DApp Disconnected",
                &format!("Disconnected from {}", session.dapp_name),
                Some([("session_id".to_string(), session_id.to_string())].into()),
            ).await;

            // Log session deletion
            self.audit_logger.log_action(
                &session.user_id.to_string(),
                "session_deleted",
                &format!("Session: {}, Reason: {}", session_id, req.reason),
                true,
                None,
            ).await;
        }

        let response = DeleteSessionResponse { success };

        Ok(Response::new(response))
    }

    /// Connect DApp to session
    async fn connect_d_app(
        &self,
        request: Request<ConnectDAppRequest>,
    ) -> Result<Response<ConnectDAppResponse>, Status> {
        let req = request.get_ref();

        // Parse session ID
        let session_id = Uuid::parse_str(&req.session_id)
            .map_err(|_| Status::invalid_argument("Invalid session ID"))?;

        // Validate session access
        let _auth_context = self.wallet_connect_guard
            .validate_session_access(&request, &session_id)
            .await?;

        // Get existing session
        let mut session = self.wallet_connect_repository
            .get_session(&session_id)
            .await
            .map_err(|e| Status::internal(format!("Failed to get session: {}", e)))?
            .ok_or_else(|| Status::not_found("Session not found"))?;

        // Update session with connection details
        session.accounts = req.accounts.clone();

        if !req.chains.is_empty() {
            let chains: Result<Vec<WCKeyType>, Status> = req.chains.iter()
                .map(|&chain| Self::proto_to_model_key_type(chain))
                .collect();
            session.supported_chains = chains?;
        }

        session.status = crate::models::wallet_connect::SessionStatus::Active;
        session.updated_at = Utc::now();

        // Save updated session
        let updated_session = self.wallet_connect_repository
            .update_session(&session)
            .await
            .map_err(|e| Status::internal(format!("Failed to connect DApp: {}", e)))?;

        // Send notification to user
        self.send_notification(
            &session.user_id,
            NotificationType::WalletConnectSession,
            "DApp Connected",
            &format!("Successfully connected to {}", session.dapp_name),
            Some([("session_id".to_string(), session_id.to_string())].into()),
        ).await;

        // Log DApp connection
        self.audit_logger.log_action(
            &session.user_id.to_string(),
            "dapp_connected",
            &format!("Session: {}, Accounts: {:?}", session_id, req.accounts),
            true,
            None,
        ).await;

        let response = ConnectDAppResponse {
            session: Some(Self::model_to_proto_session(&updated_session)),
        };

        Ok(Response::new(response))
    }

    /// Disconnect DApp from session
    async fn disconnect_d_app(
        &self,
        request: Request<DisconnectDAppRequest>,
    ) -> Result<Response<DisconnectDAppResponse>, Status> {
        let req = request.get_ref();

        // Parse session ID
        let session_id = Uuid::parse_str(&req.session_id)
            .map_err(|_| Status::invalid_argument("Invalid session ID"))?;

        // Validate session access
        let _auth_context = self.wallet_connect_guard
            .validate_session_access(&request, &session_id)
            .await?;

        // Get existing session
        let mut session = self.wallet_connect_repository
            .get_session(&session_id)
            .await
            .map_err(|e| Status::internal(format!("Failed to get session: {}", e)))?
            .ok_or_else(|| Status::not_found("Session not found"))?;

        // Update session status
        session.status = crate::models::wallet_connect::SessionStatus::Terminated;
        session.updated_at = Utc::now();

        // Save updated session
        let _updated_session = self.wallet_connect_repository
            .update_session(&session)
            .await
            .map_err(|e| Status::internal(format!("Failed to disconnect DApp: {}", e)))?;

        // Send notification to user
        self.send_notification(
            &session.user_id,
            NotificationType::WalletConnectSession,
            "DApp Disconnected",
            &format!("Disconnected from {}", session.dapp_name),
            Some([("session_id".to_string(), session_id.to_string())].into()),
        ).await;

        // Log DApp disconnection
        self.audit_logger.log_action(
            &session.user_id.to_string(),
            "dapp_disconnected",
            &format!("Session: {}, Reason: {}", session_id, req.reason),
            true,
            None,
        ).await;

        let response = DisconnectDAppResponse { success: true };

        Ok(Response::new(response))
    }

    /// Get connected DApps for a user
    async fn get_connected_d_apps(
        &self,
        request: Request<GetConnectedDAppsRequest>,
    ) -> Result<Response<GetConnectedDAppsResponse>, Status> {
        let req = request.get_ref();

        // Extract authentication and validate analytics access
        let auth_context = self.wallet_connect_guard
            .validate_analytics_access(&request, None)
            .await?;

        // Parse user ID
        let user_id = if req.user_id.is_empty() {
            Some(Uuid::parse_str(&auth_context.user_id)
                .map_err(|_| Status::invalid_argument("Invalid user ID"))?)
        } else {
            let target_user_id = Uuid::parse_str(&req.user_id)
                .map_err(|_| Status::invalid_argument("Invalid target user ID"))?;

            if target_user_id != Uuid::parse_str(&auth_context.user_id).unwrap() &&
               !self.auth_service.has_permission(&auth_context, Permission::ManageWalletConnect).await? {
                return Err(Status::permission_denied("Can only view your own connected DApps"));
            }

            Some(target_user_id)
        };

        // Set pagination defaults
        let page_size = if req.page_size > 0 && req.page_size <= 100 {
            req.page_size
        } else {
            20
        };

        let page = if req.page_token.is_empty() {
            1
        } else {
            req.page_token.parse::<i32>()
                .map_err(|_| Status::invalid_argument("Invalid page token"))?
        };

        // Get connected DApps from repository
        let (dapps, total_count) = self.wallet_connect_repository
            .get_connected_dapps(user_id, req.active_only, page, page_size)
            .await
            .map_err(|e| Status::internal(format!("Failed to get connected DApps: {}", e)))?;

        // Convert to proto
        let proto_dapps: Vec<DAppInfo> = dapps.iter()
            .map(Self::model_to_proto_dapp_info)
            .collect();

        // Generate next page token
        let next_page_token = if (page * page_size) < total_count as i32 {
            (page + 1).to_string()
        } else {
            String::new()
        };

        let response = GetConnectedDAppsResponse {
            dapps: proto_dapps,
            next_page_token,
            total_count: total_count as i32,
        };

        Ok(Response::new(response))
    }

    /// Handle a session request
    async fn handle_session_request(
        &self,
        request: Request<HandleSessionRequestRequest>,
    ) -> Result<Response<HandleSessionRequestResponse>, Status> {
        let req = request.get_ref();

        // Parse session ID
        let session_id = Uuid::parse_str(&req.session_id)
            .map_err(|_| Status::invalid_argument("Invalid session ID"))?;

        // Convert request type
        let request_type = Self::proto_to_model_request_type(req.request_type)?;
        let chain_type = Self::proto_to_model_key_type(req.chain_type)?;

        // Validate session request
        let auth_context = self.wallet_connect_guard
            .validate_session_request(&request, &session_id, request_type, &req.method, &req.params)
            .await?;

        // Parse user ID
        let user_id = Uuid::parse_str(&auth_context.user_id)
            .map_err(|_| Status::invalid_argument("Invalid user ID"))?;

        // Set default expiration if not provided (5 minutes)
        let expires_in_seconds = if req.expires_in_seconds > 0 {
            req.expires_in_seconds
        } else {
            300 // 5 minutes
        };

        // Create new session request
        let session_request = SessionRequest::new(
            session_id,
            user_id,
            request_type,
            req.method.clone(),
            req.params.clone(),
            chain_type,
            req.chain_id.clone(),
            expires_in_seconds,
        );

        // Save request to repository
        let created_request = self.wallet_connect_repository
            .create_request(&session_request)
            .await
            .map_err(|e| Status::internal(format!("Failed to create session request: {}", e)))?;

        // Send notification to user
        self.send_notification(
            &user_id,
            NotificationType::WalletConnectRequest,
            "DApp Request",
            &format!("New {} request from DApp", req.method),
            Some([
                ("session_id".to_string(), session_id.to_string()),
                ("request_id".to_string(), created_request.request_id.to_string()),
                ("method".to_string(), req.method.clone()),
            ].into()),
        ).await;

        // Log session request
        self.audit_logger.log_action(
            &user_id.to_string(),
            "session_request_created",
            &format!("Session: {}, Method: {}, Type: {:?}", session_id, req.method, request_type),
            true,
            None,
        ).await;

        let response = HandleSessionRequestResponse {
            request: Some(Self::model_to_proto_session_request(&created_request)),
        };

        Ok(Response::new(response))
    }

    /// Approve a session request
    async fn approve_request(
        &self,
        request: Request<ApproveRequestRequest>,
    ) -> Result<Response<ApproveRequestResponse>, Status> {
        let req = request.get_ref();

        // Parse request ID
        let request_id = Uuid::parse_str(&req.request_id)
            .map_err(|_| Status::invalid_argument("Invalid request ID"))?;

        // Extract authentication
        let auth_context = self.auth_service.extract_auth(&request).await?;

        // Get existing request
        let mut session_request = self.wallet_connect_repository
            .get_request(&request_id)
            .await
            .map_err(|e| Status::internal(format!("Failed to get request: {}", e)))?
            .ok_or_else(|| Status::not_found("Request not found"))?;

        // Check ownership
        let user_id = Uuid::parse_str(&auth_context.user_id)
            .map_err(|_| Status::invalid_argument("Invalid user ID"))?;

        if session_request.user_id != user_id {
            return Err(Status::permission_denied("Request does not belong to user"));
        }

        // Check if request is still pending
        if session_request.status != crate::models::wallet_connect::RequestStatus::Pending {
            return Err(Status::failed_precondition("Request is not pending"));
        }

        // Check if request has expired
        if session_request.is_expired() {
            return Err(Status::failed_precondition("Request has expired"));
        }

        // Update request with approval
        session_request.status = crate::models::wallet_connect::RequestStatus::Approved;
        session_request.result = Some(req.result.clone());
        session_request.updated_at = Utc::now();

        // Update metadata
        for (key, value) in &req.metadata {
            session_request.metadata.insert(key.clone(), value.clone());
        }

        // Save updated request
        let updated_request = self.wallet_connect_repository
            .update_request(&session_request)
            .await
            .map_err(|e| Status::internal(format!("Failed to approve request: {}", e)))?;

        // Send notification to user
        self.send_notification(
            &user_id,
            NotificationType::WalletConnectRequest,
            "Request Approved",
            &format!("DApp request {} has been approved", session_request.method),
            Some([
                ("request_id".to_string(), request_id.to_string()),
                ("method".to_string(), session_request.method.clone()),
            ].into()),
        ).await;

        // Log request approval
        self.audit_logger.log_action(
            &user_id.to_string(),
            "session_request_approved",
            &format!("Request: {}, Method: {}", request_id, session_request.method),
            true,
            None,
        ).await;

        let response = ApproveRequestResponse {
            request: Some(Self::model_to_proto_session_request(&updated_request)),
        };

        Ok(Response::new(response))
    }

    /// Reject a session request
    async fn reject_request(
        &self,
        request: Request<RejectRequestRequest>,
    ) -> Result<Response<RejectRequestResponse>, Status> {
        let req = request.get_ref();

        // Parse request ID
        let request_id = Uuid::parse_str(&req.request_id)
            .map_err(|_| Status::invalid_argument("Invalid request ID"))?;

        // Extract authentication
        let auth_context = self.auth_service.extract_auth(&request).await?;

        // Get existing request
        let mut session_request = self.wallet_connect_repository
            .get_request(&request_id)
            .await
            .map_err(|e| Status::internal(format!("Failed to get request: {}", e)))?
            .ok_or_else(|| Status::not_found("Request not found"))?;

        // Check ownership
        let user_id = Uuid::parse_str(&auth_context.user_id)
            .map_err(|_| Status::invalid_argument("Invalid user ID"))?;

        if session_request.user_id != user_id {
            return Err(Status::permission_denied("Request does not belong to user"));
        }

        // Check if request is still pending
        if session_request.status != crate::models::wallet_connect::RequestStatus::Pending {
            return Err(Status::failed_precondition("Request is not pending"));
        }

        // Update request with rejection
        session_request.status = crate::models::wallet_connect::RequestStatus::Rejected;
        session_request.error_message = Some(req.error_message.clone());
        session_request.updated_at = Utc::now();

        // Update metadata
        for (key, value) in &req.metadata {
            session_request.metadata.insert(key.clone(), value.clone());
        }

        if !req.error_code.is_empty() {
            session_request.metadata.insert("error_code".to_string(), req.error_code.clone());
        }

        // Save updated request
        let updated_request = self.wallet_connect_repository
            .update_request(&session_request)
            .await
            .map_err(|e| Status::internal(format!("Failed to reject request: {}", e)))?;

        // Send notification to user
        self.send_notification(
            &user_id,
            NotificationType::WalletConnectRequest,
            "Request Rejected",
            &format!("DApp request {} has been rejected", session_request.method),
            Some([
                ("request_id".to_string(), request_id.to_string()),
                ("method".to_string(), session_request.method.clone()),
            ].into()),
        ).await;

        // Log request rejection
        self.audit_logger.log_action(
            &user_id.to_string(),
            "session_request_rejected",
            &format!("Request: {}, Method: {}, Reason: {}", request_id, session_request.method, req.error_message),
            true,
            None,
        ).await;

        let response = RejectRequestResponse {
            request: Some(Self::model_to_proto_session_request(&updated_request)),
        };

        Ok(Response::new(response))
    }

    /// Get session analytics
    async fn get_session_analytics(
        &self,
        request: Request<GetSessionAnalyticsRequest>,
    ) -> Result<Response<GetSessionAnalyticsResponse>, Status> {
        let req = request.get_ref();

        // Parse user ID and validate analytics access
        let target_user_id = if req.user_id.is_empty() {
            None
        } else {
            Some(Uuid::parse_str(&req.user_id)
                .map_err(|_| Status::invalid_argument("Invalid user ID"))?)
        };

        let _auth_context = self.wallet_connect_guard
            .validate_analytics_access(&request, target_user_id)
            .await?;

        // Parse date range
        let start_date = if req.start_date > 0 {
            Some(DateTime::from_timestamp(req.start_date, 0)
                .ok_or_else(|| Status::invalid_argument("Invalid start_date timestamp"))?)
        } else {
            None
        };

        let end_date = if req.end_date > 0 {
            Some(DateTime::from_timestamp(req.end_date, 0)
                .ok_or_else(|| Status::invalid_argument("Invalid end_date timestamp"))?)
        } else {
            None
        };

        // Get analytics from repository
        let analytics = self.wallet_connect_repository
            .get_session_analytics(target_user_id, start_date, end_date)
            .await
            .map_err(|e| Status::internal(format!("Failed to get session analytics: {}", e)))?;

        // Convert to proto
        let proto_analytics = SessionAnalytics {
            user_id: analytics.user_id.to_string(),
            total_sessions: analytics.total_sessions,
            active_sessions: analytics.active_sessions,
            total_requests: analytics.total_requests,
            approved_requests: analytics.approved_requests,
            rejected_requests: analytics.rejected_requests,
            top_dapps: analytics.top_dapps.iter()
                .map(Self::model_to_proto_dapp_info)
                .collect(),
            most_used_chains: analytics.most_used_chains.iter()
                .map(|&kt| Self::model_to_proto_key_type(kt) as i32)
                .collect(),
            request_type_counts: analytics.request_type_counts,
            average_session_duration: analytics.average_session_duration,
            last_activity_at: analytics.last_activity_at.timestamp(),
        };

        let response = GetSessionAnalyticsResponse {
            analytics: Some(proto_analytics),
        };

        Ok(Response::new(response))
    }

    /// Flag suspicious session
    async fn flag_suspicious_session(
        &self,
        request: Request<FlagSuspiciousSessionRequest>,
    ) -> Result<Response<FlagSuspiciousSessionResponse>, Status> {
        let req = request.get_ref();

        // Parse session ID
        let session_id = Uuid::parse_str(&req.session_id)
            .map_err(|_| Status::invalid_argument("Invalid session ID"))?;

        // Validate administrative access
        let _auth_context = self.wallet_connect_guard
            .validate_administrative_access(&request, "flag_suspicious_session")
            .await?;

        // Flag the session
        let investigation_id = self.wallet_connect_repository
            .flag_suspicious_session(&session_id, &req.reason, &req.evidence)
            .await
            .map_err(|e| Status::internal(format!("Failed to flag suspicious session: {}", e)))?;

        // If auto_suspend is enabled, update session status
        if req.auto_suspend {
            if let Ok(Some(mut session)) = self.wallet_connect_repository.get_session(&session_id).await {
                session.status = crate::models::wallet_connect::SessionStatus::Suspended;
                session.updated_at = Utc::now();

                let _ = self.wallet_connect_repository.update_session(&session).await;

                // Send notification to user
                self.send_notification(
                    &session.user_id,
                    NotificationType::SecurityAlert,
                    "Session Suspended",
                    &format!("Your session with {} has been suspended due to suspicious activity", session.dapp_name),
                    Some([
                        ("session_id".to_string(), session_id.to_string()),
                        ("investigation_id".to_string(), investigation_id.clone()),
                    ].into()),
                ).await;
            }
        }

        let response = FlagSuspiciousSessionResponse {
            success: true,
            investigation_id,
        };

        Ok(Response::new(response))
    }
}
