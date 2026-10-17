//! API-level tests for accounts and balances.

mod common;

use axum::http::StatusCode;
use serde_json::json;

#[tokio::test]
async fn creates_lists_and_fetches_an_account() {
    let app = common::router();

    let (status, created) = common::post_json(
        &app,
        "/api/v1/accounts",
        json!({
            "name": "Fidelity 401k",
            "account_type": "401k",
            "institution": "Fidelity",
            "currency": "usd"
        }),
    )
    .await;

    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(created["account_type"], "401k");
    assert_eq!(created["currency"], "USD");
    assert_eq!(created["ownership_scope"], "household");
    assert_eq!(created["archived"], false);
    let id = created["id"].as_str().expect("id").to_string();

    let (status, list) = common::get(&app, "/api/v1/accounts").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(list.as_array().expect("array").len(), 1);

    let (status, one) = common::get(&app, &format!("/api/v1/accounts/{id}")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(one["name"], "Fidelity 401k");
}

#[tokio::test]
async fn rejects_an_unknown_account_type() {
    let app = common::router();
    let (status, body) = common::post_json(
        &app,
        "/api/v1/accounts",
        json!({ "name": "Mystery", "account_type": "beanie_babies" }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"]
        .as_str()
        .unwrap()
        .contains("unknown account type"));
}

#[tokio::test]
async fn rejects_a_blank_name() {
    let app = common::router();
    let (status, _) = common::post_json(
        &app,
        "/api/v1/accounts",
        json!({ "name": "   ", "account_type": "checking" }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn missing_account_is_not_found() {
    let app = common::router();
    let id = uuid::Uuid::new_v4();
    let (status, _) = common::get(&app, &format!("/api/v1/accounts/{id}")).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn records_and_lists_balances_exactly() {
    let app = common::router();
    let id = common::create_account(&app, "Savings", "savings").await;

    let (status, balance) = common::post_json(
        &app,
        &format!("/api/v1/accounts/{id}/balances"),
        json!({ "as_of": "2026-10-17", "amount": "0.000000000000000001" }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "record balance: {balance}");
    assert_eq!(balance["amount"], "0.000000000000000001");
    assert_eq!(balance["currency"], "USD");
    assert_eq!(balance["source"], "manual");

    let (status, list) = common::get(&app, &format!("/api/v1/accounts/{id}/balances")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(list.as_array().expect("array").len(), 1);
}

#[tokio::test]
async fn balance_for_the_same_day_is_replaced() {
    let app = common::router();
    let id = common::create_account(&app, "Checking", "checking").await;
    let path = format!("/api/v1/accounts/{id}/balances");

    common::post_json(
        &app,
        &path,
        json!({ "as_of": "2026-10-17", "amount": "100.00" }),
    )
    .await;
    common::post_json(
        &app,
        &path,
        json!({ "as_of": "2026-10-17", "amount": "250.50" }),
    )
    .await;

    let (_, list) = common::get(&app, &path).await;
    let items = list.as_array().expect("array");
    assert_eq!(items.len(), 1, "same-day balance should be replaced");
    assert_eq!(items[0]["amount"], "250.50");
}

#[tokio::test]
async fn rejects_a_currency_mismatch() {
    let app = common::router();
    let id = common::create_account(&app, "Euro account", "checking").await;
    // The account defaults to USD; a EUR balance must be refused.
    let (status, body) = common::post_json(
        &app,
        &format!("/api/v1/accounts/{id}/balances"),
        json!({ "as_of": "2026-10-17", "amount": "10.00", "currency": "EUR" }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"]
        .as_str()
        .unwrap()
        .contains("currency mismatch"));
}

#[tokio::test]
async fn rejects_an_invalid_date() {
    let app = common::router();
    let id = common::create_account(&app, "Cash", "cash").await;
    let (status, _) = common::post_json(
        &app,
        &format!("/api/v1/accounts/{id}/balances"),
        json!({ "as_of": "17/10/2026", "amount": "1.00" }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn archiving_hides_an_account_but_keeps_history() {
    let app = common::router();
    let id = common::create_account(&app, "Old brokerage", "taxable_brokerage").await;
    common::post_json(
        &app,
        &format!("/api/v1/accounts/{id}/balances"),
        json!({ "as_of": "2026-01-01", "amount": "500.00" }),
    )
    .await;

    let (status, _) =
        common::post_json(&app, &format!("/api/v1/accounts/{id}/archive"), json!({})).await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (_, list) = common::get(&app, "/api/v1/accounts").await;
    assert!(
        list.as_array().expect("array").is_empty(),
        "archived accounts are hidden by default"
    );

    let (_, all) = common::get(&app, "/api/v1/accounts?include_archived=true").await;
    assert_eq!(all.as_array().expect("array").len(), 1);

    // The archived account's history is still available.
    let (status, balances) = common::get(&app, &format!("/api/v1/accounts/{id}/balances")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(balances.as_array().expect("array").len(), 1);
}
