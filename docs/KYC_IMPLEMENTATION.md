# KYC (Know Your Customer) Implementation Guide

## Overview

The FO3 Wallet Core KYC implementation provides comprehensive identity verification services with enterprise-grade security, compliance features, and seamless integration with the existing authentication and audit infrastructure.

## 🏗️ Architecture

### Core Components

1. **KYC Service** (`fo3-wallet-api/src/services/kyc.rs`)
   - gRPC service implementation
   - JWT authentication integration
   - RBAC permission enforcement
   - Comprehensive audit logging

2. **Data Models** (`fo3-wallet-api/src/models/kyc.rs`)
   - KYC submission entities
   - Document metadata structures
   - Status transition management

3. **Document Storage** (`fo3-wallet-api/src/storage/documents.rs`)
   - AES-256-GCM encryption
   - Secure file handling
   - Content validation
   - Secure deletion

4. **Security Middleware** (`fo3-wallet-api/src/middleware/kyc_guard.rs`)
   - KYC status enforcement
   - Transaction limit validation
   - Jurisdiction compliance checks

5. **PII Protection** (`fo3-wallet-api/src/models/pii_protection.rs`)
   - Data anonymization utilities
   - GDPR/CCPA compliance tools
   - Retention policy management

## 🔐 Security Features

### Document Encryption
- **Algorithm**: AES-256-GCM with random nonces
- **Key Management**: Environment-based key configuration
- **File Integrity**: SHA-256 hash verification
- **Secure Deletion**: DoD 5220.22-M standard (3-pass overwrite)

### Access Control
- **Authentication**: JWT token validation
- **Authorization**: RBAC with KYC-specific permissions
- **Audit Logging**: Comprehensive compliance trails
- **Cross-User Protection**: Users can only access their own data

### Content Validation
- **File Type Verification**: Magic number validation
- **Executable Detection**: Prevents malicious file uploads
- **Size Limits**: Configurable maximum file sizes
- **Content Type Matching**: Ensures file headers match declared types

## 📊 API Endpoints

### User Operations

#### Submit KYC
```protobuf
rpc SubmitKyc(SubmitKycRequest) returns (SubmitKycResponse);
```
- **Permission**: `PERMISSION_KYC_SUBMIT`
- **Description**: Submit identity information and documents
- **Validation**: Personal info validation, document verification

#### Get KYC Status
```protobuf
rpc GetKycStatus(GetKycStatusRequest) returns (GetKycStatusResponse);
```
- **Permission**: `PERMISSION_KYC_VIEW`
- **Description**: Retrieve current KYC verification status
- **Access Control**: Users can only view their own status

#### Update Documents
```protobuf
rpc UpdateKycDocuments(UpdateKycDocumentsRequest) returns (UpdateKycDocumentsResponse);
```
- **Permission**: `PERMISSION_KYC_SUBMIT`
- **Description**: Add or remove documents from submission
- **Restrictions**: Only allowed for pending/requires-update status

### Admin Operations

#### List Submissions
```protobuf
rpc ListKycSubmissions(ListKycSubmissionsRequest) returns (ListKycSubmissionsResponse);
```
- **Permission**: `PERMISSION_KYC_ADMIN`
- **Features**: Pagination, status filtering, wallet ID filtering

#### Approve KYC
```protobuf
rpc ApproveKyc(ApproveKycRequest) returns (ApproveKycResponse);
```
- **Permission**: `PERMISSION_KYC_ADMIN`
- **Audit**: Full approval trail with reviewer identification

#### Reject KYC
```protobuf
rpc RejectKyc(RejectKycRequest) returns (RejectKycResponse);
```
- **Permission**: `PERMISSION_KYC_ADMIN`
- **Requirements**: Rejection reason mandatory

### Document Operations

#### Upload Document
```protobuf
rpc UploadDocument(stream UploadDocumentRequest) returns (UploadDocumentResponse);
```
- **Features**: Streaming upload, encryption, validation
- **Security**: Content scanning, size limits, type verification

#### Download Document
```protobuf
rpc DownloadDocument(DownloadDocumentRequest) returns (stream DownloadDocumentResponse);
```
- **Permission**: `PERMISSION_KYC_ADMIN`
- **Features**: Streaming download, audit logging

## 🗃️ Database Schema

### KYC Submissions Table
```sql
CREATE TABLE kyc_submissions (
    id UUID PRIMARY KEY,
    wallet_id UUID UNIQUE REFERENCES wallets(id),
    status VARCHAR(50) NOT NULL,
    first_name VARCHAR(255) NOT NULL,
    last_name VARCHAR(255) NOT NULL,
    date_of_birth DATE NOT NULL,
    nationality VARCHAR(100) NOT NULL,
    country_of_residence VARCHAR(100) NOT NULL,
    -- Address fields
    street_address TEXT NOT NULL,
    city VARCHAR(255) NOT NULL,
    state_province VARCHAR(255),
    postal_code VARCHAR(50) NOT NULL,
    address_country VARCHAR(100) NOT NULL,
    -- Metadata
    submitted_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    reviewed_at TIMESTAMP WITH TIME ZONE,
    reviewer_id VARCHAR(255),
    reviewer_notes TEXT,
    rejection_reason TEXT,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);
```

