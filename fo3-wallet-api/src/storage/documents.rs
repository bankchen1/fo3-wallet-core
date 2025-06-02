//! Secure document storage system for KYC documents

use std::path::{Path, PathBuf};
use std::fs;
use std::io::{self, Read, Write};
use tokio::fs as async_fs;
use tokio::io::{AsyncRead, AsyncWrite, AsyncReadExt, AsyncWriteExt};
use uuid::Uuid;
use sha2::{Sha256, Digest};
use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce, Key
};
use rand::RngCore;
use base64::{Engine as _, engine::general_purpose};
use serde::{Serialize, Deserialize};

use crate::models::kyc::{Document, DocumentType};

/// Document storage configuration
#[derive(Debug, Clone)]
pub struct DocumentStorageConfig {
    /// Base directory for document storage
    pub storage_path: PathBuf,
    /// Maximum file size in bytes (default: 10MB)
    pub max_file_size: usize,
    /// Encryption key for document encryption
    pub encryption_key: [u8; 32],
    /// Allowed content types
    pub allowed_content_types: Vec<String>,
}

impl Default for DocumentStorageConfig {
    fn default() -> Self {
        Self {
            storage_path: PathBuf::from("./data/kyc_documents"),
            max_file_size: 10 * 1024 * 1024, // 10MB
            encryption_key: [0u8; 32], // Should be loaded from environment
            allowed_content_types: vec![
                "image/jpeg".to_string(),
                "image/png".to_string(),
                "image/gif".to_string(),
                "application/pdf".to_string(),
                "image/webp".to_string(),
            ],
        }
    }
}

/// Document storage error
#[derive(Debug, thiserror::Error)]
pub enum DocumentStorageError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("Encryption error: {0}")]
    Encryption(String),
    #[error("File too large: {size} bytes (max: {max_size} bytes)")]
    FileTooLarge { size: usize, max_size: usize },
    #[error("Invalid content type: {content_type}")]
    InvalidContentType { content_type: String },
    #[error("Document not found: {id}")]
    DocumentNotFound { id: Uuid },
    #[error("Hash mismatch: expected {expected}, got {actual}")]
    HashMismatch { expected: String, actual: String },
}

/// Encrypted document metadata
#[derive(Debug, Serialize, Deserialize)]
struct EncryptedDocumentMetadata {
    /// Original filename
    filename: String,
    /// Content type
    content_type: String,
    /// Original file size
    original_size: usize,
    /// Encryption nonce
    nonce: Vec<u8>,
    /// File hash (SHA-256 of original content)
    file_hash: String,
}

/// Document storage service
#[derive(Clone)]
pub struct DocumentStorage {
    config: DocumentStorageConfig,
    cipher: Aes256Gcm,
}

impl DocumentStorage {
    /// Create a new document storage service
    pub fn new(config: DocumentStorageConfig) -> Result<Self, DocumentStorageError> {
        // Create storage directory if it doesn't exist
        fs::create_dir_all(&config.storage_path)?;

        let key = Key::<Aes256Gcm>::from_slice(&config.encryption_key);
        let cipher = Aes256Gcm::new(key);

        Ok(Self { config, cipher })
    }

    /// Store a document with encryption
    pub async fn store_document(
        &self,
        document_id: Uuid,
        content: &[u8],
        filename: &str,
        content_type: &str,
    ) -> Result<(String, String), DocumentStorageError> {
        // Validate file size
        if content.len() > self.config.max_file_size {
            return Err(DocumentStorageError::FileTooLarge {
                size: content.len(),
                max_size: self.config.max_file_size,
            });
        }

        // Validate content type
        if !self.config.allowed_content_types.contains(&content_type.to_string()) {
            return Err(DocumentStorageError::InvalidContentType {
                content_type: content_type.to_string(),
            });
        }

        // Validate file content for security threats
        self.validate_file_content(content, content_type)?;

        // Calculate file hash
        let mut hasher = Sha256::new();
        hasher.update(content);
        let file_hash = format!("{:x}", hasher.finalize());

        // Generate nonce for encryption
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

        // Encrypt the content
        let encrypted_content = self.cipher
            .encrypt(&nonce, content)
            .map_err(|e| DocumentStorageError::Encryption(e.to_string()))?;

        // Create metadata
        let metadata = EncryptedDocumentMetadata {
            filename: filename.to_string(),
            content_type: content_type.to_string(),
            original_size: content.len(),
            nonce: nonce.to_vec(),
            file_hash: file_hash.clone(),
        };

        // Generate storage paths
        let document_dir = self.config.storage_path.join(document_id.to_string());
        async_fs::create_dir_all(&document_dir).await?;

        let content_path = document_dir.join("content.enc");
        let metadata_path = document_dir.join("metadata.json");

        // Write encrypted content
        async_fs::write(&content_path, &encrypted_content).await?;

        // Write metadata
        let metadata_json = serde_json::to_string_pretty(&metadata)
            .map_err(|e| DocumentStorageError::Encryption(e.to_string()))?;
        async_fs::write(&metadata_path, metadata_json).await?;

        Ok((content_path.to_string_lossy().to_string(), file_hash))
    }

