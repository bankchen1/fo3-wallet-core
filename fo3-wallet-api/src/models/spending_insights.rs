//! Spending insights models and repository for financial analytics

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc, NaiveDate};
use uuid::Uuid;
use rust_decimal::Decimal;

use crate::models::cards::{CardTransaction, CardTransactionStatus, MerchantCategory};

/// Time period enumeration for analytics
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimePeriod {
    Daily,
    Weekly,
    Monthly,
    Quarterly,
    Yearly,
    Custom,
}

impl Default for TimePeriod {
    fn default() -> Self {
        TimePeriod::Monthly
    }
}

/// Budget status enumeration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BudgetStatus {
    OnTrack,    // Under 80% of budget
    Warning,    // 80-95% of budget
    Exceeded,   // Over 100% of budget
    Critical,   // Over 120% of budget
}

impl Default for BudgetStatus {
    fn default() -> Self {
        BudgetStatus::OnTrack
    }
}

/// Alert type enumeration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertType {
    BudgetWarning,
    BudgetExceeded,
    UnusualSpending,
    LargeTransaction,
    CategoryLimit,
    MerchantAlert,
}

/// Spending category breakdown
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategorySpending {
    pub category: String,
    pub category_code: MerchantCategory,
    pub total_amount: Decimal,
    pub currency: String,
    pub transaction_count: i64,
    pub average_amount: Decimal,
    pub percentage_of_total: f64,
    pub budget_amount: Option<Decimal>,
    pub budget_utilization: Option<f64>,
    pub top_merchants: Vec<String>,
}

/// Time-based spending data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpendingDataPoint {
    pub timestamp: DateTime<Utc>,
    pub amount: Decimal,
    pub currency: String,
    pub transaction_count: i64,
    pub period_label: String,
}

/// Budget entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Budget {
    pub id: Uuid,
    pub user_id: Uuid,
    pub category: String, // Category or "total" for overall budget
    pub amount: Decimal,
    pub currency: String,
    pub period: TimePeriod,
    pub spent_amount: Decimal,
    pub utilization: f64,
    pub status: BudgetStatus,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub alert_thresholds: Vec<f64>, // Alert thresholds (e.g., 80.0, 100.0)
}

impl Budget {
    /// Create a new budget
    pub fn new(
        user_id: Uuid,
        category: String,
        amount: Decimal,
        currency: String,
        period: TimePeriod,
        alert_thresholds: Vec<f64>,
    ) -> Self {
        let now = Utc::now();
        let (period_start, period_end) = Self::calculate_period_bounds(&period, now);

        Self {
            id: Uuid::new_v4(),
            user_id,
            category,
            amount,
            currency,
            period,
            spent_amount: Decimal::ZERO,
            utilization: 0.0,
            status: BudgetStatus::OnTrack,
            is_active: true,
            created_at: now,
            updated_at: now,
            period_start,
            period_end,
            alert_thresholds,
        }
    }

