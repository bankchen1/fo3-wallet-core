//! Trading Guard Middleware
//! 
//! Provides comprehensive security and validation for automated trading operations including:
//! - Risk limit validation and enforcement
//! - Position size limits and portfolio constraints
//! - Trading frequency and velocity limits
//! - Market condition checks and circuit breakers
//! - Fraud detection and suspicious activity monitoring

use std::sync::Arc;
use std::collections::HashMap;
use tonic::{Request, Status};
use tracing::{info, warn, error, instrument};
use chrono::{DateTime, Utc, Duration};
use serde::{Deserialize, Serialize};
use rust_decimal::Decimal;
use tokio::sync::RwLock;

use crate::middleware::{
    auth::{AuthContext, AuthService},
    audit::AuditLogger,
    rate_limit::RateLimiter,
};
use crate::error::ServiceError;

/// Trading guard for automated trading security
pub struct TradingGuard {
    auth_service: Arc<AuthService>,
    audit_logger: Arc<AuditLogger>,
    rate_limiter: Arc<RateLimiter>,
    config: TradingGuardConfig,
    user_limits: Arc<RwLock<HashMap<String, UserTradingLimits>>>,
    active_positions: Arc<RwLock<HashMap<String, Vec<Position>>>>,
    trading_history: Arc<RwLock<HashMap<String, Vec<TradingActivity>>>>,
    market_conditions: Arc<RwLock<MarketConditions>>,
}

/// Trading guard configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingGuardConfig {
    pub max_daily_trades: u32,
    pub max_position_size: Decimal,
    pub max_portfolio_risk: f64,
    pub max_leverage: f64,
    pub min_account_balance: Decimal,
    pub circuit_breaker_threshold: f64,
    pub suspicious_activity_threshold: u32,
    pub cooling_period_minutes: u32,
    pub risk_check_interval_seconds: u64,
}

/// User-specific trading limits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserTradingLimits {
    pub user_id: String,
    pub tier: TradingTier,
    pub daily_trade_limit: u32,
    pub max_position_size: Decimal,
    pub max_portfolio_value: Decimal,
    pub allowed_assets: Vec<String>,
    pub restricted_strategies: Vec<String>,
    pub risk_tolerance: RiskTolerance,
    pub last_updated: DateTime<Utc>,
}

/// Trading tier levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TradingTier {
    Basic,
    Intermediate,
    Advanced,
    Professional,
    Institutional,
}

/// Risk tolerance levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskTolerance {
    Conservative,
    Moderate,
    Aggressive,
    HighRisk,
}

/// Trading position
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub position_id: String,
    pub user_id: String,
    pub asset: String,
    pub side: PositionSide,
    pub size: Decimal,
    pub entry_price: Decimal,
    pub current_price: Decimal,
    pub unrealized_pnl: Decimal,
    pub leverage: f64,
    pub opened_at: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
}

/// Position side
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PositionSide {
    Long,
    Short,
}

/// Trading activity record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingActivity {
    pub activity_id: String,
    pub user_id: String,
    pub activity_type: ActivityType,
    pub asset: String,
    pub amount: Decimal,
    pub price: Decimal,
    pub timestamp: DateTime<Utc>,
    pub strategy_id: Option<String>,
    pub risk_score: f64,
}

/// Activity types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActivityType {
    Buy,
    Sell,
    StrategyStart,
    StrategyStop,
    PositionOpen,
    PositionClose,
    RiskLimitHit,
}

/// Market conditions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketConditions {
    pub volatility_index: f64,
    pub liquidity_index: f64,
    pub market_stress_level: StressLevel,
    pub circuit_breaker_active: bool,
    pub trading_halted: bool,
    pub last_updated: DateTime<Utc>,
}

/// Market stress levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StressLevel {
    Low,
    Medium,
    High,
    Extreme,
}

