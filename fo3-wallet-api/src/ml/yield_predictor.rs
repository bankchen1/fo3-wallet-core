//! Yield Prediction ML Service
//! 
//! Provides advanced yield prediction capabilities including:
//! - DeFi protocol yield forecasting
//! - Risk-adjusted return predictions
//! - Portfolio optimization recommendations
//! - Market condition impact analysis
//! - Liquidity mining opportunity detection

use std::sync::Arc;
use anyhow::Result;
use tracing::{info, warn, error, instrument};
use chrono::{DateTime, Utc, Duration};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use super::{
    InferenceRequest, InferenceResponse, MLService, MLError, MLResult,
    ModelMetadata, MarketDataPoint, TechnicalIndicators
};

/// Yield prediction service
pub struct YieldPredictor {
    model_path: String,
    model: Arc<RwLock<Option<YieldModel>>>,
    config: YieldPredictionConfig,
    historical_data: Arc<RwLock<Vec<YieldDataPoint>>>,
}

/// Yield prediction configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YieldPredictionConfig {
    pub prediction_horizon_days: u32,
    pub min_confidence_threshold: f64,
    pub risk_free_rate: f64,
    pub volatility_window_days: u32,
    pub feature_window_days: u32,
    pub supported_protocols: Vec<String>,
}

/// Yield prediction model
struct YieldModel {
    // In a real implementation, this would contain the actual ML model
    model_version: String,
    feature_count: usize,
}

/// Historical yield data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YieldDataPoint {
    pub protocol: String,
    pub asset: String,
    pub apy: f64,
    pub tvl: f64,
    pub volume_24h: f64,
    pub risk_score: f64,
    pub timestamp: DateTime<Utc>,
    pub market_conditions: MarketConditions,
}

/// Market conditions for yield prediction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketConditions {
    pub volatility_index: f64,
    pub liquidity_index: f64,
    pub sentiment_score: f64,
    pub macro_trend: String,
    pub defi_tvl_trend: f64,
}

/// Yield prediction result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YieldPredictionResult {
    pub protocol: String,
    pub asset: String,
    pub current_apy: f64,
    pub predicted_apy: f64,
    pub prediction_confidence: f64,
    pub risk_adjusted_return: f64,
    pub volatility_forecast: f64,
    pub time_horizon_days: u32,
    pub risk_factors: Vec<RiskFactor>,
    pub opportunity_score: f64,
    pub recommendation: YieldRecommendation,
}

/// Risk factor analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskFactor {
    pub factor_type: String,
    pub impact_score: f64,
    pub description: String,
    pub mitigation_strategy: String,
}

/// Yield recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YieldRecommendation {
    pub action: String, // "enter", "exit", "hold", "reduce", "increase"
    pub confidence: f64,
    pub reasoning: String,
    pub optimal_allocation: f64,
    pub entry_conditions: Vec<String>,
    pub exit_conditions: Vec<String>,
}

/// Portfolio optimization result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioOptimizationResult {
    pub current_portfolio: Vec<PortfolioPosition>,
    pub optimized_portfolio: Vec<PortfolioPosition>,
    pub expected_improvement: f64,
    pub risk_reduction: f64,
    pub rebalancing_actions: Vec<RebalancingAction>,
    pub optimization_score: f64,
}

/// Portfolio position
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioPosition {
    pub protocol: String,
    pub asset: String,
    pub allocation_percentage: f64,
    pub expected_apy: f64,
    pub risk_score: f64,
    pub liquidity_score: f64,
}

/// Rebalancing action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RebalancingAction {
    pub action_type: String, // "add", "remove", "rebalance"
    pub from_protocol: Option<String>,
    pub to_protocol: String,
    pub asset: String,
    pub amount_percentage: f64,
    pub expected_impact: f64,
    pub priority: u32,
}

