//! Prices, caching, user-supplied providers, and FX conversion.

use std::sync::Arc;

use bigdecimal::BigDecimal;
use networth_domain::{Currency, Money, PriceQuote, PriceSource};
use time::{Duration, OffsetDateTime};

use crate::ports::{
    FxRepository, PriceProvider, PriceRepository, ProviderError, RepositoryError, StoredPrice,
};

/// How long a provider price is considered fresh.
pub const DEFAULT_MAX_AGE_SECS: i64 = 900;

/// Errors raised by price lookups.
#[derive(Debug, thiserror::Error)]
pub enum PriceError {
    /// Persistence failed.
    #[error(transparent)]
    Repository(#[from] RepositoryError),
    /// The provider failed and no cached price exists.
    #[error("price provider error: {0}")]
    Provider(String),
}

/// Errors raised by FX conversion.
#[derive(Debug, thiserror::Error)]
pub enum FxError {
    /// Input failed validation.
    #[error("{0}")]
    Validation(String),
    /// Persistence failed.
    #[error(transparent)]
    Repository(#[from] RepositoryError),
}

/// Configuration for an optional, user-supplied price provider.
#[derive(Debug, Clone, Default)]
pub struct ProviderConfig {
    /// Provider name, for example `static`.
    pub provider: Option<String>,
    /// The user's own API key or subscription token.
    pub api_key: Option<String>,
}

impl ProviderConfig {
    /// Read the configuration from the environment. Secrets stay in `.env`.
    pub fn from_env() -> Self {
        let provider = std::env::var("PRICE_PROVIDER")
            .ok()
            .map(|value| value.trim().to_ascii_lowercase())
            .filter(|value| !value.is_empty() && value != "none");
        let api_key = std::env::var("PRICE_PROVIDER_API_KEY")
            .ok()
            .filter(|value| !value.trim().is_empty());

        Self { provider, api_key }
    }

    /// Whether a provider is enabled.
    pub fn enabled(&self) -> bool {
        self.provider.is_some()
    }
}

/// Price lookup with manual override, caching, and offline fallback.
pub struct PriceService {
    repo: Arc<dyn PriceRepository>,
    provider: Option<Arc<dyn PriceProvider>>,
    max_age: Duration,
}

impl PriceService {
    /// Build the service. `max_age_secs` controls how long a provider quote is
    /// considered fresh.
    pub fn new(
        repo: Arc<dyn PriceRepository>,
        provider: Option<Arc<dyn PriceProvider>>,
        max_age_secs: i64,
    ) -> Self {
        Self {
            repo,
            provider,
            max_age: Duration::seconds(max_age_secs),
        }
    }

    /// The configured provider name, if any.
    pub fn provider_name(&self) -> Option<&'static str> {
        self.provider.as_ref().map(|provider| provider.name())
    }

    /// Look up a price, preferring a manual override.
    pub async fn quote(&self, symbol: &str) -> Result<Option<PriceQuote>, PriceError> {
        let symbol = normalize_symbol(symbol);
        if symbol.is_empty() {
            return Ok(None);
        }

        let latest = self.repo.latest_price(&symbol).await?;

        // A manual price always wins, even if it is old.
        if let Some(stored) = &latest {
            if stored.source == PriceSource::Manual {
                return Ok(Some(to_quote(&symbol, stored, false)));
            }
        }

        if let Some(provider) = &self.provider {
            let fresh = latest
                .as_ref()
                .map(|stored| stored.as_of >= OffsetDateTime::now_utc() - self.max_age)
                .unwrap_or(false);

            if fresh {
                return Ok(latest.map(|stored| to_quote(&symbol, &stored, false)));
            }

            match provider.quote(&symbol).await {
                Ok(Some(quote)) => {
                    self.repo
                        .insert_price(&symbol, &quote.price, quote.as_of, PriceSource::Provider)
                        .await?;
                    return Ok(Some(PriceQuote {
                        symbol,
                        price: quote.price,
                        as_of: quote.as_of,
                        stale: false,
                        source: PriceSource::Provider,
                    }));
                }
                Ok(None) => {}
                Err(ProviderError(message)) => {
                    if latest.is_some() {
                        return Ok(latest.map(|stored| to_quote(&symbol, &stored, true)));
                    }
                    return Err(PriceError::Provider(message));
                }
            }
        }

        // No provider: use the last known price and mark it stale.
        Ok(latest.map(|stored| to_quote(&symbol, &stored, true)))
    }

    /// Record a manual price override.
    pub async fn set_manual(&self, symbol: &str, price: Money) -> Result<PriceQuote, PriceError> {
        let symbol = normalize_symbol(symbol);
        if symbol.is_empty() {
            return Err(PriceError::Repository(RepositoryError::Storage(
                "symbol is required".to_string(),
            )));
        }

        let as_of = OffsetDateTime::now_utc();
        self.repo
            .insert_price(&symbol, &price, as_of, PriceSource::Manual)
            .await?;

        Ok(PriceQuote {
            symbol,
            price,
            as_of,
            stale: false,
            source: PriceSource::Manual,
        })
    }
}

fn to_quote(symbol: &str, stored: &StoredPrice, stale: bool) -> PriceQuote {
    PriceQuote {
        symbol: symbol.to_string(),
        price: stored.price.clone(),
        as_of: stored.as_of,
        stale,
        source: stored.source,
    }
}

fn normalize_symbol(symbol: &str) -> String {
    symbol.trim().to_ascii_uppercase()
}

/// FX rate storage and conversion.
pub struct FxService {
    repo: Arc<dyn FxRepository>,
}

impl FxService {
    /// Build the service.
    pub fn new(repo: Arc<dyn FxRepository>) -> Self {
        Self { repo }
    }

    /// Record a rate from `base` to `quote`.
    pub async fn set_rate(
        &self,
        base: Currency,
        quote: Currency,
        rate: BigDecimal,
    ) -> Result<(), FxError> {
        if rate <= 0 {
            return Err(FxError::Validation("rate must be positive".to_string()));
        }
        self.repo
            .insert_rate(&base, &quote, &rate, OffsetDateTime::now_utc())
            .await?;
        Ok(())
    }

    /// The latest rate from `base` to `quote`.
    pub async fn rate(
        &self,
        base: &Currency,
        quote: &Currency,
    ) -> Result<Option<BigDecimal>, FxError> {
        if base == quote {
            return Ok(Some(BigDecimal::from(1)));
        }
        Ok(self
            .repo
            .latest_rate(base, quote)
            .await?
            .map(|stored| stored.rate))
    }

    /// Convert an amount into the target currency using the latest rate.
    pub async fn convert(&self, amount: &Money, target: &Currency) -> Result<Money, FxError> {
        if amount.currency() == target {
            return Ok(amount.clone());
        }

        let rate = self.rate(amount.currency(), target).await?.ok_or_else(|| {
            FxError::Validation(format!(
                "no FX rate from {} to {}",
                amount.currency(),
                target
            ))
        })?;

        Ok(Money::new(amount.amount() * rate, target.clone()))
    }
}
