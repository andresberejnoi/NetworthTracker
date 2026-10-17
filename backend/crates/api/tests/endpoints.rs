//! API-level tests for the walking-skeleton endpoints.

mod common;

use axum::http::StatusCode;

#[tokio::test]
async fn health_reports_ok() {
    let (status, body) = common::get(&common::router(), "/health").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ok");
}

#[tokio::test]
async fn ping_reports_service_metadata() {
    let (status, body) = common::get(&common::router(), "/api/v1/ping").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["message"], "pong");
    assert_eq!(body["service"], "NetworthTracker");
    assert!(body["version"].is_string());
}

#[tokio::test]
async fn unknown_route_is_not_found() {
    let (status, _) = common::get(&common::router(), "/api/v1/does-not-exist").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

/// The product version must be date-based: MAJOR.MINOR.yyyymmdd (see AGENTS.md).
#[tokio::test]
async fn version_is_date_based() {
    let (_, body) = common::get(&common::router(), "/api/v1/ping").await;
    let version = body["version"]
        .as_str()
        .expect("version should be a string");

    let parts: Vec<&str> = version.split('.').collect();
    assert_eq!(
        parts.len(),
        3,
        "expected MAJOR.MINOR.yyyymmdd, got {version}"
    );

    assert!(
        parts[0].parse::<u64>().is_ok(),
        "MAJOR must be numeric in {version}"
    );
    assert!(
        parts[1].parse::<u64>().is_ok(),
        "MINOR must be numeric in {version}"
    );

    let date = parts[2];
    assert_eq!(date.len(), 8, "date segment must be yyyymmdd in {version}");
    assert!(
        date.chars().all(|c| c.is_ascii_digit()),
        "date segment must be digits in {version}"
    );

    let year: i32 = date[0..4].parse().expect("year");
    let month: u32 = date[4..6].parse().expect("month");
    let day: u32 = date[6..8].parse().expect("day");
    assert!(
        (2000..=2100).contains(&year),
        "implausible year in {version}"
    );
    assert!((1..=12).contains(&month), "invalid month in {version}");
    assert!((1..=31).contains(&day), "invalid day in {version}");
}
