//! Earn service implementation

use std::sync::Arc;
use std::collections::HashMap;
use tonic::{Request, Response, Status};
use uuid::Uuid;
use rust_decimal::Decimal;
use chrono::{DateTime, Utc};

use crate::proto::fo3::wallet::v1::{
    earn_service_server::EarnService,
    *,
};
use serde_json;
use crate::state::AppState;
use crate::middleware::{
    auth::{AuthContext, AuthService},
    audit::AuditLogger,
    earn_guard::EarnGuard,
};
use crate::models::earn::{
    YieldProduct, YieldCalculation, YieldBreakdown, StakingPosition, LendingPosition, VaultPosition,
    EarnAnalytics, PortfolioSummary, PositionSummary, YieldChartData, YieldDataPoint,
    RiskAssessment, RiskFactor, PortfolioOptimization, OptimizationSuggestion,
    YieldProductType, ProtocolType, RiskLevel as EarnRiskLevel, PositionStatus,
    KeyType as EarnKeyType, EarnRepository,
};
use crate::models::notifications::{
    NotificationType, NotificationPriority, DeliveryChannel
};

/// Earn service implementation
#[derive(Debug)]
pub struct EarnServiceImpl {
    state: Arc<AppState>,
    auth_service: Arc<AuthService>,
    audit_logger: Arc<AuditLogger>,
    earn_guard: Arc<EarnGuard>,
    earn_repository: Arc<dyn EarnRepository>,
}

impl EarnServiceImpl {
    pub fn new(
        state: Arc<AppState>,
        auth_service: Arc<AuthService>,
        audit_logger: Arc<AuditLogger>,
        earn_guard: Arc<EarnGuard>,
        earn_repository: Arc<dyn EarnRepository>,
    ) -> Self {
        Self {
            state,
            auth_service,
            audit_logger,
            earn_guard,
            earn_repository,
        }
    }

    /// Convert proto YieldProductType to model YieldProductType
    fn proto_to_model_yield_product_type(proto_type: i32) -> Result<YieldProductType, Status> {
        match YieldProductType::try_from(proto_type) {
            Ok(YieldProductType::YieldProductTypeStaking) => Ok(crate::models::earn::YieldProductType::Staking),
            Ok(YieldProductType::YieldProductTypeLending) => Ok(crate::models::earn::YieldProductType::Lending),
            Ok(YieldProductType::YieldProductTypeVault) => Ok(crate::models::earn::YieldProductType::Vault),
            Ok(YieldProductType::YieldProductTypeLiquidityMining) => Ok(crate::models::earn::YieldProductType::LiquidityMining),
            Ok(YieldProductType::YieldProductTypeFarming) => Ok(crate::models::earn::YieldProductType::Farming),
            _ => Err(Status::invalid_argument("Invalid yield product type")),
        }
    }

    /// Convert model YieldProductType to proto YieldProductType
    fn model_to_proto_yield_product_type(model_type: crate::models::earn::YieldProductType) -> YieldProductType {
        match model_type {
            crate::models::earn::YieldProductType::Staking => YieldProductType::YieldProductTypeStaking,
            crate::models::earn::YieldProductType::Lending => YieldProductType::YieldProductTypeLending,
            crate::models::earn::YieldProductType::Vault => YieldProductType::YieldProductTypeVault,
            crate::models::earn::YieldProductType::LiquidityMining => YieldProductType::YieldProductTypeLiquidityMining,
            crate::models::earn::YieldProductType::Farming => YieldProductType::YieldProductTypeFarming,
        }
    }

    /// Convert proto ProtocolType to model ProtocolType
    fn proto_to_model_protocol_type(proto_type: i32) -> Result<ProtocolType, Status> {
        match ProtocolType::try_from(proto_type) {
            Ok(ProtocolType::ProtocolTypeLido) => Ok(crate::models::earn::ProtocolType::Lido),
            Ok(ProtocolType::ProtocolTypeAave) => Ok(crate::models::earn::ProtocolType::Aave),
            Ok(ProtocolType::ProtocolTypeCompound) => Ok(crate::models::earn::ProtocolType::Compound),
            Ok(ProtocolType::ProtocolTypeYearn) => Ok(crate::models::earn::ProtocolType::Yearn),
            Ok(ProtocolType::ProtocolTypeEigenlayer) => Ok(crate::models::earn::ProtocolType::EigenLayer),
            Ok(ProtocolType::ProtocolTypeMarinade) => Ok(crate::models::earn::ProtocolType::Marinade),
            Ok(ProtocolType::ProtocolTypeRaydium) => Ok(crate::models::earn::ProtocolType::Raydium),
            Ok(ProtocolType::ProtocolTypeOrca) => Ok(crate::models::earn::ProtocolType::Orca),
            _ => Err(Status::invalid_argument("Invalid protocol type")),
        }
    }

    /// Convert model ProtocolType to proto ProtocolType
    fn model_to_proto_protocol_type(model_type: crate::models::earn::ProtocolType) -> ProtocolType {
        match model_type {
            crate::models::earn::ProtocolType::Lido => ProtocolType::ProtocolTypeLido,
            crate::models::earn::ProtocolType::Aave => ProtocolType::ProtocolTypeAave,
            crate::models::earn::ProtocolType::Compound => ProtocolType::ProtocolTypeCompound,
            crate::models::earn::ProtocolType::Yearn => ProtocolType::ProtocolTypeYearn,
            crate::models::earn::ProtocolType::EigenLayer => ProtocolType::ProtocolTypeEigenlayer,
            crate::models::earn::ProtocolType::Marinade => ProtocolType::ProtocolTypeMarinade,
            crate::models::earn::ProtocolType::Raydium => ProtocolType::ProtocolTypeRaydium,
            crate::models::earn::ProtocolType::Orca => ProtocolType::ProtocolTypeOrca,
        }
    }

    /// Convert proto KeyType to model KeyType
    fn proto_to_model_key_type(proto_type: i32) -> Result<EarnKeyType, Status> {
        match KeyType::try_from(proto_type) {
            Ok(KeyType::KeyTypeEthereum) => Ok(EarnKeyType::Ethereum),
            Ok(KeyType::KeyTypeBitcoin) => Ok(EarnKeyType::Bitcoin),
            Ok(KeyType::KeyTypeSolana) => Ok(EarnKeyType::Solana),
            _ => Err(Status::invalid_argument("Invalid key type")),
        }
    }

    /// Convert model KeyType to proto KeyType
    fn model_to_proto_key_type(model_type: EarnKeyType) -> KeyType {
        match model_type {
            EarnKeyType::Ethereum => KeyType::KeyTypeEthereum,
            EarnKeyType::Bitcoin => KeyType::KeyTypeBitcoin,
            EarnKeyType::Solana => KeyType::KeyTypeSolana,
        }
    }

    /// Convert model RiskLevel to proto RiskLevel
    fn model_to_proto_risk_level(model_level: crate::models::earn::RiskLevel) -> RiskLevel {
        match model_level {
            crate::models::earn::RiskLevel::Low => RiskLevel::RiskLevelLow,
            crate::models::earn::RiskLevel::Medium => RiskLevel::RiskLevelMedium,
            crate::models::earn::RiskLevel::High => RiskLevel::RiskLevelHigh,
            crate::models::earn::RiskLevel::Critical => RiskLevel::RiskLevelCritical,
        }
    }

    /// Convert proto RiskLevel to model RiskLevel
    fn proto_to_model_risk_level(proto_level: i32) -> Result<crate::models::earn::RiskLevel, Status> {
        match RiskLevel::try_from(proto_level) {
            Ok(RiskLevel::RiskLevelLow) => Ok(crate::models::earn::RiskLevel::Low),
            Ok(RiskLevel::RiskLevelMedium) => Ok(crate::models::earn::RiskLevel::Medium),
            Ok(RiskLevel::RiskLevelHigh) => Ok(crate::models::earn::RiskLevel::High),
            Ok(RiskLevel::RiskLevelCritical) => Ok(crate::models::earn::RiskLevel::Critical),
            _ => Err(Status::invalid_argument("Invalid risk level")),
        }
    }

    /// Convert model PositionStatus to proto PositionStatus
    fn model_to_proto_position_status(model_status: crate::models::earn::PositionStatus) -> PositionStatus {
        match model_status {
            crate::models::earn::PositionStatus::Active => PositionStatus::PositionStatusActive,
            crate::models::earn::PositionStatus::Pending => PositionStatus::PositionStatusPending,
            crate::models::earn::PositionStatus::Unstaking => PositionStatus::PositionStatusUnstaking,
            crate::models::earn::PositionStatus::Completed => PositionStatus::PositionStatusCompleted,
            crate::models::earn::PositionStatus::Failed => PositionStatus::PositionStatusFailed,
        }
    }

    /// Convert proto PositionStatus to model PositionStatus
    fn proto_to_model_position_status(proto_status: i32) -> Result<crate::models::earn::PositionStatus, Status> {
        match PositionStatus::try_from(proto_status) {
            Ok(PositionStatus::PositionStatusActive) => Ok(crate::models::earn::PositionStatus::Active),
            Ok(PositionStatus::PositionStatusPending) => Ok(crate::models::earn::PositionStatus::Pending),
            Ok(PositionStatus::PositionStatusUnstaking) => Ok(crate::models::earn::PositionStatus::Unstaking),
            Ok(PositionStatus::PositionStatusCompleted) => Ok(crate::models::earn::PositionStatus::Completed),
            Ok(PositionStatus::PositionStatusFailed) => Ok(crate::models::earn::PositionStatus::Failed),
            _ => Err(Status::invalid_argument("Invalid position status")),
        }
    }

