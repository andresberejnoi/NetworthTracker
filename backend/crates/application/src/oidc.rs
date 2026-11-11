//! Optional OpenID Connect client for homelab identity providers.
//!
//! The client implements the authorization-code flow with PKCE and validates
//! the provider's ID token. Session issuance and user/role mapping stay in the
//! existing auth service.

use std::collections::HashSet;
use std::str::FromStr;
use std::sync::{Arc, Mutex};

use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use base64::Engine;
use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use networth_domain::UserRole;
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use time::OffsetDateTime;

/// Configuration for an OIDC provider.
#[derive(Debug, Clone)]
pub struct OidcConfig {
    /// Provider issuer, without a trailing slash.
    pub issuer: String,
    /// OIDC client id.
    pub client_id: String,
    /// Optional client secret for a confidential client.
    pub client_secret: Option<String>,
    /// Backend callback URL registered with the provider.
    pub redirect_uri: String,
    /// Frontend route that receives the one-time exchange code.
    pub frontend_callback: String,
    /// Requested scopes.
    pub scopes: Vec<String>,
    /// Login button label.
    pub button_label: String,
    /// Whether an allowed unknown user may be provisioned automatically.
    pub auto_provision: bool,
    /// Default role for provisioned/mapped users.
    pub default_role: UserRole,
    /// Optional email allowlist.
    pub allowed_emails: HashSet<String>,
    /// Optional subject allowlist.
    pub allowed_subjects: HashSet<String>,
    /// Claim name containing groups/roles.
    pub role_claim: String,
    /// Groups that map to owner.
    pub owner_groups: HashSet<String>,
    /// Groups that map to editor.
    pub editor_groups: HashSet<String>,
}

/// Errors raised by the OIDC client.
#[derive(Debug, thiserror::Error)]
pub enum OidcError {
    /// Input/response failed validation.
    #[error("{0}")]
    Validation(String),
    /// The provider or discovery endpoint failed.
    #[error("OIDC provider error: {0}")]
    Provider(String),
    /// Persistence failed.
    #[error(transparent)]
    Repository(#[from] crate::ports::RepositoryError),
}

/// Discovery document fields used by the client.
#[derive(Debug, Clone, Deserialize)]
struct Discovery {
    issuer: String,
    authorization_endpoint: String,
    token_endpoint: String,
    jwks_uri: String,
}

/// A token endpoint response.
#[derive(Debug, Deserialize)]
struct TokenResponse {
    id_token: String,
}

#[derive(Debug, Clone, Deserialize)]
struct Jwks {
    keys: Vec<Jwk>,
}

#[derive(Debug, Clone, Deserialize)]
struct Jwk {
    kid: Option<String>,
    kty: String,
    n: Option<String>,
    e: Option<String>,
}

struct CachedDiscovery {
    discovery: Discovery,
    expires_at: OffsetDateTime,
}

struct CachedJwks {
    jwks: Jwks,
    expires_at: OffsetDateTime,
}

/// ID-token claims used for identity and role mapping.
#[derive(Debug, Clone, Deserialize)]
pub struct IdTokenClaims {
    /// Issuer.
    pub iss: String,
    /// Audience (string or array).
    pub aud: Value,
    /// Subject (stable identity key).
    pub sub: String,
    /// Expiry.
    pub exp: i64,
    /// Issued-at.
    pub iat: Option<i64>,
    /// Nonce.
    pub nonce: Option<String>,
    /// Email.
    pub email: Option<String>,
    /// Verified email flag.
    pub email_verified: Option<bool>,
    /// Preferred username.
    pub preferred_username: Option<String>,
    /// Display name.
    pub name: Option<String>,
    /// Group/role claim.
    pub groups: Option<Value>,
    /// Optional custom role claim.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, Value>,
}

impl IdTokenClaims {
    /// All group/role values in the configured claim.
    pub fn role_values(&self, claim: &str) -> Vec<String> {
        let value = if claim == "groups" {
            self.groups.as_ref()
        } else {
            self.extra.get(claim)
        };
        match value {
            Some(Value::String(value)) => vec![value.clone()],
            Some(Value::Array(values)) => values
                .iter()
                .filter_map(|value| value.as_str().map(str::to_string))
                .collect(),
            _ => Vec::new(),
        }
    }

    /// Best display name from the claims.
    pub fn display_name(&self) -> Option<String> {
        self.name
            .clone()
            .or_else(|| self.preferred_username.clone())
            .or_else(|| self.email.clone())
    }