/// Trading validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingValidationResult {
    pub is_valid: bool,
    pub risk_score: f64,
    pub violations: Vec<RiskViolation>,
    pub warnings: Vec<RiskWarning>,
    pub recommended_adjustments: Vec<String>,
}

/// Risk violation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskViolation {
    pub violation_type: String,
    pub severity: ViolationSeverity,
    pub description: String,
    pub current_value: f64,
    pub limit_value: f64,
    pub action_required: String,
}

/// Risk warning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskWarning {
    pub warning_type: String,
    pub description: String,
    pub risk_level: f64,
    pub recommendation: String,
}

/// Violation severity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ViolationSeverity {
    Low,
    Medium,
    High,
    Critical,
}

impl TradingGuard {
    /// Create a new trading guard
    pub fn new(
        auth_service: Arc<AuthService>,
        audit_logger: Arc<AuditLogger>,
        rate_limiter: Arc<RateLimiter>,
    ) -> Self {
        Self {
            auth_service,
            audit_logger,
            rate_limiter,
            config: TradingGuardConfig::default(),
            user_limits: Arc::new(RwLock::new(HashMap::new())),
            active_positions: Arc::new(RwLock::new(HashMap::new())),
            trading_history: Arc::new(RwLock::new(HashMap::new())),
            market_conditions: Arc::new(RwLock::new(MarketConditions::default())),
        }
    }

    /// Validate trading request
    #[instrument(skip(self, request))]
    pub async fn validate_trading_request<T>(&self, request: &Request<T>) -> Result<TradingValidationResult, Status> {
        // Extract auth context
        let auth_context = self.auth_service.extract_auth_context(request).await
            .map_err(|e| Status::unauthenticated(e.to_string()))?;

        // Check rate limits
        self.check_rate_limits(&auth_context.user_id).await?;

        // Get user trading limits
        let user_limits = self.get_user_limits(&auth_context.user_id).await?;

        // Check market conditions
        let market_conditions = self.market_conditions.read().await;
        if market_conditions.circuit_breaker_active {
            return Ok(TradingValidationResult {
                is_valid: false,
                risk_score: 1.0,
                violations: vec![RiskViolation {
                    violation_type: "circuit_breaker".to_string(),
                    severity: ViolationSeverity::Critical,
                    description: "Market circuit breaker is active".to_string(),
                    current_value: 1.0,
                    limit_value: 0.0,
                    action_required: "Wait for market conditions to normalize".to_string(),
                }],
                warnings: vec![],
                recommended_adjustments: vec!["Suspend all trading activities".to_string()],
            });
        }

        // Validate trading limits
        let validation_result = self.validate_trading_limits(&auth_context.user_id, &user_limits).await?;

        // Log validation attempt
        self.audit_logger.log_trading_validation(
            &auth_context.user_id,
            &validation_result,
            request.remote_addr(),
        ).await;

        Ok(validation_result)
    }

    /// Check rate limits for trading
    async fn check_rate_limits(&self, user_id: &str) -> Result<(), Status> {
        // Check trading frequency
        let rate_key = format!("trading_frequency_{}", user_id);
        self.rate_limiter.check_rate_limit(&rate_key, "100/hour").await
            .map_err(|e| Status::resource_exhausted(format!("Trading rate limit exceeded: {}", e)))?;

        // Check strategy creation frequency
        let strategy_key = format!("strategy_creation_{}", user_id);
        self.rate_limiter.check_rate_limit(&strategy_key, "10/hour").await
            .map_err(|e| Status::resource_exhausted(format!("Strategy creation rate limit exceeded: {}", e)))?;

        Ok(())
    }

    /// Get user trading limits
    async fn get_user_limits(&self, user_id: &str) -> Result<UserTradingLimits, Status> {
        let user_limits = self.user_limits.read().await;
        
        user_limits.get(user_id)
            .cloned()
            .or_else(|| Some(UserTradingLimits::default_for_user(user_id)))
            .ok_or_else(|| Status::internal("Failed to get user trading limits"))
    }

