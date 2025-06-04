//! DApp signing data models and repository

use std::collections::HashMap;
use std::sync::RwLock;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use async_trait::async_trait;

/// Signature type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SignatureType {
    PersonalSign,
    TypedDataV1,
    TypedDataV3,
    TypedDataV4,
    SolanaSignMessage,
    BitcoinSignMessage,
}

/// Transaction type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransactionType {
    Transfer,
    ContractCall,
    ContractDeployment,
    TokenTransfer,
    NftTransfer,
    DefiSwap,
    DefiStake,
    DefiUnstake,
}

/// Validation status enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValidationStatus {
    Valid,
    Invalid,
    Warning,
    Blocked,
}

/// Risk level enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// Key type enumeration (matching proto)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeyType {
    Ethereum,
    Bitcoin,
    Solana,
}

/// Signature result entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignatureResult {
    pub signature: String,
    pub public_key: String,
    pub address: String,
    pub key_type: KeyType,
    pub signature_type: SignatureType,
    pub signed_at: DateTime<Utc>,
    pub transaction_hash: Option<String>, // For transaction signatures
    pub metadata: HashMap<String, String>,
}

impl SignatureResult {
    pub fn new(
        signature: String,
        public_key: String,
        address: String,
        key_type: KeyType,
        signature_type: SignatureType,
        transaction_hash: Option<String>,
    ) -> Self {
        Self {
            signature,
            public_key,
            address,
            key_type,
            signature_type,
            signed_at: Utc::now(),
            transaction_hash,
            metadata: HashMap::new(),
        }
    }
}

/// Transaction simulation result entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationResult {
    pub success: bool,
    pub error_message: Option<String>,
    pub gas_estimate: String,
    pub gas_price: String,
    pub total_fee: String,
    pub state_changes: Vec<String>, // JSON array of state changes
    pub events: Vec<String>, // JSON array of events
    pub risk_level: RiskLevel,
    pub warnings: Vec<String>,
    pub metadata: HashMap<String, String>,
}

impl SimulationResult {
    pub fn success(
        gas_estimate: String,
        gas_price: String,
        total_fee: String,
        risk_level: RiskLevel,
    ) -> Self {
        Self {
            success: true,
            error_message: None,
            gas_estimate,
            gas_price,
            total_fee,
            state_changes: Vec::new(),
            events: Vec::new(),
            risk_level,
            warnings: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    pub fn failure(error_message: String) -> Self {
        Self {
            success: false,
            error_message: Some(error_message),
            gas_estimate: "0".to_string(),
            gas_price: "0".to_string(),
            total_fee: "0".to_string(),
            state_changes: Vec::new(),
            events: Vec::new(),
            risk_level: RiskLevel::Critical,
            warnings: Vec::new(),
            metadata: HashMap::new(),
        }
    }
}

/// Transaction validation result entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub status: ValidationStatus,
    pub risk_level: RiskLevel,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
    pub is_whitelisted: bool,
    pub is_blacklisted: bool,
    pub risk_score: Decimal, // 0-100
    pub metadata: HashMap<String, String>,
}

impl ValidationResult {
    pub fn valid(risk_level: RiskLevel, risk_score: Decimal) -> Self {
        Self {
            status: ValidationStatus::Valid,
            risk_level,
            warnings: Vec::new(),
            errors: Vec::new(),
            is_whitelisted: false,
            is_blacklisted: false,
            risk_score,
            metadata: HashMap::new(),
        }
    }

    pub fn invalid(errors: Vec<String>) -> Self {
        Self {
            status: ValidationStatus::Invalid,
            risk_level: RiskLevel::Critical,
            warnings: Vec::new(),
            errors,
            is_whitelisted: false,
            is_blacklisted: false,
            risk_score: Decimal::from(100),
            metadata: HashMap::new(),
        }
    }
}

/// Signing history entry entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SigningHistoryEntry {
    pub entry_id: Uuid,
    pub user_id: Uuid,
    pub session_id: Uuid,
    pub dapp_url: String,
    pub signature_type: SignatureType,
    pub transaction_type: Option<TransactionType>,
    pub key_type: KeyType,
    pub chain_id: String,
    pub address: String,
    pub amount: Option<Decimal>, // For transactions
    pub recipient: Option<String>, // For transactions
    pub contract_address: Option<String>, // For contract interactions
    pub success: bool,
    pub error_message: Option<String>,
    pub risk_level: RiskLevel,
    pub signed_at: DateTime<Utc>,
    pub metadata: HashMap<String, String>,
}

