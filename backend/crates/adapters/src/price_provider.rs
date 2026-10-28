//! A simple user-supplied static price provider.
//!
//! Enable it with `PRICE_PROVIDER=static` and supply quotes in
//! `PRICE_STATIC_PRICES` (for example `AAPL=190.00,VTI=280.00`). It is inert
//! when the provider is not enabled, and it is intentionally the only bundled
//! provider: real providers are adapters the user configures with their own key.

use std::collections::HashMap;

use async_trait::async_trait;
use networth_application::ports::{PriceProvider, ProviderError, ProviderQuote};
use networth_domain::Money;
use time::OffsetDateTime;

/// A price provider backed by a static symbol-to-price table.
pub struct StaticPriceProvider {
    prices: HashMap<String, Money>,
}

impl StaticPriceProvider {
    /// Build from an explicit table.
    pub fn new(prices: HashMap<String, Money>) -> Self {
        Self { prices }
    }

    /// Build from `PRICE_STATIC_PRICES`.
    pub fn from_env() -> Self {
        let mut prices = HashMap::new();

        if let Ok(value) = std::env::var("PRICE_STATIC_PRICES") {
            for pair in value.split(',') {
                let Some((symbol, amount)) = pair.split_once('=') else {
                    continue;
                };
                if let Ok(price) = Money::parse(amount.trim(), "USD") {
                    prices.insert(symbol.trim().to_ascii_uppercase(), price);
                }
            }
        }

        Self { prices }
    }
}

#[async_trait]
impl PriceProvider for StaticPriceProvider {
    fn name(&self) -> &'static str {
        "static"
    }

    async fn quote(&self, symbol: &str) -> Result<Option<ProviderQuote>, ProviderError> {
        Ok(self
            .prices
            .get(&symbol.to_ascii_uppercase())
            .cloned()
            .map(|price| ProviderQuote {
                price,
                as_of: OffsetDateTime::now_utc(),
            }))
    }
}
