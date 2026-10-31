//! Optional read-only aggregation, provenance, and encrypted local credentials.

use std::sync::Arc;

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use networth_domain::{BalanceSource, Money, NewAccount, NewBalance, OwnershipScope};
use rand::rngs::OsRng;
use rand::RngCore;
use time::OffsetDateTime;

use crate::ports::{
    AccountRepository, AggregationBalance, AggregationConfig, AggregationHolding, AggregationLink,
    AggregationProvider, AggregationRepository, ProviderError, RepositoryError,
};

/// Errors raised by aggregation.
#[derive(Debug, thiserror::Error)]
pub enum AggregationError {
    /// Input failed validation or the add-on is not configured.
    #[error("{0}")]
    Validation(String),
    /// The provider failed.
    #[error("aggregation provider error: {0}")]
    Provider(String),
    /// Credential encryption failed.
    #[error("aggregation credential error: {0}")]
    Crypto(String),
    /// Persistence failed.
    #[error(transparent)]
    Repository(#[from] RepositoryError),
}

/// A local, authenticated encryption helper for provider tokens.
pub struct TokenCipher {
    cipher: Aes256Gcm,
}

impl TokenCipher {
    /// Build from a 32-byte key.
    pub fn new(key: [u8; 32]) -> Self {
        Self {
            cipher: Aes256Gcm::new_from_slice(&key).expect("AES-256 key is valid"),
        }
    }

    /// Build from `AGGREGATION_ENCRYPTION_KEY` (base64, 32 bytes).
    pub fn from_env() -> Option<Self> {
        let value = std::env::var("AGGREGATION_ENCRYPTION_KEY").ok()?;
        let key = STANDARD.decode(value.trim()).ok()?;
        let key: [u8; 32] = key.try_into().ok()?;
        Some(Self::new(key))
    }

    /// Encrypt a token; returns base64 ciphertext and nonce.
    pub fn encrypt(&self, token: &str) -> Result<(String, String), AggregationError> {
        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        let ciphertext = self
            .cipher
            .encrypt(nonce, token.as_bytes())
            .map_err(|_| AggregationError::Crypto("failed to encrypt token".to_string()))?;
        Ok((STANDARD.encode(ciphertext), STANDARD.encode(nonce_bytes)))
    }

    /// Decrypt a token.
    pub fn decrypt(&self, ciphertext: &str, nonce: &str) -> Result<String, AggregationError> {
        let ciphertext = STANDARD
            .decode(ciphertext)
            .map_err(|_| AggregationError::Crypto("invalid token ciphertext".to_string()))?;
        let nonce = STANDARD
            .decode(nonce)
            .map_err(|_| AggregationError::Crypto("invalid token nonce".to_string()))?;
        let plaintext = self
            .cipher
            .decrypt(Nonce::from_slice(&nonce), ciphertext.as_ref())
            .map_err(|_| AggregationError::Crypto("failed to decrypt token".to_string()))?;
        String::from_utf8(plaintext)
            .map_err(|_| AggregationError::Crypto("token is not valid UTF-8".to_string()))
    }
}

/// Config status exposed to the UI (never the token).
#[derive(Debug, Clone)]
pub struct AggregationStatus {
    /// Provider name.
    pub provider: Option<String>,
    /// Whether sync is enabled.
    pub enabled: bool,
    /// Whether a token is stored.
    pub has_token: bool,
    /// Whether the provider adapter is available.
    pub provider_available: bool,
}

/// One linked account with provider and local values.
#[derive(Debug, Clone)]
pub struct AggregationLinkView {
    /// Link.
    pub link: AggregationLink,
    /// Latest provider balance.
    pub provider_balance: Option<AggregationBalance>,
    /// Latest local balance.
    pub local_balance: Option<Money>,
    /// Source of the local balance.
    pub local_source: Option<BalanceSource>,
    /// Which value is in effect.
    pub in_effect: String,
    /// Provider holdings.
    pub holdings: Vec<AggregationHolding>,
}

/// The aggregation view.
#[derive(Debug, Clone)]
pub struct AggregationView {
    /// Status.
    pub status: AggregationStatus,
    /// Links.
    pub links: Vec<AggregationLinkView>,
}

/// Aggregation use cases.
pub struct AggregationService {
    repo: Arc<dyn AggregationRepository>,
    accounts: Arc<dyn AccountRepository>,
    provider: Option<Arc<dyn AggregationProvider>>,
    cipher: Option<TokenCipher>,
}

impl AggregationService {
    /// Build the service.
    pub fn new(
        repo: Arc<dyn AggregationRepository>,
        accounts: Arc<dyn AccountRepository>,
        provider: Option<Arc<dyn AggregationProvider>>,
        cipher: Option<TokenCipher>,
    ) -> Self {
        Self {
            repo,
            accounts,
            provider,
            cipher,
        }
    }

    /// Save provider config. A token is encrypted before it touches the DB.
    pub async fn save_config(
        &self,
        provider: &str,
        enabled: bool,
        token: Option<&str>,
    ) -> Result<AggregationStatus, AggregationError> {
        let provider = provider.trim().to_ascii_lowercase();
        if provider.is_empty() {
            return Err(AggregationError::Validation(
                "provider is required".to_string(),
            ));
        }
        if enabled && self.provider.is_none() {
            return Err(AggregationError::Validation(
                "the configured provider adapter is not available".to_string(),
            ));
        }
        let (ciphertext, nonce) = match token.filter(|value| !value.trim().is_empty()) {
            Some(token) => {
                let cipher = self.cipher.as_ref().ok_or_else(|| {
                    AggregationError::Validation(
                        "AGGREGATION_ENCRYPTION_KEY is required to store a token".to_string(),
                    )
                })?;
                let (ciphertext, nonce) = cipher.encrypt(token.trim())?;
                (Some(ciphertext), Some(nonce))
            }
            None => (None, None),
        };
        let config = self
            .repo
            .save_config(&provider, enabled, ciphertext.as_deref(), nonce.as_deref())
            .await?;
        Ok(self.status_from_config(Some(config)))
    }

