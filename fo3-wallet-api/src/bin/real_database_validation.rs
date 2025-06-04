//! Real Database Validation Tool
//!
//! Demonstrates ACTUAL PostgreSQL database operations with persistent data.
//! Creates real tables, inserts real data, and provides SQL commands to verify
//! the data exists in your local PostgreSQL instance.

use sqlx::{Pool, Postgres, Row};
use uuid::Uuid;
use chrono::Utc;
use rust_decimal::Decimal;
use tracing::{info, error, warn};
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("🗄️  FO3 Wallet Core REAL Database Validation");
    info!("=" .repeat(60));
    info!("🔍 This tool creates PERSISTENT data in your PostgreSQL database");
    info!("📊 You can inspect the data using psql or any PostgreSQL client");

    // Get database URL
    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://fo3_user:fo3_dev_password@localhost:5432/fo3_wallet_dev".to_string());
    
    info!("📊 Connecting to PostgreSQL: {}", database_url);

    // Connect to database
    let pool = Pool::<Postgres>::connect(&database_url).await
        .map_err(|e| {
            error!("❌ Failed to connect to PostgreSQL: {}", e);
            error!("💡 Make sure PostgreSQL is running and the database exists");
            error!("🔧 Run: ./setup_postgres.sh");
            e
        })?;
    
    info!("✅ Connected to PostgreSQL database");

    // Run validation steps
    info!("");
    info!("🔧 Step 1: Creating database schema...");
    create_real_schema(&pool).await?;
    
    info!("");
    info!("💾 Step 2: Inserting real data...");
    let validation_data = insert_real_data(&pool).await?;
    
    info!("");
    info!("🔍 Step 3: Querying and validating data...");
    validate_real_data(&pool, &validation_data).await?;
    
    info!("");
    info!("📊 Step 4: Demonstrating complex queries...");
    demonstrate_real_queries(&pool).await?;
    
    info!("");
    info!("🎯 Step 5: Providing verification commands...");
    provide_verification_commands(&database_url, &validation_data);

    info!("");
    info!("=" .repeat(60));
    info!("🎉 Real database validation completed successfully!");
    info!("📊 All data is now persistent in your PostgreSQL database");
    info!("🔍 Use the verification commands above to inspect the data");

    Ok(())
}

#[derive(Debug)]
struct ValidationData {
    wallet_id: Uuid,
    kyc_id: Uuid,
    card_id: Uuid,
    transaction_ids: Vec<Uuid>,
}

async fn create_real_schema(pool: &Pool<Postgres>) -> Result<(), sqlx::Error> {
    info!("  🔧 Creating PostgreSQL tables...");
    
    // Create wallets table
    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS demo_wallets (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            name VARCHAR(255) NOT NULL,
            encrypted_mnemonic TEXT NOT NULL,
            balance DECIMAL(20, 8) DEFAULT 0,
            created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
            updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
        )
    "#).execute(pool).await?;
    info!("    ✅ Created 'demo_wallets' table");

    // Create KYC table
    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS demo_kyc_submissions (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            user_id UUID NOT NULL,
            status VARCHAR(50) NOT NULL,
            first_name VARCHAR(100) NOT NULL,
            last_name VARCHAR(100) NOT NULL,
            email VARCHAR(255) NOT NULL,
            phone VARCHAR(20) NOT NULL,
            verification_level INTEGER DEFAULT 1,
            submitted_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
            FOREIGN KEY (user_id) REFERENCES demo_wallets(id) ON DELETE CASCADE
        )
    "#).execute(pool).await?;
    info!("    ✅ Created 'demo_kyc_submissions' table");

    // Create cards table
    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS demo_cards (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            user_id UUID NOT NULL,
            card_number VARCHAR(19) NOT NULL,
            card_type VARCHAR(20) NOT NULL,
            status VARCHAR(20) NOT NULL,
            currency VARCHAR(3) NOT NULL,
            daily_limit DECIMAL(20, 8) NOT NULL,
            monthly_limit DECIMAL(20, 8) NOT NULL,
            current_daily_spent DECIMAL(20, 8) DEFAULT 0,
            current_monthly_spent DECIMAL(20, 8) DEFAULT 0,
            created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
            FOREIGN KEY (user_id) REFERENCES demo_wallets(id) ON DELETE CASCADE
        )
    "#).execute(pool).await?;
    info!("    ✅ Created 'demo_cards' table");

    // Create transactions table
    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS demo_transactions (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            wallet_id UUID NOT NULL,
            card_id UUID,
            transaction_type VARCHAR(50) NOT NULL,
            amount DECIMAL(20, 8) NOT NULL,
            currency VARCHAR(3) NOT NULL,
            status VARCHAR(20) NOT NULL,
            description TEXT,
            merchant_name VARCHAR(255),
            created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
            FOREIGN KEY (wallet_id) REFERENCES demo_wallets(id) ON DELETE CASCADE,
            FOREIGN KEY (card_id) REFERENCES demo_cards(id) ON DELETE SET NULL
        )
    "#).execute(pool).await?;
    info!("    ✅ Created 'demo_transactions' table");

    Ok(())
}

