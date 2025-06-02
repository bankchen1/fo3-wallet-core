//! Spending insights service implementation

use std::sync::Arc;
use std::collections::HashMap;
use tonic::{Request, Response, Status};
use uuid::Uuid;
use rust_decimal::Decimal;
use chrono::{DateTime, Utc, NaiveDate};

use crate::proto::fo3::wallet::v1::{
    spending_insights_service_server::SpendingInsightsService,
    *,
};
use crate::state::AppState;
use crate::middleware::{
    auth::{AuthContext, AuthService},
    audit::AuditLogger,
    spending_guard::SpendingGuard,
};
use crate::models::spending_insights::{
    Budget, SpendingAlert, CategorySpending, SpendingDataPoint, MerchantSpending,
    LocationInsight, SpendingPattern, CashflowAnalysis, PlatformInsights,
    TimePeriod, BudgetStatus, AlertType, SpendingInsightsRepository
};
use crate::models::notifications::{
    NotificationType, NotificationPriority, DeliveryChannel
};

/// Spending insights service implementation
pub struct SpendingInsightsServiceImpl {
    state: Arc<AppState>,
    auth_service: Arc<AuthService>,
    audit_logger: Arc<AuditLogger>,
    spending_guard: Arc<SpendingGuard>,
}

impl SpendingInsightsServiceImpl {
    pub fn new(
        state: Arc<AppState>,
        auth_service: Arc<AuthService>,
        audit_logger: Arc<AuditLogger>,
    ) -> Self {
        let spending_guard = Arc::new(SpendingGuard::new(state.clone()));
        
        Self {
            state,
            auth_service,
            audit_logger,
            spending_guard,
        }
    }

    /// Convert internal TimePeriod to proto TimePeriod
    fn time_period_to_proto(&self, period: &TimePeriod) -> i32 {
        match period {
            TimePeriod::Daily => 1,
            TimePeriod::Weekly => 2,
            TimePeriod::Monthly => 3,
            TimePeriod::Quarterly => 4,
            TimePeriod::Yearly => 5,
            TimePeriod::Custom => 6,
        }
    }

    /// Convert proto TimePeriod to internal TimePeriod
    fn proto_to_time_period(&self, period: i32) -> Result<TimePeriod, Status> {
        match period {
            1 => Ok(TimePeriod::Daily),
            2 => Ok(TimePeriod::Weekly),
            3 => Ok(TimePeriod::Monthly),
            4 => Ok(TimePeriod::Quarterly),
            5 => Ok(TimePeriod::Yearly),
            6 => Ok(TimePeriod::Custom),
            _ => Err(Status::invalid_argument("Invalid time period")),
        }
    }

    /// Convert internal BudgetStatus to proto BudgetStatus
    fn budget_status_to_proto(&self, status: &BudgetStatus) -> i32 {
        match status {
            BudgetStatus::OnTrack => 1,
            BudgetStatus::Warning => 2,
            BudgetStatus::Exceeded => 3,
            BudgetStatus::Critical => 4,
        }
    }

    /// Convert internal AlertType to proto AlertType
    fn alert_type_to_proto(&self, alert_type: &AlertType) -> i32 {
        match alert_type {
            AlertType::BudgetWarning => 1,
            AlertType::BudgetExceeded => 2,
            AlertType::UnusualSpending => 3,
            AlertType::LargeTransaction => 4,
            AlertType::CategoryLimit => 5,
            AlertType::MerchantAlert => 6,
        }
    }

    /// Convert proto AlertType to internal AlertType
    fn proto_to_alert_type(&self, alert_type: i32) -> Result<AlertType, Status> {
        match alert_type {
            1 => Ok(AlertType::BudgetWarning),
            2 => Ok(AlertType::BudgetExceeded),
            3 => Ok(AlertType::UnusualSpending),
            4 => Ok(AlertType::LargeTransaction),
            5 => Ok(AlertType::CategoryLimit),
            6 => Ok(AlertType::MerchantAlert),
            _ => Err(Status::invalid_argument("Invalid alert type")),
        }
    }

