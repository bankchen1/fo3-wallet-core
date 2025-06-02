-- FO3 Wallet Core Database Initialization

-- Create extension for UUID generation
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Wallets table
CREATE TABLE IF NOT EXISTS wallets (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(255) NOT NULL,
    encrypted_mnemonic TEXT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Addresses table
CREATE TABLE IF NOT EXISTS addresses (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    wallet_id UUID NOT NULL REFERENCES wallets(id) ON DELETE CASCADE,
    key_type VARCHAR(50) NOT NULL, -- 'ethereum', 'bitcoin', 'solana'
    address VARCHAR(255) NOT NULL,
    derivation_path VARCHAR(255) NOT NULL,
    bitcoin_network VARCHAR(50), -- 'mainnet', 'testnet', 'regtest' (only for bitcoin)
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    UNIQUE(wallet_id, key_type, derivation_path)
);

-- Transactions table
CREATE TABLE IF NOT EXISTS transactions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    wallet_id UUID NOT NULL REFERENCES wallets(id) ON DELETE CASCADE,
    key_type VARCHAR(50) NOT NULL,
    transaction_hash VARCHAR(255) NOT NULL,
    from_address VARCHAR(255) NOT NULL,
    to_address VARCHAR(255) NOT NULL,
    value DECIMAL(36, 18) NOT NULL,
    gas_price DECIMAL(36, 18),
    gas_limit DECIMAL(36, 18),
    gas_used DECIMAL(36, 18),
    fee DECIMAL(36, 18),
    nonce BIGINT,
    status VARCHAR(50) NOT NULL, -- 'pending', 'confirmed', 'failed'
    block_hash VARCHAR(255),
    block_number BIGINT,
    transaction_index INTEGER,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    UNIQUE(key_type, transaction_hash)
);

-- Token balances table
CREATE TABLE IF NOT EXISTS token_balances (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    wallet_id UUID NOT NULL REFERENCES wallets(id) ON DELETE CASCADE,
    address VARCHAR(255) NOT NULL,
    token_address VARCHAR(255) NOT NULL,
    token_symbol VARCHAR(50) NOT NULL,
    token_name VARCHAR(255) NOT NULL,
    token_decimals INTEGER NOT NULL,
    balance DECIMAL(36, 18) NOT NULL DEFAULT 0,
    key_type VARCHAR(50) NOT NULL,
    last_updated TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    UNIQUE(wallet_id, address, token_address)
);

-- DeFi positions table
CREATE TABLE IF NOT EXISTS defi_positions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    wallet_id UUID NOT NULL REFERENCES wallets(id) ON DELETE CASCADE,
    protocol VARCHAR(100) NOT NULL, -- 'uniswap', 'aave', 'compound', etc.
    position_type VARCHAR(50) NOT NULL, -- 'liquidity', 'lending', 'staking'
    token_address VARCHAR(255) NOT NULL,
    amount DECIMAL(36, 18) NOT NULL,
    key_type VARCHAR(50) NOT NULL,
    metadata JSONB, -- Additional protocol-specific data
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- NFTs table (primarily for Solana)
CREATE TABLE IF NOT EXISTS nfts (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    wallet_id UUID NOT NULL REFERENCES wallets(id) ON DELETE CASCADE,
    mint VARCHAR(255) NOT NULL,
    name VARCHAR(255),
    symbol VARCHAR(50),
    uri TEXT,
    owner VARCHAR(255) NOT NULL,
    is_mutable BOOLEAN DEFAULT true,
    seller_fee_basis_points INTEGER DEFAULT 0,
    creators JSONB, -- Array of creator objects
    metadata JSONB, -- NFT metadata
    key_type VARCHAR(50) NOT NULL DEFAULT 'solana',
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    UNIQUE(mint)
);

-- API keys table (for managing external service API keys)
CREATE TABLE IF NOT EXISTS api_keys (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    service_name VARCHAR(100) NOT NULL,
    key_name VARCHAR(100) NOT NULL,
    encrypted_key TEXT NOT NULL,
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    UNIQUE(service_name, key_name)
);

