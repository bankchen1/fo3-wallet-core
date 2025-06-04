//! In-memory implementation of RewardsRepository

use super::rewards::*;
use std::collections::HashMap;
use uuid::Uuid;
use rust_decimal::Decimal;
use chrono::{DateTime, Utc};

#[async_trait::async_trait]
impl RewardsRepository for InMemoryRewardsRepository {
    // Reward rule operations
    async fn create_reward_rule(&self, rule: &RewardRule) -> Result<RewardRule, String> {
        let mut rules = self.reward_rules.write().unwrap();
        rules.insert(rule.id, rule.clone());
        Ok(rule.clone())
    }

    async fn get_reward_rule(&self, id: &Uuid) -> Result<Option<RewardRule>, String> {
        let rules = self.reward_rules.read().unwrap();
        Ok(rules.get(id).cloned())
    }

    async fn list_reward_rules(
        &self,
        rule_type: Option<RewardRuleType>,
        status: Option<RewardRuleStatus>,
        category: Option<String>,
        currency: Option<String>,
        active_only: bool,
        page: i32,
        page_size: i32,
    ) -> Result<(Vec<RewardRule>, i64), String> {
        let rules = self.reward_rules.read().unwrap();
        
        let filtered_rules: Vec<_> = rules
            .values()
            .filter(|rule| rule_type.as_ref().map_or(true, |t| rule.rule_type == *t))
            .filter(|rule| status.as_ref().map_or(true, |s| rule.status == *s))
            .filter(|rule| category.as_ref().map_or(true, |c| rule.categories.contains(c)))
            .filter(|rule| currency.as_ref().map_or(true, |c| rule.currencies.contains(c)))
            .filter(|rule| !active_only || rule.status == RewardRuleStatus::Active)
            .cloned()
            .collect();

        let total_count = filtered_rules.len() as i64;
        let start = (page * page_size) as usize;
        let end = std::cmp::min(start + page_size as usize, filtered_rules.len());
        
        let paginated_rules = if start < filtered_rules.len() {
            filtered_rules[start..end].to_vec()
        } else {
            Vec::new()
        };

        Ok((paginated_rules, total_count))
    }

    async fn update_reward_rule(&self, rule: &RewardRule) -> Result<RewardRule, String> {
        let mut rules = self.reward_rules.write().unwrap();
        rules.insert(rule.id, rule.clone());
        Ok(rule.clone())
    }

    async fn delete_reward_rule(&self, id: &Uuid) -> Result<(), String> {
        let mut rules = self.reward_rules.write().unwrap();
        rules.remove(id);
        Ok(())
    }

    // User reward operations
    async fn get_user_rewards(&self, user_id: &Uuid) -> Result<Option<UserRewards>, String> {
        let rewards = self.user_rewards.read().unwrap();
        Ok(rewards.get(user_id).cloned())
    }

    async fn create_user_rewards(&self, rewards: &UserRewards) -> Result<UserRewards, String> {
        let mut user_rewards = self.user_rewards.write().unwrap();
        user_rewards.insert(rewards.user_id, rewards.clone());
        Ok(rewards.clone())
    }

    async fn update_user_rewards(&self, rewards: &UserRewards) -> Result<UserRewards, String> {
        let mut user_rewards = self.user_rewards.write().unwrap();
        user_rewards.insert(rewards.user_id, rewards.clone());
        Ok(rewards.clone())
    }

    async fn get_reward_balance(&self, user_id: &Uuid, _currency: Option<String>) -> Result<Option<UserRewards>, String> {
        // For simplicity, currency filtering is not implemented in memory store
        self.get_user_rewards(user_id).await
    }