    /// Suggested local username.
    pub fn username(&self) -> String {
        self.preferred_username
            .clone()
            .or_else(|| {
                self.email
                    .as_deref()
                    .and_then(|email| email.split('@').next())
                    .map(str::to_string)
            })
            .unwrap_or_else(|| format!("sso-{}", &self.sub[..self.sub.len().min(8)]))
    }
}

/// The optional OIDC client.
pub struct OidcClient {
    config: OidcConfig,
    http: reqwest::Client,
    discovery_cache: Mutex<Option<CachedDiscovery>>,
    jwks_cache: Mutex<Option<CachedJwks>>,
}

impl OidcClient {
    /// Build from environment variables. Returns `Ok(None)` when disabled.
    pub fn from_env() -> Result<Option<Arc<Self>>, String> {
        if !parse_bool(
            std::env::var("OIDC_ENABLED")
                .ok()
                .as_deref()
                .unwrap_or("false"),
        ) {
            return Ok(None);
        }
        let issuer = required_env("OIDC_ISSUER")?
            .trim_end_matches('/')
            .to_string();
        let client_id = required_env("OIDC_CLIENT_ID")?;
        let redirect_uri = required_env("OIDC_REDIRECT_URI")?;
        let frontend_callback = required_env("OIDC_FRONTEND_CALLBACK")?;
        let client_secret = std::env::var("OIDC_CLIENT_SECRET")
            .ok()
            .filter(|value| !value.is_empty());
        let scopes = std::env::var("OIDC_SCOPES")
            .unwrap_or_else(|_| "openid profile email".to_string())
            .split_whitespace()
            .map(str::to_string)
            .collect::<Vec<_>>();
        if !scopes.iter().any(|scope| scope == "openid") {
            return Err("OIDC_SCOPES must include openid".to_string());
        }
        let default_role = std::env::var("OIDC_DEFAULT_ROLE")
            .unwrap_or_else(|_| "viewer".to_string())
            .parse::<UserRole>()
            .map_err(|error| format!("invalid OIDC_DEFAULT_ROLE: {error}"))?;
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|error| format!("could not build OIDC HTTP client: {error}"))?;
        Ok(Some(Arc::new(Self {
            config: OidcConfig {
                issuer,
                client_id,
                client_secret,
                redirect_uri,
                frontend_callback,
                scopes,
                button_label: std::env::var("OIDC_BUTTON_LABEL")
                    .unwrap_or_else(|_| "Sign in with SSO".to_string()),
                auto_provision: parse_bool(
                    std::env::var("OIDC_AUTO_PROVISION")
                        .ok()
                        .as_deref()
                        .unwrap_or("false"),
                ),
                default_role,
                allowed_emails: csv_set("OIDC_ALLOWED_EMAILS", true),
                allowed_subjects: csv_set("OIDC_ALLOWED_SUBJECTS", false),
                role_claim: std::env::var("OIDC_ROLE_CLAIM")
                    .unwrap_or_else(|_| "groups".to_string()),
                owner_groups: csv_set("OIDC_OWNER_GROUPS", false),
                editor_groups: csv_set("OIDC_EDITOR_GROUPS", false),
            },
            http,
            discovery_cache: Mutex::new(None),
            jwks_cache: Mutex::new(None),
        })))
    }

    /// Login button label.
    pub fn button_label(&self) -> &str {
        &self.config.button_label
    }

    /// Provider issuer.
    pub fn issuer(&self) -> &str {
        &self.config.issuer
    }

    /// Whether auto-provisioning is enabled.
    pub fn auto_provision(&self) -> bool {
        self.config.auto_provision
    }

    /// Frontend callback route.
    pub fn frontend_callback(&self) -> &str {
        &self.config.frontend_callback
    }

    /// Fetch and validate discovery metadata.
    async fn discover(&self) -> Result<Discovery, OidcError> {
        if let Some(cached) = self.discovery_cache.lock().expect("cache").as_ref() {
            if cached.expires_at > OffsetDateTime::now_utc() {
                return Ok(cached.discovery.clone());
            }
        }
        let url = format!("{}/.well-known/openid-configuration", self.config.issuer);
        let response = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(|error| OidcError::Provider(error.to_string()))?;
        if !response.status().is_success() {
            return Err(OidcError::Provider(format!(
                "discovery returned HTTP {}",
                response.status()
            )));
        }
        let discovery: Discovery = response
            .json()
            .await
            .map_err(|error| OidcError::Provider(format!("invalid discovery document: {error}")))?;
        if discovery.issuer.trim_end_matches('/') != self.config.issuer {
            return Err(OidcError::Validation(format!(
                "discovery issuer '{}' does not match configured issuer '{}'",
                discovery.issuer, self.config.issuer
            )));
        }
        *self.discovery_cache.lock().expect("cache") = Some(CachedDiscovery {
            discovery: discovery.clone(),
            expires_at: OffsetDateTime::now_utc() + time::Duration::hours(1),
        });
        Ok(discovery)
    }

