//! Earn service data models and repository

use std::collections::HashMap;
use std::sync::RwLock;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use std::str::FromStr;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use async_trait::async_trait;

/// Yield product type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum YieldProductType {
    Staking,
    Lending,
    Vault,
    LiquidityMining,
    Farming,
}

/// Protocol type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProtocolType {
    Lido,
    Aave,
    Compound,
    Yearn,
    EigenLayer,
    Marinade,
    Raydium,
    Orca,
}

/// Risk level enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// Position status enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PositionStatus {
    Active,
    Pending,
    Unstaking,
    Completed,
    Failed,
}

impl Default for PositionStatus {
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

/// Yield product entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YieldProduct {
    pub product_id: Uuid,
    pub name: String,
    pub description: String,
    pub product_type: YieldProductType,
    pub protocol: ProtocolType,
    pub chain_type: KeyType,
    pub chain_id: String,
    pub token_address: String,
    pub token_symbol: String,
    pub current_apy: Decimal,
    pub historical_apy: Decimal, // 30-day average
    pub tvl: Decimal, // Total Value Locked
    pub minimum_deposit: Decimal,
    pub maximum_deposit: Decimal,
    pub lock_period_days: i64, // 0 for no lock
    pub risk_level: RiskLevel,
    pub is_active: bool,
    pub features: Vec<String>,
    pub metadata: HashMap<String, String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl YieldProduct {
    pub fn new(
        name: String,
        description: String,
        product_type: YieldProductType,
        protocol: ProtocolType,
        chain_type: KeyType,
        chain_id: String,
        token_address: String,
        token_symbol: String,
        current_apy: Decimal,
        risk_level: RiskLevel,
    ) -> Self {
        let now = Utc::now();
        Self {
            product_id: Uuid::new_v4(),
            name,
            description,
            product_type,
            protocol,
            chain_type,
            chain_id,
            token_address,
            token_symbol,
            current_apy,
            historical_apy: current_apy, // Initialize with current APY
            tvl: Decimal::ZERO,
            minimum_deposit: Decimal::from(10), // Default $10 minimum
            maximum_deposit: Decimal::from(1_000_000), // Default $1M maximum
            lock_period_days: 0,
            risk_level,
            is_active: true,
            features: Vec::new(),
            metadata: HashMap::new(),
            created_at: now,
            updated_at: now,
        }
    }
}

/// Yield calculation entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YieldCalculation {
    pub principal_amount: Decimal,
    pub estimated_yield: Decimal,
    pub total_return: Decimal,
    pub apy_used: Decimal,
    pub time_period_days: i64,
    pub breakdown: Vec<YieldBreakdown>,
    pub fees: Decimal,
    pub net_yield: Decimal,
    pub metadata: HashMap<String, String>,
}

/// Yield breakdown entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YieldBreakdown {
    pub period: String, // "daily", "weekly", "monthly", "yearly"
    pub yield_amount: Decimal,
    pub cumulative_yield: Decimal,
    pub apy_at_period: Decimal,
}

/// Staking position entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StakingPosition {
    pub position_id: Uuid,
    pub user_id: Uuid,
    pub product_id: Uuid,
    pub validator_address: Option<String>, // For native staking
    pub staked_amount: Decimal,
    pub rewards_earned: Decimal,
    pub current_value: Decimal,
    pub status: PositionStatus,
    pub staked_at: DateTime<Utc>,
    pub unlock_at: Option<DateTime<Utc>>, // None if no lock
    pub transaction_hash: String,
    pub metadata: HashMap<String, String>,
}

impl StakingPosition {
    pub fn new(
        user_id: Uuid,
        product_id: Uuid,
        staked_amount: Decimal,
        transaction_hash: String,
    ) -> Self {
        Self {
            position_id: Uuid::new_v4(),
            user_id,
            product_id,
            validator_address: None,
            staked_amount,
            rewards_earned: Decimal::ZERO,
            current_value: staked_amount,
            status: PositionStatus::Pending,
            staked_at: Utc::now(),
            unlock_at: None,
            transaction_hash,
            metadata: HashMap::new(),
        }
    }
}

/// Lending position entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LendingPosition {
    pub position_id: Uuid,
    pub user_id: Uuid,
    pub product_id: Uuid,
    pub supplied_amount: Decimal,
    pub interest_earned: Decimal,
    pub current_value: Decimal,
    pub supply_apy: Decimal,
    pub status: PositionStatus,
    pub supplied_at: DateTime<Utc>,
    pub transaction_hash: String,
    pub metadata: HashMap<String, String>,
}

impl LendingPosition {
    pub fn new(
        user_id: Uuid,
        product_id: Uuid,
        supplied_amount: Decimal,
        supply_apy: Decimal,
        transaction_hash: String,
    ) -> Self {
        Self {
            position_id: Uuid::new_v4(),
            user_id,
            product_id,
            supplied_amount,
            interest_earned: Decimal::ZERO,
            current_value: supplied_amount,
            supply_apy,
            status: PositionStatus::Pending,
            supplied_at: Utc::now(),
            transaction_hash,
            metadata: HashMap::new(),
        }
    }
}

/// Vault position entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultPosition {
    pub position_id: Uuid,
    pub user_id: Uuid,
    pub product_id: Uuid,
    pub deposited_amount: Decimal,
    pub shares: Decimal,
    pub current_value: Decimal,
    pub yield_earned: Decimal,
    pub status: PositionStatus,
    pub deposited_at: DateTime<Utc>,
    pub transaction_hash: String,
    pub metadata: HashMap<String, String>,
}

impl VaultPosition {
    pub fn new(
        user_id: Uuid,
        product_id: Uuid,
        deposited_amount: Decimal,
        shares: Decimal,
        transaction_hash: String,
    ) -> Self {
        Self {
            position_id: Uuid::new_v4(),
            user_id,
            product_id,
            deposited_amount,
            shares,
            current_value: deposited_amount,
            yield_earned: Decimal::ZERO,
            status: PositionStatus::Pending,
            deposited_at: Utc::now(),
            transaction_hash,
            metadata: HashMap::new(),
        }
    }
}

/// Earn analytics entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EarnAnalytics {
    pub user_id: Uuid,
    pub total_deposited: Decimal,
    pub total_earned: Decimal,
    pub current_value: Decimal,
    pub average_apy: Decimal,
    pub active_positions: i32,
    pub product_distribution: Vec<YieldProductType>,
    pub protocol_distribution: Vec<ProtocolType>,
    pub chain_distribution: HashMap<String, Decimal>,
    pub best_performing_product: Option<Uuid>,
    pub total_fees_paid: Decimal,
    pub first_deposit_at: Option<DateTime<Utc>>,
    pub last_activity_at: DateTime<Utc>,
}

/// Portfolio summary entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioSummary {
    pub user_id: Uuid,
    pub total_portfolio_value: Decimal,
    pub total_yield_earned: Decimal,
    pub weighted_average_apy: Decimal,
    pub positions: Vec<PositionSummary>,
    pub overall_risk_level: RiskLevel,
    pub diversification_score: Decimal, // 0-100
    pub recommendations: Vec<String>,
    pub last_updated_at: DateTime<Utc>,
}

