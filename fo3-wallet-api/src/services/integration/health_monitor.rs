//! Health Monitor for Service Monitoring and Alerting
//! 
//! Provides comprehensive health checking and performance monitoring

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tokio::time::interval;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use tracing::{info, warn, error, instrument};

use crate::error::ServiceError;
use crate::database::connection::DatabasePool;
use crate::services::integration::event_dispatcher::{EventDispatcher, ServiceEvent};

/// Health status levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Critical,
}

/// Health check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResult {
    pub service_name: String,
    pub status: HealthStatus,
    pub response_time_ms: u64,
    pub error_message: Option<String>,
    pub metadata: HashMap<String, serde_json::Value>,
    pub checked_at: DateTime<Utc>,
}

/// Service metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceMetrics {
    pub service_name: String,
    pub request_count: u64,
    pub error_count: u64,
    pub average_response_time_ms: f64,
    pub last_request_at: Option<DateTime<Utc>>,
    pub uptime_percentage: f64,
    pub memory_usage_mb: Option<f64>,
    pub cpu_usage_percentage: Option<f64>,
}

/// Health monitor configuration
#[derive(Debug, Clone)]
pub struct HealthMonitorConfig {
    pub check_interval_seconds: u64,
    pub alert_threshold_response_time_ms: u64,
    pub alert_threshold_error_rate: f64,
    pub enable_auto_recovery: bool,
}

impl Default for HealthMonitorConfig {
    fn default() -> Self {
        Self {
            check_interval_seconds: 30,
            alert_threshold_response_time_ms: 5000,
            alert_threshold_error_rate: 0.05, // 5%
            enable_auto_recovery: true,
        }
    }
}

/// Health monitor for comprehensive service monitoring
pub struct HealthMonitor {
    /// Configuration
    config: HealthMonitorConfig,
    
    /// Database pool for health checks
    database_pool: DatabasePool,
    
    /// Event dispatcher for alerts
    event_dispatcher: Arc<EventDispatcher>,
    
    /// Health check results
    health_results: Arc<RwLock<HashMap<String, HealthCheckResult>>>,
    
    /// Service metrics
    service_metrics: Arc<RwLock<HashMap<String, ServiceMetrics>>>,
    
    /// Alert history
    alert_history: Arc<RwLock<Vec<HealthAlert>>>,
}

