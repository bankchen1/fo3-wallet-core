//! Pricing service middleware for rate limiting and validation

use std::sync::Arc;
use std::collections::HashMap;
use tonic::{Request, Status};
use tokio::sync::RwLock;
use chrono::{DateTime, Utc, Duration};

use crate::middleware::auth::{AuthContext, AuthService};
use crate::state::AppState;

/// Rate limiting configuration for pricing service
#[derive(Debug, Clone)]
pub struct PricingRateLimit {
    /// Maximum requests per minute for regular users
    pub user_requests_per_minute: u32,
    /// Maximum requests per minute for admin users
    pub admin_requests_per_minute: u32,
    /// Maximum batch size for batch pricing requests
    pub max_batch_size: u32,
    /// Maximum symbols per request
    pub max_symbols_per_request: u32,
    /// Rate limit window in seconds
    pub window_seconds: u64,
}

impl Default for PricingRateLimit {
    fn default() -> Self {
        Self {
            user_requests_per_minute: 1000,
            admin_requests_per_minute: 5000,
            max_batch_size: 100,
            max_symbols_per_request: 50,
            window_seconds: 60,
        }
    }
}

/// Rate limit tracking entry
#[derive(Debug, Clone)]
struct RateLimitEntry {
    request_count: u32,
    window_start: DateTime<Utc>,
}

/// Pricing guard for rate limiting and validation
pub struct PricingGuard {
    state: Arc<AppState>,
    auth_service: Arc<AuthService>,
    rate_limits: RwLock<HashMap<String, RateLimitEntry>>,
    config: PricingRateLimit,
}

impl PricingGuard {
    pub fn new(
        state: Arc<AppState>,
        auth_service: Arc<AuthService>,
        config: Option<PricingRateLimit>,
    ) -> Self {
        Self {
            state,
            auth_service,
            rate_limits: RwLock::new(HashMap::new()),
            config: config.unwrap_or_default(),
        }
    }

    /// Check rate limits for a user
    pub async fn check_rate_limit(&self, auth_context: &AuthContext) -> Result<(), Status> {
        let user_id = &auth_context.user_id;
        let is_admin = auth_context.role == crate::middleware::auth::UserRole::UserRoleAdmin;
        
        let max_requests = if is_admin {
            self.config.admin_requests_per_minute
        } else {
            self.config.user_requests_per_minute
        };

        let mut rate_limits = self.rate_limits.write().await;
        let now = Utc::now();
        
        // Get or create rate limit entry
        let entry = rate_limits.entry(user_id.clone()).or_insert_with(|| RateLimitEntry {
            request_count: 0,
            window_start: now,
        });

        // Check if we need to reset the window
        if now.signed_duration_since(entry.window_start).num_seconds() >= self.config.window_seconds as i64 {
            entry.request_count = 0;
            entry.window_start = now;
        }

        // Check rate limit
        if entry.request_count >= max_requests {
            let reset_time = entry.window_start + Duration::seconds(self.config.window_seconds as i64);
            let seconds_until_reset = reset_time.signed_duration_since(now).num_seconds().max(0);
            
            return Err(Status::resource_exhausted(format!(
                "Rate limit exceeded. Try again in {} seconds",
                seconds_until_reset
            )));
        }

        // Increment counter
        entry.request_count += 1;
        Ok(())
    }

    /// Validate batch request size
    pub fn validate_batch_size(&self, symbols_count: usize) -> Result<(), Status> {
        if symbols_count == 0 {
            return Err(Status::invalid_argument("At least one symbol must be provided"));
        }

        if symbols_count > self.config.max_batch_size as usize {
            return Err(Status::invalid_argument(format!(
                "Batch size {} exceeds maximum allowed size of {}",
                symbols_count, self.config.max_batch_size
            )));
        }

        if symbols_count > self.config.max_symbols_per_request as usize {
            return Err(Status::invalid_argument(format!(
                "Number of symbols {} exceeds maximum allowed per request: {}",
                symbols_count, self.config.max_symbols_per_request
            )));
        }

        Ok(())
    }

    /// Validate symbol format
    pub fn validate_symbol(&self, symbol: &str) -> Result<(), Status> {
        if symbol.is_empty() {
            return Err(Status::invalid_argument("Symbol cannot be empty"));
        }

        if symbol.len() > 20 {
            return Err(Status::invalid_argument("Symbol too long (max 20 characters)"));
        }

        // Check for valid characters (alphanumeric and some special chars)
        if !symbol.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
            return Err(Status::invalid_argument("Symbol contains invalid characters"));
        }

