//! Entry point for the NetworthTracker HTTP API.

use std::sync::Arc;

use networth_adapters::PostgresAccountRepository;
use networth_api::{app, AppState};
use networth_application::{AccountRepository, IdentityRepository, NetWorthRepository};
use sqlx::postgres::PgPoolOptions;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| "info,tower_http=info".into()),
        )
        .init();

    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        panic!("DATABASE_URL must be set (for example postgres://USER:PASSWORD@localhost:5432/networth)")
    });

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .unwrap_or_else(|error| panic!("failed to connect to the database: {error}"));

    networth_adapters::postgres::run_migrations(&pool)
        .await
        .unwrap_or_else(|error| panic!("failed to run migrations: {error}"));

    let auth_enabled = std::env::var("AUTH_DISABLED")
        .map(|value| {
            !matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes"
            )
        })
        .unwrap_or(true);

    let repository = Arc::new(PostgresAccountRepository::new(pool));
    let accounts: Arc<dyn AccountRepository> = repository.clone();
    let networth: Arc<dyn NetWorthRepository> = repository.clone();
    let identity: Arc<dyn IdentityRepository> = repository;

    let state = AppState::new(accounts, networth, identity, auth_enabled);

    let addr = std::env::var("BACKEND_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".to_string());
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .unwrap_or_else(|error| panic!("failed to bind {addr}: {error}"));

    tracing::info!("NetworthTracker API listening on {addr}");

    axum::serve(listener, app(state))
        .await
        .unwrap_or_else(|error| panic!("server error: {error}"));
}