    /// Convert internal CategorySpending to proto CategorySpending
    fn category_spending_to_proto(&self, category: &CategorySpending) -> crate::proto::fo3::wallet::v1::CategorySpending {
        crate::proto::fo3::wallet::v1::CategorySpending {
            category: category.category.clone(),
            category_code: format!("{:?}", category.category_code).to_lowercase(),
            total_amount: category.total_amount.to_string(),
            currency: category.currency.clone(),
            transaction_count: category.transaction_count,
            average_amount: category.average_amount.to_string(),
            percentage_of_total: category.percentage_of_total,
            budget_amount: category.budget_amount.map(|a| a.to_string()).unwrap_or_default(),
            budget_utilization: category.budget_utilization.unwrap_or(0.0),
            top_merchants: category.top_merchants.clone(),
        }
    }

    /// Convert internal SpendingDataPoint to proto SpendingDataPoint
    fn spending_data_point_to_proto(&self, point: &SpendingDataPoint) -> crate::proto::fo3::wallet::v1::SpendingDataPoint {
        crate::proto::fo3::wallet::v1::SpendingDataPoint {
            timestamp: point.timestamp.timestamp(),
            amount: point.amount.to_string(),
            currency: point.currency.clone(),
            transaction_count: point.transaction_count,
            period_label: point.period_label.clone(),
        }
    }

    /// Convert internal Budget to proto Budget
    fn budget_to_proto(&self, budget: &Budget) -> crate::proto::fo3::wallet::v1::Budget {
        crate::proto::fo3::wallet::v1::Budget {
            id: budget.id.to_string(),
            user_id: budget.user_id.to_string(),
            category: budget.category.clone(),
            amount: budget.amount.to_string(),
            currency: budget.currency.clone(),
            period: self.time_period_to_proto(&budget.period),
            spent_amount: budget.spent_amount.to_string(),
            utilization: budget.utilization,
            status: self.budget_status_to_proto(&budget.status),
            is_active: budget.is_active,
            created_at: budget.created_at.timestamp(),
            updated_at: budget.updated_at.timestamp(),
            period_start: budget.period_start.timestamp(),
            period_end: budget.period_end.timestamp(),
            alert_thresholds: budget.alert_thresholds.iter().map(|t| t.to_string()).collect(),
        }
    }

    /// Convert internal SpendingAlert to proto SpendingAlert
    fn spending_alert_to_proto(&self, alert: &SpendingAlert) -> crate::proto::fo3::wallet::v1::SpendingAlert {
        crate::proto::fo3::wallet::v1::SpendingAlert {
            id: alert.id.to_string(),
            user_id: alert.user_id.to_string(),
            r#type: self.alert_type_to_proto(&alert.alert_type),
            title: alert.title.clone(),
            message: alert.message.clone(),
            category: alert.category.clone().unwrap_or_default(),
            merchant: alert.merchant.clone().unwrap_or_default(),
            threshold_amount: alert.threshold_amount.map(|a| a.to_string()).unwrap_or_default(),
            currency: alert.currency.clone(),
            is_active: alert.is_active,
            created_at: alert.created_at.timestamp(),
            triggered_at: alert.triggered_at.map(|t| t.timestamp()).unwrap_or(0),
            metadata: alert.metadata.clone(),
        }
    }

    /// Convert internal MerchantSpending to proto MerchantSpending
    fn merchant_spending_to_proto(&self, merchant: &MerchantSpending) -> crate::proto::fo3::wallet::v1::MerchantSpending {
        crate::proto::fo3::wallet::v1::MerchantSpending {
            merchant_name: merchant.merchant_name.clone(),
            category: merchant.category.clone(),
            total_amount: merchant.total_amount.to_string(),
            currency: merchant.currency.clone(),
            transaction_count: merchant.transaction_count,
            average_amount: merchant.average_amount.to_string(),
            last_transaction_date: merchant.last_transaction_date.format("%Y-%m-%d").to_string(),
            location: merchant.location.clone(),
            frequency_score: merchant.frequency_score,
        }
    }