impl SigningHistoryEntry {
    pub fn new(
        user_id: Uuid,
        session_id: Uuid,
        dapp_url: String,
        signature_type: SignatureType,
        key_type: KeyType,
        chain_id: String,
        address: String,
        success: bool,
        risk_level: RiskLevel,
    ) -> Self {
        Self {
            entry_id: Uuid::new_v4(),
            user_id,
            session_id,
            dapp_url,
            signature_type,
            transaction_type: None,
            key_type,
            chain_id,
            address,
            amount: None,
            recipient: None,
            contract_address: None,
            success,
            error_message: None,
            risk_level,
            signed_at: Utc::now(),
            metadata: HashMap::new(),
        }
    }
}

/// Signing analytics entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SigningAnalytics {
    pub user_id: Uuid,
    pub total_signatures: i32,
    pub successful_signatures: i32,
    pub failed_signatures: i32,
    pub total_transactions: i32,
    pub successful_transactions: i32,
    pub total_value_signed: Decimal, // In USD
    pub most_used_chains: Vec<KeyType>,
    pub most_used_types: Vec<TransactionType>,
    pub top_dapps: Vec<String>,
    pub signature_type_counts: HashMap<String, i32>,
    pub average_transaction_value: f64,
    pub average_risk_level: RiskLevel,
    pub last_activity_at: DateTime<Utc>,
}

/// DApp signing repository trait
#[async_trait]
pub trait DAppSigningRepository: Send + Sync {
    // Signature operations
    async fn create_signature_result(&self, result: &SignatureResult) -> Result<SignatureResult, String>;
    async fn get_signature_result(&self, signature: &str) -> Result<Option<SignatureResult>, String>;

    // Simulation operations
    async fn create_simulation_result(&self, result: &SimulationResult) -> Result<SimulationResult, String>;

    // Validation operations
    async fn create_validation_result(&self, result: &ValidationResult) -> Result<ValidationResult, String>;

    // History operations
    async fn create_history_entry(&self, entry: &SigningHistoryEntry) -> Result<SigningHistoryEntry, String>;
    async fn get_signing_history(
        &self,
        user_id: Option<Uuid>,
        session_id: Option<Uuid>,
        dapp_url: Option<String>,
        key_type: Option<KeyType>,
        transaction_type: Option<TransactionType>,
        start_date: Option<DateTime<Utc>>,
        end_date: Option<DateTime<Utc>>,
        page: i32,
        page_size: i32,
    ) -> Result<(Vec<SigningHistoryEntry>, i64), String>;

    // Analytics operations
    async fn get_signing_analytics(
        &self,
        user_id: Option<Uuid>,
        start_date: Option<DateTime<Utc>>,
        end_date: Option<DateTime<Utc>>,
    ) -> Result<SigningAnalytics, String>;

    // Security operations
    async fn flag_suspicious_activity(&self, user_id: &Uuid, session_id: Option<Uuid>, reason: &str, evidence: &str) -> Result<String, String>;

    // Transaction limits
    async fn check_transaction_limits(
        &self,
        user_id: &Uuid,
        amount: &Decimal,
        key_type: KeyType,
        chain_id: &str,
        transaction_type: TransactionType,
        time_window_hours: i64,
    ) -> Result<(bool, Decimal, Decimal, Decimal), String>; // (within_limits, daily_limit, daily_used, daily_remaining)
}

/// In-memory implementation for development and testing
#[derive(Debug, Default)]
pub struct InMemoryDAppSigningRepository {
    signature_results: RwLock<HashMap<String, SignatureResult>>, // signature -> result mapping
    simulation_results: RwLock<Vec<SimulationResult>>,
    validation_results: RwLock<Vec<ValidationResult>>,
    history_entries: RwLock<HashMap<Uuid, SigningHistoryEntry>>,
    flagged_activities: RwLock<HashMap<String, String>>, // investigation_id -> details
}

impl InMemoryDAppSigningRepository {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl DAppSigningRepository for InMemoryDAppSigningRepository {
    async fn create_signature_result(&self, result: &SignatureResult) -> Result<SignatureResult, String> {
        let mut signature_results = self.signature_results.write().unwrap();
        signature_results.insert(result.signature.clone(), result.clone());
        Ok(result.clone())
    }

    async fn get_signature_result(&self, signature: &str) -> Result<Option<SignatureResult>, String> {
        let signature_results = self.signature_results.read().unwrap();
        Ok(signature_results.get(signature).cloned())
    }

    async fn create_simulation_result(&self, result: &SimulationResult) -> Result<SimulationResult, String> {
        let mut simulation_results = self.simulation_results.write().unwrap();
        simulation_results.push(result.clone());
        Ok(result.clone())
    }

    async fn create_validation_result(&self, result: &ValidationResult) -> Result<ValidationResult, String> {
        let mut validation_results = self.validation_results.write().unwrap();
        validation_results.push(result.clone());
        Ok(result.clone())
    }

