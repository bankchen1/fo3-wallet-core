//! ML Data Pipeline
//! 
//! Provides comprehensive data processing pipeline including:
//! - Real-time data ingestion from multiple sources
//! - Data cleaning and preprocessing
//! - Feature engineering and transformation
//! - Data validation and quality checks
//! - Streaming data processing

use std::sync::Arc;
use std::collections::HashMap;
use anyhow::Result;
use tracing::{info, warn, error, instrument};
use chrono::{DateTime, Utc, Duration};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use super::{
    MarketDataPoint, TechnicalIndicators, SentimentData, EngagementMetrics,
    FeatureVector, MLError, MLResult
};

/// Data pipeline for ML processing
pub struct DataPipeline {
    config: DataPipelineConfig,
    data_sources: Arc<RwLock<HashMap<String, Box<dyn DataSource>>>>,
    processors: Arc<RwLock<HashMap<String, Box<dyn DataProcessor>>>>,
    validators: Arc<RwLock<HashMap<String, Box<dyn DataValidator>>>>,
    data_cache: Arc<RwLock<DataCache>>,
    streaming_manager: Arc<StreamingManager>,
}

/// Data pipeline configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataPipelineConfig {
    pub batch_size: usize,
    pub processing_interval_ms: u64,
    pub cache_ttl_seconds: u64,
    pub max_cache_size: usize,
    pub data_sources: Vec<DataSourceConfig>,
    pub quality_thresholds: QualityThresholds,
    pub feature_engineering: FeatureEngineeringConfig,
}

/// Data source configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSourceConfig {
    pub source_id: String,
    pub source_type: String,
    pub endpoint: String,
    pub api_key: Option<String>,
    pub rate_limit: u32,
    pub enabled: bool,
    pub priority: u32,
}

/// Data quality thresholds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityThresholds {
    pub min_completeness: f64,
    pub max_staleness_minutes: u32,
    pub min_accuracy_score: f64,
    pub max_outlier_percentage: f64,
}

/// Feature engineering configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureEngineeringConfig {
    pub technical_indicators: bool,
    pub sentiment_features: bool,
    pub time_features: bool,
    pub lag_features: Vec<u32>,
    pub rolling_windows: Vec<u32>,
    pub normalization_method: String,
}

/// Data cache
#[derive(Debug, Clone)]
pub struct DataCache {
    pub market_data: HashMap<String, Vec<MarketDataPoint>>,
    pub sentiment_data: HashMap<String, Vec<SentimentData>>,
    pub feature_vectors: HashMap<String, Vec<FeatureVector>>,
    pub last_updated: HashMap<String, DateTime<Utc>>,
}

/// Streaming data manager
pub struct StreamingManager {
    active_streams: Arc<RwLock<HashMap<String, StreamingConnection>>>,
    stream_config: StreamingConfig,
}

/// Streaming configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingConfig {
    pub buffer_size: usize,
    pub flush_interval_ms: u64,
    pub reconnect_attempts: u32,
    pub heartbeat_interval_ms: u64,
}

/// Streaming connection
#[derive(Debug, Clone)]
pub struct StreamingConnection {
    pub connection_id: String,
    pub source_type: String,
    pub status: ConnectionStatus,
    pub last_heartbeat: DateTime<Utc>,
    pub messages_processed: u64,
    pub errors_count: u64,
}

/// Connection status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConnectionStatus {
    Connected,
    Disconnected,
    Reconnecting,
    Error,
}

/// Data source trait
#[async_trait::async_trait]
pub trait DataSource: Send + Sync {
    async fn fetch_data(&self, params: &DataFetchParams) -> Result<RawData>;
    async fn validate_connection(&self) -> Result<bool>;
    fn get_source_info(&self) -> DataSourceInfo;
}

/// Data processor trait
#[async_trait::async_trait]
pub trait DataProcessor: Send + Sync {
    async fn process(&self, raw_data: RawData) -> Result<ProcessedData>;
    fn get_processor_info(&self) -> ProcessorInfo;
}

/// Data validator trait
#[async_trait::async_trait]
pub trait DataValidator: Send + Sync {
    async fn validate(&self, data: &ProcessedData) -> Result<ValidationResult>;
    fn get_validation_rules(&self) -> Vec<ValidationRule>;
}

