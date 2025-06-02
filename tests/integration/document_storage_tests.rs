//! Integration tests for document storage security and functionality

use std::path::PathBuf;
use tempfile::TempDir;
use uuid::Uuid;

use fo3_wallet_api::storage::{DocumentStorage, DocumentStorageConfig, DocumentStorageError};

/// Create a test document storage with temporary directory
fn create_test_storage() -> (DocumentStorage, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let config = DocumentStorageConfig {
        storage_path: temp_dir.path().to_path_buf(),
        max_file_size: 1024 * 1024, // 1MB
        encryption_key: [42u8; 32], // Test key
        allowed_content_types: vec![
            "application/pdf".to_string(),
            "image/jpeg".to_string(),
            "image/png".to_string(),
        ],
    };

    let storage = DocumentStorage::new(config).unwrap();
    (storage, temp_dir)
}

#[tokio::test]
async fn test_document_storage_and_retrieval() {
    let (storage, _temp_dir) = create_test_storage();
    let document_id = Uuid::new_v4();

    // Test PDF content
    let pdf_content = b"%PDF-1.4\n1 0 obj\n<<\n/Type /Catalog\n>>\nendobj\nxref\n0 1\n0000000000 65535 f \ntrailer\n<<\n/Size 1\n/Root 1 0 R\n>>\nstartxref\n9\n%%EOF";
    
    // Store document
    let result = storage.store_document(
        document_id,
        pdf_content,
        "test.pdf",
        "application/pdf"
    ).await;

    assert!(result.is_ok());
    let (storage_path, file_hash) = result.unwrap();
    assert!(!storage_path.is_empty());
    assert!(!file_hash.is_empty());

    // Retrieve document
    let retrieval_result = storage.retrieve_document(document_id).await;
    assert!(retrieval_result.is_ok());

    let (retrieved_content, metadata) = retrieval_result.unwrap();
    assert_eq!(retrieved_content, pdf_content);
    assert_eq!(metadata.filename, "test.pdf");
    assert_eq!(metadata.content_type, "application/pdf");
    assert_eq!(metadata.file_hash, file_hash);
}

#[tokio::test]
async fn test_document_security_validation() {
    let (storage, _temp_dir) = create_test_storage();
    let document_id = Uuid::new_v4();

    // Test executable file rejection (PE header)
    let executable_content = b"\x4D\x5A\x90\x00\x03\x00\x00\x00"; // PE executable header
    
    let result = storage.store_document(
        document_id,
        executable_content,
        "malicious.exe",
        "application/octet-stream"
    ).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        DocumentStorageError::InvalidContentType { .. } => {
            // Expected error
        }
        _ => panic!("Expected InvalidContentType error"),
    }
}

#[tokio::test]
async fn test_content_type_validation() {
    let (storage, _temp_dir) = create_test_storage();
    let document_id = Uuid::new_v4();

    // Test PDF content with wrong content type
    let pdf_content = b"%PDF-1.4\ntest content";
    
    let result = storage.store_document(
        document_id,
        pdf_content,
        "test.jpg",
        "image/jpeg" // Wrong content type for PDF content
    ).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        DocumentStorageError::InvalidContentType { .. } => {
            // Expected error
        }
        _ => panic!("Expected InvalidContentType error"),
    }
}

#[tokio::test]
async fn test_file_size_limits() {
    let (storage, _temp_dir) = create_test_storage();
    let document_id = Uuid::new_v4();

    // Create content larger than the limit (1MB + 1 byte)
    let large_content = vec![0u8; 1024 * 1024 + 1];
    
    let result = storage.store_document(
        document_id,
        &large_content,
        "large_file.pdf",
        "application/pdf"
    ).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        DocumentStorageError::FileTooLarge { size, max_size } => {
            assert_eq!(size, 1024 * 1024 + 1);
            assert_eq!(max_size, 1024 * 1024);
        }
        _ => panic!("Expected FileTooLarge error"),
    }
}

#[tokio::test]
async fn test_document_encryption() {
    let (storage, temp_dir) = create_test_storage();
    let document_id = Uuid::new_v4();

    let original_content = b"This is sensitive KYC document content that should be encrypted";
    
    // Store document
    let result = storage.store_document(
        document_id,
        original_content,
        "sensitive.txt",
        "text/plain"
    ).await;

    assert!(result.is_ok());

    // Check that the file on disk is encrypted (not readable as plain text)
    let document_dir = temp_dir.path().join(document_id.to_string());
    let content_file = document_dir.join("content.enc");
    
    assert!(content_file.exists());
    
    let encrypted_content = tokio::fs::read(&content_file).await.unwrap();
    
    // Encrypted content should not match original content
    assert_ne!(encrypted_content, original_content);
    
    // Encrypted content should not contain readable text
    assert!(!encrypted_content.windows(10).any(|window| {
        window == b"sensitive " || window == b"document c"
    }));

    // But retrieval should work and return original content
    let (retrieved_content, _) = storage.retrieve_document(document_id).await.unwrap();
    assert_eq!(retrieved_content, original_content);
}

