//! Rate limiting middleware

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tonic::{Request, Status};

use crate::middleware::auth::AuthContext;

/// Rate limit bucket for token bucket algorithm
#[derive(Debug, Clone)]
pub struct RateLimitBucket {
    pub tokens: f64,
    pub last_refill: Instant,
    pub requests_today: u32,
    pub daily_reset: Instant,
}

impl RateLimitBucket {
    pub fn new(capacity: f64) -> Self {
        let now = Instant::now();
        Self {
            tokens: capacity,
            last_refill: now,
            requests_today: 0,
            daily_reset: now + Duration::from_secs(86400), // 24 hours
        }
    }

    /// Try to consume tokens from the bucket
    pub fn try_consume(&mut self, tokens: f64, refill_rate: f64, burst_limit: f64, daily_limit: u32) -> bool {
        let now = Instant::now();

        // Reset daily counter if needed
        if now >= self.daily_reset {
            self.requests_today = 0;
            self.daily_reset = now + Duration::from_secs(86400);
        }

        // Check daily limit
        if self.requests_today >= daily_limit {
            return false;
        }

        // Refill tokens based on time elapsed
        let time_elapsed = now.duration_since(self.last_refill).as_secs_f64();
        let tokens_to_add = time_elapsed * refill_rate / 60.0; // refill_rate is per minute
        self.tokens = (self.tokens + tokens_to_add).min(burst_limit);
        self.last_refill = now;

        // Try to consume tokens
        if self.tokens >= tokens {
            self.tokens -= tokens;
            self.requests_today += 1;
            true
        } else {
            false
        }
    }
}

/// Rate limiter service
pub struct RateLimiter {
    buckets: Arc<RwLock<HashMap<String, RateLimitBucket>>>,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self {
            buckets: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Check rate limit for a user
    pub async fn check_rate_limit(
        &self,
        user_id: &str,
        requests_per_minute: f64,
        burst_limit: f64,
        daily_limit: u32,
    ) -> Result<(), Status> {
        let mut buckets = self.buckets.write().await;
        
        let bucket = buckets
            .entry(user_id.to_string())
            .or_insert_with(|| RateLimitBucket::new(burst_limit));

        if bucket.try_consume(1.0, requests_per_minute, burst_limit, daily_limit) {
            Ok(())
        } else {
            // Calculate retry after time
            let retry_after = if bucket.requests_today >= daily_limit {
                // Daily limit exceeded, retry after daily reset
                bucket.daily_reset.duration_since(Instant::now()).as_secs()
            } else {
                // Rate limit exceeded, retry after token refill
                let tokens_needed = 1.0 - bucket.tokens;
                let time_to_refill = (tokens_needed * 60.0 / requests_per_minute) as u64;
                time_to_refill.max(1)
            };

            Err(Status::resource_exhausted(format!(
                "Rate limit exceeded. Retry after {} seconds",
                retry_after
            )))
        }
    }

    /// Check rate limit based on auth context
    pub async fn check_auth_rate_limit(&self, auth: &AuthContext) -> Result<(), Status> {
        // Get rate limits based on auth type and user role
        let (requests_per_minute, burst_limit, daily_limit) = match &auth.auth_type {
            crate::middleware::auth::AuthType::JWT(_) => {
                // JWT users get higher limits
                match auth.role {
                    crate::proto::fo3::wallet::v1::UserRole::UserRoleSuperAdmin => (1000.0, 100.0, 50000),
                    crate::proto::fo3::wallet::v1::UserRole::UserRoleAdmin => (500.0, 50.0, 25000),
                    crate::proto::fo3::wallet::v1::UserRole::UserRoleUser => (100.0, 20.0, 5000),
                    crate::proto::fo3::wallet::v1::UserRole::UserRoleViewer => (50.0, 10.0, 2500),
                    _ => (60.0, 10.0, 1000), // Default
                }
            }
            crate::middleware::auth::AuthType::ApiKey(_) => {
                // API keys get standard limits (could be customized per key)
                (100.0, 20.0, 5000)
            }
        };

        self.check_rate_limit(&auth.user_id, requests_per_minute, burst_limit, daily_limit)
            .await
    }

    /// Clean up old buckets (should be called periodically)
    pub async fn cleanup_old_buckets(&self) {
        let mut buckets = self.buckets.write().await;
        let now = Instant::now();
        
        buckets.retain(|_, bucket| {
            // Keep buckets that have been used recently or have tokens
            bucket.last_refill.elapsed() < Duration::from_secs(3600) || bucket.tokens > 0.0
        });
    }
}

/// Rate limiting interceptor
pub struct RateLimitInterceptor {
    rate_limiter: Arc<RateLimiter>,
}

impl RateLimitInterceptor {
    pub fn new(rate_limiter: Arc<RateLimiter>) -> Self {
        Self { rate_limiter }
    }

    pub async fn intercept<T>(&self, request: Request<T>) -> Result<Request<T>, Status> {
        // Get auth context from request extensions
        if let Some(auth) = request.extensions().get::<AuthContext>() {
            self.rate_limiter.check_auth_rate_limit(auth).await?;
        } else {
            // If no auth context, apply default rate limit based on IP
            let client_ip = request
                .metadata()
                .get("x-forwarded-for")
                .or_else(|| request.metadata().get("x-real-ip"))
                .and_then(|v| v.to_str().ok())
                .unwrap_or("unknown");

            self.rate_limiter
                .check_rate_limit(client_ip, 10.0, 5.0, 100) // Very restrictive for unauthenticated
                .await?;
        }

        Ok(request)
    }
}

/// Background task to clean up old rate limit buckets
pub async fn cleanup_rate_limit_buckets(rate_limiter: Arc<RateLimiter>) {
    let mut interval = tokio::time::interval(Duration::from_secs(3600)); // Clean up every hour
    
    loop {
        interval.tick().await;
        rate_limiter.cleanup_old_buckets().await;
        tracing::debug!("Cleaned up old rate limit buckets");
    }
}
