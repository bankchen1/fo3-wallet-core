-- Cards and Card Transactions Schema Migration
-- This migration creates tables for virtual card management

-- Cards table
CREATE TABLE IF NOT EXISTS cards (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES wallets(id) ON DELETE CASCADE,
    card_type VARCHAR(50) NOT NULL DEFAULT 'virtual', -- 'virtual', 'physical'
    currency VARCHAR(10) NOT NULL DEFAULT 'USD',
    status VARCHAR(50) NOT NULL DEFAULT 'active', -- 'active', 'frozen', 'cancelled', 'expired'
    balance DECIMAL(20, 8) NOT NULL DEFAULT 0.00,
    daily_limit DECIMAL(20, 8) NOT NULL DEFAULT 5000.00,
    monthly_limit DECIMAL(20, 8) NOT NULL DEFAULT 50000.00,
    masked_number VARCHAR(20) NOT NULL, -- Last 4 digits for display
    encrypted_number TEXT NOT NULL, -- Encrypted full card number
    expiry_month INTEGER NOT NULL,
    expiry_year INTEGER NOT NULL,
    encrypted_cvv TEXT NOT NULL, -- Encrypted CVV
    design_id VARCHAR(100), -- Card design identifier
    freeze_reason TEXT, -- Reason for freezing if applicable
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    expires_at TIMESTAMP WITH TIME ZONE, -- Card expiration date
    last_used_at TIMESTAMP WITH TIME ZONE -- Last transaction timestamp
);

-- Card transactions table
CREATE TABLE IF NOT EXISTS card_transactions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    card_id UUID NOT NULL REFERENCES cards(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES wallets(id) ON DELETE CASCADE,
    amount DECIMAL(20, 8) NOT NULL,
    currency VARCHAR(10) NOT NULL,
    merchant_name VARCHAR(255) NOT NULL,
    merchant_category VARCHAR(100), -- MCC category
    merchant_id VARCHAR(255), -- External merchant identifier
    description TEXT,
    status VARCHAR(50) NOT NULL DEFAULT 'pending', -- 'pending', 'approved', 'declined', 'reversed'
    transaction_type VARCHAR(50) NOT NULL, -- 'purchase', 'refund', 'fee', 'adjustment'
    reference_number VARCHAR(255) NOT NULL UNIQUE,
    authorization_code VARCHAR(50), -- Payment processor auth code
    decline_reason TEXT, -- Reason for decline if applicable
    fee_amount DECIMAL(20, 8) DEFAULT 0.00, -- Transaction fees
    cashback_amount DECIMAL(20, 8) DEFAULT 0.00, -- Cashback earned
    location_data JSONB, -- Merchant location information
    metadata JSONB, -- Additional transaction metadata
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    processed_at TIMESTAMP WITH TIME ZONE -- When transaction was processed
);

-- Card limits table (for dynamic limit management)
CREATE TABLE IF NOT EXISTS card_limits (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    card_id UUID NOT NULL REFERENCES cards(id) ON DELETE CASCADE,
    limit_type VARCHAR(50) NOT NULL, -- 'daily', 'weekly', 'monthly', 'per_transaction'
    limit_amount DECIMAL(20, 8) NOT NULL,
    current_usage DECIMAL(20, 8) DEFAULT 0.00,
    reset_period VARCHAR(50) NOT NULL, -- 'daily', 'weekly', 'monthly'
    last_reset_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    UNIQUE(card_id, limit_type)
);

-- Card spending categories table (for spending insights)
CREATE TABLE IF NOT EXISTS card_spending_categories (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    card_id UUID NOT NULL REFERENCES cards(id) ON DELETE CASCADE,
    category VARCHAR(100) NOT NULL, -- 'food', 'transport', 'entertainment', etc.
    monthly_budget DECIMAL(20, 8), -- Optional budget limit
    current_spending DECIMAL(20, 8) DEFAULT 0.00,
    transaction_count INTEGER DEFAULT 0,
    month_year VARCHAR(7) NOT NULL, -- Format: 'YYYY-MM'
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    UNIQUE(card_id, category, month_year)
);

-- Card security events table
CREATE TABLE IF NOT EXISTS card_security_events (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    card_id UUID NOT NULL REFERENCES cards(id) ON DELETE CASCADE,
    event_type VARCHAR(100) NOT NULL, -- 'fraud_alert', 'unusual_activity', 'location_mismatch', etc.
    severity VARCHAR(20) NOT NULL, -- 'low', 'medium', 'high', 'critical'
    description TEXT NOT NULL,
    event_data JSONB, -- Additional event details
    resolved BOOLEAN DEFAULT false,
    resolved_at TIMESTAMP WITH TIME ZONE,
    resolved_by VARCHAR(255), -- Admin who resolved the event
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Create indexes for performance
CREATE INDEX IF NOT EXISTS idx_cards_user_id ON cards(user_id);
CREATE INDEX IF NOT EXISTS idx_cards_status ON cards(status);
CREATE INDEX IF NOT EXISTS idx_cards_currency ON cards(currency);
CREATE INDEX IF NOT EXISTS idx_cards_created_at ON cards(created_at);
CREATE INDEX IF NOT EXISTS idx_cards_expires_at ON cards(expires_at);

CREATE INDEX IF NOT EXISTS idx_card_transactions_card_id ON card_transactions(card_id);
CREATE INDEX IF NOT EXISTS idx_card_transactions_user_id ON card_transactions(user_id);
CREATE INDEX IF NOT EXISTS idx_card_transactions_status ON card_transactions(status);
CREATE INDEX IF NOT EXISTS idx_card_transactions_created_at ON card_transactions(created_at);
CREATE INDEX IF NOT EXISTS idx_card_transactions_merchant ON card_transactions(merchant_name);
CREATE INDEX IF NOT EXISTS idx_card_transactions_reference ON card_transactions(reference_number);
CREATE INDEX IF NOT EXISTS idx_card_transactions_amount ON card_transactions(amount);

CREATE INDEX IF NOT EXISTS idx_card_limits_card_id ON card_limits(card_id);
CREATE INDEX IF NOT EXISTS idx_card_limits_type ON card_limits(limit_type);

CREATE INDEX IF NOT EXISTS idx_card_spending_card_id ON card_spending_categories(card_id);
CREATE INDEX IF NOT EXISTS idx_card_spending_month ON card_spending_categories(month_year);
CREATE INDEX IF NOT EXISTS idx_card_spending_category ON card_spending_categories(category);

CREATE INDEX IF NOT EXISTS idx_card_security_card_id ON card_security_events(card_id);
CREATE INDEX IF NOT EXISTS idx_card_security_type ON card_security_events(event_type);
CREATE INDEX IF NOT EXISTS idx_card_security_severity ON card_security_events(severity);
CREATE INDEX IF NOT EXISTS idx_card_security_resolved ON card_security_events(resolved);
