//! Guided migration preview and apply endpoints (owner only).

use axum::{extract::State, Json};
use networth_application::{
    MigrationApplyRequest, MigrationPreview, MigrationReport, MigrationRequest,
};

use crate::auth::CurrentUser;
use crate::error::ApiError;
use crate::AppState;

fn require_owner(current: &CurrentUser) -> Result<(), ApiError> {
    if !current.user.role.is_owner() {
        return Err(ApiError::Forbidden(
            "only the household owner can run a data migration".to_string(),
        ));
    }
    Ok(())
}

/// `POST /api/v1/migrations/preview`
pub async fn preview(
    State(state): State<AppState>,
    current: CurrentUser,
    Json(request): Json<MigrationRequest>,
) -> Result<Json<MigrationPreview>, ApiError> {
    require_owner(&current)?;
    Ok(Json(state.migration.preview(&request).await?))
}

/// `POST /api/v1/migrations/apply`
pub async fn apply(
    State(state): State<AppState>,
    current: CurrentUser,
    Json(request): Json<MigrationApplyRequest>,
) -> Result<Json<MigrationReport>, ApiError> {
    require_owner(&current)?;
    Ok(Json(state.migration.apply(&request).await?))
}
