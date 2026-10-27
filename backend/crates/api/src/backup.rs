//! Full backup and restore endpoints (owner only).

use axum::{extract::State, Json};
use networth_application::{Backup, RestoreReport};

use crate::auth::CurrentUser;
use crate::error::ApiError;
use crate::AppState;

/// `GET /api/v1/backup`
pub async fn export(
    State(state): State<AppState>,
    current: CurrentUser,
) -> Result<Json<Backup>, ApiError> {
    require_owner(&current)?;
    Ok(Json(state.backup.export().await?))
}

/// `POST /api/v1/backup/restore`
pub async fn restore(
    State(state): State<AppState>,
    current: CurrentUser,
    Json(backup): Json<Backup>,
) -> Result<Json<RestoreReport>, ApiError> {
    require_owner(&current)?;
    Ok(Json(state.backup.restore(&backup).await?))
}

fn require_owner(current: &CurrentUser) -> Result<(), ApiError> {
    if !current.user.role.is_owner() {
        return Err(ApiError::Forbidden(
            "only the household owner can back up or restore".to_string(),
        ));
    }
    Ok(())
}