impl YieldPredictor {
    /// Create a new yield predictor
    pub async fn new(model_path: &str) -> MLResult<Self> {
        let config = YieldPredictionConfig::default();
        
        let predictor = Self {
            model_path: model_path.to_string(),
            model: Arc::new(RwLock::new(None)),
            config,
            historical_data: Arc::new(RwLock::new(Vec::new())),
        };

        // Load model
        predictor.load_model().await?;
        
        // Initialize with historical data
        predictor.load_historical_data().await?;
        
        Ok(predictor)
    }

    /// Load the yield prediction model
    #[instrument(skip(self))]
    async fn load_model(&self) -> MLResult<()> {
        info!(model_path = %self.model_path, "Loading yield prediction model");

        // In a real implementation, this would load actual ML models
        let model = YieldModel {
            model_version: "1.0.0".to_string(),
            feature_count: 50,
        };

        {
            let mut model_lock = self.model.write().await;
            *model_lock = Some(model);
        }

        info!("Yield prediction model loaded successfully");
        Ok(())
    }

    /// Load historical yield data
    async fn load_historical_data(&self) -> MLResult<()> {
        // In a real implementation, this would load from database
        let mut historical_data = self.historical_data.write().await;
        
        // Generate mock historical data
        let protocols = vec!["Aave", "Compound", "Uniswap V3", "Curve", "Yearn"];
        let assets = vec!["USDC", "USDT", "DAI", "ETH", "WBTC"];
        
        for i in 0..100 {
            let timestamp = Utc::now() - Duration::days(i);
            for protocol in &protocols {
                for asset in &assets {
                    historical_data.push(YieldDataPoint {
                        protocol: protocol.clone(),
                        asset: asset.clone(),
                        apy: 5.0 + (i as f64 * 0.1) % 15.0,
                        tvl: 1000000.0 + (i as f64 * 10000.0),
                        volume_24h: 50000.0 + (i as f64 * 1000.0),
                        risk_score: 0.1 + (i as f64 * 0.01) % 0.8,
                        timestamp,
                        market_conditions: MarketConditions {
                            volatility_index: 0.3 + (i as f64 * 0.01) % 0.4,
                            liquidity_index: 0.7 + (i as f64 * 0.005) % 0.3,
                            sentiment_score: 0.5 + (i as f64 * 0.02) % 0.5,
                            macro_trend: "bullish".to_string(),
                            defi_tvl_trend: 1.05 + (i as f64 * 0.001) % 0.1,
                        },
                    });
                }
            }
        }
        
        Ok(())
    }

    /// Predict yield for a specific protocol and asset
    #[instrument(skip(self))]
    pub async fn predict_yield(&self, protocol: &str, asset: &str, time_horizon_days: u32) -> MLResult<YieldPredictionResult> {
        info!(protocol = %protocol, asset = %asset, time_horizon_days = %time_horizon_days, "Predicting yield");

        // Extract features
        let features = self.extract_features(protocol, asset).await?;
        
        // Run prediction
        let prediction = self.run_yield_prediction(&features, time_horizon_days).await?;
        
        // Calculate risk factors
        let risk_factors = self.analyze_risk_factors(protocol, asset).await?;
        
        // Generate recommendation
        let recommendation = self.generate_recommendation(&prediction, &risk_factors).await?;
        
        Ok(YieldPredictionResult {
            protocol: protocol.to_string(),
            asset: asset.to_string(),
            current_apy: prediction.current_apy,
            predicted_apy: prediction.predicted_apy,
            prediction_confidence: prediction.confidence,
            risk_adjusted_return: prediction.risk_adjusted_return,
            volatility_forecast: prediction.volatility_forecast,
            time_horizon_days,
            risk_factors,
            opportunity_score: prediction.opportunity_score,
            recommendation,
        })
    }

