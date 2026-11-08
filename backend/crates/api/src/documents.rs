//! Encrypted document upload, listing, download, preview, and import endpoints.

use std::collections::HashMap;

use axum::body::{Body, Bytes};
use axum::extract::{Path, Query, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use networth_application::{
    DocumentAuditEvent, DocumentPreview, DocumentService, MigrationMode, MigrationReport,
    StoredDocument,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::error::ApiError;
use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct UploadQuery {
    pub filename: Option<String>,
    pub tags: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateDocumentRequest {
    pub tags: Option<Vec<String>>,
    pub institution: Option<String>,
    pub document_type: Option<String>,
    pub confidence: Option<f64>,
    pub period: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct DocumentImportRequest {
    #[serde(default)]
    pub mapping: HashMap<String, String>,
    pub mode: MigrationMode,
    pub backup_confirmed: bool,
}

fn require_owner(current: &CurrentUser) -> Result<(), ApiError> {
    if !current.user.role.is_owner() {
        return Err(ApiError::Forbidden(
            "only the household owner can access uploaded documents".to_string(),
        ));
    }
    Ok(())
}

fn store(state: &AppState) -> Result<&DocumentService, ApiError> {
    state.documents.as_deref().ok_or_else(|| {
        ApiError::BadRequest(
            "the document store is not configured; set DOCUMENT_ENCRYPTION_KEY".to_string(),
        )
    })
}

/// `POST /api/v1/documents?filename=...&tags=...`
pub async fn upload(
    State(state): State<AppState>,
    current: CurrentUser,
    Query(query): Query<UploadQuery>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<(StatusCode, Json<StoredDocument>), ApiError> {
    require_owner(&current)?;
    let filename = query
        .filename
        .or_else(|| {
            headers
                .get("x-filename")
                .and_then(|value| value.to_str().ok())
                .map(str::to_string)
        })
        .ok_or_else(|| ApiError::BadRequest("filename is required".to_string()))?;
    let filename = std::path::Path::new(&filename)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("document")
        .to_string();
    let mime = headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("application/octet-stream")
        .to_string();
    let tags = query
        .tags
        .map(|tags| {
            tags.split(',')
                .map(str::trim)
                .filter(|tag| !tag.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    let document = store(&state)?
        .upload(Some(current.user.id), &filename, &mime, &body, tags)
        .await?;
    Ok((StatusCode::CREATED, Json(document)))
}

/// `GET /api/v1/documents`
pub async fn list(
    State(state): State<AppState>,
    current: CurrentUser,
    Query(query): Query<HashMap<String, String>>,
) -> Result<Json<Vec<StoredDocument>>, ApiError> {
    require_owner(&current)?;
    let include_deleted = query
        .get("include_deleted")
        .map(|value| value == "true")
        .unwrap_or(false);
    Ok(Json(store(&state)?.list(include_deleted).await?))
}

/// `GET /api/v1/documents/{id}`
pub async fn get(
    State(state): State<AppState>,
    current: CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<Json<StoredDocument>, ApiError> {
    require_owner(&current)?;
    Ok(Json(store(&state)?.get(id).await?))
}

/// `GET /api/v1/documents/{id}/download`
pub async fn download(
    State(state): State<AppState>,
    current: CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<Response, ApiError> {
    require_owner(&current)?;
    let (document, bytes) = store(&state)?.download(id, Some(current.user.id)).await?;
    let filename = document.original_name.replace('"', "_");
    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, document.mime_type)
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{filename}\""),
        )
        .body(Body::from(bytes))
        .map_err(|error| ApiError::Internal(error.to_string()))?;
    Ok(response.into_response())
}

/// `PATCH /api/v1/documents/{id}`
pub async fn update(
    State(state): State<AppState>,
    current: CurrentUser,
    Path(id): Path<Uuid>,
    Json(request): Json<UpdateDocumentRequest>,
) -> Result<Json<StoredDocument>, ApiError> {
    require_owner(&current)?;
    let service = store(&state)?;
    let mut document = service.get(id).await?;
    if let Some(tags) = request.tags {
        document = service.set_tags(id, Some(current.user.id), tags).await?;
    }
    if request.institution.is_some()
        || request.document_type.is_some()
        || request.confidence.is_some()
        || request.period.is_some()
    {
        document = service
            .confirm_classification(
                id,
                Some(current.user.id),
                request.institution,
                request.document_type,
                request.confidence,
                request.period,
            )
            .await?;
    }
    Ok(Json(document))
}

/// `POST /api/v1/documents/{id}/preview`
pub async fn preview(
    State(state): State<AppState>,
    current: CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<Json<DocumentPreview>, ApiError> {
    require_owner(&current)?;
    Ok(Json(store(&state)?.preview(id).await?))
}

/// `GET /api/v1/documents/{id}/audit`
pub async fn audit(
    State(state): State<AppState>,
    current: CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<DocumentAuditEvent>>, ApiError> {
    require_owner(&current)?;
    let document = store(&state)?.get(id).await?;
    Ok(Json(store(&state)?.audit_events(document.id).await?))
}

/// `POST /api/v1/documents/{id}/import`
pub async fn import(
    State(state): State<AppState>,
    current: CurrentUser,
    Path(id): Path<Uuid>,
    Json(request): Json<DocumentImportRequest>,
) -> Result<Json<MigrationReport>, ApiError> {
    require_owner(&current)?;
    Ok(Json(
        store(&state)?
            .import(
                id,
                Some(current.user.id),
                request.mapping,
                request.mode,
                request.backup_confirmed,
            )
            .await?,
    ))
}

/// `DELETE /api/v1/documents/{id}`
pub async fn delete(
    State(state): State<AppState>,
    current: CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    require_owner(&current)?;
    store(&state)?
        .soft_delete(id, Some(current.user.id))
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
