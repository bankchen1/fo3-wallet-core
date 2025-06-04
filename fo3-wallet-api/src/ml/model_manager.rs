//! ML Model Manager
//! 
//! Handles model lifecycle management including:
//! - Model loading and unloading
//! - Version management
//! - Performance monitoring
//! - A/B testing capabilities
//! - Model deployment and rollback

use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::RwLock;
use anyhow::Result;
use tracing::{info, warn, error, instrument};
use chrono::{DateTime, Utc};
use moka::future::Cache;

use super::{
    ModelMetadata, ModelType, DeploymentStatus, InferenceRequest, InferenceResponse,
    MLService, MLError, MLResult, MLConfig
};

/// Model manager for handling ML model lifecycle
pub struct ModelManager {
    models: Arc<RwLock<HashMap<String, Arc<dyn MLService>>>>,
    metadata: Arc<RwLock<HashMap<String, ModelMetadata>>>,
    config: MLConfig,
    inference_cache: Cache<String, InferenceResponse>,
    performance_metrics: Arc<RwLock<HashMap<String, ModelPerformanceMetrics>>>,
}

/// Model performance metrics
#[derive(Debug, Clone)]
pub struct ModelPerformanceMetrics {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub average_latency_ms: f64,
    pub p95_latency_ms: f64,
    pub p99_latency_ms: f64,
    pub last_updated: DateTime<Utc>,
    pub accuracy_score: f64,
    pub confidence_distribution: Vec<f64>,
}

