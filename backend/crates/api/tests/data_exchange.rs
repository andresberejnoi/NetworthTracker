//! API-level tests for spreadsheet import/export.

mod common;

use axum::http::StatusCode;

const HEADER: &str = "schema_version,record_type,account_name,account_type,institution,currency,ownership_scope,archived,as_of,amount,source";
const XLSX: &str = "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet";

fn sample() -> String {
    format!(
        "{HEADER}\n\
         1,account,Checking,checking,Local Bank,USD,household,false,,,\n\
         1,balance,Checking,,,,,,2026-01-01,100.00,manual\n"
    )
}

#[tokio::test]
async fn imports_accounts_and_balances() {
    let app = common::router();
    let (status, report) = common::post_bytes(
        &app,
        "/api/v1/import?format=csv",
        "text/csv",
        sample().into_bytes(),
    )
    .await;

    assert_eq!(status, StatusCode::OK, "report: {report}");
    assert_eq!(report["accounts_created"].as_u64(), Some(1));
    assert_eq!(report["balances_saved"].as_u64(), Some(1));
    assert!(report["errors"].as_array().expect("errors").is_empty());

    let (_, accounts) = common::get(&app, "/api/v1/accounts").await;
    let accounts = accounts.as_array().expect("accounts");
    assert_eq!(accounts.len(), 1);

    let id = accounts[0]["id"].as_str().expect("id");
    let (_, balances) = common::get(&app, &format!("/api/v1/accounts/{id}/balances")).await;
    let balances = balances.as_array().expect("balances");
    assert_eq!(balances.len(), 1);
    assert_eq!(balances[0]["amount"], "100.00");
}

#[tokio::test]
async fn reimporting_the_same_file_is_idempotent() {
    let app = common::router();
    let body = sample().into_bytes();

    let (_, first) =
        common::post_bytes(&app, "/api/v1/import?format=csv", "text/csv", body.clone()).await;
    assert_eq!(first["accounts_created"].as_u64(), Some(1));

    let (status, second) =
        common::post_bytes(&app, "/api/v1/import?format=csv", "text/csv", body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(second["accounts_created"].as_u64(), Some(0));
    assert_eq!(second["balances_saved"].as_u64(), Some(1));

    let (_, accounts) = common::get(&app, "/api/v1/accounts").await;
    assert_eq!(accounts.as_array().expect("accounts").len(), 1);
}

#[tokio::test]
async fn invalid_rows_write_nothing() {
    let app = common::router();
    let csv = format!(
        "{HEADER}\n\
         1,account,Checking,checking,,USD,household,false,,,\n\
         1,balance,Checking,,,,,,2026-01-01,oops,manual\n"
    );

    let (status, report) = common::post_bytes(
        &app,
        "/api/v1/import?format=csv",
        "text/csv",
        csv.into_bytes(),
    )
    .await;

    assert_eq!(status, StatusCode::OK, "report: {report}");
    assert_eq!(report["accounts_created"].as_u64(), Some(0));
    assert_eq!(report["balances_saved"].as_u64(), Some(0));
    let errors = report["errors"].as_array().expect("errors");
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0]["row"].as_u64(), Some(3));

    let (_, accounts) = common::get(&app, "/api/v1/accounts").await;
    assert!(accounts.as_array().expect("accounts").is_empty());
}

#[tokio::test]
async fn unknown_account_in_a_balance_is_reported() {
    let app = common::router();
    let csv = format!(
        "{HEADER}\n\
         1,balance,Ghost Account,,,,,,2026-01-01,10.00,manual\n"
    );

    let (status, report) = common::post_bytes(
        &app,
        "/api/v1/import?format=csv",
        "text/csv",
        csv.into_bytes(),
    )
    .await;

    assert_eq!(status, StatusCode::OK, "report: {report}");
    assert_eq!(report["balances_saved"].as_u64(), Some(0));
    let errors = report["errors"].as_array().expect("errors");
    assert!(errors[0]["error"]
        .as_str()
        .expect("error")
        .contains("unknown account"));
}

#[tokio::test]
async fn exports_csv_and_reimports_it() {
    let app = common::router();
    common::post_bytes(
        &app,
        "/api/v1/import?format=csv",
        "text/csv",
        sample().into_bytes(),
    )
    .await;

    let (status, bytes, content_type) = common::get_raw(&app, "/api/v1/export?format=csv").await;
    assert_eq!(status, StatusCode::OK);
    assert!(content_type.expect("content type").starts_with("text/csv"));

    let text = String::from_utf8(bytes.clone()).expect("utf8");
    assert!(text.starts_with("schema_version,record_type"));
    assert!(text.contains("Checking"));

    // A fresh household can import the export.
    let fresh = common::router();
    let (status, report) =
        common::post_bytes(&fresh, "/api/v1/import?format=csv", "text/csv", bytes).await;
    assert_eq!(status, StatusCode::OK, "report: {report}");
    assert_eq!(report["accounts_created"].as_u64(), Some(1));
    assert_eq!(report["balances_saved"].as_u64(), Some(1));
}

#[tokio::test]
async fn xlsx_round_trips() {
    let app = common::router();
    common::post_bytes(
        &app,
        "/api/v1/import?format=csv",
        "text/csv",
        sample().into_bytes(),
    )
    .await;

    let (status, bytes, content_type) = common::get_raw(&app, "/api/v1/export?format=xlsx").await;
    assert_eq!(status, StatusCode::OK);
    assert!(content_type
        .expect("content type")
        .contains("spreadsheetml"));
    assert_eq!(&bytes[..2], b"PK", "xlsx is a zip container");

    let fresh = common::router();
    let (status, report) =
        common::post_bytes(&fresh, "/api/v1/import?format=xlsx", XLSX, bytes).await;
    assert_eq!(status, StatusCode::OK, "report: {report}");
    assert_eq!(report["accounts_created"].as_u64(), Some(1));
    assert_eq!(report["balances_saved"].as_u64(), Some(1));
}

#[tokio::test]
async fn exact_decimal_round_trip() {
    let app = common::router();
    let csv = format!(
        "{HEADER}\n\
         1,account,Wallet,other_asset,,USD,household,false,,,\n\
         1,balance,Wallet,,,,,,2026-01-01,0.000000000000000001,manual\n"
    );
    common::post_bytes(
        &app,
        "/api/v1/import?format=csv",
        "text/csv",
        csv.into_bytes(),
    )
    .await;

    let (_, bytes, _) = common::get_raw(&app, "/api/v1/export?format=csv").await;
    let text = String::from_utf8(bytes).expect("utf8");
    assert!(text.contains("0.000000000000000001"));

    let fresh = common::router();
    let (_, report) = common::post_bytes(
        &fresh,
        "/api/v1/import?format=csv",
        "text/csv",
        text.into_bytes(),
    )
    .await;
    assert_eq!(report["balances_saved"].as_u64(), Some(1));

    let (_, accounts) = common::get(&fresh, "/api/v1/accounts").await;
    let id = accounts[0]["id"].as_str().expect("id");
    let (_, balances) = common::get(&fresh, &format!("/api/v1/accounts/{id}/balances")).await;
    assert_eq!(balances[0]["amount"], "0.000000000000000001");
}

#[tokio::test]
async fn template_has_the_expected_headers() {
    let (status, bytes, _) =
        common::get_raw(&common::router(), "/api/v1/template?format=csv").await;
    assert_eq!(status, StatusCode::OK);
    let text = String::from_utf8(bytes).expect("utf8");
    assert!(text.starts_with("schema_version,record_type"));
}
