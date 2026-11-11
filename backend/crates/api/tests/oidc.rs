//! End-to-end OIDC login tests against a local mock provider.

mod common;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use axum::body::Body;
use axum::extract::Form;
use axum::http::{Request, StatusCode};
use axum::routing::{get, post};
use axum::{Json, Router};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde_json::json;
use time::OffsetDateTime;
use tower::ServiceExt;

const PASSWORD: &str = "test-only-not-a-secret";
const PROVIDER_SECRET: &str = "test-only-oidc-secret";

#[tokio::test]
async fn oidc_login_provisions_owner_maps_roles_and_keeps_local_login() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("mock listener");
    let port = listener.local_addr().expect("addr").port();
    let issuer = format!("http://127.0.0.1:{port}");
    let nonce = Arc::new(Mutex::new(String::new()));

    let provider = Router::new()
        .route(
            "/.well-known/openid-configuration",
            get({
                let issuer = issuer.clone();
                move || async move {
                    Json(json!({
                        "issuer": issuer,
                        "authorization_endpoint": format!("{issuer}/authorize"),
                        "token_endpoint": format!("{issuer}/token"),
                        "jwks_uri": format!("{issuer}/jwks")
                    }))
                }
            }),
        )
        .route(
            "/token",
            post({
                let issuer = issuer.clone();
                let nonce = nonce.clone();
                move |Form(form): Form<HashMap<String, String>>| async move {
                    let now = OffsetDateTime::now_utc().unix_timestamp();
                    let code = form.get("code").map(String::as_str).unwrap_or("");
                    let (subject, email, username, name, groups) = match code {
                        "denied" => (
                            "subject-denied",
                            "stranger@example.com",
                            "sso-stranger",
                            "SSO Stranger",
                            vec![],
                        ),
                        "link" => (
                            "subject-link",
                            "owner@example.com",
                            "linked-owner",
                            "Linked Owner",
                            vec![],
                        ),
                        _ => (
                            "subject-owner",
                            "owner@example.com",
                            "sso-owner",
                            "SSO Owner",
                            vec!["networth-owners"],
                        ),
                    };
                    let nonce_value = if code == "badnonce" {
                        "wrong-nonce".to_string()
                    } else {
                        nonce.lock().expect("nonce").clone()
                    };
                    let claim_issuer = if code == "badissuer" {
                        "https://wrong.example"
                    } else {
                        issuer.as_str()
                    };
                    let audience = if code == "badaud" {
                        "other-client"
                    } else {
                        "networth-tracker"
                    };
                    let expiry = if code == "expired" {
                        now - 10
                    } else {
                        now + 3600
                    };
                    let claims = json!({
                        "iss": claim_issuer,
                        "aud": audience,
                        "sub": subject,
                        "exp": expiry,
                        "iat": now,
                        "nonce": nonce_value,
                        "email": email,
                        "email_verified": true,
                        "preferred_username": username,
                        "name": name,
                        "groups": groups
                    });
                    let signing_secret = if code == "badsig" {
                        "wrong-provider-secret"
                    } else {
                        PROVIDER_SECRET
                    };
                    let token = encode(
                        &Header::new(Algorithm::HS256),
                        &claims,
                        &EncodingKey::from_secret(signing_secret.as_bytes()),
                    )
                    .expect("encode token");
                    Json(json!({ "id_token": token }))
                }
            }),
        )
        .route("/jwks", get(|| async { Json(json!({ "keys": [] })) }));

    tokio::spawn(async move {
        axum::serve(listener, provider).await.expect("mock server");
    });

    std::env::set_var("OIDC_ENABLED", "true");
    std::env::set_var("OIDC_ISSUER", &issuer);
    std::env::set_var("OIDC_CLIENT_ID", "networth-tracker");
    std::env::set_var("OIDC_CLIENT_SECRET", PROVIDER_SECRET);
    std::env::set_var(
        "OIDC_REDIRECT_URI",
        "http://localhost:8080/api/v1/auth/oidc/callback",
    );
    std::env::set_var(
        "OIDC_FRONTEND_CALLBACK",
        "http://localhost:3000/auth/oidc/callback",
    );
    std::env::set_var("OIDC_SCOPES", "openid profile email groups");
    std::env::set_var("OIDC_AUTO_PROVISION", "true");
    std::env::set_var("OIDC_DEFAULT_ROLE", "viewer");
    std::env::set_var("OIDC_ALLOWED_EMAILS", "owner@example.com");
    std::env::set_var("OIDC_ROLE_CLAIM", "groups");
    std::env::set_var("OIDC_OWNER_GROUPS", "networth-owners");

    let app = common::router_with_auth();
    let (status, setup) = common::post_json(
        &app,
        "/api/v1/auth/setup",
        json!({ "username": "owner", "password": PASSWORD }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "setup: {setup}");
    let owner_token = setup["token"].as_str().expect("owner token").to_string();
    let owner_id = setup["user"]["id"].as_str().expect("owner id").to_string();

    let (status, auth_status) = common::get(&app, "/api/v1/auth/status").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(auth_status["oidc_enabled"], true);
    assert_eq!(auth_status["oidc_button_label"], "Sign in with SSO");

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/auth/oidc/start")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("start response");
    assert_eq!(response.status(), StatusCode::TEMPORARY_REDIRECT);
    let location = response
        .headers()
        .get("location")
        .and_then(|value| value.to_str().ok())
        .expect("authorization location")
        .to_string();
    let state = query_value(&location, "state");
    let provider_nonce = query_value(&location, "nonce");
    *nonce.lock().expect("nonce") = provider_nonce;

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/v1/auth/oidc/callback?code=test-code&state={state}"
                ))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("callback response");
    assert_eq!(response.status(), StatusCode::TEMPORARY_REDIRECT);
    let callback_location = response
        .headers()
        .get("location")
        .and_then(|value| value.to_str().ok())
        .expect("frontend location")
        .to_string();
    let exchange_code = query_value(&callback_location, "code");

    let (status, session) = common::post_json(
        &app,
        "/api/v1/auth/oidc/exchange",
        json!({ "code": exchange_code }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "exchange: {session}");
    let token = session["token"].as_str().expect("token");
    let user_id = session["user"]["id"].as_str().expect("user id");
    assert_eq!(session["user"]["username"], "sso-owner");
    assert_eq!(session["user"]["role"], "owner");

    let (status, me) = common::get_auth(&app, "/api/v1/auth/me", token).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(me["id"], user_id);

    // Local break-glass login still works for the original owner.
    let (status, local) = common::post_json(
        &app,
        "/api/v1/auth/login",
        json!({ "username": "owner", "password": PASSWORD }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "local login: {local}");

    // A non-allowlisted identity is denied even if the provider authenticates it.
    let denied_location = begin_login(&app).await;
    *nonce.lock().expect("nonce") = query_value(&denied_location, "nonce");
    let denied_state = query_value(&denied_location, "state");
    let (status, _) = callback(&app, "denied", &denied_state).await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    // A nonce mismatch is rejected before a user is created.
    let bad_nonce_location = begin_login(&app).await;
    *nonce.lock().expect("nonce") = query_value(&bad_nonce_location, "nonce");
    let bad_nonce_state = query_value(&bad_nonce_location, "state");
    let (status, _) = callback(&app, "badnonce", &bad_nonce_state).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    for code in ["badissuer", "badaud", "badsig", "expired"] {
        let location = begin_login(&app).await;
        *nonce.lock().expect("nonce") = query_value(&location, "nonce");
        let state = query_value(&location, "state");
        let (status, _) = callback(&app, code, &state).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "invalid token case {code}");
    }

    // Linking the local owner to a provider identity makes later SSO logins
    // resolve to the same local user.
    let (status, link) =
        common::post_json_auth(&app, "/api/v1/auth/oidc/link", json!({}), &owner_token).await;
    assert_eq!(status, StatusCode::OK, "link: {link}");
    let link_url = link["authorization_url"].as_str().expect("link url");
    *nonce.lock().expect("nonce") = query_value(link_url, "nonce");
    let link_state = query_value(link_url, "state");
    let (status, _) = callback(&app, "link", &link_state).await;
    assert_eq!(status, StatusCode::TEMPORARY_REDIRECT);

    let linked_login = begin_login(&app).await;
    *nonce.lock().expect("nonce") = query_value(&linked_login, "nonce");
    let linked_state = query_value(&linked_login, "state");
    let (status, linked_location) = callback(&app, "link", &linked_state).await;
    assert_eq!(status, StatusCode::TEMPORARY_REDIRECT);
    let linked_code = query_value(linked_location.as_deref().expect("linked location"), "code");
    let (status, linked_session) = common::post_json(
        &app,
        "/api/v1/auth/oidc/exchange",
        json!({ "code": linked_code }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "linked exchange: {linked_session}");
    assert_eq!(linked_session["user"]["id"], owner_id);

    // Replaying the callback state is rejected.
    let (status, _) = common::get(
        &app,
        &format!("/api/v1/auth/oidc/callback?code=test-code&state={state}"),
    )
    .await;
    assert_ne!(status, StatusCode::OK);
}

async fn begin_login(app: &axum::Router) -> String {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/auth/oidc/start")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("start response");
    assert_eq!(response.status(), StatusCode::TEMPORARY_REDIRECT);
    response
        .headers()
        .get("location")
        .and_then(|value| value.to_str().ok())
        .expect("authorization location")
        .to_string()
}

async fn callback(app: &axum::Router, code: &str, state: &str) -> (StatusCode, Option<String>) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/v1/auth/oidc/callback?code={code}&state={state}"
                ))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("callback response");
    let location = response
        .headers()
        .get("location")
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);
    (response.status(), location)
}

fn query_value(url: &str, key: &str) -> String {
    let query = url.split_once('?').map(|(_, query)| query).unwrap_or("");
    query
        .split('&')
        .find_map(|pair| {
            let (name, value) = pair.split_once('=')?;
            (name == key).then(|| value.to_string())
        })
        .unwrap_or_default()
}
