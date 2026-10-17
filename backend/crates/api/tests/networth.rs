//! API-level tests for net worth and snapshots.

mod common;

use axum::http::StatusCode;
use serde_json::json;

const AS_OF: &str = "2026-10-17";

async fn seed(app: &axum::Router) {
    let cash = common::create_account(app, "Cash", "cash").await;
    common::record_balance(app, &cash, AS_OF, "10000.00").await;

    let retirement = common::create_account(app, "401k", "401k").await;
    common::record_balance(app, &retirement, AS_OF, "100000.00").await;

    let home = common::create_account(app, "Home", "real_estate").await;
    common::record_balance(app, &home, AS_OF, "500000.00").await;
}

#[tokio::test]
async fn computes_total_liquid_and_accessible() {
    let app = common::router();
    seed(&app).await;

    let (status, body) = common::get(
        &app,
        &format!("/api/v1/networth?as_of={AS_OF}&owner_age=35"),
    )
    .await;

    assert_eq!(status, StatusCode::OK, "net worth: {body}");
    assert_eq!(body["total"], "610000.00");
    assert_eq!(body["liquid"], "10000.00");
    // cash 10,000 + 401k at 70% (70,000) + home at 94% (470,000).
    assert_eq!(body["accessible"], "550000.00");
    assert_eq!(body["account_count"], 3);
    assert_eq!(body["has_manual_data"], true);
    assert_eq!(body["has_stale_data"], false);
}

#[tokio::test]
async fn reports_breakdowns_by_account_type_and_asset_class() {
    let app = common::router();
    seed(&app).await;

    let (_, body) = common::get(
        &app,
        &format!("/api/v1/networth?as_of={AS_OF}&owner_age=35"),
    )
    .await;

    let classes = body["by_asset_class"].as_array().expect("by_asset_class");
    assert!(classes.iter().any(|row| row["key"] == "cash"));
    assert!(classes.iter().any(|row| row["key"] == "real_estate"));

    let types = body["by_account_type"].as_array().expect("by_account_type");
    assert!(types.iter().any(|row| row["key"] == "401k"));
}

#[tokio::test]
async fn what_if_policy_changes_accessible_without_persisting() {
    let app = common::router();
    seed(&app).await;

    // Higher tax rate: the 401k drops from 70,000 to 60,000.
    let (_, body) = common::get(
        &app,
        &format!("/api/v1/networth?as_of={AS_OF}&owner_age=35&tax_rate=0.3"),
    )
    .await;
    assert_eq!(body["accessible"], "540000.00");

    // No snapshot was created by a what-if computation.
    let (_, snapshots) = common::get(&app, "/api/v1/networth/snapshots").await;
    assert!(snapshots.as_array().expect("array").is_empty());
}

#[tokio::test]
async fn snapshots_are_persisted_and_immutable() {
    let app = common::router();
    seed(&app).await;

    let (status, snapshot) = common::post_json(
        &app,
        "/api/v1/networth/snapshots",
        json!({ "as_of": AS_OF, "owner_age": 35 }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "snapshot: {snapshot}");
    assert_eq!(snapshot["total"], "610000.00");
    assert_eq!(snapshot["scope"], "household");
    assert_eq!(snapshot["policy"]["tax_rate"], "0.2");

    // Change the underlying data.
    let cash = common::get(&app, "/api/v1/accounts")
        .await
        .1
        .as_array()
        .expect("accounts")
        .iter()
        .find(|a| a["name"] == "Cash")
        .expect("cash")
        .clone();
    let cash_id = cash["id"].as_str().expect("id").to_string();
    common::record_balance(&app, &cash_id, AS_OF, "0.00").await;

    // The stored snapshot keeps its original values.
    let (_, snapshots) = common::get(&app, "/api/v1/networth/snapshots").await;
    let stored = &snapshots.as_array().expect("array")[0];
    assert_eq!(stored["total"], "610000.00");

    // A fresh computation reflects the new data.
    let (_, current) = common::get(
        &app,
        &format!("/api/v1/networth?as_of={AS_OF}&owner_age=35"),
    )
    .await;
    assert_eq!(current["total"], "600000.00");
}

#[tokio::test]
async fn personal_scope_requires_an_owner() {
    let app = common::router();
    let (status, body) = common::get(
        &app,
        &format!("/api/v1/networth?as_of={AS_OF}&scope=personal"),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"].as_str().unwrap().contains("owner_user_id"));
}

#[tokio::test]
async fn personal_scope_excludes_other_users_private_accounts() {
    let app = common::router();
    let owner_a = "11111111-1111-1111-1111-111111111111";
    let owner_b = "22222222-2222-2222-2222-222222222222";

    let shared = common::create_account(&app, "Joint", "savings").await;
    common::record_balance(&app, &shared, AS_OF, "100.00").await;

    for (name, owner, amount) in [
        ("A private", owner_a, "20.00"),
        ("B private", owner_b, "999.00"),
    ] {
        let (_, account) = common::post_json(
            &app,
            "/api/v1/accounts",
            json!({
                "name": name,
                "account_type": "savings",
                "ownership_scope": "user",
                "owner_user_id": owner,
            }),
        )
        .await;
        let id = account["id"].as_str().expect("id").to_string();
        common::record_balance(&app, &id, AS_OF, amount).await;
    }

    let (_, household) = common::get(&app, &format!("/api/v1/networth?as_of={AS_OF}")).await;
    assert_eq!(household["total"], "1119.00");

    let (_, personal_a) = common::get(
        &app,
        &format!("/api/v1/networth?as_of={AS_OF}&scope=personal&owner_user_id={owner_a}"),
    )
    .await;
    assert_eq!(personal_a["total"], "120.00");

    let (_, other) = common::get(
        &app,
        &format!(
            "/api/v1/networth?as_of={AS_OF}&scope=personal&owner_user_id=33333333-3333-3333-3333-333333333333"
        ),
    )
    .await;
    assert_eq!(other["total"], "100.00");
}