    /// Validate trading limits
    async fn validate_trading_limits(&self, user_id: &str, limits: &UserTradingLimits) -> Result<TradingValidationResult, Status> {
        let mut violations = Vec::new();
        let mut warnings = Vec::new();
        let mut risk_score = 0.0;

        // Check daily trade count
        let daily_trades = self.get_daily_trade_count(user_id).await?;
        if daily_trades >= limits.daily_trade_limit {
            violations.push(RiskViolation {
                violation_type: "daily_trade_limit".to_string(),
                severity: ViolationSeverity::High,
                description: "Daily trade limit exceeded".to_string(),
                current_value: daily_trades as f64,
                limit_value: limits.daily_trade_limit as f64,
                action_required: "Wait until next day or request limit increase".to_string(),
            });
            risk_score += 0.3;
        } else if daily_trades as f32 > limits.daily_trade_limit as f32 * 0.8 {
            warnings.push(RiskWarning {
                warning_type: "approaching_daily_limit".to_string(),
                description: "Approaching daily trade limit".to_string(),
                risk_level: 0.2,
                recommendation: "Consider reducing trading frequency".to_string(),
            });
            risk_score += 0.1;
        }

        // Check portfolio risk
        let portfolio_risk = self.calculate_portfolio_risk(user_id).await?;
        if portfolio_risk > self.config.max_portfolio_risk {
            violations.push(RiskViolation {
                violation_type: "portfolio_risk".to_string(),
                severity: ViolationSeverity::High,
                description: "Portfolio risk exceeds maximum allowed".to_string(),
                current_value: portfolio_risk,
                limit_value: self.config.max_portfolio_risk,
                action_required: "Reduce position sizes or close risky positions".to_string(),
            });
            risk_score += 0.4;
        }

        // Check for suspicious activity
        let suspicious_score = self.check_suspicious_activity(user_id).await?;
        if suspicious_score > 0.7 {
            violations.push(RiskViolation {
                violation_type: "suspicious_activity".to_string(),
                severity: ViolationSeverity::Critical,
                description: "Suspicious trading patterns detected".to_string(),
                current_value: suspicious_score,
                limit_value: 0.7,
                action_required: "Account review required".to_string(),
            });
            risk_score += 0.5;
        }

        let is_valid = violations.is_empty();
        let recommended_adjustments = if !is_valid {
            vec![
                "Reduce position sizes".to_string(),
                "Implement stricter stop losses".to_string(),
                "Diversify trading strategies".to_string(),
            ]
        } else {
            vec![]
        };

        Ok(TradingValidationResult {
            is_valid,
            risk_score: risk_score.min(1.0),
            violations,
            warnings,
            recommended_adjustments,
        })
    }

    /// Get daily trade count for user
    async fn get_daily_trade_count(&self, user_id: &str) -> Result<u32, Status> {
        let history = self.trading_history.read().await;
        let today = Utc::now().date_naive();
        
        let count = history.get(user_id)
            .map(|activities| {
                activities.iter()
                    .filter(|activity| activity.timestamp.date_naive() == today)
                    .count() as u32
            })
            .unwrap_or(0);

        Ok(count)
    }

    /// Calculate portfolio risk
    async fn calculate_portfolio_risk(&self, user_id: &str) -> Result<f64, Status> {
        let positions = self.active_positions.read().await;
        
        let user_positions = positions.get(user_id).unwrap_or(&vec![]);
        
        // Simple risk calculation based on position sizes and leverage
        let total_risk = user_positions.iter()
            .map(|pos| {
                let position_value = pos.size.to_f64().unwrap_or(0.0) * pos.current_price.to_f64().unwrap_or(0.0);
                position_value * pos.leverage * 0.01 // Risk factor
            })
            .sum::<f64>();

        Ok(total_risk)
    }