    /// Current status and linked accounts.
    pub async fn view(&self) -> Result<AggregationView, AggregationError> {
        let config = self.repo.config().await?;
        let links = self.repo.links().await?;
        let mut views = Vec::with_capacity(links.len());
        for link in links {
            let provider_balance = self.repo.latest_balance(link.id).await?;
            let balances = self.accounts.list_balances(link.account_id).await?;
            let local = balances.first().cloned();
            let in_effect = match (&local, &provider_balance) {
                (Some(local), Some(provider)) if local.source == BalanceSource::Manual => {
                    if local.as_of >= provider.as_of {
                        "manual".to_string()
                    } else {
                        "provider".to_string()
                    }
                }
                (Some(_), Some(_)) => "provider".to_string(),
                (Some(_), None) => "manual".to_string(),
                (None, Some(_)) => "provider".to_string(),
                (None, None) => "none".to_string(),
            };
            let holdings = self.repo.holdings(link.id).await?;
            views.push(AggregationLinkView {
                link,
                provider_balance,
                local_balance: local.as_ref().map(|balance| balance.amount.clone()),
                local_source: local.map(|balance| balance.source),
                in_effect,
                holdings,
            });
        }
        Ok(AggregationView {
            status: self.status_from_config(config),
            links: views,
        })
    }

    /// Sync all provider accounts. A failed fetch writes nothing.
    pub async fn sync(&self) -> Result<AggregationView, AggregationError> {
        let config = self.repo.config().await?.ok_or_else(|| {
            AggregationError::Validation("aggregation is not configured".to_string())
        })?;
        if !config.enabled {
            return Err(AggregationError::Validation(
                "aggregation is disabled".to_string(),
            ));
        }
        let provider = self.provider.as_ref().ok_or_else(|| {
            AggregationError::Validation("no aggregation provider adapter is available".to_string())
        })?;
        let cipher = self.cipher.as_ref().ok_or_else(|| {
            AggregationError::Validation("AGGREGATION_ENCRYPTION_KEY is required".to_string())
        })?;
        let (ciphertext, nonce) = match (&config.token_ciphertext, &config.token_nonce) {
            (Some(ciphertext), Some(nonce)) => (ciphertext, nonce),
            _ => {
                return Err(AggregationError::Validation(
                    "no provider token is configured".to_string(),
                ))
            }
        };
        let token = cipher.decrypt(ciphertext, nonce)?;
        let snapshot = provider
            .fetch(&token)
            .await
            .map_err(|ProviderError(message)| AggregationError::Provider(message))?;

        let now = OffsetDateTime::now_utc();
        let today = now.date();
        for account in snapshot.accounts {
            let local = match self.accounts.find_account_by_name(&account.name).await? {
                Some(account) => account,
                None => {
                    self.accounts
                        .insert_account(&NewAccount {
                            name: account.name.clone(),
                            account_type: account.account_type,
                            institution: Some(provider.name().to_string()),
                            currency: account.currency.clone(),
                            ownership_scope: OwnershipScope::Household,
                            owner_user_id: None,
                        })
                        .await?
                }
            };
            let link = self
                .repo
                .upsert_link(
                    &config.provider,
                    &account.provider_id,
                    local.id,
                    &account.name,
                    account.account_type,
                    &account.currency,
                )
                .await?;

            if let Some(balance) = &account.balance {
                self.repo
                    .upsert_balance(link.id, today, balance, now)
                    .await?;
                let local_balances = self.accounts.list_balances(local.id).await?;
                let manual_today = local_balances.iter().any(|balance| {
                    balance.as_of == today && balance.source == BalanceSource::Manual
                });
                if !manual_today {
                    self.accounts
                        .upsert_balance(&NewBalance {
                            account_id: local.id,
                            as_of: today,
                            amount: balance.clone(),
                            source: BalanceSource::Provider,
                        })
                        .await?;
                }
            }
            for holding in account.holdings {
                self.repo
                    .upsert_holding(
                        link.id,
                        &holding.symbol,
                        &holding.quantity,
                        &holding.value,
                        now,
                    )
                    .await?;
            }
            self.repo.touch_link(link.id, now).await?;
        }

        self.view().await
    }

    fn status_from_config(&self, config: Option<AggregationConfig>) -> AggregationStatus {
        AggregationStatus {
            provider: config.as_ref().map(|config| config.provider.clone()),
            enabled: config
                .as_ref()
                .map(|config| config.enabled)
                .unwrap_or(false),
            has_token: config
                .as_ref()
                .map(|config| config.token_ciphertext.is_some())
                .unwrap_or(false),
            provider_available: self.provider.is_some(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encrypts_and_decrypts_tokens() {
        let cipher = TokenCipher::new([7u8; 32]);
        let (ciphertext, nonce) = cipher.encrypt("my-provider-token").unwrap();
        assert_ne!(ciphertext, "my-provider-token");
        assert_eq!(
            cipher.decrypt(&ciphertext, &nonce).unwrap(),
            "my-provider-token"
        );
    }

    #[test]
    fn a_different_key_cannot_decrypt() {
        let first = TokenCipher::new([7u8; 32]);
        let second = TokenCipher::new([8u8; 32]);
        let (ciphertext, nonce) = first.encrypt("secret").unwrap();
        assert!(second.decrypt(&ciphertext, &nonce).is_err());
    }
}