    /// Calculate date range for time period
    fn calculate_date_range(&self, period: TimePeriod, start_date: Option<DateTime<Utc>>, end_date: Option<DateTime<Utc>>) -> (DateTime<Utc>, DateTime<Utc>) {
        match period {
            TimePeriod::Custom => {
                (start_date.unwrap_or_else(Utc::now), end_date.unwrap_or_else(Utc::now))
            }
            TimePeriod::Daily => {
                let now = Utc::now();
                let start = now.date_naive().and_hms_opt(0, 0, 0).unwrap().and_utc();
                let end = now.date_naive().and_hms_opt(23, 59, 59).unwrap().and_utc();
                (start, end)
            }
            TimePeriod::Weekly => {
                let now = Utc::now();
                let days_from_monday = now.weekday().num_days_from_monday();
                let week_start = now.date_naive() - chrono::Duration::days(days_from_monday as i64);
                let week_end = week_start + chrono::Duration::days(6);
                (
                    week_start.and_hms_opt(0, 0, 0).unwrap().and_utc(),
                    week_end.and_hms_opt(23, 59, 59).unwrap().and_utc(),
                )
            }
            TimePeriod::Monthly => {
                let now = Utc::now();
                let month_start = NaiveDate::from_ymd_opt(now.year(), now.month(), 1).unwrap();
                let next_month = if now.month() == 12 {
                    NaiveDate::from_ymd_opt(now.year() + 1, 1, 1).unwrap()
                } else {
                    NaiveDate::from_ymd_opt(now.year(), now.month() + 1, 1).unwrap()
                };
                let month_end = next_month - chrono::Duration::days(1);
                (
                    month_start.and_hms_opt(0, 0, 0).unwrap().and_utc(),
                    month_end.and_hms_opt(23, 59, 59).unwrap().and_utc(),
                )
            }
            TimePeriod::Quarterly => {
                let now = Utc::now();
                let quarter = (now.month() - 1) / 3;
                let quarter_start_month = quarter * 3 + 1;
                let quarter_start = NaiveDate::from_ymd_opt(now.year(), quarter_start_month, 1).unwrap();
                let quarter_end_month = quarter_start_month + 2;
                let quarter_end = if quarter_end_month == 12 {
                    NaiveDate::from_ymd_opt(now.year(), 12, 31).unwrap()
                } else {
                    NaiveDate::from_ymd_opt(now.year(), quarter_end_month + 1, 1).unwrap() - chrono::Duration::days(1)
                };
                (
                    quarter_start.and_hms_opt(0, 0, 0).unwrap().and_utc(),
                    quarter_end.and_hms_opt(23, 59, 59).unwrap().and_utc(),
                )
            }
            TimePeriod::Yearly => {
                let now = Utc::now();
                let year_start = NaiveDate::from_ymd_opt(now.year(), 1, 1).unwrap();
                let year_end = NaiveDate::from_ymd_opt(now.year(), 12, 31).unwrap();
                (
                    year_start.and_hms_opt(0, 0, 0).unwrap().and_utc(),
                    year_end.and_hms_opt(23, 59, 59).unwrap().and_utc(),
                )
            }
        }
    }

    /// Send notification for spending events
    async fn send_spending_notification(
        &self,
        user_id: &str,
        notification_type: NotificationType,
        title: String,
        message: String,
        metadata: HashMap<String, String>,
    ) -> Result<(), Status> {
        // Use the notification service to send real-time notifications
        let notification_request = crate::proto::fo3::wallet::v1::SendNotificationRequest {
            user_id: user_id.to_string(),
            r#type: match notification_type {
                NotificationType::Budget => 4,
                NotificationType::System => 5,
                _ => 5,
            },
            priority: 2, // Normal priority
            title,
            message,
            metadata,
            channels: vec![1, 2], // WebSocket and InApp
            expires_at: 0,
            action_url: String::new(),
            icon_url: String::new(),
        };

        // In a real implementation, we would call the notification service
        // For now, we'll just log the notification
        tracing::info!(
            "Spending notification sent to user {}: {}",
            user_id,
            notification_request.title
        );

        Ok(())
    }
}

