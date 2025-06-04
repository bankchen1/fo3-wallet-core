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

-- Card funding sources table
CREATE TABLE IF NOT EXISTS card_funding_sources (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES wallets(id) ON DELETE CASCADE,
    type VARCHAR(50) NOT NULL, -- 'bank_account', 'crypto_wallet', 'ach', 'external_card', 'fiat_account'
    status VARCHAR(50) NOT NULL DEFAULT 'pending', -- 'pending', 'active', 'suspended', 'expired', 'removed'
    name VARCHAR(255) NOT NULL, -- User-defined name
    masked_identifier VARCHAR(100) NOT NULL, -- Masked account/card number
    currency VARCHAR(10) NOT NULL DEFAULT 'USD',
    provider VARCHAR(100) NOT NULL, -- Provider name (bank, exchange, etc.)
    is_primary BOOLEAN DEFAULT false,
    is_verified BOOLEAN DEFAULT false,
    daily_limit DECIMAL(20, 8) NOT NULL DEFAULT 10000.00,
    monthly_limit DECIMAL(20, 8) NOT NULL DEFAULT 100000.00,
    per_transaction_limit DECIMAL(20, 8) NOT NULL DEFAULT 5000.00,
    minimum_amount DECIMAL(20, 8) NOT NULL DEFAULT 10.00,
    daily_transaction_count INTEGER NOT NULL DEFAULT 20,
    monthly_transaction_count INTEGER NOT NULL DEFAULT 200,
    metadata JSONB NOT NULL DEFAULT '{}', -- Type-specific metadata
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    expires_at TIMESTAMP WITH TIME ZONE, -- For cards
    verification_url TEXT, -- Verification URL if needed
    external_id VARCHAR(255) -- External provider ID
);

-- Card funding transactions table
CREATE TABLE IF NOT EXISTS card_funding_transactions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES wallets(id) ON DELETE CASCADE,
    card_id UUID NOT NULL REFERENCES virtual_cards(id) ON DELETE CASCADE,
    funding_source_id UUID NOT NULL REFERENCES card_funding_sources(id) ON DELETE CASCADE,
    status VARCHAR(50) NOT NULL DEFAULT 'pending', -- 'pending', 'processing', 'completed', 'failed', 'cancelled', 'refunded'
    amount DECIMAL(20, 8) NOT NULL,
    currency VARCHAR(10) NOT NULL,
    fee_amount DECIMAL(20, 8) NOT NULL DEFAULT 0,
    fee_percentage DECIMAL(5, 4) NOT NULL DEFAULT 0, -- Fee percentage (e.g., 0.025 for 2.5%)
    exchange_rate DECIMAL(20, 8), -- Exchange rate for crypto funding
    net_amount DECIMAL(20, 8) NOT NULL, -- Amount after fees
    reference_number VARCHAR(255) NOT NULL UNIQUE,
    external_transaction_id VARCHAR(255), -- External provider transaction ID
    description TEXT,
    failure_reason TEXT,
    metadata JSONB DEFAULT '{}', -- Additional transaction data
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    completed_at TIMESTAMP WITH TIME ZONE,
    expires_at TIMESTAMP WITH TIME ZONE -- For crypto funding
);

-- Card funding limits table (user-specific limits)
CREATE TABLE IF NOT EXISTS card_funding_limits (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES wallets(id) ON DELETE CASCADE,
    daily_limit DECIMAL(20, 8) NOT NULL DEFAULT 25000.00,
    monthly_limit DECIMAL(20, 8) NOT NULL DEFAULT 250000.00,
    yearly_limit DECIMAL(20, 8) NOT NULL DEFAULT 1000000.00,
    per_transaction_limit DECIMAL(20, 8) NOT NULL DEFAULT 10000.00,
    daily_used DECIMAL(20, 8) NOT NULL DEFAULT 0,
    monthly_used DECIMAL(20, 8) NOT NULL DEFAULT 0,
    yearly_used DECIMAL(20, 8) NOT NULL DEFAULT 0,
    daily_transaction_count INTEGER NOT NULL DEFAULT 50,
    monthly_transaction_count INTEGER NOT NULL DEFAULT 500,
    daily_transactions_used INTEGER NOT NULL DEFAULT 0,
    monthly_transactions_used INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    last_reset_daily TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    last_reset_monthly TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    last_reset_yearly TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    UNIQUE(user_id)
);

-- Ledger accounts table (chart of accounts)
CREATE TABLE IF NOT EXISTS ledger_accounts (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    account_code VARCHAR(20) NOT NULL UNIQUE, -- Account code (e.g., "1001", "2001")
    account_name VARCHAR(255) NOT NULL, -- Human-readable account name
    account_type VARCHAR(50) NOT NULL, -- 'asset', 'liability', 'equity', 'revenue', 'expense', etc.
    status VARCHAR(50) NOT NULL DEFAULT 'active', -- 'active', 'inactive', 'closed', 'suspended'
    currency VARCHAR(10) NOT NULL DEFAULT 'USD',
    parent_account_id UUID REFERENCES ledger_accounts(id), -- For hierarchical accounts
    description TEXT,
    is_system_account BOOLEAN DEFAULT false, -- System-managed account
    allow_manual_entries BOOLEAN DEFAULT true, -- Allow manual journal entries
    current_balance DECIMAL(20, 8) NOT NULL DEFAULT 0, -- Current posted balance
    pending_balance DECIMAL(20, 8) NOT NULL DEFAULT 0, -- Balance including pending
    metadata JSONB DEFAULT '{}', -- Additional account metadata
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    closed_at TIMESTAMP WITH TIME ZONE -- Account closure timestamp
);

