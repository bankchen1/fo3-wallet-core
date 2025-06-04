# FO3 Wallet Core - Referral Service

## Overview

The Referral Service is a comprehensive referral program management system that enables users to refer friends, earn bonuses, and participate in multi-level referral campaigns. It provides fraud detection, analytics, and seamless integration with the rewards system.

## Features

### Core Functionality
- **Referral Code Generation**: Auto-generated and custom referral codes
- **Referral Relationships**: Track referrer-referee relationships
- **Campaign Management**: Create and manage referral campaigns
- **Bonus Processing**: Automated bonus calculation and distribution
- **Multi-level Referrals**: Support for hierarchical referral structures
- **Fraud Detection**: Advanced fraud prevention and suspicious activity flagging

### Security & Compliance
- **JWT Authentication**: Secure API access with role-based permissions
- **Rate Limiting**: Prevent abuse with configurable rate limits
- **Audit Trail**: Comprehensive logging of all referral activities
- **Data Validation**: Input validation and sanitization
- **Privacy Protection**: PII handling and data anonymization

### Analytics & Reporting
- **Real-time Metrics**: Conversion rates, bonus tracking, ROI analysis
- **User Analytics**: Individual referral performance and earnings
- **Campaign Analytics**: Campaign effectiveness and budget utilization
- **Fraud Metrics**: Suspicious activity detection and reporting

## Architecture

### Database Schema

#### Referral Codes
```sql
CREATE TABLE referral_codes (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL,
    code VARCHAR(50) UNIQUE NOT NULL,
    status VARCHAR(50) NOT NULL,
    campaign_id UUID,
    description TEXT,
    is_custom BOOLEAN DEFAULT false,
    max_uses INTEGER DEFAULT -1,
    current_uses INTEGER DEFAULT 0,
    successful_referrals INTEGER DEFAULT 0,
    pending_referrals INTEGER DEFAULT 0,
    expires_at TIMESTAMP WITH TIME ZONE,
    last_used_at TIMESTAMP WITH TIME ZONE,
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);
```

#### Referral Campaigns
```sql
CREATE TABLE referral_campaigns (
    id UUID PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    type VARCHAR(50) NOT NULL,
    status VARCHAR(50) NOT NULL,
    referrer_bonus DECIMAL(20, 8) NOT NULL,
    referee_bonus DECIMAL(20, 8) NOT NULL,
    bonus_currency VARCHAR(10) NOT NULL,
    minimum_transaction_amount DECIMAL(20, 8),
    is_multi_level BOOLEAN DEFAULT false,
    max_levels INTEGER DEFAULT 1,
    level_multipliers DECIMAL(5, 4)[] DEFAULT ARRAY[1.0000],
    start_date TIMESTAMP WITH TIME ZONE,
    end_date TIMESTAMP WITH TIME ZONE,
    bonus_expiry_days INTEGER DEFAULT 30,
    max_referrals_per_user INTEGER DEFAULT -1,
    max_total_referrals INTEGER DEFAULT -1,
    max_bonus_per_user DECIMAL(20, 8),
    total_budget DECIMAL(20, 8),
    budget_used DECIMAL(20, 8) DEFAULT 0,
    target_user_tiers TEXT[],
    target_countries TEXT[],
    excluded_users UUID[],
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    created_by UUID
);
```

#### Referral Relationships
```sql
CREATE TABLE referral_relationships (
    id UUID PRIMARY KEY,
    referrer_user_id UUID NOT NULL,
    referee_user_id UUID NOT NULL,
    referral_code_id UUID NOT NULL,
    campaign_id UUID,
    status VARCHAR(50) NOT NULL,
    referral_level INTEGER NOT NULL DEFAULT 1,
    parent_relationship_id UUID,
    signup_completed BOOLEAN DEFAULT false,
    first_transaction_completed BOOLEAN DEFAULT false,
    kyc_completed BOOLEAN DEFAULT false,
    first_transaction_date TIMESTAMP WITH TIME ZONE,
    kyc_completion_date TIMESTAMP WITH TIME ZONE,
    total_bonuses_earned DECIMAL(20, 8) DEFAULT 0,
    total_bonuses_paid DECIMAL(20, 8) DEFAULT 0,
    bonuses_pending INTEGER DEFAULT 0,
    is_suspicious BOOLEAN DEFAULT false,
    fraud_flags TEXT[],
    fraud_check_date TIMESTAMP WITH TIME ZONE,
    referral_source VARCHAR(50),
    ip_address INET,
    user_agent TEXT,
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    UNIQUE(referrer_user_id, referee_user_id),
    CHECK(referrer_user_id != referee_user_id)
);
```

