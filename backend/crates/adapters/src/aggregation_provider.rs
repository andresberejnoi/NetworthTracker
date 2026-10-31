//! A deterministic read-only aggregation provider for tests and demos.

use async_trait::async_trait;
use networth_application::ports::{
    AggregationProvider, ProviderAccount, ProviderError, ProviderHolding, ProviderSnapshot,
};
use networth_domain::{AccountType, Currency, Money, Quantity};

/// A fake provider with one brokerage account and one holding.
pub struct FakeAggregationProvider;

#[async_trait]
impl AggregationProvider for FakeAggregationProvider {
    fn name(&self) -> &'static str {
        "fake"
    }

    async fn fetch(&self, token: &str) -> Result<ProviderSnapshot, ProviderError> {
        if token.trim().is_empty() {
            return Err(ProviderError("a provider token is required".to_string()));
        }
        if token == "fail" {
            return Err(ProviderError(
                "the fake provider is unreachable".to_string(),
            ));
        }
        Ok(ProviderSnapshot {
            accounts: vec![ProviderAccount {
                provider_id: "fake-brokerage-1".to_string(),
                name: "Fake Brokerage".to_string(),
                account_type: AccountType::TaxableBrokerage,
                currency: Currency::usd(),
                balance: Some(Money::parse("12345.67", "USD").unwrap()),
                holdings: vec![ProviderHolding {
                    symbol: "VTI".to_string(),
                    quantity: Quantity::parse("10").unwrap(),
                    value: Money::parse("1500.00", "USD").unwrap(),
                }],
            }],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn returns_a_read_only_snapshot() {
        let snapshot = FakeAggregationProvider.fetch("token").await.unwrap();
        assert_eq!(snapshot.accounts.len(), 1);
        assert_eq!(snapshot.accounts[0].holdings[0].symbol, "VTI");
        assert!(FakeAggregationProvider.fetch("fail").await.is_err());
    }
}