impl ModelManager {
    /// Create a new model manager
    pub fn new(config: MLConfig) -> Self {
        let cache = Cache::builder()
            .max_capacity(10000)
            .time_to_live(std::time::Duration::from_secs(config.cache_ttl_seconds))
            .build();

        Self {
            models: Arc::new(RwLock::new(HashMap::new())),
            metadata: Arc::new(RwLock::new(HashMap::new())),
            config,
            inference_cache: cache,
            performance_metrics: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Load a model from storage
    #[instrument(skip(self))]
    pub async fn load_model(&self, model_id: &str, model_path: &str) -> MLResult<()> {
        info!(model_id = %model_id, model_path = %model_path, "Loading ML model");

        // Load model metadata
        let metadata = self.load_model_metadata(model_path).await?;
        
        // Create model service based on type
        let model_service = self.create_model_service(&metadata, model_path).await?;
        
        // Store model and metadata
        {
            let mut models = self.models.write().await;
            models.insert(model_id.to_string(), model_service);
        }
        
        {
            let mut metadata_map = self.metadata.write().await;
            metadata_map.insert(model_id.to_string(), metadata);
        }

        // Initialize performance metrics
        {
            let mut metrics = self.performance_metrics.write().await;
            metrics.insert(model_id.to_string(), ModelPerformanceMetrics::new());
        }

        info!(model_id = %model_id, "Model loaded successfully");
        Ok(())
    }

    /// Unload a model from memory
    #[instrument(skip(self))]
    pub async fn unload_model(&self, model_id: &str) -> MLResult<()> {
        info!(model_id = %model_id, "Unloading ML model");

        {
            let mut models = self.models.write().await;
            models.remove(model_id);
        }

        {
            let mut metadata_map = self.metadata.write().await;
            metadata_map.remove(model_id);
        }

        {
            let mut metrics = self.performance_metrics.write().await;
            metrics.remove(model_id);
        }

        info!(model_id = %model_id, "Model unloaded successfully");
        Ok(())
    }

    /// Get model inference with caching
    #[instrument(skip(self, request))]
    pub async fn predict(&self, request: InferenceRequest) -> MLResult<InferenceResponse> {
        let start_time = std::time::Instant::now();
        
        // Check cache first
        let cache_key = self.generate_cache_key(&request);
        if let Some(cached_response) = self.inference_cache.get(&cache_key).await {
            return Ok(cached_response);
        }

        // Get model service
        let model_service = {
            let models = self.models.read().await;
            models.get(&request.model_id)
                .ok_or_else(|| MLError::ModelNotFound { 
                    model_id: request.model_id.clone() 
                })?
                .clone()
        };

        // Perform inference
        let response = model_service.predict(request.clone()).await
            .map_err(|e| MLError::InferenceFailed { 
                reason: e.to_string() 
            })?;

        // Update performance metrics
        let latency_ms = start_time.elapsed().as_millis() as f64;
        self.update_performance_metrics(&request.model_id, latency_ms, true).await;

        // Cache response
        self.inference_cache.insert(cache_key, response.clone()).await;

        Ok(response)
    }

    /// Batch prediction with optimized processing
    #[instrument(skip(self, requests))]
    pub async fn batch_predict(&self, requests: Vec<InferenceRequest>) -> MLResult<Vec<InferenceResponse>> {
        if requests.is_empty() {
            return Ok(vec![]);
        }

        // Group requests by model
        let mut model_requests: HashMap<String, Vec<InferenceRequest>> = HashMap::new();
        for request in requests {
            model_requests.entry(request.model_id.clone())
                .or_insert_with(Vec::new)
                .push(request);
        }

        let mut all_responses = Vec::new();

        // Process each model's requests
        for (model_id, model_requests) in model_requests {
            let model_service = {
                let models = self.models.read().await;
                models.get(&model_id)
                    .ok_or_else(|| MLError::ModelNotFound { 
                        model_id: model_id.clone() 
                    })?
                    .clone()
            };

            let responses = model_service.batch_predict(model_requests).await
                .map_err(|e| MLError::InferenceFailed { 
                    reason: e.to_string() 
                })?;

            all_responses.extend(responses);
        }

        Ok(all_responses)
    }

    /// Get model metadata
    pub async fn get_model_metadata(&self, model_id: &str) -> MLResult<ModelMetadata> {
        let metadata_map = self.metadata.read().await;
        metadata_map.get(model_id)
            .cloned()
            .ok_or_else(|| MLError::ModelNotFound { 
                model_id: model_id.to_string() 
            })
    }

    /// List all loaded models
    pub async fn list_models(&self) -> Vec<String> {
        let models = self.models.read().await;
        models.keys().cloned().collect()
    }

    /// Get model performance metrics
    pub async fn get_performance_metrics(&self, model_id: &str) -> Option<ModelPerformanceMetrics> {
        let metrics = self.performance_metrics.read().await;
        metrics.get(model_id).cloned()
    }

    /// Health check for all models
    #[instrument(skip(self))]
    pub async fn health_check(&self) -> HashMap<String, bool> {
        let models = self.models.read().await;
        let mut health_status = HashMap::new();

        for (model_id, model_service) in models.iter() {
            let is_healthy = model_service.health_check().await.unwrap_or(false);
            health_status.insert(model_id.clone(), is_healthy);
        }

        health_status
    }

    /// Load model metadata from file
    async fn load_model_metadata(&self, model_path: &str) -> MLResult<ModelMetadata> {
        // Implementation would load from actual metadata file
        // For now, return a mock metadata
        Ok(ModelMetadata {
            model_id: "sentiment_v1".to_string(),
            model_name: "Crypto Sentiment Analyzer".to_string(),
            version: "1.0.0".to_string(),
            model_type: ModelType::SentimentAnalysis,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            accuracy: 0.85,
            confidence_threshold: 0.7,
            input_features: vec!["text".to_string(), "source".to_string()],
            output_schema: "sentiment_score".to_string(),
            deployment_status: DeploymentStatus::Production,
        })
    }

    /// Create model service based on type
    async fn create_model_service(&self, metadata: &ModelMetadata, model_path: &str) -> MLResult<Arc<dyn MLService>> {
        match metadata.model_type {
            ModelType::SentimentAnalysis => {
                let service = crate::ml::sentiment_analyzer::SentimentAnalyzer::new(model_path).await?;
                Ok(Arc::new(service))
            },
            ModelType::YieldPrediction => {
                let service = crate::ml::yield_predictor::YieldPredictor::new(model_path).await?;
                Ok(Arc::new(service))
            },
            _ => Err(MLError::ModelLoadingFailed { 
                path: model_path.to_string() 
            })
        }
    }

    /// Generate cache key for inference request
    fn generate_cache_key(&self, request: &InferenceRequest) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        request.model_id.hash(&mut hasher);
        request.input_data.to_string().hash(&mut hasher);
        format!("inference_{}", hasher.finish())
    }

    /// Update performance metrics
    async fn update_performance_metrics(&self, model_id: &str, latency_ms: f64, success: bool) {
        let mut metrics = self.performance_metrics.write().await;
        if let Some(model_metrics) = metrics.get_mut(model_id) {
            model_metrics.total_requests += 1;
            if success {
                model_metrics.successful_requests += 1;
            } else {
                model_metrics.failed_requests += 1;
            }
            
            // Update latency metrics (simplified)
            model_metrics.average_latency_ms = 
                (model_metrics.average_latency_ms + latency_ms) / 2.0;
            model_metrics.last_updated = Utc::now();
        }
    }
}

impl ModelPerformanceMetrics {
    fn new() -> Self {
        Self {
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            average_latency_ms: 0.0,
            p95_latency_ms: 0.0,
            p99_latency_ms: 0.0,
            last_updated: Utc::now(),
            accuracy_score: 0.0,
            confidence_distribution: vec![],
        }
    }
}
