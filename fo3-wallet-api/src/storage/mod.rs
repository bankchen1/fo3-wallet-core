//! Storage modules for the FO3 Wallet API

pub mod documents;

pub use documents::{DocumentStorage, DocumentStorageConfig, DocumentStorageError, DocumentUploadHandler};
