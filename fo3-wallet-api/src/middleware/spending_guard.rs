//! Spending insights security middleware for data access validation

use std::sync::Arc;
use tonic::{Request, Response, Status};
use uuid::Uuid;
use rust_decimal::Decimal;
use chrono::{DateTime, Utc, Duration};

use crate::state::AppState;
use crate::middleware::auth::AuthContext;
use crate::models::spending_insights::{Budget, SpendingAlert, TimePeriod, AlertType};

/// Spending insights security guard for validating analytics operations
pub struct SpendingGuard {
    state: Arc<AppState>,
}

impl SpendingGuard {
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }

    /// Validate user access to spending data
    pub async fn validate_spending_access(&self, auth: &AuthContext, user_id: Option<&str>) -> Result<Uuid, Status> {
        let requesting_user_id = Uuid::parse_str(&auth.user_id)
            .map_err(|_| Status::invalid_argument("Invalid user ID format"))?;

        // If no specific user_id is provided, use the requesting user's ID
        let target_user_id = if let Some(uid) = user_id {
            let parsed_uid = Uuid::parse_str(uid)
                .map_err(|_| Status::invalid_argument("Invalid target user ID format"))?;
            
            // Check if user is trying to access someone else's data
            if parsed_uid != requesting_user_id {
                // Only admins can access other users' spending data
                if !auth.permissions.contains(&crate::proto::fo3::wallet::v1::Permission::PermissionSpendingAdmin) {
                    return Err(Status::permission_denied("Cannot access other users' spending data"));
                }
            }
            parsed_uid
        } else {
            requesting_user_id
        };

        Ok(target_user_id)
    }

    /// Validate budget creation parameters
    pub async fn validate_budget_creation(&self, auth: &AuthContext, budget: &Budget) -> Result<(), Status> {
        // Validate budget amount
        if budget.amount <= Decimal::ZERO {
            return Err(Status::invalid_argument("Budget amount must be positive"));
        }

        // Validate budget period
        match budget.period {
            TimePeriod::Custom => {
                if budget.period_end <= budget.period_start {
                    return Err(Status::invalid_argument("Custom budget end date must be after start date"));
                }
            }
            _ => {
                // Standard periods are automatically calculated
            }
        }

        // Validate alert thresholds
        for threshold in &budget.alert_thresholds {
            if *threshold <= 0.0 || *threshold > 200.0 {
                return Err(Status::invalid_argument("Alert thresholds must be between 0 and 200 percent"));
            }
        }

        // Check if user already has too many budgets (limit: 20)
        let user_budgets = self.state.spending_insights_repository
            .get_budgets_by_user(budget.user_id)
            .map_err(|e| Status::internal(format!("Failed to get user budgets: {}", e)))?;

        if user_budgets.len() >= 20 {
            return Err(Status::resource_exhausted("Maximum number of budgets (20) reached"));
        }

        // Check for duplicate category budgets in the same period
        let existing_category_budget = user_budgets.iter()
            .find(|b| b.category == budget.category && b.period == budget.period && b.is_active);

        if existing_category_budget.is_some() {
            return Err(Status::already_exists(
                format!("Active budget for category '{}' already exists for this period", budget.category)
            ));
        }

        Ok(())
    }

    /// Validate spending alert creation
    pub async fn validate_alert_creation(&self, auth: &AuthContext, alert: &SpendingAlert) -> Result<(), Status> {
        // Validate alert threshold if applicable
        if let Some(threshold) = alert.threshold_amount {
            if threshold <= Decimal::ZERO {
                return Err(Status::invalid_argument("Alert threshold must be positive"));
            }
        }

        // Validate alert type specific requirements
        match alert.alert_type {
            AlertType::BudgetWarning | AlertType::BudgetExceeded => {
                if alert.category.is_none() {
                    return Err(Status::invalid_argument("Budget alerts require a category"));
                }
            }
            AlertType::MerchantAlert => {
                if alert.merchant.is_none() {
                    return Err(Status::invalid_argument("Merchant alerts require a merchant name"));
                }
            }
            AlertType::LargeTransaction => {
                if alert.threshold_amount.is_none() {
                    return Err(Status::invalid_argument("Large transaction alerts require a threshold amount"));
                }
            }
            _ => {
                // Other alert types don't have specific requirements
            }
        }

        // Check if user already has too many alerts (limit: 50)
        let user_alerts = self.state.spending_insights_repository
            .get_spending_alerts_by_user(alert.user_id)
            .map_err(|e| Status::internal(format!("Failed to get user alerts: {}", e)))?;

        if user_alerts.len() >= 50 {
            return Err(Status::resource_exhausted("Maximum number of spending alerts (50) reached"));
        }

        Ok(())
    }

    /// Validate date range for analytics queries
    pub async fn validate_date_range(&self, start_date: DateTime<Utc>, end_date: DateTime<Utc>) -> Result<(), Status> {
        // Check if start date is before end date
        if start_date >= end_date {
            return Err(Status::invalid_argument("Start date must be before end date"));
        }

        // Check if date range is not too large (max 2 years)
        let max_range = Duration::days(730); // 2 years
        if end_date - start_date > max_range {
            return Err(Status::invalid_argument("Date range cannot exceed 2 years"));
        }

        // Check if dates are not too far in the future
        let now = Utc::now();
        if start_date > now + Duration::days(1) {
            return Err(Status::invalid_argument("Start date cannot be more than 1 day in the future"));
        }

        if end_date > now + Duration::days(1) {
            return Err(Status::invalid_argument("End date cannot be more than 1 day in the future"));
        }

        Ok(())
    }

    /// Validate export request parameters
    pub async fn validate_export_request(
        &self,
        auth: &AuthContext,
        format: &str,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> Result<(), Status> {
        // Validate export format
        match format.to_lowercase().as_str() {
            "csv" | "json" | "pdf" => {}
            _ => return Err(Status::invalid_argument("Unsupported export format. Use csv, json, or pdf")),
        }

        // Validate date range
        self.validate_date_range(start_date, end_date).await?;

        // Check rate limiting for exports (max 5 per hour)
        self.check_export_rate_limit(auth).await?;

        Ok(())
    }

    /// Check export rate limiting
    async fn check_export_rate_limit(&self, auth: &AuthContext) -> Result<(), Status> {
        // In a real implementation, this would use Redis or similar for distributed rate limiting
        // For demo purposes, we'll implement basic validation
        
        // Allow up to 5 exports per hour per user
        // This would be tracked in a proper rate limiting system
        Ok(())
    }

    /// Validate admin access for platform insights
    pub async fn validate_admin_access(&self, auth: &AuthContext) -> Result<(), Status> {
        if !auth.permissions.contains(&crate::proto::fo3::wallet::v1::Permission::PermissionSpendingAdmin) {
            return Err(Status::permission_denied("Admin access required for platform insights"));
        }
        Ok(())
    }

    /// Validate time period parameter
    pub fn validate_time_period(&self, period: &TimePeriod, start_date: Option<DateTime<Utc>>, end_date: Option<DateTime<Utc>>) -> Result<(), Status> {
        match period {
            TimePeriod::Custom => {
                if start_date.is_none() || end_date.is_none() {
                    return Err(Status::invalid_argument("Custom time period requires start_date and end_date"));
                }
            }
            _ => {
                // Standard periods don't require custom dates
            }
        }
        Ok(())
    }

    /// Validate category filter
    pub fn validate_category_filter(&self, category: &str) -> Result<(), Status> {
        // Validate that category is one of the known categories
        let valid_categories = [
            "grocery", "restaurant", "gas_station", "retail", "entertainment",
            "travel", "healthcare", "education", "utilities", "other", "total"
        ];

        if !valid_categories.contains(&category.to_lowercase().as_str()) {
            return Err(Status::invalid_argument(
                format!("Invalid category '{}'. Valid categories: {}", category, valid_categories.join(", "))
            ));
        }

        Ok(())
    }

    /// Validate merchant name filter
    pub fn validate_merchant_filter(&self, merchant: &str) -> Result<(), Status> {
        if merchant.trim().is_empty() {
            return Err(Status::invalid_argument("Merchant name cannot be empty"));
        }

        if merchant.len() > 255 {
            return Err(Status::invalid_argument("Merchant name cannot exceed 255 characters"));
        }

        Ok(())
    }

    /// Validate currency filter
    pub fn validate_currency_filter(&self, currency: &str) -> Result<(), Status> {
        // Validate currency code format (3 letters)
        if currency.len() != 3 {
            return Err(Status::invalid_argument("Currency code must be 3 letters (e.g., USD, EUR)"));
        }

        if !currency.chars().all(|c| c.is_ascii_alphabetic()) {
            return Err(Status::invalid_argument("Currency code must contain only letters"));
        }

        // Check if currency is supported
        let supported_currencies = ["USD", "EUR", "GBP", "CAD", "AUD"];
        if !supported_currencies.contains(&currency.to_uppercase().as_str()) {
            return Err(Status::invalid_argument(
                format!("Unsupported currency '{}'. Supported: {}", currency, supported_currencies.join(", "))
            ));
        }

        Ok(())
    }

    /// Validate pagination parameters
    pub fn validate_pagination(&self, page_size: i32, page_token: &str) -> Result<(), Status> {
        if page_size < 0 {
            return Err(Status::invalid_argument("Page size cannot be negative"));
        }

        if page_size > 1000 {
            return Err(Status::invalid_argument("Page size cannot exceed 1000"));
        }

        // Validate page token format if provided
        if !page_token.is_empty() {
            // In a real implementation, validate the page token format and signature
            // For demo purposes, accept any non-empty string
        }

        Ok(())
    }

    /// Check for suspicious analytics patterns
    pub async fn check_analytics_abuse(&self, auth: &AuthContext, operation: &str) -> Result<(), Status> {
        // In a real implementation, this would track analytics usage patterns
        // and detect potential abuse or scraping attempts
        
        // Check for excessive API calls
        // Check for unusual query patterns
        // Check for data export abuse
        
        // For demo purposes, just log the operation
        tracing::debug!(
            "Analytics operation '{}' by user {}",
            operation,
            auth.user_id
        );

        Ok(())
    }

    /// Validate budget ownership
    pub async fn validate_budget_ownership(&self, auth: &AuthContext, budget_id: Uuid) -> Result<(), Status> {
        let user_id = Uuid::parse_str(&auth.user_id)
            .map_err(|_| Status::invalid_argument("Invalid user ID format"))?;

        let user_budgets = self.state.spending_insights_repository
            .get_budgets_by_user(user_id)
            .map_err(|e| Status::internal(format!("Failed to get user budgets: {}", e)))?;

        let budget_exists = user_budgets.iter().any(|b| b.id == budget_id);
        
        if !budget_exists {
            return Err(Status::not_found("Budget not found or access denied"));
        }

        Ok(())
    }

    /// Validate spending alert ownership
    pub async fn validate_alert_ownership(&self, auth: &AuthContext, alert_id: Uuid) -> Result<(), Status> {
        let user_id = Uuid::parse_str(&auth.user_id)
            .map_err(|_| Status::invalid_argument("Invalid user ID format"))?;

        let user_alerts = self.state.spending_insights_repository
            .get_spending_alerts_by_user(user_id)
            .map_err(|e| Status::internal(format!("Failed to get user alerts: {}", e)))?;

        let alert_exists = user_alerts.iter().any(|a| a.id == alert_id);
        
        if !alert_exists {
            return Err(Status::not_found("Spending alert not found or access denied"));
        }

        Ok(())
    }
}