/// Data fetch parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataFetchParams {
    pub symbols: Vec<String>,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub granularity: String,
    pub data_types: Vec<String>,
}

/// Raw data from sources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawData {
    pub source_id: String,
    pub data_type: String,
    pub timestamp: DateTime<Utc>,
    pub data: serde_json::Value,
    pub metadata: HashMap<String, String>,
}

/// Processed data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessedData {
    pub data_id: String,
    pub processed_at: DateTime<Utc>,
    pub market_data: Option<Vec<MarketDataPoint>>,
    pub sentiment_data: Option<Vec<SentimentData>>,
    pub feature_vectors: Option<Vec<FeatureVector>>,
    pub quality_score: f64,
}

/// Data source information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSourceInfo {
    pub source_id: String,
    pub source_type: String,
    pub description: String,
    pub supported_data_types: Vec<String>,
    pub rate_limit: u32,
    pub reliability_score: f64,
}

/// Processor information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessorInfo {
    pub processor_id: String,
    pub processor_type: String,
    pub description: String,
    pub input_types: Vec<String>,
    pub output_types: Vec<String>,
}

/// Validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub quality_score: f64,
    pub issues: Vec<ValidationIssue>,
    pub recommendations: Vec<String>,
}

/// Validation rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationRule {
    pub rule_id: String,
    pub rule_type: String,
    pub description: String,
    pub severity: ValidationSeverity,
    pub threshold: f64,
}

/// Validation issue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationIssue {
    pub issue_type: String,
    pub severity: ValidationSeverity,
    pub description: String,
    pub affected_fields: Vec<String>,
    pub suggested_fix: String,
}

/// Validation severity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Pipeline processing result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineResult {
    pub pipeline_id: String,
    pub processed_at: DateTime<Utc>,
    pub input_records: u64,
    pub output_records: u64,
    pub processing_time_ms: u64,
    pub quality_metrics: QualityMetrics,
    pub errors: Vec<PipelineError>,
    pub warnings: Vec<PipelineWarning>,
}

/// Quality metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityMetrics {
    pub completeness: f64,
    pub accuracy: f64,
    pub timeliness: f64,
    pub consistency: f64,
    pub overall_score: f64,
}

/// Pipeline error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineError {
    pub error_type: String,
    pub message: String,
    pub timestamp: DateTime<Utc>,
    pub affected_records: u64,
}

/// Pipeline warning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineWarning {
    pub warning_type: String,
    pub message: String,
    pub timestamp: DateTime<Utc>,
    pub severity: ValidationSeverity,
}

impl DataPipeline {
    /// Create a new data pipeline
    pub async fn new(config: DataPipelineConfig) -> MLResult<Self> {
        let data_cache = DataCache {
            market_data: HashMap::new(),
            sentiment_data: HashMap::new(),
            feature_vectors: HashMap::new(),
            last_updated: HashMap::new(),
        };

        let streaming_manager = Arc::new(StreamingManager::new(StreamingConfig::default()));

        let pipeline = Self {
            config,
            data_sources: Arc::new(RwLock::new(HashMap::new())),
            processors: Arc::new(RwLock::new(HashMap::new())),
            validators: Arc::new(RwLock::new(HashMap::new())),
            data_cache: Arc::new(RwLock::new(data_cache)),
            streaming_manager,
        };

        // Initialize data sources
        pipeline.initialize_data_sources().await?;
        
        // Initialize processors
        pipeline.initialize_processors().await?;
        
        // Initialize validators
        pipeline.initialize_validators().await?;

        Ok(pipeline)
    }

