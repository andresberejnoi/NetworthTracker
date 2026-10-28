//! API-level tests for securities, holdings, prices, and FX.

mod common;

use axum::http::StatusCode;
use serde_json::json;

#[tokio::test]
async fn creates_a_security_and_a_multi_lot_holding() {
    let app = common::router();
    let account = common::create_account(&app, "Brokerage", "taxable_brokerage").await;

    let (status, security) = common::post_json(
        &app,
        "/api/v1/securities",
        json!({
            "symbol": "vti",
            "name": "Total Market ETF",
            "asset_class": "equity",
            "sector": "other"
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "security: {security}");
    assert_eq!(security["symbol"], "VTI");

    let (status, holding) = common::post_json(
        &app,
        &format!("/api/v1/accounts/{account}/holdings"),
        json!({
            "symbol": "VTI",
            "lots": [
                { "acquired_on": "2026-01-01", "quantity": "10", "cost_basis": "1000.00" },
                { "acquired_on": "2026-02-01", "quantity": "5", "cost_basis": "600.00" }
            ]
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "holding: {holding}");
    assert_eq!(holding["quantity"], "15");
    assert_eq!(holding["cost_basis"], "1600.00");
    assert!(holding["price"].is_null());

    let (status, price) = common::post_json(
        &app,
        "/api/v1/prices",
        json!({ "symbol": "VTI", "price": "150.00" }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "price: {price}");
    assert_eq!(price["price"], "150.00");

    let (status, holdings) =
        common::get(&app, &format!("/api/v1/accounts/{account}/holdings")).await;
    assert_eq!(status, StatusCode::OK);
    let holding = &holdings.as_array().expect("holdings")[0];
    assert_eq!(holding["price"], "150.00");
    assert_eq!(holding["market_value"], "2250.00");
    assert_eq!(holding["unrealized_gain"], "650.00");
    assert_eq!(holding["lots"].as_array().expect("lots").len(), 2);
}

#[tokio::test]
async fn holdings_value_overrides_the_balance_in_net_worth() {
    let app = common::router();
    let account = common::create_account(&app, "Brokerage", "taxable_brokerage").await;
    common::record_balance(&app, &account, "2026-01-01", "1000.00").await;

    common::post_json(
        &app,
        "/api/v1/securities",
        json!({ "symbol": "VTI", "name": "Total Market", "asset_class": "equity" }),
    )
    .await;
    common::post_json(
        &app,
        &format!("/api/v1/accounts/{account}/holdings"),
        json!({
            "symbol": "VTI",
            "lots": [{ "acquired_on": "2026-01-01", "quantity": "10", "cost_basis": "1000.00" }]
        }),
    )
    .await;
    common::post_json(
        &app,
        "/api/v1/prices",
        json!({ "symbol": "VTI", "price": "150.00" }),
    )
    .await;

    let (status, networth) =
        common::get(&app, "/api/v1/networth?as_of=2026-01-01&owner_age=35").await;
    assert_eq!(status, StatusCode::OK, "net worth: {networth}");
    assert_eq!(networth["total"], "1500.00");
    assert_eq!(networth["liquid"], "1500.00");
}

#[tokio::test]
async fn batch_holdings_groups_lots_by_symbol() {
    let app = common::router();
    let account = common::create_account(&app, "Brokerage", "taxable_brokerage").await;
    common::post_json(
        &app,
        "/api/v1/securities",
        json!({ "symbol": "VTI", "name": "Total Market", "asset_class": "equity" }),
    )
    .await;

    let (status, result) = common::post_json(
        &app,
        &format!("/api/v1/accounts/{account}/holdings/batch"),
        json!({
            "rows": [
                { "symbol": "VTI", "acquired_on": "2026-01-01", "quantity": "10", "cost_basis": "1000.00" },
                { "symbol": "VTI", "acquired_on": "2026-02-01", "quantity": "5", "cost_basis": "600.00" },
                { "symbol": "VTI", "acquired_on": "not-a-date", "quantity": "1", "cost_basis": "1.00" }
            ]
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "batch: {result}");
    assert_eq!(result["saved"].as_u64(), Some(2));
    assert_eq!(result["errors"].as_array().expect("errors").len(), 1);

    let (_, holdings) = common::get(&app, &format!("/api/v1/accounts/{account}/holdings")).await;
    assert_eq!(holdings[0]["quantity"], "15");
}

#[tokio::test]
async fn provider_is_inert_without_configuration() {
    let app = common::router();

    let (status, body) = common::get(&app, "/api/v1/prices/provider").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["configured"], false);

    let (status, _) = common::get(&app, "/api/v1/prices/VTI").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn stores_and_reads_fx_rates() {
    let app = common::router();

    let (status, _) = common::post_json(
        &app,
        "/api/v1/fx-rates",
        json!({ "base": "USD", "quote": "EUR", "rate": "0.92" }),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (status, body) = common::get(&app, "/api/v1/fx-rates?base=USD&quote=EUR").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["rate"], "0.92");
}

#[tokio::test]
async fn holding_for_an_unknown_security_is_rejected() {
    let app = common::router();
    let account = common::create_account(&app, "Brokerage", "taxable_brokerage").await;

    let (status, _) = common::post_json(
        &app,
        &format!("/api/v1/accounts/{account}/holdings"),
        json!({
            "symbol": "NOPE",
            "lots": [{ "acquired_on": "2026-01-01", "quantity": "1", "cost_basis": "10.00" }]
        }),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