    /// Convert model YieldProduct to proto
    fn model_to_proto_yield_product(product: &YieldProduct) -> YieldProduct {
        YieldProduct {
            product_id: product.product_id.to_string(),
            name: product.name.clone(),
            description: product.description.clone(),
            product_type: Self::model_to_proto_yield_product_type(product.product_type) as i32,
            protocol: Self::model_to_proto_protocol_type(product.protocol) as i32,
            chain_type: Self::model_to_proto_key_type(product.chain_type) as i32,
            chain_id: product.chain_id.clone(),
            token_address: product.token_address.clone(),
            token_symbol: product.token_symbol.clone(),
            current_apy: product.current_apy.to_string(),
            historical_apy: product.historical_apy.to_string(),
            tvl: product.tvl.to_string(),
            minimum_deposit: product.minimum_deposit.to_string(),
            maximum_deposit: product.maximum_deposit.to_string(),
            lock_period_days: product.lock_period_days,
            risk_level: Self::model_to_proto_risk_level(product.risk_level) as i32,
            is_active: product.is_active,
            features: product.features.clone(),
            metadata: product.metadata.clone(),
            created_at: product.created_at.timestamp(),
            updated_at: product.updated_at.timestamp(),
        }
    }

    /// Convert model StakingPosition to proto
    fn model_to_proto_staking_position(position: &StakingPosition) -> StakingPosition {
        StakingPosition {
            position_id: position.position_id.to_string(),
            user_id: position.user_id.to_string(),
            product_id: position.product_id.to_string(),
            validator_address: position.validator_address.clone().unwrap_or_default(),
            staked_amount: position.staked_amount.to_string(),
            rewards_earned: position.rewards_earned.to_string(),
            current_value: position.current_value.to_string(),
            status: Self::model_to_proto_position_status(position.status) as i32,
            staked_at: position.staked_at.timestamp(),
            unlock_at: position.unlock_at.map(|dt| dt.timestamp()).unwrap_or(0),
            transaction_hash: position.transaction_hash.clone(),
            metadata: position.metadata.clone(),
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
impl EarnService for EarnServiceImpl {
    /// Get yield products
    async fn get_yield_products(
        &self,
        request: Request<GetYieldProductsRequest>,
    ) -> Result<Response<GetYieldProductsResponse>, Status> {
        let req = request.get_ref();

        // Validate yield product access
        let _auth_context = self.earn_guard
            .validate_yield_product_access(&request, "list")
            .await?;

        // Parse optional filters
        let product_type = if req.product_type != 0 {
            Some(Self::proto_to_model_yield_product_type(req.product_type)?)
        } else {
            None
        };

        let protocol = if req.protocol != 0 {
            Some(Self::proto_to_model_protocol_type(req.protocol)?)
        } else {
            None
        };

        let chain_type = if req.chain_type != 0 {
            Some(Self::proto_to_model_key_type(req.chain_type)?)
        } else {
            None
        };

        let chain_id = if req.chain_id.is_empty() {
            None
        } else {
            Some(req.chain_id.clone())
        };

        let max_risk_level = if req.max_risk_level != 0 {
            Some(Self::proto_to_model_risk_level(req.max_risk_level)?)
        } else {
            None
        };

        let min_apy = if req.min_apy.is_empty() {
            None
        } else {
            Some(req.min_apy.parse::<Decimal>()
                .map_err(|_| Status::invalid_argument("Invalid min_apy format"))?)
        };

        let max_apy = if req.max_apy.is_empty() {
            None
        } else {
            Some(req.max_apy.parse::<Decimal>()
                .map_err(|_| Status::invalid_argument("Invalid max_apy format"))?)
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

        let sort_by = if req.sort_by.is_empty() {
            "apy".to_string()
        } else {
            req.sort_by.clone()
        };

        // Get yield products from repository
        let (products, total_count) = self.earn_repository
            .list_yield_products(
                product_type,
                protocol,
                chain_type,
                chain_id,
                max_risk_level,
                min_apy,
                max_apy,
                req.active_only,
                sort_by,
                req.sort_desc,
                page,
                page_size,
            )
            .await
            .map_err(|e| Status::internal(format!("Failed to list yield products: {}", e)))?;

        // Convert to proto
        let proto_products: Vec<YieldProduct> = products.iter()
            .map(Self::model_to_proto_yield_product)
            .collect();

        // Generate next page token
        let next_page_token = if (page * page_size) < total_count as i32 {
            (page + 1).to_string()
        } else {
            String::new()
        };

        let response = GetYieldProductsResponse {
            products: proto_products,
            next_page_token,
            total_count: total_count as i32,
        };

        Ok(Response::new(response))
    }

    /// Get a specific yield product
    async fn get_yield_product(
        &self,
        request: Request<GetYieldProductRequest>,
    ) -> Result<Response<GetYieldProductResponse>, Status> {
        let req = request.get_ref();

        // Validate yield product access
        let _auth_context = self.earn_guard
            .validate_yield_product_access(&request, "get")
            .await?;

        // Parse product ID
        let product_id = Uuid::parse_str(&req.product_id)
            .map_err(|_| Status::invalid_argument("Invalid product ID"))?;

        // Get yield product from repository
        let product = self.earn_repository
            .get_yield_product(&product_id)
            .await
            .map_err(|e| Status::internal(format!("Failed to get yield product: {}", e)))?
            .ok_or_else(|| Status::not_found("Yield product not found"))?;

        let response = GetYieldProductResponse {
            product: Some(Self::model_to_proto_yield_product(&product)),
        };

        Ok(Response::new(response))
    }

    /// Calculate yield for a product
    async fn calculate_yield(
        &self,
        request: Request<CalculateYieldRequest>,
    ) -> Result<Response<CalculateYieldResponse>, Status> {
        let req = request.get_ref();

        // Validate yield product access
        let _auth_context = self.earn_guard
            .validate_yield_product_access(&request, "calculate")
            .await?;

        // Parse product ID
        let product_id = Uuid::parse_str(&req.product_id)
            .map_err(|_| Status::invalid_argument("Invalid product ID"))?;

        // Parse amount
        let amount = req.amount.parse::<Decimal>()
            .map_err(|_| Status::invalid_argument("Invalid amount format"))?;

        // Get yield product
        let product = self.earn_repository
            .get_yield_product(&product_id)
            .await
            .map_err(|e| Status::internal(format!("Failed to get yield product: {}", e)))?
            .ok_or_else(|| Status::not_found("Yield product not found"))?;

        // Set default time period if not provided (365 days)
        let time_period_days = if req.time_period_days > 0 {
            req.time_period_days
        } else {
            365
        };

        // Calculate yield
        let apy = product.current_apy;
        let daily_rate = apy / Decimal::from(365 * 100); // Convert APY to daily rate

        let mut breakdown = Vec::new();
        let mut cumulative_yield = Decimal::ZERO;

        // Calculate breakdown by periods
        for period in ["daily", "weekly", "monthly", "yearly"] {
            let days_in_period = match period {
                "daily" => 1,
                "weekly" => 7,
                "monthly" => 30,
                "yearly" => 365,
                _ => 1,
            };

            let period_yield = if req.include_compounding {
                // Compound interest calculation
                let rate = daily_rate * Decimal::from(days_in_period);
                amount * rate
            } else {
                // Simple interest calculation
                amount * daily_rate * Decimal::from(days_in_period)
            };

            cumulative_yield += period_yield;

            breakdown.push(YieldBreakdown {
                period: period.to_string(),
                yield_amount: period_yield,
                cumulative_yield,
                apy_at_period: apy,
            });
        }

        // Calculate total yield for the specified period
        let estimated_yield = if req.include_compounding {
            // Compound interest: A = P(1 + r)^t - P
            let rate = apy / Decimal::from(100);
            let time_years = Decimal::from(time_period_days) / Decimal::from(365);
            // Use f64 for compound calculation then convert back
            let rate_f64 = rate.to_f64().unwrap_or(0.0);
            let time_f64 = time_years.to_f64().unwrap_or(1.0);
            let compound_factor = (1.0 + rate_f64).powf(time_f64) - 1.0;
            amount * Decimal::from_f64(compound_factor).unwrap_or(Decimal::ZERO)
        } else {
            // Simple interest: I = P * r * t
            let rate = apy / Decimal::from(100);
            let time_years = Decimal::from(time_period_days) / Decimal::from(365);
            amount * rate * time_years
        };

        // Calculate fees (simplified - 0.5% management fee)
        let fees = estimated_yield * Decimal::from_str("0.005").unwrap();
        let net_yield = estimated_yield - fees;
        let total_return = amount + net_yield;

        let calculation = YieldCalculation {
            principal_amount: amount,
            estimated_yield,
            total_return,
            apy_used: apy,
            time_period_days,
            breakdown,
            fees,
            net_yield,
            metadata: HashMap::new(),
        };

        // Store calculation
        self.earn_repository
            .calculate_yield(&calculation)
            .await
            .map_err(|e| Status::internal(format!("Failed to store yield calculation: {}", e)))?;

        // Convert to proto
        let proto_calculation = YieldCalculation {
            principal_amount: calculation.principal_amount.to_string(),
            estimated_yield: calculation.estimated_yield.to_string(),
            total_return: calculation.total_return.to_string(),
            apy_used: calculation.apy_used.to_string(),
            time_period_days: calculation.time_period_days,
            breakdown: calculation.breakdown.iter().map(|b| YieldBreakdown {
                period: b.period.clone(),
                yield_amount: b.yield_amount.to_string(),
                cumulative_yield: b.cumulative_yield.to_string(),
                apy_at_period: b.apy_at_period.to_string(),
            }).collect(),
            fees: calculation.fees.to_string(),
            net_yield: calculation.net_yield.to_string(),
            metadata: calculation.metadata,
        };

        let response = CalculateYieldResponse {
            calculation: Some(proto_calculation),
        };

        Ok(Response::new(response))
    }

    /// Get yield history for a product
    async fn get_yield_history(
        &self,
        request: Request<GetYieldHistoryRequest>,
    ) -> Result<Response<GetYieldHistoryResponse>, Status> {
        let req = request.get_ref();

        // Validate yield product access
        let _auth_context = self.earn_guard
            .validate_yield_product_access(&request, "history")
            .await?;

        // Parse product ID
        let product_id = Uuid::parse_str(&req.product_id)
            .map_err(|_| Status::invalid_argument("Invalid product ID"))?;

        // Verify product exists
        let _product = self.earn_repository
            .get_yield_product(&product_id)
            .await
            .map_err(|e| Status::internal(format!("Failed to get yield product: {}", e)))?
            .ok_or_else(|| Status::not_found("Yield product not found"))?;

        // Parse date range (default to last 30 days)
        let end_date = if req.end_date > 0 {
            DateTime::from_timestamp(req.end_date, 0)
                .ok_or_else(|| Status::invalid_argument("Invalid end date"))?
        } else {
            Utc::now()
        };

        let start_date = if req.start_date > 0 {
            DateTime::from_timestamp(req.start_date, 0)
                .ok_or_else(|| Status::invalid_argument("Invalid start date"))?
        } else {
            end_date - chrono::Duration::days(30)
        };

        // Validate date range
        if start_date >= end_date {
            return Err(Status::invalid_argument("Start date must be before end date"));
        }

        // Set default period
        let period = if req.period.is_empty() {
            "daily".to_string()
        } else {
            req.period.clone()
        };

        // Validate period
        if !["daily", "weekly", "monthly"].contains(&period.as_str()) {
            return Err(Status::invalid_argument("Invalid period. Must be 'daily', 'weekly', or 'monthly'"));
        }

        // Get yield history from repository
        let history = self.earn_repository
            .get_yield_history(&product_id, start_date, end_date, &period)
            .await
            .map_err(|e| Status::internal(format!("Failed to get yield history: {}", e)))?;

        // Calculate statistics
        let apys: Vec<Decimal> = history.iter().map(|h| h.apy).collect();
        let average_apy = if !apys.is_empty() {
            apys.iter().sum::<Decimal>() / Decimal::from(apys.len())
        } else {
            Decimal::ZERO
        };

        let min_apy = apys.iter().min().copied().unwrap_or(Decimal::ZERO);
        let max_apy = apys.iter().max().copied().unwrap_or(Decimal::ZERO);

        // Convert to proto
        let proto_history: Vec<crate::proto::fo3::wallet::v1::YieldDataPoint> = history.iter()
            .map(|h| crate::proto::fo3::wallet::v1::YieldDataPoint {
                timestamp: h.timestamp.timestamp(),
                yield_amount: h.yield_amount.to_string(),
                cumulative_yield: h.cumulative_yield.to_string(),
                apy: h.apy.to_string(),
                portfolio_value: h.portfolio_value.to_string(),
            })
            .collect();

        // Audit log
        self.audit_logger.log_action(
            &_auth_context.user_id,
            "get_yield_history",
            &format!("Retrieved yield history for product {}", product_id),
            &serde_json::json!({
                "product_id": product_id,
                "start_date": start_date,
                "end_date": end_date,
                "period": period,
                "data_points": history.len()
            }),
        ).await;

        let response = GetYieldHistoryResponse {
            history: proto_history,
            average_apy: average_apy.to_string(),
            min_apy: min_apy.to_string(),
            max_apy: max_apy.to_string(),
        };

        Ok(Response::new(response))
    }

    /// Stake tokens to earn rewards
    async fn stake_tokens(
        &self,
        request: Request<StakeTokensRequest>,
    ) -> Result<Response<StakeTokensResponse>, Status> {
        let req = request.get_ref();

        // Validate staking operation
        let auth_context = self.earn_guard
            .validate_staking_operation(&request, "stake")
            .await?;

        // Parse product ID
        let product_id = Uuid::parse_str(&req.product_id)
            .map_err(|_| Status::invalid_argument("Invalid product ID"))?;

        // Parse amount
        let amount = req.amount.parse::<Decimal>()
            .map_err(|_| Status::invalid_argument("Invalid amount format"))?;

        if amount <= Decimal::ZERO {
            return Err(Status::invalid_argument("Amount must be positive"));
        }

        // Get yield product and validate it's a staking product
        let product = self.earn_repository
            .get_yield_product(&product_id)
            .await
            .map_err(|e| Status::internal(format!("Failed to get yield product: {}", e)))?
            .ok_or_else(|| Status::not_found("Yield product not found"))?;

        if product.product_type != YieldProductType::Staking {
            return Err(Status::invalid_argument("Product is not a staking product"));
        }

        if !product.is_active {
            return Err(Status::failed_precondition("Product is not active"));
        }

        // Validate amount limits
        if amount < product.minimum_deposit {
            return Err(Status::invalid_argument("Amount below minimum deposit"));
        }

        if product.maximum_deposit > Decimal::ZERO && amount > product.maximum_deposit {
            return Err(Status::invalid_argument("Amount exceeds maximum deposit"));
        }

        // Create staking position
        let position_id = Uuid::new_v4();
        let now = Utc::now();
        let unlock_at = if product.lock_period_days > 0 {
            Some(now + chrono::Duration::days(product.lock_period_days))
        } else {
            None
        };

        // Generate mock transaction hash
        let transaction_hash = format!("0x{:x}", uuid::Uuid::new_v4().as_u128());

        let position = StakingPosition {
            position_id,
            user_id: auth_context.user_id,
            product_id,
            validator_address: req.validator_address.clone(),
            staked_amount: amount,
            rewards_earned: Decimal::ZERO,
            current_value: amount,
            status: PositionStatus::Active,
            staked_at: now,
            unlock_at,
            transaction_hash: transaction_hash.clone(),
            metadata: req.metadata.clone(),
        };

        // Store position
        self.earn_repository
            .create_staking_position(&position)
            .await
            .map_err(|e| Status::internal(format!("Failed to create staking position: {}", e)))?;

        // Send notification
        self.send_notification(
            &auth_context.user_id,
            NotificationType::EarnStaking,
            "Staking Successful",
            &format!("Successfully staked {} {} in {}", amount, product.token_symbol, product.name),
        ).await;

        // Audit log
        self.audit_logger.log_action(
            &auth_context.user_id,
            "stake_tokens",
            &format!("Staked {} tokens in product {}", amount, product_id),
            &serde_json::json!({
                "position_id": position_id,
                "product_id": product_id,
                "amount": amount,
                "validator_address": req.validator_address,
                "auto_compound": req.auto_compound,
                "transaction_hash": transaction_hash
            }),
        ).await;

        let response = StakeTokensResponse {
            position: Some(Self::model_to_proto_staking_position(&position)),
            transaction_hash,
        };

        Ok(Response::new(response))
    }

    /// Unstake tokens and claim rewards
    async fn unstake_tokens(
        &self,
        request: Request<UnstakeTokensRequest>,
    ) -> Result<Response<UnstakeTokensResponse>, Status> {
        let req = request.get_ref();

        // Validate staking operation
        let auth_context = self.earn_guard
            .validate_staking_operation(&request, "unstake")
            .await?;

        // Parse position ID
        let position_id = Uuid::parse_str(&req.position_id)
            .map_err(|_| Status::invalid_argument("Invalid position ID"))?;

        // Get staking position
        let mut position = self.earn_repository
            .get_staking_position(&position_id)
            .await
            .map_err(|e| Status::internal(format!("Failed to get staking position: {}", e)))?
            .ok_or_else(|| Status::not_found("Staking position not found"))?;

        // Verify ownership
        if position.user_id != auth_context.user_id {
            return Err(Status::permission_denied("Position does not belong to user"));
        }

        // Check position status
        if position.status != PositionStatus::Active {
            return Err(Status::failed_precondition("Position is not active"));
        }

        // Check lock period
        if let Some(unlock_at) = position.unlock_at {
            if Utc::now() < unlock_at {
                return Err(Status::failed_precondition("Position is still locked"));
            }
        }

        // Parse amount (default to full amount if not specified)
        let unstake_amount = if req.amount.is_empty() {
            position.staked_amount
        } else {
            let amount = req.amount.parse::<Decimal>()
                .map_err(|_| Status::invalid_argument("Invalid amount format"))?;

            if amount <= Decimal::ZERO {
                return Err(Status::invalid_argument("Amount must be positive"));
            }

            if amount > position.staked_amount {
                return Err(Status::invalid_argument("Amount exceeds staked amount"));
            }

            amount
        };

        // Calculate rewards to claim
        let rewards_to_claim = if req.claim_rewards {
            position.rewards_earned
        } else {
            Decimal::ZERO
        };

        // Generate mock transaction hash
        let transaction_hash = format!("0x{:x}", uuid::Uuid::new_v4().as_u128());

        // Update position
        position.staked_amount -= unstake_amount;
        position.current_value = position.staked_amount;

        if req.claim_rewards {
            position.rewards_earned = Decimal::ZERO;
        }

        if position.staked_amount == Decimal::ZERO {
            position.status = PositionStatus::Completed;
        }

        // Update position in repository
        self.earn_repository
            .update_staking_position(&position)
            .await
            .map_err(|e| Status::internal(format!("Failed to update staking position: {}", e)))?;

        // Send notification
        self.send_notification(
            &auth_context.user_id,
            NotificationType::EarnUnstaking,
            "Unstaking Successful",
            &format!("Successfully unstaked {} tokens. Rewards claimed: {}", unstake_amount, rewards_to_claim),
        ).await;

        // Audit log
        self.audit_logger.log_action(
            &auth_context.user_id,
            "unstake_tokens",
            &format!("Unstaked {} tokens from position {}", unstake_amount, position_id),
            &serde_json::json!({
                "position_id": position_id,
                "unstake_amount": unstake_amount,
                "rewards_claimed": rewards_to_claim,
                "claim_rewards": req.claim_rewards,
                "transaction_hash": transaction_hash
            }),
        ).await;

        let response = UnstakeTokensResponse {
            position: Some(Self::model_to_proto_staking_position(&position)),
            transaction_hash,
            rewards_claimed: rewards_to_claim.to_string(),
        };

        Ok(Response::new(response))
    }

    /// Get user staking positions
    async fn get_staking_positions(
        &self,
        request: Request<GetStakingPositionsRequest>,
    ) -> Result<Response<GetStakingPositionsResponse>, Status> {
        let req = request.get_ref();

        // Validate staking operation
        let auth_context = self.earn_guard
            .validate_staking_operation(&request, "list")
            .await?;

        // Determine user ID (admin can view all, users can only view their own)
        let user_id = if req.user_id.is_empty() {
            auth_context.user_id
        } else {
            let requested_user_id = Uuid::parse_str(&req.user_id)
                .map_err(|_| Status::invalid_argument("Invalid user ID"))?;

            // Check if user can view other users' positions (admin only)
            if requested_user_id != auth_context.user_id {
                // Check admin permission
                if !auth_context.roles.contains(&crate::proto::fo3::wallet::v1::UserRole::Admin) {
                    return Err(Status::permission_denied("Cannot view other users' positions"));
                }
            }

            requested_user_id
        };

        // Parse optional filters
        let product_id = if req.product_id.is_empty() {
            None
        } else {
            Some(Uuid::parse_str(&req.product_id)
                .map_err(|_| Status::invalid_argument("Invalid product ID"))?)
        };

        let status = if req.status != 0 {
            Some(Self::proto_to_model_position_status(req.status)?)
        } else {
            None
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

        // Get staking positions from repository
        let (positions, total_count) = self.earn_repository
            .list_staking_positions(user_id, product_id, status, page, page_size)
            .await
            .map_err(|e| Status::internal(format!("Failed to list staking positions: {}", e)))?;

        // Calculate totals
        let total_staked: Decimal = positions.iter().map(|p| p.staked_amount).sum();
        let total_rewards: Decimal = positions.iter().map(|p| p.rewards_earned).sum();

        // Convert to proto
        let proto_positions: Vec<StakingPosition> = positions.iter()
            .map(Self::model_to_proto_staking_position)
            .collect();

        // Generate next page token
        let next_page_token = if (page * page_size) < total_count as i32 {
            (page + 1).to_string()
        } else {
            String::new()
        };

        let response = GetStakingPositionsResponse {
            positions: proto_positions,
            next_page_token,
            total_count: total_count as i32,
            total_staked: total_staked.to_string(),
            total_rewards: total_rewards.to_string(),
        };

        Ok(Response::new(response))
    }

    /// Claim staking rewards
    async fn claim_rewards(
        &self,
        request: Request<ClaimRewardsRequest>,
    ) -> Result<Response<ClaimRewardsResponse>, Status> {
        let req = request.get_ref();

        // Validate staking operation
        let auth_context = self.earn_guard
            .validate_staking_operation(&request, "claim_rewards")
            .await?;

        // Parse position ID
        let position_id = Uuid::parse_str(&req.position_id)
            .map_err(|_| Status::invalid_argument("Invalid position ID"))?;

        // Get staking position
        let mut position = self.earn_repository
            .get_staking_position(&position_id)
            .await
            .map_err(|e| Status::internal(format!("Failed to get staking position: {}", e)))?
            .ok_or_else(|| Status::not_found("Staking position not found"))?;

        // Verify ownership
        if position.user_id != auth_context.user_id {
            return Err(Status::permission_denied("Position does not belong to user"));
        }

        // Check position status
        if position.status != PositionStatus::Active {
            return Err(Status::failed_precondition("Position is not active"));
        }

        // Check if there are rewards to claim
        if position.rewards_earned <= Decimal::ZERO {
            return Err(Status::failed_precondition("No rewards available to claim"));
        }

        let rewards_claimed = position.rewards_earned;

        // Generate mock transaction hash
        let transaction_hash = format!("0x{:x}", uuid::Uuid::new_v4().as_u128());

        // Handle restaking if requested
        if req.restake {
            position.staked_amount += rewards_claimed;
            position.current_value = position.staked_amount;
        }

        // Reset rewards
        position.rewards_earned = Decimal::ZERO;

        // Update position in repository
        self.earn_repository
            .update_staking_position(&position)
            .await
            .map_err(|e| Status::internal(format!("Failed to update staking position: {}", e)))?;

        // Send notification
        let message = if req.restake {
            format!("Successfully claimed and restaked {} rewards", rewards_claimed)
        } else {
            format!("Successfully claimed {} rewards", rewards_claimed)
        };

        self.send_notification(
            &auth_context.user_id,
            NotificationType::EarnRewards,
            "Rewards Claimed",
            &message,
        ).await;

        // Audit log
        self.audit_logger.log_action(
            &auth_context.user_id,
            "claim_rewards",
            &format!("Claimed {} rewards from position {}", rewards_claimed, position_id),
            &serde_json::json!({
                "position_id": position_id,
                "rewards_claimed": rewards_claimed,
                "restake": req.restake,
                "transaction_hash": transaction_hash
            }),
        ).await;

        let response = ClaimRewardsResponse {
            rewards_claimed: rewards_claimed.to_string(),
            transaction_hash,
            updated_position: Some(Self::model_to_proto_staking_position(&position)),
        };

        Ok(Response::new(response))
    }

    /// Supply tokens to lending protocols
    async fn supply_tokens(
        &self,
        request: Request<SupplyTokensRequest>,
    ) -> Result<Response<SupplyTokensResponse>, Status> {
        let req = request.get_ref();

        // Validate lending operation
        let auth_context = self.earn_guard
            .validate_lending_operation(&request, "supply")
            .await?;

        // Parse product ID
        let product_id = Uuid::parse_str(&req.product_id)
            .map_err(|_| Status::invalid_argument("Invalid product ID"))?;

        // Parse amount
        let amount = req.amount.parse::<Decimal>()
            .map_err(|_| Status::invalid_argument("Invalid amount format"))?;

        if amount <= Decimal::ZERO {
            return Err(Status::invalid_argument("Amount must be positive"));
        }

        // Get yield product and validate it's a lending product
        let product = self.earn_repository
            .get_yield_product(&product_id)
            .await
            .map_err(|e| Status::internal(format!("Failed to get yield product: {}", e)))?
            .ok_or_else(|| Status::not_found("Yield product not found"))?;

        if product.product_type != YieldProductType::Lending {
            return Err(Status::invalid_argument("Product is not a lending product"));
        }

        if !product.is_active {
            return Err(Status::failed_precondition("Product is not active"));
        }

        // Validate amount limits
        if amount < product.minimum_deposit {
            return Err(Status::invalid_argument("Amount below minimum deposit"));
        }

        if product.maximum_deposit > Decimal::ZERO && amount > product.maximum_deposit {
            return Err(Status::invalid_argument("Amount exceeds maximum deposit"));
        }

        // Create lending position
        let position_id = Uuid::new_v4();
        let now = Utc::now();

        // Generate mock transaction hash
        let transaction_hash = format!("0x{:x}", uuid::Uuid::new_v4().as_u128());

        let position = LendingPosition {
            position_id,
            user_id: auth_context.user_id,
            product_id,
            supplied_amount: amount,
            interest_earned: Decimal::ZERO,
            current_value: amount,
            supply_apy: product.current_apy,
            status: PositionStatus::Active,
            supplied_at: now,
            transaction_hash: transaction_hash.clone(),
            metadata: req.metadata.clone(),
        };

        // Store position
        self.earn_repository
            .create_lending_position(&position)
            .await
            .map_err(|e| Status::internal(format!("Failed to create lending position: {}", e)))?;

        // Send notification
        self.send_notification(
            &auth_context.user_id,
            NotificationType::EarnLending,
            "Supply Successful",
            &format!("Successfully supplied {} {} to {}", amount, product.token_symbol, product.name),
        ).await;

        // Audit log
        self.audit_logger.log_action(
            &auth_context.user_id,
            "supply_tokens",
            &format!("Supplied {} tokens to product {}", amount, product_id),
            &serde_json::json!({
                "position_id": position_id,
                "product_id": product_id,
                "amount": amount,
                "enable_as_collateral": req.enable_as_collateral,
                "transaction_hash": transaction_hash
            }),
        ).await;

        let response = SupplyTokensResponse {
            position: Some(Self::model_to_proto_lending_position(&position)),
            transaction_hash,
        };

        Ok(Response::new(response))
    }

    /// Withdraw tokens from lending protocols
    async fn withdraw_tokens(
        &self,
        request: Request<WithdrawTokensRequest>,
    ) -> Result<Response<WithdrawTokensResponse>, Status> {
        let req = request.get_ref();

        // Validate lending operation
        let auth_context = self.earn_guard
            .validate_lending_operation(&request, "withdraw")
            .await?;

        // Parse position ID
        let position_id = Uuid::parse_str(&req.position_id)
            .map_err(|_| Status::invalid_argument("Invalid position ID"))?;

        // Get lending position
        let mut position = self.earn_repository
            .get_lending_position(&position_id)
            .await
            .map_err(|e| Status::internal(format!("Failed to get lending position: {}", e)))?
            .ok_or_else(|| Status::not_found("Lending position not found"))?;

        // Verify ownership
        if position.user_id != auth_context.user_id {
            return Err(Status::permission_denied("Position does not belong to user"));
        }

        // Check position status
        if position.status != PositionStatus::Active {
            return Err(Status::failed_precondition("Position is not active"));
        }

        // Parse amount (default to full amount if not specified)
        let withdraw_amount = if req.amount.is_empty() {
            position.supplied_amount
        } else {
            let amount = req.amount.parse::<Decimal>()
                .map_err(|_| Status::invalid_argument("Invalid amount format"))?;

            if amount <= Decimal::ZERO {
                return Err(Status::invalid_argument("Amount must be positive"));
            }

            if amount > position.supplied_amount {
                return Err(Status::invalid_argument("Amount exceeds supplied amount"));
            }

            amount
        };

        // Generate mock transaction hash
        let transaction_hash = format!("0x{:x}", uuid::Uuid::new_v4().as_u128());

        // Update position
        position.supplied_amount -= withdraw_amount;
        position.current_value = position.supplied_amount + position.interest_earned;

        if position.supplied_amount == Decimal::ZERO {
            position.status = PositionStatus::Completed;
        }

        // Update position in repository
        self.earn_repository
            .update_lending_position(&position)
            .await
            .map_err(|e| Status::internal(format!("Failed to update lending position: {}", e)))?;

        // Send notification
        self.send_notification(
            &auth_context.user_id,
            NotificationType::EarnLending,
            "Withdrawal Successful",
            &format!("Successfully withdrew {} tokens from lending position", withdraw_amount),
        ).await;

        // Audit log
        self.audit_logger.log_action(
            &auth_context.user_id,
            "withdraw_tokens",
            &format!("Withdrew {} tokens from position {}", withdraw_amount, position_id),
            &serde_json::json!({
                "position_id": position_id,
                "withdraw_amount": withdraw_amount,
                "transaction_hash": transaction_hash
            }),
        ).await;

        let response = WithdrawTokensResponse {
            position: Some(Self::model_to_proto_lending_position(&position)),
            transaction_hash,
        };

        Ok(Response::new(response))
    }

    /// Get user lending positions
    async fn get_lending_positions(
        &self,
        request: Request<GetLendingPositionsRequest>,
    ) -> Result<Response<GetLendingPositionsResponse>, Status> {
        let req = request.get_ref();

        // Validate lending operation
        let auth_context = self.earn_guard
            .validate_lending_operation(&request, "list")
            .await?;

        // Determine user ID (admin can view all, users can only view their own)
        let user_id = if req.user_id.is_empty() {
            auth_context.user_id
        } else {
            let requested_user_id = Uuid::parse_str(&req.user_id)
                .map_err(|_| Status::invalid_argument("Invalid user ID"))?;

            // Check if user can view other users' positions (admin only)
            if requested_user_id != auth_context.user_id {
                // Check admin permission
                if !auth_context.roles.contains(&crate::proto::fo3::wallet::v1::UserRole::Admin) {
                    return Err(Status::permission_denied("Cannot view other users' positions"));
                }
            }

            requested_user_id
        };

        // Parse optional filters
        let product_id = if req.product_id.is_empty() {
            None
        } else {
            Some(Uuid::parse_str(&req.product_id)
                .map_err(|_| Status::invalid_argument("Invalid product ID"))?)
        };

        let status = if req.status != 0 {
            Some(Self::proto_to_model_position_status(req.status)?)
        } else {
            None
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

        // Get lending positions from repository
        let (positions, total_count) = self.earn_repository
            .list_lending_positions(user_id, product_id, status, page, page_size)
            .await
            .map_err(|e| Status::internal(format!("Failed to list lending positions: {}", e)))?;

        // Calculate totals
        let total_supplied: Decimal = positions.iter().map(|p| p.supplied_amount).sum();
        let total_interest: Decimal = positions.iter().map(|p| p.interest_earned).sum();

        // Convert to proto
        let proto_positions: Vec<LendingPosition> = positions.iter()
            .map(Self::model_to_proto_lending_position)
            .collect();

        // Generate next page token
        let next_page_token = if (page * page_size) < total_count as i32 {
            (page + 1).to_string()
        } else {
            String::new()
        };

        let response = GetLendingPositionsResponse {
            positions: proto_positions,
            next_page_token,
            total_count: total_count as i32,
            total_supplied: total_supplied.to_string(),
            total_interest: total_interest.to_string(),
        };

        Ok(Response::new(response))
    }

    /// Deposit to yield vaults
    async fn deposit_to_vault(
        &self,
        request: Request<DepositToVaultRequest>,
    ) -> Result<Response<DepositToVaultResponse>, Status> {
        let req = request.get_ref();

        // Validate vault operation
        let auth_context = self.earn_guard
            .validate_vault_operation(&request, "deposit")
            .await?;

        // Parse product ID
        let product_id = Uuid::parse_str(&req.product_id)
            .map_err(|_| Status::invalid_argument("Invalid product ID"))?;

        // Parse amount
        let amount = req.amount.parse::<Decimal>()
            .map_err(|_| Status::invalid_argument("Invalid amount format"))?;

        if amount <= Decimal::ZERO {
            return Err(Status::invalid_argument("Amount must be positive"));
        }

        // Get yield product and validate it's a vault product
        let product = self.earn_repository
            .get_yield_product(&product_id)
            .await
            .map_err(|e| Status::internal(format!("Failed to get yield product: {}", e)))?
            .ok_or_else(|| Status::not_found("Yield product not found"))?;

        if product.product_type != YieldProductType::Vault {
            return Err(Status::invalid_argument("Product is not a vault product"));
        }

        if !product.is_active {
            return Err(Status::failed_precondition("Product is not active"));
        }

        // Validate amount limits
        if amount < product.minimum_deposit {
            return Err(Status::invalid_argument("Amount below minimum deposit"));
        }

        if product.maximum_deposit > Decimal::ZERO && amount > product.maximum_deposit {
            return Err(Status::invalid_argument("Amount exceeds maximum deposit"));
        }

        // Calculate shares (simplified 1:1 ratio for mock)
        let shares = amount;

        // Create vault position
        let position_id = Uuid::new_v4();
        let now = Utc::now();

        // Generate mock transaction hash
        let transaction_hash = format!("0x{:x}", uuid::Uuid::new_v4().as_u128());

        let position = VaultPosition {
            position_id,
            user_id: auth_context.user_id,
            product_id,
            deposited_amount: amount,
            shares,
            current_value: amount,
            yield_earned: Decimal::ZERO,
            status: PositionStatus::Active,
            deposited_at: now,
            transaction_hash: transaction_hash.clone(),
            metadata: req.metadata.clone(),
        };

        // Store position
        self.earn_repository
            .create_vault_position(&position)
            .await
            .map_err(|e| Status::internal(format!("Failed to create vault position: {}", e)))?;

        // Send notification
        self.send_notification(
            &auth_context.user_id,
            NotificationType::EarnVault,
            "Vault Deposit Successful",
            &format!("Successfully deposited {} {} to {}", amount, product.token_symbol, product.name),
        ).await;

        // Audit log
        self.audit_logger.log_action(
            &auth_context.user_id,
            "deposit_to_vault",
            &format!("Deposited {} tokens to vault {}", amount, product_id),
            &serde_json::json!({
                "position_id": position_id,
                "product_id": product_id,
                "amount": amount,
                "shares": shares,
                "transaction_hash": transaction_hash
            }),
        ).await;

        let response = DepositToVaultResponse {
            position: Some(Self::model_to_proto_vault_position(&position)),
            transaction_hash,
            shares_received: shares.to_string(),
        };

        Ok(Response::new(response))
    }

    /// Withdraw from yield vaults
    async fn withdraw_from_vault(
        &self,
        request: Request<WithdrawFromVaultRequest>,
    ) -> Result<Response<WithdrawFromVaultResponse>, Status> {
        let req = request.get_ref();

        // Validate vault operation
        let auth_context = self.earn_guard
            .validate_vault_operation(&request, "withdraw")
            .await?;

        // Parse position ID
        let position_id = Uuid::parse_str(&req.position_id)
            .map_err(|_| Status::invalid_argument("Invalid position ID"))?;

        // Get vault position
        let mut position = self.earn_repository
            .get_vault_position(&position_id)
            .await
            .map_err(|e| Status::internal(format!("Failed to get vault position: {}", e)))?
            .ok_or_else(|| Status::not_found("Vault position not found"))?;

        // Verify ownership
        if position.user_id != auth_context.user_id {
            return Err(Status::permission_denied("Position does not belong to user"));
        }

        // Check position status
        if position.status != PositionStatus::Active {
            return Err(Status::failed_precondition("Position is not active"));
        }

        // Parse amount (default to full amount if not specified)
        let (withdraw_amount, shares_to_burn) = if req.amount.is_empty() {
            (position.deposited_amount, position.shares)
        } else {
            let amount = req.amount.parse::<Decimal>()
                .map_err(|_| Status::invalid_argument("Invalid amount format"))?;

            if amount <= Decimal::ZERO {
                return Err(Status::invalid_argument("Amount must be positive"));
            }

            if req.withdraw_by_shares {
                // Withdraw by shares
                if amount > position.shares {
                    return Err(Status::invalid_argument("Amount exceeds available shares"));
                }
                // Calculate corresponding token amount (simplified 1:1 ratio)
                let token_amount = amount;
                (token_amount, amount)
            } else {
                // Withdraw by token amount
                if amount > position.deposited_amount {
                    return Err(Status::invalid_argument("Amount exceeds deposited amount"));
                }
                // Calculate corresponding shares (simplified 1:1 ratio)
                let shares = amount;
                (amount, shares)
            }
        };

        // Generate mock transaction hash
        let transaction_hash = format!("0x{:x}", uuid::Uuid::new_v4().as_u128());

        // Update position
        position.deposited_amount -= withdraw_amount;
        position.shares -= shares_to_burn;
        position.current_value = position.deposited_amount + position.yield_earned;

        if position.deposited_amount == Decimal::ZERO {
            position.status = PositionStatus::Completed;
        }

        // Update position in repository
        self.earn_repository
            .update_vault_position(&position)
            .await
            .map_err(|e| Status::internal(format!("Failed to update vault position: {}", e)))?;

        // Send notification
        self.send_notification(
            &auth_context.user_id,
            NotificationType::EarnVault,
            "Vault Withdrawal Successful",
            &format!("Successfully withdrew {} tokens from vault position", withdraw_amount),
        ).await;

        // Audit log
        self.audit_logger.log_action(
            &auth_context.user_id,
            "withdraw_from_vault",
            &format!("Withdrew {} tokens from vault position {}", withdraw_amount, position_id),
            &serde_json::json!({
                "position_id": position_id,
                "withdraw_amount": withdraw_amount,
                "shares_burned": shares_to_burn,
                "withdraw_by_shares": req.withdraw_by_shares,
                "transaction_hash": transaction_hash
            }),
        ).await;

        let response = WithdrawFromVaultResponse {
            position: Some(Self::model_to_proto_vault_position(&position)),
            transaction_hash,
            amount_withdrawn: withdraw_amount.to_string(),
        };

        Ok(Response::new(response))
    }

    /// Get user vault positions
    async fn get_vault_positions(
        &self,
        request: Request<GetVaultPositionsRequest>,
    ) -> Result<Response<GetVaultPositionsResponse>, Status> {
        let req = request.get_ref();

        // Validate vault operation
        let auth_context = self.earn_guard
            .validate_vault_operation(&request, "list")
            .await?;

        // Determine user ID (admin can view all, users can only view their own)
        let user_id = if req.user_id.is_empty() {
            auth_context.user_id
        } else {
            let requested_user_id = Uuid::parse_str(&req.user_id)
                .map_err(|_| Status::invalid_argument("Invalid user ID"))?;

            // Check if user can view other users' positions (admin only)
            if requested_user_id != auth_context.user_id {
                // Check admin permission
                if !auth_context.roles.contains(&crate::proto::fo3::wallet::v1::UserRole::Admin) {
                    return Err(Status::permission_denied("Cannot view other users' positions"));
                }
            }

            requested_user_id
        };

        // Parse optional filters
        let product_id = if req.product_id.is_empty() {
            None
        } else {
            Some(Uuid::parse_str(&req.product_id)
                .map_err(|_| Status::invalid_argument("Invalid product ID"))?)
        };

        let status = if req.status != 0 {
            Some(Self::proto_to_model_position_status(req.status)?)
        } else {
            None
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

        // Get vault positions from repository
        let (positions, total_count) = self.earn_repository
            .list_vault_positions(user_id, product_id, status, page, page_size)
            .await
            .map_err(|e| Status::internal(format!("Failed to list vault positions: {}", e)))?;

        // Calculate totals
        let total_deposited: Decimal = positions.iter().map(|p| p.deposited_amount).sum();
        let total_yield: Decimal = positions.iter().map(|p| p.yield_earned).sum();

        // Convert to proto
        let proto_positions: Vec<VaultPosition> = positions.iter()
            .map(Self::model_to_proto_vault_position)
            .collect();

        // Generate next page token
        let next_page_token = if (page * page_size) < total_count as i32 {
            (page + 1).to_string()
        } else {
            String::new()
        };

        let response = GetVaultPositionsResponse {
            positions: proto_positions,
            next_page_token,
            total_count: total_count as i32,
            total_deposited: total_deposited.to_string(),
            total_yield: total_yield.to_string(),
        };

        Ok(Response::new(response))
    }

    /// Get user earning analytics
    async fn get_earn_analytics(
        &self,
        request: Request<GetEarnAnalyticsRequest>,
    ) -> Result<Response<GetEarnAnalyticsResponse>, Status> {
        let req = request.get_ref();

        // Validate analytics access
        let auth_context = self.earn_guard
            .validate_analytics_access_simple(&request, "earn_analytics")
            .await?;

        // Determine user ID (admin can view all, users can only view their own)
        let user_id = if req.user_id.is_empty() {
            auth_context.user_id
        } else {
            let requested_user_id = Uuid::parse_str(&req.user_id)
                .map_err(|_| Status::invalid_argument("Invalid user ID"))?;

            // Check if user can view other users' analytics (admin only)
            if requested_user_id != auth_context.user_id {
                // Check admin permission
                if !auth_context.roles.contains(&crate::proto::fo3::wallet::v1::UserRole::Admin) {
                    return Err(Status::permission_denied("Cannot view other users' analytics"));
                }
            }

            requested_user_id
        };

        // Parse date range (default to last 30 days)
        let end_date = if req.end_date > 0 {
            DateTime::from_timestamp(req.end_date, 0)
                .ok_or_else(|| Status::invalid_argument("Invalid end date"))?
        } else {
            Utc::now()
        };

        let start_date = if req.start_date > 0 {
            DateTime::from_timestamp(req.start_date, 0)
                .ok_or_else(|| Status::invalid_argument("Invalid start date"))?
        } else {
            end_date - chrono::Duration::days(30)
        };

        // Validate date range
        if start_date >= end_date {
            return Err(Status::invalid_argument("Start date must be before end date"));
        }

        // Get analytics from repository
        let analytics = self.earn_repository
            .get_earn_analytics(&user_id, start_date, end_date)
            .await
            .map_err(|e| Status::internal(format!("Failed to get earn analytics: {}", e)))?;

        // Convert to proto
        let proto_analytics = crate::proto::fo3::wallet::v1::EarnAnalytics {
            user_id: analytics.user_id.to_string(),
            total_deposited: analytics.total_deposited.to_string(),
            total_earned: analytics.total_earned.to_string(),
            current_value: analytics.current_value.to_string(),
            average_apy: analytics.average_apy.to_string(),
            active_positions: analytics.active_positions,
            product_distribution: analytics.product_distribution.iter()
                .map(|pt| Self::model_to_proto_yield_product_type(*pt) as i32)
                .collect(),
            protocol_distribution: analytics.protocol_distribution.iter()
                .map(|pt| Self::model_to_proto_protocol_type(*pt) as i32)
                .collect(),
            chain_distribution: analytics.chain_distribution.iter()
                .map(|(k, v)| (k.clone(), v.to_string()))
                .collect(),
            best_performing_product: analytics.best_performing_product
                .map(|id| id.to_string())
                .unwrap_or_default(),
            total_fees_paid: analytics.total_fees_paid.to_string(),
            first_deposit_at: analytics.first_deposit_at
                .map(|dt| dt.timestamp())
                .unwrap_or(0),
            last_activity_at: analytics.last_activity_at.timestamp(),
        };

        // Audit log
        self.audit_logger.log_action(
            &auth_context.user_id,
            "get_earn_analytics",
            &format!("Retrieved earn analytics for user {}", user_id),
            &serde_json::json!({
                "target_user_id": user_id,
                "start_date": start_date,
                "end_date": end_date,
                "total_deposited": analytics.total_deposited,
                "total_earned": analytics.total_earned
            }),
        ).await;

        let response = GetEarnAnalyticsResponse {
            analytics: Some(proto_analytics),
        };

        Ok(Response::new(response))
    }

    /// Get portfolio summary
    async fn get_portfolio_summary(
        &self,
        request: Request<GetPortfolioSummaryRequest>,
    ) -> Result<Response<GetPortfolioSummaryResponse>, Status> {
        let req = request.get_ref();

        // Validate analytics access
        let auth_context = self.earn_guard
            .validate_analytics_access_simple(&request, "portfolio_summary")
            .await?;

        // Determine user ID (admin can view all, users can only view their own)
        let user_id = if req.user_id.is_empty() {
            auth_context.user_id
        } else {
            let requested_user_id = Uuid::parse_str(&req.user_id)
                .map_err(|_| Status::invalid_argument("Invalid user ID"))?;

            // Check if user can view other users' portfolio (admin only)
            if requested_user_id != auth_context.user_id {
                // Check admin permission
                if !auth_context.roles.contains(&crate::proto::fo3::wallet::v1::UserRole::Admin) {
                    return Err(Status::permission_denied("Cannot view other users' portfolio"));
                }
            }

            requested_user_id
        };

        // Get portfolio summary from repository
        let summary = self.earn_repository
            .get_portfolio_summary(&user_id)
            .await
            .map_err(|e| Status::internal(format!("Failed to get portfolio summary: {}", e)))?;

        // Convert to proto
        let proto_positions: Vec<crate::proto::fo3::wallet::v1::PositionSummary> = summary.positions.iter()
            .map(|pos| crate::proto::fo3::wallet::v1::PositionSummary {
                position_id: pos.position_id.to_string(),
                product_type: Self::model_to_proto_yield_product_type(pos.product_type) as i32,
                protocol: Self::model_to_proto_protocol_type(pos.protocol) as i32,
                token_symbol: pos.token_symbol.clone(),
                amount: pos.amount.to_string(),
                current_value: pos.current_value.to_string(),
                yield_earned: pos.yield_earned.to_string(),
                current_apy: pos.current_apy.to_string(),
                risk_level: Self::model_to_proto_risk_level(pos.risk_level) as i32,
                portfolio_percentage: pos.portfolio_percentage,
            })
            .collect();

        let proto_summary = crate::proto::fo3::wallet::v1::PortfolioSummary {
            user_id: summary.user_id.to_string(),
            total_portfolio_value: summary.total_portfolio_value.to_string(),
            total_yield_earned: summary.total_yield_earned.to_string(),
            weighted_average_apy: summary.weighted_average_apy.to_string(),
            positions: proto_positions,
            overall_risk_level: Self::model_to_proto_risk_level(summary.overall_risk_level) as i32,
            diversification_score: summary.diversification_score.to_string(),
            recommendations: summary.recommendations,
            last_updated_at: summary.last_updated_at.timestamp(),
        };

        // Audit log
        self.audit_logger.log_action(
            &auth_context.user_id,
            "get_portfolio_summary",
            &format!("Retrieved portfolio summary for user {}", user_id),
            &serde_json::json!({
                "target_user_id": user_id,
                "total_portfolio_value": summary.total_portfolio_value,
                "positions_count": summary.positions.len()
            }),
        ).await;

        let response = GetPortfolioSummaryResponse {
            summary: Some(proto_summary),
        };

        Ok(Response::new(response))
    }

    /// Get yield performance charts
    async fn get_yield_chart(
        &self,
        request: Request<GetYieldChartRequest>,
    ) -> Result<Response<GetYieldChartResponse>, Status> {
        let req = request.get_ref();

        // Validate analytics access
        let auth_context = self.earn_guard
            .validate_analytics_access_simple(&request, "yield_chart")
            .await?;

        // Determine user ID (admin can view all, users can only view their own)
        let user_id = if req.user_id.is_empty() {
            auth_context.user_id
        } else {
            let requested_user_id = Uuid::parse_str(&req.user_id)
                .map_err(|_| Status::invalid_argument("Invalid user ID"))?;

            // Check if user can view other users' charts (admin only)
            if requested_user_id != auth_context.user_id {
                // Check admin permission
                if !auth_context.roles.contains(&crate::proto::fo3::wallet::v1::UserRole::Admin) {
                    return Err(Status::permission_denied("Cannot view other users' yield charts"));
                }
            }

            requested_user_id
        };

        // Set default period
        let period = if req.period.is_empty() {
            "30d".to_string()
        } else {
            req.period.clone()
        };

        // Validate period
        if !["7d", "30d", "90d", "1y"].contains(&period.as_str()) {
            return Err(Status::invalid_argument("Invalid period. Must be '7d', '30d', '90d', or '1y'"));
        }

        // Get yield chart data from repository
        let chart_data = self.earn_repository
            .get_yield_chart(&user_id, &period)
            .await
            .map_err(|e| Status::internal(format!("Failed to get yield chart: {}", e)))?;

        // Convert to proto
        let proto_data_points: Vec<crate::proto::fo3::wallet::v1::YieldDataPoint> = chart_data.data_points.iter()
            .map(|dp| crate::proto::fo3::wallet::v1::YieldDataPoint {
                timestamp: dp.timestamp.timestamp(),
                yield_amount: dp.yield_amount.to_string(),
                cumulative_yield: dp.cumulative_yield.to_string(),
                apy: dp.apy.to_string(),
                portfolio_value: dp.portfolio_value.to_string(),
            })
            .collect();

        let proto_chart = crate::proto::fo3::wallet::v1::YieldChartData {
            data_points: proto_data_points,
            total_yield: chart_data.total_yield.to_string(),
            period: chart_data.period,
            start_date: chart_data.start_date,
            end_date: chart_data.end_date,
        };

        // Audit log
        self.audit_logger.log_action(
            &auth_context.user_id,
            "get_yield_chart",
            &format!("Retrieved yield chart for user {} (period: {})", user_id, period),
            &serde_json::json!({
                "target_user_id": user_id,
                "period": period,
                "data_points": chart_data.data_points.len(),
                "total_yield": chart_data.total_yield
            }),
        ).await;

        let response = GetYieldChartResponse {
            chart_data: Some(proto_chart),
        };

        Ok(Response::new(response))
    }

    /// Assess portfolio risk
    async fn assess_risk(
        &self,
        request: Request<AssessRiskRequest>,
    ) -> Result<Response<AssessRiskResponse>, Status> {
        let req = request.get_ref();

        // Validate risk assessment access
        let auth_context = self.earn_guard
            .validate_risk_assessment(&request, "assess")
            .await?;

        // Parse user ID
        let user_id = Uuid::parse_str(&req.user_id)
            .map_err(|_| Status::invalid_argument("Invalid user ID"))?;

        // Check if user can assess other users' risk (admin only)
        if user_id != auth_context.user_id {
            // Check admin permission
            if !auth_context.roles.contains(&crate::proto::fo3::wallet::v1::UserRole::Admin) {
                return Err(Status::permission_denied("Cannot assess other users' risk"));
            }
        }

        // Parse optional product IDs
        let product_ids: Result<Vec<Uuid>, _> = req.product_ids.iter()
            .map(|id| Uuid::parse_str(id))
            .collect();
        let product_ids = product_ids
            .map_err(|_| Status::invalid_argument("Invalid product ID in list"))?;

        // Parse optional target allocation
        let target_allocation = if req.target_allocation.is_empty() {
            None
        } else {
            Some(req.target_allocation.clone())
        };

        // Get risk assessment from repository
        let assessment = self.earn_repository
            .assess_risk(&user_id, product_ids, target_allocation)
            .await
            .map_err(|e| Status::internal(format!("Failed to assess risk: {}", e)))?;

        // Convert to proto
        let proto_risk_factors: Vec<crate::proto::fo3::wallet::v1::RiskFactor> = assessment.risk_factors.iter()
            .map(|rf| crate::proto::fo3::wallet::v1::RiskFactor {
                factor_name: rf.factor_name.clone(),
                risk_level: Self::model_to_proto_risk_level(rf.risk_level) as i32,
                description: rf.description.clone(),
                impact_score: rf.impact_score.to_string(),
                mitigation: rf.mitigation.clone(),
            })
            .collect();

        let proto_assessment = crate::proto::fo3::wallet::v1::RiskAssessment {
            overall_risk: Self::model_to_proto_risk_level(assessment.overall_risk) as i32,
            risk_score: assessment.risk_score.to_string(),
            risk_factors: proto_risk_factors,
            warnings: assessment.warnings,
            recommendations: assessment.recommendations,
            diversification_score: assessment.diversification_score.to_string(),
            concentration_risk: assessment.concentration_risk.to_string(),
            metadata: assessment.metadata.iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        };

        // Audit log
        self.audit_logger.log_action(
            &auth_context.user_id,
            "assess_risk",
            &format!("Assessed risk for user {}", user_id),
            &serde_json::json!({
                "target_user_id": user_id,
                "product_ids": req.product_ids,
                "overall_risk": assessment.overall_risk,
                "risk_score": assessment.risk_score,
                "warnings_count": assessment.warnings.len()
            }),
        ).await;

        let response = AssessRiskResponse {
            assessment: Some(proto_assessment),
        };

        Ok(Response::new(response))
    }

    /// Optimize portfolio allocation
    async fn optimize_portfolio(
        &self,
        request: Request<OptimizePortfolioRequest>,
    ) -> Result<Response<OptimizePortfolioResponse>, Status> {
        let req = request.get_ref();

        // Validate portfolio optimization access
        let auth_context = self.earn_guard
            .validate_risk_assessment(&request, "optimize")
            .await?;

        // Parse user ID
        let user_id = Uuid::parse_str(&req.user_id)
            .map_err(|_| Status::invalid_argument("Invalid user ID"))?;

        // Check if user can optimize other users' portfolio (admin only)
        if user_id != auth_context.user_id {
            // Check admin permission
            if !auth_context.roles.contains(&crate::proto::fo3::wallet::v1::UserRole::Admin) {
                return Err(Status::permission_denied("Cannot optimize other users' portfolio"));
            }
        }

        // Parse optional parameters
        let target_risk_level = if req.target_risk_level != 0 {
            Some(Self::proto_to_model_risk_level(req.target_risk_level)?)
        } else {
            None
        };

        let target_apy = if req.target_apy.is_empty() {
            None
        } else {
            Some(req.target_apy.parse::<Decimal>()
                .map_err(|_| Status::invalid_argument("Invalid target APY format"))?)
        };

        let max_rebalancing_cost = if req.max_rebalancing_cost.is_empty() {
            None
        } else {
            Some(req.max_rebalancing_cost.parse::<Decimal>()
                .map_err(|_| Status::invalid_argument("Invalid max rebalancing cost format"))?)
        };

        // Parse excluded products
        let excluded_products: Result<Vec<Uuid>, _> = req.excluded_products.iter()
            .map(|id| Uuid::parse_str(id))
            .collect();
        let excluded_products = excluded_products
            .map_err(|_| Status::invalid_argument("Invalid excluded product ID"))?;

        // Get portfolio optimization from repository
        let optimization = self.earn_repository
            .optimize_portfolio(&user_id, target_risk_level, target_apy, max_rebalancing_cost, excluded_products)
            .await
            .map_err(|e| Status::internal(format!("Failed to optimize portfolio: {}", e)))?;

        // Convert to proto
        let proto_suggestions: Vec<crate::proto::fo3::wallet::v1::OptimizationSuggestion> = optimization.suggestions.iter()
            .map(|s| crate::proto::fo3::wallet::v1::OptimizationSuggestion {
                action: s.action.clone(),
                product_id: s.product_id.to_string(),
                current_allocation: s.current_allocation.to_string(),
                suggested_allocation: s.suggested_allocation.to_string(),
                reason: s.reason.clone(),
                expected_impact: s.expected_impact.to_string(),
                priority: s.priority,
            })
            .collect();

        let proto_optimization = crate::proto::fo3::wallet::v1::PortfolioOptimization {
            current_apy: optimization.current_apy.to_string(),
            optimized_apy: optimization.optimized_apy.to_string(),
            potential_improvement: optimization.potential_improvement.to_string(),
            suggestions: proto_suggestions,
            target_risk_level: Self::model_to_proto_risk_level(optimization.target_risk_level) as i32,
            rebalancing_cost: optimization.rebalancing_cost.to_string(),
            expected_return_improvement: optimization.expected_return_improvement.to_string(),
        };

        // Audit log
        self.audit_logger.log_action(
            &auth_context.user_id,
            "optimize_portfolio",
            &format!("Optimized portfolio for user {}", user_id),
            &serde_json::json!({
                "target_user_id": user_id,
                "target_risk_level": req.target_risk_level,
                "target_apy": req.target_apy,
                "current_apy": optimization.current_apy,
                "optimized_apy": optimization.optimized_apy,
                "suggestions_count": optimization.suggestions.len()
            }),
        ).await;

        let response = OptimizePortfolioResponse {
            optimization: Some(proto_optimization),
        };

        Ok(Response::new(response))
    }

    // Proto conversion helper methods
    fn model_to_proto_staking_position(position: &StakingPosition) -> crate::proto::fo3::wallet::v1::StakingPosition {
        crate::proto::fo3::wallet::v1::StakingPosition {
            position_id: position.position_id.to_string(),
            user_id: position.user_id.to_string(),
            product_id: position.product_id.to_string(),
            validator_address: position.validator_address.clone(),
            staked_amount: position.staked_amount.to_string(),
            rewards_earned: position.rewards_earned.to_string(),
            current_value: position.current_value.to_string(),
            status: Self::model_to_proto_position_status(position.status) as i32,
            staked_at: position.staked_at.timestamp(),
            unlock_at: position.unlock_at.map(|dt| dt.timestamp()).unwrap_or(0),
            transaction_hash: position.transaction_hash.clone(),
            metadata: position.metadata.clone(),
        }
    }

    fn model_to_proto_lending_position(position: &LendingPosition) -> crate::proto::fo3::wallet::v1::LendingPosition {
        crate::proto::fo3::wallet::v1::LendingPosition {
            position_id: position.position_id.to_string(),
            user_id: position.user_id.to_string(),
            product_id: position.product_id.to_string(),
            supplied_amount: position.supplied_amount.to_string(),
            interest_earned: position.interest_earned.to_string(),
            current_value: position.current_value.to_string(),
            supply_apy: position.supply_apy.to_string(),
            status: Self::model_to_proto_position_status(position.status) as i32,
            supplied_at: position.supplied_at.timestamp(),
            transaction_hash: position.transaction_hash.clone(),
            metadata: position.metadata.clone(),
        }
    }

    fn model_to_proto_vault_position(position: &VaultPosition) -> crate::proto::fo3::wallet::v1::VaultPosition {
        crate::proto::fo3::wallet::v1::VaultPosition {
            position_id: position.position_id.to_string(),
            user_id: position.user_id.to_string(),
            product_id: position.product_id.to_string(),
            deposited_amount: position.deposited_amount.to_string(),
            shares: position.shares.to_string(),
            current_value: position.current_value.to_string(),
            yield_earned: position.yield_earned.to_string(),
            status: Self::model_to_proto_position_status(position.status) as i32,
            deposited_at: position.deposited_at.timestamp(),
            transaction_hash: position.transaction_hash.clone(),
            metadata: position.metadata.clone(),
        }
    }

    fn model_to_proto_position_status(status: PositionStatus) -> crate::proto::fo3::wallet::v1::PositionStatus {
        match status {
            PositionStatus::Active => crate::proto::fo3::wallet::v1::PositionStatus::Active,
            PositionStatus::Completed => crate::proto::fo3::wallet::v1::PositionStatus::Completed,
            PositionStatus::Cancelled => crate::proto::fo3::wallet::v1::PositionStatus::Cancelled,
            PositionStatus::Failed => crate::proto::fo3::wallet::v1::PositionStatus::Failed,
        }
    }

    fn proto_to_model_position_status(status: i32) -> Result<PositionStatus, Status> {
        match status {
            1 => Ok(PositionStatus::Active),
            2 => Ok(PositionStatus::Completed),
            3 => Ok(PositionStatus::Cancelled),
            4 => Ok(PositionStatus::Failed),
            _ => Err(Status::invalid_argument("Invalid position status")),
        }
    }

    fn model_to_proto_yield_product_type(product_type: YieldProductType) -> crate::proto::fo3::wallet::v1::YieldProductType {
        match product_type {
            YieldProductType::Staking => crate::proto::fo3::wallet::v1::YieldProductType::Staking,
            YieldProductType::Lending => crate::proto::fo3::wallet::v1::YieldProductType::Lending,
            YieldProductType::Vault => crate::proto::fo3::wallet::v1::YieldProductType::Vault,
            YieldProductType::LiquidityMining => crate::proto::fo3::wallet::v1::YieldProductType::LiquidityMining,
        }
    }

    fn model_to_proto_protocol_type(protocol: ProtocolType) -> crate::proto::fo3::wallet::v1::ProtocolType {
        match protocol {
            ProtocolType::Aave => crate::proto::fo3::wallet::v1::ProtocolType::Aave,
            ProtocolType::Compound => crate::proto::fo3::wallet::v1::ProtocolType::Compound,
            ProtocolType::Uniswap => crate::proto::fo3::wallet::v1::ProtocolType::Uniswap,
            ProtocolType::Curve => crate::proto::fo3::wallet::v1::ProtocolType::Curve,
            ProtocolType::Yearn => crate::proto::fo3::wallet::v1::ProtocolType::Yearn,
            ProtocolType::Convex => crate::proto::fo3::wallet::v1::ProtocolType::Convex,
            ProtocolType::Lido => crate::proto::fo3::wallet::v1::ProtocolType::Lido,
            ProtocolType::RocketPool => crate::proto::fo3::wallet::v1::ProtocolType::RocketPool,
        }
    }

    fn model_to_proto_risk_level(risk_level: RiskLevel) -> crate::proto::fo3::wallet::v1::RiskLevel {
        match risk_level {
            RiskLevel::Low => crate::proto::fo3::wallet::v1::RiskLevel::Low,
            RiskLevel::Medium => crate::proto::fo3::wallet::v1::RiskLevel::Medium,
            RiskLevel::High => crate::proto::fo3::wallet::v1::RiskLevel::High,
            RiskLevel::Critical => crate::proto::fo3::wallet::v1::RiskLevel::Critical,
        }
    }

    fn proto_to_model_risk_level(risk_level: i32) -> Result<RiskLevel, Status> {
        match risk_level {
            1 => Ok(RiskLevel::Low),
            2 => Ok(RiskLevel::Medium),
            3 => Ok(RiskLevel::High),
            4 => Ok(RiskLevel::Critical),
            _ => Err(Status::invalid_argument("Invalid risk level")),
        }
    }

}