    /// Process data through the pipeline
    #[instrument(skip(self, input_data))]
    pub async fn process_data(&self, input_data: Vec<RawData>) -> MLResult<PipelineResult> {
        let start_time = std::time::Instant::now();
        let pipeline_id = uuid::Uuid::new_v4().to_string();
        
        info!(pipeline_id = %pipeline_id, input_records = %input_data.len(), "Starting data pipeline processing");

        let mut processed_data = Vec::new();
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Process each data item
        for raw_data in &input_data {
            match self.process_single_item(raw_data.clone()).await {
                Ok(data) => processed_data.push(data),
                Err(e) => {
                    errors.push(PipelineError {
                        error_type: "processing_error".to_string(),
                        message: e.to_string(),
                        timestamp: Utc::now(),
                        affected_records: 1,
                    });
                }
            }
        }

        // Validate processed data
        for data in &processed_data {
            match self.validate_data(data).await {
                Ok(validation_result) => {
                    if !validation_result.is_valid {
                        warnings.push(PipelineWarning {
                            warning_type: "validation_warning".to_string(),
                            message: format!("Data quality issues detected: {:?}", validation_result.issues),
                            timestamp: Utc::now(),
                            severity: ValidationSeverity::Medium,
                        });
                    }
                },
                Err(e) => {
                    errors.push(PipelineError {
                        error_type: "validation_error".to_string(),
                        message: e.to_string(),
                        timestamp: Utc::now(),
                        affected_records: 1,
                    });
                }
            }
        }

        // Update cache
        self.update_cache(&processed_data).await?;

        // Calculate quality metrics
        let quality_metrics = self.calculate_quality_metrics(&processed_data).await;

        let processing_time = start_time.elapsed().as_millis() as u64;

        info!(
            pipeline_id = %pipeline_id,
            input_records = %input_data.len(),
            output_records = %processed_data.len(),
            processing_time_ms = %processing_time,
            "Data pipeline processing completed"
        );

        Ok(PipelineResult {
            pipeline_id,
            processed_at: Utc::now(),
            input_records: input_data.len() as u64,
            output_records: processed_data.len() as u64,
            processing_time_ms: processing_time,
            quality_metrics,
            errors,
            warnings,
        })
    }

    /// Process a single data item
    async fn process_single_item(&self, raw_data: RawData) -> MLResult<ProcessedData> {
        // Get appropriate processor
        let processors = self.processors.read().await;
        let processor = processors.get(&raw_data.data_type)
            .ok_or_else(|| MLError::FeatureExtractionFailed {
                reason: format!("No processor found for data type: {}", raw_data.data_type),
            })?;

        // Process the data
        processor.process(raw_data).await
            .map_err(|e| MLError::FeatureExtractionFailed {
                reason: e.to_string(),
            })
    }

    /// Validate processed data
    async fn validate_data(&self, data: &ProcessedData) -> MLResult<ValidationResult> {
        let validators = self.validators.read().await;
        
        // Use default validator if specific one not found
        let validator = validators.values().next()
            .ok_or_else(|| MLError::FeatureExtractionFailed {
                reason: "No validators available".to_string(),
            })?;

        validator.validate(data).await
            .map_err(|e| MLError::FeatureExtractionFailed {
                reason: e.to_string(),
            })
    }

    /// Update data cache
    async fn update_cache(&self, processed_data: &[ProcessedData]) -> MLResult<()> {
        let mut cache = self.data_cache.write().await;
        
        for data in processed_data {
            // Update market data cache
            if let Some(market_data) = &data.market_data {
                for point in market_data {
                    cache.market_data.entry(point.symbol.clone())
                        .or_insert_with(Vec::new)
                        .push(point.clone());
                }
            }

            // Update sentiment data cache
            if let Some(sentiment_data) = &data.sentiment_data {
                for sentiment in sentiment_data {
                    cache.sentiment_data.entry(sentiment.symbol.clone())
                        .or_insert_with(Vec::new)
                        .push(sentiment.clone());
                }
            }

            // Update feature vectors cache
            if let Some(feature_vectors) = &data.feature_vectors {
                for vector in feature_vectors {
                    cache.feature_vectors.entry("default".to_string())
                        .or_insert_with(Vec::new)
                        .push(vector.clone());
                }
            }

            cache.last_updated.insert(data.data_id.clone(), Utc::now());
        }

        // Clean old data
        self.clean_cache(&mut cache).await;

        Ok(())
    }

    /// Clean old data from cache
    async fn clean_cache(&self, cache: &mut DataCache) {
        let cutoff = Utc::now() - Duration::seconds(self.config.cache_ttl_seconds as i64);

        // Clean market data
        for data_vec in cache.market_data.values_mut() {
            data_vec.retain(|point| point.timestamp > cutoff);
        }

        // Clean sentiment data
        for data_vec in cache.sentiment_data.values_mut() {
            data_vec.retain(|sentiment| sentiment.timestamp > cutoff);
        }

        // Clean feature vectors
        for data_vec in cache.feature_vectors.values_mut() {
            data_vec.retain(|vector| vector.timestamp > cutoff);
        }
    }

