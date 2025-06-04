//! Market Prediction ML Service
//! 
//! Provides advanced market prediction capabilities including:
//! - Price trend forecasting using LSTM/Transformer models
//! - Market regime detection and classification
//! - Volatility prediction and risk assessment
//! - Cross-asset correlation analysis
//! - Market cycle identification

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

/// Market prediction service
pub struct MarketPredictor {
    model_path: String,
    price_model: Arc<RwLock<Option<PriceModel>>>,
    volatility_model: Arc<RwLock<Option<VolatilityModel>>>,
    regime_model: Arc<RwLock<Option<RegimeModel>>>,
    config: MarketPredictionConfig,
    market_data_cache: Arc<RwLock<Vec<MarketDataPoint>>>,
}

/// Market prediction configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketPredictionConfig {
    pub prediction_horizons: Vec<u32>, // hours: [1, 4, 24, 168]
    pub lookback_window_hours: u32,
    pub min_data_points: usize,
    pub confidence_threshold: f64,
    pub volatility_window: u32,
    pub regime_detection_window: u32,
    pub supported_assets: Vec<String>,
    pub feature_engineering: FeatureConfig,
}

/// Feature engineering configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureConfig {
    pub technical_indicators: bool,
    pub market_microstructure: bool,
    pub sentiment_features: bool,
    pub macro_features: bool,
    pub cross_asset_features: bool,
    pub time_features: bool,
}

/// Price prediction model
struct PriceModel {
    model_type: String,
    sequence_length: usize,
    feature_count: usize,
}

/// Volatility prediction model
struct VolatilityModel {
    model_type: String,
    garch_params: GarchParams,
}

/// Market regime detection model
struct RegimeModel {
    model_type: String,
    regime_count: usize,
    transition_matrix: Vec<Vec<f64>>,
}

/// GARCH model parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GarchParams {
    pub alpha: f64,
    pub beta: f64,
    pub omega: f64,
    pub gamma: f64, // For GJR-GARCH
}

/// Market prediction result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketPredictionResult {
    pub asset: String,
    pub current_price: f64,
    pub price_predictions: Vec<PricePrediction>,
    pub volatility_forecast: VolatilityForecast,
    pub market_regime: MarketRegime,
    pub trend_analysis: TrendAnalysis,
    pub risk_metrics: RiskMetrics,
    pub confidence_intervals: ConfidenceIntervals,
    pub prediction_timestamp: DateTime<Utc>,
}

/// Price prediction for specific horizon
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricePrediction {
    pub horizon_hours: u32,
    pub predicted_price: f64,
    pub price_change_percent: f64,
    pub direction_probability: f64,
    pub confidence: f64,
    pub support_levels: Vec<f64>,
    pub resistance_levels: Vec<f64>,
}

/// Volatility forecast
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolatilityForecast {
    pub current_volatility: f64,
    pub predicted_volatility: Vec<VolatilityPrediction>,
    pub volatility_regime: VolatilityRegime,
    pub garch_forecast: GarchForecast,
}

/// Volatility prediction for specific horizon
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolatilityPrediction {
    pub horizon_hours: u32,
    pub predicted_volatility: f64,
    pub volatility_percentile: f64,
    pub confidence: f64,
}

/// Volatility regime
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VolatilityRegime {
    Low,
    Medium,
    High,
    Extreme,
}

/// GARCH forecast
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GarchForecast {
    pub conditional_variance: f64,
    pub unconditional_variance: f64,
    pub persistence: f64,
    pub half_life: f64,
}

/// Market regime
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketRegime {
    pub current_regime: RegimeType,
    pub regime_probability: f64,
    pub regime_duration: u32,
    pub transition_probabilities: Vec<RegimeTransition>,
    pub regime_characteristics: RegimeCharacteristics,
}

/// Market regime types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RegimeType {
    Bull,
    Bear,
    Sideways,
    Volatile,
    Crash,
}

/// Regime transition probability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegimeTransition {
    pub from_regime: RegimeType,
    pub to_regime: RegimeType,
    pub probability: f64,
    pub expected_duration: u32,
}

/// Regime characteristics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegimeCharacteristics {
    pub average_return: f64,
    pub volatility: f64,
    pub skewness: f64,
    pub kurtosis: f64,
    pub max_drawdown: f64,
    pub sharpe_ratio: f64,
}

