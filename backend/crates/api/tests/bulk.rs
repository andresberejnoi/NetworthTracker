//! API-level tests for bulk balance entry.

mod common;

use axum::http::StatusCode;
use serde_json::json;

fn batch_path(account_id: &str) -> String {
    format!("/api/v1/accounts/{account_id}/balances/batch")
}

#[tokio::test]
async fn partial_mode_saves_valid_rows_and_reports_the_rest() {
    let app = common::router();
    let account = common::create_account(&app, "Checking", "checking").await;

    let (status, body) = common::post_json(
        &app,
        &batch_path(&account),
        json!({
            "mode": "partial",
            "rows": [
                { "as_of": "2026-01-01", "amount": "10.00" },
                { "as_of": "2026-02-01", "amount": "not-a-number" },
                { "as_of": "2026-03-01", "amount": "30.00" }
            ]
        }),
    )
    .await;

    assert_eq!(status, StatusCode::OK, "batch: {body}");
    assert_eq!(body["mode"], "partial");
    assert_eq!(body["saved"].as_array().expect("saved").len(), 2);
    let errors = body["errors"].as_array().expect("errors");
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0]["index"], 1);
    assert!(errors[0]["error"]
        .as_str()
        .expect("error")
        .contains("invalid amount"));

    let (_, balances) = common::get(&app, &format!("/api/v1/accounts/{account}/balances")).await;
    assert_eq!(balances.as_array().expect("balances").len(), 2);
}

#[tokio::test]
async fn atomic_mode_rejects_the_whole_batch_when_a_row_is_invalid() {
    let app = common::router();
    let account = common::create_account(&app, "Savings", "savings").await;

    let (status, body) = common::post_json(
        &app,
        &batch_path(&account),
        json!({
            "mode": "atomic",
            "rows": [
                { "as_of": "2026-01-01", "amount": "10.00" },
                { "as_of": "not-a-date", "amount": "20.00" }
            ]
        }),
    )
    .await;

    assert_eq!(status, StatusCode::OK, "batch: {body}");
    assert_eq!(body["mode"], "atomic");
    assert!(body["saved"].as_array().expect("saved").is_empty());
    assert_eq!(body["errors"].as_array().expect("errors").len(), 1);

    // Nothing was written.
    let (_, balances) = common::get(&app, &format!("/api/v1/accounts/{account}/balances")).await;
    assert!(balances.as_array().expect("balances").is_empty());
}

#[tokio::test]
async fn reports_currency_mismatches_per_row() {
    let app = common::router();
    let account = common::create_account(&app, "Cash", "cash").await;

    let (status, body) = common::post_json(
        &app,
        &batch_path(&account),
        json!({
            "mode": "partial",
            "rows": [
                { "as_of": "2026-01-01", "amount": "10.00" },
                { "as_of": "2026-01-02", "amount": "20.00", "currency": "EUR" }
            ]
        }),
    )
    .await;

    assert_eq!(status, StatusCode::OK, "batch: {body}");
    assert_eq!(body["saved"].as_array().expect("saved").len(), 1);
    let errors = body["errors"].as_array().expect("errors");
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0]["index"], 1);
    assert!(errors[0]["error"]
        .as_str()
        .expect("error")
        .contains("currency mismatch"));
}

#[tokio::test]
async fn defaults_to_atomic_mode() {
    let app = common::router();
    let account = common::create_account(&app, "Brokerage", "taxable_brokerage").await;

    let (status, body) = common::post_json(
        &app,
        &batch_path(&account),
        json!({
            "rows": [
                { "as_of": "2026-01-01", "amount": "10.00" },
                { "as_of": "2026-01-02", "amount": "oops" }
            ]
        }),
    )
    .await;

    assert_eq!(status, StatusCode::OK, "batch: {body}");
    assert_eq!(body["mode"], "atomic");
    assert!(body["saved"].as_array().expect("saved").is_empty());
}

#[tokio::test]
async fn batch_for_an_unknown_account_is_not_found() {
    let app = common::router();
    let (status, _) = common::post_json(
        &app,
        &batch_path("11111111-1111-1111-1111-111111111111"),
        json!({ "rows": [{ "as_of": "2026-01-01", "amount": "10.00" }] }),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
