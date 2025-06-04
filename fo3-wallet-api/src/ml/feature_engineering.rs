//! Feature Engineering for ML Models
//! 
//! Provides comprehensive feature engineering capabilities including:
//! - Technical indicator calculation
//! - Time-based feature extraction
//! - Cross-asset correlation features
//! - Sentiment-based features
//! - Market microstructure features

use std::collections::HashMap;
use anyhow::Result;
use tracing::{info, warn, error, instrument};
use chrono::{DateTime, Utc, Timelike, Weekday};
use serde::{Deserialize, Serialize};

use super::{
    MarketDataPoint, TechnicalIndicators, SentimentData, FeatureVector,
    MLError, MLResult
};

/// Feature engineering service
pub struct FeatureEngineer {
    config: FeatureEngineeringConfig,
    feature_cache: HashMap<String, Vec<f64>>,
}

/// Feature engineering configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureEngineeringConfig {
    pub technical_features: TechnicalFeatureConfig,
    pub time_features: TimeFeatureConfig,
    pub sentiment_features: SentimentFeatureConfig,
    pub cross_asset_features: CrossAssetFeatureConfig,
    pub normalization: NormalizationConfig,
    pub feature_selection: FeatureSelectionConfig,
}

/// Technical feature configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TechnicalFeatureConfig {
    pub price_features: bool,
    pub volume_features: bool,
    pub volatility_features: bool,
    pub momentum_features: bool,
    pub trend_features: bool,
    pub support_resistance: bool,
    pub candlestick_patterns: bool,
}

/// Time-based feature configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeFeatureConfig {
    pub hour_of_day: bool,
    pub day_of_week: bool,
    pub day_of_month: bool,
    pub month_of_year: bool,
    pub quarter: bool,
    pub is_weekend: bool,
    pub is_holiday: bool,
    pub market_session: bool,
}

/// Sentiment feature configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentimentFeatureConfig {
    pub social_sentiment: bool,
    pub news_sentiment: bool,
    pub whale_sentiment: bool,
    pub fear_greed_index: bool,
    pub sentiment_momentum: bool,
    pub sentiment_divergence: bool,
}

/// Cross-asset feature configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossAssetFeatureConfig {
    pub correlation_features: bool,
    pub relative_strength: bool,
    pub sector_performance: bool,
    pub market_beta: bool,
    pub spread_features: bool,
}

/// Normalization configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizationConfig {
    pub method: NormalizationMethod,
    pub window_size: u32,
    pub outlier_handling: OutlierHandling,
    pub missing_value_strategy: MissingValueStrategy,
}

/// Feature selection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureSelectionConfig {
    pub max_features: usize,
    pub correlation_threshold: f64,
    pub importance_threshold: f64,
    pub variance_threshold: f64,
}

/// Normalization methods
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NormalizationMethod {
    ZScore,
    MinMax,
    Robust,
    Quantile,
    None,
}

/// Outlier handling strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OutlierHandling {
    Remove,
    Cap,
    Transform,
    Keep,
}

/// Missing value strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MissingValueStrategy {
    Forward,
    Backward,
    Interpolate,
    Mean,
    Median,
    Zero,
}

/// Feature importance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureImportance {
    pub feature_name: String,
    pub importance_score: f64,
    pub rank: u32,
    pub category: String,
}

/// Feature statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureStatistics {
    pub feature_name: String,
    pub mean: f64,
    pub std: f64,
    pub min: f64,
    pub max: f64,
    pub median: f64,
    pub skewness: f64,
    pub kurtosis: f64,
    pub missing_percentage: f64,
}

impl FeatureEngineer {
    /// Create a new feature engineer
    pub fn new(config: FeatureEngineeringConfig) -> Self {
        Self {
            config,
            feature_cache: HashMap::new(),
        }
    }

