//! Card funding data models

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use rust_decimal::Decimal;
use chrono::{DateTime, Utc};

/// Funding source types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FundingSourceType {
    BankAccount,
    CryptoWallet,
    ACH,
    ExternalCard,
    FiatAccount,
}

impl std::fmt::Display for FundingSourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FundingSourceType::BankAccount => write!(f, "bank_account"),
            FundingSourceType::CryptoWallet => write!(f, "crypto_wallet"),
            FundingSourceType::ACH => write!(f, "ach"),
            FundingSourceType::ExternalCard => write!(f, "external_card"),
            FundingSourceType::FiatAccount => write!(f, "fiat_account"),
        }
    }
}

impl std::str::FromStr for FundingSourceType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "bank_account" => Ok(FundingSourceType::BankAccount),
            "crypto_wallet" => Ok(FundingSourceType::CryptoWallet),
            "ach" => Ok(FundingSourceType::ACH),
            "external_card" => Ok(FundingSourceType::ExternalCard),
            "fiat_account" => Ok(FundingSourceType::FiatAccount),
            _ => Err(format!("Invalid funding source type: {}", s)),
        }
    }
}

/// Funding source status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FundingSourceStatus {
    Pending,
    Active,
    Suspended,
    Expired,
    Removed,
}

impl std::fmt::Display for FundingSourceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FundingSourceStatus::Pending => write!(f, "pending"),
            FundingSourceStatus::Active => write!(f, "active"),
            FundingSourceStatus::Suspended => write!(f, "suspended"),
            FundingSourceStatus::Expired => write!(f, "expired"),
            FundingSourceStatus::Removed => write!(f, "removed"),
        }
    }
}

impl std::str::FromStr for FundingSourceStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pending" => Ok(FundingSourceStatus::Pending),
            "active" => Ok(FundingSourceStatus::Active),
            "suspended" => Ok(FundingSourceStatus::Suspended),
            "expired" => Ok(FundingSourceStatus::Expired),
            "removed" => Ok(FundingSourceStatus::Removed),
            _ => Err(format!("Invalid funding source status: {}", s)),
        }
    }
}

/// Funding transaction status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FundingTransactionStatus {
    Pending,
    Processing,
    Completed,
    Failed,
    Cancelled,
    Refunded,
}

impl std::fmt::Display for FundingTransactionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FundingTransactionStatus::Pending => write!(f, "pending"),
            FundingTransactionStatus::Processing => write!(f, "processing"),
            FundingTransactionStatus::Completed => write!(f, "completed"),
            FundingTransactionStatus::Failed => write!(f, "failed"),
            FundingTransactionStatus::Cancelled => write!(f, "cancelled"),
            FundingTransactionStatus::Refunded => write!(f, "refunded"),
        }
    }
}

impl std::str::FromStr for FundingTransactionStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pending" => Ok(FundingTransactionStatus::Pending),
            "processing" => Ok(FundingTransactionStatus::Processing),
            "completed" => Ok(FundingTransactionStatus::Completed),
            "failed" => Ok(FundingTransactionStatus::Failed),
            "cancelled" => Ok(FundingTransactionStatus::Cancelled),
            "refunded" => Ok(FundingTransactionStatus::Refunded),
            _ => Err(format!("Invalid funding transaction status: {}", s)),
        }
    }
}

/// Crypto currency types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CryptoCurrency {
    USDT,
    USDC,
    DAI,
    BUSD,
}

impl std::fmt::Display for CryptoCurrency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CryptoCurrency::USDT => write!(f, "USDT"),
            CryptoCurrency::USDC => write!(f, "USDC"),
            CryptoCurrency::DAI => write!(f, "DAI"),
            CryptoCurrency::BUSD => write!(f, "BUSD"),
        }
    }
}

/// Funding source limits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundingSourceLimits {
    pub daily_limit: Decimal,
    pub monthly_limit: Decimal,
    pub per_transaction_limit: Decimal,
    pub minimum_amount: Decimal,
    pub daily_transaction_count: i32,
    pub monthly_transaction_count: i32,
}

