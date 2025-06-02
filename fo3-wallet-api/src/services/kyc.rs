//! KYC service implementation

use std::sync::Arc;
use tonic::{Request, Response, Status, Streaming};
use uuid::Uuid;
use chrono::{DateTime, Utc, NaiveDate};

use crate::proto::fo3::wallet::v1::{
    kyc_service_server::KycService,
    *,
};
use crate::state::AppState;
use crate::middleware::{
    auth::{AuthContext, AuthService},
    audit::AuditLogger,
};
use crate::models::kyc::{
    KycSubmission, PersonalInfo, Address, Document, DocumentType, KycStatus
};
use crate::storage::{DocumentStorage, DocumentUploadHandler};

/// KYC service implementation
pub struct KycServiceImpl {
    state: Arc<AppState>,
    auth_service: Arc<AuthService>,
    audit_logger: Arc<AuditLogger>,
}

impl KycServiceImpl {
    pub fn new(
        state: Arc<AppState>,
        auth_service: Arc<AuthService>,
        audit_logger: Arc<AuditLogger>,
    ) -> Self {
        Self {
            state,
            auth_service,
            audit_logger,
        }
    }

    /// Convert proto PersonalInfo to model PersonalInfo
    fn convert_personal_info(proto_info: crate::proto::fo3::wallet::v1::PersonalInfo) -> Result<PersonalInfo, Status> {
        let address = proto_info.address
            .ok_or_else(|| Status::invalid_argument("Address is required"))?;

        let date_of_birth = NaiveDate::parse_from_str(&proto_info.date_of_birth, "%Y-%m-%d")
            .map_err(|_| Status::invalid_argument("Invalid date format. Use YYYY-MM-DD"))?;

        Ok(PersonalInfo {
            first_name: proto_info.first_name,
            last_name: proto_info.last_name,
            date_of_birth,
            nationality: proto_info.nationality,
            country_of_residence: proto_info.country_of_residence,
            address: Address {
                street_address: address.street_address,
                city: address.city,
                state_province: if address.state_province.is_empty() {
                    None
                } else {
                    Some(address.state_province)
                },
                postal_code: address.postal_code,
                country: address.country,
            },
        })
    }

    /// Convert model KycSubmission to proto KycSubmission
    fn convert_to_proto_submission(submission: &KycSubmission) -> crate::proto::fo3::wallet::v1::KycSubmission {
        let personal_info = crate::proto::fo3::wallet::v1::PersonalInfo {
            first_name: submission.personal_info.first_name.clone(),
            last_name: submission.personal_info.last_name.clone(),
            date_of_birth: submission.personal_info.date_of_birth.format("%Y-%m-%d").to_string(),
            nationality: submission.personal_info.nationality.clone(),
            country_of_residence: submission.personal_info.country_of_residence.clone(),
            address: Some(crate::proto::fo3::wallet::v1::Address {
                street_address: submission.personal_info.address.street_address.clone(),
                city: submission.personal_info.address.city.clone(),
                state_province: submission.personal_info.address.state_province.clone().unwrap_or_default(),
                postal_code: submission.personal_info.address.postal_code.clone(),
                country: submission.personal_info.address.country.clone(),
            }),
        };

        let documents: Vec<crate::proto::fo3::wallet::v1::Document> = submission.documents
            .iter()
            .map(|doc| crate::proto::fo3::wallet::v1::Document {
                id: doc.id.to_string(),
                r#type: Self::convert_document_type_to_proto(doc.document_type) as i32,
                filename: doc.filename.clone(),
                content_type: doc.content_type.clone(),
                size_bytes: doc.size_bytes,
                hash: doc.file_hash.clone(),
                uploaded_at: doc.uploaded_at.timestamp(),
                is_encrypted: doc.is_encrypted,
            })
            .collect();

        crate::proto::fo3::wallet::v1::KycSubmission {
            id: submission.id.to_string(),
            wallet_id: submission.wallet_id.to_string(),
            status: Self::convert_status_to_proto(submission.status) as i32,
            personal_info: Some(personal_info),
            documents,
            submitted_at: submission.submitted_at.timestamp(),
            reviewed_at: submission.reviewed_at.map(|dt| dt.timestamp()).unwrap_or(0),
            reviewer_id: submission.reviewer_id.clone().unwrap_or_default(),
            reviewer_notes: submission.reviewer_notes.clone().unwrap_or_default(),
            rejection_reason: submission.rejection_reason.clone().unwrap_or_default(),
            updated_at: submission.updated_at.timestamp(),
        }
    }