    /// Optimize portfolio allocation
    #[instrument(skip(self, current_positions))]
    pub async fn optimize_portfolio(&self, current_positions: Vec<PortfolioPosition>, risk_tolerance: f64) -> MLResult<PortfolioOptimizationResult> {
        info!(positions_count = %current_positions.len(), risk_tolerance = %risk_tolerance, "Optimizing portfolio");

        // Analyze current portfolio
        let current_metrics = self.calculate_portfolio_metrics(&current_positions).await?;
        
        // Generate optimization candidates
        let candidates = self.generate_optimization_candidates(&current_positions, risk_tolerance).await?;
        
        // Select optimal allocation
        let optimized_portfolio = self.select_optimal_allocation(candidates, risk_tolerance).await?;
        
        // Calculate improvement metrics
        let optimized_metrics = self.calculate_portfolio_metrics(&optimized_portfolio).await?;
        
        // Generate rebalancing actions
        let rebalancing_actions = self.generate_rebalancing_actions(&current_positions, &optimized_portfolio).await?;
        
        Ok(PortfolioOptimizationResult {
            current_portfolio: current_positions,
            optimized_portfolio,
            expected_improvement: optimized_metrics.expected_return - current_metrics.expected_return,
            risk_reduction: current_metrics.risk_score - optimized_metrics.risk_score,
            rebalancing_actions,
            optimization_score: optimized_metrics.sharpe_ratio,
        })
    }

    /// Extract features for yield prediction
    async fn extract_features(&self, protocol: &str, asset: &str) -> MLResult<Vec<f64>> {
        let historical_data = self.historical_data.read().await;
        
        // Filter data for specific protocol and asset
        let relevant_data: Vec<_> = historical_data.iter()
            .filter(|d| d.protocol == protocol && d.asset == asset)
            .take(self.config.feature_window_days as usize)
            .collect();
        
        if relevant_data.is_empty() {
            return Err(MLError::FeatureExtractionFailed { 
                reason: "No historical data available".to_string() 
            });
        }

        let mut features = Vec::new();
        
        // APY statistics
        let apys: Vec<f64> = relevant_data.iter().map(|d| d.apy).collect();
        features.push(apys.iter().sum::<f64>() / apys.len() as f64); // mean APY
        features.push(self.calculate_volatility(&apys)); // APY volatility
        features.push(*apys.iter().max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap()); // max APY
        features.push(*apys.iter().min_by(|a, b| a.partial_cmp(b).unwrap()).unwrap()); // min APY
        
        // TVL statistics
        let tvls: Vec<f64> = relevant_data.iter().map(|d| d.tvl).collect();
        features.push(tvls.iter().sum::<f64>() / tvls.len() as f64); // mean TVL
        features.push(self.calculate_trend(&tvls)); // TVL trend
        
        // Volume statistics
        let volumes: Vec<f64> = relevant_data.iter().map(|d| d.volume_24h).collect();
        features.push(volumes.iter().sum::<f64>() / volumes.len() as f64); // mean volume
        
        // Risk statistics
        let risks: Vec<f64> = relevant_data.iter().map(|d| d.risk_score).collect();
        features.push(risks.iter().sum::<f64>() / risks.len() as f64); // mean risk
        
        // Market condition features
        if let Some(latest) = relevant_data.first() {
            features.push(latest.market_conditions.volatility_index);
            features.push(latest.market_conditions.liquidity_index);
            features.push(latest.market_conditions.sentiment_score);
            features.push(latest.market_conditions.defi_tvl_trend);
        }
        
        Ok(features)
    }

    /// Run yield prediction using the ML model
    async fn run_yield_prediction(&self, features: &[f64], time_horizon_days: u32) -> MLResult<PredictionOutput> {
        // In a real implementation, this would use the actual ML model
        // For now, generate realistic predictions based on features
        
        let current_apy = features[0]; // mean APY from features
        let volatility = features[1];
        let trend = features[5]; // TVL trend
        
        // Simple prediction logic (would be replaced by actual ML model)
        let trend_factor = if trend > 0.0 { 1.05 } else { 0.95 };
        let volatility_factor = 1.0 - (volatility * 0.1);
        let time_decay = 1.0 - (time_horizon_days as f64 * 0.001);
        
        let predicted_apy = current_apy * trend_factor * volatility_factor * time_decay;
        let confidence = 0.8 - (volatility * 0.2);
        let risk_adjusted_return = predicted_apy - (volatility * 2.0);
        let volatility_forecast = volatility * 1.1; // Slightly higher volatility forecast
        let opportunity_score = (predicted_apy / current_apy - 1.0) * confidence;
        
        Ok(PredictionOutput {
            current_apy,
            predicted_apy,
            confidence,
            risk_adjusted_return,
            volatility_forecast,
            opportunity_score,
        })
    }