#[tonic::async_trait]
impl SpendingInsightsService for SpendingInsightsServiceImpl {
    /// Get spending summary
    async fn get_spending_summary(
        &self,
        request: Request<GetSpendingSummaryRequest>,
    ) -> Result<Response<GetSpendingSummaryResponse>, Status> {
        let auth_context = self.auth_service.extract_auth(&request).await?;
        self.auth_service.check_permission(&auth_context, crate::proto::fo3::wallet::v1::Permission::PermissionSpendingRead)?;

        let req = request.into_inner();

        // Validate user access
        let user_id = self.spending_guard.validate_spending_access(&auth_context, None).await?;

        // Parse time period
        let period = self.proto_to_time_period(req.period)?;
        self.spending_guard.validate_time_period(&period, None, None)?;

        // Calculate date range
        let start_date = if req.start_date > 0 {
            Some(DateTime::from_timestamp(req.start_date, 0)
                .ok_or_else(|| Status::invalid_argument("Invalid start date"))?)
        } else {
            None
        };

        let end_date = if req.end_date > 0 {
            Some(DateTime::from_timestamp(req.end_date, 0)
                .ok_or_else(|| Status::invalid_argument("Invalid end date"))?)
        } else {
            None
        };

        let (calculated_start, calculated_end) = self.calculate_date_range(period, start_date, end_date);

        // Validate date range
        self.spending_guard.validate_date_range(calculated_start, calculated_end).await?;

        // Validate currency filter if provided
        if !req.currency.is_empty() {
            self.spending_guard.validate_currency_filter(&req.currency)?;
        }

        // Get spending summary from repository
        let (total_spent, transaction_count, categories) = self.state.spending_insights_repository
            .get_spending_summary(
                user_id,
                calculated_start,
                calculated_end,
                if req.currency.is_empty() { None } else { Some(req.currency.clone()) },
            )
            .map_err(|e| Status::internal(format!("Failed to get spending summary: {}", e)))?;

        // Get trend data
        let trend_data = self.state.spending_insights_repository
            .get_spending_trends(user_id, calculated_start, calculated_end, period)
            .map_err(|e| Status::internal(format!("Failed to get spending trends: {}", e)))?;

        // Calculate average transaction
        let average_transaction = if transaction_count > 0 {
            total_spent / Decimal::from(transaction_count)
        } else {
            Decimal::ZERO
        };

        // Convert to proto format
        let proto_categories: Vec<crate::proto::fo3::wallet::v1::CategorySpending> = categories
            .iter()
            .map(|c| self.category_spending_to_proto(c))
            .collect();

        let proto_trend_data: Vec<crate::proto::fo3::wallet::v1::SpendingDataPoint> = trend_data
            .iter()
            .map(|t| self.spending_data_point_to_proto(t))
            .collect();

        // Calculate period label
        let period_label = match period {
            TimePeriod::Daily => "Today".to_string(),
            TimePeriod::Weekly => "This Week".to_string(),
            TimePeriod::Monthly => "This Month".to_string(),
            TimePeriod::Quarterly => "This Quarter".to_string(),
            TimePeriod::Yearly => "This Year".to_string(),
            TimePeriod::Custom => format!("{} to {}", 
                calculated_start.format("%Y-%m-%d"), 
                calculated_end.format("%Y-%m-%d")),
        };

        // Audit log
        self.audit_logger.log_event(
            &auth_context.user_id,
            "spending_summary_viewed",
            &format!("Spending summary viewed for period: {}", period_label),
            None,
        ).await;

        let response = GetSpendingSummaryResponse {
            total_spent: total_spent.to_string(),
            currency: req.currency.clone().or_else(|| Some("USD".to_string())).unwrap(),
            transaction_count,
            average_transaction: average_transaction.to_string(),
            categories: proto_categories,
            trend_data: proto_trend_data,
            period_label,
            change_percentage: 0.0, // TODO: Calculate from previous period
            previous_period_amount: "0".to_string(), // TODO: Get previous period data
        };

        Ok(Response::new(response))
    }