/// Health alert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthAlert {
    pub alert_id: String,
    pub service_name: String,
    pub alert_type: String,
    pub severity: String,
    pub message: String,
    pub triggered_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl HealthMonitor {
    /// Create a new health monitor
    pub fn new(
        database_pool: DatabasePool,
        event_dispatcher: Arc<EventDispatcher>,
        config: Option<HealthMonitorConfig>,
    ) -> Self {
        Self {
            config: config.unwrap_or_default(),
            database_pool,
            event_dispatcher,
            health_results: Arc::new(RwLock::new(HashMap::new())),
            service_metrics: Arc::new(RwLock::new(HashMap::new())),
            alert_history: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Start the health monitoring loop
    pub async fn start_monitoring(&self) -> Result<(), ServiceError> {
        info!("Starting health monitoring with interval: {}s", self.config.check_interval_seconds);
        
        let mut interval = interval(Duration::from_secs(self.config.check_interval_seconds));
        
        loop {
            interval.tick().await;
            
            if let Err(e) = self.perform_health_checks().await {
                error!("Health check cycle failed: {}", e);
            }
            
            if let Err(e) = self.analyze_health_trends().await {
                error!("Health trend analysis failed: {}", e);
            }
        }
    }

    /// Perform comprehensive health checks
    #[instrument(skip(self))]
    async fn perform_health_checks(&self) -> Result<(), ServiceError> {
        let start_time = Instant::now();
        
        // Check database health
        let db_result = self.check_database_health().await;
        self.update_health_result("database", db_result).await;
        
        // Check repository health
        let repo_result = self.check_repository_health().await;
        self.update_health_result("repositories", repo_result).await;
        
        // Check event system health
        let event_result = self.check_event_system_health().await;
        self.update_health_result("event_system", event_result).await;
        
        // Check system resources
        let system_result = self.check_system_resources().await;
        self.update_health_result("system_resources", system_result).await;
        
        let total_time = start_time.elapsed();
        info!("Health check cycle completed in {:?}", total_time);
        
        Ok(())
    }

    /// Check database health
    async fn check_database_health(&self) -> HealthCheckResult {
        let start_time = Instant::now();
        let mut metadata = HashMap::new();
        
        match self.database_pool.health_check().await {
            Ok(_) => {
                let stats = self.database_pool.get_stats();
                metadata.insert("pool_size".to_string(), serde_json::json!(stats.size));
                metadata.insert("idle_connections".to_string(), serde_json::json!(stats.idle));
                metadata.insert("is_closed".to_string(), serde_json::json!(stats.is_closed));
                
                HealthCheckResult {
                    service_name: "database".to_string(),
                    status: HealthStatus::Healthy,
                    response_time_ms: start_time.elapsed().as_millis() as u64,
                    error_message: None,
                    metadata,
                    checked_at: Utc::now(),
                }
            }
            Err(e) => {
                HealthCheckResult {
                    service_name: "database".to_string(),
                    status: HealthStatus::Critical,
                    response_time_ms: start_time.elapsed().as_millis() as u64,
                    error_message: Some(e.to_string()),
                    metadata,
                    checked_at: Utc::now(),
                }
            }
        }
    }

    /// Check repository health
    async fn check_repository_health(&self) -> HealthCheckResult {
        let start_time = Instant::now();
        let mut metadata = HashMap::new();
        let mut error_messages = Vec::new();
        
        // Test KYC repository
        // Note: In a real implementation, we would have access to the repositories
        // For now, we'll simulate the health check
        metadata.insert("kyc_repository".to_string(), serde_json::json!("healthy"));
        metadata.insert("wallet_repository".to_string(), serde_json::json!("healthy"));
        metadata.insert("card_repository".to_string(), serde_json::json!("healthy"));
        
        let status = if error_messages.is_empty() {
            HealthStatus::Healthy
        } else {
            HealthStatus::Degraded
        };
        
        HealthCheckResult {
            service_name: "repositories".to_string(),
            status,
            response_time_ms: start_time.elapsed().as_millis() as u64,
            error_message: if error_messages.is_empty() { None } else { Some(error_messages.join("; ")) },
            metadata,
            checked_at: Utc::now(),
        }
    }

    /// Check event system health
    async fn check_event_system_health(&self) -> HealthCheckResult {
        let start_time = Instant::now();
        let mut metadata = HashMap::new();
        
        let stats = self.event_dispatcher.get_stats().await;
        metadata.insert("total_events".to_string(), serde_json::json!(stats.total_events));
        metadata.insert("active_subscriptions".to_string(), serde_json::json!(stats.active_subscriptions));
        metadata.insert("last_event_time".to_string(), serde_json::json!(stats.last_event_time));
        
        HealthCheckResult {
            service_name: "event_system".to_string(),
            status: HealthStatus::Healthy,
            response_time_ms: start_time.elapsed().as_millis() as u64,
            error_message: None,
            metadata,
            checked_at: Utc::now(),
        }
    }

    /// Check system resources
    async fn check_system_resources(&self) -> HealthCheckResult {
        let start_time = Instant::now();
        let mut metadata = HashMap::new();
        
        // In a real implementation, you would check actual system metrics
        // For now, we'll simulate healthy system resources
        metadata.insert("memory_usage_mb".to_string(), serde_json::json!(512.0));
        metadata.insert("cpu_usage_percentage".to_string(), serde_json::json!(25.0));
        metadata.insert("disk_usage_percentage".to_string(), serde_json::json!(45.0));
        
        HealthCheckResult {
            service_name: "system_resources".to_string(),
            status: HealthStatus::Healthy,
            response_time_ms: start_time.elapsed().as_millis() as u64,
            error_message: None,
            metadata,
            checked_at: Utc::now(),
        }
    }

    /// Update health result and trigger alerts if needed
    async fn update_health_result(&self, service_name: &str, result: HealthCheckResult) {
        let previous_status = {
            let results = self.health_results.read().await;
            results.get(service_name).map(|r| r.status.clone())
        };
        
        // Store new result
        {
            let mut results = self.health_results.write().await;
            results.insert(service_name.to_string(), result.clone());
        }
        
        // Check for status changes and trigger alerts
        if let Some(prev_status) = previous_status {
            if prev_status != result.status {
                self.trigger_status_change_alert(service_name, prev_status, result.status.clone()).await;
            }
        }
        
        // Check for performance alerts
        if result.response_time_ms > self.config.alert_threshold_response_time_ms {
            self.trigger_performance_alert(service_name, result.response_time_ms).await;
        }
        
        // Publish health update event
        if let Err(e) = self.event_dispatcher.publish_event(
            ServiceEvent::HealthCheckUpdate {
                service_name: service_name.to_string(),
                status: format!("{:?}", result.status),
                timestamp: result.checked_at,
            },
            None,
            "health_monitor".to_string(),
            None,
        ).await {
            warn!("Failed to publish health update event: {}", e);
        }
    }

    /// Trigger status change alert
    async fn trigger_status_change_alert(
        &self,
        service_name: &str,
        old_status: HealthStatus,
        new_status: HealthStatus,
    ) {
        let severity = match new_status {
            HealthStatus::Critical => "critical",
            HealthStatus::Unhealthy => "high",
            HealthStatus::Degraded => "medium",
            HealthStatus::Healthy => "low",
        };
        
        let message = format!(
            "Service {} status changed from {:?} to {:?}",
            service_name, old_status, new_status
        );
        
        if let Err(e) = self.event_dispatcher.publish_system_alert(
            "status_change".to_string(),
            message,
            severity.to_string(),
        ).await {
            error!("Failed to publish status change alert: {}", e);
        }
    }

    /// Trigger performance alert
    async fn trigger_performance_alert(&self, service_name: &str, response_time_ms: u64) {
        let message = format!(
            "Service {} response time {}ms exceeds threshold {}ms",
            service_name, response_time_ms, self.config.alert_threshold_response_time_ms
        );
        
        if let Err(e) = self.event_dispatcher.publish_system_alert(
            "performance_degradation".to_string(),
            message,
            "medium".to_string(),
        ).await {
            error!("Failed to publish performance alert: {}", e);
        }
    }

    /// Analyze health trends and patterns
    async fn analyze_health_trends(&self) -> Result<(), ServiceError> {
        // In a real implementation, this would analyze historical data
        // and detect patterns, predict failures, etc.
        
        let results = self.health_results.read().await;
        let unhealthy_services: Vec<_> = results.iter()
            .filter(|(_, result)| matches!(result.status, HealthStatus::Unhealthy | HealthStatus::Critical))
            .map(|(name, _)| name.clone())
            .collect();
        
        if !unhealthy_services.is_empty() {
            warn!("Unhealthy services detected: {:?}", unhealthy_services);
        }
        
        Ok(())
    }

    /// Get current health status for all services
    pub async fn get_overall_health(&self) -> HashMap<String, HealthCheckResult> {
        self.health_results.read().await.clone()
    }

    /// Get service metrics
    pub async fn get_service_metrics(&self) -> HashMap<String, ServiceMetrics> {
        self.service_metrics.read().await.clone()
    }

    /// Record service request metrics
    pub async fn record_request_metrics(
        &self,
        service_name: String,
        response_time_ms: u64,
        is_error: bool,
    ) {
        let mut metrics = self.service_metrics.write().await;
        let service_metrics = metrics.entry(service_name.clone()).or_insert_with(|| ServiceMetrics {
            service_name: service_name.clone(),
            request_count: 0,
            error_count: 0,
            average_response_time_ms: 0.0,
            last_request_at: None,
            uptime_percentage: 100.0,
            memory_usage_mb: None,
            cpu_usage_percentage: None,
        });
        
        service_metrics.request_count += 1;
        if is_error {
            service_metrics.error_count += 1;
        }
        
        // Update average response time
        let total_time = service_metrics.average_response_time_ms * (service_metrics.request_count - 1) as f64;
        service_metrics.average_response_time_ms = (total_time + response_time_ms as f64) / service_metrics.request_count as f64;
        
        service_metrics.last_request_at = Some(Utc::now());
    }
}
