//! Entry point for the NetworthTracker HTTP API.

use std::sync::Arc;

use networth_adapters::{
    CoinPaprikaPriceProvider, FakeAggregationProvider, FakeChainProvider,
    PostgresAccountRepository, PublicChainProvider, StaticPriceProvider,
};
use networth_api::{app, AppState, Repositories};
use networth_application::{
    AggregationProvider, ChainDataProvider, PriceProvider, ProviderConfig, TokenCipher,
};
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
        crypto: repository.clone(),
        budget: repository.clone(),
        aggregation: repository.clone(),
        migrations: repository.clone(),
        documents: repository.clone(),
        prices: repository.clone(),
        fx: repository,
    };

    let provider_config = ProviderConfig::from_env();
    let provider: Option<Arc<dyn PriceProvider>> = match provider_config.provider.as_deref() {
        Some("static") => Some(Arc::new(StaticPriceProvider::from_env())),
        Some("coinpaprika") => Some(Arc::new(CoinPaprikaPriceProvider::new())),
        Some(other) => {
            tracing::warn!("unknown PRICE_PROVIDER '{other}'; using cached and manual prices only");
            None
        }
        None => None,
    };
    if provider_config.api_key.is_some() && provider.is_none() {
        tracing::warn!("PRICE_PROVIDER_API_KEY is set but no provider is enabled");
    }

    let chain_provider: Option<Arc<dyn ChainDataProvider>> = match std::env::var("CHAIN_PROVIDER")
        .ok()
        .map(|value| value.trim().to_ascii_lowercase())
        .as_deref()
    {
        Some("fake") => Some(Arc::new(FakeChainProvider::from_env())),
        Some("public") => Some(Arc::new(PublicChainProvider::new(None))),
        Some("etherscan") => {
            let key = std::env::var("CHAIN_PROVIDER_API_KEY").ok();
            if key.is_none() {
                tracing::warn!(
                    "CHAIN_PROVIDER=etherscan requires CHAIN_PROVIDER_API_KEY; using public Ethereum"
                );
            }
            Some(Arc::new(PublicChainProvider::new(key)))
        }
        Some("none") | None => None,
        Some(other) => {
            tracing::warn!("unknown CHAIN_PROVIDER '{other}'; address sync is disabled");
            None
        }
    };

    let aggregation_provider: Option<Arc<dyn AggregationProvider>> =
        match std::env::var("AGGREGATION_PROVIDER")
            .ok()
            .map(|value| value.trim().to_ascii_lowercase())
            .as_deref()
        {
            Some("fake") => Some(Arc::new(FakeAggregationProvider)),
            Some("none") | None => None,
            Some(other) => {
                tracing::warn!("unknown AGGREGATION_PROVIDER '{other}'; aggregation is disabled");
                None
            }
        };
    let aggregation_cipher = TokenCipher::from_env();
    if aggregation_provider.is_some() && aggregation_cipher.is_none() {
        tracing::warn!(
            "AGGREGATION_PROVIDER is set but AGGREGATION_ENCRYPTION_KEY is missing; tokens cannot be stored"
        );
    }

    let state = AppState::new(
        repositories,
        provider,
        chain_provider,
        aggregation_provider,
        aggregation_cipher,
        auth_enabled,
    );

    let addr = std::env::var("BACKEND_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".to_string());
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .unwrap_or_else(|error| panic!("failed to bind {addr}: {error}"));

    tracing::info!("NetworthTracker API listening on {addr}");

    axum::serve(listener, app(state))
        .await
        .unwrap_or_else(|error| panic!("server error: {error}"));
}