-- Ledger transactions table (transaction headers)
CREATE TABLE IF NOT EXISTS ledger_transactions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    reference_number VARCHAR(255) NOT NULL UNIQUE, -- External reference number
    status VARCHAR(50) NOT NULL DEFAULT 'pending', -- 'pending', 'posted', 'reversed', 'failed'
    transaction_type VARCHAR(100) NOT NULL, -- 'card_payment', 'funding', 'transfer', etc.
    description TEXT NOT NULL,
    currency VARCHAR(10) NOT NULL DEFAULT 'USD',
    total_amount DECIMAL(20, 8) NOT NULL, -- Total transaction amount
    source_service VARCHAR(100), -- Originating service (CardService, etc.)
    source_transaction_id VARCHAR(255), -- Original transaction ID from source
    metadata JSONB DEFAULT '{}', -- Additional transaction metadata
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    posted_at TIMESTAMP WITH TIME ZONE, -- When transaction was posted
    reversed_at TIMESTAMP WITH TIME ZONE, -- When transaction was reversed
    reversal_reason TEXT, -- Reason for reversal
    reversal_transaction_id UUID REFERENCES ledger_transactions(id) -- Link to reversal transaction
);

-- Journal entries table (individual debit/credit entries)
CREATE TABLE IF NOT EXISTS journal_entries (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    transaction_id UUID NOT NULL REFERENCES ledger_transactions(id) ON DELETE CASCADE,
    account_id UUID NOT NULL REFERENCES ledger_accounts(id),
    entry_type VARCHAR(10) NOT NULL, -- 'debit' or 'credit'
    amount DECIMAL(20, 8) NOT NULL, -- Entry amount (always positive)
    currency VARCHAR(10) NOT NULL DEFAULT 'USD',
    description TEXT NOT NULL,
    status VARCHAR(50) NOT NULL DEFAULT 'draft', -- 'draft', 'posted', 'reversed'
    entry_sequence INTEGER NOT NULL, -- Sequence within transaction
    metadata JSONB DEFAULT '{}', -- Additional entry metadata
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    posted_at TIMESTAMP WITH TIME ZONE -- When entry was posted
);

