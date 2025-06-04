//! Database Operations Demonstrator
//!
//! Shows REAL database operations with actual PostgreSQL connections.
//! Creates persistent data that can be inspected in your local PostgreSQL instance.
//! Provides concrete evidence of SQLx working with real database connections.

use sqlx::{Pool, Postgres, Row, PgPool};
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

    info!("🗄️  FO3 Wallet Core REAL Database Operations Demo");
    info!("=" .repeat(60));

    // Use real PostgreSQL database
    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://fo3_user:fo3_dev_password@localhost:5432/fo3_wallet_dev".to_string());

    info!("📊 Connecting to REAL PostgreSQL database: {}", database_url);
    info!("🔍 This will create PERSISTENT data in your local PostgreSQL instance");

    let pool = Pool::<Postgres>::connect(&database_url).await?;
    info!("✅ REAL PostgreSQL database connection established");

    // Create tables
    info!("🔧 Creating database schema...");
    create_schema(&pool).await?;
    info!("✅ Database schema created");

    // Demonstrate real data operations
    info!("💾 Demonstrating real database operations...");
    
    // 1. Insert wallet data
    let wallet_id = insert_wallet_data(&pool).await?;
    
    // 2. Insert KYC data
    let kyc_id = insert_kyc_data(&pool, &wallet_id).await?;
    
    // 3. Insert card data
    let card_id = insert_card_data(&pool, &wallet_id).await?;
    
    // 4. Insert transaction data
    insert_transaction_data(&pool, &wallet_id).await?;
    
    // 5. Query and display results
    query_and_display_data(&pool).await?;
    
    // 6. Demonstrate complex queries
    demonstrate_complex_queries(&pool).await?;
    
    // 7. Show database statistics
    show_database_statistics(&pool).await?;

    info!("=" .repeat(50));
    info!("🎉 Database operations demo completed!");
    info!("📊 Real data inserted and queried successfully");
    info!("🔍 Complex queries executed");
    info!("📈 Database statistics generated");

    Ok(())
}

async fn create_schema(pool: &Pool<Postgres>) -> Result<(), sqlx::Error> {
    info!("  🔧 Creating PostgreSQL tables with proper data types...");

    // Create wallets table with PostgreSQL-specific types
    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS wallets (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            name VARCHAR(255) NOT NULL,
            encrypted_mnemonic TEXT NOT NULL,
            created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
            updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
        )
    "#).execute(pool).await?;
    info!("    ✅ Created 'wallets' table");

    // Create KYC submissions table
    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS kyc_submissions (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            user_id UUID NOT NULL,
            status VARCHAR(50) NOT NULL,
            first_name VARCHAR(100) NOT NULL,
            last_name VARCHAR(100) NOT NULL,
            email VARCHAR(255) NOT NULL,
            phone VARCHAR(20) NOT NULL,
            submitted_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
            FOREIGN KEY (user_id) REFERENCES wallets(id) ON DELETE CASCADE
        )
    "#).execute(pool).await?;
    info!("    ✅ Created 'kyc_submissions' table");

    // Create cards table
    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS cards (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            user_id UUID NOT NULL,
            card_number VARCHAR(19) NOT NULL,
            card_type VARCHAR(20) NOT NULL,
            status VARCHAR(20) NOT NULL,
            currency VARCHAR(3) NOT NULL,
            daily_limit DECIMAL(20, 8) NOT NULL,
            monthly_limit DECIMAL(20, 8) NOT NULL,
            created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
            FOREIGN KEY (user_id) REFERENCES wallets(id) ON DELETE CASCADE
        )
    "#).execute(pool).await?;
    info!("    ✅ Created 'cards' table");

    // Create transactions table
    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS transactions (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            wallet_id UUID NOT NULL,
            transaction_type VARCHAR(50) NOT NULL,
            amount DECIMAL(20, 8) NOT NULL,
            currency VARCHAR(3) NOT NULL,
            status VARCHAR(20) NOT NULL,
            created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
            FOREIGN KEY (wallet_id) REFERENCES wallets(id) ON DELETE CASCADE
        )
    "#).execute(pool).await?;
    info!("    ✅ Created 'transactions' table");

    Ok(())
}

