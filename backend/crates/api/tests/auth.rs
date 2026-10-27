//! API-level tests for authentication and access control.

mod common;

const TEST_PASSWORD: &str = "test-only-not-a-secret";
use axum::http::StatusCode;
use serde_json::{json, Value};

async fn setup_owner(app: &axum::Router) -> (String, Value) {
    let (status, body) = common::post_json(
        app,
        "/api/v1/auth/setup",
        json!({ "username": "owner", "password": TEST_PASSWORD, "display_name": "Owner" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "setup: {body}");
    (
        body["token"].as_str().expect("token").to_string(),
        body["user"].clone(),
    )
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

#[tokio::test]
async fn setup_then_login_and_me() {
    let app = common::router_with_auth();

    let (_, status) = common::get(&app, "/api/v1/auth/status").await;
    assert_eq!(status["auth_enabled"], true);
    assert_eq!(status["needs_setup"], true);

    let (token, user) = setup_owner(&app).await;
    assert_eq!(user["role"], "owner");
    assert_eq!(user["username"], "owner");

    let (_, status) = common::get(&app, "/api/v1/auth/status").await;
    assert_eq!(status["needs_setup"], false);

    let (status, me) = common::get_auth(&app, "/api/v1/auth/me", &token).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(me["username"], "owner");

    let (status, session) = common::post_json(
        &app,
        "/api/v1/auth/login",
        json!({ "username": "owner", "password": TEST_PASSWORD }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(session["token"].is_string());

    let (status, _) = common::post_json(
        &app,
        "/api/v1/auth/login",
        json!({ "username": "owner", "password": "wrong-password" }),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn setup_cannot_run_twice() {
    let app = common::router_with_auth();
    setup_owner(&app).await;

    let (status, _) = common::post_json(
        &app,
        "/api/v1/auth/setup",
        json!({ "username": "other", "password": TEST_PASSWORD }),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
}

#[tokio::test]
async fn unauthenticated_requests_are_rejected() {
    let app = common::router_with_auth();
    let (status, _) = common::get(&app, "/api/v1/accounts").await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn viewer_cannot_modify_but_can_read() {
    let app = common::router_with_auth();
    let (owner_token, _) = setup_owner(&app).await;

    let (status, viewer) = common::post_json_auth(
        &app,
        "/api/v1/users",
        json!({ "username": "viewer", "password": TEST_PASSWORD, "role": "viewer" }),
        &owner_token,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "viewer: {viewer}");

    let viewer_token = login(&app, "viewer").await;

    let (status, _) = common::get_auth(&app, "/api/v1/accounts", &viewer_token).await;
    assert_eq!(status, StatusCode::OK);

    let (status, _) = common::post_json_auth(
        &app,
        "/api/v1/accounts",
        json!({ "name": "Not allowed", "account_type": "checking" }),
        &viewer_token,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let (status, _) =
        common::post_json_auth(&app, "/api/v1/import?format=csv", json!({}), &viewer_token).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn private_accounts_are_hidden_from_other_members() {
    let app = common::router_with_auth();
    let (owner_token, owner) = setup_owner(&app).await;
    assert_eq!(owner["role"], "owner");

    let (_, viewer) = common::post_json_auth(
        &app,
        "/api/v1/users",
        json!({ "username": "viewer", "password": TEST_PASSWORD, "role": "viewer" }),
        &owner_token,
    )
    .await;
    let viewer_id = viewer["id"].as_str().expect("viewer id");

    common::post_json_auth(
        &app,
        "/api/v1/users",
        json!({ "username": "editor", "password": TEST_PASSWORD, "role": "editor" }),
        &owner_token,
    )
    .await;

    let (status, account) = common::post_json_auth(
        &app,
        "/api/v1/accounts",
        json!({
            "name": "Viewer private",
            "account_type": "savings",
            "ownership_scope": "user",
            "owner_user_id": viewer_id
        }),
        &owner_token,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "account: {account}");
    let account_id = account["id"].as_str().expect("account id");

    let viewer_token = login(&app, "viewer").await;
    let (status, _) = common::get_auth(
        &app,
        &format!("/api/v1/accounts/{account_id}"),
        &viewer_token,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let editor_token = login(&app, "editor").await;
    let (status, _) = common::get_auth(
        &app,
        &format!("/api/v1/accounts/{account_id}"),
        &editor_token,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let (_, list) = common::get_auth(&app, "/api/v1/accounts", &editor_token).await;
    assert!(list.as_array().expect("accounts").is_empty());

    // The household owner can see every account.
    let (status, _) = common::get_auth(
        &app,
        &format!("/api/v1/accounts/{account_id}"),
        &owner_token,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn non_owners_cannot_manage_users() {
    let app = common::router_with_auth();
    let (owner_token, _) = setup_owner(&app).await;

    common::post_json_auth(
        &app,
        "/api/v1/users",
        json!({ "username": "editor", "password": TEST_PASSWORD, "role": "editor" }),
        &owner_token,
    )
    .await;
    let editor_token = login(&app, "editor").await;

    let (status, _) = common::get_auth(&app, "/api/v1/users", &editor_token).await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let (status, _) = common::post_json_auth(
        &app,
        "/api/v1/users",
        json!({ "username": "sneaky", "password": TEST_PASSWORD, "role": "owner" }),
        &editor_token,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn logout_invalidates_the_token() {
    let app = common::router_with_auth();
    let (token, _) = setup_owner(&app).await;

    let (status, _) = common::post_json_auth(&app, "/api/v1/auth/logout", json!({}), &token).await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (status, _) = common::get_auth(&app, "/api/v1/auth/me", &token).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}