    /// Extract features from market data
    #[instrument(skip(self, market_data))]
    pub async fn extract_features(&mut self, market_data: &[MarketDataPoint], sentiment_data: Option<&[SentimentData]>) -> MLResult<FeatureVector> {
        info!(data_points = %market_data.len(), "Extracting features from market data");

        let mut features = Vec::new();
        let mut feature_names = Vec::new();

        // Technical features
        if self.config.technical_features.price_features {
            let price_features = self.extract_price_features(market_data)?;
            features.extend(price_features.0);
            feature_names.extend(price_features.1);
        }

        if self.config.technical_features.volume_features {
            let volume_features = self.extract_volume_features(market_data)?;
            features.extend(volume_features.0);
            feature_names.extend(volume_features.1);
        }

        if self.config.technical_features.volatility_features {
            let volatility_features = self.extract_volatility_features(market_data)?;
            features.extend(volatility_features.0);
            feature_names.extend(volatility_features.1);
        }

        // Time features
        if self.config.time_features.hour_of_day {
            let time_features = self.extract_time_features(market_data)?;
            features.extend(time_features.0);
            feature_names.extend(time_features.1);
        }

        // Sentiment features
        if let Some(sentiment) = sentiment_data {
            if self.config.sentiment_features.social_sentiment {
                let sentiment_features = self.extract_sentiment_features(sentiment)?;
                features.extend(sentiment_features.0);
                feature_names.extend(sentiment_features.1);
            }
        }

        // Cross-asset features
        if self.config.cross_asset_features.correlation_features {
            let cross_features = self.extract_cross_asset_features(market_data)?;
            features.extend(cross_features.0);
            feature_names.extend(cross_features.1);
        }

        // Normalize features
        let normalized_features = self.normalize_features(&features)?;

        Ok(FeatureVector {
            features: normalized_features,
            feature_names,
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        })
    }

    /// Extract price-based features
    fn extract_price_features(&self, market_data: &[MarketDataPoint]) -> MLResult<(Vec<f64>, Vec<String>)> {
        if market_data.is_empty() {
            return Ok((vec![], vec![]));
        }

        let mut features = Vec::new();
        let mut names = Vec::new();

        // Current price
        features.push(market_data.last().unwrap().price);
        names.push("current_price".to_string());

        // Price returns
        if market_data.len() > 1 {
            let returns = self.calculate_returns(market_data);
            features.push(returns.last().copied().unwrap_or(0.0));
            names.push("price_return_1".to_string());

            // Rolling statistics
            if returns.len() >= 24 {
                let recent_returns = &returns[returns.len()-24..];
                features.push(recent_returns.iter().sum::<f64>() / recent_returns.len() as f64);
                names.push("price_return_mean_24h".to_string());

                let variance = recent_returns.iter()
                    .map(|r| (r - features.last().unwrap()).powi(2))
                    .sum::<f64>() / (recent_returns.len() - 1) as f64;
                features.push(variance.sqrt());
                names.push("price_return_std_24h".to_string());
            }
        }

        // Price momentum
        if market_data.len() > 24 {
            let momentum = (market_data.last().unwrap().price / market_data[market_data.len()-24].price - 1.0) * 100.0;
            features.push(momentum);
            names.push("price_momentum_24h".to_string());
        }

        Ok((features, names))
    }

    /// Extract volume-based features
    fn extract_volume_features(&self, market_data: &[MarketDataPoint]) -> MLResult<(Vec<f64>, Vec<String>)> {
        if market_data.is_empty() {
            return Ok((vec![], vec![]));
        }

        let mut features = Vec::new();
        let mut names = Vec::new();

        // Current volume
        features.push(market_data.last().unwrap().volume);
        names.push("current_volume".to_string());

        // Volume moving average
        if market_data.len() >= 24 {
            let recent_volumes: Vec<f64> = market_data.iter()
                .rev()
                .take(24)
                .map(|d| d.volume)
                .collect();
            
            let volume_ma = recent_volumes.iter().sum::<f64>() / recent_volumes.len() as f64;
            features.push(volume_ma);
            names.push("volume_ma_24h".to_string());

            // Volume ratio
            let volume_ratio = market_data.last().unwrap().volume / volume_ma;
            features.push(volume_ratio);
            names.push("volume_ratio".to_string());
        }

        Ok((features, names))
    }

    /// Extract volatility features
    fn extract_volatility_features(&self, market_data: &[MarketDataPoint]) -> MLResult<(Vec<f64>, Vec<String>)> {
        if market_data.len() < 2 {
            return Ok((vec![], vec![]));
        }

        let mut features = Vec::new();
        let mut names = Vec::new();

        let returns = self.calculate_returns(market_data);
        
        // Realized volatility
        if returns.len() >= 24 {
            let recent_returns = &returns[returns.len()-24..];
            let mean_return = recent_returns.iter().sum::<f64>() / recent_returns.len() as f64;
            let variance = recent_returns.iter()
                .map(|r| (r - mean_return).powi(2))
                .sum::<f64>() / (recent_returns.len() - 1) as f64;
            
            features.push(variance.sqrt() * 100.0); // Convert to percentage
            names.push("realized_volatility_24h".to_string());
        }

        // GARCH-like volatility
        if returns.len() >= 48 {
            let garch_vol = self.calculate_garch_volatility(&returns);
            features.push(garch_vol);
            names.push("garch_volatility".to_string());
        }

        Ok((features, names))
    }

