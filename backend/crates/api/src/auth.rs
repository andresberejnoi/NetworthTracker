//! Authentication extractor, DTOs, and endpoints.

use axum::{
    extract::{FromRequestParts, Path, State},
    http::{header, request::Parts, StatusCode},
    Json,
};
use networth_application::{AuthStatus, Session};
use networth_domain::{User, UserRole};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::ApiError;
use crate::AppState;

/// The authenticated caller, resolved from `Authorization: Bearer <token>`.
///
/// When authentication is disabled this is an implicit local owner, so the
/// single-user offline mode needs no credentials.
pub struct CurrentUser {
    /// The resolved user.
    pub user: User,
    /// The bearer token, when one was supplied.
    pub token: Option<String>,
}

impl FromRequestParts<AppState> for CurrentUser {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let token = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.strip_prefix("Bearer "))
            .map(|value| value.trim().to_string());

        let user = state.auth.authenticate(token.as_deref()).await?;
        Ok(Self { user, token })
    }
}

/// A user as returned by the API.
#[derive(Debug, Serialize)]
pub struct UserDto {
    /// Identifier.
    pub id: Uuid,
    /// Login name.
    pub username: String,
    /// Display name.
    pub display_name: String,
    /// Role.
    pub role: String,
}

impl From<User> for UserDto {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            username: user.username,
            display_name: user.display_name,
            role: user.role.as_str().to_string(),
        }
    }
}

/// Authentication status for the frontend.
#[derive(Debug, Serialize)]
pub struct AuthStatusDto {
    /// Whether auth is enabled.
    pub auth_enabled: bool,
    /// Whether first-run setup is needed.
    pub needs_setup: bool,
}

impl From<AuthStatus> for AuthStatusDto {
    fn from(status: AuthStatus) -> Self {
        Self {
            auth_enabled: status.auth_enabled,
            needs_setup: status.needs_setup,
        }
    }
}

/// Request body for login.
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    /// Login name.
    pub username: String,
    /// Password.
    pub password: String,
}

/// Request body for first-run setup.
#[derive(Debug, Deserialize)]
pub struct SetupRequest {
    /// Login name.
    pub username: String,
    /// Password.
    pub password: String,
    /// Display name; defaults to the username.
    #[serde(default)]
    pub display_name: Option<String>,
}

/// A session returned by setup or login.
#[derive(Debug, Serialize)]
pub struct SessionDto {
    /// Bearer token.
    pub token: String,
    /// The authenticated user.
    pub user: UserDto,
}

impl From<Session> for SessionDto {
    fn from(session: Session) -> Self {
        Self {
            token: session.token,
            user: UserDto::from(session.user),
        }
    }
}

/// Request body for creating a user.
#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    /// Login name.
    pub username: String,
    /// Password.
    pub password: String,
    /// Display name; defaults to the username.
    #[serde(default)]
    pub display_name: Option<String>,
    /// `owner`, `editor`, or `viewer`.
    pub role: String,
}

/// Request body for changing a role.
#[derive(Debug, Deserialize)]
pub struct UpdateRoleRequest {
    /// `owner`, `editor`, or `viewer`.
    pub role: String,
}

/// `GET /api/v1/auth/status`
pub async fn status(State(state): State<AppState>) -> Result<Json<AuthStatusDto>, ApiError> {
    Ok(Json(state.auth.status().await?.into()))
}

/// `POST /api/v1/auth/setup`
pub async fn setup(
    State(state): State<AppState>,
    Json(request): Json<SetupRequest>,
) -> Result<Json<SessionDto>, ApiError> {
    let session = state
        .auth
        .setup(
            &request.username,
            &request.password,
            request.display_name.as_deref().unwrap_or(""),
        )
        .await?;
    Ok(Json(SessionDto::from(session)))
}

/// `POST /api/v1/auth/login`
pub async fn login(
    State(state): State<AppState>,
    Json(request): Json<LoginRequest>,
) -> Result<Json<SessionDto>, ApiError> {
    let session = state
        .auth
        .login(&request.username, &request.password)
        .await?;
    Ok(Json(SessionDto::from(session)))
}

/// `POST /api/v1/auth/logout`
pub async fn logout(
    State(state): State<AppState>,
    current: CurrentUser,
) -> Result<StatusCode, ApiError> {
    if let Some(token) = current.token {
        state.auth.logout(&token).await?;
    }
    Ok(StatusCode::NO_CONTENT)
}

/// `GET /api/v1/auth/me`
pub async fn me(current: CurrentUser) -> Json<UserDto> {
    Json(UserDto::from(current.user))
}

/// `GET /api/v1/users`
pub async fn list_users(
    State(state): State<AppState>,
    current: CurrentUser,
) -> Result<Json<Vec<UserDto>>, ApiError> {
    let users = state.auth.list_users(&current.user).await?;
    Ok(Json(users.into_iter().map(UserDto::from).collect()))
}

/// `POST /api/v1/users`
pub async fn create_user(
    State(state): State<AppState>,
    current: CurrentUser,
    Json(request): Json<CreateUserRequest>,
) -> Result<(StatusCode, Json<UserDto>), ApiError> {
    let role = request.role.parse::<UserRole>().map_err(
        |error: networth_domain::identity::UserRoleParseError| {
            ApiError::BadRequest(error.to_string())
        },
    )?;

    let user = state
        .auth
        .create_user(
            &current.user,
            &request.username,
            &request.password,
            request.display_name.as_deref().unwrap_or(""),
            role,
        )
        .await?;

    Ok((StatusCode::CREATED, Json(UserDto::from(user))))
}

/// `PATCH /api/v1/users/{id}`
pub async fn update_role(
    State(state): State<AppState>,
    current: CurrentUser,
    Path(id): Path<Uuid>,
    Json(request): Json<UpdateRoleRequest>,
) -> Result<Json<UserDto>, ApiError> {
    let role = request.role.parse::<UserRole>().map_err(
        |error: networth_domain::identity::UserRoleParseError| {
            ApiError::BadRequest(error.to_string())
        },
    )?;

    let user = state.auth.update_role(&current.user, id, role).await?;
    Ok(Json(UserDto::from(user)))
}
