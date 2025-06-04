//! In-memory implementation of CardFundingRepository

use super::card_funding::*;
use std::collections::HashMap;
use uuid::Uuid;
use rust_decimal::Decimal;
use chrono::{DateTime, Utc};

#[async_trait::async_trait]
impl CardFundingRepository for InMemoryCardFundingRepository {
    // Funding source operations
    async fn create_funding_source(&self, source: &FundingSource) -> Result<FundingSource, String> {
        let mut sources = self.funding_sources.write().unwrap();
        sources.insert(source.id, source.clone());
        Ok(source.clone())
    }

    async fn get_funding_source(&self, id: &Uuid) -> Result<Option<FundingSource>, String> {
        let sources = self.funding_sources.read().unwrap();
        Ok(sources.get(id).cloned())
    }

    async fn get_funding_source_by_user(&self, user_id: &Uuid, source_id: &Uuid) -> Result<Option<FundingSource>, String> {
        let sources = self.funding_sources.read().unwrap();
        Ok(sources.get(source_id)
            .filter(|source| source.user_id == *user_id)
            .cloned())
    }

    async fn list_funding_sources(
        &self,
        user_id: &Uuid,
        source_type: Option<FundingSourceType>,
        status: Option<FundingSourceStatus>,
        page: i32,
        page_size: i32,
    ) -> Result<(Vec<FundingSource>, i64), String> {
        let sources = self.funding_sources.read().unwrap();
        let mut filtered: Vec<_> = sources
            .values()
            .filter(|source| source.user_id == *user_id)
            .filter(|source| source_type.as_ref().map_or(true, |t| source.source_type == *t))
            .filter(|source| status.as_ref().map_or(true, |s| source.status == *s))
            .cloned()
            .collect();

        filtered.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        let total = filtered.len() as i64;
        let start = ((page - 1) * page_size) as usize;
        let end = (start + page_size as usize).min(filtered.len());

        Ok((filtered[start..end].to_vec(), total))
    }

    async fn update_funding_source(&self, source: &FundingSource) -> Result<FundingSource, String> {
        let mut sources = self.funding_sources.write().unwrap();
        sources.insert(source.id, source.clone());
        Ok(source.clone())
    }

    async fn delete_funding_source(&self, id: &Uuid) -> Result<bool, String> {
        let mut sources = self.funding_sources.write().unwrap();
        Ok(sources.remove(id).is_some())
    }

    // Funding transaction operations
    async fn create_funding_transaction(&self, transaction: &FundingTransaction) -> Result<FundingTransaction, String> {
        let mut transactions = self.funding_transactions.write().unwrap();
        transactions.insert(transaction.id, transaction.clone());
        Ok(transaction.clone())
    }

    async fn get_funding_transaction(&self, id: &Uuid) -> Result<Option<FundingTransaction>, String> {
        let transactions = self.funding_transactions.read().unwrap();
        Ok(transactions.get(id).cloned())
    }

    async fn get_funding_transaction_by_user(&self, user_id: &Uuid, transaction_id: &Uuid) -> Result<Option<FundingTransaction>, String> {
        let transactions = self.funding_transactions.read().unwrap();
        Ok(transactions.get(transaction_id)
            .filter(|tx| tx.user_id == *user_id)
            .cloned())
    }

    async fn list_funding_transactions(
        &self,
        user_id: &Uuid,
        card_id: Option<Uuid>,
        source_id: Option<Uuid>,
        status: Option<FundingTransactionStatus>,
        page: i32,
        page_size: i32,
    ) -> Result<(Vec<FundingTransaction>, i64), String> {
        let transactions = self.funding_transactions.read().unwrap();
        let mut filtered: Vec<_> = transactions
            .values()
            .filter(|tx| tx.user_id == *user_id)
            .filter(|tx| card_id.map_or(true, |id| tx.card_id == id))
            .filter(|tx| source_id.map_or(true, |id| tx.funding_source_id == id))
            .filter(|tx| status.as_ref().map_or(true, |s| tx.status == *s))
            .cloned()
            .collect();

        filtered.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        let total = filtered.len() as i64;
        let start = ((page - 1) * page_size) as usize;
        let end = (start + page_size as usize).min(filtered.len());

        Ok((filtered[start..end].to_vec(), total))
    }

    async fn update_funding_transaction(&self, transaction: &FundingTransaction) -> Result<FundingTransaction, String> {
        let mut transactions = self.funding_transactions.write().unwrap();
        transactions.insert(transaction.id, transaction.clone());
        Ok(transaction.clone())
    }

    async fn get_transactions_by_reference(&self, reference: &str) -> Result<Option<FundingTransaction>, String> {
        let transactions = self.funding_transactions.read().unwrap();
        Ok(transactions
            .values()
            .find(|tx| tx.reference_number == reference)
            .cloned())
    }

    // Funding limits operations
    async fn get_funding_limits(&self, user_id: &Uuid) -> Result<Option<FundingLimits>, String> {
        let limits = self.funding_limits.read().unwrap();
        Ok(limits.get(user_id).cloned())
    }

    async fn create_funding_limits(&self, limits: &FundingLimits) -> Result<FundingLimits, String> {
        let mut funding_limits = self.funding_limits.write().unwrap();
        funding_limits.insert(limits.user_id, limits.clone());
        Ok(limits.clone())
    }