    async fn update_user_tier(&self, user_id: &Uuid, new_tier: UserRewardTier, reason: Option<String>) -> Result<UserRewards, String> {
        let mut user_rewards = self.user_rewards.write().unwrap();
        
        if let Some(mut rewards) = user_rewards.get(user_id).cloned() {
            let old_tier = rewards.current_tier.clone();
            rewards.current_tier = new_tier.clone();
            rewards.tier_multiplier = new_tier.multiplier();
            rewards.tier_benefits = new_tier.benefits();
            rewards.next_tier_threshold = new_tier.next_tier().map(|t| t.threshold()).unwrap_or(Decimal::ZERO);
            rewards.tier_upgrade_date = Some(Utc::now());
            rewards.updated_at = Utc::now();
            
            // Calculate tier progress
            if let Some(next_tier) = new_tier.next_tier() {
                let current_threshold = new_tier.threshold();
                let next_threshold = next_tier.threshold();
                let progress = if next_threshold > current_threshold {
                    ((rewards.lifetime_earned - current_threshold) / (next_threshold - current_threshold)).max(Decimal::ZERO).min(Decimal::ONE)
                } else {
                    Decimal::ONE
                };
                rewards.tier_progress = progress;
            } else {
                rewards.tier_progress = Decimal::ONE;
            }
            
            user_rewards.insert(*user_id, rewards.clone());
            
            // Create audit entry
            let audit_entry = RewardAuditTrailEntry {
                id: Uuid::new_v4(),
                user_id: Some(*user_id),
                action_type: "tier_change".to_string(),
                entity_type: "user_rewards".to_string(),
                entity_id: rewards.id,
                old_value: Some(format!("{:?}", old_tier)),
                new_value: Some(format!("{:?}", new_tier)),
                reason,
                performed_by: None, // Would be set by the service layer
                ip_address: None,
                user_agent: None,
                metadata: HashMap::new(),
                created_at: Utc::now(),
            };
            
            let mut audit_trail = self.audit_trail.write().unwrap();
            audit_trail.push(audit_entry);
            
            Ok(rewards)
        } else {
            Err(format!("User rewards not found for user {}", user_id))
        }
    }

    // Reward transaction operations
    async fn create_reward_transaction(&self, transaction: &RewardTransaction) -> Result<RewardTransaction, String> {
        let mut transactions = self.reward_transactions.write().unwrap();
        let mut reference_numbers = self.reference_numbers.write().unwrap();
        
        // Check for duplicate reference number
        if reference_numbers.contains_key(&transaction.reference_number) {
            return Err(format!("Reference number '{}' already exists", transaction.reference_number));
        }
        
        transactions.insert(transaction.id, transaction.clone());
        reference_numbers.insert(transaction.reference_number.clone(), transaction.id);
        
        Ok(transaction.clone())
    }

    async fn get_reward_transaction(&self, id: &Uuid) -> Result<Option<RewardTransaction>, String> {
        let transactions = self.reward_transactions.read().unwrap();
        Ok(transactions.get(id).cloned())
    }

    async fn list_reward_transactions(
        &self,
        user_id: Option<Uuid>,
        transaction_type: Option<RewardTransactionType>,
        status: Option<RewardTransactionStatus>,
        start_date: Option<DateTime<Utc>>,
        end_date: Option<DateTime<Utc>>,
        page: i32,
        page_size: i32,
    ) -> Result<(Vec<RewardTransaction>, i64), String> {
        let transactions = self.reward_transactions.read().unwrap();
        
        let filtered_transactions: Vec<_> = transactions
            .values()
            .filter(|tx| user_id.map_or(true, |id| tx.user_id == id))
            .filter(|tx| transaction_type.as_ref().map_or(true, |t| tx.transaction_type == *t))
            .filter(|tx| status.as_ref().map_or(true, |s| tx.status == *s))
            .filter(|tx| start_date.map_or(true, |d| tx.created_at >= d))
            .filter(|tx| end_date.map_or(true, |d| tx.created_at <= d))
            .cloned()
            .collect();

        let total_count = filtered_transactions.len() as i64;
        let start = (page * page_size) as usize;
        let end = std::cmp::min(start + page_size as usize, filtered_transactions.len());
        
        let paginated_transactions = if start < filtered_transactions.len() {
            filtered_transactions[start..end].to_vec()
        } else {
            Vec::new()
        };

        Ok((paginated_transactions, total_count))
    }

    async fn update_reward_transaction(&self, transaction: &RewardTransaction) -> Result<RewardTransaction, String> {
        let mut transactions = self.reward_transactions.write().unwrap();
        transactions.insert(transaction.id, transaction.clone());
        Ok(transaction.clone())
    }