    /// Build the provider authorization URL.
    pub async fn authorization_url(
        &self,
        state: &str,
        nonce: &str,
        code_challenge: &str,
    ) -> Result<String, OidcError> {
        let discovery = self.discover().await?;
        let mut url = reqwest::Url::parse(&discovery.authorization_endpoint)
            .map_err(|error| OidcError::Provider(error.to_string()))?;
        url.query_pairs_mut()
            .append_pair("response_type", "code")
            .append_pair("client_id", &self.config.client_id)
            .append_pair("redirect_uri", &self.config.redirect_uri)
            .append_pair("scope", &self.config.scopes.join(" "))
            .append_pair("state", state)
            .append_pair("nonce", nonce)
            .append_pair("code_challenge", code_challenge)
            .append_pair("code_challenge_method", "S256");
        Ok(url.into())
    }

    /// Exchange an authorization code and validate the ID token.
    pub async fn exchange(
        &self,
        code: &str,
        nonce: &str,
        code_verifier: &str,
    ) -> Result<IdTokenClaims, OidcError> {
        let discovery = self.discover().await?;
        let mut form = vec![
            ("grant_type", "authorization_code"),
            ("code", code),
            ("client_id", &self.config.client_id),
            ("redirect_uri", &self.config.redirect_uri),
            ("code_verifier", code_verifier),
        ];
        if let Some(secret) = self.config.client_secret.as_deref() {
            form.push(("client_secret", secret));
        }
        let response = self
            .http
            .post(&discovery.token_endpoint)
            .form(&form)
            .send()
            .await
            .map_err(|error| OidcError::Provider(error.to_string()))?;
        if !response.status().is_success() {
            return Err(OidcError::Provider(format!(
                "token endpoint returned HTTP {}",
                response.status()
            )));
        }
        let token: TokenResponse = response
            .json()
            .await
            .map_err(|error| OidcError::Provider(format!("invalid token response: {error}")))?;
        self.validate_id_token(&discovery, &token.id_token, nonce)
            .await
    }

    async fn validate_id_token(
        &self,
        discovery: &Discovery,
        token: &str,
        expected_nonce: &str,
    ) -> Result<IdTokenClaims, OidcError> {
        let header =
            decode_header(token).map_err(|error| OidcError::Validation(error.to_string()))?;
        let key = match header.alg {
            Algorithm::HS256 | Algorithm::HS384 | Algorithm::HS512 => {
                let secret = self.config.client_secret.as_deref().ok_or_else(|| {
                    OidcError::Validation(
                        "HS-signed ID token requires OIDC_CLIENT_SECRET".to_string(),
                    )
                })?;
                DecodingKey::from_secret(secret.as_bytes())
            }
            Algorithm::RS256 | Algorithm::RS384 | Algorithm::RS512 => {
                self.rsa_key(discovery, header.kid.as_deref()).await?
            }
            _ => {
                return Err(OidcError::Validation(format!(
                    "unsupported ID token algorithm {:?}",
                    header.alg
                )))
            }
        };
        let mut validation = Validation::new(header.alg);
        validation.validate_exp = false;
        validation.validate_aud = false;
        let data = decode::<IdTokenClaims>(token, &key, &validation)
            .map_err(|error| OidcError::Validation(format!("invalid ID token: {error}")))?;
        let claims = data.claims;
        if claims.iss.trim_end_matches('/') != self.config.issuer {
            return Err(OidcError::Validation(
                "ID token issuer mismatch".to_string(),
            ));
        }
        if !audience_matches(&claims.aud, &self.config.client_id) {
            return Err(OidcError::Validation(
                "ID token audience mismatch".to_string(),
            ));
        }
        if claims.exp <= OffsetDateTime::now_utc().unix_timestamp() {
            return Err(OidcError::Validation("ID token is expired".to_string()));
        }
        if let Some(iat) = claims.iat {
            if iat > OffsetDateTime::now_utc().unix_timestamp() + 300 {
                return Err(OidcError::Validation(
                    "ID token issued in the future".to_string(),
                ));
            }
        }
        if claims.nonce.as_deref() != Some(expected_nonce) {
            return Err(OidcError::Validation("ID token nonce mismatch".to_string()));
        }
        if claims.sub.trim().is_empty() {
            return Err(OidcError::Validation(
                "ID token subject is empty".to_string(),
            ));
        }
        Ok(claims)
    }