impl Default for FundingSourceLimits {
    fn default() -> Self {
        Self {
            daily_limit: Decimal::from(10000),
            monthly_limit: Decimal::from(100000),
            per_transaction_limit: Decimal::from(5000),
            minimum_amount: Decimal::from(10),
            daily_transaction_count: 20,
            monthly_transaction_count: 200,
        }
    }
}

/// Type-specific metadata for funding sources
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum FundingSourceMetadata {
    BankAccount {
        account_type: String,
        routing_number: String,
        bank_name: String,
    },
    CryptoWallet {
        currency: CryptoCurrency,
        network: String,
        wallet_address: String,
        exchange_name: Option<String>,
    },
    ExternalCard {
        card_type: String,
        issuer: String,
        last_four: String,
        expiry_month: String,
        expiry_year: String,
    },
    ACH {
        ach_type: String,
        processor: String,
    },
    FiatAccount {
        account_id: Uuid,
        account_type: String,
    },
}

/// Funding source entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundingSource {
    pub id: Uuid,
    pub user_id: Uuid,
    pub source_type: FundingSourceType,
    pub status: FundingSourceStatus,
    pub name: String,
    pub masked_identifier: String,
    pub currency: String,
    pub provider: String,
    pub limits: FundingSourceLimits,
    pub metadata: FundingSourceMetadata,
    pub is_primary: bool,
    pub is_verified: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub verification_url: Option<String>,
    pub external_id: Option<String>,
}

/// Funding transaction entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundingTransaction {
    pub id: Uuid,
    pub user_id: Uuid,
    pub card_id: Uuid,
    pub funding_source_id: Uuid,
    pub status: FundingTransactionStatus,
    pub amount: Decimal,
    pub currency: String,
    pub fee_amount: Decimal,
    pub fee_percentage: Decimal,
    pub exchange_rate: Option<Decimal>,
    pub net_amount: Decimal,
    pub reference_number: String,
    pub external_transaction_id: Option<String>,
    pub description: Option<String>,
    pub failure_reason: Option<String>,
    pub metadata: HashMap<String, String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
}

/// Fee calculation details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeCalculation {
    pub base_amount: Decimal,
    pub fee_percentage: Decimal,
    pub fee_amount: Decimal,
    pub net_amount: Decimal,
    pub exchange_rate: Option<Decimal>,
    pub exchange_fee: Option<Decimal>,
    pub total_fee: Decimal,
    pub fee_breakdown: Vec<FeeBreakdown>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeBreakdown {
    pub fee_type: String,
    pub amount: Decimal,
    pub description: String,
}

/// User funding limits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundingLimits {
    pub id: Uuid,
    pub user_id: Uuid,
    pub daily_limit: Decimal,
    pub monthly_limit: Decimal,
    pub yearly_limit: Decimal,
    pub per_transaction_limit: Decimal,
    pub daily_used: Decimal,
    pub monthly_used: Decimal,
    pub yearly_used: Decimal,
    pub daily_transaction_count: i32,
    pub monthly_transaction_count: i32,
    pub daily_transactions_used: i32,
    pub monthly_transactions_used: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_reset_daily: DateTime<Utc>,
    pub last_reset_monthly: DateTime<Utc>,
    pub last_reset_yearly: DateTime<Utc>,
}

impl Default for FundingLimits {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            daily_limit: Decimal::from(25000),
            monthly_limit: Decimal::from(250000),
            yearly_limit: Decimal::from(1000000),
            per_transaction_limit: Decimal::from(10000),
            daily_used: Decimal::ZERO,
            monthly_used: Decimal::ZERO,
            yearly_used: Decimal::ZERO,
            daily_transaction_count: 50,
            monthly_transaction_count: 500,
            daily_transactions_used: 0,
            monthly_transactions_used: 0,
            created_at: now,
            updated_at: now,
            last_reset_daily: now,
            last_reset_monthly: now,
            last_reset_yearly: now,
        }
    }
}

