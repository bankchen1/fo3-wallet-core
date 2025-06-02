//! Error handling for gRPC services

use tonic::{Code, Status};
use fo3_wallet::error::Error as WalletError;

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
