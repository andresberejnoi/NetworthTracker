//! API-level tests for guided generic CSV/JSON migrations.

mod common;

use axum::http::StatusCode;
use serde_json::{json, Value};

fn portfolio_rows() -> Value {
    json!([
        {
            "record_type": "account",
            "account_name": "Checking",
            "account_type": "checking",
            "institution": "Local Bank",
            "currency": "USD"
        },
        {
            "record_type": "balance",
            "account_name": "Checking",
            "as_of": "2026-01-01",
            "amount": "100.00",
            "source": "manual"
        },
        {
            "record_type": "account",
            "account_name": "Brokerage",
            "account_type": "taxable_brokerage"
        },
        {
            "record_type": "holding_lot",
            "account_name": "Brokerage",
            "symbol": "VTI",
            "security_name": "Total Market",
            "asset_class": "equity",
            "acquired_on": "2026-01-01",
            "quantity": "10",
            "cost_basis": "1000.00"
        },
        {
            "record_type": "crypto_balance",
            "asset_symbol": "BTC",
            "asset_name": "Bitcoin",
            "chain": "bitcoin",
            "decimals": "18",
            "wallet_label": "Coinbase",
            "wallet_kind": "manual",
            "as_of": "2026-01-01",
            "quantity": "0.5",
            "source": "import"
        },
        {
            "record_type": "crypto_lot",
            "asset_symbol": "BTC",
            "asset_name": "Bitcoin",
            "chain": "bitcoin",
            "decimals": "18",
            "wallet_label": "Coinbase",
            "wallet_kind": "manual",
            "acquired_on": "2026-01-01",
            "quantity": "0.5",
            "cost_basis": "10000.00"
        }
    ])
}

fn json_request() -> Value {
    json!({
        "source": "generic_json",
        "content": portfolio_rows().to_string(),
        "mapping": {}
    })
}

#[tokio::test]
async fn previews_and_applies_a_generic_json_portfolio_idempotently() {
    let app = common::router();

    let (status, preview) =
        common::post_json(&app, "/api/v1/migrations/preview", json_request()).await;
    assert_eq!(status, StatusCode::OK, "preview: {preview}");
    assert_eq!(preview["accounts"], 2);
    assert_eq!(preview["balances"], 1);
    assert_eq!(preview["holding_lots"], 1);
    assert_eq!(preview["crypto_balances"], 1);
    assert_eq!(preview["crypto_lots"], 1);

    let (status, body) = common::post_json(
        &app,
        "/api/v1/migrations/apply",
        json!({
            "source": "generic_json",
            "content": portfolio_rows().to_string(),
            "mapping": {},
            "mode": "atomic",
            "backup_confirmed": false
        }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"].as_str().expect("error").contains("backup"));

    let mut apply = json_request();
    apply["mode"] = json!("atomic");
    apply["backup_confirmed"] = json!(true);
    let (status, report) = common::post_json(&app, "/api/v1/migrations/apply", apply.clone()).await;
    assert_eq!(status, StatusCode::OK, "apply: {report}");
    assert_eq!(report["accounts_created"], 2);
    assert_eq!(report["balances_saved"], 1);
    assert_eq!(report["holding_lots_saved"], 1);
    assert_eq!(report["crypto_wallets_created"], 1);
    assert_eq!(report["crypto_balances_saved"], 1);
    assert_eq!(report["crypto_lots_saved"], 1);
    assert!(report["migration_id"].as_str().is_some());

    let (_, backup) = common::get(&app, "/api/v1/backup").await;
    assert_eq!(backup["accounts"].as_array().expect("accounts").len(), 2);
    assert_eq!(backup["holdings"][0]["lots"][0]["original_quantity"], "10");
    assert_eq!(
        backup["crypto_wallets"][0]["balances"][0]["quantity"],
        "0.5"
    );
    assert_eq!(
        backup["crypto_wallets"][0]["lots"][0]["original_quantity"],
        "0.5"
    );

    let (status, again) = common::post_json(&app, "/api/v1/migrations/apply", apply).await;
    assert_eq!(status, StatusCode::OK, "second apply: {again}");
    assert_eq!(again["accounts_created"], 0);
    assert_eq!(again["holdings_skipped"], 1);
    assert_eq!(again["crypto_wallets_created"], 0);
    assert_eq!(again["crypto_holdings_skipped"], 1);
    assert_eq!(again["crypto_balances_saved"], 1);
}

#[tokio::test]
async fn previews_generic_csv_and_rejects_atomic_validation_errors() {
    let app = common::router();
    let csv = "record_type,account_name,account_type,as_of,amount,source\n\
account,Checking,checking,,,\n\
balance,Checking,,2026-01-01,100.00,manual\n";

    let (status, preview) = common::post_json(
        &app,
        "/api/v1/migrations/preview",
        json!({ "source": "generic_csv", "content": csv, "mapping": {} }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "preview: {preview}");
    assert_eq!(preview["accounts"], 1);
    assert_eq!(preview["balances"], 1);
    assert_eq!(preview["errors"].as_array().expect("errors").len(), 0);

    let invalid = "record_type,account_name,account_type,as_of,amount,source\n\
balance,Checking,,not-a-date,nope,manual\n";
    let (status, preview) = common::post_json(
        &app,
        "/api/v1/migrations/preview",
        json!({ "source": "generic_csv", "content": invalid, "mapping": {} }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "preview: {preview}");
    assert_eq!(preview["errors"].as_array().expect("errors").len(), 1);

    let (status, body) = common::post_json(
        &app,
        "/api/v1/migrations/apply",
        json!({
            "source": "generic_csv",
            "content": invalid,
            "mapping": {},
            "mode": "atomic",
            "backup_confirmed": true
        }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"]
        .as_str()
        .expect("error")
        .contains("validation"));

    let (_, accounts) = common::get(&app, "/api/v1/accounts").await;
    assert_eq!(accounts.as_array().expect("accounts").len(), 0);
}
