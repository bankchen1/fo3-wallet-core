# RewardsService Documentation

## Overview

The RewardsService is a comprehensive points-based reward system designed to enhance user engagement and retention in the FO3 Wallet Core platform. It provides a flexible framework for awarding, tracking, and redeeming points across various user activities and transactions.

## Architecture

### Core Components

1. **Reward Rules Engine**: Configurable rules for point allocation
2. **User Tier System**: Bronze, Silver, Gold, and Platinum tiers with multipliers
3. **Points Management**: Earning, tracking, and expiration handling
4. **Redemption System**: Multiple redemption options and processing
5. **Analytics & Reporting**: Comprehensive metrics and insights
6. **Audit Trail**: Complete transaction history and compliance tracking

### Security Features

- JWT + RBAC authentication and authorization
- Rate limiting for fraud prevention
- Suspicious pattern detection
- Comprehensive audit logging
- PII protection for sensitive operations

## API Endpoints

### Reward Rule Management

#### Create Reward Rule
```protobuf
rpc CreateRewardRule(CreateRewardRuleRequest) returns (CreateRewardRuleResponse);
```

Creates a new reward rule with configurable parameters:
- **Rule Types**: Transaction, Spending, Funding, Referral, Milestone, Promotional, Tier Bonus, Category
- **Time Constraints**: Start/end dates, days of week, time ranges
- **Usage Limits**: Per user, per day, per month limits
- **Tier Requirements**: Minimum user tier for eligibility

#### List Reward Rules
```protobuf
rpc ListRewardRules(ListRewardRulesRequest) returns (ListRewardRulesResponse);
```

Retrieves reward rules with filtering options:
- Filter by type, status, category, currency
- Pagination support
- Active-only filtering

### User Reward Operations

#### Get User Rewards
```protobuf
rpc GetUserRewards(GetUserRewardsRequest) returns (GetUserRewardsResponse);
```

Returns comprehensive user reward information:
- Total available points
- Lifetime earned/redeemed points
- Current tier and progress
- Expiring points tracking
- Tier benefits and multipliers

#### Get Reward Balance
```protobuf
rpc GetRewardBalance(GetRewardBalanceRequest) returns (GetRewardBalanceResponse);
```

Quick balance check with essential information:
- Total and pending points
- Current tier and multiplier
- Next expiration details

### Points Earning

#### Award Points
```protobuf
rpc AwardPoints(AwardPointsRequest) returns (AwardPointsResponse);
```

Awards points to users with:
- Automatic tier multiplier application
- Source tracking (transaction, referral, etc.)
- Expiration date setting
- Metadata support

#### Award Transaction Reward
```protobuf
rpc AwardTransactionReward(AwardTransactionRewardRequest) returns (AwardTransactionRewardResponse);
```

Specialized endpoint for transaction-based rewards:
- Automatic rule matching
- Category-based point calculation
- Multiple rule application support

### Points Redemption

#### Redeem Points
```protobuf
rpc RedeemPoints(RedeemPointsRequest) returns (RedeemPointsResponse);
```

Process point redemptions with:
- Balance validation
- Exchange rate calculation
- Processing fee handling
- Target account specification

#### Get Redemption Options
```protobuf
rpc GetRedemptionOptions(GetRedemptionOptionsRequest) returns (GetRedemptionOptionsResponse);
```

Lists available redemption options:
- Cash, credit, gift cards, merchandise
- Tier-based filtering
- Availability tracking

## User Tier System

### Tier Structure

| Tier | Threshold | Multiplier | Benefits |
|------|-----------|------------|----------|
| Bronze | 0 points | 1.0x | Basic support |
| Silver | 1,000 points | 1.5x | Priority support, monthly bonuses |
| Gold | 5,000 points | 2.0x | Premium support, weekly bonuses, exclusive redemptions |
| Platinum | 25,000 points | 3.0x | VIP support, daily bonuses, early access, personal manager |

### Automatic Tier Progression

- Tiers are automatically upgraded based on lifetime earned points
- Tier benefits are immediately applied
- Multipliers affect all future point earnings
- Tier upgrade notifications are sent in real-time

## Reward Rule Types

