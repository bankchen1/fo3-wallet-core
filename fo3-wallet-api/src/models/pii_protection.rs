//! PII (Personally Identifiable Information) protection and data privacy utilities

use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc, Duration};
use uuid::Uuid;
use sha2::{Sha256, Digest};

/// Data retention policy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataRetentionPolicy {
    /// How long to retain KYC data after account closure
    pub kyc_retention_days: i64,
    /// How long to retain audit logs
    pub audit_retention_days: i64,
    /// How long to retain document files
    pub document_retention_days: i64,
    /// Grace period before permanent deletion
    pub deletion_grace_period_days: i64,
}

impl Default for DataRetentionPolicy {
    fn default() -> Self {
        Self {
            kyc_retention_days: 2555, // 7 years for compliance
            audit_retention_days: 2555, // 7 years for compliance
            document_retention_days: 2555, // 7 years for compliance
            deletion_grace_period_days: 30, // 30 days grace period
        }
    }
}

/// PII field classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PiiClassification {
    /// Highly sensitive data (SSN, passport numbers)
    HighlySensitive,
    /// Sensitive data (names, addresses)
    Sensitive,
    /// Internal identifiers (user IDs, wallet IDs)
    Internal,
    /// Public or non-sensitive data
    Public,
}

/// Data subject rights under GDPR/CCPA
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataSubjectRight {
    /// Right to access personal data
    Access,
    /// Right to rectification (correction)
    Rectification,
    /// Right to erasure (deletion)
    Erasure,
    /// Right to restrict processing
    Restriction,
    /// Right to data portability
    Portability,
    /// Right to object to processing
    Objection,
}

/// Data processing request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataProcessingRequest {
    pub id: Uuid,
    pub user_id: String,
    pub request_type: DataSubjectRight,
    pub requested_at: DateTime<Utc>,
    pub processed_at: Option<DateTime<Utc>>,
    pub status: ProcessingStatus,
    pub notes: Option<String>,
    pub processor_id: Option<String>,
}

/// Processing status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProcessingStatus {
    Pending,
    InProgress,
    Completed,
    Rejected,
    RequiresManualReview,
}

/// PII anonymization utilities
pub struct PiiAnonymizer;