    /// Retrieve and decrypt a document
    pub async fn retrieve_document(
        &self,
        document_id: Uuid,
    ) -> Result<(Vec<u8>, EncryptedDocumentMetadata), DocumentStorageError> {
        let document_dir = self.config.storage_path.join(document_id.to_string());
        let content_path = document_dir.join("content.enc");
        let metadata_path = document_dir.join("metadata.json");

        // Check if document exists
        if !content_path.exists() || !metadata_path.exists() {
            return Err(DocumentStorageError::DocumentNotFound { id: document_id });
        }

        // Read metadata
        let metadata_json = async_fs::read_to_string(&metadata_path).await?;
        let metadata: EncryptedDocumentMetadata = serde_json::from_str(&metadata_json)
            .map_err(|e| DocumentStorageError::Encryption(e.to_string()))?;

        // Read encrypted content
        let encrypted_content = async_fs::read(&content_path).await?;

        // Decrypt content
        let nonce = Nonce::from_slice(&metadata.nonce);
        let decrypted_content = self.cipher
            .decrypt(nonce, encrypted_content.as_ref())
            .map_err(|e| DocumentStorageError::Encryption(e.to_string()))?;

        // Verify hash
        let mut hasher = Sha256::new();
        hasher.update(&decrypted_content);
        let calculated_hash = format!("{:x}", hasher.finalize());

        if calculated_hash != metadata.file_hash {
            return Err(DocumentStorageError::HashMismatch {
                expected: metadata.file_hash.clone(),
                actual: calculated_hash,
            });
        }

        Ok((decrypted_content, metadata))
    }

    /// Delete a document (secure deletion)
    pub async fn delete_document(&self, document_id: Uuid) -> Result<(), DocumentStorageError> {
        let document_dir = self.config.storage_path.join(document_id.to_string());

        if document_dir.exists() {
            // Securely overwrite files before deletion
            let content_path = document_dir.join("content.enc");
            if content_path.exists() {
                self.secure_delete_file(&content_path).await?;
            }

            let metadata_path = document_dir.join("metadata.json");
            if metadata_path.exists() {
                async_fs::remove_file(&metadata_path).await?;
            }

            // Remove directory
            async_fs::remove_dir(&document_dir).await?;
        }

        Ok(())
    }

    /// Get document metadata without decrypting content
    pub async fn get_document_metadata(
        &self,
        document_id: Uuid,
    ) -> Result<EncryptedDocumentMetadata, DocumentStorageError> {
        let document_dir = self.config.storage_path.join(document_id.to_string());
        let metadata_path = document_dir.join("metadata.json");

        if !metadata_path.exists() {
            return Err(DocumentStorageError::DocumentNotFound { id: document_id });
        }

        let metadata_json = async_fs::read_to_string(&metadata_path).await?;
        let metadata: EncryptedDocumentMetadata = serde_json::from_str(&metadata_json)
            .map_err(|e| DocumentStorageError::Encryption(e.to_string()))?;

        Ok(metadata)
    }

    /// Validate document content type
    pub fn validate_content_type(&self, content_type: &str) -> bool {
        self.config.allowed_content_types.contains(&content_type.to_string())
    }