-- KYC submissions table
CREATE TABLE IF NOT EXISTS kyc_submissions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    wallet_id UUID NOT NULL REFERENCES wallets(id) ON DELETE CASCADE,
    status VARCHAR(50) NOT NULL DEFAULT 'pending', -- 'pending', 'under_review', 'approved', 'rejected', 'requires_update'
    first_name VARCHAR(255) NOT NULL,
    last_name VARCHAR(255) NOT NULL,
    date_of_birth DATE NOT NULL,
    nationality VARCHAR(100) NOT NULL,
    country_of_residence VARCHAR(100) NOT NULL,
    street_address TEXT NOT NULL,
    city VARCHAR(255) NOT NULL,
    state_province VARCHAR(255),
    postal_code VARCHAR(50) NOT NULL,
    address_country VARCHAR(100) NOT NULL,
    submitted_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    reviewed_at TIMESTAMP WITH TIME ZONE,
    reviewer_id VARCHAR(255), -- User ID of the reviewer
    reviewer_notes TEXT,
    rejection_reason TEXT,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    UNIQUE(wallet_id) -- One KYC submission per wallet
);

-- KYC documents table
CREATE TABLE IF NOT EXISTS kyc_documents (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    submission_id UUID NOT NULL REFERENCES kyc_submissions(id) ON DELETE CASCADE,
    document_type VARCHAR(50) NOT NULL, -- 'government_id', 'proof_of_address', 'selfie', 'bank_statement', 'other'
    filename VARCHAR(255) NOT NULL,
    content_type VARCHAR(100) NOT NULL,
    size_bytes BIGINT NOT NULL,
    file_hash VARCHAR(64) NOT NULL, -- SHA-256 hash for integrity
    storage_path TEXT NOT NULL, -- Path to encrypted file
    is_encrypted BOOLEAN DEFAULT true,
    uploaded_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    deleted_at TIMESTAMP WITH TIME ZONE -- Soft delete for compliance
);

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

-- Virtual cards table
CREATE TABLE IF NOT EXISTS virtual_cards (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES wallets(id) ON DELETE CASCADE,
    card_type VARCHAR(20) NOT NULL DEFAULT 'virtual', -- 'virtual', 'physical'
    status VARCHAR(20) NOT NULL DEFAULT 'pending', -- 'pending', 'active', 'frozen', 'expired', 'cancelled', 'blocked'
    encrypted_card_number TEXT NOT NULL, -- Encrypted full card number
    masked_number VARCHAR(20) NOT NULL, -- Masked display (****-****-****-1234)
    cardholder_name VARCHAR(255) NOT NULL,
    expiry_month VARCHAR(2) NOT NULL, -- MM
    expiry_year VARCHAR(2) NOT NULL, -- YY
    encrypted_cvv TEXT NOT NULL, -- Encrypted CVV
    encrypted_pin TEXT NOT NULL, -- Encrypted PIN
    currency VARCHAR(10) NOT NULL DEFAULT 'USD',
    balance DECIMAL(20, 8) NOT NULL DEFAULT 0,
    daily_limit DECIMAL(20, 8) NOT NULL DEFAULT 5000.00,
    monthly_limit DECIMAL(20, 8) NOT NULL DEFAULT 50000.00,
    per_transaction_limit DECIMAL(20, 8) NOT NULL DEFAULT 2500.00,
    atm_daily_limit DECIMAL(20, 8) NOT NULL DEFAULT 1000.00,
    transaction_count_daily INTEGER NOT NULL DEFAULT 50,
    transaction_count_monthly INTEGER NOT NULL DEFAULT 500,
    design_id VARCHAR(50) NOT NULL DEFAULT 'default',
    linked_account_id UUID REFERENCES fiat_accounts(id), -- Linked fiat account for funding
    is_primary BOOLEAN DEFAULT false,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    expires_at TIMESTAMP WITH TIME ZONE NOT NULL,
    frozen_at TIMESTAMP WITH TIME ZONE,
    frozen_reason TEXT
);

