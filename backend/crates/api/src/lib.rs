//! HTTP API for NetworthTracker.
//!
//! This service is a standalone JSON API and the only integration point
//! between the `backend/` and `frontend/` projects. The frontend never imports
//! backend code; it consumes these endpoints over HTTP.

mod accounts;
mod aggregation;
mod auth;
mod backup;
mod budget;
mod crypto;
mod data;
mod dto;
mod error;
mod investments;
mod networth;

pub use auth::{AuthStatusDto, CurrentUser, SessionDto, UserDto};
pub use dto::{
    AccountDto, BalanceDto, BatchSaveDto, BreakdownRowDto, CreateAccountRequest, NetWorthDto,
    PolicyDto, RecordBalanceRequest, RecordBalancesRequest, RowErrorDto, SnapshotDto,
};

use std::sync::Arc;

use axum::{
    extract::DefaultBodyLimit,
    http::{HeaderValue, Method},
    routing::{get, patch, post},
    Json, Router,
};
use networth_application::{
    AccountRepository, AccountService, AggregationProvider, AggregationRepository,
    AggregationService, AuthService, BackupService, BudgetRepository, BudgetService,
    ChainDataProvider, CryptoRepository, CryptoService, FxRepository, FxService,
    IdentityRepository, ImportService, InvestmentRepository, InvestmentsService,
    NetWorthRepository, NetWorthService, PriceProvider, PriceRepository, PriceService, TokenCipher,
    DEFAULT_MAX_AGE_SECS,
};
use serde::Serialize;
use tower_http::{
    cors::{AllowOrigin, CorsLayer},
    trace::TraceLayer,
};

use crate::accounts::{
    archive_account, create_account, get_account, list_accounts, list_balances, record_balance,
    record_balances,
};
use crate::networth::{create_snapshot, get_net_worth, list_snapshots};

/// The repository implementations the API runs on.
pub struct Repositories {
    /// Accounts and balances.
    pub accounts: Arc<dyn AccountRepository>,
    /// Net worth snapshots.
    pub networth: Arc<dyn NetWorthRepository>,
    /// Users and sessions.
    pub identity: Arc<dyn IdentityRepository>,
    /// Securities and holdings.
    pub investments: Arc<dyn InvestmentRepository>,
    /// Crypto assets, wallets, and balances.
    pub crypto: Arc<dyn CryptoRepository>,
    /// Income, spending plans, and spending records.
    pub budget: Arc<dyn BudgetRepository>,
    /// Aggregation config, links, and provenance.
    pub aggregation: Arc<dyn AggregationRepository>,
    /// Cached and manual prices.
    pub prices: Arc<dyn PriceRepository>,
    /// FX rates.
    pub fx: Arc<dyn FxRepository>,
}

/// Shared application state.
#[derive(Clone)]
pub struct AppState {
    /// Account use cases.
    pub accounts: Arc<AccountService>,
    /// Net worth use cases.
    pub networth: Arc<NetWorthService>,
    /// Import/export use cases.
    pub data: Arc<ImportService>,
    /// Authentication and user management.
    pub auth: Arc<AuthService>,
    /// Full backup and restore.
    pub backup: Arc<BackupService>,
    /// Securities and holdings.
    pub investments: Arc<InvestmentsService>,
    /// Crypto assets, wallets, and balances.
    pub crypto: Arc<CryptoService>,
    /// Budgeting use cases.
    pub budget: Arc<BudgetService>,
    /// Optional read-only aggregation.
    pub aggregation: Arc<AggregationService>,
    /// Prices.
    pub prices: Arc<PriceService>,
    /// FX conversion.
    pub fx: Arc<FxService>,
}

impl AppState {
    /// Build state over the repository set and an optional price provider.
    pub fn new(
        repositories: Repositories,
        provider: Option<Arc<dyn PriceProvider>>,
        chain_provider: Option<Arc<dyn ChainDataProvider>>,
        aggregation_provider: Option<Arc<dyn AggregationProvider>>,
        aggregation_cipher: Option<TokenCipher>,
        auth_enabled: bool,
    ) -> Self {
        let backup = Arc::new(BackupService::new(
            repositories.accounts.clone(),
            repositories.networth.clone(),
            repositories.identity.clone(),
            repositories.investments.clone(),
            repositories.crypto.clone(),
            repositories.budget.clone(),
            repositories.aggregation.clone(),
        ));
        let prices = Arc::new(PriceService::new(
            repositories.prices,
            provider,
            DEFAULT_MAX_AGE_SECS,
        ));
        let investments = Arc::new(InvestmentsService::new(
            repositories.investments,
            prices.clone(),
        ));
        let crypto = Arc::new(CryptoService::new(
            repositories.crypto,
            prices.clone(),
            chain_provider,
        ));
        let budget = Arc::new(BudgetService::new(repositories.budget));
        let aggregation = Arc::new(AggregationService::new(
            repositories.aggregation,
            repositories.accounts.clone(),
            aggregation_provider,
            aggregation_cipher,
        ));
        let fx = Arc::new(FxService::new(repositories.fx));

        Self {
            accounts: Arc::new(AccountService::new(repositories.accounts.clone())),
            networth: Arc::new(NetWorthService::new(
                repositories.accounts.clone(),
                repositories.networth.clone(),
            )),
            data: Arc::new(ImportService::new(repositories.accounts.clone())),
            auth: Arc::new(AuthService::new(
                repositories.identity.clone(),
                auth_enabled,
            )),
            backup,
            investments,
            crypto,
            budget,
            aggregation,
            prices,
            fx,
        }
    }
}

