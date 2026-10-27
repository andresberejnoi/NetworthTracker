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
use networth_application::{AccountRepository, IdentityRepository, NetWorthRepository};
use tower::ServiceExt;

/// Build an app over a fresh in-memory repository, with auth disabled.
pub fn router() -> Router {
    build(false)
}

/// Build an app over a fresh in-memory repository, with auth enabled.
pub fn router_with_auth() -> Router {
    build(true)
}

fn build(auth_enabled: bool) -> Router {
    let repository = Arc::new(InMemoryAccountRepository::new());
    let accounts: Arc<dyn AccountRepository> = repository.clone();
    let networth: Arc<dyn NetWorthRepository> = repository.clone();
    let identity: Arc<dyn IdentityRepository> = repository;
    app(AppState::new(accounts, networth, identity, auth_enabled))
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

/// `POST path` with raw bytes and an explicit content type.
pub async fn post_bytes(
    app: &Router,
    path: &str,
    content_type: &str,
    body: Vec<u8>,
) -> (StatusCode, serde_json::Value) {
    let request = Request::builder()
        .method("POST")
        .uri(path)
        .header("content-type", content_type)
        .body(Body::from(body))
        .expect("build request");
    send(app, request).await
}

/// `GET path`, returning the raw bytes and content type.
pub async fn get_raw(app: &Router, path: &str) -> (StatusCode, Vec<u8>, Option<String>) {
    let request = Request::builder()
        .uri(path)
        .body(Body::empty())
        .expect("build request");
    let response = app.clone().oneshot(request).await.expect("send request");
    let status = response.status();
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|value| value.to_str().ok())
        .map(|value| value.to_string());
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("collect body")
        .to_bytes()
        .to_vec();
    (status, bytes, content_type)
}

/// `GET path` with a bearer token.
pub async fn get_auth(app: &Router, path: &str, token: &str) -> (StatusCode, serde_json::Value) {
    let request = Request::builder()
        .uri(path)
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .expect("build request");
    send(app, request).await
}

/// `POST path` with a JSON body and a bearer token.
pub async fn post_json_auth(
    app: &Router,
    path: &str,
    body: serde_json::Value,
    token: &str,
) -> (StatusCode, serde_json::Value) {
    let request = Request::builder()
        .method("POST")
        .uri(path)
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {token}"))
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

/// Record a balance for an account.
pub async fn record_balance(app: &Router, account_id: &str, as_of: &str, amount: &str) {
    let (status, body) = post_json(
        app,
        &format!("/api/v1/accounts/{account_id}/balances"),
        serde_json::json!({ "as_of": as_of, "amount": amount }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "record balance: {body}");
}