    async fn update_funding_limits(&self, limits: &FundingLimits) -> Result<FundingLimits, String> {
        let mut funding_limits = self.funding_limits.write().unwrap();
        funding_limits.insert(limits.user_id, limits.clone());
        Ok(limits.clone())
    }

    async fn reset_daily_limits(&self, user_id: &Uuid) -> Result<bool, String> {
        let mut limits = self.funding_limits.write().unwrap();
        if let Some(user_limits) = limits.get_mut(user_id) {
            user_limits.daily_used = Decimal::ZERO;
            user_limits.daily_transactions_used = 0;
            user_limits.last_reset_daily = Utc::now();
            user_limits.updated_at = Utc::now();
            Ok(true)
        } else {
            Ok(false)
        }
    }

    async fn reset_monthly_limits(&self, user_id: &Uuid) -> Result<bool, String> {
        let mut limits = self.funding_limits.write().unwrap();
        if let Some(user_limits) = limits.get_mut(user_id) {
            user_limits.monthly_used = Decimal::ZERO;
            user_limits.monthly_transactions_used = 0;
            user_limits.last_reset_monthly = Utc::now();
            user_limits.updated_at = Utc::now();
            Ok(true)
        } else {
            Ok(false)
        }
    }

    async fn reset_yearly_limits(&self, user_id: &Uuid) -> Result<bool, String> {
        let mut limits = self.funding_limits.write().unwrap();
        if let Some(user_limits) = limits.get_mut(user_id) {
            user_limits.yearly_used = Decimal::ZERO;
            user_limits.last_reset_yearly = Utc::now();
            user_limits.updated_at = Utc::now();
            Ok(true)
        } else {
            Ok(false)
        }
    }

    // Analytics operations
    async fn get_funding_metrics(
        &self,
        start_date: &DateTime<Utc>,
        end_date: &DateTime<Utc>,
        source_type: Option<FundingSourceType>,
        currency: Option<String>,
    ) -> Result<FundingMetrics, String> {
        let transactions = self.funding_transactions.read().unwrap();
        let sources = self.funding_sources.read().unwrap();

        let filtered_transactions: Vec<_> = transactions
            .values()
            .filter(|tx| tx.created_at >= *start_date && tx.created_at <= *end_date)
            .filter(|tx| tx.status == FundingTransactionStatus::Completed)
            .filter(|tx| currency.as_ref().map_or(true, |c| tx.currency == *c))
            .filter(|tx| {
                if let Some(ref st) = source_type {
                    if let Some(source) = sources.get(&tx.funding_source_id) {
                        source.source_type == *st
                    } else {
                        false
                    }
                } else {
                    true
                }
            })
            .collect();

        let total_volume = filtered_transactions.iter().map(|tx| tx.amount).sum();
        let total_fees = filtered_transactions.iter().map(|tx| tx.fee_amount).sum();
        let total_transactions = filtered_transactions.len() as i64;
        let average_transaction_size = if total_transactions > 0 {
            total_volume / Decimal::from(total_transactions)
        } else {
            Decimal::ZERO
        };

        // Calculate success rate (simplified for in-memory implementation)
        let success_rate = Decimal::from(100); // Assume 100% for completed transactions

        // Group by source type
        let mut by_source: HashMap<FundingSourceType, (Decimal, Decimal, i64)> = HashMap::new();
        for tx in &filtered_transactions {
            if let Some(source) = sources.get(&tx.funding_source_id) {
                let entry = by_source.entry(source.source_type.clone()).or_insert((Decimal::ZERO, Decimal::ZERO, 0));
                entry.0 += tx.amount;
                entry.1 += tx.fee_amount;
                entry.2 += 1;
            }
        }

        let by_source_metrics: Vec<FundingSourceMetrics> = by_source
            .into_iter()
            .map(|(source_type, (volume, fees, count))| FundingSourceMetrics {
                source_type,
                volume,
                fees,
                transaction_count: count,
                success_rate: Decimal::from(100),
            })
            .collect();

        // Group by currency
        let mut by_currency: HashMap<String, (Decimal, Decimal, i64)> = HashMap::new();
        for tx in &filtered_transactions {
            let entry = by_currency.entry(tx.currency.clone()).or_insert((Decimal::ZERO, Decimal::ZERO, 0));
            entry.0 += tx.amount;
            entry.1 += tx.fee_amount;
            entry.2 += 1;
        }

        let by_currency_metrics: Vec<CurrencyMetrics> = by_currency
            .into_iter()
            .map(|(currency, (volume, fees, count))| CurrencyMetrics {
                currency,
                volume,
                fees,
                transaction_count: count,
            })
            .collect();

        Ok(FundingMetrics {
            total_volume,
            total_fees,
            total_transactions,
            average_transaction_size,
            by_source: by_source_metrics,
            by_currency: by_currency_metrics,
            success_rate,
        })
    }

    async fn get_user_funding_volume(
        &self,
        user_id: &Uuid,
        start_date: &DateTime<Utc>,
        end_date: &DateTime<Utc>,
    ) -> Result<Decimal, String> {
        let transactions = self.funding_transactions.read().unwrap();
        let volume = transactions
            .values()
            .filter(|tx| tx.user_id == *user_id)
            .filter(|tx| tx.created_at >= *start_date && tx.created_at <= *end_date)
            .filter(|tx| tx.status == FundingTransactionStatus::Completed)
            .map(|tx| tx.amount)
            .sum();

        Ok(volume)
    }
}