#[tokio::test]
async fn test_secure_document_deletion() {
    let (storage, temp_dir) = create_test_storage();
    let document_id = Uuid::new_v4();

    let content = b"Document to be securely deleted";
    
    // Store document
    storage.store_document(
        document_id,
        content,
        "to_delete.txt",
        "text/plain"
    ).await.unwrap();

    // Verify document exists
    assert!(storage.retrieve_document(document_id).await.is_ok());

    // Delete document
    let delete_result = storage.delete_document(document_id).await;
    assert!(delete_result.is_ok());

    // Verify document is gone
    let retrieval_result = storage.retrieve_document(document_id).await;
    assert!(retrieval_result.is_err());
    match retrieval_result.unwrap_err() {
        DocumentStorageError::DocumentNotFound { .. } => {
            // Expected error
        }
        _ => panic!("Expected DocumentNotFound error"),
    }

    // Verify directory is removed
    let document_dir = temp_dir.path().join(document_id.to_string());
    assert!(!document_dir.exists());
}

#[tokio::test]
async fn test_document_metadata_integrity() {
    let (storage, _temp_dir) = create_test_storage();
    let document_id = Uuid::new_v4();

    let content = b"Test content for metadata validation";
    
    // Store document
    storage.store_document(
        document_id,
        content,
        "metadata_test.txt",
        "text/plain"
    ).await.unwrap();

    // Get metadata without retrieving content
    let metadata_result = storage.get_document_metadata(document_id).await;
    assert!(metadata_result.is_ok());

    let metadata = metadata_result.unwrap();
    assert_eq!(metadata.filename, "metadata_test.txt");
    assert_eq!(metadata.content_type, "text/plain");
    assert_eq!(metadata.original_size, content.len());
    assert!(!metadata.file_hash.is_empty());
    assert!(!metadata.nonce.is_empty());
}

#[tokio::test]
async fn test_concurrent_document_operations() {
    let (storage, _temp_dir) = create_test_storage();
    
    // Create multiple documents concurrently
    let mut handles = Vec::new();
    
    for i in 0..10 {
        let storage_clone = storage.clone(); // Assuming we implement Clone for DocumentStorage
        let handle = tokio::spawn(async move {
            let document_id = Uuid::new_v4();
            let content = format!("Document content {}", i).into_bytes();
            
            storage_clone.store_document(
                document_id,
                &content,
                &format!("doc_{}.txt", i),
                "text/plain"
            ).await
        });
        handles.push(handle);
    }

    // Wait for all operations to complete
    let results: Vec<_> = futures::future::join_all(handles).await;
    
    // All operations should succeed
    for result in results {
        assert!(result.is_ok());
        assert!(result.unwrap().is_ok());
    }
}

#[tokio::test]
async fn test_document_hash_verification() {
    let (storage, _temp_dir) = create_test_storage();
    let document_id = Uuid::new_v4();

    let content = b"Content for hash verification test";
    
    // Store document
    let (_, original_hash) = storage.store_document(
        document_id,
        content,
        "hash_test.txt",
        "text/plain"
    ).await.unwrap();

    // Retrieve and verify hash matches
    let (retrieved_content, metadata) = storage.retrieve_document(document_id).await.unwrap();
    
    assert_eq!(retrieved_content, content);
    assert_eq!(metadata.file_hash, original_hash);
    
    // Calculate hash manually and verify
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(content);
    let expected_hash = format!("{:x}", hasher.finalize());
    
    assert_eq!(metadata.file_hash, expected_hash);
}

#[tokio::test]
async fn test_invalid_document_id() {
    let (storage, _temp_dir) = create_test_storage();
    let non_existent_id = Uuid::new_v4();

    // Try to retrieve non-existent document
    let result = storage.retrieve_document(non_existent_id).await;
    assert!(result.is_err());
    
    match result.unwrap_err() {
        DocumentStorageError::DocumentNotFound { id } => {
            assert_eq!(id, non_existent_id);
        }
        _ => panic!("Expected DocumentNotFound error"),
    }

    // Try to delete non-existent document (should not error)
    let delete_result = storage.delete_document(non_existent_id).await;
    assert!(delete_result.is_ok());
}
