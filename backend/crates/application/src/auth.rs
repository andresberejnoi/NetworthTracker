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

use crate::oidc::{
    hash_exchange_code, pkce_pair, random_urlsafe, IdTokenClaims, OidcClient, OidcError,
};
use crate::ports::{IdentityRepository, NewExternalIdentity, RepositoryError};

/// How long a session lasts.
const SESSION_DAYS: i64 = 30;
/// How long an OIDC authorization state is valid.
const OIDC_STATE_MINUTES: i64 = 10;
/// How long an SPA exchange code is valid.
const OIDC_EXCHANGE_SECONDS: i64 = 60;
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
    /// OIDC discovery, token exchange, or validation failed.
    #[error(transparent)]
    Oidc(#[from] OidcError),
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
    /// Whether SSO is configured and enabled.
    pub oidc_enabled: bool,
    /// Configured SSO button label.
    pub oidc_button_label: Option<String>,
}

/// The result of completing an OIDC callback.
#[derive(Debug, Clone)]
pub struct OidcCallback {
    /// The authenticated user.
    pub user: User,
    /// One-time SPA exchange code for a login (not used for a link).
    pub exchange_code: Option<String>,
    /// Whether this callback linked an identity to an existing user.
    pub linked: bool,
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
    oidc: Option<Arc<OidcClient>>,
}

impl AuthService {
    /// Build the service.
    pub fn new(
        repo: Arc<dyn IdentityRepository>,
        enabled: bool,
        oidc: Option<Arc<OidcClient>>,
    ) -> Self {
        Self {
            repo,
            enabled,
            oidc,
        }
    }

    /// Whether authentication is enabled.
    pub fn enabled(&self) -> bool {
        self.enabled
    }

    /// Whether OIDC SSO is configured.
    pub fn oidc_enabled(&self) -> bool {
        self.oidc.is_some()
    }

    /// Configured frontend callback route.
    pub fn oidc_frontend_callback(&self) -> Option<&str> {
        self.oidc.as_ref().map(|oidc| oidc.frontend_callback())
    }