    /// Check for suspicious activity
    async fn check_suspicious_activity(&self, user_id: &str) -> Result<f64, Status> {
        let history = self.trading_history.read().await;
        let recent_cutoff = Utc::now() - Duration::hours(24);
        
        let recent_activities = history.get(user_id)
            .map(|activities| {
                activities.iter()
                    .filter(|activity| activity.timestamp > recent_cutoff)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        // Calculate suspicion score based on various factors
        let mut suspicion_score = 0.0;

        // High frequency trading
        if recent_activities.len() > 100 {
            suspicion_score += 0.3;
        }

        // Large position changes
        let large_trades = recent_activities.iter()
            .filter(|activity| activity.amount > Decimal::from(10000))
            .count();
        if large_trades > 10 {
            suspicion_score += 0.2;
        }

        // Unusual timing patterns
        let night_trades = recent_activities.iter()
            .filter(|activity| {
                let hour = activity.timestamp.hour();
                hour < 6 || hour > 22
            })
            .count();
        if night_trades > recent_activities.len() / 2 {
            suspicion_score += 0.1;
        }

        Ok(suspicion_score.min(1.0))
    }

    /// Record trading activity
    pub async fn record_trading_activity(&self, activity: TradingActivity) -> Result<(), Status> {
        let mut history = self.trading_history.write().await;
        
        history.entry(activity.user_id.clone())
            .or_insert_with(Vec::new)
            .push(activity.clone());

        // Keep only recent activities (last 30 days)
        let cutoff = Utc::now() - Duration::days(30);
        if let Some(user_activities) = history.get_mut(&activity.user_id) {
            user_activities.retain(|a| a.timestamp > cutoff);
        }

        // Log the activity
        self.audit_logger.log_trading_activity(&activity).await;

        Ok(())
    }

    /// Update market conditions
    pub async fn update_market_conditions(&self, conditions: MarketConditions) -> Result<(), Status> {
        let mut market_conditions = self.market_conditions.write().await;
        *market_conditions = conditions;

        info!(
            volatility = %market_conditions.volatility_index,
            stress_level = ?market_conditions.market_stress_level,
            circuit_breaker = %market_conditions.circuit_breaker_active,
            "Market conditions updated"
        );

        Ok(())
    }

    /// Set user trading limits
    pub async fn set_user_limits(&self, user_id: &str, limits: UserTradingLimits) -> Result<(), Status> {
        let mut user_limits = self.user_limits.write().await;
        user_limits.insert(user_id.to_string(), limits);

        info!(user_id = %user_id, "User trading limits updated");
        Ok(())
    }
}

impl UserTradingLimits {
    fn default_for_user(user_id: &str) -> Self {
        Self {
            user_id: user_id.to_string(),
            tier: TradingTier::Basic,
            daily_trade_limit: 50,
            max_position_size: Decimal::from(1000),
            max_portfolio_value: Decimal::from(10000),
            allowed_assets: vec!["BTC".to_string(), "ETH".to_string(), "USDC".to_string()],
            restricted_strategies: vec![],
            risk_tolerance: RiskTolerance::Conservative,
            last_updated: Utc::now(),
        }
    }
}

impl Default for TradingGuardConfig {
    fn default() -> Self {
        Self {
            max_daily_trades: 100,
            max_position_size: Decimal::from(10000),
            max_portfolio_risk: 0.2, // 20%
            max_leverage: 3.0,
            min_account_balance: Decimal::from(100),
            circuit_breaker_threshold: 0.1, // 10% market drop
            suspicious_activity_threshold: 50,
            cooling_period_minutes: 15,
            risk_check_interval_seconds: 60,
        }
    }
}

impl Default for MarketConditions {
    fn default() -> Self {
        Self {
            volatility_index: 0.2,
            liquidity_index: 0.8,
            market_stress_level: StressLevel::Low,
            circuit_breaker_active: false,
            trading_halted: false,
            last_updated: Utc::now(),
        }
    }
}
