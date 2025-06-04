//! WalletConnect data models and repository

use std::collections::HashMap;
use std::sync::RwLock;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use async_trait::async_trait;

/// Session status enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionStatus {
    Pending,
    Active,
    Expired,
    Terminated,
    Suspended,
}

impl Default for SessionStatus {
    fn default() -> Self {
        Self::Pending
    }
}

/// Request type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RequestType {
    SignMessage,
    SignTransaction,
    SendTransaction,
    SwitchChain,
    AddChain,
    WatchAsset,
}

/// Request status enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RequestStatus {
    Pending,
    Approved,
    Rejected,
    Expired,
}

impl Default for RequestStatus {
    fn default() -> Self {
        Self::Pending
    }
}

/// Key type enumeration (matching proto)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeyType {
    Ethereum,
    Bitcoin,
    Solana,
}

/// WalletConnect session entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletConnectSession {
    pub session_id: Uuid,
    pub user_id: Uuid,
    pub dapp_url: String,
    pub dapp_name: String,
    pub dapp_description: String,
    pub dapp_icons: Vec<String>,
    pub supported_chains: Vec<KeyType>,
    pub accounts: Vec<String>,
    pub status: SessionStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub bridge_url: String,
    pub key: String,
    pub peer_id: String,
    pub metadata: HashMap<String, String>,
}

impl WalletConnectSession {
    pub fn new(
        user_id: Uuid,
        dapp_url: String,
        dapp_name: String,
        dapp_description: String,
        dapp_icons: Vec<String>,
        supported_chains: Vec<KeyType>,
        bridge_url: String,
        expires_in_seconds: i64,
    ) -> Self {
        let now = Utc::now();
        let expires_at = now + chrono::Duration::seconds(expires_in_seconds);
        
        Self {
            session_id: Uuid::new_v4(),
            user_id,
            dapp_url,
            dapp_name,
            dapp_description,
            dapp_icons,
            supported_chains,
            accounts: Vec::new(),
            status: SessionStatus::Pending,
            created_at: now,
            updated_at: now,
            expires_at,
            bridge_url,
            key: format!("wc_{}", Uuid::new_v4()),
            peer_id: format!("peer_{}", Uuid::new_v4()),
            metadata: HashMap::new(),
        }
    }

    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    pub fn is_active(&self) -> bool {
        self.status == SessionStatus::Active && !self.is_expired()
    }
}

/// DApp information entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DAppInfo {
    pub url: String,
    pub name: String,
    pub description: String,
    pub icons: Vec<String>,
    pub version: String,
    pub supported_chains: Vec<KeyType>,
    pub metadata: HashMap<String, String>,
    pub first_connected_at: DateTime<Utc>,
    pub last_connected_at: DateTime<Utc>,
    pub connection_count: i32,
    pub is_trusted: bool,
    pub is_flagged: bool,
}

/// Session request entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRequest {
    pub request_id: Uuid,
    pub session_id: Uuid,
    pub user_id: Uuid,
    pub request_type: RequestType,
    pub status: RequestStatus,
    pub method: String,
    pub params: String, // JSON string
    pub result: Option<String>, // JSON string
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub chain_type: KeyType,
    pub chain_id: String,
    pub metadata: HashMap<String, String>,
}

impl SessionRequest {
    pub fn new(
        session_id: Uuid,
        user_id: Uuid,
        request_type: RequestType,
        method: String,
        params: String,
        chain_type: KeyType,
        chain_id: String,
        expires_in_seconds: i64,
    ) -> Self {
        let now = Utc::now();
        let expires_at = now + chrono::Duration::seconds(expires_in_seconds);
        
        Self {
            request_id: Uuid::new_v4(),
            session_id,
            user_id,
            request_type,
            status: RequestStatus::Pending,
            method,
            params,
            result: None,
            error_message: None,
            created_at: now,
            updated_at: now,
            expires_at,
            chain_type,
            chain_id,
            metadata: HashMap::new(),
        }
    }

    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }
}

/// Session analytics entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionAnalytics {
    pub user_id: Uuid,
    pub total_sessions: i32,
    pub active_sessions: i32,
    pub total_requests: i32,
    pub approved_requests: i32,
    pub rejected_requests: i32,
    pub top_dapps: Vec<DAppInfo>,
    pub most_used_chains: Vec<KeyType>,
    pub request_type_counts: HashMap<String, i32>,
    pub average_session_duration: f64,
    pub last_activity_at: DateTime<Utc>,
}