    /// Analyze risk factors
    async fn analyze_risk_factors(&self, protocol: &str, asset: &str) -> MLResult<Vec<RiskFactor>> {
        let mut risk_factors = Vec::new();
        
        // Smart contract risk
        risk_factors.push(RiskFactor {
            factor_type: "Smart Contract Risk".to_string(),
            impact_score: 0.3,
            description: "Risk of smart contract vulnerabilities or exploits".to_string(),
            mitigation_strategy: "Diversify across multiple protocols".to_string(),
        });
        
        // Liquidity risk
        risk_factors.push(RiskFactor {
            factor_type: "Liquidity Risk".to_string(),
            impact_score: 0.2,
            description: "Risk of insufficient liquidity for withdrawals".to_string(),
            mitigation_strategy: "Monitor TVL and volume trends".to_string(),
        });
        
        // Market risk
        risk_factors.push(RiskFactor {
            factor_type: "Market Risk".to_string(),
            impact_score: 0.4,
            description: "Risk from overall market volatility".to_string(),
            mitigation_strategy: "Use stop-loss and position sizing".to_string(),
        });
        
        // Protocol-specific risks
        match protocol {
            "Aave" => {
                risk_factors.push(RiskFactor {
                    factor_type: "Liquidation Risk".to_string(),
                    impact_score: 0.25,
                    description: "Risk of collateral liquidation".to_string(),
                    mitigation_strategy: "Maintain healthy collateral ratio".to_string(),
                });
            },
            "Uniswap V3" => {
                risk_factors.push(RiskFactor {
                    factor_type: "Impermanent Loss".to_string(),
                    impact_score: 0.35,
                    description: "Risk from price divergence in LP positions".to_string(),
                    mitigation_strategy: "Use concentrated liquidity strategies".to_string(),
                });
            },
            _ => {}
        }
        
        Ok(risk_factors)
    }

    /// Generate yield recommendation
    async fn generate_recommendation(&self, prediction: &PredictionOutput, risk_factors: &[RiskFactor]) -> MLResult<YieldRecommendation> {
        let total_risk = risk_factors.iter().map(|r| r.impact_score).sum::<f64>() / risk_factors.len() as f64;
        let yield_improvement = (prediction.predicted_apy / prediction.current_apy - 1.0) * 100.0;
        
        let (action, confidence, reasoning) = if yield_improvement > 10.0 && total_risk < 0.3 {
            ("enter", 0.8, "Strong yield opportunity with manageable risk")
        } else if yield_improvement > 5.0 && total_risk < 0.4 {
            ("increase", 0.7, "Moderate yield opportunity, consider increasing allocation")
        } else if yield_improvement < -5.0 || total_risk > 0.6 {
            ("exit", 0.75, "Declining yield or high risk, consider exiting")
        } else if yield_improvement < -2.0 {
            ("reduce", 0.6, "Slight decline expected, consider reducing position")
        } else {
            ("hold", 0.5, "Stable conditions, maintain current position")
        };
        
        Ok(YieldRecommendation {
            action: action.to_string(),
            confidence,
            reasoning: reasoning.to_string(),
            optimal_allocation: if action == "enter" { 0.2 } else if action == "increase" { 0.15 } else { 0.1 },
            entry_conditions: vec![
                "TVL trend positive".to_string(),
                "Volatility below 30%".to_string(),
                "Protocol audit recent".to_string(),
            ],
            exit_conditions: vec![
                "APY drops below 3%".to_string(),
                "TVL decreases by 20%".to_string(),
                "Security incident reported".to_string(),
            ],
        })
    }

