//! HTTP API for NetworthTracker.
//!
//! This service is a standalone JSON API and the only integration point
//! between the `backend/` and `frontend/` projects. The frontend never imports
//! backend code; it consumes these endpoints over HTTP.

use axum::{
    http::{HeaderValue, Method},
    routing::get,
    Json, Router,
};
use serde::Serialize;
use tower_http::{
    cors::{AllowOrigin, CorsLayer},
    trace::TraceLayer,
};

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
/// Exposed as a library function so tests and the contract check can exercise
/// the real routes without binding a socket.
pub fn app() -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/api/v1/ping", get(ping))
        .layer(TraceLayer::new_for_http())
        .layer(cors_layer())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_builds() {
        // Building the router must not panic even with no configuration set.
        let _ = app();
    }
}