/// WalletConnect repository trait
#[async_trait]
pub trait WalletConnectRepository: Send + Sync {
    // Session operations
    async fn create_session(&self, session: &WalletConnectSession) -> Result<WalletConnectSession, String>;
    async fn get_session(&self, session_id: &Uuid) -> Result<Option<WalletConnectSession>, String>;
    async fn list_sessions(
        &self,
        user_id: Option<Uuid>,
        status: Option<SessionStatus>,
        dapp_url: Option<String>,
        chain_type: Option<KeyType>,
        created_after: Option<DateTime<Utc>>,
        created_before: Option<DateTime<Utc>>,
        page: i32,
        page_size: i32,
    ) -> Result<(Vec<WalletConnectSession>, i64), String>;
    async fn update_session(&self, session: &WalletConnectSession) -> Result<WalletConnectSession, String>;
    async fn delete_session(&self, session_id: &Uuid) -> Result<bool, String>;

    // DApp operations
    async fn get_connected_dapps(
        &self,
        user_id: Option<Uuid>,
        active_only: bool,
        page: i32,
        page_size: i32,
    ) -> Result<(Vec<DAppInfo>, i64), String>;

    // Request operations
    async fn create_request(&self, request: &SessionRequest) -> Result<SessionRequest, String>;
    async fn get_request(&self, request_id: &Uuid) -> Result<Option<SessionRequest>, String>;
    async fn update_request(&self, request: &SessionRequest) -> Result<SessionRequest, String>;
    async fn list_requests(
        &self,
        session_id: Option<Uuid>,
        user_id: Option<Uuid>,
        status: Option<RequestStatus>,
        request_type: Option<RequestType>,
        page: i32,
        page_size: i32,
    ) -> Result<(Vec<SessionRequest>, i64), String>;

    // Analytics operations
    async fn get_session_analytics(
        &self,
        user_id: Option<Uuid>,
        start_date: Option<DateTime<Utc>>,
        end_date: Option<DateTime<Utc>>,
    ) -> Result<SessionAnalytics, String>;

    // Security operations
    async fn flag_suspicious_session(&self, session_id: &Uuid, reason: &str, evidence: &str) -> Result<String, String>;
}

/// In-memory implementation for development and testing
#[derive(Debug, Default)]
pub struct InMemoryWalletConnectRepository {
    sessions: RwLock<HashMap<Uuid, WalletConnectSession>>,
    requests: RwLock<HashMap<Uuid, SessionRequest>>,
    dapp_info: RwLock<HashMap<String, DAppInfo>>, // url -> info mapping
}

impl InMemoryWalletConnectRepository {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl WalletConnectRepository for InMemoryWalletConnectRepository {
    async fn create_session(&self, session: &WalletConnectSession) -> Result<WalletConnectSession, String> {
        let mut sessions = self.sessions.write().unwrap();
        sessions.insert(session.session_id, session.clone());

        // Update DApp info
        let mut dapp_info = self.dapp_info.write().unwrap();
        let info = dapp_info.entry(session.dapp_url.clone()).or_insert_with(|| DAppInfo {
            url: session.dapp_url.clone(),
            name: session.dapp_name.clone(),
            description: session.dapp_description.clone(),
            icons: session.dapp_icons.clone(),
            version: "1.0.0".to_string(),
            supported_chains: session.supported_chains.clone(),
            metadata: session.metadata.clone(),
            first_connected_at: session.created_at,
            last_connected_at: session.created_at,
            connection_count: 0,
            is_trusted: false,
            is_flagged: false,
        });
        info.last_connected_at = session.created_at;
        info.connection_count += 1;

        Ok(session.clone())
    }

    async fn get_session(&self, session_id: &Uuid) -> Result<Option<WalletConnectSession>, String> {
        let sessions = self.sessions.read().unwrap();
        Ok(sessions.get(session_id).cloned())
    }

