//! KYC authorization middleware

use std::sync::Arc;
use tonic::{Request, Status};
use uuid::Uuid;

use crate::middleware::auth::{AuthContext, AuthService};
use crate::models::kyc::{KycStatus, KycSubmission};
use crate::state::AppState;
use crate::proto::fo3::wallet::v1::Permission;

/// KYC authorization guard
pub struct KycGuard {
    state: Arc<AppState>,
    auth_service: Arc<AuthService>,
}

impl KycGuard {
    pub fn new(state: Arc<AppState>, auth_service: Arc<AuthService>) -> Self {
        Self {
            state,
            auth_service,
        }
    }

    /// Check if user has completed KYC verification
    pub fn check_kyc_status(&self, auth: &AuthContext) -> Result<KycStatus, Status> {
        let wallet_id = Uuid::parse_str(&auth.user_id)
            .map_err(|_| Status::internal("Invalid user ID format"))?;

        let submissions = self.state.kyc_submissions.read().unwrap();
        let submission = submissions.values()
            .find(|s| s.wallet_id == wallet_id);

        match submission {
            Some(submission) => Ok(submission.status),
            None => Ok(KycStatus::Pending), // No submission means not started
        }
    }

    /// Require KYC approval for operation
    pub fn require_kyc_approval(&self, auth: &AuthContext) -> Result<(), Status> {
        // Admins can bypass KYC requirements
        if self.auth_service.check_permission(auth, Permission::PermissionAdmin).is_ok() {
            return Ok(());
        }

        let kyc_status = self.check_kyc_status(auth)?;
        
        match kyc_status {
            KycStatus::Approved => Ok(()),
            KycStatus::Pending => Err(Status::failed_precondition(
                "KYC verification required. Please submit your identity documents."
            )),
            KycStatus::UnderReview => Err(Status::failed_precondition(
                "KYC verification is under review. Please wait for approval."
            )),
            KycStatus::Rejected => Err(Status::failed_precondition(
                "KYC verification was rejected. Please contact support or resubmit with correct documents."
            )),
            KycStatus::RequiresUpdate => Err(Status::failed_precondition(
                "KYC verification requires additional information. Please update your submission."
            )),
        }
    }

    /// Check if user can perform high-value operations (requires KYC)
    pub fn check_high_value_operation(&self, auth: &AuthContext, amount: f64, threshold: f64) -> Result<(), Status> {
        if amount >= threshold {
            self.require_kyc_approval(auth)?;
        }
        Ok(())
    }

    /// Check if user can access KYC-restricted features
    pub fn check_kyc_restricted_feature(&self, auth: &AuthContext, feature: &str) -> Result<(), Status> {
        // Define features that require KYC
        let kyc_required_features = [
            "high_value_transfer",
            "defi_lending",
            "defi_staking",
            "institutional_trading",
            "fiat_onramp",
            "fiat_offramp",
        ];

        if kyc_required_features.contains(&feature) {
            self.require_kyc_approval(auth)?;
        }

        Ok(())
    }

    /// Get KYC submission for user
    pub fn get_user_kyc_submission(&self, auth: &AuthContext) -> Result<Option<KycSubmission>, Status> {
        let wallet_id = Uuid::parse_str(&auth.user_id)
            .map_err(|_| Status::internal("Invalid user ID format"))?;

        let submissions = self.state.kyc_submissions.read().unwrap();
        let submission = submissions.values()
            .find(|s| s.wallet_id == wallet_id)
            .cloned();

        Ok(submission)
    }

    /// Check if user can submit KYC (not already approved)
    pub fn can_submit_kyc(&self, auth: &AuthContext) -> Result<bool, Status> {
        let kyc_status = self.check_kyc_status(auth)?;
        
        Ok(match kyc_status {
            KycStatus::Pending | KycStatus::RequiresUpdate => true,
            KycStatus::UnderReview | KycStatus::Approved | KycStatus::Rejected => false,
        })
    }

    /// Check if user can update KYC documents
    pub fn can_update_kyc(&self, auth: &AuthContext) -> Result<bool, Status> {
        let kyc_status = self.check_kyc_status(auth)?;
        
        Ok(match kyc_status {
            KycStatus::Pending | KycStatus::RequiresUpdate => true,
            KycStatus::UnderReview | KycStatus::Approved | KycStatus::Rejected => false,
        })
    }

