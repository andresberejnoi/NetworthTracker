//! API-level tests for full backup and restore.

mod common;

const TEST_PASSWORD: &str = "test-only-not-a-secret";
use axum::http::StatusCode;
use serde_json::json;

#[tokio::test]
async fn backup_and_restore_round_trip() {
    let app = common::router();
    let account = common::create_account(&app, "Checking", "checking").await;
    common::record_balance(&app, &account, "2026-01-01", "100.00").await;

    let (status, _) = common::post_json(
        &app,
        "/api/v1/networth/snapshots",
        json!({ "as_of": "2026-01-01", "owner_age": 35 }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    let brokerage = common::create_account(&app, "Brokerage", "taxable_brokerage").await;
    let (status, _) = common::post_json(
        &app,
        "/api/v1/securities",
        json!({ "symbol": "VTI", "name": "Total Market", "asset_class": "equity" }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let (status, _) = common::post_json(
        &app,
        &format!("/api/v1/accounts/{brokerage}/holdings"),
        json!({
            "symbol": "VTI",
            "lots": [
                { "acquired_on": "2026-01-01", "quantity": "10", "cost_basis": "1000.00" },
                { "acquired_on": "2026-02-01", "quantity": "5", "cost_basis": "600.00" }
            ]
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let (status, _) = common::post_json(
        &app,
        &format!("/api/v1/accounts/{brokerage}/sales"),
        json!({
            "symbol": "VTI",
            "disposed_on": "2026-06-01",
            "quantity": "2",
            "gross_proceeds": "300.00",
            "fees": "2.00"
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    let (_, asset) = common::post_json(
        &app,
        "/api/v1/crypto/assets",
        json!({ "symbol": "BTC", "name": "Bitcoin", "chain": "bitcoin", "decimals": 18 }),
    )
    .await;
    let asset_id = asset["id"].as_str().expect("asset id");
    let (_, wallet) = common::post_json(
        &app,
        "/api/v1/crypto/wallets",
        json!({
            "asset_id": asset_id,
            "kind": "manual",
            "chain": "bitcoin",
            "label": "Coinbase"
        }),
    )
    .await;
    let wallet_id = wallet["id"].as_str().expect("wallet id");
    for (acquired_on, quantity, cost_basis) in [
        ("2026-01-01", "0.5", "10000.00"),
        ("2026-02-01", "0.25", "6000.00"),
    ] {
        let (status, _) = common::post_json(
            &app,
            &format!("/api/v1/crypto/wallets/{wallet_id}/lots"),
            json!({
                "acquired_on": acquired_on,
                "quantity": quantity,
                "cost_basis": cost_basis
            }),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
    }
    let (status, _) = common::post_json(
        &app,
        &format!("/api/v1/crypto/wallets/{wallet_id}/sales"),
        json!({
            "symbol": "BTC",
            "disposed_on": "2026-06-01",
            "quantity": "0.2",
            "gross_proceeds": "7000.00",
            "fees": "7.00"
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    let (status, _) = common::post_json(
        &app,
        "/api/v1/budget/income",
        json!({ "year": 2026, "amount": "120000.00" }),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = common::post_json(
        &app,
        "/api/v1/budget/plan",
        json!({
            "year": 2026,
            "allocations": [{ "category": "fixed_costs", "percentage": "100" }]
        }),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = common::post_json(
        &app,
        "/api/v1/budget/spending",
        json!({
            "year": 2026,
            "month": 1,
            "category": "fixed_costs",
            "amount": "1000.00"
        }),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (status, _) = common::post_json(
        &app,
        "/api/v1/aggregation/config",
        json!({ "provider": "fake", "enabled": true, "token": "test-token" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = common::post_json(&app, "/api/v1/aggregation/sync", json!({})).await;
    assert_eq!(status, StatusCode::OK);

    let (status, backup) = common::get(&app, "/api/v1/backup").await;
    assert_eq!(status, StatusCode::OK, "backup: {backup}");
    assert_eq!(backup["version"], "2");
    assert_eq!(backup["accounts"].as_array().expect("accounts").len(), 3);
    assert_eq!(backup["balances"].as_array().expect("balances").len(), 2);
    assert_eq!(backup["snapshots"].as_array().expect("snapshots").len(), 1);
    assert_eq!(
        backup["securities"].as_array().expect("securities").len(),
        1
    );
    assert_eq!(backup["holdings"].as_array().expect("holdings").len(), 1);
    assert_eq!(
        backup["holdings"][0]["lots"]
            .as_array()
            .expect("lots")
            .len(),
        2
    );
    assert_eq!(
        backup["holdings"][0]["sales"]
            .as_array()
            .expect("sales")
            .len(),
        1
    );
    assert_eq!(
        backup["crypto_assets"]
            .as_array()
            .expect("crypto assets")
            .len(),
        1
    );
    assert_eq!(
        backup["crypto_wallets"]
            .as_array()
            .expect("crypto wallets")
            .len(),
        1
    );
    assert_eq!(
        backup["crypto_wallets"][0]["lots"]
            .as_array()
            .expect("crypto lots")
            .len(),
        2
    );
    assert_eq!(
        backup["crypto_wallets"][0]["sales"]
            .as_array()
            .expect("crypto sales")
            .len(),
        1
    );
    assert_eq!(
        backup["budget_incomes"].as_array().expect("incomes").len(),
        1
    );
    assert_eq!(backup["budget_plans"].as_array().expect("plans").len(), 1);
    assert_eq!(
        backup["budget_spending"]
            .as_array()
            .expect("spending")
            .len(),
        1
    );
    assert_eq!(
        backup["aggregation_links"].as_array().expect("links").len(),
        1
    );
    assert!(!backup.to_string().contains("test-token"));

    let fresh = common::router();
    let (status, report) =
        common::post_json(&fresh, "/api/v1/backup/restore", backup.clone()).await;
    assert_eq!(status, StatusCode::OK, "restore: {report}");
    assert_eq!(report["accounts_created"].as_u64(), Some(3));
    assert_eq!(report["balances_saved"].as_u64(), Some(2));
    assert_eq!(report["snapshots_restored"].as_u64(), Some(1));
    assert_eq!(report["securities_created"].as_u64(), Some(1));
    assert_eq!(report["holdings_restored"].as_u64(), Some(1));
    assert_eq!(report["lots_restored"].as_u64(), Some(2));
    assert_eq!(report["sales_restored"].as_u64(), Some(1));
    assert_eq!(report["crypto_assets_created"].as_u64(), Some(1));
    assert_eq!(report["crypto_wallets_restored"].as_u64(), Some(1));
    assert_eq!(report["crypto_balances_saved"].as_u64(), Some(1));
    assert_eq!(report["crypto_lots_restored"].as_u64(), Some(2));
    assert_eq!(report["crypto_sales_restored"].as_u64(), Some(1));
    assert_eq!(report["budget_incomes_saved"].as_u64(), Some(1));
    assert_eq!(report["budget_plans_saved"].as_u64(), Some(1));
    assert_eq!(report["budget_spending_saved"].as_u64(), Some(1));
    assert_eq!(report["aggregation_links_saved"].as_u64(), Some(1));

    let (_, accounts) = common::get(&fresh, "/api/v1/accounts").await;
    assert_eq!(accounts.as_array().expect("accounts").len(), 3);
    let (_, snapshots) = common::get(&fresh, "/api/v1/networth/snapshots").await;
    assert_eq!(snapshots.as_array().expect("snapshots").len(), 1);
    let (_, restored) = common::get(&fresh, "/api/v1/backup").await;
    assert_eq!(
        restored["securities"].as_array().expect("securities").len(),
        1
    );
    assert_eq!(restored["holdings"].as_array().expect("holdings").len(), 1);
    assert_eq!(
        restored["crypto_wallets"]
            .as_array()
            .expect("wallets")
            .len(),
        1
    );
    assert_eq!(
        restored["aggregation_links"]
            .as_array()
            .expect("links")
            .len(),
        1
    );
    assert_eq!(restored["securities"], backup["securities"]);
    assert_eq!(restored["holdings"], backup["holdings"]);
    assert_eq!(restored["crypto_assets"], backup["crypto_assets"]);
    assert_eq!(restored["crypto_wallets"], backup["crypto_wallets"]);
    assert_eq!(restored["budget_incomes"], backup["budget_incomes"]);
    assert_eq!(restored["budget_plans"], backup["budget_plans"]);
    assert_eq!(restored["budget_spending"], backup["budget_spending"]);
    assert_eq!(restored["aggregation_links"], backup["aggregation_links"]);

    // Restoring the same backup again is idempotent.
    let (status, again) = common::post_json(&fresh, "/api/v1/backup/restore", backup).await;
    assert_eq!(status, StatusCode::OK, "second restore: {again}");
    assert_eq!(again["accounts_created"].as_u64(), Some(0));
    assert_eq!(again["snapshots_skipped"].as_u64(), Some(1));
    assert_eq!(again["holdings_skipped"].as_u64(), Some(1));
    assert_eq!(again["crypto_wallets_skipped"].as_u64(), Some(1));
    assert_eq!(again["securities_created"].as_u64(), Some(0));
    assert_eq!(again["crypto_assets_created"].as_u64(), Some(0));

    let (_, restored) = common::get(&fresh, "/api/v1/backup").await;
    assert_eq!(restored["holdings"].as_array().expect("holdings").len(), 1);
    assert_eq!(
        restored["crypto_wallets"]
            .as_array()
            .expect("wallets")
            .len(),
        1
    );
    let (_, snapshots) = common::get(&fresh, "/api/v1/networth/snapshots").await;
    assert_eq!(snapshots.as_array().expect("snapshots").len(), 1);
}

#[tokio::test]
async fn restores_a_v1_backup_with_empty_new_sections() {
    let app = common::router();
    let (status, report) = common::post_json(
        &app,
        "/api/v1/backup/restore",
        json!({
            "version": "1",
            "exported_at": "2026-01-01T00:00:00Z",
            "accounts": [{
                "name": "Legacy checking",
                "account_type": "checking",
                "institution": null,
                "currency": "USD",
                "ownership_scope": "household",
                "archived": false,
                "owner_username": null
            }],
            "balances": [{
                "account_name": "Legacy checking",
                "as_of": "2026-01-01",
                "amount": "1234.56",
                "currency": "USD",
                "source": "manual"
            }],
            "snapshots": []
        }),
    )
    .await;

    assert_eq!(status, StatusCode::OK, "v1 restore: {report}");
    assert_eq!(report["accounts_created"].as_u64(), Some(1));
    assert_eq!(report["balances_saved"].as_u64(), Some(1));
    assert_eq!(report["securities_created"].as_u64(), Some(0));
    assert_eq!(report["crypto_wallets_restored"].as_u64(), Some(0));
    assert_eq!(report["aggregation_links_saved"].as_u64(), Some(0));
}

#[tokio::test]
async fn restore_rejects_an_unknown_backup_version() {
    let app = common::router();
    let (status, body) = common::post_json(
        &app,
        "/api/v1/backup/restore",
        json!({
            "version": "99",
            "exported_at": "2026-01-01T00:00:00Z",
            "accounts": [],
            "balances": [],
            "snapshots": []
        }),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"].as_str().expect("error").contains("version"));
}

#[tokio::test]
async fn only_an_owner_can_back_up() {
    let app = common::router_with_auth();
    let (status, setup) = common::post_json(
        &app,
        "/api/v1/auth/setup",
        json!({ "username": "owner", "password": TEST_PASSWORD }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let owner_token = setup["token"].as_str().expect("token").to_string();

    common::post_json_auth(
        &app,
        "/api/v1/users",
        json!({ "username": "viewer", "password": TEST_PASSWORD, "role": "viewer" }),
        &owner_token,
    )
    .await;

    let (_, session) = common::post_json(
        &app,
        "/api/v1/auth/login",
        json!({ "username": "viewer", "password": TEST_PASSWORD }),
    )
    .await;
    let viewer_token = session["token"].as_str().expect("token");

    let (status, _) = common::get_auth(&app, "/api/v1/backup", viewer_token).await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let (status, _) = common::get_auth(&app, "/api/v1/backup", &owner_token).await;
    assert_eq!(status, StatusCode::OK);
}
