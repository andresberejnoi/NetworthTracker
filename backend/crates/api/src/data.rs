//! Import/export and template endpoints.

use axum::{
    body::Bytes,
    extract::{Query, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use networth_adapters::{read_table, write_table, SpreadsheetFormat};
use networth_application::ImportReport;
use serde::{Deserialize, Serialize};

use crate::auth::CurrentUser;
use crate::error::ApiError;
use crate::AppState;

/// Optional `?format=csv|xlsx`.
#[derive(Debug, Deserialize)]
pub struct FormatQuery {
    /// The file format.
    #[serde(default)]
    pub format: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ImportRowErrorDto {
    pub row: usize,
    pub error: String,
}

#[derive(Debug, Serialize)]
pub struct ImportReportDto {
    pub accounts_created: usize,
    pub balances_saved: usize,
    pub errors: Vec<ImportRowErrorDto>,
}

impl From<ImportReport> for ImportReportDto {
    fn from(report: ImportReport) -> Self {
        Self {
            accounts_created: report.accounts_created,
            balances_saved: report.balances_saved,
            errors: report
                .errors
                .into_iter()
                .map(|error| ImportRowErrorDto {
                    row: error.row,
                    error: error.message,
                })
                .collect(),
        }
    }
}

/// `POST /api/v1/import?format=csv|xlsx`
pub async fn import(
    State(state): State<AppState>,
    Query(query): Query<FormatQuery>,
    current: CurrentUser,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<ImportReportDto>, ApiError> {
    if !current.user.role.can_write() {
        return Err(ApiError::Forbidden(
            "you do not have permission to import".to_string(),
        ));
    }

    let format = resolve_format(query.format.as_deref(), &headers)?;
    let table =
        read_table(&body, format).map_err(|error| ApiError::BadRequest(error.to_string()))?;
    let report = state.data.import(&table).await?;
    Ok(Json(ImportReportDto::from(report)))
}

/// `GET /api/v1/export?format=csv|xlsx`
pub async fn export(
    State(state): State<AppState>,
    Query(query): Query<FormatQuery>,
    _current: CurrentUser,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let format = resolve_format(query.format.as_deref(), &headers)?;
    let table = state.data.export().await?;
    let bytes =
        write_table(&table, format).map_err(|error| ApiError::Internal(error.to_string()))?;
    Ok(file_response(bytes, format, "networth-export"))
}

/// `GET /api/v1/template?format=csv|xlsx`
pub async fn template(
    Query(query): Query<FormatQuery>,
    _current: CurrentUser,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let format = resolve_format(query.format.as_deref(), &headers)?;
    let table = networth_application::data_exchange::template();
    let bytes =
        write_table(&table, format).map_err(|error| ApiError::Internal(error.to_string()))?;
    Ok(file_response(bytes, format, "networth-import-template"))
}

fn resolve_format(
    requested: Option<&str>,
    headers: &HeaderMap,
) -> Result<SpreadsheetFormat, ApiError> {
    if let Some(name) = requested {
        return SpreadsheetFormat::from_name(name).ok_or_else(|| {
            ApiError::BadRequest(format!("unsupported format '{name}', use csv or xlsx"))
        });
    }

    if let Some(content_type) = headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
    {
        if let Some(format) = SpreadsheetFormat::from_content_type(content_type) {
            return Ok(format);
        }
    }

    // Default to CSV for downloads and for text bodies.
    Ok(SpreadsheetFormat::Csv)
}

fn file_response(bytes: Vec<u8>, format: SpreadsheetFormat, stem: &str) -> Response {
    let filename = format!("{stem}.{}", format.extension());
    let disposition = format!("attachment; filename=\"{filename}\"");

    let mut response = (StatusCode::OK, bytes).into_response();
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static(format.content_type()),
    );
    if let Ok(value) = HeaderValue::from_str(&disposition) {
        headers.insert(header::CONTENT_DISPOSITION, value);
    }
    response
}