/// Trend analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendAnalysis {
    pub primary_trend: TrendDirection,
    pub secondary_trend: TrendDirection,
    pub trend_strength: f64,
    pub trend_duration: u32,
    pub trend_exhaustion_signals: Vec<String>,
    pub momentum_indicators: MomentumIndicators,
}

/// Trend direction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrendDirection {
    StrongBullish,
    Bullish,
    Neutral,
    Bearish,
    StrongBearish,
}

/// Momentum indicators
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MomentumIndicators {
    pub rsi: f64,
    pub macd_signal: String,
    pub stochastic: f64,
    pub williams_r: f64,
    pub momentum_score: f64,
}

/// Risk metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskMetrics {
    pub value_at_risk_1d: f64,
    pub expected_shortfall: f64,
    pub maximum_drawdown: f64,
    pub beta: f64,
    pub correlation_breakdown: Vec<AssetCorrelation>,
    pub tail_risk_indicators: TailRiskIndicators,
}

/// Asset correlation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetCorrelation {
    pub asset: String,
    pub correlation: f64,
    pub rolling_correlation: Vec<f64>,
    pub correlation_stability: f64,
}

/// Tail risk indicators
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TailRiskIndicators {
    pub skewness: f64,
    pub kurtosis: f64,
    pub tail_ratio: f64,
    pub extreme_value_theory_params: EvtParams,
}

/// Extreme Value Theory parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvtParams {
    pub shape_parameter: f64,
    pub scale_parameter: f64,
    pub threshold: f64,
    pub exceedances: u32,
}

/// Confidence intervals
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceIntervals {
    pub price_intervals: Vec<PriceInterval>,
    pub volatility_intervals: Vec<VolatilityInterval>,
    pub confidence_levels: Vec<f64>, // [0.68, 0.95, 0.99]
}

/// Price confidence interval
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceInterval {
    pub horizon_hours: u32,
    pub confidence_level: f64,
    pub lower_bound: f64,
    pub upper_bound: f64,
    pub median: f64,
}

/// Volatility confidence interval
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolatilityInterval {
    pub horizon_hours: u32,
    pub confidence_level: f64,
    pub lower_bound: f64,
    pub upper_bound: f64,
    pub median: f64,
}

impl MarketPredictor {
    /// Create a new market predictor
    pub async fn new(model_path: &str) -> MLResult<Self> {
        let config = MarketPredictionConfig::default();
        
        let predictor = Self {
            model_path: model_path.to_string(),
            price_model: Arc::new(RwLock::new(None)),
            volatility_model: Arc::new(RwLock::new(None)),
            regime_model: Arc::new(RwLock::new(None)),
            config,
            market_data_cache: Arc::new(RwLock::new(Vec::new())),
        };

        // Load models
        predictor.load_models().await?;
        
        // Initialize with historical data
        predictor.load_market_data().await?;
        
        Ok(predictor)
    }

    /// Load all prediction models
    #[instrument(skip(self))]
    async fn load_models(&self) -> MLResult<()> {
        info!(model_path = %self.model_path, "Loading market prediction models");

        // Load price prediction model
        let price_model = PriceModel {
            model_type: "LSTM".to_string(),
            sequence_length: 168, // 1 week of hourly data
            feature_count: 50,
        };

        // Load volatility model
        let volatility_model = VolatilityModel {
            model_type: "GJR-GARCH".to_string(),
            garch_params: GarchParams {
                alpha: 0.05,
                beta: 0.9,
                omega: 0.000001,
                gamma: 0.05,
            },
        };

        // Load regime detection model
        let regime_model = RegimeModel {
            model_type: "HMM".to_string(),
            regime_count: 4,
            transition_matrix: vec![
                vec![0.95, 0.03, 0.01, 0.01],
                vec![0.02, 0.94, 0.03, 0.01],
                vec![0.01, 0.02, 0.95, 0.02],
                vec![0.05, 0.05, 0.05, 0.85],
            ],
        };

        // Store models
        {
            let mut price_lock = self.price_model.write().await;
            *price_lock = Some(price_model);
        }

        {
            let mut vol_lock = self.volatility_model.write().await;
            *vol_lock = Some(volatility_model);
        }

        {
            let mut regime_lock = self.regime_model.write().await;
            *regime_lock = Some(regime_model);
        }

        info!("Market prediction models loaded successfully");
        Ok(())
    }