impl PiiAnonymizer {
    /// Hash PII data for pseudonymization
    pub fn hash_pii(data: &str, salt: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data.as_bytes());
        hasher.update(salt.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Mask sensitive data for logging
    pub fn mask_sensitive_data(data: &str, classification: PiiClassification) -> String {
        match classification {
            PiiClassification::HighlySensitive => {
                if data.len() <= 4 {
                    "*".repeat(data.len())
                } else {
                    format!("{}****", &data[..2])
                }
            }
            PiiClassification::Sensitive => {
                if data.len() <= 6 {
                    "*".repeat(data.len())
                } else {
                    format!("{}***{}", &data[..2], &data[data.len()-2..])
                }
            }
            PiiClassification::Internal => {
                // Show first 8 characters for UUIDs
                if data.len() > 8 {
                    format!("{}...", &data[..8])
                } else {
                    data.to_string()
                }
            }
            PiiClassification::Public => data.to_string(),
        }
    }

    /// Redact PII from text for audit logs
    pub fn redact_pii_from_text(text: &str) -> String {
        let mut redacted = text.to_string();
        
        // Redact email patterns
        let email_regex = regex::Regex::new(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b").unwrap();
        redacted = email_regex.replace_all(&redacted, "[EMAIL_REDACTED]").to_string();
        
        // Redact phone number patterns
        let phone_regex = regex::Regex::new(r"\b\d{3}[-.]?\d{3}[-.]?\d{4}\b").unwrap();
        redacted = phone_regex.replace_all(&redacted, "[PHONE_REDACTED]").to_string();
        
        // Redact SSN patterns
        let ssn_regex = regex::Regex::new(r"\b\d{3}-\d{2}-\d{4}\b").unwrap();
        redacted = ssn_regex.replace_all(&redacted, "[SSN_REDACTED]").to_string();
        
        redacted
    }
}

/// Data retention manager
pub struct DataRetentionManager {
    policy: DataRetentionPolicy,
}

impl DataRetentionManager {
    pub fn new(policy: DataRetentionPolicy) -> Self {
        Self { policy }
    }

    /// Check if data should be deleted based on retention policy
    pub fn should_delete_kyc_data(&self, created_at: DateTime<Utc>, account_closed_at: Option<DateTime<Utc>>) -> bool {
        let retention_deadline = if let Some(closed_at) = account_closed_at {
            closed_at + Duration::days(self.policy.kyc_retention_days)
        } else {
            // If account is still active, don't delete
            return false;
        };

        Utc::now() > retention_deadline
    }

    /// Check if documents should be deleted
    pub fn should_delete_documents(&self, created_at: DateTime<Utc>, account_closed_at: Option<DateTime<Utc>>) -> bool {
        let retention_deadline = if let Some(closed_at) = account_closed_at {
            closed_at + Duration::days(self.policy.document_retention_days)
        } else {
            return false;
        };

        Utc::now() > retention_deadline
    }

    /// Check if audit logs should be deleted
    pub fn should_delete_audit_logs(&self, created_at: DateTime<Utc>) -> bool {
        let retention_deadline = created_at + Duration::days(self.policy.audit_retention_days);
        Utc::now() > retention_deadline
    }

    /// Get deletion schedule for user data
    pub fn get_deletion_schedule(&self, account_closed_at: DateTime<Utc>) -> HashMap<String, DateTime<Utc>> {
        let mut schedule = HashMap::new();
        
        schedule.insert(
            "kyc_data".to_string(),
            account_closed_at + Duration::days(self.policy.kyc_retention_days)
        );
        
        schedule.insert(
            "documents".to_string(),
            account_closed_at + Duration::days(self.policy.document_retention_days)
        );
        
        schedule.insert(
            "audit_logs".to_string(),
            account_closed_at + Duration::days(self.policy.audit_retention_days)
        );
        
        schedule
    }
}

/// Compliance audit trail
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceAuditEntry {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub user_id: String,
    pub operation: String,
    pub data_type: String,
    pub legal_basis: String,
    pub purpose: String,
    pub retention_period: Option<i64>,
    pub processor_id: String,
    pub metadata: serde_json::Value,
}

impl ComplianceAuditEntry {
    pub fn new(
        user_id: String,
        operation: String,
        data_type: String,
        legal_basis: String,
        purpose: String,
        processor_id: String,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            user_id,
            operation,
            data_type,
            legal_basis,
            purpose,
            retention_period: None,
            processor_id,
            metadata: serde_json::Value::Object(serde_json::Map::new()),
        }
    }

    pub fn with_retention_period(mut self, days: i64) -> Self {
        self.retention_period = Some(days);
        self
    }

    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = metadata;
        self
    }
}

/// GDPR/CCPA compliance utilities
pub struct ComplianceManager {
    retention_manager: DataRetentionManager,
}

impl ComplianceManager {
    pub fn new(retention_policy: DataRetentionPolicy) -> Self {
        Self {
            retention_manager: DataRetentionManager::new(retention_policy),
        }
    }

    /// Create compliance audit entry for KYC operations
    pub fn create_kyc_audit_entry(
        &self,
        user_id: String,
        operation: String,
        processor_id: String,
    ) -> ComplianceAuditEntry {
        ComplianceAuditEntry::new(
            user_id,
            operation,
            "kyc_data".to_string(),
            "Legitimate interest - AML/KYC compliance".to_string(),
            "Identity verification and anti-money laundering compliance".to_string(),
            processor_id,
        )
        .with_retention_period(2555) // 7 years
    }

    /// Create compliance audit entry for document processing
    pub fn create_document_audit_entry(
        &self,
        user_id: String,
        operation: String,
        processor_id: String,
    ) -> ComplianceAuditEntry {
        ComplianceAuditEntry::new(
            user_id,
            operation,
            "identity_documents".to_string(),
            "Legal obligation - AML regulations".to_string(),
            "Identity document verification for regulatory compliance".to_string(),
            processor_id,
        )
        .with_retention_period(2555) // 7 years
    }

    /// Validate data processing request
    pub fn validate_processing_request(&self, request: &DataProcessingRequest) -> Result<(), String> {
        match request.request_type {
            DataSubjectRight::Erasure => {
                // Check if we can legally delete the data
                // Some data must be retained for compliance
                Ok(())
            }
            DataSubjectRight::Access => {
                // Always allowed
                Ok(())
            }
            DataSubjectRight::Rectification => {
                // Allowed with proper verification
                Ok(())
            }
            _ => Ok(()),
        }
    }
}
