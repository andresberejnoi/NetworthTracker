//! Owner-only full backups created by the backend container.
//!
//! The backend already has the PostgreSQL client, gpg, the database connection,
//! and the `/backups` and `/documents` mounts. A bundled script creates and
//! verifies the encrypted archives; this module orchestrates it, serializes
//! runs, lists files, and validates download names.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::body::Body;
use axum::extract::{Path as AxumPath, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;
use time::format_description::FormatItem;
use time::macros::format_description;
use time::OffsetDateTime;
use tokio::process::Command;
use tokio::sync::Mutex;
use tokio_util::io::ReaderStream;

use crate::auth::CurrentUser;
use crate::error::ApiError;
use crate::AppState;

const TIMESTAMP_FORMAT: &[FormatItem<'static>] =
    format_description!("[year][month][day]T[hour][minute][second]Z");

/// One encrypted backup file.
#[derive(Debug, Clone, Serialize)]
pub struct BackupFile {
    /// Basename within the backup directory.
    pub name: String,
    /// File size in bytes.
    pub size_bytes: u64,
    /// Last modification time (RFC 3339).
    pub modified_at: String,
}

/// A database backup and its optional document archive.
#[derive(Debug, Clone, Serialize)]
pub struct BackupSummary {
    /// Timestamp shared by the database and document archive.
    pub timestamp: String,
    /// Encrypted PostgreSQL dump.
    pub database: BackupFile,
    /// Encrypted document archive, if documents existed.
    pub documents: Option<BackupFile>,
}

/// Full-backup orchestration.
pub struct FullBackupService {
    script: PathBuf,
    backup_dir: PathBuf,
    document_dir: PathBuf,
    keep: u32,
    lock: Mutex<()>,
}

impl FullBackupService {
    /// Build from the container environment.
    pub fn from_env() -> Arc<Self> {
        let script = std::env::var("FULL_BACKUP_SCRIPT")
            .unwrap_or_else(|_| "/usr/local/bin/networth-backup-container".to_string());
        let backup_dir = std::env::var("BACKUP_DIR").unwrap_or_else(|_| "/backups".to_string());
        let document_dir =
            std::env::var("DOCUMENT_DIR").unwrap_or_else(|_| "/documents".to_string());
        let keep = std::env::var("BACKUP_KEEP")
            .ok()
            .and_then(|value| value.parse().ok())
            .filter(|value| *value > 0)
            .unwrap_or(14);
        Arc::new(Self {
            script: PathBuf::from(script),
            backup_dir: PathBuf::from(backup_dir),
            document_dir: PathBuf::from(document_dir),
            keep,
            lock: Mutex::new(()),
        })
    }

    /// Create a full backup. At most one run is active at a time.
    pub async fn create(&self) -> Result<BackupSummary, ApiError> {
        let passphrase = std::env::var("BACKUP_PASSPHRASE").unwrap_or_default();
        if passphrase.trim().is_empty() || passphrase.trim() == "change-me" {
            return Err(ApiError::BadRequest(
                "BACKUP_PASSPHRASE is required to create an encrypted backup".to_string(),
            ));
        }
        let _guard = self
            .lock
            .try_lock()
            .map_err(|_| ApiError::Conflict("a backup is already running".to_string()))?;

        let timestamp = OffsetDateTime::now_utc()
            .format(TIMESTAMP_FORMAT)
            .map_err(|error| ApiError::Internal(error.to_string()))?;
        let output = Command::new(&self.script)
            .env("BACKUP_TIMESTAMP", &timestamp)
            .env("BACKUP_DIR", &self.backup_dir)
            .env("DOCUMENT_DIR", &self.document_dir)
            .env("BACKUP_KEEP", self.keep.to_string())
            .output()
            .await
            .map_err(|error| ApiError::Internal(format!("could not run backup: {error}")))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(ApiError::Internal(format!(
                "backup failed: {}",
                stderr.trim()
            )));
        }

        let database_name = format!("networth-{timestamp}.sql.gz.gpg");
        let documents_name = format!("networth-documents-{timestamp}.tar.gz.gpg");
        let database = self.file_entry(&database_name).await?.ok_or_else(|| {
            ApiError::Internal("backup did not create a database archive".to_string())
        })?;
        if database.size_bytes == 0 {
            return Err(ApiError::Internal(
                "backup database archive is empty".to_string(),
            ));
        }
        let documents = self.file_entry(&documents_name).await?;
        Ok(BackupSummary {
            timestamp,
            database,
            documents,
        })
    }

    /// List recognized backup files, newest first.
    pub async fn list(&self) -> Result<Vec<BackupSummary>, ApiError> {
        let mut entries = match tokio::fs::read_dir(&self.backup_dir).await {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => {
                return Err(ApiError::Internal(format!(
                    "could not list backups: {error}"
                )))
            }
        };
        let mut by_timestamp: std::collections::BTreeMap<String, BackupSummary> =
            std::collections::BTreeMap::new();
        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|error| ApiError::Internal(error.to_string()))?
        {
            let name = entry.file_name().to_string_lossy().to_string();
            if let Some(timestamp) = database_timestamp(&name) {
                let file = file_from_entry(entry).await?;
                by_timestamp.insert(
                    timestamp.clone(),
                    BackupSummary {
                        timestamp,
                        database: file,
                        documents: None,
                    },
                );
            }
        }
        let mut entries = match tokio::fs::read_dir(&self.backup_dir).await {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(by_timestamp.into_values().rev().collect())
            }
            Err(error) => return Err(ApiError::Internal(error.to_string())),
        };
        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|error| ApiError::Internal(error.to_string()))?
        {
            let name = entry.file_name().to_string_lossy().to_string();
            if let Some(timestamp) = documents_timestamp(&name) {
                if let Some(summary) = by_timestamp.get_mut(&timestamp) {
                    summary.documents = Some(file_from_entry(entry).await?);
                }
            }
        }
        Ok(by_timestamp.into_values().rev().collect())
    }

    /// Open one backup file after validating its basename.
    pub async fn download(&self, name: &str) -> Result<(String, u64, tokio::fs::File), ApiError> {
        if !valid_backup_name(name) {
            return Err(ApiError::BadRequest("invalid backup filename".to_string()));
        }
        let path = self.backup_dir.join(name);
        let metadata = tokio::fs::symlink_metadata(&path)
            .await
            .map_err(|_| ApiError::NotFound(format!("backup not found: {name}")))?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(ApiError::NotFound(format!("backup not found: {name}")));
        }
        let file = tokio::fs::File::open(&path)
            .await
            .map_err(|_| ApiError::NotFound(format!("backup not found: {name}")))?;
        Ok((name.to_string(), metadata.len(), file))
    }

    async fn file_entry(&self, name: &str) -> Result<Option<BackupFile>, ApiError> {
        let path = self.backup_dir.join(name);
        let metadata = match tokio::fs::metadata(&path).await {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(ApiError::Internal(error.to_string())),
        };
        let modified = metadata
            .modified()
            .ok()
            .map(OffsetDateTime::from)
            .map(|value| value.to_string())
            .unwrap_or_default();
        Ok(Some(BackupFile {
            name: name.to_string(),
            size_bytes: metadata.len(),
            modified_at: modified,
        }))
    }
}

