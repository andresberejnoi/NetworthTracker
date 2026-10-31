//! Crypto assets, wallets, balances, and valuation.

use std::collections::HashMap;
use std::sync::Arc;

use networth_domain::{
    validate_address, Account, AccountType, Balance, BalanceSource, Chain, CryptoAddressError,
    CryptoAsset, CryptoBalance, CryptoBalanceSource, CryptoWallet, CryptoWalletKind, Currency,
    Money, NewCryptoAsset, NewCryptoBalance, NewCryptoWallet, OwnershipScope, PriceQuote, Quantity,
};
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use crate::market_data::{PriceError, PriceService};
use crate::ports::{ChainDataProvider, CryptoRepository, ProviderError, RepositoryError};

/// Errors raised by crypto use cases.
#[derive(Debug, thiserror::Error)]
pub enum CryptoError {
    /// Input failed validation.
    #[error("{0}")]
    Validation(String),
    /// The resource does not exist.
    #[error("{0}")]
    NotFound(String),
    /// Persistence failed.
    #[error(transparent)]
    Repository(#[from] RepositoryError),
    /// Price lookup failed.
    #[error(transparent)]
    Price(#[from] PriceError),
    /// The chain provider failed.
    #[error("chain provider error: {0}")]
    Provider(String),
}

/// A crypto asset aggregated across wallets.
#[derive(Debug, Clone)]
pub struct CryptoPosition {
    /// The asset.
    pub asset: CryptoAsset,
    /// Total quantity across wallets.
    pub quantity: Quantity,
    /// Number of wallets contributing a balance.
    pub wallet_count: usize,
    /// Latest price, if available.
    pub price: Option<PriceQuote>,
    /// Quantity times price.
    pub value: Option<Money>,
}

/// Crypto use cases.
pub struct CryptoService {
    repo: Arc<dyn CryptoRepository>,
    prices: Arc<PriceService>,
    chain: Option<Arc<dyn ChainDataProvider>>,
}

impl CryptoService {
    /// Build the service.
    pub fn new(
        repo: Arc<dyn CryptoRepository>,
        prices: Arc<PriceService>,
        chain: Option<Arc<dyn ChainDataProvider>>,
    ) -> Self {
        Self {
            repo,
            prices,
            chain,
        }
    }

    /// The configured chain provider name, if any.
    pub fn provider_name(&self) -> Option<&'static str> {
        self.chain.as_ref().map(|provider| provider.name())
    }

    /// Create a crypto asset.
    pub async fn create_asset(&self, new: NewCryptoAsset) -> Result<CryptoAsset, CryptoError> {
        let symbol = new.symbol.trim().to_ascii_uppercase();
        if symbol.is_empty() {
            return Err(CryptoError::Validation("symbol is required".to_string()));
        }
        if new.name.trim().is_empty() {
            return Err(CryptoError::Validation("name is required".to_string()));
        }
        if new.decimals > 36 {
            return Err(CryptoError::Validation(
                "decimals must be between 0 and 36".to_string(),
            ));
        }
        Ok(self
            .repo
            .create_asset(&NewCryptoAsset {
                symbol,
                name: new.name.trim().to_string(),
                chain: new.chain,
                contract_address: new.contract_address,
                decimals: new.decimals,
                currency: new.currency,
                price_provider_id: new.price_provider_id,
            })
            .await?)
    }

    /// List crypto assets.
    pub async fn list_assets(&self) -> Result<Vec<CryptoAsset>, CryptoError> {
        Ok(self.repo.list_assets().await?)
    }

    /// Create a wallet, validating address wallets and rejecting secrets.
    pub async fn create_wallet(&self, new: NewCryptoWallet) -> Result<CryptoWallet, CryptoError> {
        let asset = self
            .repo
            .list_assets()
            .await?
            .into_iter()
            .find(|asset| asset.id == new.asset_id)
            .ok_or_else(|| CryptoError::NotFound(format!("asset not found: {}", new.asset_id)))?;
        if asset.chain != new.chain {
            return Err(CryptoError::Validation(format!(
                "wallet chain {} does not match asset chain {}",
                new.chain, asset.chain
            )));
        }
        if new.label.trim().is_empty() {
            return Err(CryptoError::Validation("label is required".to_string()));
        }

        let address = match new.kind {
            CryptoWalletKind::Address => {
                let raw = new.address.as_deref().unwrap_or_default();
                Some(
                    validate_address(new.chain, raw)
                        .map_err(|error| CryptoError::Validation(error.to_string()))?,
                )
            }
            CryptoWalletKind::Manual => None,
        };

        Ok(self
            .repo
            .create_wallet(&NewCryptoWallet {
                asset_id: asset.id,
                kind: new.kind,
                chain: new.chain,
                address,
                label: new.label.trim().to_string(),
                ownership_scope: new.ownership_scope,
                owner_user_id: new.owner_user_id,
            })
            .await?)
    }

    /// List wallets.
    pub async fn list_wallets(&self) -> Result<Vec<CryptoWallet>, CryptoError> {
        Ok(self.repo.list_wallets().await?)
    }

    /// Record a manual balance.
    pub async fn record_balance(
        &self,
        wallet_id: Uuid,
        as_of: Date,
        quantity: Quantity,
    ) -> Result<CryptoBalance, CryptoError> {
        if self.repo.find_wallet(wallet_id).await?.is_none() {
            return Err(CryptoError::NotFound(format!(
                "wallet not found: {wallet_id}"
            )));
        }
        Ok(self
            .repo
            .upsert_crypto_balance(&NewCryptoBalance {
                wallet_id,
                as_of,
                quantity,
                source: CryptoBalanceSource::Manual,
                provider: None,
                fetched_at: None,
            })
            .await?)
    }

    /// List a wallet's balances.
    pub async fn list_balances(&self, wallet_id: Uuid) -> Result<Vec<CryptoBalance>, CryptoError> {
        if self.repo.find_wallet(wallet_id).await?.is_none() {
            return Err(CryptoError::NotFound(format!(
                "wallet not found: {wallet_id}"
            )));
        }
        Ok(self.repo.list_balances(wallet_id).await?)
    }

    /// Sync an address wallet through the configured chain provider.
    pub async fn sync_wallet(&self, wallet_id: Uuid) -> Result<CryptoBalance, CryptoError> {
        let wallet = self
            .repo
            .find_wallet(wallet_id)
            .await?
            .ok_or_else(|| CryptoError::NotFound(format!("wallet not found: {wallet_id}")))?;
        if wallet.kind != CryptoWalletKind::Address {
            return Err(CryptoError::Validation(
                "only address wallets can be synced".to_string(),
            ));
        }
        let address = wallet
            .address
            .as_deref()
            .ok_or_else(|| CryptoError::Validation("wallet has no address".to_string()))?;
        let provider = self.chain.as_ref().ok_or_else(|| {
            CryptoError::Validation("no chain provider is configured".to_string())
        })?;
        if !provider.supports(wallet.chain) {
            return Err(CryptoError::Validation(format!(
                "the configured provider does not support {}",
                wallet.chain
            )));
        }

        let balance = provider
            .balance(wallet.chain, address)
            .await
            .map_err(|ProviderError(message)| CryptoError::Provider(message))?
            .ok_or_else(|| CryptoError::NotFound("the provider returned no balance".to_string()))?;

        Ok(self
            .repo
            .upsert_crypto_balance(&NewCryptoBalance {
                wallet_id,
                as_of: OffsetDateTime::now_utc().date(),
                quantity: balance.quantity,
                source: CryptoBalanceSource::Chain,
                provider: Some(balance.provider),
                fetched_at: Some(balance.as_of),
            })
            .await?)
    }

    /// Aggregate the latest balances by asset and value them.
    pub async fn positions(
        &self,
        as_of: Date,
        reporting: &Currency,
    ) -> Result<Vec<CryptoPosition>, CryptoError> {
        let assets = self.repo.list_assets().await?;
        let wallets = self.repo.list_wallets().await?;
        let balances = self.repo.latest_balances(as_of).await?;

        let asset_by_id: HashMap<Uuid, CryptoAsset> =
            assets.into_iter().map(|asset| (asset.id, asset)).collect();
        let wallet_asset: HashMap<Uuid, Uuid> = wallets
            .iter()
            .map(|wallet| (wallet.id, wallet.asset.id))
            .collect();

        let mut totals: HashMap<Uuid, (Quantity, usize)> = HashMap::new();
        for balance in balances {
            let Some(asset_id) = wallet_asset.get(&balance.wallet_id) else {
                continue;
            };
            let entry = totals
                .entry(*asset_id)
                .or_insert_with(|| (Quantity::zero(), 0));
            entry.0 = entry.0.checked_add(&balance.quantity);
            entry.1 += 1;
        }

        let mut positions = Vec::with_capacity(totals.len());
        for (asset_id, (quantity, wallet_count)) in totals {
            let Some(asset) = asset_by_id.get(&asset_id).cloned() else {
                continue;
            };
            let price = self.prices.quote(&asset.symbol).await?;
            let value = price.as_ref().and_then(|quote| {
                if quote.price.currency() == reporting {
                    Some(Money::new(
                        quantity.as_decimal() * quote.price.amount(),
                        reporting.clone(),
                    ))
                } else {
                    None
                }
            });
            positions.push(CryptoPosition {
                asset,
                quantity,
                wallet_count,
                price,
                value,
            });
        }
        positions.sort_by(|a, b| a.asset.symbol.cmp(&b.asset.symbol));
        Ok(positions)
    }

    /// Build synthetic crypto accounts/balances for the net worth engine.
    pub async fn account_values(
        &self,
        as_of: Date,
        reporting: &Currency,
    ) -> Result<(Vec<Account>, Vec<Balance>), CryptoError> {
        let positions = self.positions(as_of, reporting).await?;
        let mut accounts = Vec::new();
        let mut balances = Vec::new();
        for position in positions {
            let Some(value) = position.value else {
                continue;
            };
            accounts.push(Account {
                id: position.asset.id,
                name: position.asset.symbol.clone(),
                account_type: AccountType::Crypto,
                institution: Some(position.asset.chain.to_string()),
                currency: reporting.clone(),
                ownership_scope: OwnershipScope::Household,
                owner_user_id: None,
                archived: false,
            });
            balances.push(Balance {
                id: Uuid::new_v4(),
                account_id: position.asset.id,
                as_of,
                amount: value,
                source: BalanceSource::Provider,
            });
        }
        Ok((accounts, balances))
    }
}

/// Validate an address for the API without creating a wallet.
pub fn validate_public_address(chain: Chain, address: &str) -> Result<String, CryptoAddressError> {
    validate_address(chain, address)
}