-- Ledger audit trail table
CREATE TABLE IF NOT EXISTS ledger_audit_trail (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    transaction_id UUID REFERENCES ledger_transactions(id),
    account_id UUID REFERENCES ledger_accounts(id),
    action VARCHAR(100) NOT NULL, -- Action performed
    old_value TEXT, -- Previous value (JSON)
    new_value TEXT, -- New value (JSON)
    user_id UUID REFERENCES wallets(id),
    ip_address INET,
    user_agent TEXT,
    metadata JSONB DEFAULT '{}', -- Additional audit data
    timestamp TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Account balances snapshot table (for performance and historical tracking)
CREATE TABLE IF NOT EXISTS account_balance_snapshots (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    account_id UUID NOT NULL REFERENCES ledger_accounts(id),
    balance_date DATE NOT NULL,
    opening_balance DECIMAL(20, 8) NOT NULL DEFAULT 0,
    closing_balance DECIMAL(20, 8) NOT NULL DEFAULT 0,
    debit_total DECIMAL(20, 8) NOT NULL DEFAULT 0,
    credit_total DECIMAL(20, 8) NOT NULL DEFAULT 0,
    transaction_count INTEGER NOT NULL DEFAULT 0,
    currency VARCHAR(10) NOT NULL DEFAULT 'USD',
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    UNIQUE(account_id, balance_date)
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

-- Card funding indexes
CREATE INDEX IF NOT EXISTS idx_card_funding_sources_user_id ON card_funding_sources(user_id);
CREATE INDEX IF NOT EXISTS idx_card_funding_sources_type ON card_funding_sources(type);
CREATE INDEX IF NOT EXISTS idx_card_funding_sources_status ON card_funding_sources(status);
CREATE INDEX IF NOT EXISTS idx_card_funding_sources_primary ON card_funding_sources(is_primary);
CREATE INDEX IF NOT EXISTS idx_card_funding_sources_verified ON card_funding_sources(is_verified);
CREATE INDEX IF NOT EXISTS idx_card_funding_sources_provider ON card_funding_sources(provider);
CREATE INDEX IF NOT EXISTS idx_card_funding_sources_currency ON card_funding_sources(currency);
CREATE INDEX IF NOT EXISTS idx_card_funding_sources_created_at ON card_funding_sources(created_at);
CREATE INDEX IF NOT EXISTS idx_card_funding_sources_external_id ON card_funding_sources(external_id);

CREATE INDEX IF NOT EXISTS idx_card_funding_transactions_user_id ON card_funding_transactions(user_id);
CREATE INDEX IF NOT EXISTS idx_card_funding_transactions_card_id ON card_funding_transactions(card_id);
CREATE INDEX IF NOT EXISTS idx_card_funding_transactions_source_id ON card_funding_transactions(funding_source_id);
CREATE INDEX IF NOT EXISTS idx_card_funding_transactions_status ON card_funding_transactions(status);
CREATE INDEX IF NOT EXISTS idx_card_funding_transactions_created_at ON card_funding_transactions(created_at);
CREATE INDEX IF NOT EXISTS idx_card_funding_transactions_completed_at ON card_funding_transactions(completed_at);
CREATE INDEX IF NOT EXISTS idx_card_funding_transactions_reference ON card_funding_transactions(reference_number);
CREATE INDEX IF NOT EXISTS idx_card_funding_transactions_external_id ON card_funding_transactions(external_transaction_id);
CREATE INDEX IF NOT EXISTS idx_card_funding_transactions_currency ON card_funding_transactions(currency);

CREATE INDEX IF NOT EXISTS idx_card_funding_limits_user_id ON card_funding_limits(user_id);
CREATE INDEX IF NOT EXISTS idx_card_funding_limits_daily_reset ON card_funding_limits(last_reset_daily);
CREATE INDEX IF NOT EXISTS idx_card_funding_limits_monthly_reset ON card_funding_limits(last_reset_monthly);
CREATE INDEX IF NOT EXISTS idx_card_funding_limits_yearly_reset ON card_funding_limits(last_reset_yearly);

-- Ledger indexes
CREATE INDEX IF NOT EXISTS idx_ledger_accounts_code ON ledger_accounts(account_code);
CREATE INDEX IF NOT EXISTS idx_ledger_accounts_type ON ledger_accounts(account_type);
CREATE INDEX IF NOT EXISTS idx_ledger_accounts_status ON ledger_accounts(status);
CREATE INDEX IF NOT EXISTS idx_ledger_accounts_currency ON ledger_accounts(currency);
CREATE INDEX IF NOT EXISTS idx_ledger_accounts_parent ON ledger_accounts(parent_account_id);
CREATE INDEX IF NOT EXISTS idx_ledger_accounts_system ON ledger_accounts(is_system_account);
CREATE INDEX IF NOT EXISTS idx_ledger_accounts_created_at ON ledger_accounts(created_at);

CREATE INDEX IF NOT EXISTS idx_ledger_transactions_reference ON ledger_transactions(reference_number);
CREATE INDEX IF NOT EXISTS idx_ledger_transactions_status ON ledger_transactions(status);
CREATE INDEX IF NOT EXISTS idx_ledger_transactions_type ON ledger_transactions(transaction_type);
CREATE INDEX IF NOT EXISTS idx_ledger_transactions_currency ON ledger_transactions(currency);
CREATE INDEX IF NOT EXISTS idx_ledger_transactions_source_service ON ledger_transactions(source_service);
CREATE INDEX IF NOT EXISTS idx_ledger_transactions_source_id ON ledger_transactions(source_transaction_id);
CREATE INDEX IF NOT EXISTS idx_ledger_transactions_created_at ON ledger_transactions(created_at);
CREATE INDEX IF NOT EXISTS idx_ledger_transactions_posted_at ON ledger_transactions(posted_at);
CREATE INDEX IF NOT EXISTS idx_ledger_transactions_reversal ON ledger_transactions(reversal_transaction_id);

CREATE INDEX IF NOT EXISTS idx_journal_entries_transaction_id ON journal_entries(transaction_id);
CREATE INDEX IF NOT EXISTS idx_journal_entries_account_id ON journal_entries(account_id);
CREATE INDEX IF NOT EXISTS idx_journal_entries_type ON journal_entries(entry_type);
CREATE INDEX IF NOT EXISTS idx_journal_entries_status ON journal_entries(status);
CREATE INDEX IF NOT EXISTS idx_journal_entries_currency ON journal_entries(currency);
CREATE INDEX IF NOT EXISTS idx_journal_entries_created_at ON journal_entries(created_at);
CREATE INDEX IF NOT EXISTS idx_journal_entries_posted_at ON journal_entries(posted_at);
CREATE INDEX IF NOT EXISTS idx_journal_entries_sequence ON journal_entries(transaction_id, entry_sequence);

CREATE INDEX IF NOT EXISTS idx_ledger_audit_trail_transaction_id ON ledger_audit_trail(transaction_id);
CREATE INDEX IF NOT EXISTS idx_ledger_audit_trail_account_id ON ledger_audit_trail(account_id);
CREATE INDEX IF NOT EXISTS idx_ledger_audit_trail_action ON ledger_audit_trail(action);
CREATE INDEX IF NOT EXISTS idx_ledger_audit_trail_user_id ON ledger_audit_trail(user_id);
CREATE INDEX IF NOT EXISTS idx_ledger_audit_trail_timestamp ON ledger_audit_trail(timestamp);
CREATE INDEX IF NOT EXISTS idx_ledger_audit_trail_ip ON ledger_audit_trail(ip_address);

CREATE INDEX IF NOT EXISTS idx_account_balance_snapshots_account_id ON account_balance_snapshots(account_id);
CREATE INDEX IF NOT EXISTS idx_account_balance_snapshots_date ON account_balance_snapshots(balance_date);
CREATE INDEX IF NOT EXISTS idx_account_balance_snapshots_currency ON account_balance_snapshots(currency);
CREATE INDEX IF NOT EXISTS idx_account_balance_snapshots_created_at ON account_balance_snapshots(created_at);

-- Reward rules table
CREATE TABLE IF NOT EXISTS reward_rules (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(255) NOT NULL,
    description TEXT,
    type VARCHAR(50) NOT NULL, -- 'transaction', 'spending', 'funding', 'referral', 'milestone', 'promotional', 'tier_bonus', 'category'
    status VARCHAR(50) NOT NULL DEFAULT 'active', -- 'active', 'inactive', 'expired', 'suspended'

    -- Rule configuration
    points_per_unit DECIMAL(20, 8) NOT NULL DEFAULT 0, -- Points awarded per unit
    minimum_amount DECIMAL(20, 8) DEFAULT 0, -- Minimum transaction amount
    maximum_points DECIMAL(20, 8), -- Maximum points per transaction
    minimum_tier VARCHAR(50) DEFAULT 'bronze', -- 'bronze', 'silver', 'gold', 'platinum'
    categories TEXT[], -- Applicable merchant categories
    currencies TEXT[] DEFAULT ARRAY['USD'], -- Applicable currencies

    -- Time constraints
    start_date TIMESTAMP WITH TIME ZONE,
    end_date TIMESTAMP WITH TIME ZONE,
    days_of_week INTEGER[], -- 0=Sunday, 1=Monday, etc.
    start_time TIME, -- HH:MM format
    end_time TIME, -- HH:MM format

    -- Usage limits
    max_uses_per_user INTEGER DEFAULT -1, -- -1 for unlimited
    max_uses_per_day INTEGER DEFAULT -1,
    max_uses_per_month INTEGER DEFAULT -1,
    total_uses_remaining INTEGER DEFAULT -1,

    -- Metadata
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    created_by UUID REFERENCES wallets(id)
);

-- User rewards table
CREATE TABLE IF NOT EXISTS user_rewards (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES wallets(id) ON DELETE CASCADE,
    total_points DECIMAL(20, 8) NOT NULL DEFAULT 0, -- Total available points
    lifetime_earned DECIMAL(20, 8) NOT NULL DEFAULT 0, -- Lifetime points earned
    lifetime_redeemed DECIMAL(20, 8) NOT NULL DEFAULT 0, -- Lifetime points redeemed
    pending_points DECIMAL(20, 8) NOT NULL DEFAULT 0, -- Pending points
    expiring_points DECIMAL(20, 8) NOT NULL DEFAULT 0, -- Points expiring soon
    current_tier VARCHAR(50) NOT NULL DEFAULT 'bronze', -- 'bronze', 'silver', 'gold', 'platinum'
    tier_progress DECIMAL(20, 8) NOT NULL DEFAULT 0, -- Progress to next tier
    next_tier_threshold DECIMAL(20, 8) NOT NULL DEFAULT 1000, -- Points needed for next tier

    -- Tier benefits
    tier_multiplier DECIMAL(5, 4) NOT NULL DEFAULT 1.0000, -- Current tier multiplier
    tier_benefits TEXT[], -- List of tier-specific benefits

    -- Expiration tracking
    next_expiration_date TIMESTAMP WITH TIME ZONE,
    next_expiration_amount DECIMAL(20, 8) DEFAULT 0,

    -- Metadata
    last_activity_date TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    tier_upgrade_date TIMESTAMP WITH TIME ZONE,
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    UNIQUE(user_id)
);

-- Reward transactions table
CREATE TABLE IF NOT EXISTS reward_transactions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES wallets(id) ON DELETE CASCADE,
    type VARCHAR(50) NOT NULL, -- 'earned', 'redeemed', 'expired', 'adjusted', 'bonus', 'penalty'
    status VARCHAR(50) NOT NULL DEFAULT 'pending', -- 'pending', 'completed', 'failed', 'cancelled', 'expired'

    -- Transaction details
    points DECIMAL(20, 8) NOT NULL, -- Points amount (positive or negative)
    multiplier DECIMAL(5, 4) NOT NULL DEFAULT 1.0000, -- Tier multiplier applied
    base_points DECIMAL(20, 8) NOT NULL DEFAULT 0, -- Base points before multiplier
    currency VARCHAR(10) NOT NULL DEFAULT 'USD',
    exchange_rate DECIMAL(20, 8), -- Points-to-currency rate

    -- Source information
    source_type VARCHAR(100), -- 'transaction', 'referral', 'milestone', etc.
    source_id VARCHAR(255), -- ID of source transaction/event
    reward_rule_id UUID REFERENCES reward_rules(id),
    reference_number VARCHAR(255) NOT NULL UNIQUE,

    -- Expiration
    expires_at TIMESTAMP WITH TIME ZONE,
    is_expired BOOLEAN DEFAULT false,

    -- Metadata
    description TEXT,
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Redemptions table
CREATE TABLE IF NOT EXISTS redemptions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES wallets(id) ON DELETE CASCADE,
    type VARCHAR(50) NOT NULL, -- 'cash', 'credit', 'gift_card', 'merchandise', 'discount', 'charity'
    status VARCHAR(50) NOT NULL DEFAULT 'pending', -- 'pending', 'processing', 'completed', 'failed', 'cancelled', 'expired'

    -- Redemption details
    points_redeemed DECIMAL(20, 8) NOT NULL, -- Points used
    cash_value DECIMAL(20, 8) NOT NULL, -- Cash equivalent
    currency VARCHAR(10) NOT NULL DEFAULT 'USD',
    exchange_rate DECIMAL(20, 8) NOT NULL, -- Points-to-cash rate

    -- Redemption target
    target_account VARCHAR(255), -- Account for cash/credit redemptions
    gift_card_code VARCHAR(255), -- Gift card code
    merchant_name VARCHAR(255), -- Merchant for gift cards/merchandise
    tracking_number VARCHAR(255), -- Shipping tracking

    -- Processing information
    processing_fee DECIMAL(20, 8) DEFAULT 0, -- Processing fee
    net_amount DECIMAL(20, 8) NOT NULL, -- Net amount after fees
    estimated_delivery TIMESTAMP WITH TIME ZONE, -- Delivery estimate
    actual_delivery TIMESTAMP WITH TIME ZONE, -- Actual delivery

    -- Metadata
    description TEXT,
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    completed_at TIMESTAMP WITH TIME ZONE
);

-- Redemption options table
CREATE TABLE IF NOT EXISTS redemption_options (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(255) NOT NULL,
    description TEXT,
    type VARCHAR(50) NOT NULL, -- 'cash', 'credit', 'gift_card', 'merchandise', 'discount', 'charity'
    is_active BOOLEAN DEFAULT true,

    -- Pricing
    points_required DECIMAL(20, 8) NOT NULL, -- Points needed
    cash_value DECIMAL(20, 8) NOT NULL, -- Cash equivalent
    currency VARCHAR(10) NOT NULL DEFAULT 'USD',
    processing_fee DECIMAL(5, 4) DEFAULT 0, -- Processing fee percentage

    -- Availability
    quantity_available INTEGER DEFAULT -1, -- -1 for unlimited
    quantity_redeemed INTEGER DEFAULT 0,
    minimum_tier VARCHAR(50) DEFAULT 'bronze', -- Minimum tier required

    -- Constraints
    minimum_points_balance DECIMAL(20, 8) DEFAULT 0, -- Minimum balance required
    max_redemptions_per_user INTEGER DEFAULT -1, -- Per user limit
    max_redemptions_per_day INTEGER DEFAULT -1, -- Daily limit

    -- Metadata
    image_url TEXT,
    tags TEXT[],
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Reward audit trail table
CREATE TABLE IF NOT EXISTS reward_audit_trail (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID REFERENCES wallets(id),
    action_type VARCHAR(100) NOT NULL, -- 'award', 'redeem', 'expire', 'adjust', 'tier_change'
    entity_type VARCHAR(100) NOT NULL, -- 'reward_transaction', 'redemption', 'user_rewards'
    entity_id UUID NOT NULL,
    old_value TEXT, -- JSON string of old state
    new_value TEXT, -- JSON string of new state
    reason TEXT,
    performed_by UUID REFERENCES wallets(id), -- User ID who performed the action
    ip_address INET,
    user_agent TEXT,
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Reward indexes
CREATE INDEX IF NOT EXISTS idx_reward_rules_type ON reward_rules(type);
CREATE INDEX IF NOT EXISTS idx_reward_rules_status ON reward_rules(status);
CREATE INDEX IF NOT EXISTS idx_reward_rules_created_by ON reward_rules(created_by);
CREATE INDEX IF NOT EXISTS idx_reward_rules_start_date ON reward_rules(start_date);
CREATE INDEX IF NOT EXISTS idx_reward_rules_end_date ON reward_rules(end_date);
CREATE INDEX IF NOT EXISTS idx_reward_rules_minimum_tier ON reward_rules(minimum_tier);
CREATE INDEX IF NOT EXISTS idx_reward_rules_categories ON reward_rules USING GIN(categories);
CREATE INDEX IF NOT EXISTS idx_reward_rules_currencies ON reward_rules USING GIN(currencies);
CREATE INDEX IF NOT EXISTS idx_reward_rules_created_at ON reward_rules(created_at);

CREATE INDEX IF NOT EXISTS idx_user_rewards_user_id ON user_rewards(user_id);
CREATE INDEX IF NOT EXISTS idx_user_rewards_current_tier ON user_rewards(current_tier);
CREATE INDEX IF NOT EXISTS idx_user_rewards_total_points ON user_rewards(total_points);
CREATE INDEX IF NOT EXISTS idx_user_rewards_last_activity ON user_rewards(last_activity_date);
CREATE INDEX IF NOT EXISTS idx_user_rewards_tier_upgrade ON user_rewards(tier_upgrade_date);
CREATE INDEX IF NOT EXISTS idx_user_rewards_next_expiration ON user_rewards(next_expiration_date);
CREATE INDEX IF NOT EXISTS idx_user_rewards_created_at ON user_rewards(created_at);

CREATE INDEX IF NOT EXISTS idx_reward_transactions_user_id ON reward_transactions(user_id);
CREATE INDEX IF NOT EXISTS idx_reward_transactions_type ON reward_transactions(type);
CREATE INDEX IF NOT EXISTS idx_reward_transactions_status ON reward_transactions(status);
CREATE INDEX IF NOT EXISTS idx_reward_transactions_source_type ON reward_transactions(source_type);
CREATE INDEX IF NOT EXISTS idx_reward_transactions_source_id ON reward_transactions(source_id);
CREATE INDEX IF NOT EXISTS idx_reward_transactions_rule_id ON reward_transactions(reward_rule_id);
CREATE INDEX IF NOT EXISTS idx_reward_transactions_reference ON reward_transactions(reference_number);
CREATE INDEX IF NOT EXISTS idx_reward_transactions_expires_at ON reward_transactions(expires_at);
CREATE INDEX IF NOT EXISTS idx_reward_transactions_is_expired ON reward_transactions(is_expired);
CREATE INDEX IF NOT EXISTS idx_reward_transactions_currency ON reward_transactions(currency);
CREATE INDEX IF NOT EXISTS idx_reward_transactions_created_at ON reward_transactions(created_at);

CREATE INDEX IF NOT EXISTS idx_redemptions_user_id ON redemptions(user_id);
CREATE INDEX IF NOT EXISTS idx_redemptions_type ON redemptions(type);
CREATE INDEX IF NOT EXISTS idx_redemptions_status ON redemptions(status);
CREATE INDEX IF NOT EXISTS idx_redemptions_currency ON redemptions(currency);
CREATE INDEX IF NOT EXISTS idx_redemptions_merchant_name ON redemptions(merchant_name);
CREATE INDEX IF NOT EXISTS idx_redemptions_tracking_number ON redemptions(tracking_number);
CREATE INDEX IF NOT EXISTS idx_redemptions_estimated_delivery ON redemptions(estimated_delivery);
CREATE INDEX IF NOT EXISTS idx_redemptions_actual_delivery ON redemptions(actual_delivery);
CREATE INDEX IF NOT EXISTS idx_redemptions_created_at ON redemptions(created_at);
CREATE INDEX IF NOT EXISTS idx_redemptions_completed_at ON redemptions(completed_at);

CREATE INDEX IF NOT EXISTS idx_redemption_options_type ON redemption_options(type);
CREATE INDEX IF NOT EXISTS idx_redemption_options_is_active ON redemption_options(is_active);
CREATE INDEX IF NOT EXISTS idx_redemption_options_minimum_tier ON redemption_options(minimum_tier);
CREATE INDEX IF NOT EXISTS idx_redemption_options_currency ON redemption_options(currency);
CREATE INDEX IF NOT EXISTS idx_redemption_options_points_required ON redemption_options(points_required);
CREATE INDEX IF NOT EXISTS idx_redemption_options_tags ON redemption_options USING GIN(tags);
CREATE INDEX IF NOT EXISTS idx_redemption_options_created_at ON redemption_options(created_at);

CREATE INDEX IF NOT EXISTS idx_reward_audit_trail_user_id ON reward_audit_trail(user_id);
CREATE INDEX IF NOT EXISTS idx_reward_audit_trail_action_type ON reward_audit_trail(action_type);
CREATE INDEX IF NOT EXISTS idx_reward_audit_trail_entity_type ON reward_audit_trail(entity_type);
CREATE INDEX IF NOT EXISTS idx_reward_audit_trail_entity_id ON reward_audit_trail(entity_id);
CREATE INDEX IF NOT EXISTS idx_reward_audit_trail_performed_by ON reward_audit_trail(performed_by);
CREATE INDEX IF NOT EXISTS idx_reward_audit_trail_ip_address ON reward_audit_trail(ip_address);
CREATE INDEX IF NOT EXISTS idx_reward_audit_trail_created_at ON reward_audit_trail(created_at);

-- Referral codes table
CREATE TABLE IF NOT EXISTS referral_codes (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES wallets(id) ON DELETE CASCADE,
    code VARCHAR(50) NOT NULL UNIQUE,
    status VARCHAR(50) NOT NULL DEFAULT 'active', -- 'active', 'inactive', 'expired', 'suspended', 'exhausted'

    -- Code configuration
    campaign_id UUID, -- References referral_campaigns(id)
    description TEXT,
    is_custom BOOLEAN DEFAULT false,

    -- Usage tracking
    max_uses INTEGER DEFAULT -1, -- -1 for unlimited
    current_uses INTEGER DEFAULT 0,
    successful_referrals INTEGER DEFAULT 0,
    pending_referrals INTEGER DEFAULT 0,

    -- Time constraints
    expires_at TIMESTAMP WITH TIME ZONE,
    last_used_at TIMESTAMP WITH TIME ZONE,

    -- Metadata
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Referral campaigns table
CREATE TABLE IF NOT EXISTS referral_campaigns (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(255) NOT NULL,
    description TEXT,
    type VARCHAR(50) NOT NULL, -- 'signup', 'first_transaction', 'spending_milestone', 'tier_upgrade', 'multi_level', 'time_limited'
    status VARCHAR(50) NOT NULL DEFAULT 'draft', -- 'draft', 'active', 'paused', 'completed', 'cancelled'

    -- Campaign configuration
    referrer_bonus DECIMAL(20, 8) NOT NULL DEFAULT 0,
    referee_bonus DECIMAL(20, 8) NOT NULL DEFAULT 0,
    bonus_currency VARCHAR(10) NOT NULL DEFAULT 'points',
    minimum_transaction_amount DECIMAL(20, 8) DEFAULT 0,

    -- Multi-level configuration
    is_multi_level BOOLEAN DEFAULT false,
    max_levels INTEGER DEFAULT 1,
    level_multipliers DECIMAL(5, 4)[] DEFAULT ARRAY[1.0000], -- Multipliers for each level

    -- Time constraints
    start_date TIMESTAMP WITH TIME ZONE,
    end_date TIMESTAMP WITH TIME ZONE,
    bonus_expiry_days INTEGER DEFAULT 30,

    -- Usage limits
    max_referrals_per_user INTEGER DEFAULT -1, -- -1 for unlimited
    max_total_referrals INTEGER DEFAULT -1,
    max_bonus_per_user DECIMAL(20, 8),
    total_budget DECIMAL(20, 8),
    budget_used DECIMAL(20, 8) DEFAULT 0,

    -- Targeting
    target_user_tiers TEXT[] DEFAULT ARRAY['bronze', 'silver', 'gold', 'platinum'],
    target_countries TEXT[],
    excluded_users UUID[],

    -- Metadata
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    created_by UUID REFERENCES wallets(id)
);

-- Referral relationships table
CREATE TABLE IF NOT EXISTS referral_relationships (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    referrer_user_id UUID NOT NULL REFERENCES wallets(id) ON DELETE CASCADE,
    referee_user_id UUID NOT NULL REFERENCES wallets(id) ON DELETE CASCADE,
    referral_code_id UUID NOT NULL REFERENCES referral_codes(id),
    campaign_id UUID REFERENCES referral_campaigns(id),
    status VARCHAR(50) NOT NULL DEFAULT 'pending', -- 'pending', 'active', 'completed', 'cancelled', 'fraudulent'

    -- Relationship details
    referral_level INTEGER NOT NULL DEFAULT 1, -- 1 = direct, 2 = sub-referral, etc.
    parent_relationship_id UUID REFERENCES referral_relationships(id),

    -- Milestone tracking
    signup_completed BOOLEAN DEFAULT false,
    first_transaction_completed BOOLEAN DEFAULT false,
    kyc_completed BOOLEAN DEFAULT false,
    first_transaction_date TIMESTAMP WITH TIME ZONE,
    kyc_completion_date TIMESTAMP WITH TIME ZONE,

    -- Bonus tracking
    total_bonuses_earned DECIMAL(20, 8) DEFAULT 0,
    total_bonuses_paid DECIMAL(20, 8) DEFAULT 0,
    bonuses_pending INTEGER DEFAULT 0,

    -- Fraud detection
    is_suspicious BOOLEAN DEFAULT false,
    fraud_flags TEXT[],
    fraud_check_date TIMESTAMP WITH TIME ZONE,

    -- Metadata
    referral_source VARCHAR(50), -- 'web', 'mobile', 'email', etc.
    ip_address INET,
    user_agent TEXT,
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),

    -- Constraints
    UNIQUE(referrer_user_id, referee_user_id), -- Prevent duplicate relationships
    CHECK(referrer_user_id != referee_user_id) -- Prevent self-referrals
);

-- Referral bonuses table
CREATE TABLE IF NOT EXISTS referral_bonuses (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    referral_relationship_id UUID NOT NULL REFERENCES referral_relationships(id) ON DELETE CASCADE,
    campaign_id UUID REFERENCES referral_campaigns(id),
    user_id UUID NOT NULL REFERENCES wallets(id) ON DELETE CASCADE,
    type VARCHAR(50) NOT NULL, -- 'referrer', 'referee', 'milestone', 'tier_bonus', 'campaign_bonus'
    status VARCHAR(50) NOT NULL DEFAULT 'pending', -- 'pending', 'processing', 'completed', 'failed', 'cancelled', 'expired'

    -- Bonus details
    bonus_amount DECIMAL(20, 8) NOT NULL,
    bonus_currency VARCHAR(10) NOT NULL DEFAULT 'points',
    exchange_rate DECIMAL(20, 8) DEFAULT 1.0000,
    milestone_type VARCHAR(50), -- 'signup', 'first_transaction', etc.
    milestone_value DECIMAL(20, 8),

    -- Processing details
    reward_transaction_id UUID, -- References reward_transactions(id)
    processing_fee DECIMAL(20, 8) DEFAULT 0,
    net_amount DECIMAL(20, 8) NOT NULL,

    -- Time tracking
    earned_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    processed_at TIMESTAMP WITH TIME ZONE,
    expires_at TIMESTAMP WITH TIME ZONE,

    -- Metadata
    description TEXT,
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Referral audit trail table
CREATE TABLE IF NOT EXISTS referral_audit_trail (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID REFERENCES wallets(id),
    relationship_id UUID REFERENCES referral_relationships(id),
    action_type VARCHAR(100) NOT NULL, -- 'create', 'update', 'bonus', 'flag', 'suspend'
    entity_type VARCHAR(100) NOT NULL, -- 'referral_code', 'relationship', 'campaign', 'bonus'
    entity_id UUID NOT NULL,
    old_value TEXT, -- JSON string of old state
    new_value TEXT, -- JSON string of new state
    reason TEXT,
    performed_by UUID REFERENCES wallets(id),
    ip_address INET,
    user_agent TEXT,
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

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
CREATE TRIGGER update_card_funding_sources_updated_at BEFORE UPDATE ON card_funding_sources FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_card_funding_transactions_updated_at BEFORE UPDATE ON card_funding_transactions FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_card_funding_limits_updated_at BEFORE UPDATE ON card_funding_limits FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_ledger_accounts_updated_at BEFORE UPDATE ON ledger_accounts FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_ledger_transactions_updated_at BEFORE UPDATE ON ledger_transactions FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_journal_entries_updated_at BEFORE UPDATE ON journal_entries FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_reward_rules_updated_at BEFORE UPDATE ON reward_rules FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_user_rewards_updated_at BEFORE UPDATE ON user_rewards FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_reward_transactions_updated_at BEFORE UPDATE ON reward_transactions FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_redemptions_updated_at BEFORE UPDATE ON redemptions FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_redemption_options_updated_at BEFORE UPDATE ON redemption_options FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Referral indexes
CREATE INDEX IF NOT EXISTS idx_referral_codes_user_id ON referral_codes(user_id);
CREATE INDEX IF NOT EXISTS idx_referral_codes_code ON referral_codes(code);
CREATE INDEX IF NOT EXISTS idx_referral_codes_status ON referral_codes(status);
CREATE INDEX IF NOT EXISTS idx_referral_codes_campaign_id ON referral_codes(campaign_id);
CREATE INDEX IF NOT EXISTS idx_referral_codes_expires_at ON referral_codes(expires_at);
CREATE INDEX IF NOT EXISTS idx_referral_codes_last_used_at ON referral_codes(last_used_at);
CREATE INDEX IF NOT EXISTS idx_referral_codes_created_at ON referral_codes(created_at);

CREATE INDEX IF NOT EXISTS idx_referral_campaigns_type ON referral_campaigns(type);
CREATE INDEX IF NOT EXISTS idx_referral_campaigns_status ON referral_campaigns(status);
CREATE INDEX IF NOT EXISTS idx_referral_campaigns_start_date ON referral_campaigns(start_date);
CREATE INDEX IF NOT EXISTS idx_referral_campaigns_end_date ON referral_campaigns(end_date);
CREATE INDEX IF NOT EXISTS idx_referral_campaigns_created_by ON referral_campaigns(created_by);
CREATE INDEX IF NOT EXISTS idx_referral_campaigns_target_tiers ON referral_campaigns USING GIN(target_user_tiers);
CREATE INDEX IF NOT EXISTS idx_referral_campaigns_target_countries ON referral_campaigns USING GIN(target_countries);
CREATE INDEX IF NOT EXISTS idx_referral_campaigns_created_at ON referral_campaigns(created_at);

CREATE INDEX IF NOT EXISTS idx_referral_relationships_referrer ON referral_relationships(referrer_user_id);
CREATE INDEX IF NOT EXISTS idx_referral_relationships_referee ON referral_relationships(referee_user_id);
CREATE INDEX IF NOT EXISTS idx_referral_relationships_code_id ON referral_relationships(referral_code_id);
CREATE INDEX IF NOT EXISTS idx_referral_relationships_campaign_id ON referral_relationships(campaign_id);
CREATE INDEX IF NOT EXISTS idx_referral_relationships_status ON referral_relationships(status);
CREATE INDEX IF NOT EXISTS idx_referral_relationships_level ON referral_relationships(referral_level);
CREATE INDEX IF NOT EXISTS idx_referral_relationships_parent ON referral_relationships(parent_relationship_id);
CREATE INDEX IF NOT EXISTS idx_referral_relationships_signup ON referral_relationships(signup_completed);
CREATE INDEX IF NOT EXISTS idx_referral_relationships_first_tx ON referral_relationships(first_transaction_completed);
CREATE INDEX IF NOT EXISTS idx_referral_relationships_kyc ON referral_relationships(kyc_completed);
CREATE INDEX IF NOT EXISTS idx_referral_relationships_suspicious ON referral_relationships(is_suspicious);
CREATE INDEX IF NOT EXISTS idx_referral_relationships_source ON referral_relationships(referral_source);
CREATE INDEX IF NOT EXISTS idx_referral_relationships_ip ON referral_relationships(ip_address);
CREATE INDEX IF NOT EXISTS idx_referral_relationships_created_at ON referral_relationships(created_at);

CREATE INDEX IF NOT EXISTS idx_referral_bonuses_relationship_id ON referral_bonuses(referral_relationship_id);
CREATE INDEX IF NOT EXISTS idx_referral_bonuses_campaign_id ON referral_bonuses(campaign_id);
CREATE INDEX IF NOT EXISTS idx_referral_bonuses_user_id ON referral_bonuses(user_id);
CREATE INDEX IF NOT EXISTS idx_referral_bonuses_type ON referral_bonuses(type);
CREATE INDEX IF NOT EXISTS idx_referral_bonuses_status ON referral_bonuses(status);
CREATE INDEX IF NOT EXISTS idx_referral_bonuses_milestone_type ON referral_bonuses(milestone_type);
CREATE INDEX IF NOT EXISTS idx_referral_bonuses_reward_tx_id ON referral_bonuses(reward_transaction_id);
CREATE INDEX IF NOT EXISTS idx_referral_bonuses_earned_at ON referral_bonuses(earned_at);
CREATE INDEX IF NOT EXISTS idx_referral_bonuses_processed_at ON referral_bonuses(processed_at);
CREATE INDEX IF NOT EXISTS idx_referral_bonuses_expires_at ON referral_bonuses(expires_at);
CREATE INDEX IF NOT EXISTS idx_referral_bonuses_created_at ON referral_bonuses(created_at);

CREATE INDEX IF NOT EXISTS idx_referral_audit_trail_user_id ON referral_audit_trail(user_id);
CREATE INDEX IF NOT EXISTS idx_referral_audit_trail_relationship_id ON referral_audit_trail(relationship_id);
CREATE INDEX IF NOT EXISTS idx_referral_audit_trail_action_type ON referral_audit_trail(action_type);
CREATE INDEX IF NOT EXISTS idx_referral_audit_trail_entity_type ON referral_audit_trail(entity_type);
CREATE INDEX IF NOT EXISTS idx_referral_audit_trail_entity_id ON referral_audit_trail(entity_id);
CREATE INDEX IF NOT EXISTS idx_referral_audit_trail_performed_by ON referral_audit_trail(performed_by);
CREATE INDEX IF NOT EXISTS idx_referral_audit_trail_ip_address ON referral_audit_trail(ip_address);
CREATE INDEX IF NOT EXISTS idx_referral_audit_trail_created_at ON referral_audit_trail(created_at);

-- Referral triggers
CREATE TRIGGER update_referral_codes_updated_at BEFORE UPDATE ON referral_codes FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_referral_campaigns_updated_at BEFORE UPDATE ON referral_campaigns FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_referral_relationships_updated_at BEFORE UPDATE ON referral_relationships FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_referral_bonuses_updated_at BEFORE UPDATE ON referral_bonuses FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