    /// Validate file size
    pub fn validate_file_size(&self, size: usize) -> bool {
        size <= self.config.max_file_size
    }

    /// Securely delete a file by overwriting it multiple times
    async fn secure_delete_file(&self, path: &Path) -> Result<(), DocumentStorageError> {
        use tokio::fs::OpenOptions;

        let file_size = async_fs::metadata(path).await?.len() as usize;

        // Open file for writing
        let mut file = OpenOptions::new()
            .write(true)
            .truncate(false)
            .open(path)
            .await?;

        // Overwrite with random data 3 times (DoD 5220.22-M standard)
        for _ in 0..3 {
            let mut random_data = vec![0u8; file_size];
            rand::thread_rng().fill_bytes(&mut random_data);
            file.write_all(&random_data).await?;
            file.flush().await?;
        }

        // Finally remove the file
        async_fs::remove_file(path).await?;

        Ok(())
    }

    /// Generate audit log entry for document operations
    fn create_audit_entry(&self, operation: &str, document_id: Uuid, success: bool, error: Option<&str>) -> serde_json::Value {
        let mut entry = serde_json::json!({
            "operation": operation,
            "document_id": document_id.to_string(),
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "success": success,
        });

        if let Some(err) = error {
            entry["error"] = serde_json::Value::String(err.to_string());
        }

        entry
    }

    /// Validate file content for security threats
    fn validate_file_content(&self, content: &[u8], content_type: &str) -> Result<(), DocumentStorageError> {
        // Check for executable file signatures
        let executable_signatures = [
            b"\x4D\x5A", // PE executable (Windows)
            b"\x7F\x45\x4C\x46", // ELF executable (Linux)
            b"\xFE\xED\xFA", // Mach-O executable (macOS)
            b"\xCA\xFE\xBA\xBE", // Java class file
        ];

        for signature in &executable_signatures {
            if content.starts_with(signature) {
                return Err(DocumentStorageError::InvalidContentType {
                    content_type: "Executable files are not allowed".to_string(),
                });
            }
        }

        // Validate content type matches file header
        match content_type {
            "application/pdf" => {
                if !content.starts_with(b"%PDF-") {
                    return Err(DocumentStorageError::InvalidContentType {
                        content_type: "PDF content type mismatch".to_string(),
                    });
                }
            }
            "image/jpeg" => {
                if !content.starts_with(b"\xFF\xD8\xFF") {
                    return Err(DocumentStorageError::InvalidContentType {
                        content_type: "JPEG content type mismatch".to_string(),
                    });
                }
            }
            "image/png" => {
                if !content.starts_with(b"\x89PNG\r\n\x1A\n") {
                    return Err(DocumentStorageError::InvalidContentType {
                        content_type: "PNG content type mismatch".to_string(),
                    });
                }
            }
            _ => {} // Allow other types without specific validation
        }

        Ok(())
    }
}

/// Document upload stream handler
pub struct DocumentUploadHandler {
    storage: DocumentStorage,
    max_chunk_size: usize,
}

impl DocumentUploadHandler {
    pub fn new(storage: DocumentStorage) -> Self {
        Self {
            storage,
            max_chunk_size: 1024 * 1024, // 1MB chunks
        }
    }

    /// Handle streaming document upload
    pub async fn handle_upload_stream<S>(
        &self,
        document_id: Uuid,
        filename: &str,
        content_type: &str,
        mut stream: S,
    ) -> Result<(String, String), DocumentStorageError>
    where
        S: AsyncRead + Unpin,
    {
        let mut buffer = Vec::new();
        let mut chunk = vec![0u8; self.max_chunk_size];

        // Read all chunks
        loop {
            let bytes_read = stream.read(&mut chunk).await?;
            if bytes_read == 0 {
                break;
            }
            buffer.extend_from_slice(&chunk[..bytes_read]);

            // Check size limit
            if buffer.len() > self.storage.config.max_file_size {
                return Err(DocumentStorageError::FileTooLarge {
                    size: buffer.len(),
                    max_size: self.storage.config.max_file_size,
                });
            }
        }

        // Store the complete document
        self.storage.store_document(document_id, &buffer, filename, content_type).await
    }
}
