//! Request and response bodies for the HTTP API.
//!
//! Amounts are always strings, matching the domain's money rules.

use networth_domain::{Account, Balance};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// An account as returned by the API.
#[derive(Debug, Serialize)]
pub struct AccountDto {
    /// Surrogate identifier.
    pub id: Uuid,
    /// User-facing name.
    pub name: String,
    /// Stable account type identifier.
    pub account_type: String,
    /// Institution or custodian, if known.
    pub institution: Option<String>,
    /// Currency code.
    pub currency: String,
    /// `household` or `user`.
    pub ownership_scope: String,
    /// Whether the account is archived.
    pub archived: bool,
}

impl From<Account> for AccountDto {
    fn from(account: Account) -> Self {
        Self {
            id: account.id,
            name: account.name,
            account_type: account.account_type.as_str().to_string(),
            institution: account.institution,
            currency: account.currency.to_string(),
            ownership_scope: account.ownership_scope.as_str().to_string(),
            archived: account.archived,
        }
    }
}

fn default_currency() -> String {
    "USD".to_string()
}

fn default_scope() -> String {
    "household".to_string()
}

/// Request body for creating an account.
#[derive(Debug, Deserialize)]
pub struct CreateAccountRequest {
    /// User-facing name.
    pub name: String,
    /// Account type identifier, for example `401k` or `checking`.
    pub account_type: String,
    /// Institution or custodian, if known.
    #[serde(default)]
    pub institution: Option<String>,
    /// Currency code; defaults to `USD`.
    #[serde(default = "default_currency")]
    pub currency: String,
    /// `household` or `user`; defaults to `household`.
    #[serde(default = "default_scope")]
    pub ownership_scope: String,
}

/// A dated balance as returned by the API.
#[derive(Debug, Serialize)]
pub struct BalanceDto {
    /// Surrogate identifier.
    pub id: Uuid,
    /// The account this balance belongs to.
    pub account_id: Uuid,
    /// ISO date (`YYYY-MM-DD`).
    pub as_of: String,
    /// Exact amount as a decimal string.
    pub amount: String,
    /// Currency code.
    pub currency: String,
    /// `manual`, `import`, or `provider`.
    pub source: String,
}

impl From<Balance> for BalanceDto {
    fn from(balance: Balance) -> Self {
        Self {
            id: balance.id,
            account_id: balance.account_id,
            as_of: balance.as_of.to_string(),
            amount: balance.amount.amount_string(),
            currency: balance.amount.currency().to_string(),
            source: balance.source.as_str().to_string(),
        }
    }
}

/// Request body for recording a balance.
#[derive(Debug, Deserialize)]
pub struct RecordBalanceRequest {
    /// ISO date (`YYYY-MM-DD`).
    pub as_of: String,
    /// Exact amount as a decimal string.
    pub amount: String,
    /// Currency code; defaults to the account's currency.
    #[serde(default)]
    pub currency: Option<String>,
    /// `manual`, `import`, or `provider`; defaults to `manual`.
    #[serde(default)]
    pub source: Option<String>,
}