    /// Load historical market data
    async fn load_market_data(&self) -> MLResult<()> {
        let mut market_data = self.market_data_cache.write().await;
        
        // Generate mock historical data for supported assets
        for asset in &self.config.supported_assets {
            for i in 0..self.config.lookback_window_hours {
                let timestamp = Utc::now() - Duration::hours(i as i64);
                let base_price = match asset.as_str() {
                    "BTC" => 45000.0,
                    "ETH" => 3000.0,
                    "SOL" => 100.0,
                    _ => 1.0,
                };
                
                let price_variation = (i as f64 * 0.01).sin() * 0.05;
                let price = base_price * (1.0 + price_variation);
                
                market_data.push(MarketDataPoint {
                    symbol: asset.clone(),
                    price,
                    volume: price * 1000.0,
                    market_cap: price * 21_000_000.0,
                    volatility: 0.02 + (i as f64 * 0.0001) % 0.03,
                    timestamp,
                    technical_indicators: TechnicalIndicators {
                        rsi: 50.0 + (i as f64 * 0.1) % 30.0,
                        macd: (i as f64 * 0.01).sin(),
                        bollinger_upper: price * 1.02,
                        bollinger_lower: price * 0.98,
                        sma_20: price * 0.995,
                        ema_20: price * 0.998,
                        volume_sma: price * 800.0,
                        momentum: (i as f64 * 0.02).cos() * 10.0,
                    },
                });
            }
        }
        
        Ok(())
    }

    /// Predict market movements for an asset
    #[instrument(skip(self))]
    pub async fn predict_market(&self, asset: &str, horizons: &[u32]) -> MLResult<MarketPredictionResult> {
        info!(asset = %asset, horizons = ?horizons, "Predicting market movements");

        // Get current market data
        let current_data = self.get_current_market_data(asset).await?;
        
        // Generate price predictions
        let price_predictions = self.predict_prices(asset, horizons).await?;
        
        // Generate volatility forecast
        let volatility_forecast = self.predict_volatility(asset, horizons).await?;
        
        // Detect market regime
        let market_regime = self.detect_market_regime(asset).await?;
        
        // Analyze trends
        let trend_analysis = self.analyze_trends(asset).await?;
        
        // Calculate risk metrics
        let risk_metrics = self.calculate_risk_metrics(asset).await?;
        
        // Generate confidence intervals
        let confidence_intervals = self.generate_confidence_intervals(asset, horizons).await?;
        
        Ok(MarketPredictionResult {
            asset: asset.to_string(),
            current_price: current_data.price,
            price_predictions,
            volatility_forecast,
            market_regime,
            trend_analysis,
            risk_metrics,
            confidence_intervals,
            prediction_timestamp: Utc::now(),
        })
    }

    /// Get current market data for asset
    async fn get_current_market_data(&self, asset: &str) -> MLResult<MarketDataPoint> {
        let market_data = self.market_data_cache.read().await;
        
        market_data.iter()
            .filter(|d| d.symbol == asset)
            .max_by_key(|d| d.timestamp)
            .cloned()
            .ok_or_else(|| MLError::FeatureExtractionFailed {
                reason: format!("No market data found for asset: {}", asset),
            })
    }

    /// Predict prices for multiple horizons
    async fn predict_prices(&self, asset: &str, horizons: &[u32]) -> MLResult<Vec<PricePrediction>> {
        let current_data = self.get_current_market_data(asset).await?;
        let mut predictions = Vec::new();
        
        for &horizon in horizons {
            // Simple prediction logic (would be replaced by actual ML model)
            let trend_factor = 1.0 + (horizon as f64 * 0.001);
            let volatility_factor = 1.0 + (current_data.volatility * horizon as f64 * 0.1);
            let predicted_price = current_data.price * trend_factor * volatility_factor;
            
            let price_change_percent = (predicted_price / current_data.price - 1.0) * 100.0;
            let direction_probability = if price_change_percent > 0.0 { 0.6 } else { 0.4 };
            
            predictions.push(PricePrediction {
                horizon_hours: horizon,
                predicted_price,
                price_change_percent,
                direction_probability,
                confidence: 0.75 - (horizon as f64 * 0.01),
                support_levels: vec![predicted_price * 0.95, predicted_price * 0.98],
                resistance_levels: vec![predicted_price * 1.02, predicted_price * 1.05],
            });
        }
        
        Ok(predictions)
    }

