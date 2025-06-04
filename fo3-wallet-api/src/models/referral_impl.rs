//! In-memory implementation of ReferralRepository

use super::referral::*;
use std::collections::HashMap;
use uuid::Uuid;
use rust_decimal::Decimal;
use chrono::{DateTime, Utc};

#[async_trait::async_trait]
impl ReferralRepository for InMemoryReferralRepository {
    // Referral code operations
    async fn create_referral_code(&self, code: &ReferralCode) -> Result<ReferralCode, String> {
        let mut codes = self.referral_codes.write().unwrap();
        let mut code_lookup = self.code_lookup.write().unwrap();
        
        // Check for duplicate code
        if code_lookup.contains_key(&code.code) {
            return Err(format!("Referral code '{}' already exists", code.code));
        }
        
        codes.insert(code.id, code.clone());
        code_lookup.insert(code.code.clone(), code.id);
        Ok(code.clone())
    }

    async fn get_referral_code(&self, id: &Uuid) -> Result<Option<ReferralCode>, String> {
        let codes = self.referral_codes.read().unwrap();
        Ok(codes.get(id).cloned())
    }

    async fn get_referral_code_by_code(&self, code: &str) -> Result<Option<ReferralCode>, String> {
        let code_lookup = self.code_lookup.read().unwrap();
        let codes = self.referral_codes.read().unwrap();
        
        if let Some(id) = code_lookup.get(code) {
            Ok(codes.get(id).cloned())
        } else {
            Ok(None)
        }
    }

    async fn list_user_referral_codes(
        &self,
        user_id: &Uuid,
        status: Option<ReferralCodeStatus>,
        campaign_id: Option<Uuid>,
        page: i32,
        page_size: i32,
    ) -> Result<(Vec<ReferralCode>, i64), String> {
        let codes = self.referral_codes.read().unwrap();
        
        let filtered_codes: Vec<_> = codes
            .values()
            .filter(|code| code.user_id == *user_id)
            .filter(|code| status.as_ref().map_or(true, |s| code.status == *s))
            .filter(|code| campaign_id.map_or(true, |id| code.campaign_id == Some(id)))
            .cloned()
            .collect();

        let total_count = filtered_codes.len() as i64;
        let start = (page * page_size) as usize;
        let end = std::cmp::min(start + page_size as usize, filtered_codes.len());
        
        let paginated_codes = if start < filtered_codes.len() {
            filtered_codes[start..end].to_vec()
        } else {
            Vec::new()
        };

        Ok((paginated_codes, total_count))
    }

    async fn update_referral_code(&self, code: &ReferralCode) -> Result<ReferralCode, String> {
        let mut codes = self.referral_codes.write().unwrap();
        codes.insert(code.id, code.clone());
        Ok(code.clone())
    }

    async fn deactivate_referral_code(&self, id: &Uuid, reason: String) -> Result<ReferralCode, String> {
        let mut codes = self.referral_codes.write().unwrap();
        
        if let Some(mut code) = codes.get(id).cloned() {
            code.status = ReferralCodeStatus::Inactive;
            code.updated_at = Utc::now();
            codes.insert(*id, code.clone());
            
            // Create audit entry
            let audit_entry = ReferralAuditTrailEntry {
                id: Uuid::new_v4(),
                user_id: Some(code.user_id),
                relationship_id: None,
                action_type: "deactivate".to_string(),
                entity_type: "referral_code".to_string(),
                entity_id: code.id,
                old_value: Some("active".to_string()),
                new_value: Some("inactive".to_string()),
                reason: Some(reason),
                performed_by: None,
                ip_address: None,
                user_agent: None,
                metadata: HashMap::new(),
                created_at: Utc::now(),
            };
            
            let mut audit_trail = self.audit_trail.write().unwrap();
            audit_trail.push(audit_entry);
            
            Ok(code)
        } else {
            Err(format!("Referral code not found: {}", id))
        }
    }

    // Referral campaign operations
    async fn create_referral_campaign(&self, campaign: &ReferralCampaign) -> Result<ReferralCampaign, String> {
        let mut campaigns = self.referral_campaigns.write().unwrap();
        campaigns.insert(campaign.id, campaign.clone());
        Ok(campaign.clone())
    }

