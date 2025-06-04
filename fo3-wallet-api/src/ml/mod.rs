//! Machine Learning Infrastructure for FO3 Wallet Core
//! 
//! This module provides comprehensive ML capabilities including:
//! - Model management and versioning
//! - Real-time inference pipeline
//! - Sentiment analysis models
//! - Predictive analytics
//! - Market intelligence algorithms
//! - Risk assessment models

pub mod model_manager;
pub mod sentiment_analyzer;
pub mod yield_predictor;
pub mod market_predictor;
pub mod risk_assessor;
pub mod trading_signals;
pub mod data_pipeline;
pub mod feature_engineering;

// Re-export main components
pub use model_manager::ModelManager;
pub use sentiment_analyzer::SentimentAnalyzer;
pub use yield_predictor::YieldPredictor;
pub use market_predictor::MarketPredictor;
pub use risk_assessor::RiskAssessor;
pub use trading_signals::TradingSignalsGenerator;
pub use data_pipeline::DataPipeline;
pub use feature_engineering::FeatureEngineer;

use std::sync::Arc;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// ML model metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelMetadata {
    pub model_id: String,
    pub model_name: String,
    pub version: String,
    pub model_type: ModelType,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub accuracy: f64,
    pub confidence_threshold: f64,
    pub input_features: Vec<String>,
    pub output_schema: String,
    pub deployment_status: DeploymentStatus,
}

/// ML model types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModelType {
    SentimentAnalysis,
    YieldPrediction,
    MarketTrend,
    RiskAssessment,
    TradingSignal,
    AnomalyDetection,
    PortfolioOptimization,
}

/// Model deployment status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeploymentStatus {
    Training,
    Testing,
    Staging,
    Production,
    Deprecated,
}

/// ML inference request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceRequest {
    pub model_id: String,
    pub input_data: serde_json::Value,
    pub request_id: String,
    pub timestamp: DateTime<Utc>,
}

/// ML inference response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceResponse {
    pub request_id: String,
    pub model_id: String,
    pub prediction: serde_json::Value,
    pub confidence: f64,
    pub processing_time_ms: u64,
    pub timestamp: DateTime<Utc>,
}

/// Feature vector for ML models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureVector {
    pub features: Vec<f64>,
    pub feature_names: Vec<String>,
    pub timestamp: DateTime<Utc>,
    pub metadata: std::collections::HashMap<String, String>,
}

/// Market data point for ML processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketDataPoint {
    pub symbol: String,
    pub price: f64,
    pub volume: f64,
    pub market_cap: f64,
    pub volatility: f64,
    pub timestamp: DateTime<Utc>,
    pub technical_indicators: TechnicalIndicators,
}

/// Technical indicators for ML features
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TechnicalIndicators {
    pub rsi: f64,
    pub macd: f64,
    pub bollinger_upper: f64,
    pub bollinger_lower: f64,
    pub sma_20: f64,
    pub ema_20: f64,
    pub volume_sma: f64,
    pub momentum: f64,
}

/// Sentiment data for ML processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentimentData {
    pub symbol: String,
    pub text: String,
    pub source: String,
    pub timestamp: DateTime<Utc>,
    pub engagement_metrics: EngagementMetrics,
}

/// Social media engagement metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngagementMetrics {
    pub likes: u64,
    pub shares: u64,
    pub comments: u64,
    pub reach: u64,
    pub sentiment_score: Option<f64>,
}

/// ML configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MLConfig {
    pub model_storage_path: String,
    pub inference_timeout_ms: u64,
    pub batch_size: usize,
    pub max_concurrent_requests: usize,
    pub cache_ttl_seconds: u64,
    pub enable_gpu: bool,
    pub model_refresh_interval_hours: u64,
}

impl Default for MLConfig {
    fn default() -> Self {
        Self {
            model_storage_path: "/app/models".to_string(),
            inference_timeout_ms: 5000,
            batch_size: 32,
            max_concurrent_requests: 100,
            cache_ttl_seconds: 300,
            enable_gpu: false,
            model_refresh_interval_hours: 24,
        }
    }
}

/// ML service trait for common operations
#[async_trait::async_trait]
pub trait MLService: Send + Sync {
    async fn predict(&self, request: InferenceRequest) -> Result<InferenceResponse>;
    async fn batch_predict(&self, requests: Vec<InferenceRequest>) -> Result<Vec<InferenceResponse>>;
    async fn get_model_info(&self, model_id: &str) -> Result<ModelMetadata>;
    async fn health_check(&self) -> Result<bool>;
}

/// Error types for ML operations
#[derive(Debug, thiserror::Error)]
pub enum MLError {
    #[error("Model not found: {model_id}")]
    ModelNotFound { model_id: String },
    
    #[error("Inference failed: {reason}")]
    InferenceFailed { reason: String },
    
    #[error("Invalid input data: {details}")]
    InvalidInput { details: String },
    
    #[error("Model loading failed: {path}")]
    ModelLoadingFailed { path: String },
    
    #[error("Feature extraction failed: {reason}")]
    FeatureExtractionFailed { reason: String },
    
    #[error("Timeout during inference")]
    InferenceTimeout,
    
    #[error("Resource exhausted: {resource}")]
    ResourceExhausted { resource: String },
}

/// Result type for ML operations
pub type MLResult<T> = Result<T, MLError>;
