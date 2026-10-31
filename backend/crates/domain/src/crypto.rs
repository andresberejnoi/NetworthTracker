//! Cryptocurrency assets, wallets, balances, and public-address validation.
//!
//! The domain never handles private keys or seed phrases: only public addresses
//! are accepted. Balances are dated facts; tax lots live in the shared
//! `investments::Lot` ledger.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use crate::accounts::OwnershipScope;
use crate::currency::Currency;
use crate::quantity::Quantity;

/// A supported native chain in v1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Chain {
    /// Bitcoin mainnet.
    Bitcoin,
    /// Ethereum mainnet.
    Ethereum,
}

impl Chain {
    /// Stable identifier for storage and the API.
    pub fn as_str(self) -> &'static str {
        match self {
            Chain::Bitcoin => "bitcoin",
            Chain::Ethereum => "ethereum",
        }
    }
}

impl fmt::Display for Chain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Errors raised when parsing a [`Chain`].
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[error("unknown chain: {0}")]
pub struct ChainParseError(pub String);

impl FromStr for Chain {
    type Err = ChainParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "bitcoin" | "btc" => Ok(Chain::Bitcoin),
            "ethereum" | "eth" => Ok(Chain::Ethereum),
            other => Err(ChainParseError(other.to_string())),
        }
    }
}

/// Whether a wallet is an address or a manual bucket.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CryptoWalletKind {
    /// A public address synced from a chain provider.
    Address,
    /// A manually maintained bucket (exchange, hardware wallet, total).
    Manual,
}

impl CryptoWalletKind {
    /// Stable identifier for storage and the API.
    pub fn as_str(self) -> &'static str {
        match self {
            CryptoWalletKind::Address => "address",
            CryptoWalletKind::Manual => "manual",
        }
    }
}

impl fmt::Display for CryptoWalletKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Errors raised when parsing a [`CryptoWalletKind`].
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[error("unknown wallet kind: {0}")]
pub struct CryptoWalletKindParseError(pub String);

impl FromStr for CryptoWalletKind {
    type Err = CryptoWalletKindParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "address" => Ok(CryptoWalletKind::Address),
            "manual" => Ok(CryptoWalletKind::Manual),
            other => Err(CryptoWalletKindParseError(other.to_string())),
        }
    }
}

/// Where a crypto balance came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CryptoBalanceSource {
    /// Entered by the user.
    Manual,
    /// Fetched from a chain provider.
    Chain,
    /// Imported from a file.
    Import,
}

impl CryptoBalanceSource {
    /// Stable identifier for storage and the API.
    pub fn as_str(self) -> &'static str {
        match self {
            CryptoBalanceSource::Manual => "manual",
            CryptoBalanceSource::Chain => "chain",
            CryptoBalanceSource::Import => "import",
        }
    }
}

impl fmt::Display for CryptoBalanceSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Errors raised when parsing a [`CryptoBalanceSource`].
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[error("unknown balance source: {0}")]
pub struct CryptoBalanceSourceParseError(pub String);

impl FromStr for CryptoBalanceSource {
    type Err = CryptoBalanceSourceParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "manual" => Ok(CryptoBalanceSource::Manual),
            "chain" => Ok(CryptoBalanceSource::Chain),
            "import" => Ok(CryptoBalanceSource::Import),
            other => Err(CryptoBalanceSourceParseError(other.to_string())),
        }
    }
}

/// A crypto asset/native coin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CryptoAsset {
    /// Identifier.
    pub id: Uuid,
    /// Ticker symbol, uppercase.
    pub symbol: String,
    /// Display name.
    pub name: String,
    /// Native chain.
    pub chain: Chain,
    /// Optional token contract; native assets leave this empty.
    pub contract_address: Option<String>,
    /// Base-unit decimals.
    pub decimals: u32,
    /// Quote currency.
    pub currency: Currency,
    /// Provider-specific coin id (for example `btc-bitcoin`).
    pub price_provider_id: Option<String>,
}