-- Card transactions table
CREATE TABLE IF NOT EXISTS card_transactions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    card_id UUID NOT NULL REFERENCES virtual_cards(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES wallets(id) ON DELETE CASCADE,
    transaction_type VARCHAR(20) NOT NULL, -- 'purchase', 'refund', 'authorization', 'top_up', 'fee'
    status VARCHAR(20) NOT NULL DEFAULT 'pending', -- 'pending', 'approved', 'declined', 'reversed', 'settled'
    amount DECIMAL(20, 8) NOT NULL,
    currency VARCHAR(10) NOT NULL,
    fee_amount DECIMAL(20, 8) DEFAULT 0,
    net_amount DECIMAL(20, 8) NOT NULL, -- amount + fee_amount
    merchant_name VARCHAR(255),
    merchant_category VARCHAR(100),
    merchant_category_code VARCHAR(20), -- 'grocery', 'restaurant', etc.
    merchant_location VARCHAR(255),
    merchant_country VARCHAR(10),
    merchant_mcc VARCHAR(4), -- Merchant Category Code (4-digit)
    description TEXT,
    reference_number VARCHAR(255) NOT NULL,
    authorization_code VARCHAR(50),
    metadata JSONB, -- Additional transaction data
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    authorized_at TIMESTAMP WITH TIME ZONE,
    settled_at TIMESTAMP WITH TIME ZONE,
    decline_reason TEXT
);

-- Spending budgets table
CREATE TABLE IF NOT EXISTS spending_budgets (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES wallets(id) ON DELETE CASCADE,
    category VARCHAR(100) NOT NULL, -- Category or "total" for overall budget
    amount DECIMAL(20, 8) NOT NULL,
    currency VARCHAR(10) NOT NULL DEFAULT 'USD',
    period VARCHAR(20) NOT NULL, -- 'daily', 'weekly', 'monthly', 'quarterly', 'yearly', 'custom'
    spent_amount DECIMAL(20, 8) NOT NULL DEFAULT 0,
    utilization DOUBLE PRECISION NOT NULL DEFAULT 0,
    status VARCHAR(20) NOT NULL DEFAULT 'on_track', -- 'on_track', 'warning', 'exceeded', 'critical'
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    period_start TIMESTAMP WITH TIME ZONE NOT NULL,
    period_end TIMESTAMP WITH TIME ZONE NOT NULL,
    alert_thresholds JSONB -- Array of alert threshold percentages
);

-- Spending alerts table
CREATE TABLE IF NOT EXISTS spending_alerts (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES wallets(id) ON DELETE CASCADE,
    alert_type VARCHAR(50) NOT NULL, -- 'budget_warning', 'budget_exceeded', 'unusual_spending', etc.
    title VARCHAR(255) NOT NULL,
    message TEXT NOT NULL,
    category VARCHAR(100), -- Category (if applicable)
    merchant VARCHAR(255), -- Merchant (if applicable)
    threshold_amount DECIMAL(20, 8), -- Threshold amount (if applicable)
    currency VARCHAR(10) NOT NULL DEFAULT 'USD',
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    triggered_at TIMESTAMP WITH TIME ZONE,
    metadata JSONB -- Additional alert data
);

-- Create indexes for better performance
CREATE INDEX IF NOT EXISTS idx_wallets_name ON wallets(name);
CREATE INDEX IF NOT EXISTS idx_addresses_wallet_id ON addresses(wallet_id);
CREATE INDEX IF NOT EXISTS idx_addresses_key_type ON addresses(key_type);
CREATE INDEX IF NOT EXISTS idx_addresses_address ON addresses(address);
CREATE INDEX IF NOT EXISTS idx_transactions_wallet_id ON transactions(wallet_id);
CREATE INDEX IF NOT EXISTS idx_transactions_hash ON transactions(transaction_hash);
CREATE INDEX IF NOT EXISTS idx_transactions_status ON transactions(status);
CREATE INDEX IF NOT EXISTS idx_transactions_created_at ON transactions(created_at);
CREATE INDEX IF NOT EXISTS idx_token_balances_wallet_id ON token_balances(wallet_id);
CREATE INDEX IF NOT EXISTS idx_token_balances_address ON token_balances(address);
CREATE INDEX IF NOT EXISTS idx_defi_positions_wallet_id ON defi_positions(wallet_id);
CREATE INDEX IF NOT EXISTS idx_defi_positions_protocol ON defi_positions(protocol);
CREATE INDEX IF NOT EXISTS idx_nfts_wallet_id ON nfts(wallet_id);
CREATE INDEX IF NOT EXISTS idx_nfts_owner ON nfts(owner);
CREATE INDEX IF NOT EXISTS idx_nfts_mint ON nfts(mint);