/// Position summary entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionSummary {
    pub position_id: Uuid,
    pub product_type: YieldProductType,
    pub protocol: ProtocolType,
    pub token_symbol: String,
    pub amount: Decimal,
    pub current_value: Decimal,
    pub yield_earned: Decimal,
    pub current_apy: Decimal,
    pub risk_level: RiskLevel,
    pub portfolio_percentage: f64,
}

/// Yield chart data entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YieldChartData {
    pub data_points: Vec<YieldDataPoint>,
    pub total_yield: Decimal,
    pub period: String, // "7d", "30d", "90d", "1y"
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
}

/// Yield data point entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YieldDataPoint {
    pub timestamp: DateTime<Utc>,
    pub yield_amount: Decimal,
    pub cumulative_yield: Decimal,
    pub apy: Decimal,
    pub portfolio_value: Decimal,
}

/// Risk assessment entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAssessment {
    pub overall_risk: RiskLevel,
    pub risk_score: Decimal, // 0-100
    pub risk_factors: Vec<RiskFactor>,
    pub warnings: Vec<String>,
    pub recommendations: Vec<String>,
    pub diversification_score: Decimal,
    pub concentration_risk: Decimal,
    pub metadata: HashMap<String, String>,
}

/// Risk factor entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskFactor {
    pub factor_name: String,
    pub risk_level: RiskLevel,
    pub description: String,
    pub impact_score: Decimal, // 0-100
    pub mitigation: String,
}

/// Portfolio optimization entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioOptimization {
    pub current_apy: Decimal,
    pub optimized_apy: Decimal,
    pub potential_improvement: Decimal,
    pub suggestions: Vec<OptimizationSuggestion>,
    pub target_risk_level: RiskLevel,
    pub rebalancing_cost: Decimal,
    pub expected_return_improvement: Decimal,
}

/// Optimization suggestion entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationSuggestion {
    pub action: String, // "rebalance", "add", "remove", "increase", "decrease"
    pub product_id: Uuid,
    pub current_allocation: Decimal,
    pub suggested_allocation: Decimal,
    pub reason: String,
    pub expected_impact: Decimal,
    pub priority: i32, // 1-10
}

/// Earn repository trait
#[async_trait]
pub trait EarnRepository: Send + Sync {
    // Yield product operations
    async fn create_yield_product(&self, product: &YieldProduct) -> Result<YieldProduct, String>;
    async fn get_yield_product(&self, product_id: &Uuid) -> Result<Option<YieldProduct>, String>;
    async fn list_yield_products(
        &self,
        product_type: Option<YieldProductType>,
        protocol: Option<ProtocolType>,
        chain_type: Option<KeyType>,
        chain_id: Option<String>,
        risk_level: Option<RiskLevel>,
        min_apy: Option<Decimal>,
        max_apy: Option<Decimal>,
        active_only: bool,
        sort_by: String,
        sort_desc: bool,
        page: i32,
        page_size: i32,
    ) -> Result<(Vec<YieldProduct>, i64), String>;
    async fn update_yield_product(&self, product: &YieldProduct) -> Result<YieldProduct, String>;

    // Yield calculation operations
    async fn calculate_yield(&self, calculation: &YieldCalculation) -> Result<YieldCalculation, String>;
    async fn get_yield_history(
        &self,
        product_id: &Uuid,
        start_date: Option<DateTime<Utc>>,
        end_date: Option<DateTime<Utc>>,
        period: String,
    ) -> Result<Vec<YieldDataPoint>, String>;

    // Staking operations
    async fn create_staking_position(&self, position: &StakingPosition) -> Result<StakingPosition, String>;
    async fn get_staking_position(&self, position_id: &Uuid) -> Result<Option<StakingPosition>, String>;
    async fn list_staking_positions(
        &self,
        user_id: Option<Uuid>,
        product_id: Option<Uuid>,
        status: Option<PositionStatus>,
        page: i32,
        page_size: i32,
    ) -> Result<(Vec<StakingPosition>, i64), String>;
    async fn update_staking_position(&self, position: &StakingPosition) -> Result<StakingPosition, String>;

    // Lending operations
    async fn create_lending_position(&self, position: &LendingPosition) -> Result<LendingPosition, String>;
    async fn get_lending_position(&self, position_id: &Uuid) -> Result<Option<LendingPosition>, String>;
    async fn list_lending_positions(
        &self,
        user_id: Option<Uuid>,
        product_id: Option<Uuid>,
        status: Option<PositionStatus>,
        page: i32,
        page_size: i32,
    ) -> Result<(Vec<LendingPosition>, i64), String>;
    async fn update_lending_position(&self, position: &LendingPosition) -> Result<LendingPosition, String>;

    // Vault operations
    async fn create_vault_position(&self, position: &VaultPosition) -> Result<VaultPosition, String>;
    async fn get_vault_position(&self, position_id: &Uuid) -> Result<Option<VaultPosition>, String>;
    async fn list_vault_positions(
        &self,
        user_id: Option<Uuid>,
        product_id: Option<Uuid>,
        status: Option<PositionStatus>,
        page: i32,
        page_size: i32,
    ) -> Result<(Vec<VaultPosition>, i64), String>;
    async fn update_vault_position(&self, position: &VaultPosition) -> Result<VaultPosition, String>;

    // Analytics operations
    async fn get_earn_analytics(
        &self,
        user_id: Option<Uuid>,
        start_date: Option<DateTime<Utc>>,
        end_date: Option<DateTime<Utc>>,
    ) -> Result<EarnAnalytics, String>;
    async fn get_portfolio_summary(&self, user_id: &Uuid) -> Result<PortfolioSummary, String>;
    async fn get_yield_chart(
        &self,
        user_id: Option<Uuid>,
        product_id: Option<Uuid>,
        period: String,
        start_date: Option<DateTime<Utc>>,
        end_date: Option<DateTime<Utc>>,
    ) -> Result<YieldChartData, String>;

    // Risk and optimization operations
    async fn assess_risk(&self, user_id: &Uuid, product_ids: Option<Vec<Uuid>>) -> Result<RiskAssessment, String>;
    async fn optimize_portfolio(
        &self,
        user_id: &Uuid,
        target_risk_level: Option<RiskLevel>,
        target_apy: Option<Decimal>,
        max_rebalancing_cost: Option<Decimal>,
    ) -> Result<PortfolioOptimization, String>;
}

/// In-memory implementation for development and testing
#[derive(Debug, Default)]
pub struct InMemoryEarnRepository {
    yield_products: RwLock<HashMap<Uuid, YieldProduct>>,
    staking_positions: RwLock<HashMap<Uuid, StakingPosition>>,
    lending_positions: RwLock<HashMap<Uuid, LendingPosition>>,
    vault_positions: RwLock<HashMap<Uuid, VaultPosition>>,
    yield_calculations: RwLock<Vec<YieldCalculation>>,
    yield_history: RwLock<HashMap<Uuid, Vec<YieldDataPoint>>>, // product_id -> history
}