    /// Calculate quality metrics
    async fn calculate_quality_metrics(&self, processed_data: &[ProcessedData]) -> QualityMetrics {
        if processed_data.is_empty() {
            return QualityMetrics {
                completeness: 0.0,
                accuracy: 0.0,
                timeliness: 0.0,
                consistency: 0.0,
                overall_score: 0.0,
            };
        }

        let completeness = processed_data.iter()
            .map(|d| if d.market_data.is_some() || d.sentiment_data.is_some() { 1.0 } else { 0.0 })
            .sum::<f64>() / processed_data.len() as f64;

        let accuracy = processed_data.iter()
            .map(|d| d.quality_score)
            .sum::<f64>() / processed_data.len() as f64;

        let timeliness = 0.9; // Mock calculation
        let consistency = 0.85; // Mock calculation

        let overall_score = (completeness + accuracy + timeliness + consistency) / 4.0;

        QualityMetrics {
            completeness,
            accuracy,
            timeliness,
            consistency,
            overall_score,
        }
    }

    /// Get cached data
    pub async fn get_cached_data(&self, data_type: &str, symbol: &str) -> Option<Vec<MarketDataPoint>> {
        let cache = self.data_cache.read().await;
        cache.market_data.get(symbol).cloned()
    }

    /// Initialize data sources
    async fn initialize_data_sources(&self) -> MLResult<()> {
        // Implementation would register actual data sources
        info!("Initializing data sources");
        Ok(())
    }

    /// Initialize processors
    async fn initialize_processors(&self) -> MLResult<()> {
        // Implementation would register actual processors
        info!("Initializing data processors");
        Ok(())
    }

    /// Initialize validators
    async fn initialize_validators(&self) -> MLResult<()> {
        // Implementation would register actual validators
        info!("Initializing data validators");
        Ok(())
    }
}

impl StreamingManager {
    fn new(config: StreamingConfig) -> Self {
        Self {
            active_streams: Arc::new(RwLock::new(HashMap::new())),
            stream_config: config,
        }
    }

    /// Start streaming connection
    pub async fn start_stream(&self, source_id: &str, source_type: &str) -> MLResult<String> {
        let connection_id = uuid::Uuid::new_v4().to_string();
        let connection = StreamingConnection {
            connection_id: connection_id.clone(),
            source_type: source_type.to_string(),
            status: ConnectionStatus::Connected,
            last_heartbeat: Utc::now(),
            messages_processed: 0,
            errors_count: 0,
        };

        let mut streams = self.active_streams.write().await;
        streams.insert(connection_id.clone(), connection);

        info!(connection_id = %connection_id, source_type = %source_type, "Started streaming connection");
        Ok(connection_id)
    }

    /// Stop streaming connection
    pub async fn stop_stream(&self, connection_id: &str) -> MLResult<()> {
        let mut streams = self.active_streams.write().await;
        streams.remove(connection_id);
        
        info!(connection_id = %connection_id, "Stopped streaming connection");
        Ok(())
    }

    /// Get stream status
    pub async fn get_stream_status(&self, connection_id: &str) -> Option<StreamingConnection> {
        let streams = self.active_streams.read().await;
        streams.get(connection_id).cloned()
    }
}

impl Default for DataPipelineConfig {
    fn default() -> Self {
        Self {
            batch_size: 100,
            processing_interval_ms: 1000,
            cache_ttl_seconds: 3600,
            max_cache_size: 10000,
            data_sources: vec![],
            quality_thresholds: QualityThresholds {
                min_completeness: 0.8,
                max_staleness_minutes: 5,
                min_accuracy_score: 0.7,
                max_outlier_percentage: 0.05,
            },
            feature_engineering: FeatureEngineeringConfig {
                technical_indicators: true,
                sentiment_features: true,
                time_features: true,
                lag_features: vec![1, 6, 24],
                rolling_windows: vec![24, 168, 720],
                normalization_method: "z_score".to_string(),
            },
        }
    }
}

impl Default for StreamingConfig {
    fn default() -> Self {
        Self {
            buffer_size: 1000,
            flush_interval_ms: 5000,
            reconnect_attempts: 3,
            heartbeat_interval_ms: 30000,
        }
    }
}
