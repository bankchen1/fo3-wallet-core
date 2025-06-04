#!/bin/bash

# FO3 Wallet Core Phase 4: Production-Ready Multi-User Database Infrastructure Setup
# Sets up PostgreSQL with production schema, user isolation, and multi-user testing

set -e

echo "🎯 FO3 Wallet Core Phase 4: Production-Ready Multi-User Setup"
echo "=" $(printf '=%.0s' {1..70})

# Configuration
DB_NAME="fo3_wallet_prod"
DB_USER="fo3_user"
DB_PASSWORD="fo3_secure_password_change_me"
DB_HOST="localhost"
DB_PORT="5432"

echo "📋 Production Database Configuration:"
echo "  🏷️  Database: $DB_NAME"
echo "  👤 User: $DB_USER"
echo "  🏠 Host: $DB_HOST:$DB_PORT"
echo "  🔐 Security: Production-grade with user isolation"

# Check if PostgreSQL is running
echo ""
echo "🔍 Checking PostgreSQL status..."
if ! pg_isready -h $DB_HOST -p $DB_PORT > /dev/null 2>&1; then
    echo "❌ PostgreSQL is not running on $DB_HOST:$DB_PORT"
    echo ""
    echo "📝 To start PostgreSQL:"
    echo "  macOS (Homebrew): brew services start postgresql"
    echo "  Ubuntu/Debian: sudo systemctl start postgresql"
    echo "  Docker: docker-compose up -d postgres"
    exit 1
fi
echo "✅ PostgreSQL is running"

# Create production database and user
echo ""
echo "🔧 Setting up production database and user..."

# Connect as postgres user to create database and user
psql -h $DB_HOST -p $DB_PORT -U postgres -c "
DO \$\$
BEGIN
    -- Create user if not exists
    IF NOT EXISTS (SELECT FROM pg_catalog.pg_roles WHERE rolname = '$DB_USER') THEN
        CREATE USER $DB_USER WITH PASSWORD '$DB_PASSWORD';
        RAISE NOTICE 'User $DB_USER created';
    ELSE
        RAISE NOTICE 'User $DB_USER already exists';
    END IF;
    
    -- Create database if not exists
    IF NOT EXISTS (SELECT FROM pg_database WHERE datname = '$DB_NAME') THEN
        CREATE DATABASE $DB_NAME OWNER $DB_USER;
        RAISE NOTICE 'Database $DB_NAME created';
    ELSE
        RAISE NOTICE 'Database $DB_NAME already exists';
    END IF;
END
\$\$;
" 2>/dev/null || {
    echo "❌ Failed to connect as 'postgres' user"
    echo ""
    echo "📝 Alternative setup methods:"
    echo "  1. Use Docker Compose: docker-compose up -d postgres"
    echo "  2. Manual setup with your PostgreSQL admin user"
    exit 1
}

# Grant comprehensive privileges for production
echo "🔐 Granting production privileges..."
psql -h $DB_HOST -p $DB_PORT -U postgres -d $DB_NAME -c "
-- Grant all privileges on database
GRANT ALL PRIVILEGES ON DATABASE $DB_NAME TO $DB_USER;
GRANT ALL ON SCHEMA public TO $DB_USER;
GRANT ALL ON ALL TABLES IN SCHEMA public TO $DB_USER;
GRANT ALL ON ALL SEQUENCES IN SCHEMA public TO $DB_USER;
GRANT ALL ON ALL FUNCTIONS IN SCHEMA public TO $DB_USER;

-- Allow user to create databases for testing
ALTER USER $DB_USER CREATEDB;

-- Set default privileges for future objects
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT ALL ON TABLES TO $DB_USER;
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT ALL ON SEQUENCES TO $DB_USER;
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT ALL ON FUNCTIONS TO $DB_USER;
" > /dev/null 2>&1

# Set up environment variables
export DATABASE_URL="postgresql://$DB_USER:$DB_PASSWORD@$DB_HOST:$DB_PORT/$DB_NAME"
export JWT_SECRET="fo3_jwt_secret_change_me_in_production_$(openssl rand -hex 32)"
export ENCRYPTION_KEY="fo3_encryption_key_change_me_$(openssl rand -hex 32)"

# Test connection
echo ""
echo "🧪 Testing production database connection..."
if psql "$DATABASE_URL" -c "SELECT version();" > /dev/null 2>&1; then
    echo "✅ Production database connection successful"
else
    echo "❌ Production database connection failed"
    exit 1
fi

# Run database migrations
echo ""
echo "🔄 Running production database migrations..."
cd fo3-wallet-api

# Check if migrations exist
if [ ! -d "migrations" ]; then
    echo "❌ Migrations directory not found"
    exit 1
fi