/// Response body for `GET /health`.
#[derive(Debug, Serialize)]
pub struct Health {
    pub status: &'static str,
}

/// Response body for `GET /api/v1/ping`.
#[derive(Debug, Serialize)]
pub struct Ping {
    pub message: &'static str,
    pub service: &'static str,
    pub version: &'static str,
}

async fn health() -> Json<Health> {
    Json(Health { status: "ok" })
}

async fn ping() -> Json<Ping> {
    Json(Ping {
        message: "pong",
        service: networth_application::service_name(),
        version: networth_application::service_version(),
    })
}

/// Build the application router.
///
/// Exposed as a library function so tests, the contract check, and the binary
/// all exercise the same routes.
pub fn app(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/api/v1/ping", get(ping))
        .route("/api/v1/auth/status", get(auth::status))
        .route("/api/v1/auth/setup", post(auth::setup))
        .route("/api/v1/auth/login", post(auth::login))
        .route("/api/v1/auth/logout", post(auth::logout))
        .route("/api/v1/auth/me", get(auth::me))
        .route(
            "/api/v1/users",
            get(auth::list_users).post(auth::create_user),
        )
        .route("/api/v1/users/{id}", patch(auth::update_role))
        .route("/api/v1/accounts", get(list_accounts).post(create_account))
        .route("/api/v1/accounts/{id}", get(get_account))
        .route("/api/v1/accounts/{id}/archive", post(archive_account))
        .route(
            "/api/v1/accounts/{id}/balances",
            get(list_balances).post(record_balance),
        )
        .route(
            "/api/v1/accounts/{id}/balances/batch",
            post(record_balances),
        )
        .route(
            "/api/v1/securities",
            get(investments::list_securities).post(investments::create_security),
        )
        .route(
            "/api/v1/accounts/{id}/holdings",
            get(investments::list_holdings).post(investments::create_holding),
        )
        .route(
            "/api/v1/accounts/{id}/sales",
            get(investments::list_sales).post(investments::create_sale),
        )
        .route(
            "/api/v1/crypto/assets",
            get(crypto::list_assets).post(crypto::create_asset),
        )
        .route(
            "/api/v1/crypto/wallets",
            get(crypto::list_wallets).post(crypto::create_wallet),
        )
        .route(
            "/api/v1/crypto/wallets/{id}/balances",
            get(crypto::list_balances).post(crypto::record_balance),
        )
        .route(
            "/api/v1/crypto/wallets/{id}/sync",
            post(crypto::sync_wallet),
        )
        .route(
            "/api/v1/crypto/wallets/{id}/lots",
            get(crypto::list_lots).post(crypto::create_lot),
        )
        .route(
            "/api/v1/crypto/wallets/{id}/sales",
            get(crypto::list_sales).post(crypto::create_sale),
        )
        .route("/api/v1/crypto/positions", get(crypto::list_positions))
        .route("/api/v1/crypto/provider", get(crypto::provider_status))
        .route("/api/v1/budget", get(budget::get_budget))
        .route("/api/v1/budget/income", post(budget::set_income))
        .route("/api/v1/budget/plan", post(budget::set_plan))
        .route("/api/v1/budget/spending", post(budget::record_spending))
        .route("/api/v1/aggregation", get(aggregation::get_aggregation))
        .route("/api/v1/aggregation/config", post(aggregation::save_config))
        .route("/api/v1/aggregation/sync", post(aggregation::sync))
        .route(
            "/api/v1/accounts/{id}/holdings/batch",
            post(investments::record_holdings_batch),
        )
        .route(
            "/api/v1/accounts/{id}/holdings/{holding_id}",
            axum::routing::delete(investments::delete_holding),
        )
        .route("/api/v1/prices/provider", get(investments::provider_status))
        .route("/api/v1/prices", post(investments::set_price))
        .route("/api/v1/prices/{symbol}", get(investments::get_price))
        .route(
            "/api/v1/fx-rates",
            get(investments::get_fx_rate).post(investments::set_fx_rate),
        )
        .route("/api/v1/networth", get(get_net_worth))
        .route(
            "/api/v1/networth/snapshots",
            get(list_snapshots).post(create_snapshot),
        )
        .route(
            "/api/v1/import",
            post(data::import).layer(DefaultBodyLimit::max(10 * 1024 * 1024)),
        )
        .route("/api/v1/export", get(data::export))
        .route("/api/v1/template", get(data::template))
        .route("/api/v1/backup", get(backup::export))
        .route("/api/v1/backup/restore", post(backup::restore))
        .layer(TraceLayer::new_for_http())
        .layer(cors_layer())
        .with_state(state)
}

/// Build the CORS layer from `CORS_ALLOWED_ORIGINS` (comma-separated).
///
/// Defaults to the local frontend origins. Because the frontend is deployed
/// separately and calls this API from the browser, the allowlist is explicit
/// rather than a wildcard.
fn cors_layer() -> CorsLayer {
    let origins = std::env::var("CORS_ALLOWED_ORIGINS")
        .unwrap_or_else(|_| "http://localhost:5173,http://localhost:3000".to_string());

    let allowed: Vec<HeaderValue> = origins
        .split(',')
        .filter_map(|origin| origin.trim().parse::<HeaderValue>().ok())
        .collect();

    CorsLayer::new()
        .allow_origin(AllowOrigin::list(allowed))
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers(tower_http::cors::Any)
}
