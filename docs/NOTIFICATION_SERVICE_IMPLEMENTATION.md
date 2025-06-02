# Notification Service Implementation Guide

## Overview

The FO3 Wallet Core Notification Service provides comprehensive real-time notification management with WebSocket integration, user preferences, price alerts, and seamless integration with existing KYC and Fiat Gateway services. The implementation follows established FO3 Wallet Core patterns with JWT authentication, RBAC permissions, and comprehensive audit logging.

## 🏗️ Architecture

### Core Components

1. **Notification Service** (`fo3-wallet-api/src/services/notifications.rs`)
   - gRPC service implementation for all notification operations
   - Real-time WebSocket message delivery
   - User preference management
   - Price alert configuration and monitoring
   - Admin broadcast capabilities

2. **Data Models** (`fo3-wallet-api/src/models/notifications.rs`)
   - Notification entities and preferences
   - Price alert configurations
   - Delivery tracking and metrics
   - Repository trait for data access
   - In-memory repository implementation

3. **WebSocket Integration** (`fo3-wallet-api/src/websocket/mod.rs`)
   - Extended existing WebSocket manager
   - Real-time message delivery to connected users
   - Connection management and activity tracking

4. **Event Integration**
   - Seamless integration with KYC status changes
   - Fiat Gateway transaction notifications
   - Security event alerts
   - System announcements

## 🔧 Configuration

### Environment Variables

```bash
# WebSocket configuration
WEBSOCKET_MAX_CONNECTIONS=1000
WEBSOCKET_HEARTBEAT_INTERVAL=30

# Notification settings
NOTIFICATION_CLEANUP_INTERVAL=3600
NOTIFICATION_MAX_RETENTION_DAYS=30
NOTIFICATION_BATCH_SIZE=100
```

### Supported Notification Types

1. **Fiat Transaction** (`NOTIFICATION_TYPE_FIAT_TRANSACTION`)
   - Deposit completed/failed
   - Withdrawal approved/rejected
   - Transaction status updates

2. **KYC Status** (`NOTIFICATION_TYPE_KYC_STATUS`)
   - Verification approved/rejected
   - Documents required
   - Status updates

3. **Security** (`NOTIFICATION_TYPE_SECURITY`)
   - Login from new device
   - Password changes
   - API key management

4. **Price Alert** (`NOTIFICATION_TYPE_PRICE_ALERT`)
   - Price threshold notifications
   - Percentage change alerts
   - Custom user-defined alerts

5. **System** (`NOTIFICATION_TYPE_SYSTEM`)
   - Maintenance announcements
   - Feature updates
   - Service status

6. **Card** (`NOTIFICATION_TYPE_CARD`)
   - Virtual card issuance
   - Transaction notifications
   - Card status changes

7. **Reward** (`NOTIFICATION_TYPE_REWARD`)
   - Cashback earned
   - Referral bonuses
   - Deposit rewards

## 🚀 API Reference

### Core Notification Operations

#### Send Notification
```protobuf
rpc SendNotification(SendNotificationRequest) returns (SendNotificationResponse);
```

**Request:**
```json
{
  "user_id": "user123",
  "type": 1,
  "priority": 2,
  "title": "Transaction Complete",
  "message": "Your deposit of $100 has been processed",
  "metadata": {
    "transaction_id": "tx123",
    "amount": "100.00",
    "currency": "USD"
  },
  "channels": [1, 2],
  "expires_at": 1640995200,
  "action_url": "/transactions/tx123"
}
```

**Response:**
```json
{
  "notification": {
    "id": "notif123",
    "user_id": "user123",
    "type": 1,
    "priority": 2,
    "title": "Transaction Complete",
    "message": "Your deposit of $100 has been processed",
    "is_read": false,
    "created_at": 1640995200
  },
  "delivered": true,
  "failed_channels": []
}
```

#### Get User Notifications
```protobuf
rpc GetNotifications(GetNotificationsRequest) returns (GetNotificationsResponse);
```