    async fn list_sessions(
        &self,
        user_id: Option<Uuid>,
        status: Option<SessionStatus>,
        dapp_url: Option<String>,
        chain_type: Option<KeyType>,
        created_after: Option<DateTime<Utc>>,
        created_before: Option<DateTime<Utc>>,
        page: i32,
        page_size: i32,
    ) -> Result<(Vec<WalletConnectSession>, i64), String> {
        let sessions = self.sessions.read().unwrap();
        let mut filtered_sessions: Vec<WalletConnectSession> = sessions
            .values()
            .filter(|session| {
                user_id.map_or(true, |uid| session.user_id == uid) &&
                status.map_or(true, |s| session.status == s) &&
                dapp_url.as_ref().map_or(true, |url| session.dapp_url.contains(url)) &&
                chain_type.map_or(true, |ct| session.supported_chains.contains(&ct)) &&
                created_after.map_or(true, |date| session.created_at >= date) &&
                created_before.map_or(true, |date| session.created_at <= date)
            })
            .cloned()
            .collect();

        // Sort by created_at descending
        filtered_sessions.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        let total_count = filtered_sessions.len() as i64;
        let start = ((page - 1) * page_size) as usize;
        let end = (start + page_size as usize).min(filtered_sessions.len());

        let paginated_sessions = if start < filtered_sessions.len() {
            filtered_sessions[start..end].to_vec()
        } else {
            Vec::new()
        };

        Ok((paginated_sessions, total_count))
    }

    async fn update_session(&self, session: &WalletConnectSession) -> Result<WalletConnectSession, String> {
        let mut sessions = self.sessions.write().unwrap();
        sessions.insert(session.session_id, session.clone());
        Ok(session.clone())
    }

    async fn delete_session(&self, session_id: &Uuid) -> Result<bool, String> {
        let mut sessions = self.sessions.write().unwrap();
        Ok(sessions.remove(session_id).is_some())
    }

    async fn get_connected_dapps(
        &self,
        user_id: Option<Uuid>,
        active_only: bool,
        page: i32,
        page_size: i32,
    ) -> Result<(Vec<DAppInfo>, i64), String> {
        let sessions = self.sessions.read().unwrap();
        let dapp_info = self.dapp_info.read().unwrap();

        // Get unique DApp URLs for the user
        let mut dapp_urls: std::collections::HashSet<String> = sessions
            .values()
            .filter(|session| {
                user_id.map_or(true, |uid| session.user_id == uid) &&
                (!active_only || session.is_active())
            })
            .map(|session| session.dapp_url.clone())
            .collect();

        let mut filtered_dapps: Vec<DAppInfo> = dapp_urls
            .into_iter()
            .filter_map(|url| dapp_info.get(&url).cloned())
            .collect();

        // Sort by last_connected_at descending
        filtered_dapps.sort_by(|a, b| b.last_connected_at.cmp(&a.last_connected_at));

        let total_count = filtered_dapps.len() as i64;
        let start = ((page - 1) * page_size) as usize;
        let end = (start + page_size as usize).min(filtered_dapps.len());

        let paginated_dapps = if start < filtered_dapps.len() {
            filtered_dapps[start..end].to_vec()
        } else {
            Vec::new()
        };

        Ok((paginated_dapps, total_count))
    }

    async fn create_request(&self, request: &SessionRequest) -> Result<SessionRequest, String> {
        let mut requests = self.requests.write().unwrap();
        requests.insert(request.request_id, request.clone());
        Ok(request.clone())
    }

    async fn get_request(&self, request_id: &Uuid) -> Result<Option<SessionRequest>, String> {
        let requests = self.requests.read().unwrap();
        Ok(requests.get(request_id).cloned())
    }

    async fn update_request(&self, request: &SessionRequest) -> Result<SessionRequest, String> {
        let mut requests = self.requests.write().unwrap();
        requests.insert(request.request_id, request.clone());
        Ok(request.clone())
    }

    async fn list_requests(
        &self,
        session_id: Option<Uuid>,
        user_id: Option<Uuid>,
        status: Option<RequestStatus>,
        request_type: Option<RequestType>,
        page: i32,
        page_size: i32,
    ) -> Result<(Vec<SessionRequest>, i64), String> {
        let requests = self.requests.read().unwrap();
        let mut filtered_requests: Vec<SessionRequest> = requests
            .values()
            .filter(|request| {
                session_id.map_or(true, |sid| request.session_id == sid) &&
                user_id.map_or(true, |uid| request.user_id == uid) &&
                status.map_or(true, |s| request.status == s) &&
                request_type.map_or(true, |rt| request.request_type == rt)
            })
            .cloned()
            .collect();

        // Sort by created_at descending
        filtered_requests.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        let total_count = filtered_requests.len() as i64;
        let start = ((page - 1) * page_size) as usize;
        let end = (start + page_size as usize).min(filtered_requests.len());

        let paginated_requests = if start < filtered_requests.len() {
            filtered_requests[start..end].to_vec()
        } else {
            Vec::new()
        };

        Ok((paginated_requests, total_count))
    }

