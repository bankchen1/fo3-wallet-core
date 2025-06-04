-- FO3 Wallet Core Database Initial Schema Migration
-- This migration creates the core tables for the FO3 Wallet system

-- Create extension for UUID generation (PostgreSQL only)
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

-- Create basic indexes for performance
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
