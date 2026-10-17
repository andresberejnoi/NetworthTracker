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
    /// The user who owns a private account.
    pub owner_user_id: Option<Uuid>,
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
            owner_user_id: account.owner_user_id,
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
    /// The user who owns a `user`-scoped account.
    #[serde(default)]
    pub owner_user_id: Option<Uuid>,
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

/// A breakdown row in the API.
#[derive(Debug, Serialize)]
pub struct BreakdownRowDto {
    /// Group key, such as an account type or asset class.
    pub key: String,
    /// Exact net amount as a decimal string (liabilities are negative).
    pub amount: String,
    /// Currency code.
    pub currency: String,
}

/// Net worth as returned by the API.
#[derive(Debug, Serialize)]
pub struct NetWorthDto {
    /// ISO date.
    pub as_of: String,
    /// Currency code.
    pub currency: String,
    /// Assets minus liabilities.
    pub total: String,
    /// Liquid assets minus liabilities.
    pub liquid: String,
    /// Net realizable value.
    pub accessible: String,
    /// Net amount by account type.
    pub by_account_type: Vec<BreakdownRowDto>,
    /// Net amount by asset class.
    pub by_asset_class: Vec<BreakdownRowDto>,
    /// Accounts that contributed a balance.
    pub account_count: usize,
    /// Whether any contributing balance was entered by hand.
    pub has_manual_data: bool,
    /// Whether any contributing balance is stale.
    pub has_stale_data: bool,
}

impl From<networth_domain::NetWorthBreakdown> for NetWorthDto {
    fn from(value: networth_domain::NetWorthBreakdown) -> Self {
        let currency = value.currency.to_string();
        let rows = |rows: Vec<networth_domain::BreakdownRow>| {
            rows.into_iter()
                .map(|row| BreakdownRowDto {
                    key: row.key,
                    amount: row.amount.amount_string(),
                    currency: row.amount.currency().to_string(),
                })
                .collect()
        };

        Self {
            as_of: value.as_of.to_string(),
            currency,
            total: value.total.amount_string(),
            liquid: value.liquid.amount_string(),
            accessible: value.accessible.amount_string(),
            by_account_type: rows(value.by_account_type),
            by_asset_class: rows(value.by_asset_class),
            account_count: value.account_count,
            has_manual_data: value.has_manual_data,
            has_stale_data: value.has_stale_data,
        }
    }
}

/// The redemption policy recorded with a snapshot.
#[derive(Debug, Serialize)]
pub struct PolicyDto {
    /// Effective tax rate.
    pub tax_rate: String,
    /// Early-withdrawal penalty rate.
    pub penalty_rate: String,
    /// Age below which the penalty applies.
    pub penalty_age: u32,
    /// Real-estate selling cost rate.
    pub selling_rate: String,
    /// Fixed selling costs.
    pub fixed_costs: String,
}

/// A stored snapshot as returned by the API.
#[derive(Debug, Serialize)]
pub struct SnapshotDto {
    /// Identifier.
    pub id: uuid::Uuid,
    /// ISO date.
    pub as_of: String,
    /// `household` or `personal`.
    pub scope: String,
    /// The user for a personal snapshot.
    pub owner_user_id: Option<uuid::Uuid>,
    /// The age used for penalties.
    pub owner_age: Option<u32>,
    /// Currency code.
    pub currency: String,
    /// Total net worth.
    pub total: String,
    /// Liquid net worth.
    pub liquid: String,
    /// Accessible net worth.
    pub accessible: String,
    /// The policy used.
    pub policy: PolicyDto,
}

impl From<networth_domain::Snapshot> for SnapshotDto {
    fn from(value: networth_domain::Snapshot) -> Self {
        Self {
            id: value.id,
            as_of: value.as_of.to_string(),
            scope: value.scope.as_str().to_string(),
            owner_user_id: value.owner_user_id,
            owner_age: value.owner_age,
            currency: value.currency.to_string(),
            total: value.total.amount_string(),
            liquid: value.liquid.amount_string(),
            accessible: value.accessible.amount_string(),
            policy: PolicyDto {
                tax_rate: value.policy.tax_rate.to_string(),
                penalty_rate: value.policy.early_withdrawal_penalty_rate.to_string(),
                penalty_age: value.policy.early_withdrawal_before_age,
                selling_rate: value.policy.selling_cost_rate.to_string(),
                fixed_costs: value.policy.fixed_costs.to_string(),
            },
        }
    }
}

/// Optional policy overrides, accepted by the net worth and snapshot endpoints.
#[derive(Debug, Default, Deserialize)]
pub struct PolicyParams {
    /// Effective tax rate, as a decimal string.
    #[serde(default)]
    pub tax_rate: Option<String>,
    /// Early-withdrawal penalty rate.
    #[serde(default)]
    pub penalty_rate: Option<String>,
    /// Age below which the penalty applies.
    #[serde(default)]
    pub penalty_age: Option<u32>,
    /// Real-estate selling cost rate.
    #[serde(default)]
    pub selling_rate: Option<String>,
    /// Fixed selling costs.
    #[serde(default)]
    pub fixed_costs: Option<String>,
}

/// Query parameters for `GET /api/v1/networth`.
#[derive(Debug, Deserialize)]
pub struct NetWorthQuery {
    /// ISO date; defaults to today (UTC).
    #[serde(default)]
    pub as_of: Option<String>,
    /// `household` or `personal`.
    #[serde(default)]
    pub scope: Option<String>,
    /// The user for a personal view.
    #[serde(default)]
    pub owner_user_id: Option<uuid::Uuid>,
    /// The owner's age, used for penalties.
    #[serde(default)]
    pub owner_age: Option<u32>,
    /// Policy overrides (also enables what-if scenarios).
    #[serde(flatten)]
    pub policy: PolicyParams,
}

/// Request body for `POST /api/v1/networth/snapshots`.
#[derive(Debug, Deserialize)]
pub struct SnapshotRequest {
    /// ISO date; defaults to today (UTC).
    #[serde(default)]
    pub as_of: Option<String>,
    /// `household` or `personal`.
    #[serde(default)]
    pub scope: Option<String>,
    /// The user for a personal snapshot.
    #[serde(default)]
    pub owner_user_id: Option<uuid::Uuid>,
    /// The owner's age, used for penalties.
    #[serde(default)]
    pub owner_age: Option<u32>,
    /// Policy overrides recorded with the snapshot.
    #[serde(flatten)]
    pub policy: PolicyParams,
}

/// Query parameters for `GET /api/v1/networth/snapshots`.
#[derive(Debug, Deserialize)]
pub struct SnapshotListQuery {
    /// Restrict to `household` or `personal`.
    #[serde(default)]
    pub scope: Option<String>,
    /// Restrict to one owner.
    #[serde(default)]
    pub owner_user_id: Option<uuid::Uuid>,
}
