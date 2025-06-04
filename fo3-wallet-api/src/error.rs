//! Error handling for gRPC services

use tonic::{Code, Status};
use fo3_wallet::error::Error as WalletError;
use thiserror::Error;

/// Service error types for FO3 Wallet Core
#[derive(Error, Debug)]
pub enum ServiceError {
    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Cache error: {0}")]
    CacheError(String),

    #[error("Authentication error: {0}")]
    AuthenticationError(String),

    #[error("Authorization error: {0}")]
    AuthorizationError(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Not found: {0}")]
    NotFoundError(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Security error: {0}")]
    SecurityError(String),

    #[error("Rate limit exceeded: {0}")]
    RateLimitExceeded(String),

    #[error("External service error: {0}")]
    ExternalServiceError(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Configuration error: {0}")]
    ConfigurationError(String),

    #[error("Internal error: {0}")]
    InternalError(String),

    #[error("Wallet error: {0}")]
    WalletError(#[from] WalletError),
}

impl From<ServiceError> for Status {
    fn from(error: ServiceError) -> Self {
        match error {
            ServiceError::ValidationError(msg) => Status::invalid_argument(msg),
            ServiceError::AuthenticationError(msg) => Status::unauthenticated(msg),
            ServiceError::AuthorizationError(msg) => Status::permission_denied(msg),
            ServiceError::NotFound(msg) => Status::not_found(msg),
            ServiceError::NotFoundError(msg) => Status::not_found(msg),
            ServiceError::Conflict(msg) => Status::already_exists(msg),
            ServiceError::SecurityError(msg) => Status::permission_denied(msg),
            ServiceError::RateLimitExceeded(msg) => Status::resource_exhausted(msg),
            ServiceError::ExternalServiceError(msg) => Status::unavailable(msg),
            ServiceError::NetworkError(msg) => Status::unavailable(msg),
            ServiceError::DatabaseError(msg) => Status::internal(msg),
            ServiceError::CacheError(msg) => Status::internal(msg),
            ServiceError::SerializationError(msg) => Status::internal(msg),
            ServiceError::ConfigurationError(msg) => Status::internal(msg),
            ServiceError::InternalError(msg) => Status::internal(msg),
            ServiceError::WalletError(wallet_error) => wallet_error_to_status(wallet_error),
        }
    }
}

/// Convert wallet errors to gRPC status
pub fn wallet_error_to_status(error: WalletError) -> Status {
    match error {
        WalletError::InvalidMnemonic(_) => Status::invalid_argument(error.to_string()),
        WalletError::InvalidPrivateKey(_) => Status::invalid_argument(error.to_string()),
        WalletError::InvalidAddress(_) => Status::invalid_argument(error.to_string()),
        WalletError::InvalidDerivationPath(_) => Status::invalid_argument(error.to_string()),
        WalletError::NetworkError(_) => Status::unavailable(error.to_string()),
        WalletError::TransactionError(_) => Status::internal(error.to_string()),
        WalletError::InsufficientFunds => Status::failed_precondition(error.to_string()),
        _ => Status::internal(error.to_string()),
    }
}

/// Convert string errors to gRPC status
pub fn string_error_to_status(error: String) -> Status {
    Status::internal(error)
}

/// Convert not found errors to gRPC status
pub fn not_found_error(message: &str) -> Status {
    Status::not_found(message)
}

/// Convert invalid argument errors to gRPC status
pub fn invalid_argument_error(message: &str) -> Status {
    Status::invalid_argument(message)
}
