//! Entry point for the NetworthTracker HTTP API.

use std::sync::Arc;

use networth_adapters::{PostgresAccountRepository, StaticPriceProvider};
use networth_api::{app, AppState, Repositories};
use networth_application::{PriceProvider, ProviderConfig};
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
    let repositories = Repositories {
        accounts: repository.clone(),
        networth: repository.clone(),
        identity: repository.clone(),
        investments: repository.clone(),
        prices: repository.clone(),
        fx: repository,
    };

    let provider_config = ProviderConfig::from_env();
    let provider: Option<Arc<dyn PriceProvider>> = match provider_config.provider.as_deref() {
        Some("static") => Some(Arc::new(StaticPriceProvider::from_env())),
        Some(other) => {
            tracing::warn!("unknown PRICE_PROVIDER '{other}'; using cached and manual prices only");
            None
        }
        None => None,
    };
    if provider_config.api_key.is_some() && provider.is_none() {
        tracing::warn!("PRICE_PROVIDER_API_KEY is set but no provider is enabled");
    }

    let state = AppState::new(repositories, provider, auth_enabled);

    let addr = std::env::var("BACKEND_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".to_string());
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .unwrap_or_else(|error| panic!("failed to bind {addr}: {error}"));

    tracing::info!("NetworthTracker API listening on {addr}");

    axum::serve(listener, app(state))
        .await
        .unwrap_or_else(|error| panic!("server error: {error}"));
}
