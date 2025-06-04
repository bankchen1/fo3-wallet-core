//! Database performance optimization and monitoring
//!
//! Provides query optimization, connection pool monitoring, and performance analytics.

use sqlx::{Pool, Postgres, Sqlite, Row, Executor};
use std::time::{Duration, Instant};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, error, debug, instrument};
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};

use crate::error::ServiceError;
use crate::database::connection::DatabasePool;

/// Database performance monitor
pub struct DatabasePerformanceMonitor {
    query_stats: Arc<RwLock<HashMap<String, QueryStats>>>,
    connection_stats: Arc<RwLock<ConnectionPoolStats>>,
    slow_query_threshold: Duration,
    enable_query_logging: bool,
}

/// Query performance statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryStats {
    pub query_hash: String,
    pub query_template: String,
    pub execution_count: u64,
    pub total_duration: Duration,
    pub average_duration: Duration,
    pub min_duration: Duration,
    pub max_duration: Duration,
    pub last_executed: DateTime<Utc>,
    pub error_count: u64,
    pub rows_affected_total: u64,
    pub cache_hit_count: u64,
}

/// Connection pool statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionPoolStats {
    pub total_connections: u32,
    pub active_connections: u32,
    pub idle_connections: u32,
    pub max_connections: u32,
    pub connection_wait_time_avg: Duration,
    pub connection_wait_time_max: Duration,
    pub connection_errors: u64,
    pub connection_timeouts: u64,
    pub pool_utilization: f64,
    pub last_updated: DateTime<Utc>,
}

/// Query execution result with performance metrics
#[derive(Debug)]
pub struct QueryExecutionResult<T> {
    pub result: T,
    pub execution_time: Duration,
    pub rows_affected: u64,
    pub cache_hit: bool,
}

/// Database index recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexRecommendation {
    pub table_name: String,
    pub column_names: Vec<String>,
    pub index_type: IndexType,
    pub estimated_improvement: f64,
    pub query_patterns: Vec<String>,
    pub priority: RecommendationPriority,
}

/// Index types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IndexType {
    BTree,
    Hash,
    Gin,
    Gist,
    Composite,
}

/// Recommendation priority
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecommendationPriority {
    High,
    Medium,
    Low,
}

impl DatabasePerformanceMonitor {
    /// Create new performance monitor
    pub fn new(slow_query_threshold_ms: u64, enable_query_logging: bool) -> Self {
        Self {
            query_stats: Arc::new(RwLock::new(HashMap::new())),
            connection_stats: Arc::new(RwLock::new(ConnectionPoolStats::default())),
            slow_query_threshold: Duration::from_millis(slow_query_threshold_ms),
            enable_query_logging,
        }
    }
    
    /// Execute query with performance monitoring
    #[instrument(skip(self, pool, query))]
    pub async fn execute_query<T, F, Fut>(
        &self,
        pool: &DatabasePool,
        query: &str,
        query_params: &str,
        executor: F,
    ) -> Result<QueryExecutionResult<T>, ServiceError>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T, sqlx::Error>>,
    {
        let start_time = Instant::now();
        let query_hash = self.hash_query(query);
        
        debug!("Executing query: {}", query_hash);
        
        // Execute the query
        let result = executor().await
            .map_err(|e| {
                error!("Query execution failed: {}", e);
                ServiceError::DatabaseError(format!("Query execution failed: {}", e))
            })?;
        
        let execution_time = start_time.elapsed();
        
        // Update query statistics
        self.update_query_stats(&query_hash, query, execution_time, true, 0).await;
        
        // Log slow queries
        if execution_time > self.slow_query_threshold {
            warn!(
                "Slow query detected: {} took {:?} (threshold: {:?})",
                query_hash, execution_time, self.slow_query_threshold
            );
            
            if self.enable_query_logging {
                info!("Slow query details: {}", query);
            }
        }
        
        Ok(QueryExecutionResult {
            result,
            execution_time,
            rows_affected: 0, // Would need to be determined based on query type
            cache_hit: false,
        })
    }
    