        Ok(())
    }

    /// Validate quote currency
    pub fn validate_quote_currency(&self, currency: &str) -> Result<(), Status> {
        if currency.is_empty() {
            return Err(Status::invalid_argument("Quote currency cannot be empty"));
        }

        let valid_currencies = ["USD", "EUR", "GBP", "JPY", "CNY", "CAD", "AUD", "BTC", "ETH"];
        if !valid_currencies.contains(&currency.to_uppercase().as_str()) {
            return Err(Status::invalid_argument(format!(
                "Unsupported quote currency: {}. Supported currencies: {}",
                currency,
                valid_currencies.join(", ")
            )));
        }

        Ok(())
    }

    /// Validate time range for historical data
    pub fn validate_time_range(&self, start_time: i64, end_time: i64) -> Result<(), Status> {
        if start_time <= 0 || end_time <= 0 {
            return Err(Status::invalid_argument("Invalid timestamp"));
        }

        if start_time >= end_time {
            return Err(Status::invalid_argument("Start time must be before end time"));
        }

        let now = Utc::now().timestamp();
        if start_time > now || end_time > now {
            return Err(Status::invalid_argument("Time range cannot be in the future"));
        }

        // Limit historical data to 1 year
        let max_range_seconds = 365 * 24 * 60 * 60; // 1 year
        if end_time - start_time > max_range_seconds {
            return Err(Status::invalid_argument("Time range too large (max 1 year)"));
        }

        Ok(())
    }

    /// Validate pagination parameters
    pub fn validate_pagination(&self, page_size: i32, page_token: &str) -> Result<(), Status> {
        if page_size < 0 {
            return Err(Status::invalid_argument("Page size cannot be negative"));
        }

        if page_size > 1000 {
            return Err(Status::invalid_argument("Page size too large (max 1000)"));
        }

        // Basic page token validation (in production, this would be more sophisticated)
        if !page_token.is_empty() && page_token.len() > 1000 {
            return Err(Status::invalid_argument("Invalid page token"));
        }

        Ok(())
    }

    /// Clean up expired rate limit entries
    pub async fn cleanup_expired_entries(&self) {
        let mut rate_limits = self.rate_limits.write().await;
        let now = Utc::now();
        let window_duration = Duration::seconds(self.config.window_seconds as i64);

        rate_limits.retain(|_, entry| {
            now.signed_duration_since(entry.window_start) < window_duration * 2
        });
    }

    /// Get current rate limit status for a user
    pub async fn get_rate_limit_status(&self, user_id: &str) -> (u32, u32, DateTime<Utc>) {
        let rate_limits = self.rate_limits.read().await;
        
        if let Some(entry) = rate_limits.get(user_id) {
            let max_requests = self.config.user_requests_per_minute; // Default to user limit
            let reset_time = entry.window_start + Duration::seconds(self.config.window_seconds as i64);
            (entry.request_count, max_requests, reset_time)
        } else {
            (0, self.config.user_requests_per_minute, Utc::now())
        }
    }
}

/// Pricing interceptor for gRPC services
pub struct PricingInterceptor {
    pricing_guard: Arc<PricingGuard>,
    require_auth: bool,
}

impl PricingInterceptor {
    pub fn new(pricing_guard: Arc<PricingGuard>, require_auth: bool) -> Self {
        Self {
            pricing_guard,
            require_auth,
        }
    }

    pub async fn intercept<T>(&self, mut request: Request<T>) -> Result<Request<T>, Status> {
        if self.require_auth {
            let auth_context = request.extensions().get::<AuthContext>()
                .ok_or_else(|| Status::unauthenticated("Authentication required"))?;

            // Check rate limits
            self.pricing_guard.check_rate_limit(auth_context).await?;
        }

        Ok(request)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::middleware::auth::{UserRole, Permission, AuthType};

    fn create_test_auth_context() -> AuthContext {
        AuthContext {
            user_id: "test_user".to_string(),
            username: "test".to_string(),
            role: UserRole::UserRoleUser,
            permissions: vec![Permission::PermissionPricingRead],
            auth_type: AuthType::JWT("test_token".to_string()),
        }
    }

    #[tokio::test]
    async fn test_rate_limiting() {
        let state = Arc::new(crate::state::AppState::new());
        let auth_service = Arc::new(AuthService::new(state.clone()));
        
        let config = PricingRateLimit {
            user_requests_per_minute: 2,
            admin_requests_per_minute: 10,
            max_batch_size: 5,
            max_symbols_per_request: 3,
            window_seconds: 60,
        };
        
        let guard = PricingGuard::new(state, auth_service, Some(config));
        let auth_context = create_test_auth_context();

        // First two requests should succeed
        assert!(guard.check_rate_limit(&auth_context).await.is_ok());
        assert!(guard.check_rate_limit(&auth_context).await.is_ok());

        // Third request should fail
        assert!(guard.check_rate_limit(&auth_context).await.is_err());
    }

    #[test]
    fn test_symbol_validation() {
        let state = Arc::new(crate::state::AppState::new());
        let auth_service = Arc::new(AuthService::new(state.clone()));
        let guard = PricingGuard::new(state, auth_service, None);

        // Valid symbols
        assert!(guard.validate_symbol("BTC").is_ok());
        assert!(guard.validate_symbol("ETH-USD").is_ok());
        assert!(guard.validate_symbol("TOKEN_123").is_ok());

        // Invalid symbols
        assert!(guard.validate_symbol("").is_err());
        assert!(guard.validate_symbol("BTC@USD").is_err());
        assert!(guard.validate_symbol(&"A".repeat(25)).is_err());
    }

    #[test]
    fn test_batch_size_validation() {
        let state = Arc::new(crate::state::AppState::new());
        let auth_service = Arc::new(AuthService::new(state.clone()));
        let guard = PricingGuard::new(state, auth_service, None);

        // Valid batch sizes
        assert!(guard.validate_batch_size(1).is_ok());
        assert!(guard.validate_batch_size(50).is_ok());

        // Invalid batch sizes
        assert!(guard.validate_batch_size(0).is_err());
        assert!(guard.validate_batch_size(101).is_err());
    }
}