async fn file_from_entry(entry: tokio::fs::DirEntry) -> Result<BackupFile, ApiError> {
    let metadata = entry
        .metadata()
        .await
        .map_err(|error| ApiError::Internal(error.to_string()))?;
    let modified = metadata
        .modified()
        .ok()
        .map(OffsetDateTime::from)
        .map(|value| value.to_string())
        .unwrap_or_default();
    Ok(BackupFile {
        name: entry.file_name().to_string_lossy().to_string(),
        size_bytes: metadata.len(),
        modified_at: modified,
    })
}

fn database_timestamp(name: &str) -> Option<String> {
    name.strip_prefix("networth-")
        .and_then(|name| name.strip_suffix(".sql.gz.gpg"))
        .filter(|timestamp| valid_timestamp(timestamp))
        .map(str::to_string)
}

fn documents_timestamp(name: &str) -> Option<String> {
    name.strip_prefix("networth-documents-")
        .and_then(|name| name.strip_suffix(".tar.gz.gpg"))
        .filter(|timestamp| valid_timestamp(timestamp))
        .map(str::to_string)
}

fn valid_timestamp(value: &str) -> bool {
    value.len() == 16
        && value
            .chars()
            .all(|character| character.is_ascii_digit() || character == 'T' || character == 'Z')
}