    async fn get_referral_campaign(&self, id: &Uuid) -> Result<Option<ReferralCampaign>, String> {
        let campaigns = self.referral_campaigns.read().unwrap();
        Ok(campaigns.get(id).cloned())
    }

    async fn list_referral_campaigns(
        &self,
        campaign_type: Option<ReferralCampaignType>,
        status: Option<ReferralCampaignStatus>,
        active_only: bool,
        start_date: Option<DateTime<Utc>>,
        end_date: Option<DateTime<Utc>>,
        page: i32,
        page_size: i32,
    ) -> Result<(Vec<ReferralCampaign>, i64), String> {
        let campaigns = self.referral_campaigns.read().unwrap();
        
        let filtered_campaigns: Vec<_> = campaigns
            .values()
            .filter(|campaign| campaign_type.as_ref().map_or(true, |t| campaign.campaign_type == *t))
            .filter(|campaign| status.as_ref().map_or(true, |s| campaign.status == *s))
            .filter(|campaign| !active_only || campaign.is_active())
            .filter(|campaign| start_date.map_or(true, |d| campaign.created_at >= d))
            .filter(|campaign| end_date.map_or(true, |d| campaign.created_at <= d))
            .cloned()
            .collect();

        let total_count = filtered_campaigns.len() as i64;
        let start = (page * page_size) as usize;
        let end = std::cmp::min(start + page_size as usize, filtered_campaigns.len());
        
        let paginated_campaigns = if start < filtered_campaigns.len() {
            filtered_campaigns[start..end].to_vec()
        } else {
            Vec::new()
        };

        Ok((paginated_campaigns, total_count))
    }

    async fn update_referral_campaign(&self, campaign: &ReferralCampaign) -> Result<ReferralCampaign, String> {
        let mut campaigns = self.referral_campaigns.write().unwrap();
        campaigns.insert(campaign.id, campaign.clone());
        Ok(campaign.clone())
    }

    async fn delete_referral_campaign(&self, id: &Uuid) -> Result<(), String> {
        let mut campaigns = self.referral_campaigns.write().unwrap();
        campaigns.remove(id);
        Ok(())
    }

    // Referral relationship operations
    async fn create_referral_relationship(&self, relationship: &ReferralRelationship) -> Result<ReferralRelationship, String> {
        let mut relationships = self.referral_relationships.write().unwrap();
        
        // Check for duplicate relationship
        let duplicate = relationships.values().any(|r| 
            r.referrer_user_id == relationship.referrer_user_id && 
            r.referee_user_id == relationship.referee_user_id
        );
        
        if duplicate {
            return Err("Referral relationship already exists between these users".to_string());
        }
        
        // Check for self-referral
        if relationship.referrer_user_id == relationship.referee_user_id {
            return Err("Users cannot refer themselves".to_string());
        }
        
        relationships.insert(relationship.id, relationship.clone());
        Ok(relationship.clone())
    }

    async fn get_referral_relationship(&self, id: &Uuid) -> Result<Option<ReferralRelationship>, String> {
        let relationships = self.referral_relationships.read().unwrap();
        Ok(relationships.get(id).cloned())
    }

    async fn list_referral_relationships(
        &self,
        user_id: Option<Uuid>,
        referrer_user_id: Option<Uuid>,
        referee_user_id: Option<Uuid>,
        status: Option<ReferralRelationshipStatus>,
        campaign_id: Option<Uuid>,
        start_date: Option<DateTime<Utc>>,
        end_date: Option<DateTime<Utc>>,
        page: i32,
        page_size: i32,
    ) -> Result<(Vec<ReferralRelationship>, i64), String> {
        let relationships = self.referral_relationships.read().unwrap();
        
        let filtered_relationships: Vec<_> = relationships
            .values()
            .filter(|rel| user_id.map_or(true, |id| rel.referrer_user_id == id || rel.referee_user_id == id))
            .filter(|rel| referrer_user_id.map_or(true, |id| rel.referrer_user_id == id))
            .filter(|rel| referee_user_id.map_or(true, |id| rel.referee_user_id == id))
            .filter(|rel| status.as_ref().map_or(true, |s| rel.status == *s))
            .filter(|rel| campaign_id.map_or(true, |id| rel.campaign_id == Some(id)))
            .filter(|rel| start_date.map_or(true, |d| rel.created_at >= d))
            .filter(|rel| end_date.map_or(true, |d| rel.created_at <= d))
            .cloned()
            .collect();

        let total_count = filtered_relationships.len() as i64;
        let start = (page * page_size) as usize;
        let end = std::cmp::min(start + page_size as usize, filtered_relationships.len());
        
        let paginated_relationships = if start < filtered_relationships.len() {
            filtered_relationships[start..end].to_vec()
        } else {
            Vec::new()
        };

        Ok((paginated_relationships, total_count))
    }

