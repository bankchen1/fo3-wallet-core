## EarnService API

**Description:** DeFi yield aggregation and portfolio management service  
**Methods:** 22 gRPC methods  
**Categories:** Yield Products, Staking Operations, Lending Operations, Vault Operations, Analytics & Reporting, Risk & Optimization

### Method Categories

#### Yield Products

**`get_yield_products`**  
List available yield products with filtering and pagination

- **Authentication:** Required (JWT+RBAC)
- **Rate Limit:** Service-specific limits apply
- **Response Time:** <200ms (standard) / <500ms (complex)
- **Audit Logging:** Comprehensive audit trail

**`get_yield_product`**  
Get detailed information about a specific yield product

- **Authentication:** Required (JWT+RBAC)
- **Rate Limit:** Service-specific limits apply
- **Response Time:** <200ms (standard) / <500ms (complex)
- **Audit Logging:** Comprehensive audit trail

**`calculate_yield`**  
Calculate projected yield for a given investment

- **Authentication:** Required (JWT+RBAC)
- **Rate Limit:** Service-specific limits apply
- **Response Time:** <200ms (standard) / <500ms (complex)
- **Audit Logging:** Comprehensive audit trail

**`get_yield_history`**  
Get historical yield data for a product

- **Authentication:** Required (JWT+RBAC)
- **Rate Limit:** Service-specific limits apply
- **Response Time:** <200ms (standard) / <500ms (complex)
- **Audit Logging:** Comprehensive audit trail


#### Staking Operations

**`stake_tokens`**  
Stake tokens to earn rewards

- **Authentication:** Required (JWT+RBAC)
- **Rate Limit:** Service-specific limits apply
- **Response Time:** <200ms (standard) / <500ms (complex)
- **Audit Logging:** Comprehensive audit trail

**`unstake_tokens`**  
Unstake tokens and claim rewards

- **Authentication:** Required (JWT+RBAC)
- **Rate Limit:** Service-specific limits apply
- **Response Time:** <200ms (standard) / <500ms (complex)
- **Audit Logging:** Comprehensive audit trail

**`get_staking_positions`**  
List user staking positions

- **Authentication:** Required (JWT+RBAC)
- **Rate Limit:** Service-specific limits apply
- **Response Time:** <200ms (standard) / <500ms (complex)
- **Audit Logging:** Comprehensive audit trail

**`claim_rewards`**  
Claim staking rewards

- **Authentication:** Required (JWT+RBAC)
- **Rate Limit:** Service-specific limits apply
- **Response Time:** <200ms (standard) / <500ms (complex)
- **Audit Logging:** Comprehensive audit trail


#### Lending Operations

**`supply_tokens`**  
Supply tokens to lending protocols

- **Authentication:** Required (JWT+RBAC)
- **Rate Limit:** Service-specific limits apply
- **Response Time:** <200ms (standard) / <500ms (complex)
- **Audit Logging:** Comprehensive audit trail

**`withdraw_tokens`**  
Withdraw supplied tokens

- **Authentication:** Required (JWT+RBAC)
- **Rate Limit:** Service-specific limits apply
- **Response Time:** <200ms (standard) / <500ms (complex)
- **Audit Logging:** Comprehensive audit trail

**`get_lending_positions`**  
List user lending positions

- **Authentication:** Required (JWT+RBAC)
- **Rate Limit:** Service-specific limits apply
- **Response Time:** <200ms (standard) / <500ms (complex)
- **Audit Logging:** Comprehensive audit trail


#### Vault Operations

**`deposit_to_vault`**  
Deposit tokens to yield vaults

- **Authentication:** Required (JWT+RBAC)
- **Rate Limit:** Service-specific limits apply
- **Response Time:** <200ms (standard) / <500ms (complex)
- **Audit Logging:** Comprehensive audit trail

**`withdraw_from_vault`**  
Withdraw tokens from yield vaults

- **Authentication:** Required (JWT+RBAC)
- **Rate Limit:** Service-specific limits apply
- **Response Time:** <200ms (standard) / <500ms (complex)
- **Audit Logging:** Comprehensive audit trail

**`get_vault_positions`**  
List user vault positions

- **Authentication:** Required (JWT+RBAC)
- **Rate Limit:** Service-specific limits apply
- **Response Time:** <200ms (standard) / <500ms (complex)
- **Audit Logging:** Comprehensive audit trail


#### Analytics & Reporting

**`get_earn_analytics`**  
Get comprehensive earning analytics

- **Authentication:** Required (JWT+RBAC)
- **Rate Limit:** Service-specific limits apply
- **Response Time:** <200ms (standard) / <500ms (complex)
- **Audit Logging:** Comprehensive audit trail

**`get_portfolio_summary`**  
Get portfolio overview and summary

- **Authentication:** Required (JWT+RBAC)
- **Rate Limit:** Service-specific limits apply
- **Response Time:** <200ms (standard) / <500ms (complex)
- **Audit Logging:** Comprehensive audit trail

**`get_yield_chart`**  
Get yield performance charts

- **Authentication:** Required (JWT+RBAC)
- **Rate Limit:** Service-specific limits apply
- **Response Time:** <200ms (standard) / <500ms (complex)
- **Audit Logging:** Comprehensive audit trail


#### Risk & Optimization

**`assess_risk`**  
Assess portfolio risk factors

- **Authentication:** Required (JWT+RBAC)
- **Rate Limit:** Service-specific limits apply
- **Response Time:** <200ms (standard) / <500ms (complex)
- **Audit Logging:** Comprehensive audit trail

**`optimize_portfolio`**  
Get portfolio optimization suggestions

- **Authentication:** Required (JWT+RBAC)
- **Rate Limit:** Service-specific limits apply
- **Response Time:** <200ms (standard) / <500ms (complex)
- **Audit Logging:** Comprehensive audit trail


