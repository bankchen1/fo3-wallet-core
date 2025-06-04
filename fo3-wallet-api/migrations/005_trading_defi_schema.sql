-- Trading and DeFi Schema Migration
-- This migration creates tables for trading strategies and DeFi positions

-- Trading strategies table
CREATE TABLE IF NOT EXISTS trading_strategies (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES wallets(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    strategy_type VARCHAR(100) NOT NULL, -- 'momentum', 'arbitrage', 'dca', 'grid', 'mean_reversion'
    status VARCHAR(50) NOT NULL DEFAULT 'active', -- 'active', 'paused', 'stopped', 'completed'
    configuration JSONB NOT NULL, -- Strategy-specific configuration
    risk_level VARCHAR(20) NOT NULL DEFAULT 'medium', -- 'low', 'medium', 'high'
    max_allocation DECIMAL(20, 8) NOT NULL, -- Maximum funds to allocate
    current_allocation DECIMAL(20, 8) DEFAULT 0.00, -- Currently allocated funds
    target_symbols TEXT[], -- Array of trading symbols
    performance_metrics JSONB, -- Performance tracking data
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    started_at TIMESTAMP WITH TIME ZONE,
    stopped_at TIMESTAMP WITH TIME ZONE
);

-- Trading orders table
CREATE TABLE IF NOT EXISTS trading_orders (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    strategy_id UUID NOT NULL REFERENCES trading_strategies(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES wallets(id) ON DELETE CASCADE,
    symbol VARCHAR(20) NOT NULL, -- 'BTC', 'ETH', 'SOL', etc.
    order_type VARCHAR(50) NOT NULL, -- 'market', 'limit', 'stop_loss', 'take_profit'
    side VARCHAR(10) NOT NULL, -- 'buy', 'sell'
    quantity DECIMAL(36, 18) NOT NULL,
    price DECIMAL(36, 18), -- Null for market orders
    filled_quantity DECIMAL(36, 18) DEFAULT 0,
    average_fill_price DECIMAL(36, 18),
    status VARCHAR(50) NOT NULL DEFAULT 'pending', -- 'pending', 'filled', 'partially_filled', 'cancelled', 'rejected'
    exchange VARCHAR(100), -- External exchange identifier
    external_order_id VARCHAR(255), -- Exchange order ID
    fees DECIMAL(20, 8) DEFAULT 0.00,
    metadata JSONB, -- Additional order data
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    filled_at TIMESTAMP WITH TIME ZONE,
    cancelled_at TIMESTAMP WITH TIME ZONE
);

-- Trading positions table
CREATE TABLE IF NOT EXISTS trading_positions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    strategy_id UUID NOT NULL REFERENCES trading_strategies(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES wallets(id) ON DELETE CASCADE,
    symbol VARCHAR(20) NOT NULL,
    position_type VARCHAR(10) NOT NULL, -- 'long', 'short'
    quantity DECIMAL(36, 18) NOT NULL,
    entry_price DECIMAL(36, 18) NOT NULL,
    current_price DECIMAL(36, 18),
    unrealized_pnl DECIMAL(20, 8) DEFAULT 0.00,
    realized_pnl DECIMAL(20, 8) DEFAULT 0.00,
    status VARCHAR(50) NOT NULL DEFAULT 'open', -- 'open', 'closed', 'liquidated'
    stop_loss DECIMAL(36, 18), -- Stop loss price
    take_profit DECIMAL(36, 18), -- Take profit price
    margin_used DECIMAL(20, 8) DEFAULT 0.00,
    fees_paid DECIMAL(20, 8) DEFAULT 0.00,
    opened_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    closed_at TIMESTAMP WITH TIME ZONE,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- DeFi protocols table
CREATE TABLE IF NOT EXISTS defi_protocols (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(100) NOT NULL UNIQUE, -- 'uniswap', 'aave', 'compound', 'curve'
    protocol_type VARCHAR(50) NOT NULL, -- 'dex', 'lending', 'staking', 'yield_farming'
    blockchain VARCHAR(50) NOT NULL, -- 'ethereum', 'polygon', 'arbitrum', 'solana'
    contract_address VARCHAR(255), -- Main protocol contract
    is_active BOOLEAN DEFAULT true,
    risk_rating VARCHAR(20) DEFAULT 'medium', -- 'low', 'medium', 'high'
    tvl DECIMAL(20, 2), -- Total Value Locked
    apy_range_min DECIMAL(8, 4), -- Minimum APY
    apy_range_max DECIMAL(8, 4), -- Maximum APY
    metadata JSONB, -- Protocol-specific data
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- DeFi products table (specific yield products within protocols)
CREATE TABLE IF NOT EXISTS defi_products (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    protocol_id UUID NOT NULL REFERENCES defi_protocols(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL, -- 'USDC/ETH LP', 'AAVE USDC Lending'
    product_type VARCHAR(50) NOT NULL, -- 'liquidity_pool', 'lending', 'staking', 'farming'
    token_symbols TEXT[] NOT NULL, -- ['USDC', 'ETH'] for LP tokens
    contract_address VARCHAR(255), -- Product-specific contract
    current_apy DECIMAL(8, 4) NOT NULL,
    min_deposit DECIMAL(20, 8) DEFAULT 0.00,
    max_deposit DECIMAL(20, 8), -- Null for unlimited
    lock_period_days INTEGER DEFAULT 0, -- 0 for no lock
    auto_compound BOOLEAN DEFAULT false,
    is_active BOOLEAN DEFAULT true,
    risk_level VARCHAR(20) DEFAULT 'medium',
    fees JSONB, -- Fee structure
    metadata JSONB, -- Product-specific data
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- User DeFi positions table
CREATE TABLE IF NOT EXISTS user_defi_positions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES wallets(id) ON DELETE CASCADE,
    product_id UUID NOT NULL REFERENCES defi_products(id) ON DELETE CASCADE,
    position_type VARCHAR(50) NOT NULL, -- 'staked', 'lent', 'liquidity_provided'
    principal_amount DECIMAL(36, 18) NOT NULL, -- Original deposit amount
    current_value DECIMAL(36, 18) NOT NULL, -- Current position value
    rewards_earned DECIMAL(36, 18) DEFAULT 0, -- Total rewards earned
    pending_rewards DECIMAL(36, 18) DEFAULT 0, -- Unclaimed rewards
    entry_apy DECIMAL(8, 4), -- APY when position was opened
    current_apy DECIMAL(8, 4), -- Current APY
    status VARCHAR(50) NOT NULL DEFAULT 'active', -- 'active', 'unstaking', 'withdrawn'
    lock_expires_at TIMESTAMP WITH TIME ZONE, -- When lock period ends
    last_reward_claim TIMESTAMP WITH TIME ZONE, -- Last reward claim timestamp
    transaction_hash VARCHAR(255), -- Blockchain transaction hash
    metadata JSONB, -- Position-specific data
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    withdrawn_at TIMESTAMP WITH TIME ZONE
);

-- Create indexes for performance
CREATE INDEX IF NOT EXISTS idx_trading_strategies_user_id ON trading_strategies(user_id);
CREATE INDEX IF NOT EXISTS idx_trading_strategies_status ON trading_strategies(status);
CREATE INDEX IF NOT EXISTS idx_trading_strategies_type ON trading_strategies(strategy_type);

CREATE INDEX IF NOT EXISTS idx_trading_orders_strategy_id ON trading_orders(strategy_id);
CREATE INDEX IF NOT EXISTS idx_trading_orders_user_id ON trading_orders(user_id);
CREATE INDEX IF NOT EXISTS idx_trading_orders_symbol ON trading_orders(symbol);
CREATE INDEX IF NOT EXISTS idx_trading_orders_status ON trading_orders(status);
CREATE INDEX IF NOT EXISTS idx_trading_orders_created_at ON trading_orders(created_at);

CREATE INDEX IF NOT EXISTS idx_trading_positions_strategy_id ON trading_positions(strategy_id);
CREATE INDEX IF NOT EXISTS idx_trading_positions_user_id ON trading_positions(user_id);
CREATE INDEX IF NOT EXISTS idx_trading_positions_symbol ON trading_positions(symbol);
CREATE INDEX IF NOT EXISTS idx_trading_positions_status ON trading_positions(status);

CREATE INDEX IF NOT EXISTS idx_defi_protocols_name ON defi_protocols(name);
CREATE INDEX IF NOT EXISTS idx_defi_protocols_type ON defi_protocols(protocol_type);
CREATE INDEX IF NOT EXISTS idx_defi_protocols_blockchain ON defi_protocols(blockchain);
CREATE INDEX IF NOT EXISTS idx_defi_protocols_active ON defi_protocols(is_active);

CREATE INDEX IF NOT EXISTS idx_defi_products_protocol_id ON defi_products(protocol_id);
CREATE INDEX IF NOT EXISTS idx_defi_products_type ON defi_products(product_type);
CREATE INDEX IF NOT EXISTS idx_defi_products_active ON defi_products(is_active);
CREATE INDEX IF NOT EXISTS idx_defi_products_apy ON defi_products(current_apy);

CREATE INDEX IF NOT EXISTS idx_user_defi_positions_user_id ON user_defi_positions(user_id);
CREATE INDEX IF NOT EXISTS idx_user_defi_positions_product_id ON user_defi_positions(product_id);
CREATE INDEX IF NOT EXISTS idx_user_defi_positions_status ON user_defi_positions(status);
CREATE INDEX IF NOT EXISTS idx_user_defi_positions_created_at ON user_defi_positions(created_at);
