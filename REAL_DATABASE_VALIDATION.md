# FO3 Wallet Core - Real Database Validation Guide

This guide demonstrates **REAL PostgreSQL database operations** with persistent data that you can inspect in your local PostgreSQL instance.

## 🔍 Current vs Real Database Testing

### ❌ Previous Approach (Mock/Simulated)
- Used `sqlite::memory:` (in-memory, temporary)
- Data disappeared when program ended
- No persistent storage
- Could not inspect data externally

### ✅ New Approach (Real Database Operations)
- Uses **real PostgreSQL** on localhost:5432
- Creates **persistent tables and data**
- Data remains after program execution
- Can be inspected with `psql` or any PostgreSQL client

## 🚀 Quick Setup

### 1. Setup PostgreSQL Database

```bash
# Make setup script executable
chmod +x setup_postgres.sh

# Run setup (creates database and user)
./setup_postgres.sh
```

### 2. Run Real Database Validation

```bash
# Set database URL
export DATABASE_URL="postgresql://fo3_user:fo3_dev_password@localhost:5432/fo3_wallet_dev"

# Run real database validation
cd fo3-wallet-api
cargo run --bin real_database_validation
```

### 3. Inspect Real Data

```bash
# Connect to PostgreSQL
psql "postgresql://fo3_user:fo3_dev_password@localhost:5432/fo3_wallet_dev"

# View tables
\dt demo_*

# Inspect data
SELECT * FROM demo_wallets;
SELECT * FROM demo_kyc_submissions;
SELECT * FROM demo_cards;
SELECT * FROM demo_transactions;
```

## 📊 What Gets Created

The validation tool creates **real, persistent** PostgreSQL tables:

### Tables Created
- `demo_wallets` - Wallet information with balances
- `demo_kyc_submissions` - KYC verification data
- `demo_cards` - Virtual card information
- `demo_transactions` - Transaction history

### Sample Data Inserted
- **1 Wallet**: "FO3 Real Demo Wallet" with $1000 balance
- **1 KYC Record**: Alice Johnson, verified, level 2
- **1 Card**: Virtual card with $5000 daily limit
- **4 Transactions**: Deposit, coffee purchase, grocery shopping, ATM withdrawal

## 🔍 Verification Commands

After running the validation, you can verify the data exists:

```sql
-- Connect to database
psql "postgresql://fo3_user:fo3_dev_password@localhost:5432/fo3_wallet_dev"

-- View all demo tables
\dt demo_*

-- Check wallet data
SELECT id, name, balance, created_at FROM demo_wallets;

-- Check KYC data
SELECT first_name, last_name, status, verification_level FROM demo_kyc_submissions;

-- Check card data
SELECT card_type, status, daily_limit, current_daily_spent FROM demo_cards;

-- Check transactions
SELECT transaction_type, amount, status, description, merchant_name FROM demo_transactions;

-- Complex JOIN query
SELECT 
    w.name as wallet_name,
    k.first_name || ' ' || k.last_name as user_name,
    k.status as kyc_status,
    COUNT(t.id) as transaction_count,
    SUM(CASE WHEN t.transaction_type = 'purchase' THEN t.amount ELSE 0 END) as total_purchases
FROM demo_wallets w
JOIN demo_kyc_submissions k ON w.id = k.user_id
LEFT JOIN demo_transactions t ON w.id = t.wallet_id
GROUP BY w.id, w.name, k.first_name, k.last_name, k.status;
```

## 🛠️ Troubleshooting

### PostgreSQL Not Running
```bash
# macOS (Homebrew)
brew services start postgresql

# Ubuntu/Debian
sudo systemctl start postgresql

# Docker
docker run --name fo3-postgres \
  -e POSTGRES_DB=fo3_wallet_dev \
  -e POSTGRES_USER=fo3_user \
  -e POSTGRES_PASSWORD=fo3_dev_password \
  -p 5432:5432 -d postgres:15
```

### Connection Issues
```bash
# Test connection
pg_isready -h localhost -p 5432

# Check if database exists
psql -h localhost -p 5432 -U postgres -l | grep fo3_wallet_dev

# Create database manually if needed
createdb -h localhost -p 5432 -U postgres fo3_wallet_dev
```

### Permission Issues
```bash
# Grant privileges
psql -h localhost -p 5432 -U postgres -c "
  GRANT ALL PRIVILEGES ON DATABASE fo3_wallet_dev TO fo3_user;
  GRANT ALL ON SCHEMA public TO fo3_user;
"
```

## 📈 Performance Validation

The real database validation also demonstrates:

- **Connection pooling** with SQLx
- **Transaction safety** with proper error handling
- **Complex queries** with JOINs and aggregations
- **Data type validation** (UUID, DECIMAL, TIMESTAMP)
- **Foreign key constraints** and referential integrity

## 🧹 Cleanup

To remove the demo data:

```sql
-- Connect to database
psql "postgresql://fo3_user:fo3_dev_password@localhost:5432/fo3_wallet_dev"

-- Drop demo tables
DROP TABLE IF EXISTS demo_transactions CASCADE;
DROP TABLE IF EXISTS demo_cards CASCADE;
DROP TABLE IF EXISTS demo_kyc_submissions CASCADE;
DROP TABLE IF EXISTS demo_wallets CASCADE;
```

## 🎯 Key Differences from Mock Testing

| Aspect | Mock/Simulated | Real Database |
|--------|----------------|---------------|
| **Persistence** | ❌ Temporary | ✅ Persistent |
| **Inspection** | ❌ Cannot inspect | ✅ Full SQL access |
| **Data Types** | ❌ Limited | ✅ Full PostgreSQL types |
| **Constraints** | ❌ Basic | ✅ Foreign keys, indexes |
| **Performance** | ❌ Not realistic | ✅ Real performance |
| **Debugging** | ❌ Limited | ✅ Full query analysis |

## 🚀 Next Steps

1. **Run the validation** to see real data creation
2. **Inspect the data** using the provided SQL commands
3. **Verify persistence** by restarting and checking data still exists
4. **Test modifications** by updating records and seeing changes
5. **Performance testing** with larger datasets

This approach provides **concrete evidence** of real database operations that you can verify independently.