# Run migrations using sqlx
if command -v sqlx &> /dev/null; then
    echo "📊 Running migrations with sqlx..."
    sqlx migrate run --database-url "$DATABASE_URL"
else
    echo "📊 Running migrations with cargo..."
    cargo run --bin database_migration_runner
fi

echo "✅ Database migrations completed"

# Create test users for multi-user validation
echo ""
echo "👥 Creating test users for multi-user validation..."

psql "$DATABASE_URL" -c "
-- Create users table if not exists
CREATE TABLE IF NOT EXISTS users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    username VARCHAR(50) UNIQUE NOT NULL,
    email VARCHAR(255) UNIQUE NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    role VARCHAR(20) NOT NULL DEFAULT 'basic_user',
    tier VARCHAR(20) NOT NULL DEFAULT 'bronze',
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Insert test users
INSERT INTO users (username, email, password_hash, role, tier) VALUES
('alice_basic', 'alice@fo3test.com', '\$2b\$12\$dummy_hash_for_testing', 'basic_user', 'bronze'),
('bob_premium', 'bob@fo3test.com', '\$2b\$12\$dummy_hash_for_testing', 'premium_user', 'gold'),
('charlie_admin', 'charlie@fo3test.com', '\$2b\$12\$dummy_hash_for_testing', 'admin', 'platinum')
ON CONFLICT (username) DO NOTHING;
"

echo "✅ Test users created"

# Validate database schema
echo ""
echo "📋 Validating production database schema..."

EXPECTED_TABLES=(
    "users"
    "wallets" 
    "kyc_submissions"
    "kyc_documents"
    "cards"
    "bank_accounts"
    "fiat_transactions"
    "transactions"
    "audit_logs"
)

echo "🔍 Checking required tables..."
for table in "${EXPECTED_TABLES[@]}"; do
    if psql "$DATABASE_URL" -c "SELECT 1 FROM $table LIMIT 1;" > /dev/null 2>&1; then
        echo "  ✅ Table '$table' exists and accessible"
    else
        echo "  ⚠️  Table '$table' missing or inaccessible"
    fi
done

# Performance optimization
echo ""
echo "⚡ Applying production performance optimizations..."

psql "$DATABASE_URL" -c "
-- Create indexes for user isolation and performance
CREATE INDEX IF NOT EXISTS idx_wallets_user_id ON wallets(user_id);
CREATE INDEX IF NOT EXISTS idx_wallets_user_active ON wallets(user_id, is_active);
CREATE INDEX IF NOT EXISTS idx_kyc_user_id ON kyc_submissions(user_id);
CREATE INDEX IF NOT EXISTS idx_cards_user_id ON cards(user_id);
CREATE INDEX IF NOT EXISTS idx_transactions_wallet_id ON transactions(wallet_id);
CREATE INDEX IF NOT EXISTS idx_transactions_user_created ON transactions(user_id, created_at);
CREATE INDEX IF NOT EXISTS idx_audit_logs_user_id ON audit_logs(user_id);
CREATE INDEX IF NOT EXISTS idx_audit_logs_created ON audit_logs(created_at);

-- Update table statistics
ANALYZE;
"

echo "✅ Performance optimizations applied"

# Display setup summary
echo ""
echo "🎉 Phase 4 Production Setup Completed!"
echo ""
echo "📊 Production Database Details:"
echo "  DATABASE_URL=$DATABASE_URL"
echo "  JWT_SECRET=$JWT_SECRET"
echo "  ENCRYPTION_KEY=$ENCRYPTION_KEY"
echo ""
echo "👥 Test Users Created:"
echo "  alice_basic (Bronze tier, Basic user)"
echo "  bob_premium (Gold tier, Premium user)" 
echo "  charlie_admin (Platinum tier, Admin)"
echo ""
echo "🚀 To run production validation:"
echo "  export DATABASE_URL=\"$DATABASE_URL\""
echo "  export JWT_SECRET=\"$JWT_SECRET\""
echo "  export ENCRYPTION_KEY=\"$ENCRYPTION_KEY\""
echo "  cd fo3-wallet-api"
echo "  cargo run --bin phase4_production_validation"
echo ""
echo "🔍 To inspect production data:"
echo "  psql \"$DATABASE_URL\""
echo "  \\dt  -- List tables"
echo "  SELECT * FROM users;"
echo "  SELECT * FROM wallets;"
echo ""
echo "📈 Performance monitoring:"
echo "  SELECT schemaname, tablename, n_tup_ins, n_tup_upd, n_tup_del FROM pg_stat_user_tables;"
echo ""
echo "🔐 Security validation:"
echo "  -- Verify user isolation"
echo "  -- Test RBAC enforcement"
echo "  -- Validate audit logging"

cd ..