#### Referral Bonuses
```sql
CREATE TABLE referral_bonuses (
    id UUID PRIMARY KEY,
    referral_relationship_id UUID NOT NULL,
    campaign_id UUID,
    user_id UUID NOT NULL,
    type VARCHAR(50) NOT NULL,
    status VARCHAR(50) NOT NULL,
    bonus_amount DECIMAL(20, 8) NOT NULL,
    bonus_currency VARCHAR(10) NOT NULL,
    exchange_rate DECIMAL(20, 8) DEFAULT 1.0000,
    milestone_type VARCHAR(50),
    milestone_value DECIMAL(20, 8),
    reward_transaction_id UUID,
    processing_fee DECIMAL(20, 8) DEFAULT 0,
    net_amount DECIMAL(20, 8) NOT NULL,
    earned_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    processed_at TIMESTAMP WITH TIME ZONE,
    expires_at TIMESTAMP WITH TIME ZONE,
    description TEXT,
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);
```

## API Reference

### Core Operations

#### Generate Referral Code
```protobuf
rpc GenerateReferralCode(GenerateReferralCodeRequest) returns (GenerateReferralCodeResponse);
```

**Request:**
```json
{
  "user_id": "uuid",
  "campaign_id": "uuid",
  "custom_code": "MYCUSTOMCODE",
  "description": "My referral code",
  "max_uses": 100,
  "expires_at": "2024-12-31T23:59:59Z",
  "metadata": {}
}
```

**Response:**
```json
{
  "referral_code": {
    "id": "uuid",
    "user_id": "uuid",
    "code": "FO3-ABCD1234-EFGH",
    "status": 1,
    "is_custom": false,
    "max_uses": 100,
    "current_uses": 0,
    "created_at": "2024-01-01T00:00:00Z"
  },
  "message": "Referral code generated successfully"
}
```

#### Validate Referral Code
```protobuf
rpc ValidateReferralCode(ValidateReferralCodeRequest) returns (ValidateReferralCodeResponse);
```

#### Create Referral Relationship
```protobuf
rpc CreateReferralRelationship(CreateReferralRelationshipRequest) returns (CreateReferralRelationshipResponse);
```

#### Process Referral Bonus
```protobuf
rpc ProcessReferralBonus(ProcessReferralBonusRequest) returns (ProcessReferralBonusResponse);
```

### Campaign Management

#### Create Referral Campaign
```protobuf
rpc CreateReferralCampaign(CreateReferralCampaignRequest) returns (CreateReferralCampaignResponse);
```

#### List Referral Campaigns
```protobuf
rpc ListReferralCampaigns(ListReferralCampaignsRequest) returns (ListReferralCampaignsResponse);
```

### Analytics

#### Get Referral Metrics
```protobuf
rpc GetReferralMetrics(GetReferralMetricsRequest) returns (GetReferralMetricsResponse);
```

#### Get User Referral Analytics
```protobuf
rpc GetUserReferralAnalytics(GetUserReferralAnalyticsRequest) returns (GetUserReferralAnalyticsResponse);
```

## Configuration

### Environment Variables

```bash
# Referral Service Configuration
REFERRAL_MAX_CODES_PER_USER=10
REFERRAL_DEFAULT_EXPIRY_DAYS=30
REFERRAL_FRAUD_CHECK_ENABLED=true
REFERRAL_RATE_LIMIT_ENABLED=true

# Campaign Configuration
REFERRAL_MAX_CAMPAIGN_BUDGET=1000000
REFERRAL_DEFAULT_BONUS_CURRENCY=points
REFERRAL_MAX_LEVELS=5

# Security Configuration
REFERRAL_REQUIRE_KYC=true
REFERRAL_MIN_TRANSACTION_AMOUNT=10.00
REFERRAL_FRAUD_THRESHOLD=0.05
```

