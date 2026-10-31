//! API-level tests for the optional read-only aggregation add-on.

mod common;

use axum::http::StatusCode;
use serde_json::json;

async fn configure(app: &axum::Router, token: &str, enabled: bool) -> StatusCode {
    let (status, _) = common::post_json(
        app,
        "/api/v1/aggregation/config",
        json!({ "provider": "fake", "enabled": enabled, "token": token }),
    )
    .await;
    status
}

#[tokio::test]
async fn is_inert_until_configured() {
    let app = common::router();
    let (status, view) = common::get(&app, "/api/v1/aggregation").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(view["status"]["enabled"], false);
    assert_eq!(view["status"]["has_token"], false);
    assert_eq!(view["links"].as_array().expect("links").len(), 0);

    let (status, body) = common::post_json(&app, "/api/v1/aggregation/sync", json!({})).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "sync: {body}");
}

#[tokio::test]
async fn syncs_a_fake_provider_with_provenance() {
    let app = common::router();
    assert_eq!(configure(&app, "test-token", true).await, StatusCode::OK);

    let (status, view) = common::post_json(&app, "/api/v1/aggregation/sync", json!({})).await;
    assert_eq!(status, StatusCode::OK, "sync: {view}");
    assert_eq!(view["status"]["enabled"], true);
    assert_eq!(view["status"]["has_token"], true);
    assert_eq!(view["links"].as_array().expect("links").len(), 1);
    let link = &view["links"][0];
    assert_eq!(link["display_name"], "Fake Brokerage");
    assert_eq!(link["provider_balance"], "12345.67");
    assert_eq!(link["local_balance"], "12345.67");
    assert_eq!(link["in_effect"], "provider");
    assert_eq!(link["holdings"][0]["symbol"], "VTI");
    assert_eq!(link["holdings"][0]["quantity"], "10");
}

#[tokio::test]
async fn manual_balance_is_retained_on_resync() {
    let app = common::router();
    assert_eq!(configure(&app, "test-token", true).await, StatusCode::OK);
    let (_, view) = common::post_json(&app, "/api/v1/aggregation/sync", json!({})).await;
    let account_id = view["links"][0]["account_id"]
        .as_str()
        .expect("account id")
        .to_string();
    let today = time::OffsetDateTime::now_utc().date().to_string();
    common::record_balance(&app, &account_id, &today, "999.00").await;

    let (status, view) = common::post_json(&app, "/api/v1/aggregation/sync", json!({})).await;
    assert_eq!(status, StatusCode::OK, "sync: {view}");
    let link = &view["links"][0];
    assert_eq!(link["provider_balance"], "12345.67");
    assert_eq!(link["local_balance"], "999.00");
    assert_eq!(link["local_source"], "manual");
    assert_eq!(link["in_effect"], "manual");
}

#[tokio::test]
async fn provider_failure_is_graceful() {
    let app = common::router();
    assert_eq!(configure(&app, "fail", true).await, StatusCode::OK);
    let (status, body) = common::post_json(&app, "/api/v1/aggregation/sync", json!({})).await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR, "sync: {body}");

    let (status, view) = common::get(&app, "/api/v1/aggregation").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(view["links"].as_array().expect("links").len(), 0);
}
