//! API-level tests for owner-only GUI full backups.

mod common;

use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use axum::http::StatusCode;
use serde_json::{json, Value};

const TEST_PASSWORD: &str = "test-only-not-a-secret";
const FAKE_SCRIPT: &str = r#"#!/usr/bin/env bash
set -euo pipefail
sleep "${FAKE_BACKUP_SLEEP:-0}"
: "${BACKUP_TIMESTAMP:?}"
printf 'database-%s' "$BACKUP_TIMESTAMP" > "$BACKUP_DIR/networth-$BACKUP_TIMESTAMP.sql.gz.gpg"
if [[ -d "$DOCUMENT_DIR" ]]; then
  printf 'documents-%s' "$BACKUP_TIMESTAMP" > "$BACKUP_DIR/networth-documents-$BACKUP_TIMESTAMP.tar.gz.gpg"
fi
"#;

struct TestEnv {
    root: PathBuf,
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
        std::env::remove_var("FULL_BACKUP_SCRIPT");
        std::env::remove_var("BACKUP_DIR");
        std::env::remove_var("DOCUMENT_DIR");
        std::env::remove_var("BACKUP_PASSPHRASE");
        std::env::remove_var("FAKE_BACKUP_SLEEP");
    }
}

fn configure() -> TestEnv {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "networth-gui-backups-{}-{suffix}",
        std::process::id()
    ));
    let backups = root.join("backups");
    let documents = root.join("documents");
    std::fs::create_dir_all(&backups).expect("backup dir");
    std::fs::create_dir_all(&documents).expect("document dir");
    let script = root.join("fake-backup.sh");
    std::fs::write(&script, FAKE_SCRIPT).expect("fake script");
    let mut permissions = std::fs::metadata(&script)
        .expect("script metadata")
        .permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&script, permissions).expect("script permissions");

    std::env::set_var("FULL_BACKUP_SCRIPT", &script);
    std::env::set_var("BACKUP_DIR", &backups);
    std::env::set_var("DOCUMENT_DIR", &documents);
    std::env::set_var("BACKUP_PASSPHRASE", "test-only-backup-passphrase");
    TestEnv { root }
}

async fn setup_owner(app: &axum::Router) -> String {
    let (status, body) = common::post_json(
        app,
        "/api/v1/auth/setup",
        json!({ "username": "owner", "password": TEST_PASSWORD, "display_name": "Owner" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "setup: {body}");
    body["token"].as_str().expect("token").to_string()
}

async fn login(app: &axum::Router, username: &str) -> String {
    let (status, body) = common::post_json(
        app,
        "/api/v1/auth/login",
        json!({ "username": username, "password": TEST_PASSWORD }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "login {username}: {body}");
    body["token"].as_str().expect("token").to_string()
}

async fn create_viewer(app: &axum::Router, owner_token: &str) -> String {
    let (status, body) = common::post_json_auth(
        app,
        "/api/v1/users",
        json!({ "username": "viewer", "password": TEST_PASSWORD, "role": "viewer" }),
        owner_token,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "viewer: {body}");
    login(app, "viewer").await
}

fn backup_name(body: &Value, field: &str) -> String {
    body[field]["name"].as_str().expect(field).to_string()
}

#[tokio::test]
async fn owner_backup_workflow_and_safety_rules() {
    let _env = configure();
    let app = common::router_with_auth();
    let owner_token = setup_owner(&app).await;

    let (status, empty) = common::get_auth(&app, "/api/v1/admin/backups", &owner_token).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(empty.as_array().expect("list").len(), 0);

    let viewer_token = create_viewer(&app, &owner_token).await;
    let (status, _) = common::get_auth(&app, "/api/v1/admin/backups", &viewer_token).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _) =
        common::post_json_auth(&app, "/api/v1/admin/backups", json!({}), &viewer_token).await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let (status, created) =
        common::post_json_auth(&app, "/api/v1/admin/backups", json!({}), &owner_token).await;
    assert_eq!(status, StatusCode::CREATED, "create: {created}");
    let database_name = backup_name(&created, "database");
    let documents_name = backup_name(&created, "documents");
    assert!(database_name.ends_with(".sql.gz.gpg"));
    assert!(documents_name.ends_with(".tar.gz.gpg"));

    let (status, listed) = common::get_auth(&app, "/api/v1/admin/backups", &owner_token).await;
    assert_eq!(status, StatusCode::OK, "list: {listed}");
    let listed = listed.as_array().expect("list");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0]["database"]["name"], database_name);
    assert_eq!(listed[0]["documents"]["name"], documents_name);

    let (status, bytes, content_type) = common::get_raw_with_token(
        &app,
        &format!("/api/v1/admin/backups/{database_name}/download"),
        Some(&owner_token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(content_type.as_deref(), Some("application/octet-stream"));
    assert!(String::from_utf8_lossy(&bytes).starts_with("database-"));

    let (status, bytes, _) = common::get_raw_with_token(
        &app,
        &format!("/api/v1/admin/backups/{documents_name}/download"),
        Some(&owner_token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(String::from_utf8_lossy(&bytes).starts_with("documents-"));

    let (status, _, _) = common::get_raw_with_token(
        &app,
        "/api/v1/admin/backups/not-a-backup/download",
        Some(&owner_token),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    std::env::remove_var("BACKUP_PASSPHRASE");
    let (status, body) =
        common::post_json_auth(&app, "/api/v1/admin/backups", json!({}), &owner_token).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "passphrase: {body}");
    std::env::set_var("BACKUP_PASSPHRASE", "test-only-backup-passphrase");

    std::env::set_var("FAKE_BACKUP_SLEEP", "1");
    let first = common::post_json_auth(&app, "/api/v1/admin/backups", json!({}), &owner_token);
    let second = common::post_json_auth(&app, "/api/v1/admin/backups", json!({}), &owner_token);
    let (first, second) = tokio::join!(first, second);
    let statuses = [first.0, second.0];
    assert_eq!(
        statuses
            .iter()
            .filter(|status| **status == StatusCode::CREATED)
            .count(),
        1,
        "expected one successful backup: {first:?} {second:?}"
    );
    assert_eq!(
        statuses
            .iter()
            .filter(|status| **status == StatusCode::CONFLICT)
            .count(),
        1,
        "expected one busy backup: {first:?} {second:?}"
    );
}