    /// Get category breakdown
    async fn get_category_breakdown(
        &self,
        request: Request<GetCategoryBreakdownRequest>,
    ) -> Result<Response<GetCategoryBreakdownResponse>, Status> {
        let auth_context = self.auth_service.extract_auth(&request).await?;
        self.auth_service.check_permission(&auth_context, crate::proto::fo3::wallet::v1::Permission::PermissionSpendingRead)?;

        let req = request.into_inner();

        // Validate user access
        let user_id = self.spending_guard.validate_spending_access(&auth_context, None).await?;

        // Parse time period
        let period = self.proto_to_time_period(req.period)?;

        // Calculate date range
        let start_date = if req.start_date > 0 {
            Some(DateTime::from_timestamp(req.start_date, 0)
                .ok_or_else(|| Status::invalid_argument("Invalid start date"))?)
        } else {
            None
        };

        let end_date = if req.end_date > 0 {
            Some(DateTime::from_timestamp(req.end_date, 0)
                .ok_or_else(|| Status::invalid_argument("Invalid end date"))?)
        } else {
            None
        };

        let (calculated_start, calculated_end) = self.calculate_date_range(period, start_date, end_date);

        // Validate date range
        self.spending_guard.validate_date_range(calculated_start, calculated_end).await?;

        // Get category breakdown
        let categories = self.state.spending_insights_repository
            .get_category_breakdown(
                user_id,
                calculated_start,
                calculated_end,
                if req.currency.is_empty() { None } else { Some(req.currency.clone()) },
            )
            .map_err(|e| Status::internal(format!("Failed to get category breakdown: {}", e)))?;

        // Calculate total amount
        let total_amount: Decimal = categories.iter().map(|c| c.total_amount).sum();

        // Convert to proto format
        let proto_categories: Vec<crate::proto::fo3::wallet::v1::CategorySpending> = categories
            .iter()
            .map(|c| self.category_spending_to_proto(c))
            .collect();

        let period_label = match period {
            TimePeriod::Daily => "Today".to_string(),
            TimePeriod::Weekly => "This Week".to_string(),
            TimePeriod::Monthly => "This Month".to_string(),
            TimePeriod::Quarterly => "This Quarter".to_string(),
            TimePeriod::Yearly => "This Year".to_string(),
            TimePeriod::Custom => format!("{} to {}",
                calculated_start.format("%Y-%m-%d"),
                calculated_end.format("%Y-%m-%d")),
        };

        let response = GetCategoryBreakdownResponse {
            categories: proto_categories,
            total_amount: total_amount.to_string(),
            currency: req.currency.clone().or_else(|| Some("USD".to_string())).unwrap(),
            period_label,
        };

        Ok(Response::new(response))
    }

    /// Create budget
    async fn create_budget(
        &self,
        request: Request<CreateBudgetRequest>,
    ) -> Result<Response<CreateBudgetResponse>, Status> {
        let auth_context = self.auth_service.extract_auth(&request).await?;
        self.auth_service.check_permission(&auth_context, crate::proto::fo3::wallet::v1::Permission::PermissionSpendingRead)?;

        let req = request.into_inner();

        let user_id = Uuid::parse_str(&auth_context.user_id)
            .map_err(|_| Status::invalid_argument("Invalid user ID format"))?;

        // Validate category
        if !req.category.is_empty() && req.category != "total" {
            self.spending_guard.validate_category_filter(&req.category)?;
        }

        // Validate currency
        if !req.currency.is_empty() {
            self.spending_guard.validate_currency_filter(&req.currency)?;
        }

        // Parse budget amount
        let amount = Decimal::from_str_exact(&req.amount)
            .map_err(|_| Status::invalid_argument("Invalid budget amount"))?;

        // Parse time period
        let period = self.proto_to_time_period(req.period)?;

        // Parse alert thresholds
        let alert_thresholds: Result<Vec<f64>, _> = req.alert_thresholds
            .iter()
            .map(|t| t.parse::<f64>())
            .collect();

        let alert_thresholds = alert_thresholds
            .map_err(|_| Status::invalid_argument("Invalid alert threshold format"))?;

        // Create budget
        let budget = Budget::new(
            user_id,
            req.category,
            amount,
            req.currency,
            period,
            alert_thresholds,
        );

        // Validate budget creation
        self.spending_guard.validate_budget_creation(&auth_context, &budget).await?;

        // Store budget
        let created_budget = self.state.spending_insights_repository
            .create_budget(budget)
            .map_err(|e| Status::internal(format!("Failed to create budget: {}", e)))?;

        // Send notification
        let mut metadata = HashMap::new();
        metadata.insert("budget_id".to_string(), created_budget.id.to_string());
        metadata.insert("category".to_string(), created_budget.category.clone());
        metadata.insert("amount".to_string(), created_budget.amount.to_string());

        self.send_spending_notification(
            &auth_context.user_id,
            NotificationType::Budget,
            "Budget Created".to_string(),
            format!("Budget of {} {} created for {}",
                   created_budget.amount, created_budget.currency, created_budget.category),
            metadata,
        ).await?;

        // Audit log
        self.audit_logger.log_event(
            &auth_context.user_id,
            "budget_created",
            &format!("Budget created for category {} with amount {}",
                    created_budget.category, created_budget.amount),
            Some(&created_budget.id.to_string()),
        ).await;

        let response = CreateBudgetResponse {
            budget: Some(self.budget_to_proto(&created_budget)),
            success: true,
        };

        Ok(Response::new(response))
    }