    async fn expire_points(&self, user_id: Option<Uuid>, expiration_date: DateTime<Utc>, dry_run: bool) -> Result<(Vec<RewardTransaction>, i64), String> {
        let mut transactions = self.reward_transactions.write().unwrap();
        let mut expired_transactions = Vec::new();
        let mut users_affected = std::collections::HashSet::new();
        
        for transaction in transactions.values_mut() {
            if user_id.map_or(true, |id| transaction.user_id == id) &&
               transaction.expires_at.map_or(false, |exp| exp <= expiration_date) &&
               !transaction.is_expired &&
               transaction.transaction_type == RewardTransactionType::Earned {
                
                if !dry_run {
                    transaction.is_expired = true;
                    transaction.status = RewardTransactionStatus::Expired;
                    transaction.updated_at = Utc::now();
                }
                
                expired_transactions.push(transaction.clone());
                users_affected.insert(transaction.user_id);
            }
        }
        
        Ok((expired_transactions, users_affected.len() as i64))
    }

    // Redemption operations
    async fn create_redemption(&self, redemption: &Redemption) -> Result<Redemption, String> {
        let mut redemptions = self.redemptions.write().unwrap();
        redemptions.insert(redemption.id, redemption.clone());
        Ok(redemption.clone())
    }

    async fn get_redemption(&self, id: &Uuid) -> Result<Option<Redemption>, String> {
        let redemptions = self.redemptions.read().unwrap();
        Ok(redemptions.get(id).cloned())
    }

    async fn list_redemptions(
        &self,
        user_id: Option<Uuid>,
        redemption_type: Option<RedemptionType>,
        status: Option<RedemptionStatus>,
        start_date: Option<DateTime<Utc>>,
        end_date: Option<DateTime<Utc>>,
        page: i32,
        page_size: i32,
    ) -> Result<(Vec<Redemption>, i64), String> {
        let redemptions = self.redemptions.read().unwrap();
        
        let filtered_redemptions: Vec<_> = redemptions
            .values()
            .filter(|r| user_id.map_or(true, |id| r.user_id == id))
            .filter(|r| redemption_type.as_ref().map_or(true, |t| r.redemption_type == *t))
            .filter(|r| status.as_ref().map_or(true, |s| r.status == *s))
            .filter(|r| start_date.map_or(true, |d| r.created_at >= d))
            .filter(|r| end_date.map_or(true, |d| r.created_at <= d))
            .cloned()
            .collect();

        let total_count = filtered_redemptions.len() as i64;
        let start = (page * page_size) as usize;
        let end = std::cmp::min(start + page_size as usize, filtered_redemptions.len());
        
        let paginated_redemptions = if start < filtered_redemptions.len() {
            filtered_redemptions[start..end].to_vec()
        } else {
            Vec::new()
        };

        Ok((paginated_redemptions, total_count))
    }

    async fn update_redemption(&self, redemption: &Redemption) -> Result<Redemption, String> {
        let mut redemptions = self.redemptions.write().unwrap();
        redemptions.insert(redemption.id, redemption.clone());
        Ok(redemption.clone())
    }

    async fn cancel_redemption(&self, id: &Uuid, reason: String) -> Result<Redemption, String> {
        let mut redemptions = self.redemptions.write().unwrap();
        
        if let Some(mut redemption) = redemptions.get(id).cloned() {
            redemption.status = RedemptionStatus::Cancelled;
            redemption.description = Some(reason);
            redemption.updated_at = Utc::now();
            
            redemptions.insert(*id, redemption.clone());
            Ok(redemption)
        } else {
            Err(format!("Redemption not found: {}", id))
        }
    }

    // Redemption option operations
    async fn create_redemption_option(&self, option: &RedemptionOption) -> Result<RedemptionOption, String> {
        let mut options = self.redemption_options.write().unwrap();
        options.insert(option.id, option.clone());
        Ok(option.clone())
    }

    async fn get_redemption_option(&self, id: &Uuid) -> Result<Option<RedemptionOption>, String> {
        let options = self.redemption_options.read().unwrap();
        Ok(options.get(id).cloned())
    }

