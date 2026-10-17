//! Shared helpers for API-level tests.
//!
//! Each integration-test binary gets its own copy of this module, so not every
//! helper is used by every binary.
#![allow(dead_code)]

use std::sync::Arc;

use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
};
use http_body_util::BodyExt;
use networth_adapters::InMemoryAccountRepository;
use networth_api::{app, AppState};
use tower::ServiceExt;

/// Build an app over a fresh in-memory repository.
pub fn router() -> Router {
    app(AppState::new(Arc::new(InMemoryAccountRepository::new())))
}

/// Send a request and decode any JSON body.
pub async fn send(app: &Router, request: Request<Body>) -> (StatusCode, serde_json::Value) {
    let response = app.clone().oneshot(request).await.expect("send request");

    let status = response.status();
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("collect body")
        .to_bytes();

    let json = if bytes.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null)
    };

    (status, json)
}

/// `GET path`.
pub async fn get(app: &Router, path: &str) -> (StatusCode, serde_json::Value) {
    let request = Request::builder()
        .uri(path)
        .body(Body::empty())
        .expect("build request");
    send(app, request).await
}

/// `POST path` with a JSON body.
pub async fn post_json(
    app: &Router,
    path: &str,
    body: serde_json::Value,
) -> (StatusCode, serde_json::Value) {
    let request = Request::builder()
        .method("POST")
        .uri(path)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .expect("build request");
    send(app, request).await
}

/// Create an account and return its id.
pub async fn create_account(app: &Router, name: &str, account_type: &str) -> String {
    let (status, body) = post_json(
        app,
        "/api/v1/accounts",
        serde_json::json!({ "name": name, "account_type": account_type }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "create account: {body}");
    body["id"].as_str().expect("id").to_string()
}