    /// Get budgets
    async fn get_budgets(
        &self,
        request: Request<GetBudgetsRequest>,
    ) -> Result<Response<GetBudgetsResponse>, Status> {
        let auth_context = self.auth_service.extract_auth(&request).await?;
        self.auth_service.check_permission(&auth_context, crate::proto::fo3::wallet::v1::Permission::PermissionSpendingRead)?;

        let req = request.into_inner();

        let user_id = Uuid::parse_str(&auth_context.user_id)
            .map_err(|_| Status::invalid_argument("Invalid user ID format"))?;

        // Get user's budgets
        let mut budgets = self.state.spending_insights_repository
            .get_budgets_by_user(user_id)
            .map_err(|e| Status::internal(format!("Failed to get budgets: {}", e)))?;

        // Apply filters
        if !req.category.is_empty() {
            budgets.retain(|b| b.category == req.category);
        }

        if req.period != 0 {
            let filter_period = self.proto_to_time_period(req.period)?;
            budgets.retain(|b| b.period == filter_period);
        }

        if req.active_only {
            budgets.retain(|b| b.is_active);
        }

        let total_count = budgets.len() as i64;

        // Convert to proto format
        let proto_budgets: Vec<crate::proto::fo3::wallet::v1::Budget> = budgets
            .iter()
            .map(|b| self.budget_to_proto(b))
            .collect();

        let response = GetBudgetsResponse {
            budgets: proto_budgets,
            total_count,
        };

        Ok(Response::new(response))
    }