    /// Convert model KycStatus to proto KycStatus
    fn convert_status_to_proto(status: KycStatus) -> crate::proto::fo3::wallet::v1::KycStatus {
        match status {
            KycStatus::Pending => crate::proto::fo3::wallet::v1::KycStatus::KycStatusPending,
            KycStatus::UnderReview => crate::proto::fo3::wallet::v1::KycStatus::KycStatusUnderReview,
            KycStatus::Approved => crate::proto::fo3::wallet::v1::KycStatus::KycStatusApproved,
            KycStatus::Rejected => crate::proto::fo3::wallet::v1::KycStatus::KycStatusRejected,
            KycStatus::RequiresUpdate => crate::proto::fo3::wallet::v1::KycStatus::KycStatusRequiresUpdate,
        }
    }

    /// Convert proto KycStatus to model KycStatus
    fn convert_status_from_proto(status: i32) -> Result<KycStatus, Status> {
        match crate::proto::fo3::wallet::v1::KycStatus::try_from(status) {
            Ok(crate::proto::fo3::wallet::v1::KycStatus::KycStatusPending) => Ok(KycStatus::Pending),
            Ok(crate::proto::fo3::wallet::v1::KycStatus::KycStatusUnderReview) => Ok(KycStatus::UnderReview),
            Ok(crate::proto::fo3::wallet::v1::KycStatus::KycStatusApproved) => Ok(KycStatus::Approved),
            Ok(crate::proto::fo3::wallet::v1::KycStatus::KycStatusRejected) => Ok(KycStatus::Rejected),
            Ok(crate::proto::fo3::wallet::v1::KycStatus::KycStatusRequiresUpdate) => Ok(KycStatus::RequiresUpdate),
            _ => Err(Status::invalid_argument("Invalid KYC status")),
        }
    }

    /// Convert model DocumentType to proto DocumentType
    fn convert_document_type_to_proto(doc_type: DocumentType) -> crate::proto::fo3::wallet::v1::DocumentType {
        match doc_type {
            DocumentType::GovernmentId => crate::proto::fo3::wallet::v1::DocumentType::DocumentTypeGovernmentId,
            DocumentType::ProofOfAddress => crate::proto::fo3::wallet::v1::DocumentType::DocumentTypeProofOfAddress,
            DocumentType::Selfie => crate::proto::fo3::wallet::v1::DocumentType::DocumentTypeSelfie,
            DocumentType::BankStatement => crate::proto::fo3::wallet::v1::DocumentType::DocumentTypeBankStatement,
            DocumentType::Other => crate::proto::fo3::wallet::v1::DocumentType::DocumentTypeOther,
        }
    }

    /// Convert proto DocumentType to model DocumentType
    fn convert_document_type_from_proto(doc_type: i32) -> Result<DocumentType, Status> {
        match crate::proto::fo3::wallet::v1::DocumentType::try_from(doc_type) {
            Ok(crate::proto::fo3::wallet::v1::DocumentType::DocumentTypeGovernmentId) => Ok(DocumentType::GovernmentId),
            Ok(crate::proto::fo3::wallet::v1::DocumentType::DocumentTypeProofOfAddress) => Ok(DocumentType::ProofOfAddress),
            Ok(crate::proto::fo3::wallet::v1::DocumentType::DocumentTypeSelfie) => Ok(DocumentType::Selfie),
            Ok(crate::proto::fo3::wallet::v1::DocumentType::DocumentTypeBankStatement) => Ok(DocumentType::BankStatement),
            Ok(crate::proto::fo3::wallet::v1::DocumentType::DocumentTypeOther) => Ok(DocumentType::Other),
            _ => Err(Status::invalid_argument("Invalid document type")),
        }
    }

    /// Check if user can access KYC submission
    fn check_kyc_access(&self, auth: &AuthContext, submission: &KycSubmission) -> Result<(), Status> {
        // Users can only access their own KYC submissions
        if auth.user_id != submission.wallet_id.to_string() {
            // Unless they have admin permission
            self.auth_service.check_permission(auth, crate::proto::fo3::wallet::v1::Permission::PermissionKycAdmin)?;
        }
        Ok(())
    }
}