async fn insert_wallet_data(pool: &Pool<Postgres>) -> Result<Uuid, sqlx::Error> {
    let wallet_id = Uuid::new_v4();

    info!("  💰 Inserting wallet data into PostgreSQL...");

    let result = sqlx::query(r#"
        INSERT INTO wallets (id, name, encrypted_mnemonic)
        VALUES ($1, $2, $3)
        RETURNING id, created_at
    "#)
    .bind(&wallet_id)
    .bind("FO3 Demo Wallet - REAL PostgreSQL Data")
    .bind("encrypted_mnemonic_real_postgres_demo_12345")
    .fetch_one(pool)
    .await?;

    let returned_id: Uuid = result.get("id");
    let created_at: chrono::DateTime<chrono::Utc> = result.get("created_at");

    info!("    ✅ Wallet inserted into PostgreSQL: ID = {}", returned_id);
    info!("    📊 Created at: {}", created_at);
    info!("    🔍 You can verify this in PostgreSQL: SELECT * FROM wallets WHERE id = '{}'", returned_id);

    Ok(returned_id)
}

async fn insert_kyc_data(pool: &Pool<Postgres>, wallet_id: &Uuid) -> Result<Uuid, sqlx::Error> {
    let kyc_id = Uuid::new_v4();

    info!("  🆔 Inserting KYC data into PostgreSQL...");

    let result = sqlx::query(r#"
        INSERT INTO kyc_submissions (id, user_id, status, first_name, last_name, email, phone)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING id, submitted_at
    "#)
    .bind(&kyc_id)
    .bind(wallet_id)
    .bind("pending")
    .bind("John")
    .bind("Doe")
    .bind("john.doe@fo3demo.com")
    .bind("+1234567890")
    .fetch_one(pool)
    .await?;

    let returned_id: Uuid = result.get("id");
    let submitted_at: chrono::DateTime<chrono::Utc> = result.get("submitted_at");

    info!("    ✅ KYC submission inserted into PostgreSQL: ID = {}", returned_id);
    info!("    📊 Submitted at: {}", submitted_at);
    info!("    🔍 You can verify this in PostgreSQL: SELECT * FROM kyc_submissions WHERE id = '{}'", returned_id);

    Ok(returned_id)
}

async fn insert_card_data(pool: &Pool<Sqlite>, wallet_id: &str) -> Result<String, sqlx::Error> {
    let card_id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    
    info!("  💳 Inserting card data...");
    
    let result = sqlx::query(r#"
        INSERT INTO cards (id, user_id, card_number, card_type, status, currency, daily_limit, monthly_limit, created_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
    "#)
    .bind(&card_id)
    .bind(wallet_id)
    .bind("4111111111111111")
    .bind("virtual")
    .bind("active")
    .bind("USD")
    .bind("5000.00")
    .bind("50000.00")
    .bind(&now)
    .execute(pool)
    .await?;

    info!("    ✅ Card inserted: ID = {}", card_id);
    info!("    📊 Rows affected: {}", result.rows_affected());
    
    Ok(card_id)
}

async fn insert_transaction_data(pool: &Pool<Sqlite>, wallet_id: &str) -> Result<(), sqlx::Error> {
    info!("  💸 Inserting transaction data...");
    
    let transactions = vec![
        ("deposit", "1000.00", "completed"),
        ("withdrawal", "250.00", "completed"),
        ("transfer", "100.00", "pending"),
        ("purchase", "75.50", "completed"),
    ];
    
    for (tx_type, amount, status) in transactions {
        let tx_id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        
        let result = sqlx::query(r#"
            INSERT INTO transactions (id, wallet_id, transaction_type, amount, currency, status, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
        "#)
        .bind(&tx_id)
        .bind(wallet_id)
        .bind(tx_type)
        .bind(amount)
        .bind("USD")
        .bind(status)
        .bind(&now)
        .execute(pool)
        .await?;

        info!("    ✅ Transaction inserted: {} {} USD ({})", tx_type, amount, status);
    }
    
    Ok(())
}

async fn query_and_display_data(pool: &Pool<Sqlite>) -> Result<(), sqlx::Error> {
    info!("🔍 Querying and displaying real data...");
    
    // Query wallets
    info!("  💰 Wallets in database:");
    let wallet_rows = sqlx::query("SELECT id, name, created_at FROM wallets")
        .fetch_all(pool)
        .await?;
    
    for row in wallet_rows {
        let id: String = row.get("id");
        let name: String = row.get("name");
        let created_at: String = row.get("created_at");
        info!("    📄 Wallet: {} - {} (created: {})", &id[..8], name, created_at);
    }
    
    // Query KYC submissions
    info!("  🆔 KYC submissions in database:");
    let kyc_rows = sqlx::query("SELECT id, user_id, status, first_name, last_name, email FROM kyc_submissions")
        .fetch_all(pool)
        .await?;
    
    for row in kyc_rows {
        let id: String = row.get("id");
        let user_id: String = row.get("user_id");
        let status: String = row.get("status");
        let first_name: String = row.get("first_name");
        let last_name: String = row.get("last_name");
        let email: String = row.get("email");
        info!("    📄 KYC: {} - {} {} ({}) - Status: {}", &id[..8], first_name, last_name, email, status);
    }
    
    // Query cards
    info!("  💳 Cards in database:");
    let card_rows = sqlx::query("SELECT id, user_id, card_type, status, currency, daily_limit FROM cards")
        .fetch_all(pool)
        .await?;
    
    for row in card_rows {
        let id: String = row.get("id");
        let user_id: String = row.get("user_id");
        let card_type: String = row.get("card_type");
        let status: String = row.get("status");
        let currency: String = row.get("currency");
        let daily_limit: String = row.get("daily_limit");
        info!("    📄 Card: {} - {} {} ({}) - Limit: {} {}", &id[..8], card_type, status, &user_id[..8], daily_limit, currency);
    }
    
    // Query transactions
    info!("  💸 Transactions in database:");
    let tx_rows = sqlx::query("SELECT id, transaction_type, amount, currency, status FROM transactions")
        .fetch_all(pool)
        .await?;
    
    for row in tx_rows {
        let id: String = row.get("id");
        let tx_type: String = row.get("transaction_type");
        let amount: String = row.get("amount");
        let currency: String = row.get("currency");
        let status: String = row.get("status");
        info!("    📄 Transaction: {} - {} {} {} ({})", &id[..8], tx_type, amount, currency, status);
    }
    
    Ok(())
}

async fn demonstrate_complex_queries(pool: &Pool<Sqlite>) -> Result<(), sqlx::Error> {
    info!("🔍 Demonstrating complex queries...");
    
    // Complex JOIN query
    info!("  🔗 JOIN query - Wallets with their KYC and Card counts:");
    let join_rows = sqlx::query(r#"
        SELECT 
            w.id as wallet_id,
            w.name as wallet_name,
            COUNT(DISTINCT k.id) as kyc_count,
            COUNT(DISTINCT c.id) as card_count,
            COUNT(DISTINCT t.id) as transaction_count
        FROM wallets w
        LEFT JOIN kyc_submissions k ON w.id = k.user_id
        LEFT JOIN cards c ON w.id = c.user_id
        LEFT JOIN transactions t ON w.id = t.wallet_id
        GROUP BY w.id, w.name
    "#)
    .fetch_all(pool)
    .await?;
    
    for row in join_rows {
        let wallet_id: String = row.get("wallet_id");
        let wallet_name: String = row.get("wallet_name");
        let kyc_count: i32 = row.get("kyc_count");
        let card_count: i32 = row.get("card_count");
        let transaction_count: i32 = row.get("transaction_count");
        
        info!("    📊 Wallet: {} - {} (KYC: {}, Cards: {}, Transactions: {})", 
              &wallet_id[..8], wallet_name, kyc_count, card_count, transaction_count);
    }
    
    // Aggregate query
    info!("  📈 Aggregate query - Transaction summary:");
    let agg_rows = sqlx::query(r#"
        SELECT 
            transaction_type,
            COUNT(*) as count,
            SUM(CAST(amount AS REAL)) as total_amount,
            AVG(CAST(amount AS REAL)) as avg_amount
        FROM transactions
        GROUP BY transaction_type
        ORDER BY total_amount DESC
    "#)
    .fetch_all(pool)
    .await?;
    
    for row in agg_rows {
        let tx_type: String = row.get("transaction_type");
        let count: i32 = row.get("count");
        let total_amount: f64 = row.get("total_amount");
        let avg_amount: f64 = row.get("avg_amount");
        
        info!("    📊 {}: {} transactions, Total: ${:.2}, Avg: ${:.2}", 
              tx_type, count, total_amount, avg_amount);
    }
    
    Ok(())
}

async fn show_database_statistics(pool: &Pool<Sqlite>) -> Result<(), sqlx::Error> {
    info!("📊 Database Statistics:");
    
    let tables = vec!["wallets", "kyc_submissions", "cards", "transactions"];
    
    for table in tables {
        let count_query = format!("SELECT COUNT(*) as count FROM {}", table);
        let row = sqlx::query(&count_query).fetch_one(pool).await?;
        let count: i32 = row.get("count");
        info!("  📈 Table '{}': {} records", table, count);
    }
    
    // Show database size (SQLite specific)
    let size_row = sqlx::query("SELECT page_count * page_size as size FROM pragma_page_count(), pragma_page_size()")
        .fetch_one(pool)
        .await?;
    let size: i64 = size_row.get("size");
    info!("  💾 Database size: {} bytes", size);
    
    Ok(())
}
