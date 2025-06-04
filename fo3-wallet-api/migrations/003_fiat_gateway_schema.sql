-- Fiat Gateway Schema Migration
-- This migration creates tables for fiat banking operations

-- Fiat bank accounts table
CREATE TABLE IF NOT EXISTS fiat_accounts (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES wallets(id) ON DELETE CASCADE,
    provider VARCHAR(50) NOT NULL, -- 'ach', 'visa', 'paypal', 'wire', 'sepa'
    account_type VARCHAR(50) NOT NULL, -- 'checking', 'savings', 'credit_card', 'paypal'
    account_name VARCHAR(255) NOT NULL,
    encrypted_account_number TEXT NOT NULL, -- Encrypted full account number
    masked_account_number VARCHAR(20) NOT NULL, -- Last 4 digits for display
    routing_number VARCHAR(50), -- For ACH/wire transfers
    bank_name VARCHAR(255),
    currency VARCHAR(10) NOT NULL DEFAULT 'USD',
    country VARCHAR(10) NOT NULL,
    is_verified BOOLEAN DEFAULT false,
    is_primary BOOLEAN DEFAULT false,
    verification_method VARCHAR(50), -- 'micro_deposits', 'instant', 'manual'
    verification_data JSONB, -- Store verification-specific data
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    verified_at TIMESTAMP WITH TIME ZONE,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    deleted_at TIMESTAMP WITH TIME ZONE -- Soft delete for compliance
);

-- Fiat transactions table
CREATE TABLE IF NOT EXISTS fiat_transactions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES wallets(id) ON DELETE CASCADE,
    bank_account_id UUID REFERENCES fiat_accounts(id),
    transaction_type VARCHAR(20) NOT NULL, -- 'deposit', 'withdrawal'
    status VARCHAR(50) NOT NULL DEFAULT 'pending', -- 'pending', 'processing', 'completed', 'failed', 'cancelled', 'requires_approval', 'approved', 'rejected'
    amount DECIMAL(20, 8) NOT NULL, -- High precision for amounts
    currency VARCHAR(10) NOT NULL,
    fee_amount DECIMAL(20, 8) DEFAULT 0,
    net_amount DECIMAL(20, 8) NOT NULL, -- amount - fee_amount
    provider VARCHAR(50) NOT NULL,
    external_transaction_id VARCHAR(255), -- Provider's transaction ID
    reference_number VARCHAR(255), -- Human-readable reference
    description TEXT,
    failure_reason TEXT,
    approval_notes TEXT,
    approver_id VARCHAR(255), -- Admin who approved/rejected
    metadata JSONB, -- Additional provider-specific data
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    completed_at TIMESTAMP WITH TIME ZONE,
    expires_at TIMESTAMP WITH TIME ZONE -- For time-limited operations
);

-- Fiat transaction limits table
CREATE TABLE IF NOT EXISTS fiat_limits (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES wallets(id) ON DELETE CASCADE,
    currency VARCHAR(10) NOT NULL,
    daily_deposit_limit DECIMAL(20, 8) DEFAULT 10000.00,
    daily_withdrawal_limit DECIMAL(20, 8) DEFAULT 10000.00,
    monthly_deposit_limit DECIMAL(20, 8) DEFAULT 100000.00,
    monthly_withdrawal_limit DECIMAL(20, 8) DEFAULT 100000.00,
    single_transaction_limit DECIMAL(20, 8) DEFAULT 50000.00,
    requires_approval_above DECIMAL(20, 8) DEFAULT 10000.00,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_by VARCHAR(255), -- Admin who updated limits
    UNIQUE(user_id, currency)
);

-- Fiat Gateway indexes
CREATE INDEX IF NOT EXISTS idx_fiat_accounts_user_id ON fiat_accounts(user_id);
CREATE INDEX IF NOT EXISTS idx_fiat_accounts_provider ON fiat_accounts(provider);
CREATE INDEX IF NOT EXISTS idx_fiat_accounts_verified ON fiat_accounts(is_verified);
CREATE INDEX IF NOT EXISTS idx_fiat_accounts_primary ON fiat_accounts(is_primary);
CREATE INDEX IF NOT EXISTS idx_fiat_accounts_currency ON fiat_accounts(currency);
CREATE INDEX IF NOT EXISTS idx_fiat_transactions_user_id ON fiat_transactions(user_id);
CREATE INDEX IF NOT EXISTS idx_fiat_transactions_account_id ON fiat_transactions(bank_account_id);
CREATE INDEX IF NOT EXISTS idx_fiat_transactions_type ON fiat_transactions(transaction_type);
CREATE INDEX IF NOT EXISTS idx_fiat_transactions_status ON fiat_transactions(status);
CREATE INDEX IF NOT EXISTS idx_fiat_transactions_created_at ON fiat_transactions(created_at);
CREATE INDEX IF NOT EXISTS idx_fiat_transactions_external_id ON fiat_transactions(external_transaction_id);
CREATE INDEX IF NOT EXISTS idx_fiat_transactions_reference ON fiat_transactions(reference_number);
CREATE INDEX IF NOT EXISTS idx_fiat_limits_user_id ON fiat_limits(user_id);
CREATE INDEX IF NOT EXISTS idx_fiat_limits_currency ON fiat_limits(currency);