    /// Calculate period bounds for budget
    fn calculate_period_bounds(period: &TimePeriod, reference_date: DateTime<Utc>) -> (DateTime<Utc>, DateTime<Utc>) {
        let date = reference_date.date_naive();
        
        match period {
            TimePeriod::Daily => {
                let start = date.and_hms_opt(0, 0, 0).unwrap().and_utc();
                let end = date.and_hms_opt(23, 59, 59).unwrap().and_utc();
                (start, end)
            }
            TimePeriod::Weekly => {
                let days_from_monday = date.weekday().num_days_from_monday();
                let week_start = date - chrono::Duration::days(days_from_monday as i64);
                let week_end = week_start + chrono::Duration::days(6);
                (
                    week_start.and_hms_opt(0, 0, 0).unwrap().and_utc(),
                    week_end.and_hms_opt(23, 59, 59).unwrap().and_utc(),
                )
            }
            TimePeriod::Monthly => {
                let month_start = NaiveDate::from_ymd_opt(date.year(), date.month(), 1).unwrap();
                let next_month = if date.month() == 12 {
                    NaiveDate::from_ymd_opt(date.year() + 1, 1, 1).unwrap()
                } else {
                    NaiveDate::from_ymd_opt(date.year(), date.month() + 1, 1).unwrap()
                };
                let month_end = next_month - chrono::Duration::days(1);
                (
                    month_start.and_hms_opt(0, 0, 0).unwrap().and_utc(),
                    month_end.and_hms_opt(23, 59, 59).unwrap().and_utc(),
                )
            }
            TimePeriod::Quarterly => {
                let quarter = (date.month() - 1) / 3;
                let quarter_start_month = quarter * 3 + 1;
                let quarter_start = NaiveDate::from_ymd_opt(date.year(), quarter_start_month, 1).unwrap();
                let quarter_end_month = quarter_start_month + 2;
                let quarter_end = if quarter_end_month == 12 {
                    NaiveDate::from_ymd_opt(date.year(), 12, 31).unwrap()
                } else {
                    NaiveDate::from_ymd_opt(date.year(), quarter_end_month + 1, 1).unwrap() - chrono::Duration::days(1)
                };
                (
                    quarter_start.and_hms_opt(0, 0, 0).unwrap().and_utc(),
                    quarter_end.and_hms_opt(23, 59, 59).unwrap().and_utc(),
                )
            }
            TimePeriod::Yearly => {
                let year_start = NaiveDate::from_ymd_opt(date.year(), 1, 1).unwrap();
                let year_end = NaiveDate::from_ymd_opt(date.year(), 12, 31).unwrap();
                (
                    year_start.and_hms_opt(0, 0, 0).unwrap().and_utc(),
                    year_end.and_hms_opt(23, 59, 59).unwrap().and_utc(),
                )
            }
            TimePeriod::Custom => {
                // For custom periods, use the reference date as both start and end
                // This should be overridden by the caller
                (reference_date, reference_date)
            }
        }
    }

    /// Update budget with new spending amount
    pub fn update_spending(&mut self, spent_amount: Decimal) {
        self.spent_amount = spent_amount;
        self.utilization = if self.amount > Decimal::ZERO {
            (spent_amount / self.amount * Decimal::from(100)).to_f64().unwrap_or(0.0)
        } else {
            0.0
        };

        // Update status based on utilization
        self.status = if self.utilization < 80.0 {
            BudgetStatus::OnTrack
        } else if self.utilization < 100.0 {
            BudgetStatus::Warning
        } else if self.utilization < 120.0 {
            BudgetStatus::Exceeded
        } else {
            BudgetStatus::Critical
        };

        self.updated_at = Utc::now();
    }

    /// Check if budget should trigger an alert
    pub fn should_trigger_alert(&self, threshold: f64) -> bool {
        self.is_active && self.utilization >= threshold
    }
}

/// Spending alert entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpendingAlert {
    pub id: Uuid,
    pub user_id: Uuid,
    pub alert_type: AlertType,
    pub title: String,
    pub message: String,
    pub category: Option<String>,
    pub merchant: Option<String>,
    pub threshold_amount: Option<Decimal>,
    pub currency: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub triggered_at: Option<DateTime<Utc>>,
    pub metadata: HashMap<String, String>,
}

impl SpendingAlert {
    /// Create a new spending alert
    pub fn new(
        user_id: Uuid,
        alert_type: AlertType,
        title: String,
        message: String,
        currency: String,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            user_id,
            alert_type,
            title,
            message,
            category: None,
            merchant: None,
            threshold_amount: None,
            currency,
            is_active: true,
            created_at: Utc::now(),
            triggered_at: None,
            metadata: HashMap::new(),
        }
    }

    /// Trigger the alert
    pub fn trigger(&mut self) {
        self.triggered_at = Some(Utc::now());
    }
}

/// Merchant spending summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerchantSpending {
    pub merchant_name: String,
    pub category: String,
    pub total_amount: Decimal,
    pub currency: String,
    pub transaction_count: i64,
    pub average_amount: Decimal,
    pub last_transaction_date: DateTime<Utc>,
    pub location: String,
    pub frequency_score: f64, // How often user shops here (0-1)
}

