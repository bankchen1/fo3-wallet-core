## DAppSigningService API

**Description:** Multi-chain transaction signing and simulation service  
**Methods:** 13 gRPC methods  
**Categories:** Signing Requests, Transaction Simulation, Batch Operations, Analytics

### Method Categories

#### Signing Requests

**`create_signing_request`**  
Create new signing request

- **Authentication:** Required (JWT+RBAC)
- **Rate Limit:** Service-specific limits apply
- **Response Time:** <200ms (standard) / <500ms (complex)
- **Audit Logging:** Comprehensive audit trail

**`approve_signing_request`**  
Approve and sign transaction

- **Authentication:** Required (JWT+RBAC)
- **Rate Limit:** Service-specific limits apply
- **Response Time:** <200ms (standard) / <500ms (complex)
- **Audit Logging:** Comprehensive audit trail

**`reject_signing_request`**  
Reject signing request

- **Authentication:** Required (JWT+RBAC)
- **Rate Limit:** Service-specific limits apply
- **Response Time:** <200ms (standard) / <500ms (complex)
- **Audit Logging:** Comprehensive audit trail

**`get_signing_request`**  
Get signing request details

- **Authentication:** Required (JWT+RBAC)
- **Rate Limit:** Service-specific limits apply
- **Response Time:** <200ms (standard) / <500ms (complex)
- **Audit Logging:** Comprehensive audit trail

**`list_signing_requests`**  
List signing requests

- **Authentication:** Required (JWT+RBAC)
- **Rate Limit:** Service-specific limits apply
- **Response Time:** <200ms (standard) / <500ms (complex)
- **Audit Logging:** Comprehensive audit trail

**`cancel_signing_request`**  
Cancel signing request

- **Authentication:** Required (JWT+RBAC)
- **Rate Limit:** Service-specific limits apply
- **Response Time:** <200ms (standard) / <500ms (complex)
- **Audit Logging:** Comprehensive audit trail


#### Transaction Simulation

**`simulate_transaction`**  
Simulate transaction execution

- **Authentication:** Required (JWT+RBAC)
- **Rate Limit:** Service-specific limits apply
- **Response Time:** <200ms (standard) / <500ms (complex)
- **Audit Logging:** Comprehensive audit trail

**`estimate_gas`**  
Estimate gas for transaction

- **Authentication:** Required (JWT+RBAC)
- **Rate Limit:** Service-specific limits apply
- **Response Time:** <200ms (standard) / <500ms (complex)
- **Audit Logging:** Comprehensive audit trail

**`get_transaction_status`**  
Get transaction status

- **Authentication:** Required (JWT+RBAC)
- **Rate Limit:** Service-specific limits apply
- **Response Time:** <200ms (standard) / <500ms (complex)
- **Audit Logging:** Comprehensive audit trail

**`get_supported_chains`**  
Get supported blockchain networks

- **Authentication:** Required (JWT+RBAC)
- **Rate Limit:** Service-specific limits apply
- **Response Time:** <200ms (standard) / <500ms (complex)
- **Audit Logging:** Comprehensive audit trail


#### Batch Operations

**`batch_sign_transactions`**  
Sign multiple transactions

- **Authentication:** Required (JWT+RBAC)
- **Rate Limit:** Service-specific limits apply
- **Response Time:** <200ms (standard) / <500ms (complex)
- **Audit Logging:** Comprehensive audit trail


#### Analytics

**`get_signing_analytics`**  
Get signing analytics

- **Authentication:** Required (JWT+RBAC)
- **Rate Limit:** Service-specific limits apply
- **Response Time:** <200ms (standard) / <500ms (complex)
- **Audit Logging:** Comprehensive audit trail