    async fn list_redemption_options(
        &self,
        redemption_type: Option<RedemptionType>,
        minimum_tier: Option<UserRewardTier>,
        minimum_points: Option<Decimal>,
        active_only: bool,
        page: i32,
        page_size: i32,
    ) -> Result<(Vec<RedemptionOption>, i64), String> {
        let options = self.redemption_options.read().unwrap();

        let filtered_options: Vec<_> = options
            .values()
            .filter(|opt| redemption_type.as_ref().map_or(true, |t| opt.redemption_type == *t))
            .filter(|opt| minimum_tier.as_ref().map_or(true, |t| opt.minimum_tier <= *t))
            .filter(|opt| minimum_points.as_ref().map_or(true, |p| opt.points_required <= *p))
            .filter(|opt| !active_only || opt.is_active)
            .cloned()
            .collect();

        let total_count = filtered_options.len() as i64;
        let start = (page * page_size) as usize;
        let end = std::cmp::min(start + page_size as usize, filtered_options.len());

        let paginated_options = if start < filtered_options.len() {
            filtered_options[start..end].to_vec()
        } else {
            Vec::new()
        };

        Ok((paginated_options, total_count))
    }

    async fn update_redemption_option(&self, option: &RedemptionOption) -> Result<RedemptionOption, String> {
        let mut options = self.redemption_options.write().unwrap();
        options.insert(option.id, option.clone());
        Ok(option.clone())
    }

    // Analytics operations
    async fn get_reward_metrics(
        &self,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
        rule_types: Vec<RewardRuleType>,
        tiers: Vec<UserRewardTier>,
        currencies: Vec<String>,
    ) -> Result<RewardMetrics, String> {
        let transactions = self.reward_transactions.read().unwrap();
        let redemptions = self.redemptions.read().unwrap();
        let user_rewards = self.user_rewards.read().unwrap();

        // Filter transactions by date range
        let filtered_transactions: Vec<_> = transactions
            .values()
            .filter(|tx| tx.created_at >= start_date && tx.created_at <= end_date)
            .filter(|tx| currencies.is_empty() || currencies.contains(&tx.currency))
            .collect();

        // Filter redemptions by date range
        let filtered_redemptions: Vec<_> = redemptions
            .values()
            .filter(|r| r.created_at >= start_date && r.created_at <= end_date)
            .filter(|r| currencies.is_empty() || currencies.contains(&r.currency))
            .collect();

        // Calculate overall metrics
        let total_points_awarded = transactions
            .values()
            .filter(|tx| tx.transaction_type == RewardTransactionType::Earned)
            .map(|tx| tx.points)
            .sum();

        let total_points_redeemed = transactions
            .values()
            .filter(|tx| tx.transaction_type == RewardTransactionType::Redeemed)
            .map(|tx| tx.points.abs())
            .sum();

        let total_points_expired = transactions
            .values()
            .filter(|tx| tx.transaction_type == RewardTransactionType::Expired)
            .map(|tx| tx.points.abs())
            .sum();

        let total_cash_value = redemptions
            .values()
            .filter(|r| r.status == RedemptionStatus::Completed)
            .map(|r| r.cash_value)
            .sum();

        // Period metrics
        let period_points_awarded = filtered_transactions
            .iter()
            .filter(|tx| tx.transaction_type == RewardTransactionType::Earned)
            .map(|tx| tx.points)
            .sum();

        let period_points_redeemed = filtered_transactions
            .iter()
            .filter(|tx| tx.transaction_type == RewardTransactionType::Redeemed)
            .map(|tx| tx.points.abs())
            .sum();

        let period_transactions = filtered_transactions.len() as i64;
        let period_redemptions = filtered_redemptions.len() as i64;

        // Tier distribution
        let bronze_users = user_rewards.values().filter(|r| r.current_tier == UserRewardTier::Bronze).count() as i64;
        let silver_users = user_rewards.values().filter(|r| r.current_tier == UserRewardTier::Silver).count() as i64;
        let gold_users = user_rewards.values().filter(|r| r.current_tier == UserRewardTier::Gold).count() as i64;
        let platinum_users = user_rewards.values().filter(|r| r.current_tier == UserRewardTier::Platinum).count() as i64;

        let total_users = user_rewards.len() as i64;
        let active_users = user_rewards
            .values()
            .filter(|r| r.last_activity_date >= start_date)
            .count() as i64;

        // Top categories (simplified for in-memory implementation)
        let top_categories = vec![
            CategoryMetrics {
                category: "grocery".to_string(),
                points_awarded: Decimal::from(1000),
                transaction_count: 50,
                average_points: Decimal::from(20),
            },
            CategoryMetrics {
                category: "restaurant".to_string(),
                points_awarded: Decimal::from(800),
                transaction_count: 40,
                average_points: Decimal::from(20),
            },
        ];

        // Top redemptions
        let top_redemptions = vec![
            RedemptionMetrics {
                redemption_type: RedemptionType::Cash,
                points_redeemed: Decimal::from(5000),
                redemption_count: 25,
                average_points: Decimal::from(200),
            },
            RedemptionMetrics {
                redemption_type: RedemptionType::GiftCard,
                points_redeemed: Decimal::from(3000),
                redemption_count: 15,
                average_points: Decimal::from(200),
            },
        ];

        Ok(RewardMetrics {
            total_points_awarded,
            total_points_redeemed,
            total_points_expired,
            total_cash_value,
            total_users,
            active_users,
            period_start: start_date,
            period_end: end_date,
            period_points_awarded,
            period_points_redeemed,
            period_transactions,
            period_redemptions,
            bronze_users,
            silver_users,
            gold_users,
            platinum_users,
            top_categories,
            top_redemptions,
            generated_at: Utc::now(),
            metadata: HashMap::new(),
        })
    }