    /// Extract time-based features
    fn extract_time_features(&self, market_data: &[MarketDataPoint]) -> MLResult<(Vec<f64>, Vec<String>)> {
        if market_data.is_empty() {
            return Ok((vec![], vec![]));
        }

        let mut features = Vec::new();
        let mut names = Vec::new();

        let timestamp = market_data.last().unwrap().timestamp;

        // Hour of day (0-23)
        if self.config.time_features.hour_of_day {
            features.push(timestamp.hour() as f64);
            names.push("hour_of_day".to_string());
        }

        // Day of week (0-6, Monday=0)
        if self.config.time_features.day_of_week {
            let day_of_week = match timestamp.weekday() {
                Weekday::Mon => 0.0,
                Weekday::Tue => 1.0,
                Weekday::Wed => 2.0,
                Weekday::Thu => 3.0,
                Weekday::Fri => 4.0,
                Weekday::Sat => 5.0,
                Weekday::Sun => 6.0,
            };
            features.push(day_of_week);
            names.push("day_of_week".to_string());
        }

        // Is weekend
        if self.config.time_features.is_weekend {
            let is_weekend = matches!(timestamp.weekday(), Weekday::Sat | Weekday::Sun);
            features.push(if is_weekend { 1.0 } else { 0.0 });
            names.push("is_weekend".to_string());
        }

        // Market session (simplified)
        if self.config.time_features.market_session {
            let hour = timestamp.hour();
            let session = match hour {
                0..=7 => 0.0,   // Asian session
                8..=15 => 1.0,  // European session
                16..=23 => 2.0, // US session
                _ => 0.0,
            };
            features.push(session);
            names.push("market_session".to_string());
        }

        Ok((features, names))
    }

    /// Extract sentiment-based features
    fn extract_sentiment_features(&self, sentiment_data: &[SentimentData]) -> MLResult<(Vec<f64>, Vec<String>)> {
        if sentiment_data.is_empty() {
            return Ok((vec![], vec![]));
        }

        let mut features = Vec::new();
        let mut names = Vec::new();

        // Average sentiment score
        let avg_sentiment = sentiment_data.iter()
            .filter_map(|s| s.engagement_metrics.sentiment_score)
            .sum::<f64>() / sentiment_data.len() as f64;
        features.push(avg_sentiment);
        names.push("avg_sentiment".to_string());

        // Sentiment momentum
        if sentiment_data.len() > 1 {
            let recent_sentiment = sentiment_data.iter()
                .rev()
                .take(10)
                .filter_map(|s| s.engagement_metrics.sentiment_score)
                .collect::<Vec<f64>>();
            
            if recent_sentiment.len() > 1 {
                let momentum = recent_sentiment.first().unwrap() - recent_sentiment.last().unwrap();
                features.push(momentum);
                names.push("sentiment_momentum".to_string());
            }
        }

        // Engagement metrics
        let total_engagement: u64 = sentiment_data.iter()
            .map(|s| s.engagement_metrics.likes + s.engagement_metrics.shares + s.engagement_metrics.comments)
            .sum();
        features.push(total_engagement as f64);
        names.push("total_engagement".to_string());

        Ok((features, names))
    }

    /// Extract cross-asset features
    fn extract_cross_asset_features(&self, market_data: &[MarketDataPoint]) -> MLResult<(Vec<f64>, Vec<String>)> {
        let mut features = Vec::new();
        let mut names = Vec::new();

        // For now, add placeholder correlation features
        // In a real implementation, this would calculate correlations with other assets
        features.push(0.8); // Mock BTC correlation
        names.push("btc_correlation".to_string());

        features.push(0.6); // Mock ETH correlation
        names.push("eth_correlation".to_string());

        features.push(1.2); // Mock market beta
        names.push("market_beta".to_string());

        Ok((features, names))
    }

    /// Calculate price returns
    fn calculate_returns(&self, market_data: &[MarketDataPoint]) -> Vec<f64> {
        if market_data.len() < 2 {
            return vec![];
        }

        market_data.windows(2)
            .map(|window| (window[1].price / window[0].price - 1.0) * 100.0)
            .collect()
    }

    /// Calculate GARCH-like volatility
    fn calculate_garch_volatility(&self, returns: &[f64]) -> f64 {
        if returns.len() < 10 {
            return 0.0;
        }

        // Simplified GARCH(1,1) calculation
        let alpha = 0.1;
        let beta = 0.85;
        let omega = 0.000001;

        let mut variance = returns.iter().map(|r| r.powi(2)).sum::<f64>() / returns.len() as f64;
        
        for return_val in returns.iter().rev().take(10) {
            variance = omega + alpha * return_val.powi(2) + beta * variance;
        }

        variance.sqrt()
    }