fn valid_backup_name(name: &str) -> bool {
    !name.is_empty()
        && Path::new(name).file_name().and_then(|value| value.to_str()) == Some(name)
        && (database_timestamp(name).is_some() || documents_timestamp(name).is_some())
}

fn require_owner(current: &CurrentUser) -> Result<(), ApiError> {
    if !current.user.role.is_owner() {
        return Err(ApiError::Forbidden(
            "only the household owner can manage backups".to_string(),
        ));
    }
    Ok(())
}

/// `POST /api/v1/admin/backups`
pub async fn create(
    State(state): State<AppState>,
    current: CurrentUser,
) -> Result<(StatusCode, Json<BackupSummary>), ApiError> {
    require_owner(&current)?;
    let summary = state.full_backup.create().await?;
    Ok((StatusCode::CREATED, Json(summary)))
}

/// `GET /api/v1/admin/backups`
pub async fn list(
    State(state): State<AppState>,
    current: CurrentUser,
) -> Result<Json<Vec<BackupSummary>>, ApiError> {
    require_owner(&current)?;
    Ok(Json(state.full_backup.list().await?))
}

/// `GET /api/v1/admin/backups/{name}/download`
pub async fn download(
    State(state): State<AppState>,
    current: CurrentUser,
    AxumPath(name): AxumPath<String>,
) -> Result<Response, ApiError> {
    require_owner(&current)?;
    let (name, size, file) = state.full_backup.download(&name).await?;
    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/octet-stream")
        .header(header::CONTENT_LENGTH, size.to_string())
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{name}\""),
        )
        .body(Body::from_stream(ReaderStream::new(file)))
        .map_err(|error| ApiError::Internal(error.to_string()))?;
    Ok(response.into_response())
}

#[cfg(test)]
mod tests {
    use super::{database_timestamp, documents_timestamp, valid_backup_name};

    #[test]
    fn recognizes_expected_archive_names() {
        assert_eq!(
            database_timestamp("networth-20261114T120000Z.sql.gz.gpg").as_deref(),
            Some("20261114T120000Z")
        );
        assert_eq!(
            documents_timestamp("networth-documents-20261114T120000Z.tar.gz.gpg").as_deref(),
            Some("20261114T120000Z")
        );
        assert!(valid_backup_name("networth-20261114T120000Z.sql.gz.gpg"));
        assert!(valid_backup_name(
            "networth-documents-20261114T120000Z.tar.gz.gpg"
        ));
    }

    #[test]
    fn rejects_paths_and_unknown_files() {
        assert!(!valid_backup_name(
            "../networth-20261114T120000Z.sql.gz.gpg"
        ));
        assert!(!valid_backup_name(
            "nested/networth-20261114T120000Z.sql.gz.gpg"
        ));
        assert!(!valid_backup_name(
            "networth-20261114T120000Z.sql.gz.gpg/.."
        ));
        assert!(!valid_backup_name("networth-not-a-date.sql.gz.gpg"));
        assert!(!valid_backup_name("notes.txt"));
        assert!(!valid_backup_name(""));
    }
}