#[tonic::async_trait]
impl KycService for KycServiceImpl {
    type DownloadDocumentStream = tokio_stream::wrappers::ReceiverStream<Result<DownloadDocumentResponse, Status>>;
    /// Submit KYC information for verification
    async fn submit_kyc(
        &self,
        request: Request<SubmitKycRequest>,
    ) -> Result<Response<SubmitKycResponse>, Status> {
        let auth_context = request.extensions().get::<AuthContext>()
            .ok_or_else(|| Status::unauthenticated("Authentication required"))?;

        // Check KYC submit permission
        self.auth_service.check_permission(auth_context, crate::proto::fo3::wallet::v1::Permission::PermissionKycSubmit)?;

        let req = request.into_inner();

        // Validate wallet ID
        let wallet_id = Uuid::parse_str(&req.wallet_id)
            .map_err(|_| Status::invalid_argument("Invalid wallet ID format"))?;

        // Users can only submit KYC for their own wallet
        if auth_context.user_id != req.wallet_id {
            return Err(Status::permission_denied("Can only submit KYC for your own wallet"));
        }

        // Convert personal info
        let personal_info = req.personal_info
            .ok_or_else(|| Status::invalid_argument("Personal info is required"))?;
        let personal_info = Self::convert_personal_info(personal_info)?;

        // Check if KYC already exists for this wallet
        {
            let submissions = self.state.kyc_submissions.read().unwrap();
            if submissions.contains_key(&req.wallet_id) {
                return Err(Status::already_exists("KYC submission already exists for this wallet"));
            }
        }

        // Create new KYC submission
        let mut submission = KycSubmission::new(wallet_id, personal_info);

        // Add documents if provided
        for doc_id in req.document_ids {
            let document_uuid = Uuid::parse_str(&doc_id)
                .map_err(|_| Status::invalid_argument("Invalid document ID format"))?;
            
            // In a real implementation, we would fetch document metadata from storage
            // For now, we'll create a placeholder document
            let document = Document {
                id: document_uuid,
                submission_id: submission.id,
                document_type: DocumentType::Other, // Would be determined from storage
                filename: "placeholder.pdf".to_string(),
                content_type: "application/pdf".to_string(),
                size_bytes: 0,
                file_hash: "placeholder_hash".to_string(),
                storage_path: format!("documents/{}", document_uuid),
                is_encrypted: true,
                uploaded_at: Utc::now(),
                deleted_at: None,
            };
            submission.add_document(document);
        }

        // Store submission
        {
            let mut submissions = self.state.kyc_submissions.write().unwrap();
            submissions.insert(submission.id.to_string(), submission.clone());
        }

        // Log audit event
        self.audit_logger.log_kyc_submission(auth_context, &submission.id.to_string(), true).await;

        let response = SubmitKycResponse {
            submission: Some(Self::convert_to_proto_submission(&submission)),
        };

        Ok(Response::new(response))
    }

    /// Get KYC status for a wallet
    async fn get_kyc_status(
        &self,
        request: Request<GetKycStatusRequest>,
    ) -> Result<Response<GetKycStatusResponse>, Status> {
        let auth_context = request.extensions().get::<AuthContext>()
            .ok_or_else(|| Status::unauthenticated("Authentication required"))?;

        // Check KYC view permission
        self.auth_service.check_permission(auth_context, crate::proto::fo3::wallet::v1::Permission::PermissionKycView)?;

        let req = request.into_inner();

        // Users can only view their own KYC status unless they have admin permission
        if auth_context.user_id != req.wallet_id {
            self.auth_service.check_permission(auth_context, crate::proto::fo3::wallet::v1::Permission::PermissionKycAdmin)?;
        }

        // Find submission by wallet ID
        let submission = {
            let submissions = self.state.kyc_submissions.read().unwrap();
            submissions.values()
                .find(|s| s.wallet_id.to_string() == req.wallet_id)
                .cloned()
        };

        let submission = submission
            .ok_or_else(|| Status::not_found("KYC submission not found for this wallet"))?;

        // Log audit event
        self.audit_logger.log_kyc_status_view(auth_context, &submission.id.to_string()).await;

        let response = GetKycStatusResponse {
            submission: Some(Self::convert_to_proto_submission(&submission)),
        };

        Ok(Response::new(response))
    }

