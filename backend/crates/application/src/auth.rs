//! Local authentication: first-run setup, login, sessions, and roles.
//!
//! Passwords are hashed with Argon2id. Session tokens are returned to the
//! client and only their SHA-256 hash is stored. When authentication is
//! disabled (single-user offline mode) every request acts as an implicit owner.

use std::sync::Arc;

use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use networth_domain::{NewUser, User, UserRole};
use sha2::{Digest, Sha256};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use crate::ports::{IdentityRepository, RepositoryError};

/// How long a session lasts.
const SESSION_DAYS: i64 = 30;
/// Minimum password length.
const MIN_PASSWORD_LEN: usize = 8;

/// Errors raised by authentication and authorization.
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    /// Authentication is turned off in this installation.
    #[error("authentication is disabled")]
    Disabled,
    /// Missing or invalid credentials.
    #[error("invalid username or password")]
    Unauthorized,
    /// Authenticated but not allowed.
    #[error("you do not have permission to do that")]
    Forbidden,
    /// Input failed validation.
    #[error("{0}")]
    Validation(String),
    /// The request conflicts with existing state.
    #[error("{0}")]
    Conflict(String),
    /// An unexpected failure.
    #[error("internal error: {0}")]
    Internal(String),
    /// Persistence failed.
    #[error(transparent)]
    Repository(#[from] RepositoryError),
}

/// Whether auth is on and whether first-run setup is needed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthStatus {
    /// Whether authentication is enabled.
    pub auth_enabled: bool,
    /// Whether the household has no users yet.
    pub needs_setup: bool,
}

/// A successful login: the bearer token and the user.
#[derive(Debug, Clone)]
pub struct Session {
    /// Bearer token to send as `Authorization: Bearer <token>`.
    pub token: String,
    /// The authenticated user.
    pub user: User,
}

/// The user used when authentication is disabled.
pub fn implicit_user() -> User {
    User {
        id: Uuid::from_u128(1),
        username: "local".to_string(),
        display_name: "Local User".to_string(),
        role: UserRole::Owner,
    }
}

/// Authentication and user-management use cases.
pub struct AuthService {
    repo: Arc<dyn IdentityRepository>,
    enabled: bool,
}

impl AuthService {
    /// Build the service.
    pub fn new(repo: Arc<dyn IdentityRepository>, enabled: bool) -> Self {
        Self { repo, enabled }
    }

    /// Whether authentication is enabled.
    pub fn enabled(&self) -> bool {
        self.enabled
    }

    /// Report whether auth is on and whether setup is required.
    pub async fn status(&self) -> Result<AuthStatus, AuthError> {
        if !self.enabled {
            return Ok(AuthStatus {
                auth_enabled: false,
                needs_setup: false,
            });
        }
        let count = self.repo.count_users().await?;
        Ok(AuthStatus {
            auth_enabled: true,
            needs_setup: count == 0,
        })
    }

    /// Create the first owner. Only allowed while no users exist.
    pub async fn setup(
        &self,
        username: &str,
        password: &str,
        display_name: &str,
    ) -> Result<Session, AuthError> {
        if !self.enabled {
            return Err(AuthError::Disabled);
        }
        if self.repo.count_users().await? > 0 {
            return Err(AuthError::Conflict(
                "the household is already set up".to_string(),
            ));
        }

        let user = self
            .create_user_record(username, password, display_name, UserRole::Owner)
            .await?;

        self.issue_session(user).await
    }

    /// Authenticate with a username and password.
    pub async fn login(&self, username: &str, password: &str) -> Result<Session, AuthError> {
        if !self.enabled {
            return Err(AuthError::Disabled);
        }

        let found = self
            .repo
            .find_user_by_username(username)
            .await?
            .ok_or(AuthError::Unauthorized)?;

        if !verify_password(password, &found.password_hash) {
            return Err(AuthError::Unauthorized);
        }

        self.issue_session(found.user).await
    }

    /// Invalidate a session token.
    pub async fn logout(&self, token: &str) -> Result<(), AuthError> {
        self.repo.delete_session(&hash_token(token)).await?;
        Ok(())
    }

    /// Resolve the current user from an optional bearer token.
    pub async fn authenticate(&self, token: Option<&str>) -> Result<User, AuthError> {
        if !self.enabled {
            return Ok(implicit_user());
        }

        let token = token.ok_or(AuthError::Unauthorized)?;
        let (user, expires_at) = self
            .repo
            .find_session(&hash_token(token))
            .await?
            .ok_or(AuthError::Unauthorized)?;

        if expires_at < OffsetDateTime::now_utc() {
            return Err(AuthError::Unauthorized);
        }

        Ok(user)
    }

    /// Create a user. Only an owner may do this.
    pub async fn create_user(
        &self,
        actor: &User,
        username: &str,
        password: &str,
        display_name: &str,
        role: UserRole,
    ) -> Result<User, AuthError> {
        if !actor.role.is_owner() {
            return Err(AuthError::Forbidden);
        }
        if !self.enabled {
            return Err(AuthError::Disabled);
        }

        self.create_user_record(username, password, display_name, role)
            .await
    }

    /// List users. Only an owner may do this.
    pub async fn list_users(&self, actor: &User) -> Result<Vec<User>, AuthError> {
        if !actor.role.is_owner() {
            return Err(AuthError::Forbidden);
        }
        Ok(self.repo.list_users().await?)
    }

    /// Change a user's role. Only an owner may do this.
    pub async fn update_role(
        &self,
        actor: &User,
        target: Uuid,
        role: UserRole,
    ) -> Result<User, AuthError> {
        if !actor.role.is_owner() {
            return Err(AuthError::Forbidden);
        }
        Ok(self.repo.update_user_role(target, role).await?)
    }

    async fn create_user_record(
        &self,
        username: &str,
        password: &str,
        display_name: &str,
        role: UserRole,
    ) -> Result<User, AuthError> {
        let username = username.trim().to_ascii_lowercase();
        if username.len() < 3 {
            return Err(AuthError::Validation(
                "username must be at least 3 characters".to_string(),
            ));
        }

        let display_name = display_name.trim();
        let display_name = if display_name.is_empty() {
            username.clone()
        } else {
            display_name.to_string()
        };

        let password_hash = hash_password(password)?;

        self.repo
            .create_user(&NewUser {
                username,
                display_name,
                password_hash,
                role,
            })
            .await
            .map_err(|error| match error {
                RepositoryError::Conflict(message) => AuthError::Conflict(message),
                other => AuthError::Repository(other),
            })
    }

    async fn issue_session(&self, user: User) -> Result<Session, AuthError> {
        let token = generate_token();
        let expires_at = OffsetDateTime::now_utc() + Duration::days(SESSION_DAYS);

        self.repo
            .create_session(user.id, &hash_token(&token), expires_at)
            .await?;

        Ok(Session { token, user })
    }
}

fn hash_password(password: &str) -> Result<String, AuthError> {
    if password.len() < MIN_PASSWORD_LEN {
        return Err(AuthError::Validation(format!(
            "password must be at least {MIN_PASSWORD_LEN} characters"
        )));
    }

    let salt = SaltString::encode_b64(&Uuid::new_v4().into_bytes())
        .map_err(|error| AuthError::Internal(error.to_string()))?;
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|error| AuthError::Internal(error.to_string()))
}

fn verify_password(password: &str, hash: &str) -> bool {
    let Ok(parsed) = PasswordHash::new(hash) else {
        return false;
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok()
}

fn generate_token() -> String {
    format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple())
}

fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    format!("{:x}", hasher.finalize())
}