#### Mark Notifications as Read
```protobuf
rpc MarkAsRead(MarkAsReadRequest) returns (MarkAsReadResponse);
```

### User Preference Management

#### Get Notification Preferences
```protobuf
rpc GetNotificationPreferences(GetNotificationPreferencesRequest) returns (GetNotificationPreferencesResponse);
```

#### Update Notification Preferences
```protobuf
rpc UpdateNotificationPreferences(UpdateNotificationPreferencesRequest) returns (UpdateNotificationPreferencesResponse);
```

**Example Preferences:**
```json
{
  "user_id": "user123",
  "fiat_transaction_enabled": true,
  "kyc_status_enabled": true,
  "security_alerts_enabled": true,
  "price_alerts_enabled": true,
  "system_announcements_enabled": false,
  "preferred_channels": [1, 2],
  "quiet_hours_enabled": true,
  "quiet_hours_start": 22,
  "quiet_hours_end": 8,
  "timezone": "UTC"
}
```

### Price Alert Management

#### Create Price Alert
```protobuf
rpc CreatePriceAlert(CreatePriceAlertRequest) returns (CreatePriceAlertResponse);
```

**Request:**
```json
{
  "user_id": "user123",
  "symbol": "BTC",
  "quote_currency": "USD",
  "condition": 1,
  "threshold_value": "50000.00",
  "is_repeating": false,
  "max_triggers": 1,
  "note": "BTC price target"
}
```

#### List Price Alerts
```protobuf
rpc ListPriceAlerts(ListPriceAlertsRequest) returns (ListPriceAlertsResponse);
```

#### Update/Delete Price Alerts
```protobuf
rpc UpdatePriceAlert(UpdatePriceAlertRequest) returns (UpdatePriceAlertResponse);
rpc DeletePriceAlert(DeletePriceAlertRequest) returns (DeletePriceAlertResponse);
```

### Admin Operations

#### Broadcast Notification (Admin Only)
```protobuf
rpc BroadcastNotification(BroadcastNotificationRequest) returns (BroadcastNotificationResponse);
```

#### Get Notification Metrics (Admin Only)
```protobuf
rpc GetNotificationMetrics(GetNotificationMetricsRequest) returns (GetNotificationMetricsResponse);
```

## 🔒 Security & Authentication

### Required Permissions

- **PERMISSION_NOTIFICATION_READ**: Read notifications and manage preferences
- **PERMISSION_NOTIFICATION_ADMIN**: Broadcast notifications and access metrics

### User Isolation

- Users can only access their own notifications and preferences
- Admin users can broadcast to all users and access system metrics
- Price alerts are user-specific and isolated

### Rate Limiting

- WebSocket message delivery: <100ms target
- Notification creation: Standard API rate limits apply
- Broadcast operations: Admin-only with audit logging

## ⚡ Performance Features

### Real-time Delivery

- **WebSocket Integration**: Immediate delivery to connected users
- **Fallback Storage**: In-app notifications for offline users
- **Delivery Tracking**: Success/failure metrics per channel
- **Connection Management**: Automatic cleanup of inactive connections

### Notification Preferences

- **Granular Control**: Per-type notification settings
- **Quiet Hours**: Time-based notification blocking
- **Channel Preferences**: WebSocket, in-app, email, SMS, push
- **Timezone Support**: User-specific time handling

### Price Alert System

- **Condition Types**: Above, below, percentage change
- **Trigger Management**: Single or repeating alerts
- **Expiration Support**: Time-based alert expiry
- **Integration**: Seamless connection with PricingService

## 📊 Integration with Existing Services

### KYC Service Integration

```rust
// Example: KYC status change notification
let event_data = NotificationEventData::KycStatus {
    submission_id: kyc_submission.id.clone(),
    status: "approved".to_string(),
    rejection_reason: None,
    required_documents: vec![],
};

let notification = notification_service
    .create_notification_from_event(&user_id, event_data)
    .await?;
```

### Fiat Gateway Integration

