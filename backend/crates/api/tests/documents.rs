//! API-level tests for the encrypted document store and document import.

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::json;

const DOCUMENT_KEY: &str = "CQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQk=";

fn csv() -> &'static str {
    "record_type,account_name,account_type,as_of,amount,source\n\
account,Fidelity Checking,checking,,,\n\
balance,Fidelity Checking,,2026-01-01,100.00,import\n"
}

fn configure_document_env() -> String {
    let dir = std::env::temp_dir().join(format!("networth-documents-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("document test dir");
    std::env::set_var("DOCUMENT_ENCRYPTION_KEY", DOCUMENT_KEY);
    std::env::set_var("DOCUMENT_DIR", &dir);
    dir.to_string_lossy().to_string()
}

#[tokio::test]
async fn uploads_deduplicates_downloads_previews_and_imports() {
    let dir = configure_document_env();
    let app = common::router();

    let (status, document) = common::post_bytes(
        &app,
        "/api/v1/documents?filename=fidelity-holdings.csv&tags=retirement,2026",
        "text/csv",
        csv().as_bytes().to_vec(),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "upload: {document}");
    let id = document["id"].as_str().expect("id").to_string();
    assert_eq!(document["institution"], "Fidelity");
    assert_eq!(document["tags"][0], "retirement");
    assert_eq!(document["status"], "parsed");

    let (status, duplicate) = common::post_bytes(
        &app,
        "/api/v1/documents?filename=fidelity-copy.csv",
        "text/csv",
        csv().as_bytes().to_vec(),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "dedupe: {duplicate}");
    assert_eq!(duplicate["id"], id);

    let (status, documents) = common::get(&app, "/api/v1/documents").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(documents.as_array().expect("documents").len(), 1);

    let (status, bytes, content_type) =
        common::get_raw(&app, &format!("/api/v1/documents/{id}/download")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(content_type.as_deref(), Some("text/csv"));
    assert_eq!(bytes, csv().as_bytes());

    let (status, preview) =
        common::post_json(&app, &format!("/api/v1/documents/{id}/preview"), json!({})).await;
    assert_eq!(status, StatusCode::OK, "preview: {preview}");
    assert_eq!(preview["format"], "csv");
    assert_eq!(preview["table"]["headers"][0], "record_type");

    let request = Request::builder()
        .method("PATCH")
        .uri(format!("/api/v1/documents/{id}"))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "tags": ["confirmed"],
                "institution": "Fidelity",
                "document_type": "holdings",
                "confidence": 0.99,
                "period": "2026"
            })
            .to_string(),
        ))
        .expect("patch request");
    let (status, updated) = common::send(&app, request).await;
    assert_eq!(status, StatusCode::OK, "patch: {updated}");
    assert_eq!(updated["tags"][0], "confirmed");
    assert_eq!(updated["status"], "confirmed");

    let (status, report) = common::post_json(
        &app,
        &format!("/api/v1/documents/{id}/import"),
        json!({
            "mapping": {},
            "mode": "atomic",
            "backup_confirmed": true
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "import: {report}");
    assert_eq!(report["accounts_created"], 1);
    assert_eq!(report["balances_saved"], 1);

    let (status, audit) = common::get(&app, &format!("/api/v1/documents/{id}/audit")).await;
    assert_eq!(status, StatusCode::OK);
    let actions = audit
        .as_array()
        .expect("audit")
        .iter()
        .filter_map(|event| event["action"].as_str())
        .collect::<Vec<_>>();
    assert!(actions.contains(&"upload"));
    assert!(actions.contains(&"download"));
    assert!(actions.contains(&"classify"));
    assert!(actions.contains(&"import"));

    let (status, _) = common::delete(&app, &format!("/api/v1/documents/{id}")).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (_, documents) = common::get(&app, "/api/v1/documents").await;
    assert_eq!(documents.as_array().expect("documents").len(), 0);
    let (_, documents) = common::get(&app, "/api/v1/documents?include_deleted=true").await;
    assert_eq!(documents.as_array().expect("documents").len(), 1);

    let _ = std::fs::remove_dir_all(dir);
}
