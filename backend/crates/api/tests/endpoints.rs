//! API-level tests for the walking-skeleton endpoints.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use tower::ServiceExt;

async fn get(path: &str) -> (StatusCode, serde_json::Value) {
    let response = networth_api::app()
        .oneshot(
            Request::builder()
                .uri(path)
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("send request");

    let status = response.status();
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("collect body")
        .to_bytes();
    let json = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
    (status, json)
}

#[tokio::test]
async fn health_reports_ok() {
    let (status, body) = get("/health").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ok");
}

#[tokio::test]
async fn ping_reports_service_metadata() {
    let (status, body) = get("/api/v1/ping").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["message"], "pong");
    assert_eq!(body["service"], "NetworthTracker");
    assert!(body["version"].is_string());
}

#[tokio::test]
async fn unknown_route_is_not_found() {
    let (status, _) = get("/api/v1/does-not-exist").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