    /// List KYC submissions (admin only)
    async fn list_kyc_submissions(
        &self,
        request: Request<ListKycSubmissionsRequest>,
    ) -> Result<Response<ListKycSubmissionsResponse>, Status> {
        let auth_context = request.extensions().get::<AuthContext>()
            .ok_or_else(|| Status::unauthenticated("Authentication required"))?;

        // Check admin permission
        self.auth_service.check_permission(auth_context, crate::proto::fo3::wallet::v1::Permission::PermissionKycAdmin)?;

        let req = request.into_inner();
        let page_size = if req.page_size <= 0 { 50 } else { req.page_size.min(100) };

        // Get all submissions
        let all_submissions: Vec<KycSubmission> = {
            let submissions = self.state.kyc_submissions.read().unwrap();
            submissions.values().cloned().collect()
        };

        // Apply filters
        let mut filtered_submissions: Vec<KycSubmission> = all_submissions.into_iter()
            .filter(|submission| {
                // Status filter
                if req.status_filter != 0 {
                    if let Ok(filter_status) = Self::convert_status_from_proto(req.status_filter) {
                        if submission.status != filter_status {
                            return false;
                        }
                    }
                }

                // Wallet ID filter
                if !req.wallet_id_filter.is_empty() {
                    if submission.wallet_id.to_string() != req.wallet_id_filter {
                        return false;
                    }
                }

                true
            })
            .collect();

        // Sort by submission date (newest first)
        filtered_submissions.sort_by(|a, b| b.submitted_at.cmp(&a.submitted_at));

        let total_count = filtered_submissions.len() as i32;

        // Handle pagination
        let start_index = if req.page_token.is_empty() {
            0
        } else {
            req.page_token.parse::<usize>().unwrap_or(0)
        };

        let end_index = (start_index + page_size as usize).min(filtered_submissions.len());
        let page_submissions = filtered_submissions[start_index..end_index].to_vec();

        let next_page_token = if end_index < filtered_submissions.len() {
            end_index.to_string()
        } else {
            String::new()
        };

        let proto_submissions: Vec<crate::proto::fo3::wallet::v1::KycSubmission> = page_submissions
            .iter()
            .map(Self::convert_to_proto_submission)
            .collect();

        let response = ListKycSubmissionsResponse {
            submissions: proto_submissions,
            next_page_token,
            total_count,
        };

        Ok(Response::new(response))
    }

    /// Approve KYC submission (admin only)
    async fn approve_kyc(
        &self,
        request: Request<ApproveKycRequest>,
    ) -> Result<Response<ApproveKycResponse>, Status> {
        let auth_context = request.extensions().get::<AuthContext>()
            .ok_or_else(|| Status::unauthenticated("Authentication required"))?;

        // Check admin permission
        self.auth_service.check_permission(auth_context, crate::proto::fo3::wallet::v1::Permission::PermissionKycAdmin)?;

        let req = request.into_inner();

        // Find and update submission
        let mut submission = {
            let mut submissions = self.state.kyc_submissions.write().unwrap();
            let submission = submissions.get_mut(&req.submission_id)
                .ok_or_else(|| Status::not_found("KYC submission not found"))?;

            if submission.is_final() {
                return Err(Status::failed_precondition("KYC submission is already in final state"));
            }

            submission.approve(
                auth_context.user_id.clone(),
                if req.reviewer_notes.is_empty() { None } else { Some(req.reviewer_notes.clone()) }
            );

            submission.clone()
        };

        // Log audit event
        self.audit_logger.log_kyc_approval(
            auth_context,
            &submission.id.to_string(),
            if req.reviewer_notes.is_empty() { None } else { Some(&req.reviewer_notes) }
        ).await;

        let response = ApproveKycResponse {
            submission: Some(Self::convert_to_proto_submission(&submission)),
        };

        Ok(Response::new(response))
    }