    /// Calculate portfolio metrics
    async fn calculate_portfolio_metrics(&self, positions: &[PortfolioPosition]) -> MLResult<PortfolioMetrics> {
        let total_allocation: f64 = positions.iter().map(|p| p.allocation_percentage).sum();
        let expected_return: f64 = positions.iter()
            .map(|p| p.expected_apy * p.allocation_percentage / 100.0)
            .sum();
        let risk_score: f64 = positions.iter()
            .map(|p| p.risk_score * p.allocation_percentage / 100.0)
            .sum();
        let liquidity_score: f64 = positions.iter()
            .map(|p| p.liquidity_score * p.allocation_percentage / 100.0)
            .sum();
        
        let sharpe_ratio = if risk_score > 0.0 {
            (expected_return - self.config.risk_free_rate) / risk_score
        } else {
            0.0
        };
        
        Ok(PortfolioMetrics {
            expected_return,
            risk_score,
            liquidity_score,
            sharpe_ratio,
            diversification_score: self.calculate_diversification_score(positions),
        })
    }

    /// Generate optimization candidates
    async fn generate_optimization_candidates(&self, current_positions: &[PortfolioPosition], risk_tolerance: f64) -> MLResult<Vec<Vec<PortfolioPosition>>> {
        // In a real implementation, this would use sophisticated optimization algorithms
        // For now, generate a few candidate portfolios
        let mut candidates = Vec::new();
        
        // Conservative rebalancing
        let mut conservative = current_positions.to_vec();
        for position in &mut conservative {
            if position.risk_score > risk_tolerance {
                position.allocation_percentage *= 0.8;
            }
        }
        candidates.push(conservative);
        
        // Aggressive rebalancing
        let mut aggressive = current_positions.to_vec();
        for position in &mut aggressive {
            if position.expected_apy > 10.0 && position.risk_score < risk_tolerance * 1.2 {
                position.allocation_percentage *= 1.2;
            }
        }
        candidates.push(aggressive);
        
        Ok(candidates)
    }

    /// Select optimal allocation
    async fn select_optimal_allocation(&self, candidates: Vec<Vec<PortfolioPosition>>, risk_tolerance: f64) -> MLResult<Vec<PortfolioPosition>> {
        let mut best_portfolio = candidates[0].clone();
        let mut best_score = 0.0;
        
        for candidate in candidates {
            let metrics = self.calculate_portfolio_metrics(&candidate).await?;
            let score = metrics.sharpe_ratio * (1.0 - (metrics.risk_score - risk_tolerance).abs());
            
            if score > best_score {
                best_score = score;
                best_portfolio = candidate;
            }
        }
        
        Ok(best_portfolio)
    }

    /// Generate rebalancing actions
    async fn generate_rebalancing_actions(&self, current: &[PortfolioPosition], optimized: &[PortfolioPosition]) -> MLResult<Vec<RebalancingAction>> {
        let mut actions = Vec::new();
        
        // Compare allocations and generate actions
        for (curr, opt) in current.iter().zip(optimized.iter()) {
            let diff = opt.allocation_percentage - curr.allocation_percentage;
            
            if diff.abs() > 1.0 { // Only rebalance if difference > 1%
                let action_type = if diff > 0.0 { "add" } else { "remove" };
                
                actions.push(RebalancingAction {
                    action_type: action_type.to_string(),
                    from_protocol: None,
                    to_protocol: opt.protocol.clone(),
                    asset: opt.asset.clone(),
                    amount_percentage: diff.abs(),
                    expected_impact: diff * opt.expected_apy / 100.0,
                    priority: if diff.abs() > 5.0 { 1 } else { 2 },
                });
            }
        }
        
        Ok(actions)
    }

    /// Calculate volatility
    fn calculate_volatility(&self, values: &[f64]) -> f64 {
        if values.len() < 2 {
            return 0.0;
        }
        
        let mean = values.iter().sum::<f64>() / values.len() as f64;
        let variance = values.iter()
            .map(|v| (v - mean).powi(2))
            .sum::<f64>() / (values.len() - 1) as f64;
        
        variance.sqrt()
    }

