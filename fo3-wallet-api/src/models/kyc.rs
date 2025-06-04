//! KYC data models and database entities

use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc, NaiveDate};
use uuid::Uuid;

/// KYC submission status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KycStatus {
    Pending,
    UnderReview,
    Approved,
    Rejected,
    RequiresUpdate,
}

impl From<KycStatus> for String {
    fn from(status: KycStatus) -> Self {
        match status {
            KycStatus::Pending => "pending".to_string(),
            KycStatus::UnderReview => "under_review".to_string(),
            KycStatus::Approved => "approved".to_string(),
            KycStatus::Rejected => "rejected".to_string(),
            KycStatus::RequiresUpdate => "requires_update".to_string(),
        }
    }
}

impl TryFrom<String> for KycStatus {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "pending" => Ok(KycStatus::Pending),
            "under_review" => Ok(KycStatus::UnderReview),
            "approved" => Ok(KycStatus::Approved),
            "rejected" => Ok(KycStatus::Rejected),
            "requires_update" => Ok(KycStatus::RequiresUpdate),
            _ => Err(format!("Invalid KYC status: {}", value)),
        }
    }
}

impl KycStatus {
    /// Create KycStatus from string
    pub fn from_string(value: &str) -> Self {
        match value {
            "pending" => KycStatus::Pending,
            "under_review" => KycStatus::UnderReview,
            "approved" => KycStatus::Approved,
            "rejected" => KycStatus::Rejected,
            "requires_update" => KycStatus::RequiresUpdate,
            _ => KycStatus::Pending, // Default to pending for unknown values
        }
    }

    /// Convert to string
    pub fn to_string(&self) -> String {
        String::from(*self)
    }
}

/// Document type for KYC verification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DocumentType {
    GovernmentId,
    ProofOfAddress,
    Selfie,
    BankStatement,
    Other,
}

impl From<DocumentType> for String {
    fn from(doc_type: DocumentType) -> Self {
        match doc_type {
            DocumentType::GovernmentId => "government_id".to_string(),
            DocumentType::ProofOfAddress => "proof_of_address".to_string(),
            DocumentType::Selfie => "selfie".to_string(),
            DocumentType::BankStatement => "bank_statement".to_string(),
            DocumentType::Other => "other".to_string(),
        }
    }
}

impl TryFrom<String> for DocumentType {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "government_id" => Ok(DocumentType::GovernmentId),
            "proof_of_address" => Ok(DocumentType::ProofOfAddress),
            "selfie" => Ok(DocumentType::Selfie),
            "bank_statement" => Ok(DocumentType::BankStatement),
            "other" => Ok(DocumentType::Other),
            _ => Err(format!("Invalid document type: {}", value)),
        }
    }
}

/// Personal information for KYC
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalInfo {
    pub first_name: String,
    pub last_name: String,
    pub date_of_birth: NaiveDate,
    pub nationality: String,
    pub country_of_residence: String,
    pub address: Address,
}

/// Address information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Address {
    pub street_address: String,
    pub city: String,
    pub state_province: Option<String>,
    pub postal_code: String,
    pub country: String,
}

/// KYC document metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: Uuid,
    pub submission_id: Uuid,
    pub document_type: DocumentType,
    pub filename: String,
    pub content_type: String,
    pub size_bytes: i64,
    pub file_hash: String,
    pub storage_path: String,
    pub is_encrypted: bool,
    pub uploaded_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

/// KYC submission entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KycSubmission {
    pub id: Uuid,
    pub wallet_id: Uuid,
    pub status: KycStatus,
    pub personal_info: PersonalInfo,
    pub documents: Vec<Document>,
    pub submitted_at: DateTime<Utc>,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub reviewer_id: Option<String>,
    pub reviewer_notes: Option<String>,
    pub rejection_reason: Option<String>,
    pub updated_at: DateTime<Utc>,
}