    async fn update_referral_relationship(&self, relationship: &ReferralRelationship) -> Result<ReferralRelationship, String> {
        let mut relationships = self.referral_relationships.write().unwrap();
        relationships.insert(relationship.id, relationship.clone());
        Ok(relationship.clone())
    }

    async fn get_referral_tree(&self, user_id: &Uuid, max_depth: i32, include_inactive: bool) -> Result<ReferralTreeNode, String> {
        let relationships = self.referral_relationships.read().unwrap();
        
        // Build tree recursively
        fn build_tree_node(
            user_id: &Uuid,
            level: i32,
            max_depth: i32,
            include_inactive: bool,
            relationships: &HashMap<Uuid, ReferralRelationship>,
        ) -> ReferralTreeNode {
            let direct_referrals: Vec<_> = relationships
                .values()
                .filter(|rel| rel.referrer_user_id == *user_id)
                .filter(|rel| include_inactive || rel.status == ReferralRelationshipStatus::Active)
                .collect();
            
            let mut children = Vec::new();
            let mut total_referrals = direct_referrals.len() as i64;
            
            if level < max_depth {
                for rel in &direct_referrals {
                    let child = build_tree_node(
                        &rel.referee_user_id,
                        level + 1,
                        max_depth,
                        include_inactive,
                        relationships,
                    );
                    total_referrals += child.total_referrals;
                    children.push(child);
                }
            }
            
            ReferralTreeNode {
                user_id: *user_id,
                username: None, // Would be populated from user service
                level,
                direct_referrals: direct_referrals.len() as i64,
                total_referrals,
                total_bonuses_earned: direct_referrals.iter()
                    .map(|rel| rel.total_bonuses_earned)
                    .sum(),
                children,
                joined_at: Utc::now(), // Would be actual join date
                is_active: true, // Would be actual status
            }
        }
        
        Ok(build_tree_node(user_id, 1, max_depth, include_inactive, &relationships))
    }