/// Fields for creating a crypto asset.
#[derive(Debug, Clone)]
pub struct NewCryptoAsset {
    /// Ticker symbol.
    pub symbol: String,
    /// Display name.
    pub name: String,
    /// Native chain.
    pub chain: Chain,
    /// Optional token contract.
    pub contract_address: Option<String>,
    /// Base-unit decimals.
    pub decimals: u32,
    /// Quote currency.
    pub currency: Currency,
    /// Provider-specific coin id (for example `btc-bitcoin`).
    pub price_provider_id: Option<String>,
}

/// A crypto wallet/pool.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CryptoWallet {
    /// Identifier.
    pub id: Uuid,
    /// The asset this wallet tracks.
    pub asset: CryptoAsset,
    /// Address or manual bucket.
    pub kind: CryptoWalletKind,
    /// Chain.
    pub chain: Chain,
    /// Public address for address wallets.
    pub address: Option<String>,
    /// User-facing label.
    pub label: String,
    /// Household-visible or private.
    pub ownership_scope: OwnershipScope,
    /// Owner for a private wallet.
    pub owner_user_id: Option<Uuid>,
    /// Whether archived.
    pub archived: bool,
}

/// Fields for creating a crypto wallet.
#[derive(Debug, Clone)]
pub struct NewCryptoWallet {
    /// Asset identifier.
    pub asset_id: Uuid,
    /// Address or manual bucket.
    pub kind: CryptoWalletKind,
    /// Chain.
    pub chain: Chain,
    /// Public address for address wallets.
    pub address: Option<String>,
    /// User-facing label.
    pub label: String,
    /// Household-visible or private.
    pub ownership_scope: OwnershipScope,
    /// Owner for a private wallet.
    pub owner_user_id: Option<Uuid>,
}

/// A dated crypto balance observation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CryptoBalance {
    /// Identifier.
    pub id: Uuid,
    /// Wallet.
    pub wallet_id: Uuid,
    /// Observation date.
    pub as_of: Date,
    /// Exact quantity.
    pub quantity: Quantity,
    /// Where it came from.
    pub source: CryptoBalanceSource,
    /// Provider name, for chain syncs.
    pub provider: Option<String>,
    /// When the provider was queried.
    pub fetched_at: Option<OffsetDateTime>,
}

/// Fields for recording a crypto balance.
#[derive(Debug, Clone)]
pub struct NewCryptoBalance {
    /// Wallet.
    pub wallet_id: Uuid,
    /// Observation date.
    pub as_of: Date,
    /// Exact quantity.
    pub quantity: Quantity,
    /// Where it came from.
    pub source: CryptoBalanceSource,
    /// Provider name.
    pub provider: Option<String>,
    /// Fetch timestamp.
    pub fetched_at: Option<OffsetDateTime>,
}

/// Errors raised when validating a public crypto address.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum CryptoAddressError {
    /// The input is empty.
    #[error("address is required")]
    Empty,
    /// The input looks like a seed phrase.
    #[error("seed phrases are not accepted; enter a public address only")]
    SeedPhrase,
    /// The input looks like a private key.
    #[error("private keys are not accepted; enter a public address only")]
    PrivateKey,
    /// The address is not valid for the selected chain.
    #[error("invalid {chain} address: {value}")]
    Invalid {
        /// Chain name.
        chain: String,
        /// Rejected value.
        value: String,
    },
}