/// Location-based spending insights
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationInsight {
    pub location: String,
    pub total_amount: Decimal,
    pub currency: String,
    pub transaction_count: i64,
    pub top_categories: Vec<String>,
    pub top_merchants: Vec<String>,
    pub percentage_of_total: f64,
}

/// Spending pattern analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpendingPattern {
    pub pattern_type: String, // "weekly", "seasonal", "merchant_loyalty", etc.
    pub description: String,
    pub confidence: f64, // Confidence score (0-1)
    pub average_amount: Decimal,
    pub currency: String,
    pub peak_periods: Vec<String>,
    pub insights: HashMap<String, String>,
}

/// Cashflow analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CashflowAnalysis {
    pub period: String,
    pub total_inflow: Decimal,  // Total money in (top-ups)
    pub total_outflow: Decimal, // Total money out (spending)
    pub net_flow: Decimal,      // Net cashflow
    pub currency: String,
    pub daily_flow: Vec<SpendingDataPoint>,
    pub average_daily_spending: Decimal,
    pub projected_monthly_spending: Decimal,
    pub spending_velocity: f64, // Spending rate trend
}

/// Platform-wide insights (admin only)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformInsights {
    pub total_users: i64,
    pub total_volume: Decimal,
    pub currency: String,
    pub total_transactions: i64,
    pub average_transaction: Decimal,
    pub top_categories: Vec<CategorySpending>,
    pub top_merchants: Vec<MerchantSpending>,
    pub growth_metrics: HashMap<String, String>,
    pub volume_trend: Vec<SpendingDataPoint>,
}

/// Spending insights repository trait
pub trait SpendingInsightsRepository: Send + Sync {
    /// Get spending summary for user
    fn get_spending_summary(
        &self,
        user_id: Uuid,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
        currency: Option<String>,
    ) -> Result<(Decimal, i64, Vec<CategorySpending>), String>;

    /// Get category breakdown
    fn get_category_breakdown(
        &self,
        user_id: Uuid,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
        currency: Option<String>,
    ) -> Result<Vec<CategorySpending>, String>;

    /// Get spending trends
    fn get_spending_trends(
        &self,
        user_id: Uuid,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
        period: TimePeriod,
    ) -> Result<Vec<SpendingDataPoint>, String>;

    /// Create budget
    fn create_budget(&self, budget: Budget) -> Result<Budget, String>;

    /// Get budgets by user
    fn get_budgets_by_user(&self, user_id: Uuid) -> Result<Vec<Budget>, String>;

    /// Update budget
    fn update_budget(&self, budget: Budget) -> Result<Budget, String>;

    /// Delete budget
    fn delete_budget(&self, budget_id: Uuid) -> Result<(), String>;

    /// Create spending alert
    fn create_spending_alert(&self, alert: SpendingAlert) -> Result<SpendingAlert, String>;

    /// Get spending alerts by user
    fn get_spending_alerts_by_user(&self, user_id: Uuid) -> Result<Vec<SpendingAlert>, String>;

    /// Update spending alert
    fn update_spending_alert(&self, alert: SpendingAlert) -> Result<SpendingAlert, String>;

    /// Delete spending alert
    fn delete_spending_alert(&self, alert_id: Uuid) -> Result<(), String>;

    /// Get top merchants for user
    fn get_top_merchants(
        &self,
        user_id: Uuid,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
        limit: usize,
    ) -> Result<Vec<MerchantSpending>, String>;

