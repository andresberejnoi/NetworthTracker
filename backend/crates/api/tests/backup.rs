//! API-level tests for full backup and restore.

mod common;

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

    let (status, backup) = common::get(&app, "/api/v1/backup").await;
    assert_eq!(status, StatusCode::OK, "backup: {backup}");
    assert_eq!(backup["version"], "1");
    assert_eq!(backup["accounts"].as_array().expect("accounts").len(), 1);
    assert_eq!(backup["balances"].as_array().expect("balances").len(), 1);
    assert_eq!(backup["snapshots"].as_array().expect("snapshots").len(), 1);

    let fresh = common::router();
    let (status, report) =
        common::post_json(&fresh, "/api/v1/backup/restore", backup.clone()).await;
    assert_eq!(status, StatusCode::OK, "restore: {report}");
    assert_eq!(report["accounts_created"].as_u64(), Some(1));
    assert_eq!(report["balances_saved"].as_u64(), Some(1));
    assert_eq!(report["snapshots_restored"].as_u64(), Some(1));

    let (_, accounts) = common::get(&fresh, "/api/v1/accounts").await;
    assert_eq!(accounts.as_array().expect("accounts").len(), 1);
    let (_, snapshots) = common::get(&fresh, "/api/v1/networth/snapshots").await;
    assert_eq!(snapshots.as_array().expect("snapshots").len(), 1);

    // Restoring the same backup again is idempotent.
    let (status, again) = common::post_json(&fresh, "/api/v1/backup/restore", backup).await;
    assert_eq!(status, StatusCode::OK, "second restore: {again}");
    assert_eq!(again["accounts_created"].as_u64(), Some(0));
    assert_eq!(again["snapshots_skipped"].as_u64(), Some(1));

    let (_, snapshots) = common::get(&fresh, "/api/v1/networth/snapshots").await;
    assert_eq!(snapshots.as_array().expect("snapshots").len(), 1);
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
        json!({ "username": "owner", "password": "password123" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let owner_token = setup["token"].as_str().expect("token").to_string();

    common::post_json_auth(
        &app,
        "/api/v1/users",
        json!({ "username": "viewer", "password": "password123", "role": "viewer" }),
        &owner_token,
    )
    .await;

    let (_, session) = common::post_json(
        &app,
        "/api/v1/auth/login",
        json!({ "username": "viewer", "password": "password123" }),
    )
    .await;
    let viewer_token = session["token"].as_str().expect("token");

    let (status, _) = common::get_auth(&app, "/api/v1/backup", viewer_token).await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let (status, _) = common::get_auth(&app, "/api/v1/backup", &owner_token).await;
    assert_eq!(status, StatusCode::OK);
}