    /// Reject KYC submission (admin only)
    async fn reject_kyc(
        &self,
        request: Request<RejectKycRequest>,
    ) -> Result<Response<RejectKycResponse>, Status> {
        let auth_context = request.extensions().get::<AuthContext>()
            .ok_or_else(|| Status::unauthenticated("Authentication required"))?;

        // Check admin permission
        self.auth_service.check_permission(auth_context, crate::proto::fo3::wallet::v1::Permission::PermissionKycAdmin)?;

        let req = request.into_inner();

        if req.rejection_reason.is_empty() {
            return Err(Status::invalid_argument("Rejection reason is required"));
        }

        // Find and update submission
        let mut submission = {
            let mut submissions = self.state.kyc_submissions.write().unwrap();
            let submission = submissions.get_mut(&req.submission_id)
                .ok_or_else(|| Status::not_found("KYC submission not found"))?;

            if submission.is_final() {
                return Err(Status::failed_precondition("KYC submission is already in final state"));
            }

            submission.reject(
                auth_context.user_id.clone(),
                req.rejection_reason.clone(),
                if req.reviewer_notes.is_empty() { None } else { Some(req.reviewer_notes.clone()) }
            );

            submission.clone()
        };

        // Log audit event
        self.audit_logger.log_kyc_rejection(
            auth_context,
            &submission.id.to_string(),
            &req.rejection_reason
        ).await;

        let response = RejectKycResponse {
            submission: Some(Self::convert_to_proto_submission(&submission)),
        };

        Ok(Response::new(response))
    }

    /// Update KYC documents
    async fn update_kyc_documents(
        &self,
        request: Request<UpdateKycDocumentsRequest>,
    ) -> Result<Response<UpdateKycDocumentsResponse>, Status> {
        let auth_context = request.extensions().get::<AuthContext>()
            .ok_or_else(|| Status::unauthenticated("Authentication required"))?;

        // Check KYC submit permission
        self.auth_service.check_permission(auth_context, crate::proto::fo3::wallet::v1::Permission::PermissionKycSubmit)?;

        let req = request.into_inner();

        // Find and update submission
        let mut submission = {
            let mut submissions = self.state.kyc_submissions.write().unwrap();
            let submission = submissions.get_mut(&req.submission_id)
                .ok_or_else(|| Status::not_found("KYC submission not found"))?;

            // Check access
            self.check_kyc_access(auth_context, submission)?;

            if !submission.can_be_updated() {
                return Err(Status::failed_precondition("KYC submission cannot be updated in current state"));
            }

            // Remove documents
            for doc_id in &req.remove_document_ids {
                let document_uuid = Uuid::parse_str(doc_id)
                    .map_err(|_| Status::invalid_argument("Invalid document ID format"))?;
                submission.remove_document(document_uuid);
            }

            // Add new documents (placeholder implementation)
            for doc_id in &req.document_ids {
                let document_uuid = Uuid::parse_str(doc_id)
                    .map_err(|_| Status::invalid_argument("Invalid document ID format"))?;

                let document = Document {
                    id: document_uuid,
                    submission_id: submission.id,
                    document_type: DocumentType::Other,
                    filename: "updated_document.pdf".to_string(),
                    content_type: "application/pdf".to_string(),
                    size_bytes: 0,
                    file_hash: "updated_hash".to_string(),
                    storage_path: format!("documents/{}", document_uuid),
                    is_encrypted: true,
                    uploaded_at: Utc::now(),
                    deleted_at: None,
                };
                submission.add_document(document);
            }

            submission.clone()
        };

        let response = UpdateKycDocumentsResponse {
            submission: Some(Self::convert_to_proto_submission(&submission)),
        };

        Ok(Response::new(response))
    }

    /// Upload document for KYC (streaming)
    async fn upload_document(
        &self,
        request: Request<Streaming<UploadDocumentRequest>>,
    ) -> Result<Response<UploadDocumentResponse>, Status> {
        // This is a placeholder implementation for document upload
        // In a real implementation, this would handle streaming file uploads

        let document_id = Uuid::new_v4();

        let response = UploadDocumentResponse {
            document_id: document_id.to_string(),
            upload_url: format!("https://storage.example.com/documents/{}", document_id),
        };

        Ok(Response::new(response))
    }

    /// Download document (admin only, streaming)
    async fn download_document(
        &self,
        request: Request<DownloadDocumentRequest>,
    ) -> Result<Response<Self::DownloadDocumentStream>, Status> {
        let auth_context = request.extensions().get::<AuthContext>()
            .ok_or_else(|| Status::unauthenticated("Authentication required"))?;

        // Check admin permission
        self.auth_service.check_permission(auth_context, crate::proto::fo3::wallet::v1::Permission::PermissionKycAdmin)?;

        // This is a placeholder implementation for document download
        // In a real implementation, this would stream the document content

        Err(Status::unimplemented("Document download not yet implemented"))
    }
}