    async fn get_referral_stats(
        &self,
        user_id: &Uuid,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> Result<(i64, i64, i64, Decimal, Decimal, Decimal, Vec<CampaignMetrics>), String> {
        let relationships = self.referral_relationships.read().unwrap();
        let bonuses = self.referral_bonuses.read().unwrap();
        
        let user_relationships: Vec<_> = relationships
            .values()
            .filter(|rel| rel.referrer_user_id == *user_id)
            .filter(|rel| rel.created_at >= start_date && rel.created_at <= end_date)
            .collect();
        
        let total_referrals = user_relationships.len() as i64;
        let successful_referrals = user_relationships
            .iter()
            .filter(|rel| rel.status == ReferralRelationshipStatus::Completed)
            .count() as i64;
        let pending_referrals = user_relationships
            .iter()
            .filter(|rel| rel.status == ReferralRelationshipStatus::Pending)
            .count() as i64;
        
        let user_bonuses: Vec<_> = bonuses
            .values()
            .filter(|bonus| bonus.user_id == *user_id)
            .filter(|bonus| bonus.created_at >= start_date && bonus.created_at <= end_date)
            .collect();
        
        let total_bonuses_earned: Decimal = user_bonuses
            .iter()
            .filter(|bonus| bonus.status == ReferralBonusStatus::Completed)
            .map(|bonus| bonus.bonus_amount)
            .sum();
        
        let total_bonuses_pending: Decimal = user_bonuses
            .iter()
            .filter(|bonus| bonus.status == ReferralBonusStatus::Pending)
            .map(|bonus| bonus.bonus_amount)
            .sum();
        
        let conversion_rate = if total_referrals > 0 {
            Decimal::from(successful_referrals) / Decimal::from(total_referrals)
        } else {
            Decimal::ZERO
        };
        
        // Simplified campaign metrics for in-memory implementation
        let campaign_stats = vec![
            CampaignMetrics {
                campaign_id: Uuid::new_v4(),
                campaign_name: "Default Campaign".to_string(),
                total_referrals,
                successful_referrals,
                total_bonuses_paid: total_bonuses_earned,
                budget_utilization: Decimal::from_str_exact("0.5").unwrap(),
                roi: Decimal::from_str_exact("1.2").unwrap(),
            }
        ];
        
        Ok((
            total_referrals,
            successful_referrals,
            pending_referrals,
            total_bonuses_earned,
            total_bonuses_pending,
            conversion_rate,
            campaign_stats,
        ))
    }

    // Referral bonus operations
    async fn create_referral_bonus(&self, bonus: &ReferralBonus) -> Result<ReferralBonus, String> {
        let mut bonuses = self.referral_bonuses.write().unwrap();
        bonuses.insert(bonus.id, bonus.clone());
        Ok(bonus.clone())
    }

    async fn get_referral_bonus(&self, id: &Uuid) -> Result<Option<ReferralBonus>, String> {
        let bonuses = self.referral_bonuses.read().unwrap();
        Ok(bonuses.get(id).cloned())
    }

    async fn list_referral_bonuses(
        &self,
        user_id: Option<Uuid>,
        relationship_id: Option<Uuid>,
        campaign_id: Option<Uuid>,
        status: Option<ReferralBonusStatus>,
        bonus_type: Option<ReferralBonusType>,
        start_date: Option<DateTime<Utc>>,
        end_date: Option<DateTime<Utc>>,
        page: i32,
        page_size: i32,
    ) -> Result<(Vec<ReferralBonus>, i64), String> {
        let bonuses = self.referral_bonuses.read().unwrap();

        let filtered_bonuses: Vec<_> = bonuses
            .values()
            .filter(|bonus| user_id.map_or(true, |id| bonus.user_id == id))
            .filter(|bonus| relationship_id.map_or(true, |id| bonus.referral_relationship_id == id))
            .filter(|bonus| campaign_id.map_or(true, |id| bonus.campaign_id == Some(id)))
            .filter(|bonus| status.as_ref().map_or(true, |s| bonus.status == *s))
            .filter(|bonus| bonus_type.as_ref().map_or(true, |t| bonus.bonus_type == *t))
            .filter(|bonus| start_date.map_or(true, |d| bonus.created_at >= d))
            .filter(|bonus| end_date.map_or(true, |d| bonus.created_at <= d))
            .cloned()
            .collect();

        let total_count = filtered_bonuses.len() as i64;
        let start = (page * page_size) as usize;
        let end = std::cmp::min(start + page_size as usize, filtered_bonuses.len());

        let paginated_bonuses = if start < filtered_bonuses.len() {
            filtered_bonuses[start..end].to_vec()
        } else {
            Vec::new()
        };

        Ok((paginated_bonuses, total_count))
    }

    async fn update_referral_bonus(&self, bonus: &ReferralBonus) -> Result<ReferralBonus, String> {
        let mut bonuses = self.referral_bonuses.write().unwrap();
        bonuses.insert(bonus.id, bonus.clone());
        Ok(bonus.clone())
    }

    async fn process_referral_bonuses(
        &self,
        relationship_id: &Uuid,
        milestone_type: &str,
        milestone_value: Option<Decimal>,
        force_processing: bool,
    ) -> Result<Vec<ReferralBonus>, String> {
        let relationships = self.referral_relationships.read().unwrap();
        let campaigns = self.referral_campaigns.read().unwrap();
        let mut bonuses = self.referral_bonuses.write().unwrap();

        let relationship = relationships.get(relationship_id)
            .ok_or_else(|| "Referral relationship not found".to_string())?;

        if !force_processing && !relationship.is_eligible_for_bonus(milestone_type) {
            return Err("Relationship not eligible for bonus".to_string());
        }

        let mut created_bonuses = Vec::new();

        // Get campaign if available
        if let Some(campaign_id) = relationship.campaign_id {
            if let Some(campaign) = campaigns.get(&campaign_id) {
                if campaign.is_active() {
                    // Create referrer bonus
                    let referrer_bonus = ReferralBonus {
                        id: Uuid::new_v4(),
                        referral_relationship_id: *relationship_id,
                        campaign_id: Some(campaign_id),
                        user_id: relationship.referrer_user_id,
                        bonus_type: ReferralBonusType::Referrer,
                        status: ReferralBonusStatus::Pending,
                        bonus_amount: campaign.referrer_bonus,
                        bonus_currency: campaign.bonus_currency.clone(),
                        exchange_rate: Decimal::ONE,
                        milestone_type: Some(milestone_type.to_string()),
                        milestone_value,
                        reward_transaction_id: None,
                        processing_fee: Decimal::ZERO,
                        net_amount: campaign.referrer_bonus,
                        earned_at: Utc::now(),
                        processed_at: None,
                        expires_at: Some(Utc::now() + chrono::Duration::days(campaign.bonus_expiry_days as i64)),
                        description: Some(format!("Referrer bonus for {} milestone", milestone_type)),
                        metadata: HashMap::new(),
                        created_at: Utc::now(),
                        updated_at: Utc::now(),
                    };

                    bonuses.insert(referrer_bonus.id, referrer_bonus.clone());
                    created_bonuses.push(referrer_bonus);

                    // Create referee bonus
                    let referee_bonus = ReferralBonus {
                        id: Uuid::new_v4(),
                        referral_relationship_id: *relationship_id,
                        campaign_id: Some(campaign_id),
                        user_id: relationship.referee_user_id,
                        bonus_type: ReferralBonusType::Referee,
                        status: ReferralBonusStatus::Pending,
                        bonus_amount: campaign.referee_bonus,
                        bonus_currency: campaign.bonus_currency.clone(),
                        exchange_rate: Decimal::ONE,
                        milestone_type: Some(milestone_type.to_string()),
                        milestone_value,
                        reward_transaction_id: None,
                        processing_fee: Decimal::ZERO,
                        net_amount: campaign.referee_bonus,
                        earned_at: Utc::now(),
                        processed_at: None,
                        expires_at: Some(Utc::now() + chrono::Duration::days(campaign.bonus_expiry_days as i64)),
                        description: Some(format!("Referee bonus for {} milestone", milestone_type)),
                        metadata: HashMap::new(),
                        created_at: Utc::now(),
                        updated_at: Utc::now(),
                    };

                    bonuses.insert(referee_bonus.id, referee_bonus.clone());
                    created_bonuses.push(referee_bonus);
                }
            }
        }

        Ok(created_bonuses)
    }

    // Analytics operations
    async fn get_referral_metrics(
        &self,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
        campaign_ids: Vec<Uuid>,
        user_ids: Vec<Uuid>,
        include_fraud_metrics: bool,
    ) -> Result<ReferralMetrics, String> {
        let codes = self.referral_codes.read().unwrap();
        let relationships = self.referral_relationships.read().unwrap();
        let bonuses = self.referral_bonuses.read().unwrap();

        // Filter data by date range and criteria
        let filtered_relationships: Vec<_> = relationships
            .values()
            .filter(|rel| rel.created_at >= start_date && rel.created_at <= end_date)
            .filter(|rel| campaign_ids.is_empty() || campaign_ids.contains(&rel.campaign_id.unwrap_or_default()))
            .filter(|rel| user_ids.is_empty() || user_ids.contains(&rel.referrer_user_id) || user_ids.contains(&rel.referee_user_id))
            .collect();

        let filtered_bonuses: Vec<_> = bonuses
            .values()
            .filter(|bonus| bonus.created_at >= start_date && bonus.created_at <= end_date)
            .filter(|bonus| campaign_ids.is_empty() || campaign_ids.contains(&bonus.campaign_id.unwrap_or_default()))
            .filter(|bonus| user_ids.is_empty() || user_ids.contains(&bonus.user_id))
            .collect();

        // Calculate metrics
        let total_referral_codes = codes.len() as i64;
        let active_referral_codes = codes.values()
            .filter(|code| code.status == ReferralCodeStatus::Active)
            .count() as i64;

        let total_referrals = filtered_relationships.len() as i64;
        let successful_referrals = filtered_relationships
            .iter()
            .filter(|rel| rel.status == ReferralRelationshipStatus::Completed)
            .count() as i64;
        let pending_referrals = filtered_relationships
            .iter()
            .filter(|rel| rel.status == ReferralRelationshipStatus::Pending)
            .count() as i64;

        let total_bonuses_paid: Decimal = filtered_bonuses
            .iter()
            .filter(|bonus| bonus.status == ReferralBonusStatus::Completed)
            .map(|bonus| bonus.bonus_amount)
            .sum();

        let total_bonuses_pending: Decimal = filtered_bonuses
            .iter()
            .filter(|bonus| bonus.status == ReferralBonusStatus::Pending)
            .map(|bonus| bonus.bonus_amount)
            .sum();

        let signup_conversion_rate = if total_referrals > 0 {
            Decimal::from(successful_referrals) / Decimal::from(total_referrals)
        } else {
            Decimal::ZERO
        };

        let transaction_conversion_rate = if total_referrals > 0 {
            let tx_completed = filtered_relationships
                .iter()
                .filter(|rel| rel.first_transaction_completed)
                .count() as i64;
            Decimal::from(tx_completed) / Decimal::from(total_referrals)
        } else {
            Decimal::ZERO
        };

        let average_bonus_per_referral = if total_referrals > 0 {
            total_bonuses_paid / Decimal::from(total_referrals)
        } else {
            Decimal::ZERO
        };

        // Simplified metrics for in-memory implementation
        let top_referrers = vec![
            TopReferrer {
                user_id: Uuid::new_v4(),
                username: Some("top_referrer_1".to_string()),
                total_referrals: 10,
                successful_referrals: 8,
                total_bonuses_earned: Decimal::from(1000),
                conversion_rate: Decimal::from_str_exact("0.8").unwrap(),
            }
        ];

        let top_campaigns = vec![
            CampaignMetrics {
                campaign_id: Uuid::new_v4(),
                campaign_name: "Signup Campaign".to_string(),
                total_referrals,
                successful_referrals,
                total_bonuses_paid,
                budget_utilization: Decimal::from_str_exact("0.6").unwrap(),
                roi: Decimal::from_str_exact("1.5").unwrap(),
            }
        ];

        let (flagged_relationships, cancelled_relationships, fraud_rate) = if include_fraud_metrics {
            let flagged = filtered_relationships
                .iter()
                .filter(|rel| rel.is_suspicious)
                .count() as i64;
            let cancelled = filtered_relationships
                .iter()
                .filter(|rel| rel.status == ReferralRelationshipStatus::Cancelled)
                .count() as i64;
            let fraud_rate = if total_referrals > 0 {
                Decimal::from(flagged + cancelled) / Decimal::from(total_referrals)
            } else {
                Decimal::ZERO
            };
            (flagged, cancelled, fraud_rate)
        } else {
            (0, 0, Decimal::ZERO)
        };

        Ok(ReferralMetrics {
            total_referral_codes,
            active_referral_codes,
            total_referrals,
            successful_referrals,
            pending_referrals,
            total_bonuses_paid,
            total_bonuses_pending,
            period_start: start_date,
            period_end: end_date,
            period_referrals: total_referrals,
            period_signups: successful_referrals,
            period_bonuses_paid: total_bonuses_paid,
            signup_conversion_rate,
            transaction_conversion_rate,
            average_bonus_per_referral,
            roi: Decimal::from_str_exact("1.3").unwrap(),
            top_referrers,
            top_campaigns,
            flagged_relationships,
            cancelled_relationships,
            fraud_rate,
            generated_at: Utc::now(),
            metadata: HashMap::new(),
        })
    }

    async fn get_user_referral_analytics(
        &self,
        user_id: &Uuid,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> Result<(i64, i64, i64, Decimal, Decimal, Decimal, Vec<CampaignMetrics>, Vec<ReferralBonus>), String> {
        let (total_referrals, successful_referrals, pending_referrals, total_bonuses_earned, total_bonuses_pending, conversion_rate, campaign_stats) =
            self.get_referral_stats(user_id, start_date, end_date).await?;

        let bonuses = self.referral_bonuses.read().unwrap();
        let recent_bonuses: Vec<_> = bonuses
            .values()
            .filter(|bonus| bonus.user_id == *user_id)
            .filter(|bonus| bonus.created_at >= start_date && bonus.created_at <= end_date)
            .cloned()
            .collect();

        Ok((
            total_referrals,
            successful_referrals,
            pending_referrals,
            total_bonuses_earned,
            total_bonuses_pending,
            conversion_rate,
            campaign_stats,
            recent_bonuses,
        ))
    }

    // Administrative operations
    async fn flag_suspicious_activity(
        &self,
        relationship_id: &Uuid,
        fraud_flags: Vec<String>,
        reason: String,
        auto_suspend: bool,
    ) -> Result<ReferralRelationship, String> {
        let mut relationships = self.referral_relationships.write().unwrap();

        if let Some(mut relationship) = relationships.get(relationship_id).cloned() {
            relationship.is_suspicious = true;
            relationship.fraud_flags = fraud_flags.clone();
            relationship.fraud_check_date = Some(Utc::now());
            relationship.updated_at = Utc::now();

            if auto_suspend {
                relationship.status = ReferralRelationshipStatus::Fraudulent;
            }

            relationships.insert(*relationship_id, relationship.clone());

            // Create audit entry
            let audit_entry = ReferralAuditTrailEntry {
                id: Uuid::new_v4(),
                user_id: Some(relationship.referrer_user_id),
                relationship_id: Some(*relationship_id),
                action_type: "flag_suspicious".to_string(),
                entity_type: "referral_relationship".to_string(),
                entity_id: *relationship_id,
                old_value: Some("clean".to_string()),
                new_value: Some(format!("flagged: {:?}", fraud_flags)),
                reason: Some(reason),
                performed_by: None,
                ip_address: None,
                user_agent: None,
                metadata: HashMap::new(),
                created_at: Utc::now(),
            };

            let mut audit_trail = self.audit_trail.write().unwrap();
            audit_trail.push(audit_entry);

            Ok(relationship)
        } else {
            Err(format!("Referral relationship not found: {}", relationship_id))
        }
    }

    async fn bulk_process_bonuses(
        &self,
        relationship_ids: Vec<Uuid>,
        milestone_type: String,
        batch_id: String,
        reason: String,
    ) -> Result<(Vec<ReferralBonus>, i64, i64, Vec<String>), String> {
        let mut all_bonuses = Vec::new();
        let mut successful_bonuses = 0i64;
        let mut failed_bonuses = 0i64;
        let mut error_messages = Vec::new();

        for relationship_id in relationship_ids {
            match self.process_referral_bonuses(&relationship_id, &milestone_type, None, false).await {
                Ok(bonuses) => {
                    successful_bonuses += bonuses.len() as i64;
                    all_bonuses.extend(bonuses);
                },
                Err(e) => {
                    failed_bonuses += 1;
                    error_messages.push(format!("Relationship {}: {}", relationship_id, e));
                }
            }
        }

        // Create audit entry for bulk operation
        let audit_entry = ReferralAuditTrailEntry {
            id: Uuid::new_v4(),
            user_id: None,
            relationship_id: None,
            action_type: "bulk_process_bonuses".to_string(),
            entity_type: "referral_bonus".to_string(),
            entity_id: Uuid::new_v4(), // Batch ID as UUID
            old_value: None,
            new_value: Some(format!("Processed {} relationships", relationship_ids.len())),
            reason: Some(reason),
            performed_by: None,
            ip_address: None,
            user_agent: None,
            metadata: [("batch_id".to_string(), batch_id.clone())].iter().cloned().collect(),
            created_at: Utc::now(),
        };

        let mut audit_trail = self.audit_trail.write().unwrap();
        audit_trail.push(audit_entry);

        Ok((all_bonuses, successful_bonuses, failed_bonuses, error_messages))
    }

    // Audit operations
    async fn create_audit_entry(&self, entry: &ReferralAuditTrailEntry) -> Result<ReferralAuditTrailEntry, String> {
        let mut audit_trail = self.audit_trail.write().unwrap();
        audit_trail.push(entry.clone());
        Ok(entry.clone())
    }

    async fn get_audit_trail(
        &self,
        user_id: Option<Uuid>,
        relationship_id: Option<Uuid>,
        start_date: Option<DateTime<Utc>>,
        end_date: Option<DateTime<Utc>>,
        action_types: Vec<String>,
        page: i32,
        page_size: i32,
    ) -> Result<(Vec<ReferralAuditTrailEntry>, i64), String> {
        let audit_trail = self.audit_trail.read().unwrap();

        let filtered_entries: Vec<_> = audit_trail
            .iter()
            .filter(|entry| user_id.map_or(true, |id| entry.user_id == Some(id)))
            .filter(|entry| relationship_id.map_or(true, |id| entry.relationship_id == Some(id)))
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