### Rate Limits

| Operation | Limit | Window |
|-----------|-------|--------|
| Code Generation | 10 | 1 hour |
| Relationship Creation | 5 | 1 hour |
| Campaign Creation | 5 | 1 hour |
| Bonus Processing | 50 | 1 hour |
| Analytics Access | 30 | 1 hour |

## Security Features

### Fraud Detection

1. **Pattern Analysis**
   - Multiple referrals from same IP
   - Rapid referral creation
   - Suspicious user behavior

2. **Validation Checks**
   - Self-referral prevention
   - Duplicate relationship detection
   - Code expiration validation

3. **Risk Scoring**
   - User reputation scoring
   - Transaction pattern analysis
   - Geographic anomaly detection

### Access Control

1. **Permission Levels**
   - `VIEW_REFERRALS`: View own referral data
   - `MANAGE_REFERRALS`: Create codes and relationships
   - `ADMIN_REFERRALS`: Full campaign management
   - `VIEW_REPORTS`: Access analytics

2. **Data Protection**
   - PII encryption at rest
   - Secure data transmission
   - Audit trail logging

## Integration

### Rewards Service Integration
```rust
// Automatic bonus processing
let bonus_transaction = rewards_service.create_reward_transaction(
    user_id,
    RewardTransactionType::ReferralBonus,
    bonus_amount,
    "Referral bonus earned"
).await?;
```

### Notification Service Integration
```rust
// Real-time notifications
notification_service.send_notification(
    user_id,
    NotificationType::ReferralSuccess,
    "New Referral!",
    "Someone used your referral code!"
).await?;
```

### KYC Service Integration
```rust
// KYC milestone tracking
if kyc_service.is_user_verified(user_id).await? {
    referral_service.update_milestone(
        relationship_id,
        "kyc_completed"
    ).await?;
}
```

## Monitoring & Observability

### Metrics
- Referral conversion rates
- Bonus distribution amounts
- Campaign performance
- Fraud detection rates
- API response times

### Alerts
- High fraud detection rates
- Campaign budget exhaustion
- Unusual referral patterns
- System performance issues

### Dashboards
- Real-time referral activity
- Campaign performance metrics
- User engagement analytics
- Financial impact tracking

## Testing

### Unit Tests
```bash
cargo test referral_service
```

### Integration Tests
```bash
cargo test --test referral_integration
```

### Load Tests
```bash
# Test referral code generation under load
cargo test --release test_referral_load
```

## Deployment

### Docker Configuration
```dockerfile
FROM rust:1.70 as builder
WORKDIR /app
COPY . .
RUN cargo build --release --bin fo3-wallet-api

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates
COPY --from=builder /app/target/release/fo3-wallet-api /usr/local/bin/
EXPOSE 50051
CMD ["fo3-wallet-api"]
```

### Kubernetes Deployment
```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: fo3-referral-service
spec:
  replicas: 3
  selector:
    matchLabels:
      app: fo3-referral-service
  template:
    metadata:
      labels:
        app: fo3-referral-service
    spec:
      containers:
      - name: referral-service
        image: fo3/wallet-api:latest
        ports:
        - containerPort: 50051
        env:
        - name: DATABASE_URL
          valueFrom:
            secretKeyRef:
              name: database-secret
              key: url
```

## Best Practices

### Code Generation
- Use secure random generation
- Implement collision detection
- Set appropriate expiration times
- Monitor usage patterns

### Campaign Management
- Set realistic budgets
- Monitor conversion rates
- Implement A/B testing
- Track ROI metrics

### Fraud Prevention
- Implement multiple validation layers
- Monitor suspicious patterns
- Use machine learning for detection
- Maintain audit trails

### Performance Optimization
- Use database indexing
- Implement caching strategies
- Optimize query patterns
- Monitor response times
