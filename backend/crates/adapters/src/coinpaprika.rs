//! Free, keyless CoinPaprika price adapter for crypto symbols.
//!
//! Prices are market observations, not account amounts. The adapter maps common
//! crypto symbols to CoinPaprika coin ids; unknown symbols return no quote so
//! the caller can fall back to a cached or manual price.

use std::collections::HashMap;

use async_trait::async_trait;
use networth_application::ports::{PriceProvider, ProviderError, ProviderQuote};
use networth_domain::Money;
use serde_json::Value;
use time::OffsetDateTime;

/// CoinPaprika price adapter.
pub struct CoinPaprikaPriceProvider {
    client: reqwest::Client,
    base_url: String,
}

impl CoinPaprikaPriceProvider {
    /// Build with the public CoinPaprika endpoint.
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
            base_url: std::env::var("COINPAPRIKA_BASE_URL")
                .unwrap_or_else(|_| "https://api.coinpaprika.com/v1".to_string()),
        }
    }

    fn coin_id(symbol: &str) -> Option<&'static str> {
        let mut map = HashMap::new();
        map.insert("BTC", "btc-bitcoin");
        map.insert("ETH", "eth-ethereum");
        map.insert("LTC", "ltc-litecoin");
        map.insert("SOL", "sol-solana");
        map.insert("USDC", "usdc-usd-coin");
        map.insert("USDT", "usdt-tether");
        map.insert("DOGE", "doge-dogecoin");
        map.insert("ADA", "ada-cardano");
        map.insert("XRP", "xrp-xrp");
        map.insert("DOT", "dot-polkadot");
        map.insert("LINK", "link-chainlink");
        map.insert("BCH", "bch-bitcoin-cash");
        map.get(symbol.to_ascii_uppercase().as_str()).copied()
    }
}

impl Default for CoinPaprikaPriceProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PriceProvider for CoinPaprikaPriceProvider {
    fn name(&self) -> &'static str {
        "coinpaprika"
    }

    async fn quote(&self, symbol: &str) -> Result<Option<ProviderQuote>, ProviderError> {
        let Some(coin_id) = Self::coin_id(symbol) else {
            return Ok(None);
        };
        let response = self
            .client
            .get(format!("{}/tickers/{coin_id}", self.base_url))
            .send()
            .await
            .map_err(|error| ProviderError(error.to_string()))?;
        if !response.status().is_success() {
            return Err(ProviderError(format!(
                "CoinPaprika returned {}",
                response.status()
            )));
        }
        let body: Value = response
            .json()
            .await
            .map_err(|error| ProviderError(error.to_string()))?;
        let Some(price) = body["quotes"]["USD"]["price"].as_f64() else {
            return Ok(None);
        };
        let price = Money::parse(&price.to_string(), "USD")
            .map_err(|error| ProviderError(error.to_string()))?;
        Ok(Some(ProviderQuote {
            price,
            as_of: OffsetDateTime::now_utc(),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_common_crypto_symbols() {
        assert_eq!(
            CoinPaprikaPriceProvider::coin_id("btc"),
            Some("btc-bitcoin")
        );
        assert_eq!(
            CoinPaprikaPriceProvider::coin_id("ETH"),
            Some("eth-ethereum")
        );
        assert_eq!(CoinPaprikaPriceProvider::coin_id("NOPE"), None);
    }
}