async fn insert_real_data(pool: &Pool<Postgres>) -> Result<ValidationData, sqlx::Error> {
    info!("  💰 Inserting wallet data...");
    
    // Insert wallet
    let wallet_result = sqlx::query(r#"
        INSERT INTO demo_wallets (name, encrypted_mnemonic, balance)
        VALUES ($1, $2, $3)
        RETURNING id, created_at
    "#)
    .bind("FO3 Real Demo Wallet")
    .bind("encrypted_mnemonic_real_postgres_validation_12345")
    .bind(Decimal::from(1000))
    .fetch_one(pool)
    .await?;

    let wallet_id: Uuid = wallet_result.get("id");
    let wallet_created: chrono::DateTime<chrono::Utc> = wallet_result.get("created_at");
    info!("    ✅ Wallet created: {} at {}", wallet_id, wallet_created);

    // Insert KYC data
    info!("  🆔 Inserting KYC data...");
    let kyc_result = sqlx::query(r#"
        INSERT INTO demo_kyc_submissions (user_id, status, first_name, last_name, email, phone, verification_level)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING id, submitted_at
    "#)
    .bind(&wallet_id)
    .bind("verified")
    .bind("Alice")
    .bind("Johnson")
    .bind("alice.johnson@fo3real.com")
    .bind("+1555123456")
    .bind(2)
    .fetch_one(pool)
    .await?;

    let kyc_id: Uuid = kyc_result.get("id");
    let kyc_submitted: chrono::DateTime<chrono::Utc> = kyc_result.get("submitted_at");
    info!("    ✅ KYC created: {} at {}", kyc_id, kyc_submitted);

    // Insert card data
    info!("  💳 Inserting card data...");
    let card_result = sqlx::query(r#"
        INSERT INTO demo_cards (user_id, card_number, card_type, status, currency, daily_limit, monthly_limit)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING id, created_at
    "#)
    .bind(&wallet_id)
    .bind("4111111111111111")
    .bind("virtual")
    .bind("active")
    .bind("USD")
    .bind(Decimal::from(5000))
    .bind(Decimal::from(50000))
    .fetch_one(pool)
    .await?;

    let card_id: Uuid = card_result.get("id");
    let card_created: chrono::DateTime<chrono::Utc> = card_result.get("created_at");
    info!("    ✅ Card created: {} at {}", card_id, card_created);

    // Insert transaction data
    info!("  💸 Inserting transaction data...");
    let mut transaction_ids = Vec::new();
    
    let transactions = vec![
        ("deposit", Decimal::from(1000), "completed", "Initial deposit", None),
        ("purchase", Decimal::from(25), "completed", "Coffee purchase", Some("Starbucks")),
        ("purchase", Decimal::from(150), "completed", "Grocery shopping", Some("Whole Foods")),
        ("withdrawal", Decimal::from(200), "pending", "ATM withdrawal", None),
    ];

    for (tx_type, amount, status, description, merchant) in transactions {
        let tx_result = sqlx::query(r#"
            INSERT INTO demo_transactions (wallet_id, card_id, transaction_type, amount, currency, status, description, merchant_name)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING id, created_at
        "#)
        .bind(&wallet_id)
        .bind(if tx_type == "purchase" { Some(&card_id) } else { None })
        .bind(tx_type)
        .bind(&amount)
        .bind("USD")
        .bind(status)
        .bind(description)
        .bind(merchant)
        .fetch_one(pool)
        .await?;

        let tx_id: Uuid = tx_result.get("id");
        let tx_created: chrono::DateTime<chrono::Utc> = tx_result.get("created_at");
        transaction_ids.push(tx_id);
        info!("    ✅ Transaction created: {} ({} {}) at {}", tx_id, tx_type, amount, tx_created);
    }

    Ok(ValidationData {
        wallet_id,
        kyc_id,
        card_id,
        transaction_ids,
    })
}

async fn validate_real_data(pool: &Pool<Postgres>, data: &ValidationData) -> Result<(), sqlx::Error> {
    info!("  🔍 Validating inserted data...");

    // Validate wallet
    let wallet_row = sqlx::query("SELECT name, balance, created_at FROM demo_wallets WHERE id = $1")
        .bind(&data.wallet_id)
        .fetch_one(pool)
        .await?;

    let wallet_name: String = wallet_row.get("name");
    let wallet_balance: Decimal = wallet_row.get("balance");
    info!("    ✅ Wallet validated: {} (Balance: {})", wallet_name, wallet_balance);

    // Validate KYC
    let kyc_row = sqlx::query("SELECT first_name, last_name, status, verification_level FROM demo_kyc_submissions WHERE id = $1")
        .bind(&data.kyc_id)
        .fetch_one(pool)
        .await?;

    let first_name: String = kyc_row.get("first_name");
    let last_name: String = kyc_row.get("last_name");
    let status: String = kyc_row.get("status");
    let level: i32 = kyc_row.get("verification_level");
    info!("    ✅ KYC validated: {} {} ({}, Level {})", first_name, last_name, status, level);

    // Validate card
    let card_row = sqlx::query("SELECT card_type, status, daily_limit FROM demo_cards WHERE id = $1")
        .bind(&data.card_id)
        .fetch_one(pool)
        .await?;

    let card_type: String = card_row.get("card_type");
    let card_status: String = card_row.get("status");
    let daily_limit: Decimal = card_row.get("daily_limit");
    info!("    ✅ Card validated: {} {} (Daily limit: {})", card_type, card_status, daily_limit);

    // Validate transactions
    let tx_count = sqlx::query("SELECT COUNT(*) as count FROM demo_transactions WHERE wallet_id = $1")
        .bind(&data.wallet_id)
        .fetch_one(pool)
        .await?;

    let count: i64 = tx_count.get("count");
    info!("    ✅ Transactions validated: {} transactions found", count);

    Ok(())
}

async fn demonstrate_real_queries(pool: &Pool<Postgres>) -> Result<(), sqlx::Error> {
    info!("  📊 Running complex JOIN queries...");

    // Complex query with JOINs
    let join_rows = sqlx::query(r#"
        SELECT
            w.id as wallet_id,
            w.name as wallet_name,
            w.balance,
            k.first_name || ' ' || k.last_name as full_name,
            k.status as kyc_status,
            k.verification_level,
            COUNT(DISTINCT c.id) as card_count,
            COUNT(DISTINCT t.id) as transaction_count,
            COALESCE(SUM(CASE WHEN t.transaction_type = 'purchase' THEN t.amount ELSE 0 END), 0) as total_purchases
        FROM demo_wallets w
        LEFT JOIN demo_kyc_submissions k ON w.id = k.user_id
        LEFT JOIN demo_cards c ON w.id = c.user_id
        LEFT JOIN demo_transactions t ON w.id = t.wallet_id
        GROUP BY w.id, w.name, w.balance, k.first_name, k.last_name, k.status, k.verification_level
        ORDER BY w.created_at DESC
    "#)
    .fetch_all(pool)
    .await?;

    for row in join_rows {
        let wallet_id: Uuid = row.get("wallet_id");
        let wallet_name: String = row.get("wallet_name");
        let balance: Decimal = row.get("balance");
        let full_name: Option<String> = row.get("full_name");
        let kyc_status: Option<String> = row.get("kyc_status");
        let verification_level: Option<i32> = row.get("verification_level");
        let card_count: i64 = row.get("card_count");
        let transaction_count: i64 = row.get("transaction_count");
        let total_purchases: Decimal = row.get("total_purchases");

        info!("    📊 Wallet: {} - {} (Balance: {})", &wallet_id.to_string()[..8], wallet_name, balance);
        if let Some(name) = full_name {
            info!("      👤 User: {} (KYC: {:?}, Level: {:?})", name, kyc_status, verification_level);
        }
        info!("      💳 Cards: {}, 💸 Transactions: {}, 🛒 Total Purchases: {}", card_count, transaction_count, total_purchases);
    }

    Ok(())
}

fn provide_verification_commands(database_url: &str, data: &ValidationData) {
    info!("  📋 PostgreSQL Verification Commands:");
    info!("");
    info!("    🔗 Connect to database:");
    info!("      psql \"{}\"", database_url);
    info!("");
    info!("    📊 View all tables:");
    info!("      \\dt demo_*");
    info!("");
    info!("    🔍 Inspect specific records:");
    info!("      SELECT * FROM demo_wallets WHERE id = '{}';", data.wallet_id);
    info!("      SELECT * FROM demo_kyc_submissions WHERE id = '{}';", data.kyc_id);
    info!("      SELECT * FROM demo_cards WHERE id = '{}';", data.card_id);
    info!("      SELECT * FROM demo_transactions WHERE wallet_id = '{}';", data.wallet_id);
    info!("");
    info!("    📈 Complex queries:");
    info!("      SELECT w.name, k.first_name, k.last_name, COUNT(t.id) as tx_count");
    info!("      FROM demo_wallets w");
    info!("      JOIN demo_kyc_submissions k ON w.id = k.user_id");
    info!("      LEFT JOIN demo_transactions t ON w.id = t.wallet_id");
    info!("      GROUP BY w.id, w.name, k.first_name, k.last_name;");
    info!("");
    info!("    🧹 Clean up (optional):");
    info!("      DROP TABLE demo_transactions, demo_cards, demo_kyc_submissions, demo_wallets CASCADE;");
}