    /// Calculate trend
    fn calculate_trend(&self, values: &[f64]) -> f64 {
        if values.len() < 2 {
            return 0.0;
        }
        
        let first = values[values.len() - 1];
        let last = values[0];
        
        (last - first) / first
    }

    /// Calculate diversification score
    fn calculate_diversification_score(&self, positions: &[PortfolioPosition]) -> f64 {
        // Simple diversification score based on number of positions and allocation distribution
        let position_count = positions.len() as f64;
        let max_allocation = positions.iter()
            .map(|p| p.allocation_percentage)
            .fold(0.0, f64::max);
        
        let concentration_penalty = if max_allocation > 50.0 { 0.5 } else { 1.0 };
        
        (position_count / 10.0).min(1.0) * concentration_penalty
    }
}

/// Internal prediction output
struct PredictionOutput {
    current_apy: f64,
    predicted_apy: f64,
    confidence: f64,
    risk_adjusted_return: f64,
    volatility_forecast: f64,
    opportunity_score: f64,
}

/// Portfolio metrics
struct PortfolioMetrics {
    expected_return: f64,
    risk_score: f64,
    liquidity_score: f64,
    sharpe_ratio: f64,
    diversification_score: f64,
}

#[async_trait::async_trait]
impl MLService for YieldPredictor {
    async fn predict(&self, request: InferenceRequest) -> Result<InferenceResponse> {
        let start_time = std::time::Instant::now();
        
        // Extract parameters from input data
        let protocol = request.input_data["protocol"].as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'protocol' field in input data"))?;
        let asset = request.input_data["asset"].as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'asset' field in input data"))?;
        let time_horizon = request.input_data["time_horizon_days"].as_u64().unwrap_or(30) as u32;
        
        // Predict yield
        let result = self.predict_yield(protocol, asset, time_horizon).await
            .map_err(|e| anyhow::anyhow!("Yield prediction failed: {}", e))?;
        
        let processing_time = start_time.elapsed().as_millis() as u64;
        
        Ok(InferenceResponse {
            request_id: request.request_id,
            model_id: request.model_id,
            prediction: serde_json::to_value(result)?,
            confidence: 0.8, // Would be calculated from actual model
            processing_time_ms: processing_time,
            timestamp: Utc::now(),
        })
    }

    async fn batch_predict(&self, requests: Vec<InferenceRequest>) -> Result<Vec<InferenceResponse>> {
        let mut responses = Vec::new();
        
        for request in requests {
            let response = self.predict(request).await?;
            responses.push(response);
        }
        
        Ok(responses)
    }

    async fn get_model_info(&self, model_id: &str) -> Result<ModelMetadata> {
        Ok(ModelMetadata {
            model_id: model_id.to_string(),
            model_name: "DeFi Yield Predictor".to_string(),
            version: "1.0.0".to_string(),
            model_type: super::ModelType::YieldPrediction,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            accuracy: 0.78,
            confidence_threshold: self.config.min_confidence_threshold,
            input_features: vec!["protocol".to_string(), "asset".to_string(), "time_horizon_days".to_string()],
            output_schema: "YieldPredictionResult".to_string(),
            deployment_status: super::DeploymentStatus::Production,
        })
    }

    async fn health_check(&self) -> Result<bool> {
        let model_loaded = self.model.read().await.is_some();
        let has_data = !self.historical_data.read().await.is_empty();
        
        Ok(model_loaded && has_data)
    }
}

impl Default for YieldPredictionConfig {
    fn default() -> Self {
        Self {
            prediction_horizon_days: 30,
            min_confidence_threshold: 0.6,
            risk_free_rate: 0.02, // 2% annual
            volatility_window_days: 30,
            feature_window_days: 90,
            supported_protocols: vec![
                "Aave".to_string(),
                "Compound".to_string(),
                "Uniswap V3".to_string(),
                "Curve".to_string(),
                "Yearn".to_string(),
                "Convex".to_string(),
                "Balancer".to_string(),
            ],
        }
    }
}
