# FO3 Wallet Core API Reference

**Version:** Phase 3 (Production Ready)  
**Protocol:** gRPC  
**Total Services:** 16 services with 58+ methods  
**Authentication:** JWT + RBAC  

## 🚀 Overview

FO3 Wallet Core provides a comprehensive gRPC API for enterprise-grade DeFi infrastructure, including wallet management, DeFi yield aggregation, multi-chain transaction signing, and community-driven token discovery.

## 📋 Service Index

| Service | Methods | Status | Description |
|---------|---------|--------|-------------|
| [AuthService](#authservice) | 6 | ✅ Complete | Authentication and authorization |
| [WalletService](#walletservice) | 8 | ✅ Complete | Wallet management and operations |
| [TransactionService](#transactionservice) | 6 | ✅ Complete | Transaction processing |
| [DeFiService](#defiservice) | 5 | ✅ Complete | DeFi protocol interactions |
| [KYCService](#kycservice) | 7 | ✅ Complete | Know Your Customer compliance |
| [FiatGatewayService](#fiatgatewayservice) | 8 | ✅ Complete | Fiat currency operations |
| [PricingService](#pricingservice) | 4 | ✅ Complete | Real-time price data |
| [NotificationService](#notificationservice) | 5 | ✅ Complete | Real-time notifications |
| [CardService](#cardservice) | 9 | ✅ Complete | Virtual card management |
| [SpendingInsightsService](#spendinginsightsservice) | 6 | ✅ Complete | Spending analytics |
| [CardFundingService](#cardfundingservice) | 8 | ✅ Complete | Card funding operations |
| [LedgerService](#ledgerservice) | 12 | ✅ Complete | Double-entry bookkeeping |
| [RewardsService](#rewardsservice) | 10 | ✅ Complete | Rewards and loyalty program |
| [ReferralService](#referralservice) | 15 | ✅ Complete | Referral system |
| [EarnService](#earnservice) | 22 | ✅ Complete | DeFi yield aggregation |
| [WalletConnectService](#walletconnectservice) | 13 | ✅ Complete | WalletConnect v2 integration |
| [DAppSigningService](#dappsigningservice) | 13 | ✅ Complete | Multi-chain transaction signing |
| [MoonshotTradingService](#moonshottrading) | 10 | 🆕 New | Community token discovery |

## 🔐 Authentication

All API calls require JWT authentication via the `Authorization` header:

```
Authorization: Bearer <jwt_token>
```

### Getting an Access Token

```bash
grpcurl -plaintext \
  -d '{"email": "user@example.com", "password": "password123"}' \
  localhost:50051 fo3.wallet.v1.AuthService/Login
```

## 🌐 Base URL and Endpoints

- **gRPC Endpoint:** `localhost:50051` (development) / `api.fo3wallet.com:50051` (production)
- **WebSocket:** `localhost:8080` (development) / `ws.fo3wallet.com:8080` (production)
- **Metrics:** `localhost:9090` (development) / `metrics.fo3wallet.com:9090` (production)

## 📊 Rate Limits

| Operation Category | Limit | Window | Description |
|-------------------|-------|--------|-------------|
| **Authentication** | 10/minute | 1 minute | Login, logout, token refresh |
| **Wallet Operations** | 100/hour | 1 hour | Balance, transactions, addresses |
| **DeFi Operations** | 50/hour | 1 hour | Staking, lending, yield farming |
| **Card Operations** | 200/hour | 1 hour | Card management and transactions |
| **KYC Operations** | 20/hour | 1 hour | Document upload and verification |
| **Fiat Gateway** | 30/hour | 1 hour | Deposits and withdrawals |
| **Pricing Data** | 1000/hour | 1 hour | Price feeds and market data |
| **Notifications** | 500/hour | 1 hour | Push notifications and alerts |
| **Analytics** | 200/hour | 1 hour | Spending insights and reports |
| **Rewards** | 100/hour | 1 hour | Points, redemptions, tiers |
| **Referrals** | 50/hour | 1 hour | Referral management |
| **Earn Service** | 100/hour | 1 hour | Yield products and staking |
| **WalletConnect** | 100/hour | 1 hour | DApp connections |
| **Transaction Signing** | 200/hour | 1 hour | Multi-chain signing |
| **Moonshot Trading** | 100/hour | 1 hour | Token discovery and voting |

## 🔧 Service Details

### AuthService

Authentication and user management service.

**Methods:**
- `Login` - User authentication
- `Logout` - Session termination
- `RefreshToken` - Token renewal
- `Register` - User registration
- `ChangePassword` - Password update
- `ResetPassword` - Password reset

**Example:**
```bash
# Login
grpcurl -plaintext \
  -d '{"email": "user@example.com", "password": "password123"}' \
  localhost:50051 fo3.wallet.v1.AuthService/Login

# Response
{
  "accessToken": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "refreshToken": "refresh_token_here",
  "expiresIn": 3600,
  "user": {
    "userId": "user-123",
    "email": "user@example.com",
    "role": "USER"
  }
}
```

### WalletService

Core wallet management functionality.

**Methods:**
- `CreateWallet` - Create new wallet
- `GetWallet` - Retrieve wallet details
- `ListWallets` - List user wallets
- `GetBalance` - Get wallet balance
- `GetTransactionHistory` - Transaction history
- `GenerateAddress` - Generate new address
- `ImportWallet` - Import existing wallet
- `ExportWallet` - Export wallet data

**Example:**
```bash
# Get wallet balance
grpcurl -plaintext \
  -H "Authorization: Bearer <token>" \
  -d '{"walletId": "wallet-123", "currency": "ETH"}' \
  localhost:50051 fo3.wallet.v1.WalletService/GetBalance

# Response
{
  "balance": "1.234567890123456789",
  "currency": "ETH",
  "usdValue": "2468.90",
  "lastUpdated": "2024-01-15T10:30:00Z"
}
```

### EarnService

DeFi yield aggregation and portfolio management.

**Methods:**
- `GetYieldProducts` - List available yield products
- `GetProductDetails` - Detailed product information
- `DepositToProduct` - Deposit funds to yield product
- `WithdrawFromProduct` - Withdraw funds
- `GetUserPositions` - User's active positions
- `GetEarningsHistory` - Historical earnings
- `GetStakingProducts` - Staking opportunities
- `StakeTokens` - Stake tokens
- `UnstakeTokens` - Unstake tokens
- `ClaimStakingRewards` - Claim rewards
- `GetLendingProducts` - Lending opportunities
- `SupplyToLending` - Supply tokens for lending
- `WithdrawFromLending` - Withdraw supplied tokens
- `GetVaultProducts` - Vault products
- `DepositToVault` - Deposit to vault
- `WithdrawFromVault` - Withdraw from vault
- `GetPortfolioSummary` - Portfolio overview
- `GetEarnAnalytics` - Analytics and insights
- `GetRiskAssessment` - Risk analysis
- `OptimizePortfolio` - Portfolio optimization
- `GetYieldPredictions` - Yield predictions
- `GetProductComparisons` - Compare products

**Example:**
```bash
# Get yield products
grpcurl -plaintext \
  -H "Authorization: Bearer <token>" \
  -d '{"category": "DEFI", "minApy": 5.0, "maxRisk": "MEDIUM"}' \
  localhost:50051 fo3.wallet.v1.EarnService/GetYieldProducts

# Response
{
  "products": [
    {
      "productId": "product-123",
      "name": "ETH Staking",
      "description": "Ethereum 2.0 staking",
      "apy": "6.5",
      "category": "STAKING",
      "riskLevel": "LOW",
      "minDeposit": "0.1",
      "currency": "ETH",
      "tvl": "1000000.0"
    }
  ],
  "totalCount": 25,
  "page": 1,
  "hasNextPage": true
}
```

### MoonshotTradingService

Community-driven token discovery and trading platform.

**Methods:**
- `GetTrendingTokens` - Real-time trending tokens
- `SubmitTokenProposal` - Submit new token proposal
- `VoteOnToken` - Vote on token proposals
- `GetTokenRankings` - Token rankings
- `GetMoonshotAnalytics` - Platform analytics
- `GetUserVotingHistory` - User voting history
- `GetTokenDetails` - Detailed token information
- `GetUserProposals` - User's proposals
- `GetTokenSentiment` - Sentiment analysis
- `GetTokenPredictions` - Price predictions

**Example:**
```bash
# Get trending tokens
grpcurl -plaintext \
  -H "Authorization: Bearer <token>" \
  -d '{"page": 1, "pageSize": 10, "timeFrame": "24h", "sortBy": "volume"}' \
  localhost:50051 fo3.wallet.v1.MoonshotTradingService/GetTrendingTokens

# Response
{
  "tokens": [
    {
      "tokenId": "token-123",
      "symbol": "MOON",
      "name": "MoonCoin",
      "description": "Community-driven moon mission token",
      "contractAddress": "0x1234...5678",
      "blockchain": "ethereum",
      "metrics": {
        "currentPrice": "0.001234",
        "marketCap": "1000000.00",
        "volume24h": "100000.00",
        "priceChangePercentage24h": "+15.67",
        "communityScore": 4.2,
        "totalVotes": 156
      }
    }
  ],
  "totalCount": 1250,
  "page": 1,
  "hasNextPage": true
}

# Submit token proposal
grpcurl -plaintext \
  -H "Authorization: Bearer <token>" \
  -d '{
    "userId": "user-123",
    "symbol": "NEWTOKEN",
    "name": "New Test Token",
    "description": "Revolutionary new DeFi token",
    "contractAddress": "0x1234567890123456789012345678901234567890",
    "blockchain": "ethereum",
    "justification": "This token brings innovative features to DeFi"
  }' \
  localhost:50051 fo3.wallet.v1.MoonshotTradingService/SubmitTokenProposal

# Response
{
  "proposalId": "proposal-456",
  "status": "VOTING",
  "votingEndsAt": "2024-01-22T10:30:00Z",
  "message": "Token proposal submitted successfully. Voting period: 7 days."
}
```

## 🔄 WebSocket Events

Real-time events via WebSocket connection:

```javascript
// Connect to WebSocket
const ws = new WebSocket('ws://localhost:8080');

// Subscribe to events
ws.send(JSON.stringify({
  type: 'subscribe',
  events: ['transaction_confirmed', 'price_alert', 'yield_update']
}));

// Handle events
ws.onmessage = (event) => {
  const data = JSON.parse(event.data);
  console.log('Event:', data);
};
```

## 📈 Error Handling

All gRPC methods return standard gRPC status codes:

| Code | Status | Description |
|------|--------|-------------|
| 0 | OK | Success |
| 3 | INVALID_ARGUMENT | Invalid request parameters |
| 7 | PERMISSION_DENIED | Insufficient permissions |
| 8 | RESOURCE_EXHAUSTED | Rate limit exceeded |
| 14 | UNAVAILABLE | Service temporarily unavailable |
| 16 | UNAUTHENTICATED | Authentication required |

**Error Response Example:**
```json
{
  "code": 3,
  "message": "Invalid wallet ID format",
  "details": [
    {
      "field": "walletId",
      "error": "must be a valid UUID"
    }
  ]
}
```

## 🧪 Testing with grpcurl

Install grpcurl for testing:
```bash
# Install grpcurl
go install github.com/fullstorydev/grpcurl/cmd/grpcurl@latest

# List all services
grpcurl -plaintext localhost:50051 list

# List methods for a service
grpcurl -plaintext localhost:50051 list fo3.wallet.v1.WalletService

# Describe a method
grpcurl -plaintext localhost:50051 describe fo3.wallet.v1.WalletService.GetBalance
```

## 📚 Additional Resources

- [Development Setup Guide](./DEVELOPMENT_SETUP.md)
- [Mobile Integration Guide](./MOBILE_INTEGRATION.md)
- [Deployment Guide](./DEPLOYMENT_GUIDE.md)
- [Service Implementation Guides](./services/)
- [Phase 2D Completion Report](./PHASE2D_COMPLETION_REPORT.md)

## 🆘 Support

For API support and questions:
- GitHub Issues: [fo3-wallet-core/issues](https://github.com/bankchen1/fo3-wallet-core/issues)
- Documentation: [docs/](./services/)
- Email: support@fo3wallet.com
