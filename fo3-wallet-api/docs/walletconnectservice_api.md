## WalletConnectService API

**Description:** WalletConnect v2 protocol implementation for DApp connections  
**Methods:** 13 gRPC methods  
**Categories:** Session Management, Request Handling, Analytics, Maintenance

### Method Categories

#### Session Management

**`create_session`**  
Create new WalletConnect session

- **Authentication:** Required (JWT+RBAC)
- **Rate Limit:** Service-specific limits apply
- **Response Time:** <200ms (standard) / <500ms (complex)
- **Audit Logging:** Comprehensive audit trail

**`approve_session`**  
Approve WalletConnect session

- **Authentication:** Required (JWT+RBAC)
- **Rate Limit:** Service-specific limits apply
- **Response Time:** <200ms (standard) / <500ms (complex)
- **Audit Logging:** Comprehensive audit trail

**`reject_session`**  
Reject WalletConnect session

- **Authentication:** Required (JWT+RBAC)
- **Rate Limit:** Service-specific limits apply
- **Response Time:** <200ms (standard) / <500ms (complex)
- **Audit Logging:** Comprehensive audit trail

**`disconnect_session`**  
Disconnect WalletConnect session

- **Authentication:** Required (JWT+RBAC)
- **Rate Limit:** Service-specific limits apply
- **Response Time:** <200ms (standard) / <500ms (complex)
- **Audit Logging:** Comprehensive audit trail

**`get_session`**  
Get session details

- **Authentication:** Required (JWT+RBAC)
- **Rate Limit:** Service-specific limits apply
- **Response Time:** <200ms (standard) / <500ms (complex)
- **Audit Logging:** Comprehensive audit trail

**`list_sessions`**  
List user sessions

- **Authentication:** Required (JWT+RBAC)
- **Rate Limit:** Service-specific limits apply
- **Response Time:** <200ms (standard) / <500ms (complex)
- **Audit Logging:** Comprehensive audit trail

**`update_session`**  
Update session configuration

- **Authentication:** Required (JWT+RBAC)
- **Rate Limit:** Service-specific limits apply
- **Response Time:** <200ms (standard) / <500ms (complex)
- **Audit Logging:** Comprehensive audit trail


#### Request Handling

**`send_session_event`**  
Send event to DApp

- **Authentication:** Required (JWT+RBAC)
- **Rate Limit:** Service-specific limits apply
- **Response Time:** <200ms (standard) / <500ms (complex)
- **Audit Logging:** Comprehensive audit trail

**`get_session_requests`**  
Get pending session requests

- **Authentication:** Required (JWT+RBAC)
- **Rate Limit:** Service-specific limits apply
- **Response Time:** <200ms (standard) / <500ms (complex)
- **Audit Logging:** Comprehensive audit trail

**`respond_to_request`**  
Respond to DApp request

- **Authentication:** Required (JWT+RBAC)
- **Rate Limit:** Service-specific limits apply
- **Response Time:** <200ms (standard) / <500ms (complex)
- **Audit Logging:** Comprehensive audit trail


#### Analytics

**`get_session_analytics`**  
Get session analytics

- **Authentication:** Required (JWT+RBAC)
- **Rate Limit:** Service-specific limits apply
- **Response Time:** <200ms (standard) / <500ms (complex)
- **Audit Logging:** Comprehensive audit trail


#### Maintenance

**`cleanup_expired_sessions`**  
Clean up expired sessions

- **Authentication:** Required (JWT+RBAC)
- **Rate Limit:** Service-specific limits apply
- **Response Time:** <200ms (standard) / <500ms (complex)
- **Audit Logging:** Comprehensive audit trail


