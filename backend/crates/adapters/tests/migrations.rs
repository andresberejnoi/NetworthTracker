#![cfg(feature = "db-tests")]
//! Verify that applying a new migration to an existing database preserves data.
//!
//! The test builds a "previous version" database by applying every migration
//! except the last one, seeds representative rows, then applies the remaining
//! migration and asserts the rows and exact values survived.

use std::path::{Path, PathBuf};

use sqlx::migrate::Migrator;
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

fn with_database(url: &str, database: &str) -> String {
    let (prefix, _) = url
        .rsplit_once('/')
        .expect("DATABASE_URL should end with a database name");
    format!("{prefix}/{database}")
}

#[tokio::test]
async fn applying_a_new_migration_preserves_existing_data() {
    let base_url = std::env::var("DATABASE_URL").expect("DATABASE_URL is required");

    let admin = PgPoolOptions::new()
        .max_connections(1)
        .connect(&base_url)
        .await
        .expect("connect to the admin database");

    let db_name = format!("migration_test_{}", Uuid::new_v4().simple());
    sqlx::query(&format!("CREATE DATABASE \"{db_name}\""))
        .execute(&admin)
        .await
        .expect("create test database");

    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&with_database(&base_url, &db_name))
        .await
        .expect("connect to the test database");

    // Build a "previous version" migrator from all migrations except the last.
    let migrations_dir = Path::new("../../migrations");
    let mut files: Vec<PathBuf> = std::fs::read_dir(migrations_dir)
        .expect("read migrations directory")
        .filter_map(|entry| {
            let path = entry.expect("directory entry").path();
            (path.extension().and_then(|extension| extension.to_str()) == Some("sql"))
                .then_some(path)
        })
        .collect();
    files.sort();
    assert!(files.len() >= 2, "need at least two migrations");

    let previous_dir = std::env::temp_dir().join(format!("networth-previous-{db_name}"));
    std::fs::create_dir_all(&previous_dir).expect("create temp migrations dir");
    for file in &files[..files.len() - 1] {
        std::fs::copy(file, previous_dir.join(file.file_name().unwrap())).expect("copy migration");
    }

    Migrator::new(previous_dir.as_path())
        .await
        .expect("load previous migrations")
        .run(&pool)
        .await
        .expect("apply previous migrations");

    // Seed representative data in the previous schema.
    let (account_id,): (Uuid,) = sqlx::query_as(
        "INSERT INTO accounts (name, account_type, currency) \
         VALUES ('Checking', 'checking', 'USD') RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .expect("insert account");

    sqlx::query(
        "INSERT INTO account_balances (account_id, as_of, amount, currency) \
         VALUES ($1, DATE '2026-01-01', 1234.56, 'USD')",
    )
    .bind(account_id)
    .execute(&pool)
    .await
    .expect("insert balance");

    sqlx::query(
        "INSERT INTO net_worth_snapshots \
         (as_of, scope, currency, total, liquid, liquidity, accessible, \
          tax_rate, penalty_rate, penalty_age, selling_rate, fixed_costs) \
         VALUES (DATE '2026-01-01', 'household', 'USD', \
                 1234.56, 1234.56, 1234.56, 1234.56, 0.2, 0.1, 59, 0.06, 0)",
    )
    .execute(&pool)
    .await
    .expect("insert snapshot");

    sqlx::query(
        "INSERT INTO users (username, display_name, password_hash, role) \
         VALUES ('owner', 'Owner', 'hash', 'owner')",
    )
    .execute(&pool)
    .await
    .expect("insert user");

    // Apply the remaining migrations (the "new version").
    Migrator::new(migrations_dir)
        .await
        .expect("load all migrations")
        .run(&pool)
        .await
        .expect("apply remaining migrations");

    // Existing rows survive with exact values.
    let accounts: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM accounts")
        .fetch_one(&pool)
        .await
        .expect("count accounts");
    assert_eq!(accounts.0, 1);

    let balances: (String,) = sqlx::query_as("SELECT amount::text FROM account_balances")
        .fetch_one(&pool)
        .await
        .expect("read balance");
    assert_eq!(balances.0, "1234.56");

    let snapshots: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM net_worth_snapshots")
        .fetch_one(&pool)
        .await
        .expect("count snapshots");
    assert_eq!(snapshots.0, 1);

    let users: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(&pool)
        .await
        .expect("count users");
    assert_eq!(users.0, 1);

    // The new schema exists and is writable.
    sqlx::query(
        "INSERT INTO securities (symbol, name, asset_class, currency) \
         VALUES ('VTI', 'Total Market', 'equity', 'USD')",
    )
    .execute(&pool)
    .await
    .expect("insert security");

    let securities: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM securities")
        .fetch_one(&pool)
        .await
        .expect("count securities");
    assert_eq!(securities.0, 1);

    pool.close().await;
    sqlx::query(&format!("DROP DATABASE \"{db_name}\" WITH (FORCE)"))
        .execute(&admin)
        .await
        .expect("drop test database");
    let _ = std::fs::remove_dir_all(previous_dir);
}