/// Validate and normalize a public address for a chain.
///
/// This rejects seed phrases and private keys before checking the public
/// address format. It never stores or logs the rejected secret.
pub fn validate_address(chain: Chain, input: &str) -> Result<String, CryptoAddressError> {
    let value = input.trim();
    if value.is_empty() {
        return Err(CryptoAddressError::Empty);
    }

    let words: Vec<&str> = value.split_whitespace().collect();
    if matches!(words.len(), 12 | 15 | 18 | 21 | 24)
        && words.iter().all(|word| {
            !word.is_empty()
                && word
                    .chars()
                    .all(|character| character.is_ascii_alphabetic())
        })
    {
        return Err(CryptoAddressError::SeedPhrase);
    }

    let without_prefix = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
        .unwrap_or(value);
    if without_prefix.len() == 64 && without_prefix.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(CryptoAddressError::PrivateKey);
    }
    if value.starts_with("xprv")
        || value.starts_with("yprv")
        || value.starts_with("zprv")
        || value.starts_with("tprv")
        || value.starts_with("uprv")
        || value.starts_with("vprv")
        || (matches!(value.chars().next(), Some('5' | 'K' | 'L' | 'c'))
            && value.len() >= 51
            && value.len() <= 52)
    {
        return Err(CryptoAddressError::PrivateKey);
    }
    if value.contains("\"cipher\"") || value.contains("\"crypto\"") {
        return Err(CryptoAddressError::PrivateKey);
    }

    match chain {
        Chain::Bitcoin => {
            let base58 = (value.starts_with('1') || value.starts_with('3'))
                && (26..=35).contains(&value.len())
                && value
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() && !matches!(c, '0' | 'O' | 'I' | 'l'));
            let bech32 = value.starts_with("bc1")
                && (14..=74).contains(&value.len())
                && value
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit());
            if !base58 && !bech32 {
                return Err(CryptoAddressError::Invalid {
                    chain: chain.as_str().to_string(),
                    value: value.to_string(),
                });
            }
            Ok(value.to_string())
        }
        Chain::Ethereum => {
            let valid = value.len() == 42
                && (value.starts_with("0x") || value.starts_with("0X"))
                && value[2..].chars().all(|c| c.is_ascii_hexdigit());
            if !valid {
                return Err(CryptoAddressError::Invalid {
                    chain: chain.as_str().to_string(),
                    value: value.to_string(),
                });
            }
            Ok(format!("0x{}", value[2..].to_ascii_lowercase()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_bitcoin_addresses() {
        let legacy = "1BoatSLRHtKNngkdXEeobR76b53LETtpyT";
        let bech32 = "bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq";
        assert_eq!(validate_address(Chain::Bitcoin, legacy).unwrap(), legacy);
        assert_eq!(validate_address(Chain::Bitcoin, bech32).unwrap(), bech32);
    }

    #[test]
    fn accepts_and_normalizes_ethereum_addresses() {
        let address = "0x52908400098527886E0F7030069857D2E4169EE7";
        assert_eq!(
            validate_address(Chain::Ethereum, address).unwrap(),
            "0x52908400098527886e0f7030069857d2e4169ee7"
        );
    }

    #[test]
    fn rejects_seed_phrases_and_private_keys() {
        let seed =
            "abandon ability able about above absent absorb abstract absurd abuse access accident";
        assert_eq!(
            validate_address(Chain::Bitcoin, seed),
            Err(CryptoAddressError::SeedPhrase)
        );
        let key = format!("0x{}", "0123456789abcdef".repeat(4));
        assert_eq!(
            validate_address(Chain::Ethereum, key),
            Err(CryptoAddressError::PrivateKey)
        );
        let xprv = "xprv9s21ZrQH143K3QTDL4LXw2F7HEK3wJUD2nW2nRk4stbPy6cq3jPPqjiChkVvvNKmPGJxWUtg6LnF5kejMRNNU3TGtRBeJgk33yuGBxrMPHi";
        assert_eq!(
            validate_address(Chain::Bitcoin, xprv),
            Err(CryptoAddressError::PrivateKey)
        );
    }

    #[test]
    fn rejects_wrong_chain_formats() {
        assert!(matches!(
            validate_address(Chain::Ethereum, "1BoatSLRHtKNngkdXEeobR76b53LETtpyT"),
            Err(CryptoAddressError::Invalid { .. })
        ));
        assert!(matches!(
            validate_address(Chain::Bitcoin, "0x52908400098527886E0F7030069857D2E4169EE7"),
            Err(CryptoAddressError::Invalid { .. })
        ));
    }
}