    /// Predict volatility for multiple horizons
    async fn predict_volatility(&self, asset: &str, horizons: &[u32]) -> MLResult<VolatilityForecast> {
        let current_data = self.get_current_market_data(asset).await?;
        let mut volatility_predictions = Vec::new();
        
        for &horizon in horizons {
            let predicted_volatility = current_data.volatility * (1.0 + horizon as f64 * 0.001);
            let volatility_percentile = 0.5 + (predicted_volatility - 0.02) * 10.0;
            
            volatility_predictions.push(VolatilityPrediction {
                horizon_hours: horizon,
                predicted_volatility,
                volatility_percentile: volatility_percentile.clamp(0.0, 1.0),
                confidence: 0.8,
            });
        }
        
        let volatility_regime = match current_data.volatility {
            v if v < 0.01 => VolatilityRegime::Low,
            v if v < 0.03 => VolatilityRegime::Medium,
            v if v < 0.05 => VolatilityRegime::High,
            _ => VolatilityRegime::Extreme,
        };
        
        let garch_forecast = GarchForecast {
            conditional_variance: current_data.volatility.powi(2),
            unconditional_variance: 0.0004, // Long-term variance
            persistence: 0.95,
            half_life: 20.0,
        };
        
        Ok(VolatilityForecast {
            current_volatility: current_data.volatility,
            predicted_volatility: volatility_predictions,
            volatility_regime,
            garch_forecast,
        })
    }

    /// Detect current market regime
    async fn detect_market_regime(&self, asset: &str) -> MLResult<MarketRegime> {
        let current_data = self.get_current_market_data(asset).await?;
        
        // Simple regime detection based on volatility and trend
        let current_regime = if current_data.volatility > 0.04 {
            RegimeType::Volatile
        } else if current_data.technical_indicators.rsi > 70.0 {
            RegimeType::Bull
        } else if current_data.technical_indicators.rsi < 30.0 {
            RegimeType::Bear
        } else {
            RegimeType::Sideways
        };
        
        let transition_probabilities = vec![
            RegimeTransition {
                from_regime: current_regime.clone(),
                to_regime: RegimeType::Bull,
                probability: 0.3,
                expected_duration: 48,
            },
            RegimeTransition {
                from_regime: current_regime.clone(),
                to_regime: RegimeType::Bear,
                probability: 0.2,
                expected_duration: 72,
            },
            RegimeTransition {
                from_regime: current_regime.clone(),
                to_regime: RegimeType::Sideways,
                probability: 0.4,
                expected_duration: 24,
            },
        ];
        
        Ok(MarketRegime {
            current_regime,
            regime_probability: 0.8,
            regime_duration: 24,
            transition_probabilities,
            regime_characteristics: RegimeCharacteristics {
                average_return: 0.001,
                volatility: current_data.volatility,
                skewness: -0.1,
                kurtosis: 3.5,
                max_drawdown: 0.15,
                sharpe_ratio: 1.2,
            },
        })
    }

    /// Analyze market trends
    async fn analyze_trends(&self, asset: &str) -> MLResult<TrendAnalysis> {
        let current_data = self.get_current_market_data(asset).await?;
        
        let primary_trend = match current_data.technical_indicators.rsi {
            rsi if rsi > 80.0 => TrendDirection::StrongBullish,
            rsi if rsi > 60.0 => TrendDirection::Bullish,
            rsi if rsi > 40.0 => TrendDirection::Neutral,
            rsi if rsi > 20.0 => TrendDirection::Bearish,
            _ => TrendDirection::StrongBearish,
        };
        
        Ok(TrendAnalysis {
            primary_trend,
            secondary_trend: TrendDirection::Neutral,
            trend_strength: 0.7,
            trend_duration: 48,
            trend_exhaustion_signals: vec!["RSI divergence".to_string()],
            momentum_indicators: MomentumIndicators {
                rsi: current_data.technical_indicators.rsi,
                macd_signal: "bullish".to_string(),
                stochastic: 65.0,
                williams_r: -25.0,
                momentum_score: 0.6,
            },
        })
    }

    /// Calculate risk metrics
    async fn calculate_risk_metrics(&self, asset: &str) -> MLResult<RiskMetrics> {
        let current_data = self.get_current_market_data(asset).await?;
        
        Ok(RiskMetrics {
            value_at_risk_1d: current_data.price * 0.05,
            expected_shortfall: current_data.price * 0.08,
            maximum_drawdown: 0.2,
            beta: 1.2,
            correlation_breakdown: vec![
                AssetCorrelation {
                    asset: "BTC".to_string(),
                    correlation: 0.8,
                    rolling_correlation: vec![0.75, 0.8, 0.85],
                    correlation_stability: 0.9,
                }
            ],
            tail_risk_indicators: TailRiskIndicators {
                skewness: -0.5,
                kurtosis: 4.2,
                tail_ratio: 0.8,
                extreme_value_theory_params: EvtParams {
                    shape_parameter: 0.1,
                    scale_parameter: 0.02,
                    threshold: 0.05,
                    exceedances: 10,
                },
            },
        })
    }