/// Crypto funding details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CryptoFundingDetails {
    pub currency: CryptoCurrency,
    pub network: String,
    pub deposit_address: String,
    pub required_confirmations: u32,
    pub current_confirmations: u32,
    pub transaction_hash: Option<String>,
    pub exchange_rate: Decimal,
    pub expires_at: DateTime<Utc>,
}

/// Funding metrics for analytics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundingMetrics {
    pub total_volume: Decimal,
    pub total_fees: Decimal,
    pub total_transactions: i64,
    pub average_transaction_size: Decimal,
    pub by_source: Vec<FundingSourceMetrics>,
    pub by_currency: Vec<CurrencyMetrics>,
    pub success_rate: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundingSourceMetrics {
    pub source_type: FundingSourceType,
    pub volume: Decimal,
    pub fees: Decimal,
    pub transaction_count: i64,
    pub success_rate: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrencyMetrics {
    pub currency: String,
    pub volume: Decimal,
    pub fees: Decimal,
    pub transaction_count: i64,
}

/// Repository trait for card funding operations
#[async_trait::async_trait]
pub trait CardFundingRepository: Send + Sync {
    // Funding source operations
    async fn create_funding_source(&self, source: &FundingSource) -> Result<FundingSource, String>;
    async fn get_funding_source(&self, id: &Uuid) -> Result<Option<FundingSource>, String>;
    async fn get_funding_source_by_user(&self, user_id: &Uuid, source_id: &Uuid) -> Result<Option<FundingSource>, String>;
    async fn list_funding_sources(&self, user_id: &Uuid, source_type: Option<FundingSourceType>, status: Option<FundingSourceStatus>, page: i32, page_size: i32) -> Result<(Vec<FundingSource>, i64), String>;
    async fn update_funding_source(&self, source: &FundingSource) -> Result<FundingSource, String>;
    async fn delete_funding_source(&self, id: &Uuid) -> Result<bool, String>;

    // Funding transaction operations
    async fn create_funding_transaction(&self, transaction: &FundingTransaction) -> Result<FundingTransaction, String>;
    async fn get_funding_transaction(&self, id: &Uuid) -> Result<Option<FundingTransaction>, String>;
    async fn get_funding_transaction_by_user(&self, user_id: &Uuid, transaction_id: &Uuid) -> Result<Option<FundingTransaction>, String>;
    async fn list_funding_transactions(&self, user_id: &Uuid, card_id: Option<Uuid>, source_id: Option<Uuid>, status: Option<FundingTransactionStatus>, page: i32, page_size: i32) -> Result<(Vec<FundingTransaction>, i64), String>;
    async fn update_funding_transaction(&self, transaction: &FundingTransaction) -> Result<FundingTransaction, String>;
    async fn get_transactions_by_reference(&self, reference: &str) -> Result<Option<FundingTransaction>, String>;

    // Funding limits operations
    async fn get_funding_limits(&self, user_id: &Uuid) -> Result<Option<FundingLimits>, String>;
    async fn create_funding_limits(&self, limits: &FundingLimits) -> Result<FundingLimits, String>;
    async fn update_funding_limits(&self, limits: &FundingLimits) -> Result<FundingLimits, String>;
    async fn reset_daily_limits(&self, user_id: &Uuid) -> Result<bool, String>;
    async fn reset_monthly_limits(&self, user_id: &Uuid) -> Result<bool, String>;
    async fn reset_yearly_limits(&self, user_id: &Uuid) -> Result<bool, String>;

    // Analytics operations
    async fn get_funding_metrics(&self, start_date: &DateTime<Utc>, end_date: &DateTime<Utc>, source_type: Option<FundingSourceType>, currency: Option<String>) -> Result<FundingMetrics, String>;
    async fn get_user_funding_volume(&self, user_id: &Uuid, start_date: &DateTime<Utc>, end_date: &DateTime<Utc>) -> Result<Decimal, String>;
}

/// In-memory implementation for development and testing
#[derive(Debug, Default)]
pub struct InMemoryCardFundingRepository {
    funding_sources: std::sync::RwLock<HashMap<Uuid, FundingSource>>,
    funding_transactions: std::sync::RwLock<HashMap<Uuid, FundingTransaction>>,
    funding_limits: std::sync::RwLock<HashMap<Uuid, FundingLimits>>,
}

impl InMemoryCardFundingRepository {
    pub fn new() -> Self {
        Self::default()
    }

    fn generate_reference_number() -> String {
        format!("FND{}", Uuid::new_v4().to_string().replace('-', "").to_uppercase()[..12].to_string())
    }
}

#[async_trait::async_trait]
impl CardFundingRepository for InMemoryCardFundingRepository {
    // Funding source operations
    async fn create_funding_source(&self, source: &FundingSource) -> Result<FundingSource, String> {
        let mut sources = self.funding_sources.write().unwrap();
        sources.insert(source.id, source.clone());
        Ok(source.clone())
    }

    async fn get_funding_source(&self, id: &Uuid) -> Result<Option<FundingSource>, String> {
        let sources = self.funding_sources.read().unwrap();
        Ok(sources.get(id).cloned())
    }

    async fn get_funding_source_by_user(&self, user_id: &Uuid, source_id: &Uuid) -> Result<Option<FundingSource>, String> {
        let sources = self.funding_sources.read().unwrap();
        Ok(sources.get(source_id)
            .filter(|source| source.user_id == *user_id)
            .cloned())
    }

    async fn list_funding_sources(
        &self,
        user_id: &Uuid,
        source_type: Option<FundingSourceType>,
        status: Option<FundingSourceStatus>,
        page: i32,
        page_size: i32
    ) -> Result<(Vec<FundingSource>, i64), String> {
        let sources = self.funding_sources.read().unwrap();
        let mut filtered_sources: Vec<FundingSource> = sources
            .values()
            .filter(|source| {
                source.user_id == *user_id &&
                source_type.as_ref().map_or(true, |t| source.source_type == *t) &&
                status.as_ref().map_or(true, |s| source.status == *s)
            })
            .cloned()
            .collect();

        // Sort by created_at descending
        filtered_sources.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        let total_count = filtered_sources.len() as i64;
        let start = ((page - 1) * page_size) as usize;
        let end = (start + page_size as usize).min(filtered_sources.len());

        let paginated_sources = if start < filtered_sources.len() {
            filtered_sources[start..end].to_vec()
        } else {
            Vec::new()
        };

        Ok((paginated_sources, total_count))
    }

    async fn update_funding_source(&self, source: &FundingSource) -> Result<FundingSource, String> {
        let mut sources = self.funding_sources.write().unwrap();
        sources.insert(source.id, source.clone());
        Ok(source.clone())
    }

    async fn delete_funding_source(&self, id: &Uuid) -> Result<bool, String> {
        let mut sources = self.funding_sources.write().unwrap();
        Ok(sources.remove(id).is_some())
    }

    // Funding transaction operations
    async fn create_funding_transaction(&self, transaction: &FundingTransaction) -> Result<FundingTransaction, String> {
        let mut transactions = self.funding_transactions.write().unwrap();
        transactions.insert(transaction.id, transaction.clone());
        Ok(transaction.clone())
    }

    async fn get_funding_transaction(&self, id: &Uuid) -> Result<Option<FundingTransaction>, String> {
        let transactions = self.funding_transactions.read().unwrap();
        Ok(transactions.get(id).cloned())
    }

    async fn get_funding_transaction_by_user(&self, user_id: &Uuid, transaction_id: &Uuid) -> Result<Option<FundingTransaction>, String> {
        let transactions = self.funding_transactions.read().unwrap();
        Ok(transactions.get(transaction_id)
            .filter(|tx| tx.user_id == *user_id)
            .cloned())
    }

    async fn list_funding_transactions(
        &self,
        user_id: &Uuid,
        card_id: Option<Uuid>,
        source_id: Option<Uuid>,
        status: Option<FundingTransactionStatus>,
        page: i32,
        page_size: i32
    ) -> Result<(Vec<FundingTransaction>, i64), String> {
        let transactions = self.funding_transactions.read().unwrap();
        let mut filtered_transactions: Vec<FundingTransaction> = transactions
            .values()
            .filter(|tx| {
                tx.user_id == *user_id &&
                card_id.map_or(true, |id| tx.card_id == id) &&
                source_id.map_or(true, |id| tx.funding_source_id == id) &&
                status.as_ref().map_or(true, |s| tx.status == *s)
            })
            .cloned()
            .collect();

        // Sort by created_at descending
        filtered_transactions.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        let total_count = filtered_transactions.len() as i64;
        let start = ((page - 1) * page_size) as usize;
        let end = (start + page_size as usize).min(filtered_transactions.len());

        let paginated_transactions = if start < filtered_transactions.len() {
            filtered_transactions[start..end].to_vec()
        } else {
            Vec::new()
        };

        Ok((paginated_transactions, total_count))
    }

    async fn update_funding_transaction(&self, transaction: &FundingTransaction) -> Result<FundingTransaction, String> {
        let mut transactions = self.funding_transactions.write().unwrap();
        transactions.insert(transaction.id, transaction.clone());
        Ok(transaction.clone())
    }

    async fn get_transactions_by_reference(&self, reference: &str) -> Result<Option<FundingTransaction>, String> {
        let transactions = self.funding_transactions.read().unwrap();
        Ok(transactions.values()
            .find(|tx| tx.reference_number == reference)
            .cloned())
    }

    // Funding limits operations
    async fn get_funding_limits(&self, user_id: &Uuid) -> Result<Option<FundingLimits>, String> {
        let limits = self.funding_limits.read().unwrap();
        Ok(limits.get(user_id).cloned())
    }

    async fn create_funding_limits(&self, limits: &FundingLimits) -> Result<FundingLimits, String> {
        let mut funding_limits = self.funding_limits.write().unwrap();
        funding_limits.insert(limits.user_id, limits.clone());
        Ok(limits.clone())
    }

    async fn update_funding_limits(&self, limits: &FundingLimits) -> Result<FundingLimits, String> {
        let mut funding_limits = self.funding_limits.write().unwrap();
        funding_limits.insert(limits.user_id, limits.clone());
        Ok(limits.clone())
    }

    async fn reset_daily_limits(&self, user_id: &Uuid) -> Result<bool, String> {
        let mut limits = self.funding_limits.write().unwrap();
        if let Some(user_limits) = limits.get_mut(user_id) {
            user_limits.daily_used = Decimal::ZERO;
            user_limits.daily_transactions_used = 0;
            user_limits.last_reset_daily = Utc::now();
            user_limits.updated_at = Utc::now();
            Ok(true)
        } else {
            Ok(false)
        }
    }

    async fn reset_monthly_limits(&self, user_id: &Uuid) -> Result<bool, String> {
        let mut limits = self.funding_limits.write().unwrap();
        if let Some(user_limits) = limits.get_mut(user_id) {
            user_limits.monthly_used = Decimal::ZERO;
            user_limits.monthly_transactions_used = 0;
            user_limits.last_reset_monthly = Utc::now();
            user_limits.updated_at = Utc::now();
            Ok(true)
        } else {
            Ok(false)
        }
    }

    async fn reset_yearly_limits(&self, user_id: &Uuid) -> Result<bool, String> {
        let mut limits = self.funding_limits.write().unwrap();
        if let Some(user_limits) = limits.get_mut(user_id) {
            user_limits.yearly_used = Decimal::ZERO;
            user_limits.last_reset_yearly = Utc::now();
            user_limits.updated_at = Utc::now();
            Ok(true)
        } else {
            Ok(false)
        }
    }

    // Analytics operations
    async fn get_funding_metrics(
        &self,
        start_date: &DateTime<Utc>,
        end_date: &DateTime<Utc>,
        source_type: Option<FundingSourceType>,
        currency: Option<String>
    ) -> Result<FundingMetrics, String> {
        let transactions = self.funding_transactions.read().unwrap();
        let sources = self.funding_sources.read().unwrap();

        // Filter transactions by date range and optional filters
        let filtered_transactions: Vec<&FundingTransaction> = transactions
            .values()
            .filter(|tx| {
                tx.created_at >= *start_date &&
                tx.created_at <= *end_date &&
                currency.as_ref().map_or(true, |c| tx.currency == *c) &&
                tx.status == FundingTransactionStatus::Completed
            })
            .filter(|tx| {
                if let Some(ref filter_type) = source_type {
                    if let Some(source) = sources.get(&tx.funding_source_id) {
                        source.source_type == *filter_type
                    } else {
                        false
                    }
                } else {
                    true
                }
            })
            .collect();

        let total_transactions = filtered_transactions.len() as i64;
        let total_volume: Decimal = filtered_transactions.iter().map(|tx| tx.amount).sum();
        let total_fees: Decimal = filtered_transactions.iter().map(|tx| tx.fee_amount).sum();
        let average_transaction_size = if total_transactions > 0 {
            total_volume / Decimal::from(total_transactions)
        } else {
            Decimal::ZERO
        };

        // Calculate success rate (completed vs all attempted)
        let all_transactions: Vec<&FundingTransaction> = transactions
            .values()
            .filter(|tx| {
                tx.created_at >= *start_date &&
                tx.created_at <= *end_date &&
                currency.as_ref().map_or(true, |c| tx.currency == *c)
            })
            .collect();

        let success_rate = if !all_transactions.is_empty() {
            Decimal::from(total_transactions) / Decimal::from(all_transactions.len())
        } else {
            Decimal::ZERO
        };

        // Group by source type
        let mut source_metrics: HashMap<FundingSourceType, FundingSourceMetrics> = HashMap::new();
        for tx in &filtered_transactions {
            if let Some(source) = sources.get(&tx.funding_source_id) {
                let entry = source_metrics.entry(source.source_type.clone()).or_insert_with(|| {
                    FundingSourceMetrics {
                        source_type: source.source_type.clone(),
                        volume: Decimal::ZERO,
                        fees: Decimal::ZERO,
                        transaction_count: 0,
                        success_rate: Decimal::ZERO,
                    }
                });
                entry.volume += tx.amount;
                entry.fees += tx.fee_amount;
                entry.transaction_count += 1;
            }
        }

        // Calculate success rates for each source type
        for (source_type, metrics) in source_metrics.iter_mut() {
            let source_total_attempts = all_transactions
                .iter()
                .filter(|tx| {
                    sources.get(&tx.funding_source_id)
                        .map_or(false, |s| s.source_type == *source_type)
                })
                .count();

            metrics.success_rate = if source_total_attempts > 0 {
                Decimal::from(metrics.transaction_count) / Decimal::from(source_total_attempts)
            } else {
                Decimal::ZERO
            };
        }

        // Group by currency
        let mut currency_metrics: HashMap<String, CurrencyMetrics> = HashMap::new();
        for tx in &filtered_transactions {
            let entry = currency_metrics.entry(tx.currency.clone()).or_insert_with(|| {
                CurrencyMetrics {
                    currency: tx.currency.clone(),
                    volume: Decimal::ZERO,
                    fees: Decimal::ZERO,
                    transaction_count: 0,
                }
            });
            entry.volume += tx.amount;
            entry.fees += tx.fee_amount;
            entry.transaction_count += 1;
        }

        Ok(FundingMetrics {
            total_volume,
            total_fees,
            total_transactions,
            average_transaction_size,
            by_source: source_metrics.into_values().collect(),
            by_currency: currency_metrics.into_values().collect(),
            success_rate,
        })
    }

    async fn get_user_funding_volume(
        &self,
        user_id: &Uuid,
        start_date: &DateTime<Utc>,
        end_date: &DateTime<Utc>
    ) -> Result<Decimal, String> {
        let transactions = self.funding_transactions.read().unwrap();

        let volume: Decimal = transactions
            .values()
            .filter(|tx| {
                tx.user_id == *user_id &&
                tx.created_at >= *start_date &&
                tx.created_at <= *end_date &&
                tx.status == FundingTransactionStatus::Completed
            })
            .map(|tx| tx.amount)
            .sum();

        Ok(volume)
    }
}