    /// Report whether auth is on and whether setup is required.
    pub async fn status(&self) -> Result<AuthStatus, AuthError> {
        if !self.enabled {
            return Ok(AuthStatus {
                auth_enabled: false,
                needs_setup: false,
                oidc_enabled: false,
                oidc_button_label: None,
            });
        }
        let count = self.repo.count_users().await?;
        Ok(AuthStatus {
            auth_enabled: true,
            needs_setup: count == 0,
            oidc_enabled: self.oidc.is_some(),
            oidc_button_label: self
                .oidc
                .as_ref()
                .map(|oidc| oidc.button_label().to_string()),
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

        let Some(password_hash) = found.password_hash.as_deref() else {
            return Err(AuthError::Unauthorized);
        };
        if !verify_password(password, password_hash) {
            return Err(AuthError::Unauthorized);
        }

        self.issue_session(found.user).await
    }

    /// Begin an OIDC login, or link the current user when `link_user_id` is set.
    pub async fn oidc_start(&self, link_user_id: Option<Uuid>) -> Result<String, AuthError> {
        if !self.enabled {
            return Err(AuthError::Disabled);
        }
        let oidc = self
            .oidc
            .as_ref()
            .ok_or_else(|| AuthError::Validation("SSO is not configured".to_string()))?;
        if let Some(user_id) = link_user_id {
            if self.repo.find_user(user_id).await?.is_none() {
                return Err(AuthError::Unauthorized);
            }
        }
        let state = random_urlsafe(32);
        let nonce = random_urlsafe(32);
        let (verifier, challenge) = pkce_pair();
        let expires_at = OffsetDateTime::now_utc() + Duration::minutes(OIDC_STATE_MINUTES);
        self.repo
            .create_oidc_state(&state, &nonce, &verifier, link_user_id, expires_at)
            .await?;
        Ok(oidc.authorization_url(&state, &nonce, &challenge).await?)
    }

    /// Complete an OIDC callback. A login returns a one-time SPA exchange code;
    /// a link returns the linked user without issuing a new session.
    pub async fn oidc_callback(&self, code: &str, state: &str) -> Result<OidcCallback, AuthError> {
        if !self.enabled {
            return Err(AuthError::Disabled);
        }
        let oidc = self
            .oidc
            .as_ref()
            .ok_or_else(|| AuthError::Validation("SSO is not configured".to_string()))?;
        let Some(login_state) = self.repo.consume_oidc_state(state).await? else {
            self.repo
                .record_auth_audit(
                    None,
                    "oidc_denied",
                    oidc.issuer(),
                    "",
                    r#"{"reason":"invalid_or_replayed_state"}"#,
                )
                .await?;
            return Err(AuthError::Unauthorized);
        };
        if login_state.expires_at < OffsetDateTime::now_utc() {
            self.repo
                .record_auth_audit(
                    None,
                    "oidc_denied",
                    oidc.issuer(),
                    "",
                    r#"{"reason":"expired_state"}"#,
                )
                .await?;
            return Err(AuthError::Unauthorized);
        }
        let claims = match oidc
            .exchange(code, &login_state.nonce, &login_state.code_verifier)
            .await
        {
            Ok(claims) => claims,
            Err(error) => {
                self.repo
                    .record_auth_audit(
                        None,
                        "oidc_failed",
                        oidc.issuer(),
                        "",
                        r#"{"reason":"token_exchange_or_validation_failed"}"#,
                    )
                    .await?;
                return Err(error.into());
            }
        };

        if let Some(user_id) = login_state.link_user_id {
            let user = self
                .repo
                .find_user(user_id)
                .await?
                .ok_or(AuthError::Unauthorized)?;
            if let Some(existing) = self
                .repo
                .find_external_identity(oidc.issuer(), &claims.sub)
                .await?
            {
                if existing.user_id != user_id {
                    return Err(AuthError::Conflict(
                        "that SSO identity is already linked to another user".to_string(),
                    ));
                }
                self.repo
                    .touch_external_identity(oidc.issuer(), &claims.sub)
                    .await?;
            } else {
                self.repo
                    .link_external_identity(&NewExternalIdentity {
                        user_id,
                        issuer: oidc.issuer().to_string(),
                        subject: claims.sub.clone(),
                        email: claims.email.clone(),
                        email_verified: claims.email_verified == Some(true),
                        display_name: claims.display_name(),
                    })
                    .await?;
            }
            self.repo
                .record_auth_audit(
                    Some(user.id),
                    "oidc_link",
                    oidc.issuer(),
                    &claims.sub,
                    r#"{"linked":true}"#,
                )
                .await?;
            return Ok(OidcCallback {
                user,
                exchange_code: None,
                linked: true,
            });
        }

        let user = match self
            .repo
            .find_external_identity(oidc.issuer(), &claims.sub)
            .await?
        {
            Some(identity) => {
                let user = self
                    .repo
                    .find_user(identity.user_id)
                    .await?
                    .ok_or(AuthError::Unauthorized)?;
                if let Some(role) = oidc.mapped_role(&claims) {
                    if user.role != role {
                        let user = self.repo.update_user_role(user.id, role).await?;
                        self.repo
                            .touch_external_identity(oidc.issuer(), &claims.sub)
                            .await?;
                        user
                    } else {
                        self.repo
                            .touch_external_identity(oidc.issuer(), &claims.sub)
                            .await?;
                        user
                    }
                } else {
                    self.repo
                        .touch_external_identity(oidc.issuer(), &claims.sub)
                        .await?;
                    user
                }
            }
            None => self.provision_oidc(oidc, &claims).await?,
        };

        self.repo
            .record_auth_audit(
                Some(user.id),
                "oidc_login",
                oidc.issuer(),
                &claims.sub,
                r#"{"success":true}"#,
            )
            .await?;

        let exchange_code = random_urlsafe(32);
        self.repo
            .create_exchange_code(
                &hash_exchange_code(&exchange_code),
                user.id,
                OffsetDateTime::now_utc() + Duration::seconds(OIDC_EXCHANGE_SECONDS),
            )
            .await?;
        Ok(OidcCallback {
            user,
            exchange_code: Some(exchange_code),
            linked: false,
        })
    }

    /// Exchange a one-time SPA code for a normal app session.
    pub async fn oidc_exchange(&self, code: &str) -> Result<Session, AuthError> {
        if !self.enabled {
            return Err(AuthError::Disabled);
        }
        let user_id = self
            .repo
            .consume_exchange_code(&hash_exchange_code(code))
            .await?
            .ok_or(AuthError::Unauthorized)?;
        let user = self
            .repo
            .find_user(user_id)
            .await?
            .ok_or(AuthError::Unauthorized)?;
        self.issue_session(user).await
    }

    async fn provision_oidc(
        &self,
        oidc: &OidcClient,
        claims: &IdTokenClaims,
    ) -> Result<User, AuthError> {
        if self.repo.count_users().await? == 0 {
            self.repo
                .record_auth_audit(
                    None,
                    "oidc_denied",
                    oidc.issuer(),
                    &claims.sub,
                    r#"{"reason":"local_owner_setup_required"}"#,
                )
                .await?;
            return Err(AuthError::Forbidden);
        }
        if !oidc.auto_provision() {
            self.repo
                .record_auth_audit(
                    None,
                    "oidc_denied",
                    oidc.issuer(),
                    &claims.sub,
                    r#"{"reason":"auto_provision_disabled"}"#,
                )
                .await?;
            return Err(AuthError::Forbidden);
        }
        if !oidc.allows(claims) {
            self.repo
                .record_auth_audit(
                    None,
                    "oidc_denied",
                    oidc.issuer(),
                    &claims.sub,
                    r#"{"reason":"not_allowlisted"}"#,
                )
                .await?;
            return Err(AuthError::Forbidden);
        }
        let base = sanitize_username(&claims.username());
        let mut username = base.clone();
        let mut suffix = 1usize;
        while self.repo.find_user_by_username(&username).await?.is_some() {
            username = format!("{base}{suffix}");
            suffix += 1;
        }
        let display_name = claims.display_name().unwrap_or_else(|| username.clone());
        let role = oidc.role_for(claims);
        let user = self
            .repo
            .create_sso_user(&username, &display_name, role)
            .await?;
        self.repo
            .link_external_identity(&NewExternalIdentity {
                user_id: user.id,
                issuer: oidc.issuer().to_string(),
                subject: claims.sub.clone(),
                email: claims.email.clone(),
                email_verified: claims.email_verified == Some(true),
                display_name: claims.display_name(),
            })
            .await?;
        Ok(user)
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

fn sanitize_username(value: &str) -> String {
    let mut username: String = value
        .trim()
        .to_ascii_lowercase()
        .chars()
        .filter(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-')
        })
        .take(32)
        .collect();
    if username.len() < 3 {
        username = "sso-user".to_string();
    }
    username
}

fn generate_token() -> String {
    format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple())
}

fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    format!("{:x}", hasher.finalize())
}