### Transaction Rewards
- Points per transaction count
- Minimum transaction amount requirements
- Category-specific multipliers

### Spending Rewards
- Points per dollar spent
- Merchant category bonuses
- Time-based promotions

### Funding Rewards
- Crypto funding bonuses
- First-time funding rewards
- Volume-based incentives

### Referral Rewards
- Signup bonuses for referrer and referee
- Milestone-based referral rewards
- Tier-based referral multipliers

### Promotional Rewards
- Time-limited campaigns
- Special event bonuses
- Seasonal promotions

## Security & Fraud Prevention

### Rate Limiting
- Points awarding: 100 requests/hour per user
- Redemptions: 10 requests/hour per user
- Rule creation: 10 requests/hour per admin

### Pattern Detection
- Unusual award frequency monitoring
- Large redemption alerts
- Cross-user pattern analysis
- Velocity checks

### Audit Trail
- Complete transaction history
- User action tracking
- IP address logging
- Reason code requirements

## Integration Points

### LedgerService Integration
- Double-entry bookkeeping for all reward transactions
- Real-time balance reconciliation
- Immutable transaction records

### CardService Integration
- Automatic transaction reward processing
- Real-time point calculation
- Card-specific bonus categories

### NotificationService Integration
- Real-time reward notifications
- Tier upgrade alerts
- Expiration warnings
- Redemption confirmations

### CardFundingService Integration
- Funding activity rewards
- Crypto funding bonuses
- Fee-based incentives

## Analytics & Reporting

### Reward Metrics
- Total points awarded/redeemed/expired
- User tier distribution
- Category performance analysis
- Redemption pattern insights

### User Analytics
- Individual reward history
- Category breakdown
- Tier progression tracking
- Projected earnings

### Administrative Reports
- Compliance reports
- Fraud detection summaries
- Performance metrics
- Export capabilities

## Configuration Examples

### Basic Transaction Reward Rule
```json
{
  "name": "Standard Transaction Reward",
  "type": "SPENDING",
  "points_per_unit": "1.0",
  "minimum_amount": "10.0",
  "categories": ["grocery", "restaurant"],
  "currencies": ["USD"],
  "minimum_tier": "BRONZE"
}
```

### Promotional Campaign
```json
{
  "name": "Holiday Bonus",
  "type": "PROMOTIONAL",
  "points_per_unit": "2.0",
  "start_date": "2024-12-01T00:00:00Z",
  "end_date": "2024-12-31T23:59:59Z",
  "max_uses_per_user": 50,
  "categories": ["retail", "entertainment"]
}
```

### Referral Bonus
```json
{
  "name": "Friend Referral",
  "type": "REFERRAL",
  "points_per_unit": "500.0",
  "max_uses_per_user": 10,
  "minimum_tier": "BRONZE"
}
```

## Error Handling

### Common Error Codes
- `INVALID_ARGUMENT`: Invalid request parameters
- `PERMISSION_DENIED`: Insufficient permissions
- `NOT_FOUND`: Resource not found
- `FAILED_PRECONDITION`: Business rule violation
- `RESOURCE_EXHAUSTED`: Rate limit exceeded

### Validation Rules
- Points amounts must be positive
- Tier requirements must be valid
- Date ranges must be logical
- Usage limits must be reasonable

## Performance Considerations

### Response Time Targets
- Point queries: <50ms
- Point awarding: <200ms
- Redemption processing: <500ms
- Analytics queries: <1000ms

### Scalability Features
- In-memory caching for frequent queries
- Batch processing for bulk operations
- Asynchronous notification delivery
- Efficient database indexing

## Monitoring & Observability

### Key Metrics
- Points awarded per second
- Redemption success rate
- Tier upgrade frequency
- Fraud detection alerts

### Health Checks
- Repository connectivity
- Cache performance
- External service integration
- Rate limiter status

## Future Enhancements

### Planned Features
- Machine learning-based fraud detection
- Dynamic tier thresholds
- Gamification elements
- Social sharing rewards
- Partner merchant integration
- Mobile app push notifications

### API Versioning
- Backward compatibility maintenance
- Gradual feature rollout
- Deprecation notices
- Migration guides
