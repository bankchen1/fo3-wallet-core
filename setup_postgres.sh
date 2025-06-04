#!/bin/bash

# FO3 Wallet Core PostgreSQL Setup Script
# Sets up a real PostgreSQL database for validation testing

set -e

echo "🗄️  FO3 Wallet Core PostgreSQL Setup"
echo "=" $(printf '=%.0s' {1..50})

# Configuration
DB_NAME="fo3_wallet_dev"
DB_USER="fo3_user"
DB_PASSWORD="fo3_dev_password"
DB_HOST="localhost"
DB_PORT="5432"

echo "📋 Database Configuration:"
echo "  🏷️  Database: $DB_NAME"
echo "  👤 User: $DB_USER"
echo "  🏠 Host: $DB_HOST:$DB_PORT"

# Check if PostgreSQL is running
echo ""
echo "🔍 Checking PostgreSQL status..."
if ! pg_isready -h $DB_HOST -p $DB_PORT > /dev/null 2>&1; then
    echo "❌ PostgreSQL is not running on $DB_HOST:$DB_PORT"
    echo ""
    echo "📝 To start PostgreSQL:"
    echo "  macOS (Homebrew): brew services start postgresql"
    echo "  Ubuntu/Debian: sudo systemctl start postgresql"
    echo "  Docker: docker run --name fo3-postgres -e POSTGRES_PASSWORD=postgres -p 5432:5432 -d postgres:15"
    exit 1
fi
echo "✅ PostgreSQL is running"

# Create database and user
echo ""
echo "🔧 Setting up database and user..."

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
    echo "  1. Connect as your system user:"
    echo "     createuser -h $DB_HOST -p $DB_PORT $DB_USER"
    echo "     createdb -h $DB_HOST -p $DB_PORT -O $DB_USER $DB_NAME"
    echo ""
    echo "  2. Use Docker:"
    echo "     docker run --name fo3-postgres \\"
    echo "       -e POSTGRES_DB=$DB_NAME \\"
    echo "       -e POSTGRES_USER=$DB_USER \\"
    echo "       -e POSTGRES_PASSWORD=$DB_PASSWORD \\"
    echo "       -p 5432:5432 -d postgres:15"
    exit 1
}

# Grant privileges
echo "🔐 Granting privileges..."
psql -h $DB_HOST -p $DB_PORT -U postgres -d $DB_NAME -c "
GRANT ALL PRIVILEGES ON DATABASE $DB_NAME TO $DB_USER;
GRANT ALL ON SCHEMA public TO $DB_USER;
ALTER USER $DB_USER CREATEDB;
" > /dev/null 2>&1

# Test connection
echo ""
echo "🧪 Testing database connection..."
export DATABASE_URL="postgresql://$DB_USER:$DB_PASSWORD@$DB_HOST:$DB_PORT/$DB_NAME"

if psql "$DATABASE_URL" -c "SELECT version();" > /dev/null 2>&1; then
    echo "✅ Database connection successful"
else
    echo "❌ Database connection failed"
    exit 1
fi

# Display connection information
echo ""
echo "🎉 PostgreSQL setup completed!"
echo ""
echo "📊 Connection Details:"
echo "  DATABASE_URL=$DATABASE_URL"
echo ""
echo "🔍 To verify the setup:"
echo "  psql \"$DATABASE_URL\" -c \"\\dt\""
echo ""
echo "🚀 To run the real database demo:"
echo "  export DATABASE_URL=\"$DATABASE_URL\""
echo "  cd fo3-wallet-api"
echo "  cargo run --bin database_demo"
echo ""
echo "📋 To inspect data after running demo:"
echo "  psql \"$DATABASE_URL\" -c \"SELECT * FROM wallets;\""
echo "  psql \"$DATABASE_URL\" -c \"SELECT * FROM kyc_submissions;\""
echo "  psql \"$DATABASE_URL\" -c \"SELECT * FROM cards;\""
echo "  psql \"$DATABASE_URL\" -c \"SELECT * FROM transactions;\""