impl InMemoryEarnRepository {
    pub fn new() -> Self {
        let mut repo = Self::default();

        // Initialize with some sample yield products
        repo.initialize_sample_products();

        repo
    }

    fn initialize_sample_products(&mut self) {
        let mut products = self.yield_products.write().unwrap();

        // Ethereum Staking (Lido)
        let eth_staking = YieldProduct::new(
            "Ethereum Staking".to_string(),
            "Liquid staking with Lido protocol".to_string(),
            YieldProductType::Staking,
            ProtocolType::Lido,
            KeyType::Ethereum,
            "1".to_string(),
            "0xae7ab96520DE3A18E5e111B5EaAb095312D7fE84".to_string(),
            "stETH".to_string(),
            Decimal::from_str("4.2").unwrap(), // 4.2% APY
            RiskLevel::Low,
        );
        products.insert(eth_staking.product_id, eth_staking);

        // USDC Lending (Aave)
        let usdc_lending = YieldProduct::new(
            "USDC Lending".to_string(),
            "Supply USDC to Aave protocol".to_string(),
            YieldProductType::Lending,
            ProtocolType::Aave,
            KeyType::Ethereum,
            "1".to_string(),
            "0xA0b86a33E6441b8C4505B6B8C0C4C0C4C0C4C0C4".to_string(),
            "USDC".to_string(),
            Decimal::from_str("3.8").unwrap(), // 3.8% APY
            RiskLevel::Low,
        );
        products.insert(usdc_lending.product_id, usdc_lending);

        // Yearn Vault
        let yearn_vault = YieldProduct::new(
            "Yearn USDC Vault".to_string(),
            "Automated yield farming with Yearn".to_string(),
            YieldProductType::Vault,
            ProtocolType::Yearn,
            KeyType::Ethereum,
            "1".to_string(),
            "0xa354F35829Ae975e850e23e9615b11Da1B3dC4DE".to_string(),
            "yvUSDC".to_string(),
            Decimal::from_str("6.5").unwrap(), // 6.5% APY
            RiskLevel::Medium,
        );
        products.insert(yearn_vault.product_id, yearn_vault);

        // Solana Staking (Marinade)
        let sol_staking = YieldProduct::new(
            "Solana Staking".to_string(),
            "Liquid staking with Marinade protocol".to_string(),
            YieldProductType::Staking,
            ProtocolType::Marinade,
            KeyType::Solana,
            "mainnet-beta".to_string(),
            "mSoLzYCxHdYgdzU16g5QSh3i5K3z3KZK7ytfqcJm7So".to_string(),
            "mSOL".to_string(),
            Decimal::from_str("7.2").unwrap(), // 7.2% APY
            RiskLevel::Medium,
        );
        products.insert(sol_staking.product_id, sol_staking);
    }
}

#[async_trait]
impl EarnRepository for InMemoryEarnRepository {
    // Yield product operations
    async fn create_yield_product(&self, product: &YieldProduct) -> Result<YieldProduct, String> {
        let mut products = self.yield_products.write().unwrap();
        products.insert(product.product_id, product.clone());
        Ok(product.clone())
    }

    async fn get_yield_product(&self, product_id: &Uuid) -> Result<Option<YieldProduct>, String> {
        let products = self.yield_products.read().unwrap();
        Ok(products.get(product_id).cloned())
    }