impl KycSubmission {
    /// Create a new KYC submission
    pub fn new(wallet_id: Uuid, personal_info: PersonalInfo) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            wallet_id,
            status: KycStatus::Pending,
            personal_info,
            documents: Vec::new(),
            submitted_at: now,
            reviewed_at: None,
            reviewer_id: None,
            reviewer_notes: None,
            rejection_reason: None,
            updated_at: now,
        }
    }

    /// Add a document to the submission
    pub fn add_document(&mut self, document: Document) {
        self.documents.push(document);
        self.updated_at = Utc::now();
    }

    /// Remove a document from the submission
    pub fn remove_document(&mut self, document_id: Uuid) -> bool {
        let initial_len = self.documents.len();
        self.documents.retain(|doc| doc.id != document_id);
        let removed = self.documents.len() != initial_len;
        if removed {
            self.updated_at = Utc::now();
        }
        removed
    }

    /// Update status to under review
    pub fn mark_under_review(&mut self) {
        self.status = KycStatus::UnderReview;
        self.updated_at = Utc::now();
    }

    /// Approve the KYC submission
    pub fn approve(&mut self, reviewer_id: String, notes: Option<String>) {
        self.status = KycStatus::Approved;
        self.reviewer_id = Some(reviewer_id);
        self.reviewer_notes = notes;
        self.reviewed_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    /// Reject the KYC submission
    pub fn reject(&mut self, reviewer_id: String, reason: String, notes: Option<String>) {
        self.status = KycStatus::Rejected;
        self.reviewer_id = Some(reviewer_id);
        self.rejection_reason = Some(reason);
        self.reviewer_notes = notes;
        self.reviewed_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    /// Mark as requiring updates
    pub fn require_update(&mut self, reviewer_id: String, reason: String) {
        self.status = KycStatus::RequiresUpdate;
        self.reviewer_id = Some(reviewer_id);
        self.rejection_reason = Some(reason);
        self.reviewed_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    /// Check if the submission is in a final state
    pub fn is_final(&self) -> bool {
        matches!(self.status, KycStatus::Approved | KycStatus::Rejected)
    }

    /// Check if the submission can be updated
    pub fn can_be_updated(&self) -> bool {
        matches!(self.status, KycStatus::Pending | KycStatus::RequiresUpdate)
    }

    /// Get documents by type
    pub fn get_documents_by_type(&self, doc_type: DocumentType) -> Vec<&Document> {
        self.documents.iter()
            .filter(|doc| doc.document_type == doc_type && doc.deleted_at.is_none())
            .collect()
    }

    /// Check if all required documents are present
    pub fn has_required_documents(&self) -> bool {
        let required_types = [DocumentType::GovernmentId, DocumentType::ProofOfAddress];
        required_types.iter().all(|&doc_type| {
            !self.get_documents_by_type(doc_type).is_empty()
        })
    }
}

impl Document {
    /// Create a new document
    pub fn new(
        submission_id: Uuid,
        document_type: DocumentType,
        filename: String,
        content_type: String,
        size_bytes: i64,
        file_hash: String,
        storage_path: String,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            submission_id,
            document_type,
            filename,
            content_type,
            size_bytes,
            file_hash,
            storage_path,
            is_encrypted: true,
            uploaded_at: Utc::now(),
            deleted_at: None,
        }
    }

    /// Mark document as deleted (soft delete)
    pub fn mark_deleted(&mut self) {
        self.deleted_at = Some(Utc::now());
    }

    /// Check if document is deleted
    pub fn is_deleted(&self) -> bool {
        self.deleted_at.is_some()
    }
}

/// KYC repository trait for database operations
pub trait KycRepository {
    type Error;

    /// Create a new KYC submission
    async fn create_submission(&self, submission: &KycSubmission) -> Result<(), Self::Error>;

    /// Get KYC submission by ID
    async fn get_submission_by_id(&self, id: Uuid) -> Result<Option<KycSubmission>, Self::Error>;

    /// Get KYC submission by wallet ID
    async fn get_submission_by_wallet_id(&self, wallet_id: Uuid) -> Result<Option<KycSubmission>, Self::Error>;

    /// Update KYC submission
    async fn update_submission(&self, submission: &KycSubmission) -> Result<(), Self::Error>;

    /// List KYC submissions with pagination
    async fn list_submissions(&self, limit: Option<i32>, offset: Option<i32>) -> Result<Vec<KycSubmission>, Self::Error>;

    /// Delete KYC submission
    async fn delete_submission(&self, id: Uuid) -> Result<(), Self::Error>;

    /// Create a document
    async fn create_document(&self, document: &Document) -> Result<(), Self::Error>;

    /// Get document by ID
    async fn get_document_by_id(&self, id: Uuid) -> Result<Option<Document>, Self::Error>;

    /// Update document
    async fn update_document(&self, document: &Document) -> Result<(), Self::Error>;

    /// Get documents by submission ID
    async fn get_documents_by_submission_id(&self, submission_id: Uuid) -> Result<Vec<Document>, Self::Error>;
}
