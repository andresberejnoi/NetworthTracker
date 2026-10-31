//! A deterministic fake chain provider for offline tests and demos.
//!
//! Enable it with `CHAIN_PROVIDER=fake` and optionally seed balances with
//! `CHAIN_FAKE_BALANCES=bitcoin:bc1...=1.25,ethereum:0x...=2.5`. Real chain
//! adapters implement the same `ChainDataProvider` trait.

use std::collections::HashMap;

use async_trait::async_trait;
use networth_application::ports::{ChainBalance, ChainDataProvider, ProviderError};
use networth_domain::{Chain, Quantity};
use time::OffsetDateTime;

/// A chain provider backed by a fixed address-to-balance table.
pub struct FakeChainProvider {
    balances: HashMap<(Chain, String), Quantity>,
}

impl FakeChainProvider {
    /// Build from an explicit table.
    pub fn new(balances: HashMap<(Chain, String), Quantity>) -> Self {
        Self { balances }
    }

    /// Build from `CHAIN_FAKE_BALANCES`.
    pub fn from_env() -> Self {
        let mut balances = HashMap::new();
        if let Ok(value) = std::env::var("CHAIN_FAKE_BALANCES") {
            for entry in value.split(',') {
                let Some((key, amount)) = entry.split_once('=') else {
                    continue;
                };
                let Some((chain, address)) = key.split_once(':') else {
                    continue;
                };
                let (Ok(chain), Ok(quantity)) = (chain.parse::<Chain>(), Quantity::parse(amount))
                else {
                    continue;
                };
                balances.insert((chain, address.trim().to_string()), quantity);
            }
        }
        Self { balances }
    }
}

#[async_trait]
impl ChainDataProvider for FakeChainProvider {
    fn name(&self) -> &'static str {
        "fake"
    }

    fn supports(&self, _chain: Chain) -> bool {
        true
    }

    async fn balance(
        &self,
        chain: Chain,
        address: &str,
    ) -> Result<Option<ChainBalance>, ProviderError> {
        Ok(self
            .balances
            .get(&(chain, address.to_string()))
            .cloned()
            .map(|quantity| ChainBalance {
                quantity,
                as_of: OffsetDateTime::now_utc(),
                provider: "fake".to_string(),
            }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn returns_configured_balances() {
        let address = "bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq";
        let mut table = HashMap::new();
        table.insert(
            (Chain::Bitcoin, address.to_string()),
            Quantity::parse("0.000000000000000001").unwrap(),
        );
        let provider = FakeChainProvider::new(table);

        let balance = provider
            .balance(Chain::Bitcoin, address)
            .await
            .unwrap()
            .expect("balance");
        assert_eq!(balance.quantity.to_string(), "0.000000000000000001");
        assert!(provider
            .balance(Chain::Bitcoin, "unknown")
            .await
            .unwrap()
            .is_none());
    }
}