    async fn create_history_entry(&self, entry: &SigningHistoryEntry) -> Result<SigningHistoryEntry, String> {
        let mut history_entries = self.history_entries.write().unwrap();
        history_entries.insert(entry.entry_id, entry.clone());
        Ok(entry.clone())
    }

    async fn get_signing_history(
        &self,
        user_id: Option<Uuid>,
        session_id: Option<Uuid>,
        dapp_url: Option<String>,
        key_type: Option<KeyType>,
        transaction_type: Option<TransactionType>,
        start_date: Option<DateTime<Utc>>,
        end_date: Option<DateTime<Utc>>,
        page: i32,
        page_size: i32,
    ) -> Result<(Vec<SigningHistoryEntry>, i64), String> {
        let history_entries = self.history_entries.read().unwrap();
        let mut filtered_entries: Vec<SigningHistoryEntry> = history_entries
            .values()
            .filter(|entry| {
                user_id.map_or(true, |uid| entry.user_id == uid) &&
                session_id.map_or(true, |sid| entry.session_id == sid) &&
                dapp_url.as_ref().map_or(true, |url| entry.dapp_url.contains(url)) &&
                key_type.map_or(true, |kt| entry.key_type == kt) &&
                transaction_type.map_or(true, |tt| entry.transaction_type == Some(tt)) &&
                start_date.map_or(true, |date| entry.signed_at >= date) &&
                end_date.map_or(true, |date| entry.signed_at <= date)
            })
            .cloned()
            .collect();

        // Sort by signed_at descending
        filtered_entries.sort_by(|a, b| b.signed_at.cmp(&a.signed_at));

        let total_count = filtered_entries.len() as i64;
        let start = ((page - 1) * page_size) as usize;
        let end = (start + page_size as usize).min(filtered_entries.len());

        let paginated_entries = if start < filtered_entries.len() {
            filtered_entries[start..end].to_vec()
        } else {
            Vec::new()
        };

        Ok((paginated_entries, total_count))
    }

    async fn get_signing_analytics(
        &self,
        user_id: Option<Uuid>,
        start_date: Option<DateTime<Utc>>,
        end_date: Option<DateTime<Utc>>,
    ) -> Result<SigningAnalytics, String> {
        let history_entries = self.history_entries.read().unwrap();

        let filtered_entries: Vec<&SigningHistoryEntry> = history_entries
            .values()
            .filter(|entry| {
                user_id.map_or(true, |uid| entry.user_id == uid) &&
                start_date.map_or(true, |date| entry.signed_at >= date) &&
                end_date.map_or(true, |date| entry.signed_at <= date)
            })
            .collect();

        let total_signatures = filtered_entries.len() as i32;
        let successful_signatures = filtered_entries.iter().filter(|e| e.success).count() as i32;
        let failed_signatures = total_signatures - successful_signatures;

        let transaction_entries: Vec<&SigningHistoryEntry> = filtered_entries
            .iter()
            .filter(|e| e.transaction_type.is_some())
            .cloned()
            .collect();

        let total_transactions = transaction_entries.len() as i32;
        let successful_transactions = transaction_entries.iter().filter(|e| e.success).count() as i32;

        // Calculate total value signed (sum of amounts)
        let total_value_signed: Decimal = transaction_entries
            .iter()
            .filter_map(|e| e.amount)
            .sum();

        // Get most used chains
        let mut chain_counts: HashMap<KeyType, i32> = HashMap::new();
        for entry in &filtered_entries {
            *chain_counts.entry(entry.key_type).or_insert(0) += 1;
        }
        let mut most_used_chains: Vec<KeyType> = chain_counts
            .into_iter()
            .map(|(chain, _count)| chain)
            .collect();
        most_used_chains.sort_by_key(|chain| std::cmp::Reverse(chain_counts.get(chain).unwrap_or(&0)));

        // Get most used transaction types
        let mut type_counts: HashMap<TransactionType, i32> = HashMap::new();
        for entry in &transaction_entries {
            if let Some(tx_type) = entry.transaction_type {
                *type_counts.entry(tx_type).or_insert(0) += 1;
            }
        }
        let mut most_used_types: Vec<TransactionType> = type_counts
            .into_iter()
            .map(|(tx_type, _count)| tx_type)
            .collect();
        most_used_types.sort_by_key(|tx_type| std::cmp::Reverse(type_counts.get(tx_type).unwrap_or(&0)));

        // Get top DApps
        let mut dapp_counts: HashMap<String, i32> = HashMap::new();
        for entry in &filtered_entries {
            *dapp_counts.entry(entry.dapp_url.clone()).or_insert(0) += 1;
        }
        let mut top_dapps: Vec<String> = dapp_counts
            .into_iter()
            .map(|(dapp, _count)| dapp)
            .collect();
        top_dapps.sort_by_key(|dapp| std::cmp::Reverse(dapp_counts.get(dapp).unwrap_or(&0)));
        top_dapps.truncate(10); // Top 10

        // Get signature type counts
        let mut signature_type_counts: HashMap<String, i32> = HashMap::new();
        for entry in &filtered_entries {
            let type_name = format!("{:?}", entry.signature_type);
            *signature_type_counts.entry(type_name).or_insert(0) += 1;
        }

        // Calculate average transaction value
        let average_transaction_value = if total_transactions > 0 {
            total_value_signed.to_f64().unwrap_or(0.0) / total_transactions as f64
        } else {
            0.0
        };

        // Calculate average risk level (simplified)
        let risk_sum: i32 = filtered_entries
            .iter()
            .map(|e| match e.risk_level {
                RiskLevel::Low => 1,
                RiskLevel::Medium => 2,
                RiskLevel::High => 3,
                RiskLevel::Critical => 4,
            })
            .sum();
        let average_risk_level = if total_signatures > 0 {
            match risk_sum / total_signatures {
                1 => RiskLevel::Low,
                2 => RiskLevel::Medium,
                3 => RiskLevel::High,
                _ => RiskLevel::Critical,
            }
        } else {
            RiskLevel::Low
        };

        let last_activity_at = filtered_entries
            .iter()
            .map(|e| e.signed_at)
            .max()
            .unwrap_or_else(Utc::now);

        Ok(SigningAnalytics {
            user_id: user_id.unwrap_or_default(),
            total_signatures,
            successful_signatures,
            failed_signatures,
            total_transactions,
            successful_transactions,
            total_value_signed,
            most_used_chains,
            most_used_types,
            top_dapps,
            signature_type_counts,
            average_transaction_value,
            average_risk_level,
            last_activity_at,
        })
    }