-- KYC indexes
CREATE INDEX IF NOT EXISTS idx_kyc_submissions_wallet_id ON kyc_submissions(wallet_id);
CREATE INDEX IF NOT EXISTS idx_kyc_submissions_status ON kyc_submissions(status);
CREATE INDEX IF NOT EXISTS idx_kyc_submissions_submitted_at ON kyc_submissions(submitted_at);
CREATE INDEX IF NOT EXISTS idx_kyc_submissions_reviewer_id ON kyc_submissions(reviewer_id);
CREATE INDEX IF NOT EXISTS idx_kyc_documents_submission_id ON kyc_documents(submission_id);
CREATE INDEX IF NOT EXISTS idx_kyc_documents_type ON kyc_documents(document_type);
CREATE INDEX IF NOT EXISTS idx_kyc_documents_hash ON kyc_documents(file_hash);

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

-- Card indexes
CREATE INDEX IF NOT EXISTS idx_virtual_cards_user_id ON virtual_cards(user_id);
CREATE INDEX IF NOT EXISTS idx_virtual_cards_status ON virtual_cards(status);
CREATE INDEX IF NOT EXISTS idx_virtual_cards_created_at ON virtual_cards(created_at);
CREATE INDEX IF NOT EXISTS idx_virtual_cards_linked_account ON virtual_cards(linked_account_id);
CREATE INDEX IF NOT EXISTS idx_virtual_cards_primary ON virtual_cards(is_primary);
CREATE INDEX IF NOT EXISTS idx_virtual_cards_currency ON virtual_cards(currency);
CREATE INDEX IF NOT EXISTS idx_card_transactions_card_id ON card_transactions(card_id);
CREATE INDEX IF NOT EXISTS idx_card_transactions_user_id ON card_transactions(user_id);
CREATE INDEX IF NOT EXISTS idx_card_transactions_status ON card_transactions(status);
CREATE INDEX IF NOT EXISTS idx_card_transactions_created_at ON card_transactions(created_at);
CREATE INDEX IF NOT EXISTS idx_card_transactions_type ON card_transactions(transaction_type);
CREATE INDEX IF NOT EXISTS idx_card_transactions_merchant_category ON card_transactions(merchant_category_code);
CREATE INDEX IF NOT EXISTS idx_card_transactions_reference ON card_transactions(reference_number);

-- Spending insights indexes
CREATE INDEX IF NOT EXISTS idx_spending_budgets_user_id ON spending_budgets(user_id);
CREATE INDEX IF NOT EXISTS idx_spending_budgets_category ON spending_budgets(category);
CREATE INDEX IF NOT EXISTS idx_spending_budgets_period ON spending_budgets(period);
CREATE INDEX IF NOT EXISTS idx_spending_budgets_status ON spending_budgets(status);
CREATE INDEX IF NOT EXISTS idx_spending_budgets_active ON spending_budgets(is_active);
CREATE INDEX IF NOT EXISTS idx_spending_budgets_period_range ON spending_budgets(period_start, period_end);
CREATE INDEX IF NOT EXISTS idx_spending_alerts_user_id ON spending_alerts(user_id);
CREATE INDEX IF NOT EXISTS idx_spending_alerts_type ON spending_alerts(alert_type);
CREATE INDEX IF NOT EXISTS idx_spending_alerts_active ON spending_alerts(is_active);
CREATE INDEX IF NOT EXISTS idx_spending_alerts_category ON spending_alerts(category);
CREATE INDEX IF NOT EXISTS idx_spending_alerts_merchant ON spending_alerts(merchant);
CREATE INDEX IF NOT EXISTS idx_spending_alerts_triggered ON spending_alerts(triggered_at);

-- Create updated_at trigger function
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

-- Create triggers for updated_at
CREATE TRIGGER update_wallets_updated_at BEFORE UPDATE ON wallets FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_transactions_updated_at BEFORE UPDATE ON transactions FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_defi_positions_updated_at BEFORE UPDATE ON defi_positions FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_nfts_updated_at BEFORE UPDATE ON nfts FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_api_keys_updated_at BEFORE UPDATE ON api_keys FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_kyc_submissions_updated_at BEFORE UPDATE ON kyc_submissions FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_fiat_accounts_updated_at BEFORE UPDATE ON fiat_accounts FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_fiat_transactions_updated_at BEFORE UPDATE ON fiat_transactions FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_fiat_limits_updated_at BEFORE UPDATE ON fiat_limits FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_virtual_cards_updated_at BEFORE UPDATE ON virtual_cards FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_spending_budgets_updated_at BEFORE UPDATE ON spending_budgets FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