    /// Update query statistics
    async fn update_query_stats(
        &self,
        query_hash: &str,
        query: &str,
        execution_time: Duration,
        success: bool,
        rows_affected: u64,
    ) {
        let mut stats = self.query_stats.write().await;
        
        let query_stats = stats.entry(query_hash.to_string()).or_insert_with(|| QueryStats {
            query_hash: query_hash.to_string(),
            query_template: self.normalize_query(query),
            execution_count: 0,
            total_duration: Duration::ZERO,
            average_duration: Duration::ZERO,
            min_duration: execution_time,
            max_duration: execution_time,
            last_executed: Utc::now(),
            error_count: 0,
            rows_affected_total: 0,
            cache_hit_count: 0,
        });
        
        query_stats.execution_count += 1;
        query_stats.total_duration += execution_time;
        query_stats.average_duration = query_stats.total_duration / query_stats.execution_count as u32;
        query_stats.min_duration = query_stats.min_duration.min(execution_time);
        query_stats.max_duration = query_stats.max_duration.max(execution_time);
        query_stats.last_executed = Utc::now();
        query_stats.rows_affected_total += rows_affected;
        
        if !success {
            query_stats.error_count += 1;
        }
    }
    
    /// Update connection pool statistics
    pub async fn update_connection_stats(&self, pool: &DatabasePool) -> Result<(), ServiceError> {
        let mut stats = self.connection_stats.write().await;
        
        match pool {
            DatabasePool::Postgres(pg_pool) => {
                stats.total_connections = pg_pool.size();
                stats.active_connections = pg_pool.size() - pg_pool.num_idle();
                stats.idle_connections = pg_pool.num_idle();
                stats.max_connections = pg_pool.options().get_max_connections();
            }
            DatabasePool::Sqlite(sqlite_pool) => {
                stats.total_connections = sqlite_pool.size();
                stats.active_connections = sqlite_pool.size() - sqlite_pool.num_idle();
                stats.idle_connections = sqlite_pool.num_idle();
                stats.max_connections = sqlite_pool.options().get_max_connections();
            }
        }
        
        stats.pool_utilization = stats.active_connections as f64 / stats.max_connections as f64;
        stats.last_updated = Utc::now();
        
        // Log high pool utilization
        if stats.pool_utilization > 0.8 {
            warn!(
                "High database pool utilization: {:.1}% ({}/{})",
                stats.pool_utilization * 100.0,
                stats.active_connections,
                stats.max_connections
            );
        }
        
        Ok(())
    }
    
    /// Get query performance statistics
    pub async fn get_query_stats(&self) -> HashMap<String, QueryStats> {
        let stats = self.query_stats.read().await;
        stats.clone()
    }
    
    /// Get connection pool statistics
    pub async fn get_connection_stats(&self) -> ConnectionPoolStats {
        let stats = self.connection_stats.read().await;
        stats.clone()
    }
    
    /// Get slow queries
    pub async fn get_slow_queries(&self, limit: usize) -> Vec<QueryStats> {
        let stats = self.query_stats.read().await;
        let mut slow_queries: Vec<QueryStats> = stats
            .values()
            .filter(|stat| stat.average_duration > self.slow_query_threshold)
            .cloned()
            .collect();
        
        slow_queries.sort_by(|a, b| b.average_duration.cmp(&a.average_duration));
        slow_queries.truncate(limit);
        slow_queries
    }
    
    /// Generate index recommendations
    pub async fn generate_index_recommendations(&self) -> Vec<IndexRecommendation> {
        let stats = self.query_stats.read().await;
        let mut recommendations = Vec::new();
        
        for query_stat in stats.values() {
            if query_stat.average_duration > self.slow_query_threshold && query_stat.execution_count > 10 {
                // Analyze query pattern for potential index recommendations
                if let Some(recommendation) = self.analyze_query_for_index(&query_stat.query_template) {
                    recommendations.push(recommendation);
                }
            }
        }
        
        recommendations
    }
    