    async fn flag_suspicious_activity(&self, user_id: &Uuid, session_id: Option<Uuid>, reason: &str, evidence: &str) -> Result<String, String> {
        let investigation_id = format!("inv_{}", Uuid::new_v4());
        let mut flagged_activities = self.flagged_activities.write().unwrap();

        let details = serde_json::json!({
            "user_id": user_id,
            "session_id": session_id,
            "reason": reason,
            "evidence": evidence,
            "flagged_at": Utc::now().to_rfc3339(),
        }).to_string();

        flagged_activities.insert(investigation_id.clone(), details);
        Ok(investigation_id)
    }

    async fn check_transaction_limits(
        &self,
        user_id: &Uuid,
        amount: &Decimal,
        key_type: KeyType,
        chain_id: &str,
        transaction_type: TransactionType,
        time_window_hours: i64,
    ) -> Result<(bool, Decimal, Decimal, Decimal), String> {
        let history_entries = self.history_entries.read().unwrap();
        let cutoff_time = Utc::now() - chrono::Duration::hours(time_window_hours);

        // Get recent transactions for this user, chain, and type
        let recent_transactions: Vec<&SigningHistoryEntry> = history_entries
            .values()
            .filter(|entry| {
                entry.user_id == *user_id &&
                entry.key_type == key_type &&
                entry.chain_id == chain_id &&
                entry.transaction_type == Some(transaction_type) &&
                entry.signed_at >= cutoff_time &&
                entry.success
            })
            .collect();

        // Calculate daily used amount
        let daily_used: Decimal = recent_transactions
            .iter()
            .filter_map(|e| e.amount)
            .sum();

        // Set limits based on transaction type and risk level (simplified)
        let daily_limit = match transaction_type {
            TransactionType::Transfer => Decimal::from(10000), // $10,000
            TransactionType::TokenTransfer => Decimal::from(5000), // $5,000
            TransactionType::DefiSwap => Decimal::from(20000), // $20,000
            TransactionType::DefiStake => Decimal::from(50000), // $50,000
            _ => Decimal::from(1000), // $1,000 for others
        };

        let transaction_limit = match transaction_type {
            TransactionType::Transfer => Decimal::from(5000), // $5,000 per transaction
            TransactionType::TokenTransfer => Decimal::from(2500), // $2,500 per transaction
            TransactionType::DefiSwap => Decimal::from(10000), // $10,000 per transaction
            TransactionType::DefiStake => Decimal::from(25000), // $25,000 per transaction
            _ => Decimal::from(500), // $500 for others
        };

        let daily_remaining = daily_limit - daily_used;
        let within_limits = daily_remaining >= *amount && *amount <= transaction_limit;

        Ok((within_limits, daily_limit, daily_used, daily_remaining))
    }
}