    /// Get location insights
    fn get_location_insights(
        &self,
        user_id: Uuid,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> Result<Vec<LocationInsight>, String>;

    /// Get spending patterns
    fn get_spending_patterns(
        &self,
        user_id: Uuid,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> Result<Vec<SpendingPattern>, String>;

    /// Get cashflow analysis
    fn get_cashflow_analysis(
        &self,
        user_id: Uuid,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> Result<CashflowAnalysis, String>;

    /// Get platform insights (admin only)
    fn get_platform_insights(
        &self,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> Result<PlatformInsights, String>;
}

/// In-memory spending insights repository implementation
pub struct InMemorySpendingInsightsRepository {
    budgets: Arc<RwLock<HashMap<Uuid, Budget>>>,
    spending_alerts: Arc<RwLock<HashMap<Uuid, SpendingAlert>>>,
    user_budgets: Arc<RwLock<HashMap<Uuid, Vec<Uuid>>>>, // user_id -> budget_ids
    user_alerts: Arc<RwLock<HashMap<Uuid, Vec<Uuid>>>>,  // user_id -> alert_ids
}

impl InMemorySpendingInsightsRepository {
    pub fn new() -> Self {
        Self {
            budgets: Arc::new(RwLock::new(HashMap::new())),
            spending_alerts: Arc::new(RwLock::new(HashMap::new())),
            user_budgets: Arc::new(RwLock::new(HashMap::new())),
            user_alerts: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Analyze transactions to generate category spending
    fn analyze_transactions_by_category(
        &self,
        transactions: &[CardTransaction],
        total_amount: Decimal,
    ) -> Vec<CategorySpending> {
        let mut category_map: HashMap<MerchantCategory, (Decimal, i64, Vec<String>)> = HashMap::new();

        for transaction in transactions {
            if transaction.status != CardTransactionStatus::Settled {
                continue;
            }

            let category = transaction.merchant
                .as_ref()
                .map(|m| m.category_code.clone())
                .unwrap_or(MerchantCategory::Other);

            let merchant_name = transaction.merchant
                .as_ref()
                .map(|m| m.name.clone())
                .unwrap_or_else(|| "Unknown".to_string());

            let entry = category_map.entry(category.clone()).or_insert((Decimal::ZERO, 0, Vec::new()));
            entry.0 += transaction.amount;
            entry.1 += 1;
            if !entry.2.contains(&merchant_name) && entry.2.len() < 5 {
                entry.2.push(merchant_name);
            }
        }

        category_map
            .into_iter()
            .map(|(category, (amount, count, merchants))| {
                let percentage = if total_amount > Decimal::ZERO {
                    (amount / total_amount * Decimal::from(100)).to_f64().unwrap_or(0.0)
                } else {
                    0.0
                };

                let average_amount = if count > 0 {
                    amount / Decimal::from(count)
                } else {
                    Decimal::ZERO
                };

                CategorySpending {
                    category: format!("{:?}", category),
                    category_code: category,
                    total_amount: amount,
                    currency: "USD".to_string(), // Default currency
                    transaction_count: count,
                    average_amount,
                    percentage_of_total: percentage,
                    budget_amount: None,
                    budget_utilization: None,
                    top_merchants: merchants,
                }
            })
            .collect()
    }
}

impl SpendingInsightsRepository for InMemorySpendingInsightsRepository {
    fn get_spending_summary(
        &self,
        user_id: Uuid,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
        currency: Option<String>,
    ) -> Result<(Decimal, i64, Vec<CategorySpending>), String> {
        // This would typically query the card transactions from the card repository
        // For now, return mock data
        let total_amount = Decimal::from(1250);
        let transaction_count = 15;
        let categories = vec![
            CategorySpending {
                category: "Restaurant".to_string(),
                category_code: MerchantCategory::Restaurant,
                total_amount: Decimal::from(450),
                currency: currency.unwrap_or_else(|| "USD".to_string()),
                transaction_count: 6,
                average_amount: Decimal::from(75),
                percentage_of_total: 36.0,
                budget_amount: Some(Decimal::from(500)),
                budget_utilization: Some(90.0),
                top_merchants: vec!["Coffee Shop".to_string(), "Pizza Place".to_string()],
            },
            CategorySpending {
                category: "Grocery".to_string(),
                category_code: MerchantCategory::Grocery,
                total_amount: Decimal::from(350),
                currency: currency.unwrap_or_else(|| "USD".to_string()),
                transaction_count: 4,
                average_amount: Decimal::from(87.5),
                percentage_of_total: 28.0,
                budget_amount: Some(Decimal::from(400)),
                budget_utilization: Some(87.5),
                top_merchants: vec!["Supermarket".to_string(), "Organic Store".to_string()],
            },
        ];

        Ok((total_amount, transaction_count, categories))
    }

    fn get_category_breakdown(
        &self,
        user_id: Uuid,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
        currency: Option<String>,
    ) -> Result<Vec<CategorySpending>, String> {
        let (_, _, categories) = self.get_spending_summary(user_id, start_date, end_date, currency)?;
        Ok(categories)
    }

    fn get_spending_trends(
        &self,
        user_id: Uuid,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
        period: TimePeriod,
    ) -> Result<Vec<SpendingDataPoint>, String> {
        // Generate mock trend data
        let mut trends = Vec::new();
        let mut current_date = start_date;
        let mut day_counter = 0;

        while current_date <= end_date && day_counter < 30 {
            trends.push(SpendingDataPoint {
                timestamp: current_date,
                amount: Decimal::from(50 + (day_counter % 7) * 20), // Varying amounts
                currency: "USD".to_string(),
                transaction_count: 2 + (day_counter % 3),
                period_label: current_date.format("%b %d").to_string(),
            });

            current_date += chrono::Duration::days(1);
            day_counter += 1;
        }

        Ok(trends)
    }

    fn create_budget(&self, budget: Budget) -> Result<Budget, String> {
        let mut budgets = self.budgets.write().map_err(|_| "Failed to acquire write lock")?;
        let mut user_budgets = self.user_budgets.write().map_err(|_| "Failed to acquire write lock")?;

        let budget_id = budget.id;
        let user_id = budget.user_id;

        budgets.insert(budget_id, budget.clone());
        user_budgets.entry(user_id).or_insert_with(Vec::new).push(budget_id);

        Ok(budget)
    }

    fn get_budgets_by_user(&self, user_id: Uuid) -> Result<Vec<Budget>, String> {
        let budgets = self.budgets.read().map_err(|_| "Failed to acquire read lock")?;
        let user_budgets = self.user_budgets.read().map_err(|_| "Failed to acquire read lock")?;

        let budget_ids = user_budgets.get(&user_id).unwrap_or(&Vec::new());
        let user_budget_list = budget_ids.iter()
            .filter_map(|id| budgets.get(id).cloned())
            .collect();

        Ok(user_budget_list)
    }

    fn update_budget(&self, budget: Budget) -> Result<Budget, String> {
        let mut budgets = self.budgets.write().map_err(|_| "Failed to acquire write lock")?;

        if !budgets.contains_key(&budget.id) {
            return Err("Budget not found".to_string());
        }

        budgets.insert(budget.id, budget.clone());
        Ok(budget)
    }

    fn delete_budget(&self, budget_id: Uuid) -> Result<(), String> {
        let mut budgets = self.budgets.write().map_err(|_| "Failed to acquire write lock")?;
        let mut user_budgets = self.user_budgets.write().map_err(|_| "Failed to acquire write lock")?;

        if let Some(budget) = budgets.remove(&budget_id) {
            // Remove from user's budget list
            if let Some(user_budget_list) = user_budgets.get_mut(&budget.user_id) {
                user_budget_list.retain(|&id| id != budget_id);
            }
        }

        Ok(())
    }

    fn create_spending_alert(&self, alert: SpendingAlert) -> Result<SpendingAlert, String> {
        let mut alerts = self.spending_alerts.write().map_err(|_| "Failed to acquire write lock")?;
        let mut user_alerts = self.user_alerts.write().map_err(|_| "Failed to acquire write lock")?;

        let alert_id = alert.id;
        let user_id = alert.user_id;

        alerts.insert(alert_id, alert.clone());
        user_alerts.entry(user_id).or_insert_with(Vec::new).push(alert_id);

        Ok(alert)
    }

    fn get_spending_alerts_by_user(&self, user_id: Uuid) -> Result<Vec<SpendingAlert>, String> {
        let alerts = self.spending_alerts.read().map_err(|_| "Failed to acquire read lock")?;
        let user_alerts = self.user_alerts.read().map_err(|_| "Failed to acquire read lock")?;

        let alert_ids = user_alerts.get(&user_id).unwrap_or(&Vec::new());
        let user_alert_list = alert_ids.iter()
            .filter_map(|id| alerts.get(id).cloned())
            .collect();

        Ok(user_alert_list)
    }

    fn update_spending_alert(&self, alert: SpendingAlert) -> Result<SpendingAlert, String> {
        let mut alerts = self.spending_alerts.write().map_err(|_| "Failed to acquire write lock")?;

        if !alerts.contains_key(&alert.id) {
            return Err("Spending alert not found".to_string());
        }

        alerts.insert(alert.id, alert.clone());
        Ok(alert)
    }

    fn delete_spending_alert(&self, alert_id: Uuid) -> Result<(), String> {
        let mut alerts = self.spending_alerts.write().map_err(|_| "Failed to acquire write lock")?;
        let mut user_alerts = self.user_alerts.write().map_err(|_| "Failed to acquire write lock")?;

        if let Some(alert) = alerts.remove(&alert_id) {
            // Remove from user's alert list
            if let Some(user_alert_list) = user_alerts.get_mut(&alert.user_id) {
                user_alert_list.retain(|&id| id != alert_id);
            }
        }

        Ok(())
    }

    fn get_top_merchants(
        &self,
        user_id: Uuid,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
        limit: usize,
    ) -> Result<Vec<MerchantSpending>, String> {
        // Generate mock merchant data
        let merchants = vec![
            MerchantSpending {
                merchant_name: "Coffee Shop".to_string(),
                category: "Restaurant".to_string(),
                total_amount: Decimal::from(180),
                currency: "USD".to_string(),
                transaction_count: 12,
                average_amount: Decimal::from(15),
                last_transaction_date: Utc::now() - chrono::Duration::days(1),
                location: "New York, NY".to_string(),
                frequency_score: 0.85,
            },
            MerchantSpending {
                merchant_name: "Supermarket".to_string(),
                category: "Grocery".to_string(),
                total_amount: Decimal::from(250),
                currency: "USD".to_string(),
                transaction_count: 4,
                average_amount: Decimal::from(62.5),
                last_transaction_date: Utc::now() - chrono::Duration::days(3),
                location: "New York, NY".to_string(),
                frequency_score: 0.65,
            },
        ];

        Ok(merchants.into_iter().take(limit).collect())
    }

    fn get_location_insights(
        &self,
        user_id: Uuid,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> Result<Vec<LocationInsight>, String> {
        // Generate mock location data
        let locations = vec![
            LocationInsight {
                location: "New York, NY".to_string(),
                total_amount: Decimal::from(1100),
                currency: "USD".to_string(),
                transaction_count: 18,
                top_categories: vec!["Restaurant".to_string(), "Grocery".to_string()],
                top_merchants: vec!["Coffee Shop".to_string(), "Supermarket".to_string()],
                percentage_of_total: 88.0,
            },
            LocationInsight {
                location: "Boston, MA".to_string(),
                total_amount: Decimal::from(150),
                currency: "USD".to_string(),
                transaction_count: 2,
                top_categories: vec!["Travel".to_string()],
                top_merchants: vec!["Hotel".to_string()],
                percentage_of_total: 12.0,
            },
        ];

        Ok(locations)
    }

    fn get_spending_patterns(
        &self,
        user_id: Uuid,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> Result<Vec<SpendingPattern>, String> {
        // Generate mock pattern data
        let patterns = vec![
            SpendingPattern {
                pattern_type: "weekly".to_string(),
                description: "Higher spending on weekends".to_string(),
                confidence: 0.85,
                average_amount: Decimal::from(75),
                currency: "USD".to_string(),
                peak_periods: vec!["Saturday".to_string(), "Sunday".to_string()],
                insights: {
                    let mut map = HashMap::new();
                    map.insert("weekend_multiplier".to_string(), "1.4x".to_string());
                    map.insert("primary_category".to_string(), "Restaurant".to_string());
                    map
                },
            },
            SpendingPattern {
                pattern_type: "merchant_loyalty".to_string(),
                description: "Regular visits to Coffee Shop".to_string(),
                confidence: 0.92,
                average_amount: Decimal::from(15),
                currency: "USD".to_string(),
                peak_periods: vec!["Monday".to_string(), "Wednesday".to_string(), "Friday".to_string()],
                insights: {
                    let mut map = HashMap::new();
                    map.insert("visit_frequency".to_string(), "3x per week".to_string());
                    map.insert("consistency_score".to_string(), "0.92".to_string());
                    map
                },
            },
        ];

        Ok(patterns)
    }

    fn get_cashflow_analysis(
        &self,
        user_id: Uuid,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> Result<CashflowAnalysis, String> {
        // Generate mock cashflow data
        let daily_flow = vec![
            SpendingDataPoint {
                timestamp: start_date,
                amount: Decimal::from(-45), // Negative for outflow
                currency: "USD".to_string(),
                transaction_count: 2,
                period_label: "Day 1".to_string(),
            },
            SpendingDataPoint {
                timestamp: start_date + chrono::Duration::days(1),
                amount: Decimal::from(-32),
                currency: "USD".to_string(),
                transaction_count: 1,
                period_label: "Day 2".to_string(),
            },
        ];

        let analysis = CashflowAnalysis {
            period: "Last 30 days".to_string(),
            total_inflow: Decimal::from(500),  // Top-ups
            total_outflow: Decimal::from(1250), // Spending
            net_flow: Decimal::from(-750),     // Net negative
            currency: "USD".to_string(),
            daily_flow,
            average_daily_spending: Decimal::from(41.67),
            projected_monthly_spending: Decimal::from(1250),
            spending_velocity: 1.15, // 15% increase trend
        };

        Ok(analysis)
    }

    fn get_platform_insights(
        &self,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> Result<PlatformInsights, String> {
        // Generate mock platform data
        let insights = PlatformInsights {
            total_users: 1250,
            total_volume: Decimal::from(125000),
            currency: "USD".to_string(),
            total_transactions: 8500,
            average_transaction: Decimal::from(14.71),
            top_categories: vec![
                CategorySpending {
                    category: "Restaurant".to_string(),
                    category_code: MerchantCategory::Restaurant,
                    total_amount: Decimal::from(45000),
                    currency: "USD".to_string(),
                    transaction_count: 3200,
                    average_amount: Decimal::from(14.06),
                    percentage_of_total: 36.0,
                    budget_amount: None,
                    budget_utilization: None,
                    top_merchants: vec!["Coffee Chain".to_string(), "Fast Food".to_string()],
                },
            ],
            top_merchants: vec![
                MerchantSpending {
                    merchant_name: "Coffee Chain".to_string(),
                    category: "Restaurant".to_string(),
                    total_amount: Decimal::from(18000),
                    currency: "USD".to_string(),
                    transaction_count: 1200,
                    average_amount: Decimal::from(15),
                    last_transaction_date: Utc::now(),
                    location: "Multiple".to_string(),
                    frequency_score: 0.95,
                },
            ],
            growth_metrics: {
                let mut map = HashMap::new();
                map.insert("monthly_growth".to_string(), "12.5%".to_string());
                map.insert("user_growth".to_string(), "8.3%".to_string());
                map
            },
            volume_trend: vec![
                SpendingDataPoint {
                    timestamp: start_date,
                    amount: Decimal::from(100000),
                    currency: "USD".to_string(),
                    transaction_count: 7000,
                    period_label: "Month 1".to_string(),
                },
                SpendingDataPoint {
                    timestamp: end_date,
                    amount: Decimal::from(125000),
                    currency: "USD".to_string(),
                    transaction_count: 8500,
                    period_label: "Month 2".to_string(),
                },
            ],
        };

        Ok(insights)
    }
}

impl Default for InMemorySpendingInsightsRepository {
    fn default() -> Self {
        Self::new()
    }
}
