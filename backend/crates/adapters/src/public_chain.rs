//! Free/public chain adapters for Bitcoin and Ethereum.
//!
//! Bitcoin uses mempool.space (no key). Ethereum uses Blockscout by default or
//! Etherscan V2 with a user-supplied `CHAIN_PROVIDER_API_KEY`. Both are
//! read-only public-address lookups; no private keys are involved.

use std::str::FromStr;

use async_trait::async_trait;
use bigdecimal::BigDecimal;
use networth_application::ports::{ChainBalance, ChainDataProvider, ProviderError};
use networth_domain::{Chain, Quantity};
use serde_json::Value;
use time::OffsetDateTime;

/// Public chain provider with optional Etherscan credentials.
pub struct PublicChainProvider {
    client: reqwest::Client,
    etherscan_key: Option<String>,
}

impl PublicChainProvider {
    /// Build with an optional user-supplied Etherscan key.
    pub fn new(etherscan_key: Option<String>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            client,
            etherscan_key,
        }
    }
}

#[async_trait]
impl ChainDataProvider for PublicChainProvider {
    fn name(&self) -> &'static str {
        "public"
    }

    fn supports(&self, _chain: Chain) -> bool {
        true
    }

    async fn balance(
        &self,
        chain: Chain,
        address: &str,
    ) -> Result<Option<ChainBalance>, ProviderError> {
        let (quantity, provider) = match chain {
            Chain::Bitcoin => {
                let response = self
                    .client
                    .get(format!("https://mempool.space/api/address/{address}"))
                    .send()
                    .await
                    .map_err(|error| ProviderError(error.to_string()))?;
                if !response.status().is_success() {
                    return Err(ProviderError(format!(
                        "mempool.space returned {}",
                        response.status()
                    )));
                }
                let body: Value = response
                    .json()
                    .await
                    .map_err(|error| ProviderError(error.to_string()))?;
                let funded = body["chain_stats"]["funded_txo_sum"]
                    .as_u64()
                    .ok_or_else(|| {
                        ProviderError("mempool.space response missing funded sum".into())
                    })?;
                let spent = body["chain_stats"]["spent_txo_sum"]
                    .as_u64()
                    .ok_or_else(|| {
                        ProviderError("mempool.space response missing spent sum".into())
                    })?;
                (parse_sats(funded.saturating_sub(spent)), "mempool.space")
            }
            Chain::Ethereum => {
                if let Some(key) = &self.etherscan_key {
                    let response = self
                        .client
                        .get("https://api.etherscan.io/v2/api")
                        .query(&[
                            ("chainid", "1"),
                            ("module", "account"),
                            ("action", "balance"),
                            ("address", address),
                            ("tag", "latest"),
                            ("apikey", key.as_str()),
                        ])
                        .send()
                        .await
                        .map_err(|error| ProviderError(error.to_string()))?;
                    if !response.status().is_success() {
                        return Err(ProviderError(format!(
                            "Etherscan returned {}",
                            response.status()
                        )));
                    }
                    let body: Value = response
                        .json()
                        .await
                        .map_err(|error| ProviderError(error.to_string()))?;
                    let wei = body["result"]
                        .as_str()
                        .ok_or_else(|| ProviderError("Etherscan response missing result".into()))?;
                    (parse_wei(wei)?, "etherscan")
                } else {
                    let response = self
                        .client
                        .get(format!(
                            "https://eth.blockscout.com/api/v2/addresses/{address}"
                        ))
                        .send()
                        .await
                        .map_err(|error| ProviderError(error.to_string()))?;
                    if !response.status().is_success() {
                        return Err(ProviderError(format!(
                            "Blockscout returned {}",
                            response.status()
                        )));
                    }
                    let body: Value = response
                        .json()
                        .await
                        .map_err(|error| ProviderError(error.to_string()))?;
                    let wei = body["coin_balance"].as_str().ok_or_else(|| {
                        ProviderError("Blockscout response missing coin_balance".into())
                    })?;
                    (parse_wei(wei)?, "blockscout")
                }
            }
        };

        Ok(Some(ChainBalance {
            quantity,
            as_of: OffsetDateTime::now_utc(),
            provider: provider.to_string(),
        }))
    }
}

fn parse_sats(sats: u64) -> Quantity {
    Quantity::new(BigDecimal::from(sats) / BigDecimal::from(100_000_000u64))
        .expect("sats are non-negative")
}

fn parse_wei(wei: &str) -> Result<Quantity, ProviderError> {
    let value = BigDecimal::from_str(wei.trim())
        .map_err(|error| ProviderError(format!("invalid wei amount: {error}")))?;
    Quantity::new(value / BigDecimal::from(1_000_000_000_000_000_000u64))
        .map_err(|error| ProviderError(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_sats_exactly() {
        assert_eq!(parse_sats(1).to_string(), "0.00000001");
        assert_eq!(parse_sats(100_000_000).to_string(), "1");
    }

    #[test]
    fn converts_wei_exactly() {
        assert_eq!(parse_wei("1").unwrap().to_string(), "0.000000000000000001");
        assert_eq!(parse_wei("1000000000000000000").unwrap().to_string(), "1");
        assert!(parse_wei("nope").is_err());
    }
}
