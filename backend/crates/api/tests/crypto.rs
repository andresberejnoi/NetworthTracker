//! API-level tests for crypto assets, wallets, balances, sync, and net worth.

mod common;

use axum::http::StatusCode;
use serde_json::json;

async fn create_asset(app: &axum::Router, symbol: &str, chain: &str) -> String {
    let (status, asset) = common::post_json(
        app,
        "/api/v1/crypto/assets",
        json!({
            "symbol": symbol,
            "name": format!("{symbol} asset"),
            "chain": chain,
            "decimals": 18
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "asset: {asset}");
    asset["id"].as_str().expect("asset id").to_string()
}

async fn create_wallet(
    app: &axum::Router,
    asset_id: &str,
    kind: &str,
    chain: &str,
    label: &str,
    address: Option<&str>,
) -> String {
    let (status, wallet) = common::post_json(
        app,
        "/api/v1/crypto/wallets",
        json!({
            "asset_id": asset_id,
            "kind": kind,
            "chain": chain,
            "label": label,
            "address": address
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "wallet: {wallet}");
    wallet["id"].as_str().expect("wallet id").to_string()
}

#[tokio::test]
async fn manual_wallet_aggregates_and_values_in_net_worth() {
    let app = common::router();
    let asset_id = create_asset(&app, "BTC", "bitcoin").await;
    let wallet_id = create_wallet(&app, &asset_id, "manual", "bitcoin", "Coinbase", None).await;

    let (status, balance) = common::post_json(
        &app,
        &format!("/api/v1/crypto/wallets/{wallet_id}/balances"),
        json!({ "as_of": "2026-01-01", "quantity": "0.003" }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "balance: {balance}");
    assert_eq!(balance["quantity"], "0.003");
    assert_eq!(balance["source"], "manual");

    common::post_json(
        &app,
        "/api/v1/prices",
        json!({ "symbol": "BTC", "price": "70000.00" }),
    )
    .await;

    let (status, positions) = common::get(&app, "/api/v1/crypto/positions?as_of=2026-01-01").await;
    assert_eq!(status, StatusCode::OK, "positions: {positions}");
    assert_eq!(positions[0]["symbol"], "BTC");
    assert_eq!(positions[0]["quantity"], "0.003");
    assert_eq!(positions[0]["wallet_count"], 1);
    assert_eq!(positions[0]["value"], "210.00");

    let (status, networth) =
        common::get(&app, "/api/v1/networth?as_of=2026-01-01&owner_age=35").await;
    assert_eq!(status, StatusCode::OK, "networth: {networth}");
    assert_eq!(networth["total"], "210.00");
    assert_eq!(networth["liquid"], "210.00");
    assert_eq!(networth["by_asset_class"][0]["key"], "crypto");
    assert_eq!(networth["by_asset_class"][0]["amount"], "210.00");
}

#[tokio::test]
async fn aggregates_two_wallets_of_the_same_asset() {
    let app = common::router();
    let asset_id = create_asset(&app, "ETH", "ethereum").await;
    let first = create_wallet(&app, &asset_id, "manual", "ethereum", "Coinbase", None).await;
    let second = create_wallet(&app, &asset_id, "manual", "ethereum", "Ledger", None).await;

    for (wallet, quantity) in [(&first, "0.5"), (&second, "0.25")] {
        let (status, _) = common::post_json(
            &app,
            &format!("/api/v1/crypto/wallets/{wallet}/balances"),
            json!({ "as_of": "2026-01-01", "quantity": quantity }),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
    }

    let (_, positions) = common::get(&app, "/api/v1/crypto/positions?as_of=2026-01-01").await;
    assert_eq!(positions[0]["quantity"], "0.75");
    assert_eq!(positions[0]["wallet_count"], 2);
    assert!(positions[0]["value"].is_null());
}

#[tokio::test]
async fn address_wallet_syncs_from_the_fake_provider() {
    let address = "bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq";
    let app = common::router_with_chain(address, "0.000000000000000001");
    let asset_id = create_asset(&app, "BTC", "bitcoin").await;
    let wallet_id = create_wallet(
        &app,
        &asset_id,
        "address",
        "bitcoin",
        "Cold wallet",
        Some(address),
    )
    .await;

    let (status, balance) = common::post_json(
        &app,
        &format!("/api/v1/crypto/wallets/{wallet_id}/sync"),
        json!({}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "sync: {balance}");
    assert_eq!(balance["quantity"], "0.000000000000000001");
    assert_eq!(balance["source"], "chain");
    assert_eq!(balance["provider"], "fake");

    let (status, provider) = common::get(&app, "/api/v1/crypto/provider").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(provider["configured"], true);
    assert_eq!(provider["name"], "fake");
}

#[tokio::test]
async fn rejects_seed_phrases_and_private_keys() {
    let app = common::router();
    let asset_id = create_asset(&app, "BTC", "bitcoin").await;

    let seed =
        "abandon ability able about above absent absorb abstract absurd abuse access accident";
    let (status, body) = common::post_json(
        &app,
        "/api/v1/crypto/wallets",
        json!({
            "asset_id": asset_id,
            "kind": "address",
            "chain": "bitcoin",
            "label": "bad",
            "address": seed
        }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "seed: {body}");

    let key = format!("0x{}", "0123456789abcdef".repeat(4));
    let (status, body) = common::post_json(
        &app,
        "/api/v1/crypto/wallets",
        json!({
            "asset_id": asset_id,
            "kind": "address",
            "chain": "ethereum",
            "label": "bad",
            "address": key
        }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "key: {body}");
}

#[tokio::test]
async fn chain_sync_is_inert_without_a_provider() {
    let app = common::router();
    let asset_id = create_asset(&app, "BTC", "bitcoin").await;
    let wallet_id = create_wallet(
        &app,
        &asset_id,
        "address",
        "bitcoin",
        "Cold wallet",
        Some("bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq"),
    )
    .await;

    let (status, provider) = common::get(&app, "/api/v1/crypto/provider").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(provider["configured"], false);

    let (status, body) = common::post_json(
        &app,
        &format!("/api/v1/crypto/wallets/{wallet_id}/sync"),
        json!({}),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "sync: {body}");
}

#[tokio::test]
async fn eighteen_decimal_quantities_survive_storage_and_json() {
    let app = common::router();
    let asset_id = create_asset(&app, "BTC", "bitcoin").await;
    let wallet_id = create_wallet(&app, &asset_id, "manual", "bitcoin", "Dust", None).await;
    let dust = "0.000000000000000001";

    let (status, _) = common::post_json(
        &app,
        &format!("/api/v1/crypto/wallets/{wallet_id}/balances"),
        json!({ "as_of": "2026-01-01", "quantity": dust }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    let (_, balances) = common::get(
        &app,
        &format!("/api/v1/crypto/wallets/{wallet_id}/balances"),
    )
    .await;
    assert_eq!(balances[0]["quantity"], dust);

    let (_, positions) = common::get(&app, "/api/v1/crypto/positions?as_of=2026-01-01").await;
    assert_eq!(positions[0]["quantity"], dust);

    // One atomic unit at 10^18 USD per unit values at exactly 1.00.
    common::post_json(
        &app,
        "/api/v1/prices",
        json!({ "symbol": "BTC", "price": "1000000000000000000.00" }),
    )
    .await;
    let (_, positions) = common::get(&app, "/api/v1/crypto/positions?as_of=2026-01-01").await;
    assert_eq!(positions[0]["value"], "1.00");

    let (_, networth) = common::get(&app, "/api/v1/networth?as_of=2026-01-01&owner_age=35").await;
    assert_eq!(networth["total"], "1.00");
    assert_eq!(networth["liquid"], "1.00");
}

#[tokio::test]
async fn crypto_lots_and_fifo_sales_use_the_shared_ledger() {
    let app = common::router();
    let asset_id = create_asset(&app, "BTC", "bitcoin").await;
    let wallet_id = create_wallet(&app, &asset_id, "manual", "bitcoin", "Tax wallet", None).await;

    for (date, quantity, cost) in [
        ("2026-01-01", "10", "1000.00"),
        ("2026-02-01", "5", "600.00"),
    ] {
        let (status, lots) = common::post_json(
            &app,
            &format!("/api/v1/crypto/wallets/{wallet_id}/lots"),
            json!({
                "acquired_on": date,
                "quantity": quantity,
                "cost_basis": cost
            }),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED, "lot: {lots}");
    }

    let (_, lots) = common::get(&app, &format!("/api/v1/crypto/wallets/{wallet_id}/lots")).await;
    assert_eq!(lots.as_array().expect("lots").len(), 2);

    let (status, sale) = common::post_json(
        &app,
        &format!("/api/v1/crypto/wallets/{wallet_id}/sales"),
        json!({
            "symbol": "BTC",
            "disposed_on": "2026-06-01",
            "quantity": "12",
            "gross_proceeds": "1800.00",
            "fees": "12.00"
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "sale: {sale}");
    assert_eq!(sale["realized_gain"], "548.00");
    assert_eq!(sale["disposals"].as_array().expect("disposals").len(), 2);

    let (_, sales) = common::get(&app, &format!("/api/v1/crypto/wallets/{wallet_id}/sales")).await;
    assert_eq!(sales.as_array().expect("sales").len(), 1);
    assert_eq!(sales[0]["realized_gain"], "548.00");

    let (_, lots) = common::get(&app, &format!("/api/v1/crypto/wallets/{wallet_id}/lots")).await;
    let remaining: Vec<&str> = lots
        .as_array()
        .expect("lots")
        .iter()
        .map(|lot| lot["quantity"].as_str().expect("quantity"))
        .collect();
    assert_eq!(remaining, vec!["0", "3"]);
}