    /// Validate transaction amount against KYC limits
    pub fn validate_transaction_limits(&self, auth: &AuthContext, amount: f64) -> Result<(), Status> {
        let kyc_status = self.check_kyc_status(auth)?;
        
        // Define limits based on KYC status
        let daily_limit = match kyc_status {
            KycStatus::Approved => 100_000.0, // $100k for verified users
            KycStatus::UnderReview => 10_000.0, // $10k for pending verification
            KycStatus::Pending | KycStatus::RequiresUpdate => 1_000.0, // $1k for unverified
            KycStatus::Rejected => 0.0, // No transactions for rejected
        };

        if amount > daily_limit {
            return Err(Status::failed_precondition(format!(
                "Transaction amount ${:.2} exceeds daily limit of ${:.2} for your KYC status: {:?}",
                amount, daily_limit, kyc_status
            )));
        }

        Ok(())
    }

    /// Check compliance requirements for specific jurisdictions
    pub fn check_jurisdiction_compliance(&self, auth: &AuthContext, jurisdiction: &str) -> Result<(), Status> {
        let submission = self.get_user_kyc_submission(auth)?;
        
        match submission {
            Some(submission) => {
                // Check if user's country matches jurisdiction requirements
                if submission.status != KycStatus::Approved {
                    return Err(Status::failed_precondition(
                        "KYC verification required for this jurisdiction"
                    ));
                }

                // Additional jurisdiction-specific checks could be added here
                match jurisdiction {
                    "US" => {
                        // US-specific compliance checks
                        if submission.personal_info.country_of_residence != "US" {
                            return Err(Status::failed_precondition(
                                "US jurisdiction requires US residency"
                            ));
                        }
                    }
                    "EU" => {
                        // EU-specific compliance checks
                        let eu_countries = ["DE", "FR", "IT", "ES", "NL", "BE", "AT", "PT", "IE", "FI", "LU"];
                        if !eu_countries.contains(&submission.personal_info.country_of_residence.as_str()) {
                            return Err(Status::failed_precondition(
                                "EU jurisdiction requires EU residency"
                            ));
                        }
                    }
                    _ => {
                        // Default: just require approved KYC
                    }
                }

                Ok(())
            }
            None => Err(Status::failed_precondition(
                "KYC verification required for this jurisdiction"
            )),
        }
    }
}

/// KYC interceptor for gRPC services
pub struct KycInterceptor {
    kyc_guard: Arc<KycGuard>,
    require_kyc: bool,
}

impl KycInterceptor {
    pub fn new(kyc_guard: Arc<KycGuard>, require_kyc: bool) -> Self {
        Self {
            kyc_guard,
            require_kyc,
        }
    }

    pub async fn intercept<T>(&self, mut request: Request<T>) -> Result<Request<T>, Status> {
        if self.require_kyc {
            let auth_context = request.extensions().get::<AuthContext>()
                .ok_or_else(|| Status::unauthenticated("Authentication required"))?;

            self.kyc_guard.require_kyc_approval(auth_context)?;
        }

        Ok(request)
    }
}

/// Macro to create KYC-protected gRPC service
#[macro_export]
macro_rules! kyc_protected_service {
    ($service:expr, $kyc_guard:expr, $require_kyc:expr) => {{
        let interceptor = KycInterceptor::new($kyc_guard, $require_kyc);
        tonic::service::interceptor($service, move |req| {
            let interceptor = interceptor.clone();
            async move { interceptor.intercept(req).await }
        })
    }};
}

/// Helper functions for common KYC checks
impl KycGuard {
    /// Check if operation requires enhanced due diligence
    pub fn requires_enhanced_due_diligence(&self, amount: f64) -> bool {
        amount >= 50_000.0 // $50k threshold for enhanced due diligence
    }

    /// Check if user is on any watchlists (placeholder)
    pub fn check_watchlist(&self, auth: &AuthContext) -> Result<(), Status> {
        // In a real implementation, this would check against:
        // - OFAC sanctions lists
        // - PEP (Politically Exposed Persons) lists
        // - Internal risk lists
        
        // For now, just return OK
        Ok(())
    }

    /// Calculate risk score based on KYC data (placeholder)
    pub fn calculate_risk_score(&self, auth: &AuthContext) -> Result<f64, Status> {
        let submission = self.get_user_kyc_submission(auth)?;
        
        match submission {
            Some(submission) => {
                let mut risk_score = 0.0;
                
                // Base score based on KYC status
                risk_score += match submission.status {
                    KycStatus::Approved => 0.1,
                    KycStatus::UnderReview => 0.3,
                    KycStatus::Pending => 0.5,
                    KycStatus::RequiresUpdate => 0.4,
                    KycStatus::Rejected => 0.9,
                };

                // Additional risk factors could be added here
                // - Country risk
                // - Age of account
                // - Transaction patterns
                // - Document quality scores

                Ok(risk_score)
            }
            None => Ok(0.8), // High risk for no KYC
        }
    }
}