    async fn rsa_key(
        &self,
        discovery: &Discovery,
        kid: Option<&str>,
    ) -> Result<DecodingKey, OidcError> {
        let cached = self
            .jwks_cache
            .lock()
            .expect("cache")
            .as_ref()
            .filter(|cached| cached.expires_at > OffsetDateTime::now_utc())
            .map(|cached| cached.jwks.clone());
        let jwks = if let Some(jwks) = cached {
            jwks
        } else {
            let response = self
                .http
                .get(&discovery.jwks_uri)
                .send()
                .await
                .map_err(|error| OidcError::Provider(error.to_string()))?;
            if !response.status().is_success() {
                return Err(OidcError::Provider(format!(
                    "JWKS returned HTTP {}",
                    response.status()
                )));
            }
            let jwks: Jwks = response
                .json()
                .await
                .map_err(|error| OidcError::Provider(format!("invalid JWKS: {error}")))?;
            *self.jwks_cache.lock().expect("cache") = Some(CachedJwks {
                jwks: jwks.clone(),
                expires_at: OffsetDateTime::now_utc() + time::Duration::hours(1),
            });
            jwks
        };
        let key = jwks
            .keys
            .into_iter()
            .find(|key| {
                key.kty == "RSA"
                    && kid
                        .map(|kid| key.kid.as_deref() == Some(kid))
                        .unwrap_or(true)
                    && key.n.is_some()
                    && key.e.is_some()
            })
            .ok_or_else(|| OidcError::Validation("no matching RSA signing key".to_string()))?;
        DecodingKey::from_rsa_components(
            key.n.as_deref().unwrap_or_default(),
            key.e.as_deref().unwrap_or_default(),
        )
        .map_err(|error| OidcError::Validation(error.to_string()))
    }

    /// Whether an unknown identity is allowed to provision.
    pub fn allows(&self, claims: &IdTokenClaims) -> bool {
        if self.config.allowed_emails.is_empty() && self.config.allowed_subjects.is_empty() {
            return false;
        }
        let email = claims.email.as_deref().unwrap_or("").to_ascii_lowercase();
        let subject_allowed = self.config.allowed_subjects.contains(&claims.sub)
            || self
                .config
                .allowed_subjects
                .contains(&format!("{}:{}", self.config.issuer, claims.sub));
        let email_allowed =
            claims.email_verified == Some(true) && self.config.allowed_emails.contains(&email);
        subject_allowed || email_allowed
    }

    /// The explicitly mapped role, if a group mapping matches.
    pub fn mapped_role(&self, claims: &IdTokenClaims) -> Option<UserRole> {
        let groups = claims.role_values(&self.config.role_claim);
        if groups
            .iter()
            .any(|group| self.config.owner_groups.contains(group))
        {
            Some(UserRole::Owner)
        } else if groups
            .iter()
            .any(|group| self.config.editor_groups.contains(group))
        {
            Some(UserRole::Editor)
        } else {
            None
        }
    }

    /// Map claims to a role. Owner wins over editor, then the default role.
    pub fn role_for(&self, claims: &IdTokenClaims) -> UserRole {
        self.mapped_role(claims).unwrap_or(self.config.default_role)
    }
}

fn required_env(name: &str) -> Result<String, String> {
    std::env::var(name)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("{name} is required when OIDC_ENABLED=true"))
}

fn parse_bool(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

fn csv_set(name: &str, lower: bool) -> HashSet<String> {
    std::env::var(name)
        .ok()
        .into_iter()
        .flat_map(|value| {
            value
                .split(',')
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .map(|item| {
                    if lower {
                        item.to_ascii_lowercase()
                    } else {
                        item.to_string()
                    }
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

fn audience_matches(value: &Value, client_id: &str) -> bool {
    match value {
        Value::String(value) => value == client_id,
        Value::Array(values) => values.iter().any(|value| value.as_str() == Some(client_id)),
        _ => false,
    }
}

/// Generate a PKCE verifier and challenge.
pub fn pkce_pair() -> (String, String) {
    let verifier = random_urlsafe(32);
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    (verifier, challenge)
}

/// Generate a random URL-safe secret.
pub fn random_urlsafe(len: usize) -> String {
    let mut bytes = vec![0u8; len];
    use rand::RngCore;
    rand::thread_rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

/// Hash an exchange code for storage.
pub fn hash_exchange_code(code: &str) -> String {
    let digest = Sha256::digest(code.as_bytes());
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Decode a base64 value without exposing it.
pub fn decode_base64(value: &str) -> Result<Vec<u8>, String> {
    STANDARD.decode(value).map_err(|error| error.to_string())
}

#[allow(dead_code)]
fn role_from_str(value: &str) -> Option<UserRole> {
    UserRole::from_str(value).ok()
}