    /// Normalize features
    fn normalize_features(&self, features: &[f64]) -> MLResult<Vec<f64>> {
        match self.config.normalization.method {
            NormalizationMethod::ZScore => {
                let mean = features.iter().sum::<f64>() / features.len() as f64;
                let variance = features.iter()
                    .map(|f| (f - mean).powi(2))
                    .sum::<f64>() / features.len() as f64;
                let std = variance.sqrt();
                
                if std == 0.0 {
                    Ok(features.to_vec())
                } else {
                    Ok(features.iter().map(|f| (f - mean) / std).collect())
                }
            },
            NormalizationMethod::MinMax => {
                let min_val = features.iter().fold(f64::INFINITY, |a, &b| a.min(b));
                let max_val = features.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
                let range = max_val - min_val;
                
                if range == 0.0 {
                    Ok(features.to_vec())
                } else {
                    Ok(features.iter().map(|f| (f - min_val) / range).collect())
                }
            },
            NormalizationMethod::None => Ok(features.to_vec()),
            _ => Ok(features.to_vec()), // Placeholder for other methods
        }
    }

    /// Calculate feature importance
    pub fn calculate_feature_importance(&self, feature_names: &[String], target_correlation: &[f64]) -> Vec<FeatureImportance> {
        feature_names.iter()
            .zip(target_correlation.iter())
            .enumerate()
            .map(|(i, (name, &correlation))| {
                FeatureImportance {
                    feature_name: name.clone(),
                    importance_score: correlation.abs(),
                    rank: i as u32 + 1,
                    category: self.categorize_feature(name),
                }
            })
            .collect()
    }

    /// Calculate feature statistics
    pub fn calculate_feature_statistics(&self, features: &[FeatureVector]) -> Vec<FeatureStatistics> {
        if features.is_empty() {
            return vec![];
        }

        let feature_count = features[0].features.len();
        let mut statistics = Vec::new();

        for i in 0..feature_count {
            let values: Vec<f64> = features.iter().map(|f| f.features[i]).collect();
            let feature_name = features[0].feature_names.get(i)
                .cloned()
                .unwrap_or_else(|| format!("feature_{}", i));

            let mean = values.iter().sum::<f64>() / values.len() as f64;
            let variance = values.iter()
                .map(|v| (v - mean).powi(2))
                .sum::<f64>() / values.len() as f64;
            let std = variance.sqrt();

            let mut sorted_values = values.clone();
            sorted_values.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let median = sorted_values[sorted_values.len() / 2];
            let min = sorted_values[0];
            let max = sorted_values[sorted_values.len() - 1];

            statistics.push(FeatureStatistics {
                feature_name,
                mean,
                std,
                min,
                max,
                median,
                skewness: 0.0, // Placeholder
                kurtosis: 0.0, // Placeholder
                missing_percentage: 0.0, // Placeholder
            });
        }

        statistics
    }

    /// Categorize feature by name
    fn categorize_feature(&self, feature_name: &str) -> String {
        if feature_name.contains("price") {
            "price".to_string()
        } else if feature_name.contains("volume") {
            "volume".to_string()
        } else if feature_name.contains("volatility") {
            "volatility".to_string()
        } else if feature_name.contains("sentiment") {
            "sentiment".to_string()
        } else if feature_name.contains("time") || feature_name.contains("hour") || feature_name.contains("day") {
            "time".to_string()
        } else {
            "other".to_string()
        }
    }
}

impl Default for FeatureEngineeringConfig {
    fn default() -> Self {
        Self {
            technical_features: TechnicalFeatureConfig {
                price_features: true,
                volume_features: true,
                volatility_features: true,
                momentum_features: true,
                trend_features: true,
                support_resistance: true,
                candlestick_patterns: false,
            },
            time_features: TimeFeatureConfig {
                hour_of_day: true,
                day_of_week: true,
                day_of_month: false,
                month_of_year: false,
                quarter: false,
                is_weekend: true,
                is_holiday: false,
                market_session: true,
            },
            sentiment_features: SentimentFeatureConfig {
                social_sentiment: true,
                news_sentiment: true,
                whale_sentiment: false,
                fear_greed_index: true,
                sentiment_momentum: true,
                sentiment_divergence: false,
            },
            cross_asset_features: CrossAssetFeatureConfig {
                correlation_features: true,
                relative_strength: true,
                sector_performance: false,
                market_beta: true,
                spread_features: false,
            },
            normalization: NormalizationConfig {
                method: NormalizationMethod::ZScore,
                window_size: 100,
                outlier_handling: OutlierHandling::Cap,
                missing_value_strategy: MissingValueStrategy::Forward,
            },
            feature_selection: FeatureSelectionConfig {
                max_features: 50,
                correlation_threshold: 0.95,
                importance_threshold: 0.01,
                variance_threshold: 0.001,
            },
        }
    }
}