    /// Analyze query for index recommendations
    fn analyze_query_for_index(&self, query: &str) -> Option<IndexRecommendation> {
        let query_lower = query.to_lowercase();
        
        // Simple heuristics for index recommendations
        if query_lower.contains("where") && query_lower.contains("=") {
            // Extract table and column information (simplified)
            if let Some(table_name) = self.extract_table_name(&query_lower) {
                if let Some(column_name) = self.extract_where_column(&query_lower) {
                    return Some(IndexRecommendation {
                        table_name,
                        column_names: vec![column_name],
                        index_type: IndexType::BTree,
                        estimated_improvement: 0.3, // 30% improvement estimate
                        query_patterns: vec![query.to_string()],
                        priority: RecommendationPriority::Medium,
                    });
                }
            }
        }
        
        None
    }
    
    /// Extract table name from query (simplified)
    fn extract_table_name(&self, query: &str) -> Option<String> {
        // Very basic table name extraction
        if let Some(from_pos) = query.find("from ") {
            let after_from = &query[from_pos + 5..];
            if let Some(space_pos) = after_from.find(' ') {
                return Some(after_from[..space_pos].trim().to_string());
            }
        }
        None
    }
    
    /// Extract WHERE column name (simplified)
    fn extract_where_column(&self, query: &str) -> Option<String> {
        // Very basic WHERE column extraction
        if let Some(where_pos) = query.find("where ") {
            let after_where = &query[where_pos + 6..];
            if let Some(equals_pos) = after_where.find('=') {
                let column_part = after_where[..equals_pos].trim();
                return Some(column_part.to_string());
            }
        }
        None
    }
    
    /// Hash query for statistics tracking
    fn hash_query(&self, query: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let normalized = self.normalize_query(query);
        let mut hasher = DefaultHasher::new();
        normalized.hash(&mut hasher);
        format!("query_{:x}", hasher.finish())
    }
    
    /// Normalize query for pattern matching
    fn normalize_query(&self, query: &str) -> String {
        // Replace parameter placeholders with generic markers
        let normalized = query
            .replace(|c: char| c.is_numeric(), "?")
            .replace("'", "")
            .replace("\"", "");
        
        // Remove extra whitespace
        normalized
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .to_lowercase()
    }
    
    /// Reset statistics
    pub async fn reset_stats(&self) {
        let mut query_stats = self.query_stats.write().await;
        let mut connection_stats = self.connection_stats.write().await;
        
        query_stats.clear();
        *connection_stats = ConnectionPoolStats::default();
        
        info!("Database performance statistics reset");
    }
    
    /// Get performance summary
    pub async fn get_performance_summary(&self) -> PerformanceSummary {
        let query_stats = self.query_stats.read().await;
        let connection_stats = self.connection_stats.read().await;
        
        let total_queries = query_stats.values().map(|s| s.execution_count).sum();
        let total_errors = query_stats.values().map(|s| s.error_count).sum();
        let slow_query_count = query_stats
            .values()
            .filter(|s| s.average_duration > self.slow_query_threshold)
            .count();
        
        PerformanceSummary {
            total_queries,
            total_errors,
            slow_query_count,
            error_rate: if total_queries > 0 { total_errors as f64 / total_queries as f64 } else { 0.0 },
            pool_utilization: connection_stats.pool_utilization,
            average_query_time: if !query_stats.is_empty() {
                query_stats.values().map(|s| s.average_duration).sum::<Duration>() / query_stats.len() as u32
            } else {
                Duration::ZERO
            },
        }
    }
}

impl Default for ConnectionPoolStats {
    fn default() -> Self {
        Self {
            total_connections: 0,
            active_connections: 0,
            idle_connections: 0,
            max_connections: 0,
            connection_wait_time_avg: Duration::ZERO,
            connection_wait_time_max: Duration::ZERO,
            connection_errors: 0,
            connection_timeouts: 0,
            pool_utilization: 0.0,
            last_updated: Utc::now(),
        }
    }
}

/// Performance summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceSummary {
    pub total_queries: u64,
    pub total_errors: u64,
    pub slow_query_count: usize,
    pub error_rate: f64,
    pub pool_utilization: f64,
    pub average_query_time: Duration,
}

/// Database optimization recommendations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationRecommendations {
    pub index_recommendations: Vec<IndexRecommendation>,
    pub query_optimizations: Vec<String>,
    pub connection_pool_recommendations: Vec<String>,
    pub general_recommendations: Vec<String>,
}