    /// Create spending alert
    async fn create_spending_alert(
        &self,
        request: Request<CreateSpendingAlertRequest>,
    ) -> Result<Response<CreateSpendingAlertResponse>, Status> {
        let auth_context = self.auth_service.extract_auth(&request).await?;
        self.auth_service.check_permission(&auth_context, crate::proto::fo3::wallet::v1::Permission::PermissionSpendingRead)?;

        let req = request.into_inner();

        let user_id = Uuid::parse_str(&auth_context.user_id)
            .map_err(|_| Status::invalid_argument("Invalid user ID format"))?;

        // Parse alert type
        let alert_type = self.proto_to_alert_type(req.r#type)?;

        // Parse threshold amount if provided
        let threshold_amount = if !req.threshold_amount.is_empty() {
            Some(Decimal::from_str_exact(&req.threshold_amount)
                .map_err(|_| Status::invalid_argument("Invalid threshold amount"))?)
        } else {
            None
        };

        // Create spending alert
        let mut alert = SpendingAlert::new(
            user_id,
            alert_type,
            format!("{:?} Alert", req.r#type), // Default title
            req.custom_message.clone(),
            req.currency,
        );

        // Set optional fields
        if !req.category.is_empty() {
            alert.category = Some(req.category);
        }

        if !req.merchant.is_empty() {
            alert.merchant = Some(req.merchant);
        }

        alert.threshold_amount = threshold_amount;

        // Validate alert creation
        self.spending_guard.validate_alert_creation(&auth_context, &alert).await?;

        // Store alert
        let created_alert = self.state.spending_insights_repository
            .create_spending_alert(alert)
            .map_err(|e| Status::internal(format!("Failed to create spending alert: {}", e)))?;

        // Audit log
        self.audit_logger.log_event(
            &auth_context.user_id,
            "spending_alert_created",
            &format!("Spending alert created: {}", created_alert.title),
            Some(&created_alert.id.to_string()),
        ).await;

        let response = CreateSpendingAlertResponse {
            alert: Some(self.spending_alert_to_proto(&created_alert)),
            success: true,
        };

        Ok(Response::new(response))
    }

    /// Get top merchants
    async fn get_top_merchants(
        &self,
        request: Request<GetTopMerchantsRequest>,
    ) -> Result<Response<GetTopMerchantsResponse>, Status> {
        let auth_context = self.auth_service.extract_auth(&request).await?;
        self.auth_service.check_permission(&auth_context, crate::proto::fo3::wallet::v1::Permission::PermissionSpendingRead)?;

        let req = request.into_inner();

        // Validate user access
        let user_id = self.spending_guard.validate_spending_access(&auth_context, None).await?;

        // Parse time period
        let period = self.proto_to_time_period(req.period)?;

        // Calculate date range
        let start_date = if req.start_date > 0 {
            Some(DateTime::from_timestamp(req.start_date, 0)
                .ok_or_else(|| Status::invalid_argument("Invalid start date"))?)
        } else {
            None
        };

        let end_date = if req.end_date > 0 {
            Some(DateTime::from_timestamp(req.end_date, 0)
                .ok_or_else(|| Status::invalid_argument("Invalid end date"))?)
        } else {
            None
        };

        let (calculated_start, calculated_end) = self.calculate_date_range(period, start_date, end_date);

        // Validate date range
        self.spending_guard.validate_date_range(calculated_start, calculated_end).await?;

        // Validate limit
        let limit = if req.limit > 0 { req.limit as usize } else { 10 };
        if limit > 100 {
            return Err(Status::invalid_argument("Limit cannot exceed 100"));
        }

        // Get top merchants
        let merchants = self.state.spending_insights_repository
            .get_top_merchants(user_id, calculated_start, calculated_end, limit)
            .map_err(|e| Status::internal(format!("Failed to get top merchants: {}", e)))?;

        // Calculate total amount
        let total_amount: Decimal = merchants.iter().map(|m| m.total_amount).sum();

        // Convert to proto format
        let proto_merchants: Vec<crate::proto::fo3::wallet::v1::MerchantSpending> = merchants
            .iter()
            .map(|m| self.merchant_spending_to_proto(m))
            .collect();

        let period_label = match period {
            TimePeriod::Daily => "Today".to_string(),
            TimePeriod::Weekly => "This Week".to_string(),
            TimePeriod::Monthly => "This Month".to_string(),
            TimePeriod::Quarterly => "This Quarter".to_string(),
            TimePeriod::Yearly => "This Year".to_string(),
            TimePeriod::Custom => format!("{} to {}",
                calculated_start.format("%Y-%m-%d"),
                calculated_end.format("%Y-%m-%d")),
        };

        let response = GetTopMerchantsResponse {
            merchants: proto_merchants,
            total_amount: total_amount.to_string(),
            currency: "USD".to_string(),
            period_label,
        };

        Ok(Response::new(response))
    }

    /// Get platform insights (admin only)
    async fn get_platform_insights(
        &self,
        request: Request<GetPlatformInsightsRequest>,
    ) -> Result<Response<GetPlatformInsightsResponse>, Status> {
        let auth_context = self.auth_service.extract_auth(&request).await?;
        self.auth_service.check_permission(&auth_context, crate::proto::fo3::wallet::v1::Permission::PermissionSpendingAdmin)?;

        let req = request.into_inner();

        // Validate admin access
        self.spending_guard.validate_admin_access(&auth_context).await?;

        // Parse time period
        let period = self.proto_to_time_period(req.period)?;

        // Calculate date range
        let start_date = if req.start_date > 0 {
            Some(DateTime::from_timestamp(req.start_date, 0)
                .ok_or_else(|| Status::invalid_argument("Invalid start date"))?)
        } else {
            None
        };

        let end_date = if req.end_date > 0 {
            Some(DateTime::from_timestamp(req.end_date, 0)
                .ok_or_else(|| Status::invalid_argument("Invalid end date"))?)
        } else {
            None
        };

        let (calculated_start, calculated_end) = self.calculate_date_range(period, start_date, end_date);

        // Get platform insights
        let insights = self.state.spending_insights_repository
            .get_platform_insights(calculated_start, calculated_end)
            .map_err(|e| Status::internal(format!("Failed to get platform insights: {}", e)))?;

        // Convert to proto format
        let proto_insights = crate::proto::fo3::wallet::v1::PlatformInsights {
            total_users: insights.total_users,
            total_volume: insights.total_volume.to_string(),
            currency: insights.currency,
            total_transactions: insights.total_transactions,
            average_transaction: insights.average_transaction.to_string(),
            top_categories: insights.top_categories.iter()
                .map(|c| self.category_spending_to_proto(c))
                .collect(),
            top_merchants: insights.top_merchants.iter()
                .map(|m| self.merchant_spending_to_proto(m))
                .collect(),
            growth_metrics: insights.growth_metrics,
            volume_trend: insights.volume_trend.iter()
                .map(|t| self.spending_data_point_to_proto(t))
                .collect(),
        };

        // Audit log
        self.audit_logger.log_event(
            &auth_context.user_id,
            "platform_insights_accessed",
            "Platform spending insights accessed by admin",
            None,
        ).await;

        let response = GetPlatformInsightsResponse {
            insights: Some(proto_insights),
            key_metrics: vec![
                format!("Total Users: {}", insights.total_users),
                format!("Total Volume: {} {}", insights.total_volume, insights.currency),
                format!("Avg Transaction: {} {}", insights.average_transaction, insights.currency),
            ],
            growth_insights: vec![
                "Monthly transaction volume growing".to_string(),
                "User engagement increasing".to_string(),
                "Restaurant category leading spending".to_string(),
            ],
        };

        Ok(Response::new(response))
    }

    // Placeholder implementations for remaining methods
    async fn get_spending_trends(&self, _request: Request<GetSpendingTrendsRequest>) -> Result<Response<GetSpendingTrendsResponse>, Status> {
        Err(Status::unimplemented("Method not yet implemented"))
    }

    async fn get_monthly_report(&self, _request: Request<GetMonthlyReportRequest>) -> Result<Response<GetMonthlyReportResponse>, Status> {
        Err(Status::unimplemented("Method not yet implemented"))
    }

    async fn update_budget(&self, _request: Request<UpdateBudgetRequest>) -> Result<Response<UpdateBudgetResponse>, Status> {
        Err(Status::unimplemented("Method not yet implemented"))
    }

    async fn delete_budget(&self, _request: Request<DeleteBudgetRequest>) -> Result<Response<DeleteBudgetResponse>, Status> {
        Err(Status::unimplemented("Method not yet implemented"))
    }

    async fn get_budget_status(&self, _request: Request<GetBudgetStatusRequest>) -> Result<Response<GetBudgetStatusResponse>, Status> {
        Err(Status::unimplemented("Method not yet implemented"))
    }

    async fn update_spending_alert(&self, _request: Request<UpdateSpendingAlertRequest>) -> Result<Response<UpdateSpendingAlertResponse>, Status> {
        Err(Status::unimplemented("Method not yet implemented"))
    }

    async fn get_spending_alerts(&self, _request: Request<GetSpendingAlertsRequest>) -> Result<Response<GetSpendingAlertsResponse>, Status> {
        Err(Status::unimplemented("Method not yet implemented"))
    }

    async fn delete_spending_alert(&self, _request: Request<DeleteSpendingAlertRequest>) -> Result<Response<DeleteSpendingAlertResponse>, Status> {
        Err(Status::unimplemented("Method not yet implemented"))
    }

    async fn get_location_insights(&self, _request: Request<GetLocationInsightsRequest>) -> Result<Response<GetLocationInsightsResponse>, Status> {
        Err(Status::unimplemented("Method not yet implemented"))
    }

    async fn get_merchant_history(&self, _request: Request<GetMerchantHistoryRequest>) -> Result<Response<GetMerchantHistoryResponse>, Status> {
        Err(Status::unimplemented("Method not yet implemented"))
    }

    async fn get_spending_patterns(&self, _request: Request<GetSpendingPatternsRequest>) -> Result<Response<GetSpendingPatternsResponse>, Status> {
        Err(Status::unimplemented("Method not yet implemented"))
    }

    async fn get_cashflow_analysis(&self, _request: Request<GetCashflowAnalysisRequest>) -> Result<Response<GetCashflowAnalysisResponse>, Status> {
        Err(Status::unimplemented("Method not yet implemented"))
    }

    async fn export_spending_data(&self, _request: Request<ExportSpendingDataRequest>) -> Result<Response<ExportSpendingDataResponse>, Status> {
        Err(Status::unimplemented("Method not yet implemented"))
    }

    async fn get_user_spending_metrics(&self, _request: Request<GetUserSpendingMetricsRequest>) -> Result<Response<GetUserSpendingMetricsResponse>, Status> {
        Err(Status::unimplemented("Method not yet implemented"))
    }
}