```rust
// Example: Transaction completion notification
let event_data = NotificationEventData::FiatTransaction {
    transaction_id: transaction.id.clone(),
    transaction_type: "deposit".to_string(),
    status: "completed".to_string(),
    amount: transaction.amount,
    currency: transaction.currency.clone(),
};
```

### PricingService Integration

- Automatic price alert monitoring
- Real-time price change notifications
- Configurable threshold alerts
- Integration with price data feeds

## 🧪 Testing

### Test Coverage

The implementation includes comprehensive tests:

- **Unit Tests**: Core notification logic and validation
- **Integration Tests**: End-to-end service operations
- **WebSocket Tests**: Real-time delivery functionality
- **Security Tests**: Authentication and authorization

### Running Tests

```bash
# Run all notification tests
cargo test notifications

# Run integration tests
cargo test --test notification_integration_tests

# Run with coverage
cargo tarpaulin --include-tests --out html
```

## 📈 Monitoring & Observability

### Metrics Tracked

- Total notifications sent
- Delivery success rates by channel
- Active WebSocket connections
- Price alert trigger counts
- User preference statistics
- Average delivery times

### Audit Logging

All notification operations are logged with:
- User ID and authentication context
- Operation type and parameters
- Delivery status and channels
- Timestamp and performance metrics

### Health Checks

- WebSocket connection health
- Notification delivery rates
- Repository availability
- Price alert monitoring status

## 🔄 Event-Driven Architecture

### Notification Event Creation

The service provides a flexible event system for creating notifications from various sources:

```rust
pub enum NotificationEventData {
    FiatTransaction { transaction_id, status, amount, currency },
    KycStatus { submission_id, status, rejection_reason },
    Security { event_type, ip_address, device_info },
    PriceAlert { symbol, current_price, threshold_price },
    System { announcement_type, severity, affected_services },
    Card { card_id, event_type, amount, merchant },
    Reward { reward_type, amount, currency },
}
```

### Integration Examples

```rust
// KYC service integration
notification_service.create_notification_from_event(
    &user_id,
    NotificationEventData::KycStatus {
        submission_id: submission.id,
        status: "approved".to_string(),
        rejection_reason: None,
        required_documents: vec![],
    }
).await?;

// Fiat gateway integration
notification_service.create_notification_from_event(
    &user_id,
    NotificationEventData::FiatTransaction {
        transaction_id: tx.id,
        transaction_type: "withdrawal".to_string(),
        status: "approved".to_string(),
        amount: tx.amount,
        currency: tx.currency,
    }
).await?;
```

## 🚀 Deployment

### Docker Configuration

The notification service is included in the main FO3 Wallet Core container:

```dockerfile
# Environment variables for notification service
ENV WEBSOCKET_MAX_CONNECTIONS=1000
ENV NOTIFICATION_CLEANUP_INTERVAL=3600
ENV NOTIFICATION_MAX_RETENTION_DAYS=30
```

### Production Considerations

1. **WebSocket Scaling**: Consider Redis pub/sub for multi-instance deployments
2. **Database Integration**: PostgreSQL for persistent notification storage
3. **Message Queuing**: Redis or RabbitMQ for reliable delivery
4. **Push Notifications**: Integration with FCM/APNS for mobile apps
5. **Email/SMS**: Integration with SendGrid/Twilio for additional channels

## 📈 Future Enhancements

### Planned Features

1. **Advanced Delivery Channels**: Email, SMS, mobile push notifications
2. **Template System**: Customizable notification templates
3. **Batch Operations**: Bulk notification management
4. **Analytics Dashboard**: User engagement and delivery metrics
5. **A/B Testing**: Notification content optimization
6. **Localization**: Multi-language notification support

### Scalability Improvements

1. **Message Queuing**: Redis/RabbitMQ for reliable delivery
2. **Database Sharding**: Horizontal scaling for large user bases
3. **CDN Integration**: Global notification delivery optimization
4. **Microservice Architecture**: Separate notification service deployment

This implementation provides a robust foundation for real-time notifications within the FO3 Wallet Core ecosystem, with enterprise-grade security, performance, and monitoring capabilities.
