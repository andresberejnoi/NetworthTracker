//! HTTP API for NetworthTracker.
//!
//! This service is a standalone JSON API and the only integration point
//! between the `backend/` and `frontend/` projects. The frontend never imports
//! backend code; it consumes these endpoints over HTTP.

mod accounts;
mod data;
mod dto;
mod error;
mod networth;

pub use dto::{
    AccountDto, BalanceDto, BatchSaveDto, BreakdownRowDto, CreateAccountRequest, NetWorthDto,
    PolicyDto, RecordBalanceRequest, RecordBalancesRequest, RowErrorDto, SnapshotDto,
};

use std::sync::Arc;

use axum::{
    extract::DefaultBodyLimit,
    http::{HeaderValue, Method},
    routing::{get, post},
    Json, Router,
};
use networth_application::{
    AccountRepository, AccountService, ImportService, NetWorthRepository, NetWorthService,
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

/// Shared application state.
#[derive(Clone)]
pub struct AppState {
    /// Account use cases.
    pub accounts: Arc<AccountService>,
    /// Net worth use cases.
    pub networth: Arc<NetWorthService>,
    /// Import/export use cases.
    pub data: Arc<ImportService>,
}

impl AppState {
    /// Build state over the account and net worth repositories.
    pub fn new(
        accounts: Arc<dyn AccountRepository>,
        networth: Arc<dyn NetWorthRepository>,
    ) -> Self {
        Self {
            accounts: Arc::new(AccountService::new(accounts.clone())),
            networth: Arc::new(NetWorthService::new(accounts.clone(), networth)),
            data: Arc::new(ImportService::new(accounts)),
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