    async fn list_yield_products(
        &self,
        product_type: Option<YieldProductType>,
        protocol: Option<ProtocolType>,
        chain_type: Option<KeyType>,
        chain_id: Option<String>,
        max_risk_level: Option<RiskLevel>,
        min_apy: Option<Decimal>,
        max_apy: Option<Decimal>,
        active_only: bool,
        sort_by: String,
        sort_desc: bool,
        page: i32,
        page_size: i32,
    ) -> Result<(Vec<YieldProduct>, i64), String> {
        let products = self.yield_products.read().unwrap();

        // Apply filters
        let mut filtered: Vec<YieldProduct> = products.values()
            .filter(|p| {
                if active_only && !p.is_active {
                    return false;
                }
                if let Some(pt) = product_type {
                    if p.product_type != pt {
                        return false;
                    }
                }
                if let Some(proto) = protocol {
                    if p.protocol != proto {
                        return false;
                    }
                }
                if let Some(ct) = chain_type {
                    if p.chain_type != ct {
                        return false;
                    }
                }
                if let Some(ref cid) = chain_id {
                    if p.chain_id != *cid {
                        return false;
                    }
                }
                if let Some(max_risk) = max_risk_level {
                    if p.risk_level as u8 > max_risk as u8 {
                        return false;
                    }
                }
                if let Some(min) = min_apy {
                    if p.current_apy < min {
                        return false;
                    }
                }
                if let Some(max) = max_apy {
                    if p.current_apy > max {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect();

        // Sort
        match sort_by.as_str() {
            "apy" => {
                if sort_desc {
                    filtered.sort_by(|a, b| b.current_apy.cmp(&a.current_apy));
                } else {
                    filtered.sort_by(|a, b| a.current_apy.cmp(&b.current_apy));
                }
            }
            "tvl" => {
                if sort_desc {
                    filtered.sort_by(|a, b| b.tvl.cmp(&a.tvl));
                } else {
                    filtered.sort_by(|a, b| a.tvl.cmp(&b.tvl));
                }
            }
            "risk" => {
                if sort_desc {
                    filtered.sort_by(|a, b| (b.risk_level as u8).cmp(&(a.risk_level as u8)));
                } else {
                    filtered.sort_by(|a, b| (a.risk_level as u8).cmp(&(b.risk_level as u8)));
                }
            }
            "name" => {
                if sort_desc {
                    filtered.sort_by(|a, b| b.name.cmp(&a.name));
                } else {
                    filtered.sort_by(|a, b| a.name.cmp(&b.name));
                }
            }
            _ => {
                // Default sort by APY descending
                filtered.sort_by(|a, b| b.current_apy.cmp(&a.current_apy));
            }
        }

        let total_count = filtered.len() as i64;

        // Apply pagination
        let start = ((page - 1) * page_size) as usize;
        let end = (start + page_size as usize).min(filtered.len());

        if start >= filtered.len() {
            return Ok((Vec::new(), total_count));
        }

        let paginated = filtered[start..end].to_vec();
        Ok((paginated, total_count))
    }

    async fn update_yield_product(&self, product: &YieldProduct) -> Result<YieldProduct, String> {
        let mut products = self.yield_products.write().unwrap();
        if products.contains_key(&product.product_id) {
            products.insert(product.product_id, product.clone());
            Ok(product.clone())
        } else {
            Err("Product not found".to_string())
        }
    }

    // Yield calculation operations
    async fn calculate_yield(&self, calculation: &YieldCalculation) -> Result<YieldCalculation, String> {
        let mut calculations = self.yield_calculations.write().unwrap();
        calculations.push(calculation.clone());
        Ok(calculation.clone())
    }

    async fn get_yield_history(
        &self,
        product_id: &Uuid,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
        period: &str,
    ) -> Result<Vec<YieldDataPoint>, String> {
        let history = self.yield_history.read().unwrap();

        // Get or generate mock history for the product
        if let Some(product_history) = history.get(product_id) {
            // Filter by date range
            let filtered: Vec<YieldDataPoint> = product_history.iter()
                .filter(|dp| dp.timestamp >= start_date && dp.timestamp <= end_date)
                .cloned()
                .collect();
            Ok(filtered)
        } else {
            // Generate mock history data
            let mut mock_history = Vec::new();
            let mut current_date = start_date;
            let mut cumulative_yield = Decimal::ZERO;
            let base_apy = Decimal::from_str("5.0").unwrap();

            while current_date <= end_date {
                let daily_yield = Decimal::from_str("0.1").unwrap(); // Mock daily yield
                cumulative_yield += daily_yield;

                mock_history.push(YieldDataPoint {
                    timestamp: current_date,
                    yield_amount: daily_yield,
                    cumulative_yield,
                    apy: base_apy,
                    portfolio_value: Decimal::from_str("1000.0").unwrap() + cumulative_yield,
                });

                current_date = match period {
                    "daily" => current_date + chrono::Duration::days(1),
                    "weekly" => current_date + chrono::Duration::weeks(1),
                    "monthly" => current_date + chrono::Duration::days(30),
                    _ => current_date + chrono::Duration::days(1),
                };
            }

            Ok(mock_history)
        }
    }

    // Staking operations
    async fn create_staking_position(&self, position: &StakingPosition) -> Result<StakingPosition, String> {
        let mut positions = self.staking_positions.write().unwrap();
        positions.insert(position.position_id, position.clone());
        Ok(position.clone())
    }

    async fn get_staking_position(&self, position_id: &Uuid) -> Result<Option<StakingPosition>, String> {
        let positions = self.staking_positions.read().unwrap();
        Ok(positions.get(position_id).cloned())
    }

    async fn list_staking_positions(
        &self,
        user_id: Uuid,
        product_id: Option<Uuid>,
        status: Option<PositionStatus>,
        page: i32,
        page_size: i32,
    ) -> Result<(Vec<StakingPosition>, i64), String> {
        let positions = self.staking_positions.read().unwrap();

        // Apply filters
        let mut filtered: Vec<StakingPosition> = positions.values()
            .filter(|p| {
                if p.user_id != user_id {
                    return false;
                }
                if let Some(pid) = product_id {
                    if p.product_id != pid {
                        return false;
                    }
                }
                if let Some(s) = status {
                    if p.status != s {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect();

        // Sort by staked_at descending (newest first)
        filtered.sort_by(|a, b| b.staked_at.cmp(&a.staked_at));

        let total_count = filtered.len() as i64;

        // Apply pagination
        let start = ((page - 1) * page_size) as usize;
        let end = (start + page_size as usize).min(filtered.len());

        if start >= filtered.len() {
            return Ok((Vec::new(), total_count));
        }

        let paginated = filtered[start..end].to_vec();
        Ok((paginated, total_count))
    }

    async fn update_staking_position(&self, position: &StakingPosition) -> Result<StakingPosition, String> {
        let mut positions = self.staking_positions.write().unwrap();
        if positions.contains_key(&position.position_id) {
            positions.insert(position.position_id, position.clone());
            Ok(position.clone())
        } else {
            Err("Staking position not found".to_string())
        }
    }

    // Lending operations
    async fn create_lending_position(&self, position: &LendingPosition) -> Result<LendingPosition, String> {
        let mut positions = self.lending_positions.write().unwrap();
        positions.insert(position.position_id, position.clone());
        Ok(position.clone())
    }

    async fn get_lending_position(&self, position_id: &Uuid) -> Result<Option<LendingPosition>, String> {
        let positions = self.lending_positions.read().unwrap();
        Ok(positions.get(position_id).cloned())
    }

    async fn list_lending_positions(
        &self,
        user_id: Uuid,
        product_id: Option<Uuid>,
        status: Option<PositionStatus>,
        page: i32,
        page_size: i32,
    ) -> Result<(Vec<LendingPosition>, i64), String> {
        let positions = self.lending_positions.read().unwrap();

        // Apply filters
        let mut filtered: Vec<LendingPosition> = positions.values()
            .filter(|p| {
                if p.user_id != user_id {
                    return false;
                }
                if let Some(pid) = product_id {
                    if p.product_id != pid {
                        return false;
                    }
                }
                if let Some(s) = status {
                    if p.status != s {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect();

        // Sort by supplied_at descending (newest first)
        filtered.sort_by(|a, b| b.supplied_at.cmp(&a.supplied_at));

        let total_count = filtered.len() as i64;

        // Apply pagination
        let start = ((page - 1) * page_size) as usize;
        let end = (start + page_size as usize).min(filtered.len());

        if start >= filtered.len() {
            return Ok((Vec::new(), total_count));
        }

        let paginated = filtered[start..end].to_vec();
        Ok((paginated, total_count))
    }

    async fn update_lending_position(&self, position: &LendingPosition) -> Result<LendingPosition, String> {
        let mut positions = self.lending_positions.write().unwrap();
        if positions.contains_key(&position.position_id) {
            positions.insert(position.position_id, position.clone());
            Ok(position.clone())
        } else {
            Err("Lending position not found".to_string())
        }
    }

    // Vault operations
    async fn create_vault_position(&self, position: &VaultPosition) -> Result<VaultPosition, String> {
        let mut positions = self.vault_positions.write().unwrap();
        positions.insert(position.position_id, position.clone());
        Ok(position.clone())
    }

    async fn get_vault_position(&self, position_id: &Uuid) -> Result<Option<VaultPosition>, String> {
        let positions = self.vault_positions.read().unwrap();
        Ok(positions.get(position_id).cloned())
    }

    async fn list_vault_positions(
        &self,
        user_id: Uuid,
        product_id: Option<Uuid>,
        status: Option<PositionStatus>,
        page: i32,
        page_size: i32,
    ) -> Result<(Vec<VaultPosition>, i64), String> {
        let positions = self.vault_positions.read().unwrap();

        // Apply filters
        let mut filtered: Vec<VaultPosition> = positions.values()
            .filter(|p| {
                if p.user_id != user_id {
                    return false;
                }
                if let Some(pid) = product_id {
                    if p.product_id != pid {
                        return false;
                    }
                }
                if let Some(s) = status {
                    if p.status != s {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect();

        // Sort by deposited_at descending (newest first)
        filtered.sort_by(|a, b| b.deposited_at.cmp(&a.deposited_at));

        let total_count = filtered.len() as i64;

        // Apply pagination
        let start = ((page - 1) * page_size) as usize;
        let end = (start + page_size as usize).min(filtered.len());

        if start >= filtered.len() {
            return Ok((Vec::new(), total_count));
        }

        let paginated = filtered[start..end].to_vec();
        Ok((paginated, total_count))
    }

    async fn update_vault_position(&self, position: &VaultPosition) -> Result<VaultPosition, String> {
        let mut positions = self.vault_positions.write().unwrap();
        if positions.contains_key(&position.position_id) {
            positions.insert(position.position_id, position.clone());
            Ok(position.clone())
        } else {
            Err("Vault position not found".to_string())
        }
    }

    // Analytics operations
    async fn get_earn_analytics(
        &self,
        user_id: &Uuid,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> Result<EarnAnalytics, String> {
        let staking_positions = self.staking_positions.read().unwrap();
        let lending_positions = self.lending_positions.read().unwrap();
        let vault_positions = self.vault_positions.read().unwrap();
        let products = self.yield_products.read().unwrap();

        // Get all user positions
        let user_staking: Vec<&StakingPosition> = staking_positions.values()
            .filter(|p| p.user_id == *user_id)
            .collect();
        let user_lending: Vec<&LendingPosition> = lending_positions.values()
            .filter(|p| p.user_id == *user_id)
            .collect();
        let user_vault: Vec<&VaultPosition> = vault_positions.values()
            .filter(|p| p.user_id == *user_id)
            .collect();

        // Calculate totals
        let total_deposited = user_staking.iter().map(|p| p.staked_amount).sum::<Decimal>()
            + user_lending.iter().map(|p| p.supplied_amount).sum::<Decimal>()
            + user_vault.iter().map(|p| p.deposited_amount).sum::<Decimal>();

        let total_earned = user_staking.iter().map(|p| p.rewards_earned).sum::<Decimal>()
            + user_lending.iter().map(|p| p.interest_earned).sum::<Decimal>()
            + user_vault.iter().map(|p| p.yield_earned).sum::<Decimal>();

        let current_value = user_staking.iter().map(|p| p.current_value).sum::<Decimal>()
            + user_lending.iter().map(|p| p.current_value).sum::<Decimal>()
            + user_vault.iter().map(|p| p.current_value).sum::<Decimal>();

        // Calculate average APY (weighted by position value)
        let mut total_weighted_apy = Decimal::ZERO;
        let mut total_weight = Decimal::ZERO;

        for position in &user_staking {
            if let Some(product) = products.get(&position.product_id) {
                let weight = position.current_value;
                total_weighted_apy += product.current_apy * weight;
                total_weight += weight;
            }
        }
        for position in &user_lending {
            if let Some(product) = products.get(&position.product_id) {
                let weight = position.current_value;
                total_weighted_apy += product.current_apy * weight;
                total_weight += weight;
            }
        }
        for position in &user_vault {
            if let Some(product) = products.get(&position.product_id) {
                let weight = position.current_value;
                total_weighted_apy += product.current_apy * weight;
                total_weight += weight;
            }
        }

        let average_apy = if total_weight > Decimal::ZERO {
            total_weighted_apy / total_weight
        } else {
            Decimal::ZERO
        };

        let active_positions = (user_staking.len() + user_lending.len() + user_vault.len()) as i32;

        // Product distribution
        let mut product_types = Vec::new();
        for position in &user_staking {
            if let Some(product) = products.get(&position.product_id) {
                product_types.push(product.product_type);
            }
        }
        for position in &user_lending {
            if let Some(product) = products.get(&position.product_id) {
                product_types.push(product.product_type);
            }
        }
        for position in &user_vault {
            if let Some(product) = products.get(&position.product_id) {
                product_types.push(product.product_type);
            }
        }

        // Protocol distribution
        let mut protocols = Vec::new();
        for position in &user_staking {
            if let Some(product) = products.get(&position.product_id) {
                protocols.push(product.protocol);
            }
        }
        for position in &user_lending {
            if let Some(product) = products.get(&position.product_id) {
                protocols.push(product.protocol);
            }
        }
        for position in &user_vault {
            if let Some(product) = products.get(&position.product_id) {
                protocols.push(product.protocol);
            }
        }

        // Chain distribution
        let mut chain_distribution = HashMap::new();
        for position in &user_staking {
            if let Some(product) = products.get(&position.product_id) {
                let entry = chain_distribution.entry(product.chain_id.clone()).or_insert(Decimal::ZERO);
                *entry += position.current_value;
            }
        }
        for position in &user_lending {
            if let Some(product) = products.get(&position.product_id) {
                let entry = chain_distribution.entry(product.chain_id.clone()).or_insert(Decimal::ZERO);
                *entry += position.current_value;
            }
        }
        for position in &user_vault {
            if let Some(product) = products.get(&position.product_id) {
                let entry = chain_distribution.entry(product.chain_id.clone()).or_insert(Decimal::ZERO);
                *entry += position.current_value;
            }
        }

        // Find best performing product (highest APY)
        let mut best_performing_product = None;
        let mut highest_apy = Decimal::ZERO;
        for position in &user_staking {
            if let Some(product) = products.get(&position.product_id) {
                if product.current_apy > highest_apy {
                    highest_apy = product.current_apy;
                    best_performing_product = Some(product.product_id);
                }
            }
        }

        // Calculate fees (simplified - 0.5% of total earned)
        let total_fees_paid = total_earned * Decimal::from_str("0.005").unwrap();

        // Find first deposit date
        let mut first_deposit_at = None;
        if let Some(earliest_staking) = user_staking.iter().min_by_key(|p| p.staked_at) {
            first_deposit_at = Some(earliest_staking.staked_at);
        }
        if let Some(earliest_lending) = user_lending.iter().min_by_key(|p| p.supplied_at) {
            if first_deposit_at.is_none() || earliest_lending.supplied_at < first_deposit_at.unwrap() {
                first_deposit_at = Some(earliest_lending.supplied_at);
            }
        }
        if let Some(earliest_vault) = user_vault.iter().min_by_key(|p| p.deposited_at) {
            if first_deposit_at.is_none() || earliest_vault.deposited_at < first_deposit_at.unwrap() {
                first_deposit_at = Some(earliest_vault.deposited_at);
            }
        }

        // Find last activity date
        let mut last_activity_at = Utc::now();
        if let Some(latest_staking) = user_staking.iter().max_by_key(|p| p.staked_at) {
            last_activity_at = latest_staking.staked_at;
        }
        if let Some(latest_lending) = user_lending.iter().max_by_key(|p| p.supplied_at) {
            if latest_lending.supplied_at > last_activity_at {
                last_activity_at = latest_lending.supplied_at;
            }
        }
        if let Some(latest_vault) = user_vault.iter().max_by_key(|p| p.deposited_at) {
            if latest_vault.deposited_at > last_activity_at {
                last_activity_at = latest_vault.deposited_at;
            }
        }

        Ok(EarnAnalytics {
            user_id: *user_id,
            total_deposited,
            total_earned,
            current_value,
            average_apy,
            active_positions,
            product_distribution: product_types,
            protocol_distribution: protocols,
            chain_distribution,
            best_performing_product,
            total_fees_paid,
            first_deposit_at,
            last_activity_at,
        })
    }

    async fn get_portfolio_summary(&self, user_id: &Uuid) -> Result<PortfolioSummary, String> {
        let staking_positions = self.staking_positions.read().unwrap();
        let lending_positions = self.lending_positions.read().unwrap();
        let vault_positions = self.vault_positions.read().unwrap();
        let products = self.yield_products.read().unwrap();

        // Get all user positions
        let user_staking: Vec<&StakingPosition> = staking_positions.values()
            .filter(|p| p.user_id == *user_id && p.status == PositionStatus::Active)
            .collect();
        let user_lending: Vec<&LendingPosition> = lending_positions.values()
            .filter(|p| p.user_id == *user_id && p.status == PositionStatus::Active)
            .collect();
        let user_vault: Vec<&VaultPosition> = vault_positions.values()
            .filter(|p| p.user_id == *user_id && p.status == PositionStatus::Active)
            .collect();

        // Calculate total portfolio value
        let total_portfolio_value = user_staking.iter().map(|p| p.current_value).sum::<Decimal>()
            + user_lending.iter().map(|p| p.current_value).sum::<Decimal>()
            + user_vault.iter().map(|p| p.current_value).sum::<Decimal>();

        let total_yield_earned = user_staking.iter().map(|p| p.rewards_earned).sum::<Decimal>()
            + user_lending.iter().map(|p| p.interest_earned).sum::<Decimal>()
            + user_vault.iter().map(|p| p.yield_earned).sum::<Decimal>();

        // Calculate weighted average APY
        let mut total_weighted_apy = Decimal::ZERO;
        let mut total_weight = Decimal::ZERO;

        for position in &user_staking {
            if let Some(product) = products.get(&position.product_id) {
                let weight = position.current_value;
                total_weighted_apy += product.current_apy * weight;
                total_weight += weight;
            }
        }
        for position in &user_lending {
            if let Some(product) = products.get(&position.product_id) {
                let weight = position.current_value;
                total_weighted_apy += product.current_apy * weight;
                total_weight += weight;
            }
        }
        for position in &user_vault {
            if let Some(product) = products.get(&position.product_id) {
                let weight = position.current_value;
                total_weighted_apy += product.current_apy * weight;
                total_weight += weight;
            }
        }

        let weighted_average_apy = if total_weight > Decimal::ZERO {
            total_weighted_apy / total_weight
        } else {
            Decimal::ZERO
        };

        // Create position summaries
        let mut positions = Vec::new();

        for position in &user_staking {
            if let Some(product) = products.get(&position.product_id) {
                let portfolio_percentage = if total_portfolio_value > Decimal::ZERO {
                    (position.current_value / total_portfolio_value * Decimal::from(100)).to_f64().unwrap_or(0.0)
                } else {
                    0.0
                };

                positions.push(PositionSummary {
                    position_id: position.position_id,
                    product_type: product.product_type,
                    protocol: product.protocol,
                    token_symbol: product.token_symbol.clone(),
                    amount: position.staked_amount,
                    current_value: position.current_value,
                    yield_earned: position.rewards_earned,
                    current_apy: product.current_apy,
                    risk_level: product.risk_level,
                    portfolio_percentage,
                });
            }
        }

        for position in &user_lending {
            if let Some(product) = products.get(&position.product_id) {
                let portfolio_percentage = if total_portfolio_value > Decimal::ZERO {
                    (position.current_value / total_portfolio_value * Decimal::from(100)).to_f64().unwrap_or(0.0)
                } else {
                    0.0
                };

                positions.push(PositionSummary {
                    position_id: position.position_id,
                    product_type: product.product_type,
                    protocol: product.protocol,
                    token_symbol: product.token_symbol.clone(),
                    amount: position.supplied_amount,
                    current_value: position.current_value,
                    yield_earned: position.interest_earned,
                    current_apy: product.current_apy,
                    risk_level: product.risk_level,
                    portfolio_percentage,
                });
            }
        }

        for position in &user_vault {
            if let Some(product) = products.get(&position.product_id) {
                let portfolio_percentage = if total_portfolio_value > Decimal::ZERO {
                    (position.current_value / total_portfolio_value * Decimal::from(100)).to_f64().unwrap_or(0.0)
                } else {
                    0.0
                };

                positions.push(PositionSummary {
                    position_id: position.position_id,
                    product_type: product.product_type,
                    protocol: product.protocol,
                    token_symbol: product.token_symbol.clone(),
                    amount: position.deposited_amount,
                    current_value: position.current_value,
                    yield_earned: position.yield_earned,
                    current_apy: product.current_apy,
                    risk_level: product.risk_level,
                    portfolio_percentage,
                });
            }
        }

        // Calculate overall risk level (weighted average)
        let mut total_risk_weight = Decimal::ZERO;
        let mut total_risk_value = Decimal::ZERO;

        for position_summary in &positions {
            let weight = position_summary.current_value;
            total_risk_weight += weight;
            total_risk_value += weight * Decimal::from(position_summary.risk_level as u8);
        }

        let overall_risk_level = if total_risk_weight > Decimal::ZERO {
            let avg_risk = total_risk_value / total_risk_weight;
            match avg_risk.to_u8().unwrap_or(1) {
                1 => RiskLevel::Low,
                2 => RiskLevel::Medium,
                3 => RiskLevel::High,
                _ => RiskLevel::Medium,
            }
        } else {
            RiskLevel::Low
        };

        // Calculate diversification score (0-100)
        let unique_protocols = positions.iter()
            .map(|p| p.protocol)
            .collect::<std::collections::HashSet<_>>()
            .len();
        let unique_product_types = positions.iter()
            .map(|p| p.product_type)
            .collect::<std::collections::HashSet<_>>()
            .len();

        let diversification_score = if positions.is_empty() {
            Decimal::ZERO
        } else {
            // Simple diversification score based on protocol and product type diversity
            let protocol_score = Decimal::from(unique_protocols.min(5) * 10); // Max 50 points
            let product_type_score = Decimal::from(unique_product_types.min(5) * 10); // Max 50 points
            protocol_score + product_type_score
        };

        // Generate recommendations
        let mut recommendations = Vec::new();
        if diversification_score < Decimal::from(50) {
            recommendations.push("Consider diversifying across more protocols and product types".to_string());
        }
        if weighted_average_apy < Decimal::from(3) {
            recommendations.push("Explore higher-yield opportunities to improve returns".to_string());
        }
        if positions.len() > 10 {
            recommendations.push("Consider consolidating positions to reduce management complexity".to_string());
        }

        Ok(PortfolioSummary {
            user_id: *user_id,
            total_portfolio_value,
            total_yield_earned,
            weighted_average_apy,
            positions,
            overall_risk_level,
            diversification_score,
            recommendations,
            last_updated_at: Utc::now(),
        })
    }

    async fn get_yield_chart(
        &self,
        user_id: &Uuid,
        period: &str,
    ) -> Result<YieldChartData, String> {
        // Generate mock yield chart data based on period
        let mut data_points = Vec::new();
        let mut cumulative_yield = Decimal::ZERO;

        let (days, start_date, end_date) = match period {
            "7d" => (7, Utc::now() - chrono::Duration::days(7), Utc::now()),
            "30d" => (30, Utc::now() - chrono::Duration::days(30), Utc::now()),
            "90d" => (90, Utc::now() - chrono::Duration::days(90), Utc::now()),
            "1y" => (365, Utc::now() - chrono::Duration::days(365), Utc::now()),
            _ => (30, Utc::now() - chrono::Duration::days(30), Utc::now()),
        };

        let daily_yield_base = Decimal::from_str("0.15").unwrap(); // Base daily yield
        let base_portfolio_value = Decimal::from_str("10000.0").unwrap();

        for i in 0..days {
            let timestamp = start_date + chrono::Duration::days(i);

            // Add some randomness to the yield (±20%)
            let randomness = Decimal::from_str("0.8").unwrap() +
                (Decimal::from(i % 5) * Decimal::from_str("0.1").unwrap());
            let daily_yield = daily_yield_base * randomness;

            cumulative_yield += daily_yield;
            let portfolio_value = base_portfolio_value + cumulative_yield;

            // Calculate APY based on cumulative performance
            let days_elapsed = i + 1;
            let apy = if days_elapsed > 0 {
                (cumulative_yield / base_portfolio_value) * Decimal::from(365) / Decimal::from(days_elapsed) * Decimal::from(100)
            } else {
                Decimal::from_str("5.0").unwrap()
            };

            data_points.push(YieldDataPoint {
                timestamp,
                yield_amount: daily_yield,
                cumulative_yield,
                apy,
                portfolio_value,
            });
        }

        Ok(YieldChartData {
            data_points,
            total_yield: cumulative_yield,
            period: period.to_string(),
            start_date: start_date.format("%Y-%m-%d").to_string(),
            end_date: end_date.format("%Y-%m-%d").to_string(),
        })
    }

    // Risk and optimization operations
    async fn assess_risk(
        &self,
        user_id: &Uuid,
        product_ids: Vec<Uuid>,
        target_allocation: Option<String>,
    ) -> Result<RiskAssessment, String> {
        let staking_positions = self.staking_positions.read().unwrap();
        let lending_positions = self.lending_positions.read().unwrap();
        let vault_positions = self.vault_positions.read().unwrap();
        let products = self.yield_products.read().unwrap();

        // Get user positions
        let user_staking: Vec<&StakingPosition> = staking_positions.values()
            .filter(|p| p.user_id == *user_id && p.status == PositionStatus::Active)
            .collect();
        let user_lending: Vec<&LendingPosition> = lending_positions.values()
            .filter(|p| p.user_id == *user_id && p.status == PositionStatus::Active)
            .collect();
        let user_vault: Vec<&VaultPosition> = vault_positions.values()
            .filter(|p| p.user_id == *user_id && p.status == PositionStatus::Active)
            .collect();

        // Calculate portfolio metrics
        let total_value = user_staking.iter().map(|p| p.current_value).sum::<Decimal>()
            + user_lending.iter().map(|p| p.current_value).sum::<Decimal>()
            + user_vault.iter().map(|p| p.current_value).sum::<Decimal>();

        // Assess risk factors
        let mut risk_factors = Vec::new();
        let mut warnings = Vec::new();
        let mut recommendations = Vec::new();

        // Protocol concentration risk
        let mut protocol_values = HashMap::new();
        for position in &user_staking {
            if let Some(product) = products.get(&position.product_id) {
                let entry = protocol_values.entry(product.protocol).or_insert(Decimal::ZERO);
                *entry += position.current_value;
            }
        }
        for position in &user_lending {
            if let Some(product) = products.get(&position.product_id) {
                let entry = protocol_values.entry(product.protocol).or_insert(Decimal::ZERO);
                *entry += position.current_value;
            }
        }
        for position in &user_vault {
            if let Some(product) = products.get(&position.product_id) {
                let entry = protocol_values.entry(product.protocol).or_insert(Decimal::ZERO);
                *entry += position.current_value;
            }
        }

        let max_protocol_concentration = if total_value > Decimal::ZERO {
            protocol_values.values().max().copied().unwrap_or(Decimal::ZERO) / total_value * Decimal::from(100)
        } else {
            Decimal::ZERO
        };

        if max_protocol_concentration > Decimal::from(50) {
            risk_factors.push(RiskFactor {
                factor_name: "Protocol Concentration".to_string(),
                risk_level: RiskLevel::High,
                description: "High concentration in a single protocol".to_string(),
                impact_score: Decimal::from(80),
                mitigation: "Diversify across multiple protocols".to_string(),
            });
            warnings.push("Over 50% of portfolio concentrated in single protocol".to_string());
            recommendations.push("Consider diversifying across multiple DeFi protocols".to_string());
        }

        // Smart contract risk
        let unique_protocols = protocol_values.len();
        if unique_protocols < 3 {
            risk_factors.push(RiskFactor {
                factor_name: "Smart Contract Risk".to_string(),
                risk_level: RiskLevel::Medium,
                description: "Limited protocol diversification".to_string(),
                impact_score: Decimal::from(60),
                mitigation: "Increase protocol diversification".to_string(),
            });
        }

        // Liquidity risk
        let staking_percentage = if total_value > Decimal::ZERO {
            user_staking.iter().map(|p| p.current_value).sum::<Decimal>() / total_value * Decimal::from(100)
        } else {
            Decimal::ZERO
        };

        if staking_percentage > Decimal::from(70) {
            risk_factors.push(RiskFactor {
                factor_name: "Liquidity Risk".to_string(),
                risk_level: RiskLevel::Medium,
                description: "High percentage in locked staking positions".to_string(),
                impact_score: Decimal::from(50),
                mitigation: "Balance with more liquid positions".to_string(),
            });
            warnings.push("Over 70% of portfolio in staking positions with lock periods".to_string());
        }

        // Calculate overall risk score (0-100)
        let risk_score = if risk_factors.is_empty() {
            Decimal::from(20) // Low risk baseline
        } else {
            let avg_impact: Decimal = risk_factors.iter()
                .map(|rf| rf.impact_score)
                .sum::<Decimal>() / Decimal::from(risk_factors.len());
            avg_impact.min(Decimal::from(100))
        };

        // Determine overall risk level
        let overall_risk = match risk_score.to_u8().unwrap_or(20) {
            0..=30 => RiskLevel::Low,
            31..=60 => RiskLevel::Medium,
            61..=80 => RiskLevel::High,
            _ => RiskLevel::Critical,
        };

        // Calculate diversification score
        let diversification_score = if total_value > Decimal::ZERO {
            let protocol_diversity = Decimal::from(unique_protocols.min(10) * 10); // Max 100
            let product_type_diversity = Decimal::from(
                (user_staking.len().min(1) + user_lending.len().min(1) + user_vault.len().min(1)) * 20
            ); // Max 60
            (protocol_diversity + product_type_diversity).min(Decimal::from(100))
        } else {
            Decimal::ZERO
        };

        // Concentration risk
        let concentration_risk = max_protocol_concentration;

        // Add general recommendations
        if diversification_score < Decimal::from(50) {
            recommendations.push("Improve diversification across protocols and product types".to_string());
        }
        if risk_score > Decimal::from(70) {
            recommendations.push("Consider reducing exposure to high-risk positions".to_string());
        }

        let mut metadata = HashMap::new();
        metadata.insert("total_positions".to_string(), (user_staking.len() + user_lending.len() + user_vault.len()).to_string());
        metadata.insert("unique_protocols".to_string(), unique_protocols.to_string());
        metadata.insert("max_protocol_concentration".to_string(), max_protocol_concentration.to_string());

        Ok(RiskAssessment {
            overall_risk,
            risk_score,
            risk_factors,
            warnings,
            recommendations,
            diversification_score,
            concentration_risk,
            metadata,
        })
    }

    async fn optimize_portfolio(
        &self,
        user_id: &Uuid,
        target_risk_level: Option<RiskLevel>,
        target_apy: Option<Decimal>,
        max_rebalancing_cost: Option<Decimal>,
        excluded_products: Vec<Uuid>,
    ) -> Result<PortfolioOptimization, String> {
        let staking_positions = self.staking_positions.read().unwrap();
        let lending_positions = self.lending_positions.read().unwrap();
        let vault_positions = self.vault_positions.read().unwrap();
        let products = self.yield_products.read().unwrap();

        // Get user positions
        let user_staking: Vec<&StakingPosition> = staking_positions.values()
            .filter(|p| p.user_id == *user_id && p.status == PositionStatus::Active)
            .collect();
        let user_lending: Vec<&LendingPosition> = lending_positions.values()
            .filter(|p| p.user_id == *user_id && p.status == PositionStatus::Active)
            .collect();
        let user_vault: Vec<&VaultPosition> = vault_positions.values()
            .filter(|p| p.user_id == *user_id && p.status == PositionStatus::Active)
            .collect();

        // Calculate current portfolio metrics
        let total_value = user_staking.iter().map(|p| p.current_value).sum::<Decimal>()
            + user_lending.iter().map(|p| p.current_value).sum::<Decimal>()
            + user_vault.iter().map(|p| p.current_value).sum::<Decimal>();

        // Calculate current weighted APY
        let mut current_weighted_apy = Decimal::ZERO;
        let mut total_weight = Decimal::ZERO;

        for position in &user_staking {
            if let Some(product) = products.get(&position.product_id) {
                let weight = position.current_value;
                current_weighted_apy += product.current_apy * weight;
                total_weight += weight;
            }
        }
        for position in &user_lending {
            if let Some(product) = products.get(&position.product_id) {
                let weight = position.current_value;
                current_weighted_apy += product.current_apy * weight;
                total_weight += weight;
            }
        }
        for position in &user_vault {
            if let Some(product) = products.get(&position.product_id) {
                let weight = position.current_value;
                current_weighted_apy += product.current_apy * weight;
                total_weight += weight;
            }
        }

        let current_apy = if total_weight > Decimal::ZERO {
            current_weighted_apy / total_weight
        } else {
            Decimal::ZERO
        };

        // Set default target risk level if not provided
        let target_risk = target_risk_level.unwrap_or(RiskLevel::Medium);

        // Find optimal products based on criteria
        let available_products: Vec<&YieldProduct> = products.values()
            .filter(|p| {
                p.is_active &&
                !excluded_products.contains(&p.product_id) &&
                (target_risk_level.is_none() || p.risk_level as u8 <= target_risk as u8)
            })
            .collect();

        // Generate optimization suggestions
        let mut suggestions = Vec::new();

        // Suggestion 1: Diversification improvement
        let mut protocol_counts = HashMap::new();
        for position in &user_staking {
            if let Some(product) = products.get(&position.product_id) {
                *protocol_counts.entry(product.protocol).or_insert(0) += 1;
            }
        }
        for position in &user_lending {
            if let Some(product) = products.get(&position.product_id) {
                *protocol_counts.entry(product.protocol).or_insert(0) += 1;
            }
        }
        for position in &user_vault {
            if let Some(product) = products.get(&position.product_id) {
                *protocol_counts.entry(product.protocol).or_insert(0) += 1;
            }
        }

        if protocol_counts.len() < 3 {
            // Find a high-APY product from a different protocol
            if let Some(best_product) = available_products.iter()
                .filter(|p| !protocol_counts.contains_key(&p.protocol))
                .max_by_key(|p| p.current_apy)
            {
                suggestions.push(OptimizationSuggestion {
                    action: "ADD".to_string(),
                    product_id: best_product.product_id,
                    current_allocation: Decimal::ZERO,
                    suggested_allocation: Decimal::from(20), // 20% allocation
                    reason: "Improve protocol diversification".to_string(),
                    expected_impact: Decimal::from_str("0.5").unwrap(), // 0.5% APY improvement
                    priority: 1,
                });
            }
        }

        // Suggestion 2: APY optimization
        if let Some(target_apy_value) = target_apy {
            if current_apy < target_apy_value {
                // Find highest APY product within risk tolerance
                if let Some(high_apy_product) = available_products.iter()
                    .filter(|p| p.current_apy > current_apy)
                    .max_by_key(|p| p.current_apy)
                {
                    suggestions.push(OptimizationSuggestion {
                        action: "INCREASE".to_string(),
                        product_id: high_apy_product.product_id,
                        current_allocation: Decimal::from(10), // Assume current 10%
                        suggested_allocation: Decimal::from(25), // Increase to 25%
                        reason: format!("Increase allocation to higher-yield product ({}% APY)", high_apy_product.current_apy),
                        expected_impact: high_apy_product.current_apy - current_apy,
                        priority: 2,
                    });
                }
            }
        }

        // Suggestion 3: Risk reduction
        if target_risk == RiskLevel::Low {
            // Find positions with high risk and suggest reducing them
            for position in &user_staking {
                if let Some(product) = products.get(&position.product_id) {
                    if product.risk_level == RiskLevel::High {
                        let current_allocation = if total_value > Decimal::ZERO {
                            position.current_value / total_value * Decimal::from(100)
                        } else {
                            Decimal::ZERO
                        };

                        suggestions.push(OptimizationSuggestion {
                            action: "REDUCE".to_string(),
                            product_id: product.product_id,
                            current_allocation,
                            suggested_allocation: current_allocation / Decimal::from(2), // Reduce by half
                            reason: "Reduce high-risk exposure".to_string(),
                            expected_impact: Decimal::from_str("-0.2").unwrap(), // Slight APY reduction for risk reduction
                            priority: 3,
                        });
                    }
                }
            }
        }

        // Calculate optimized APY (simplified estimation)
        let apy_improvement = suggestions.iter()
            .map(|s| s.expected_impact)
            .sum::<Decimal>();
        let optimized_apy = current_apy + apy_improvement;

        // Calculate potential improvement percentage
        let potential_improvement = if current_apy > Decimal::ZERO {
            (apy_improvement / current_apy) * Decimal::from(100)
        } else {
            Decimal::ZERO
        };

        // Estimate rebalancing cost (0.1% of total value)
        let rebalancing_cost = total_value * Decimal::from_str("0.001").unwrap();

        // Calculate expected return improvement (annual)
        let expected_return_improvement = total_value * (apy_improvement / Decimal::from(100));

        Ok(PortfolioOptimization {
            current_apy,
            optimized_apy,
            potential_improvement,
            suggestions,
            target_risk_level: target_risk,
            rebalancing_cost,
            expected_return_improvement,
        })
    }
}