### KYC Documents Table
```sql
CREATE TABLE kyc_documents (
    id UUID PRIMARY KEY,
    submission_id UUID REFERENCES kyc_submissions(id),
    document_type VARCHAR(50) NOT NULL,
    filename VARCHAR(255) NOT NULL,
    content_type VARCHAR(100) NOT NULL,
    size_bytes BIGINT NOT NULL,
    file_hash VARCHAR(64) NOT NULL,
    storage_path TEXT NOT NULL,
    is_encrypted BOOLEAN DEFAULT true,
    uploaded_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    deleted_at TIMESTAMP WITH TIME ZONE
);
```

## 🔄 Status Workflow

```
Pending → Under Review → Approved
    ↓           ↓           ↑
    ↓    Requires Update ←──┘
    ↓           ↓
    └─────→ Rejected
```

### Status Descriptions
- **Pending**: Initial submission, awaiting admin review
- **Under Review**: Admin is actively reviewing the submission
- **Approved**: Identity verified and approved
- **Rejected**: Submission rejected with reason
- **Requires Update**: Additional information needed

## 🛡️ Compliance Features

### GDPR/CCPA Compliance
- **Data Minimization**: Only collect necessary information
- **Purpose Limitation**: Clear purpose for each data collection
- **Retention Policies**: Automatic data deletion after retention period
- **Right to Access**: Users can view their data
- **Right to Rectification**: Users can update their information
- **Right to Erasure**: Secure data deletion capabilities

### Audit Trail
- **Comprehensive Logging**: All KYC operations logged
- **PII Protection**: Sensitive data masked in logs
- **Compliance Metadata**: Legal basis and purpose tracking
- **Retention Tracking**: Automatic compliance with retention policies

### Data Protection
- **Encryption at Rest**: All documents encrypted with AES-256-GCM
- **Encryption in Transit**: TLS 1.3 for all communications
- **Access Logging**: All data access events tracked
- **Secure Deletion**: DoD-standard secure file deletion

## 🧪 Testing

### Test Coverage
- **Unit Tests**: >95% coverage for core functionality
- **Integration Tests**: Document storage and encryption
- **E2E Tests**: Complete workflow validation
- **Security Tests**: Authorization and access control
- **Performance Tests**: Document upload benchmarks

### Running Tests
```bash
# Run all KYC tests
cargo test kyc

# Run integration tests
cargo test --test integration

# Run E2E tests
cargo test --test e2e kyc_tests

# Run with coverage
cargo tarpaulin --out Html
```

## 🚀 Deployment

### Environment Variables
```bash
# KYC Configuration
KYC_STORAGE_PATH=./data/kyc_documents
KYC_MAX_FILE_SIZE=10485760
KYC_ENCRYPTION_KEY=base64_encoded_32_byte_key

# Database
DATABASE_URL=postgresql://user:pass@localhost/fo3_wallet

# Security
JWT_SECRET=your_jwt_secret
```

### Docker Deployment
```bash
# Build and run with KYC support
docker-compose up -d

# Verify KYC service
docker-compose logs fo3-wallet-api | grep -i kyc
```

## 📈 Monitoring

### Metrics
- KYC submission rates
- Approval/rejection ratios
- Document upload performance
- Storage utilization
- Compliance audit events

### Grafana Dashboards
- KYC workflow metrics
- Document storage statistics
- Compliance monitoring
- Performance indicators

## 🔧 Configuration

### Permission Setup
```rust
// User permissions
PERMISSION_KYC_SUBMIT   // Submit own KYC
PERMISSION_KYC_VIEW     // View own KYC status

// Admin permissions
PERMISSION_KYC_ADMIN    // Review and approve KYC submissions
```

### Document Types
- `GOVERNMENT_ID`: Passport, driver's license, national ID
- `PROOF_OF_ADDRESS`: Utility bills, bank statements
- `SELFIE`: Photo with ID document
- `BANK_STATEMENT`: Financial verification
- `OTHER`: Additional supporting documents

## 🚨 Security Considerations

### Production Checklist
- [ ] Generate secure encryption keys
- [ ] Configure proper file permissions
- [ ] Set up secure backup procedures
- [ ] Implement key rotation policies
- [ ] Configure audit log retention
- [ ] Set up monitoring and alerting
- [ ] Review access control policies
- [ ] Test disaster recovery procedures

### Best Practices
1. **Key Management**: Use secure key storage (HSM/KMS)
2. **Access Control**: Implement least privilege principle
3. **Monitoring**: Set up real-time security monitoring
4. **Backup**: Encrypted backups with secure storage
5. **Compliance**: Regular compliance audits
6. **Training**: Staff training on data protection

## 📞 Support

For implementation questions or issues:
1. Check the test suite for usage examples
2. Review audit logs for troubleshooting
3. Consult the API documentation
4. Contact the development team

## 🔄 Future Enhancements

### Planned Features
- Automated document verification (OCR/AI)
- Biometric verification integration
- Enhanced risk scoring algorithms
- Multi-jurisdiction compliance automation
- Real-time fraud detection
- Advanced analytics and reporting