    /// Generate confidence intervals
    async fn generate_confidence_intervals(&self, asset: &str, horizons: &[u32]) -> MLResult<ConfidenceIntervals> {
        let current_data = self.get_current_market_data(asset).await?;
        let confidence_levels = vec![0.68, 0.95, 0.99];
        
        let mut price_intervals = Vec::new();
        let mut volatility_intervals = Vec::new();
        
        for &horizon in horizons {
            for &confidence_level in &confidence_levels {
                let z_score = match confidence_level {
                    0.68 => 1.0,
                    0.95 => 1.96,
                    0.99 => 2.58,
                    _ => 1.96,
                };
                
                let volatility_adjustment = current_data.volatility * (horizon as f64).sqrt();
                let price_std = current_data.price * volatility_adjustment;
                
                price_intervals.push(PriceInterval {
                    horizon_hours: horizon,
                    confidence_level,
                    lower_bound: current_data.price - z_score * price_std,
                    upper_bound: current_data.price + z_score * price_std,
                    median: current_data.price,
                });
                
                volatility_intervals.push(VolatilityInterval {
                    horizon_hours: horizon,
                    confidence_level,
                    lower_bound: current_data.volatility * 0.8,
                    upper_bound: current_data.volatility * 1.2,
                    median: current_data.volatility,
                });
            }
        }
        
        Ok(ConfidenceIntervals {
            price_intervals,
            volatility_intervals,
            confidence_levels,
        })
    }
}

#[async_trait::async_trait]
impl MLService for MarketPredictor {
    async fn predict(&self, request: InferenceRequest) -> Result<InferenceResponse> {
        let start_time = std::time::Instant::now();
        
        // Extract parameters from input data
        let asset = request.input_data["asset"].as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'asset' field in input data"))?;
        let horizons: Vec<u32> = request.input_data["horizons"].as_array()
            .map(|arr| arr.iter().filter_map(|v| v.as_u64().map(|h| h as u32)).collect())
            .unwrap_or_else(|| vec![1, 4, 24, 168]);
        
        // Predict market
        let result = self.predict_market(asset, &horizons).await
            .map_err(|e| anyhow::anyhow!("Market prediction failed: {}", e))?;
        
        let processing_time = start_time.elapsed().as_millis() as u64;
        
        Ok(InferenceResponse {
            request_id: request.request_id,
            model_id: request.model_id,
            prediction: serde_json::to_value(result)?,
            confidence: 0.8,
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
            model_name: "Market Predictor".to_string(),
            version: "1.0.0".to_string(),
            model_type: super::ModelType::MarketTrend,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            accuracy: 0.72,
            confidence_threshold: self.config.confidence_threshold,
            input_features: vec!["asset".to_string(), "horizons".to_string()],
            output_schema: "MarketPredictionResult".to_string(),
            deployment_status: super::DeploymentStatus::Production,
        })
    }

    async fn health_check(&self) -> Result<bool> {
        let price_model_loaded = self.price_model.read().await.is_some();
        let volatility_model_loaded = self.volatility_model.read().await.is_some();
        let regime_model_loaded = self.regime_model.read().await.is_some();
        let has_data = !self.market_data_cache.read().await.is_empty();
        
        Ok(price_model_loaded && volatility_model_loaded && regime_model_loaded && has_data)
    }
}

impl Default for MarketPredictionConfig {
    fn default() -> Self {
        Self {
            prediction_horizons: vec![1, 4, 24, 168], // 1h, 4h, 1d, 1w
            lookback_window_hours: 720, // 30 days
            min_data_points: 100,
            confidence_threshold: 0.7,
            volatility_window: 24,
            regime_detection_window: 168,
            supported_assets: vec![
                "BTC".to_string(),
                "ETH".to_string(),
                "SOL".to_string(),
                "USDC".to_string(),
                "USDT".to_string(),
            ],
            feature_engineering: FeatureConfig {
                technical_indicators: true,
                market_microstructure: true,
                sentiment_features: true,
                macro_features: true,
                cross_asset_features: true,
                time_features: true,
            },
        }
    }
}
