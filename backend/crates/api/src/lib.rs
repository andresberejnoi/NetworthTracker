//! HTTP API for NetworthTracker.
//!
//! This service is a standalone JSON API and the only integration point
//! between the `backend/` and `frontend/` projects. The frontend never imports
//! backend code; it consumes these endpoints over HTTP.

mod accounts;
mod dto;
mod error;

pub use dto::{AccountDto, BalanceDto, CreateAccountRequest, RecordBalanceRequest};

use std::sync::Arc;

use axum::{
    http::{HeaderValue, Method},
    routing::get,
    Json, Router,
};
use networth_application::{AccountRepository, AccountService};
use serde::Serialize;
use tower_http::{
    cors::{AllowOrigin, CorsLayer},
    trace::TraceLayer,
};

use crate::accounts::{
    archive_account, create_account, get_account, list_accounts, list_balances, record_balance,
};

/// Shared application state.
#[derive(Clone)]
pub struct AppState {
    /// Account use cases.
    pub accounts: Arc<AccountService>,
}

impl AppState {
    /// Build state over an account repository.
    pub fn new(repo: Arc<dyn AccountRepository>) -> Self {
        Self {
            accounts: Arc::new(AccountService::new(repo)),
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
        .route(
            "/api/v1/accounts/{id}/archive",
            axum::routing::post(archive_account),
        )
        .route(
            "/api/v1/accounts/{id}/balances",
            get(list_balances).post(record_balance),
        )
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