    async fn get_session_analytics(
        &self,
        user_id: Option<Uuid>,
        start_date: Option<DateTime<Utc>>,
        end_date: Option<DateTime<Utc>>,
    ) -> Result<SessionAnalytics, String> {
        let sessions = self.sessions.read().unwrap();
        let requests = self.requests.read().unwrap();
        let dapp_info = self.dapp_info.read().unwrap();

        let filtered_sessions: Vec<&WalletConnectSession> = sessions
            .values()
            .filter(|session| {
                user_id.map_or(true, |uid| session.user_id == uid) &&
                start_date.map_or(true, |date| session.created_at >= date) &&
                end_date.map_or(true, |date| session.created_at <= date)
            })
            .collect();

        let filtered_requests: Vec<&SessionRequest> = requests
            .values()
            .filter(|request| {
                user_id.map_or(true, |uid| request.user_id == uid) &&
                start_date.map_or(true, |date| request.created_at >= date) &&
                end_date.map_or(true, |date| request.created_at <= date)
            })
            .collect();

        let total_sessions = filtered_sessions.len() as i32;
        let active_sessions = filtered_sessions.iter().filter(|s| s.is_active()).count() as i32;
        let total_requests = filtered_requests.len() as i32;
        let approved_requests = filtered_requests.iter().filter(|r| r.status == RequestStatus::Approved).count() as i32;
        let rejected_requests = filtered_requests.iter().filter(|r| r.status == RequestStatus::Rejected).count() as i32;

        // Calculate average session duration
        let total_duration: i64 = filtered_sessions
            .iter()
            .map(|session| {
                let end_time = if session.status == SessionStatus::Active {
                    Utc::now()
                } else {
                    session.updated_at
                };
                (end_time - session.created_at).num_seconds()
            })
            .sum();
        let average_session_duration = if total_sessions > 0 {
            total_duration as f64 / total_sessions as f64
        } else {
            0.0
        };

        // Get top DApps
        let mut dapp_counts: HashMap<String, i32> = HashMap::new();
        for session in &filtered_sessions {
            *dapp_counts.entry(session.dapp_url.clone()).or_insert(0) += 1;
        }
        let mut top_dapps: Vec<DAppInfo> = dapp_counts
            .into_iter()
            .filter_map(|(url, _count)| dapp_info.get(&url).cloned())
            .collect();
        top_dapps.sort_by(|a, b| b.connection_count.cmp(&a.connection_count));
        top_dapps.truncate(10); // Top 10

        // Get most used chains
        let mut chain_counts: HashMap<KeyType, i32> = HashMap::new();
        for session in &filtered_sessions {
            for chain in &session.supported_chains {
                *chain_counts.entry(*chain).or_insert(0) += 1;
            }
        }
        let mut most_used_chains: Vec<KeyType> = chain_counts
            .into_iter()
            .map(|(chain, _count)| chain)
            .collect();
        most_used_chains.sort_by_key(|chain| std::cmp::Reverse(chain_counts.get(chain).unwrap_or(&0)));

        // Get request type counts
        let mut request_type_counts: HashMap<String, i32> = HashMap::new();
        for request in &filtered_requests {
            let type_name = format!("{:?}", request.request_type);
            *request_type_counts.entry(type_name).or_insert(0) += 1;
        }

        let last_activity_at = filtered_sessions
            .iter()
            .map(|s| s.updated_at)
            .max()
            .unwrap_or_else(Utc::now);

        Ok(SessionAnalytics {
            user_id: user_id.unwrap_or_default(),
            total_sessions,
            active_sessions,
            total_requests,
            approved_requests,
            rejected_requests,
            top_dapps,
            most_used_chains,
            request_type_counts,
            average_session_duration,
            last_activity_at,
        })
    }

    async fn flag_suspicious_session(&self, session_id: &Uuid, reason: &str, evidence: &str) -> Result<String, String> {
        let mut sessions = self.sessions.write().unwrap();
        if let Some(session) = sessions.get_mut(session_id) {
            session.status = SessionStatus::Suspended;
            session.metadata.insert("flagged_reason".to_string(), reason.to_string());
            session.metadata.insert("flagged_evidence".to_string(), evidence.to_string());
            session.metadata.insert("flagged_at".to_string(), Utc::now().to_rfc3339());

            let investigation_id = format!("inv_{}", Uuid::new_v4());
            session.metadata.insert("investigation_id".to_string(), investigation_id.clone());

            Ok(investigation_id)
        } else {
            Err("Session not found".to_string())
        }
    }
}