    async fn get_user_reward_analytics(
        &self,
        user_id: &Uuid,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> Result<(UserRewards, Vec<RewardTransaction>, Vec<CategoryMetrics>), String> {
        let user_rewards = self.user_rewards.read().unwrap();
        let transactions = self.reward_transactions.read().unwrap();

        let rewards = user_rewards
            .get(user_id)
            .cloned()
            .ok_or_else(|| format!("User rewards not found for user {}", user_id))?;

        let recent_transactions: Vec<_> = transactions
            .values()
            .filter(|tx| tx.user_id == *user_id)
            .filter(|tx| tx.created_at >= start_date && tx.created_at <= end_date)
            .cloned()
            .collect();

        // Simplified category breakdown for in-memory implementation
        let category_breakdown = vec![
            CategoryMetrics {
                category: "grocery".to_string(),
                points_awarded: Decimal::from(200),
                transaction_count: 10,
                average_points: Decimal::from(20),
            },
            CategoryMetrics {
                category: "restaurant".to_string(),
                points_awarded: Decimal::from(150),
                transaction_count: 8,
                average_points: Decimal::from(18),
            },
        ];

        Ok((rewards, recent_transactions, category_breakdown))
    }

    // Audit operations
    async fn create_audit_entry(&self, entry: &RewardAuditTrailEntry) -> Result<RewardAuditTrailEntry, String> {
        let mut audit_trail = self.audit_trail.write().unwrap();
        audit_trail.push(entry.clone());
        Ok(entry.clone())
    }

    async fn get_audit_trail(
        &self,
        user_id: Option<Uuid>,
        start_date: Option<DateTime<Utc>>,
        end_date: Option<DateTime<Utc>>,
        action_types: Vec<String>,
        page: i32,
        page_size: i32,
    ) -> Result<(Vec<RewardAuditTrailEntry>, i64), String> {
        let audit_trail = self.audit_trail.read().unwrap();

        let filtered_entries: Vec<_> = audit_trail
            .iter()
            .filter(|entry| user_id.map_or(true, |id| entry.user_id == Some(id)))
            .filter(|entry| start_date.map_or(true, |d| entry.created_at >= d))
            .filter(|entry| end_date.map_or(true, |d| entry.created_at <= d))
            .filter(|entry| action_types.is_empty() || action_types.contains(&entry.action_type))
            .cloned()
            .collect();

        let total_count = filtered_entries.len() as i64;
        let start = (page * page_size) as usize;
        let end = std::cmp::min(start + page_size as usize, filtered_entries.len());

        let paginated_entries = if start < filtered_entries.len() {
            filtered_entries[start..end].to_vec()
        } else {
            Vec::new()
        };

        Ok((paginated_entries, total_count))
    }
}
