//! Full JSON backup and restore, including snapshot history.
//!
//! The backup is a portable JSON document with accounts, balances, and
//! snapshots. Restore is idempotent: accounts match by name, balances upsert
//! per day, and identical snapshots are skipped.

use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use std::sync::Arc;

use bigdecimal::BigDecimal;
use networth_domain::{
    allocate_fifo, Account, AccountType, AggregationScope, BalanceSource, BasisQuality,
    BudgetAllocation, BudgetCategory, Chain, CryptoAsset, CryptoBalanceSource, CryptoWalletKind,
    Currency, Disposal, Lot, LotSource, Money, NewAccount, NewBalance, NewBudgetIncome,
    NewBudgetSpending, NewCryptoAsset, NewCryptoBalance, NewCryptoWallet, NewLot, NewSale,
    NewSecurity, NewSnapshot, OwnershipScope, Quantity, RedemptionPolicy, Sale, SaleMethod, Sector,
    Security, SecurityAssetClass, Snapshot, SnapshotFilter,
};
use serde::{Deserialize, Serialize};
use time::{
    format_description::well_known::{Iso8601, Rfc3339},
    Date, OffsetDateTime,
};
use uuid::Uuid;

use crate::ports::{
    AccountRepository, AggregationRepository, BudgetRepository, CryptoRepository,
    IdentityRepository, InvestmentRepository, NetWorthRepository, RepositoryError,
};

/// The current backup format version.
pub const BACKUP_VERSION: &str = "2";
/// The original backup format, still accepted for restore.
pub const V1_BACKUP_VERSION: &str = "1";

/// A complete household backup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Backup {
    /// Format version.
    pub version: String,
    /// When the backup was produced (RFC 3339).
    pub exported_at: String,
    /// Accounts.
    #[serde(default)]
    pub accounts: Vec<BackupAccount>,
    /// Dated balances.
    #[serde(default)]
    pub balances: Vec<BackupBalance>,
    /// Immutable net worth snapshots.
    #[serde(default)]
    pub snapshots: Vec<BackupSnapshot>,
    /// Securities.
    #[serde(default)]
    pub securities: Vec<BackupSecurity>,
    /// Investment positions, lots, and sales.
    #[serde(default)]
    pub holdings: Vec<BackupHolding>,
    /// Crypto assets.
    #[serde(default)]
    pub crypto_assets: Vec<BackupCryptoAsset>,
    /// Crypto wallets with balances, lots, and sales.
    #[serde(default)]
    pub crypto_wallets: Vec<BackupCryptoWallet>,
    /// Annual budget income records.
    #[serde(default)]
    pub budget_incomes: Vec<BackupBudgetIncome>,
    /// Annual spending plans.
    #[serde(default)]
    pub budget_plans: Vec<BackupBudgetPlan>,
    /// Category/month spending records.
    #[serde(default)]
    pub budget_spending: Vec<BackupBudgetSpending>,
    /// Aggregation links and their provider provenance.
    #[serde(default)]
    pub aggregation_links: Vec<BackupAggregationLink>,
}

/// An account in a backup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupAccount {
    /// Name (the stable key).
    pub name: String,
    /// Account type identifier.
    pub account_type: String,
    /// Institution, if any.
    pub institution: Option<String>,
    /// Currency code.
    pub currency: String,
    /// `household` or `user`.
    pub ownership_scope: String,
    /// Whether the account is archived.
    pub archived: bool,
    /// The owner's username for a private account.
    pub owner_username: Option<String>,
}

/// A balance in a backup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupBalance {
    /// The account name.
    pub account_name: String,
    /// ISO date.
    pub as_of: String,
    /// Exact amount as a decimal string.
    pub amount: String,
    /// Currency code.
    pub currency: String,
    /// `manual`, `import`, or `provider`.
    pub source: String,
}

/// A snapshot in a backup, including the policy used.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupSnapshot {
    /// ISO date.
    pub as_of: String,
    /// `household` or `personal`.
    pub scope: String,
    /// The user for a personal snapshot.
    pub owner_user_id: Option<Uuid>,
    /// The age used for penalties.
    pub owner_age: Option<u32>,
    /// Currency code.
    pub currency: String,
    /// Total net worth.
    pub total: String,
    /// Liquid net worth.
    pub liquid: String,
    /// Liquidity (cash on hand).
    pub liquidity: String,
    /// Accessible net worth.
    pub accessible: String,
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

/// A security in a backup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupSecurity {
    /// Ticker symbol (stable key).
    pub symbol: String,
    /// Display name.
    pub name: String,
    /// Asset class identifier.
    pub asset_class: String,
    /// Optional sector identifier.
    pub sector: Option<String>,
    /// Currency code.
    pub currency: String,
}

/// An investment position with its lots and sales.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupHolding {
    /// The owning account name.
    pub account_name: String,
    /// Security ticker symbol.
    pub symbol: String,
    /// Tax lots.
    #[serde(default)]
    pub lots: Vec<BackupLot>,
    /// Sales recorded against this position.
    #[serde(default)]
    pub sales: Vec<BackupSale>,
}

/// A tax lot in a backup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupLot {
    /// ISO acquisition date.
    pub acquired_on: String,
    /// Original quantity as a decimal string.
    pub original_quantity: String,
    /// Remaining quantity as a decimal string.
    pub remaining_quantity: String,
    /// Original cost basis as a decimal string.
    pub original_cost_basis: String,
    /// Remaining cost basis as a decimal string.
    pub remaining_cost_basis: String,
    /// Acquisition fees.
    pub fees: String,
    /// Lot source identifier.
    pub source: String,
    /// `actual` or `estimated`.
    pub basis_quality: String,
    /// Optional basis note.
    pub basis_note: Option<String>,
}

/// A sale in a backup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupSale {
    /// ISO disposal date.
    pub disposed_on: String,
    /// Quantity sold.
    pub quantity: String,
    /// Gross proceeds.
    pub gross_proceeds: String,
    /// Selling fees.
    pub fees: String,
    /// Currency code.
    pub currency: String,
    /// Lot-consumption method.
    pub method: String,
    /// Per-lot disposal rows.
    #[serde(default)]
    pub disposals: Vec<BackupDisposal>,
}

/// One lot consumed by a sale.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupDisposal {
    /// ISO acquisition date of the consumed lot.
    pub acquired_on: String,
    /// Quantity consumed.
    pub quantity: String,
    /// Gross proceeds allocated.
    pub gross_proceeds: String,
    /// Fees allocated.
    pub fees: String,
    /// Cost basis allocated.
    pub cost_basis: String,
    /// `actual` or `estimated`.
    pub basis_quality: String,
}

/// A crypto asset in a backup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupCryptoAsset {
    /// Ticker symbol (stable key).
    pub symbol: String,
    /// Display name.
    pub name: String,
    /// Chain identifier.
    pub chain: String,
    /// Optional token contract.
    pub contract_address: Option<String>,
    /// Base-unit decimals.
    pub decimals: u32,
    /// Currency code.
    pub currency: String,
    /// Optional price-provider coin id.
    pub price_provider_id: Option<String>,
}

/// A crypto wallet with balances, lots, and sales.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupCryptoWallet {
    /// Asset ticker symbol.
    pub asset_symbol: String,
    /// `manual` or `address`.
    pub kind: String,
    /// Chain identifier.
    pub chain: String,
    /// Public address for address wallets.
    pub address: Option<String>,
    /// User-facing label.
    pub label: String,
    /// `household` or `user`.
    pub ownership_scope: String,
    /// Owner username for a private wallet.
    pub owner_username: Option<String>,
    /// Whether the wallet is archived.
    pub archived: bool,
    /// Dated balances.
    #[serde(default)]
    pub balances: Vec<BackupCryptoBalance>,
    /// Tax lots.
    #[serde(default)]
    pub lots: Vec<BackupLot>,
    /// Sales.
    #[serde(default)]
    pub sales: Vec<BackupSale>,
}

/// A crypto balance in a backup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupCryptoBalance {
    /// ISO observation date.
    pub as_of: String,
    /// Exact quantity as a decimal string.
    pub quantity: String,
    /// `manual`, `chain`, or `import`.
    pub source: String,
    /// Provider name, if any.
    pub provider: Option<String>,
    /// RFC 3339 fetch time, if any.
    pub fetched_at: Option<String>,
}

/// Annual budget income in a backup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupBudgetIncome {
    /// Owner username.
    pub owner_username: String,
    /// Year.
    pub year: i32,
    /// Amount.
    pub amount: String,
    /// Currency code.
    pub currency: String,
}

/// A spending plan in a backup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupBudgetPlan {
    /// Owner username.
    pub owner_username: String,
    /// Year.
    pub year: i32,
    /// Currency code.
    pub currency: String,
    /// Category allocations.
    #[serde(default)]
    pub allocations: Vec<BackupBudgetAllocation>,
}

/// One category allocation in a spending plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupBudgetAllocation {
    /// Category identifier.
    pub category: String,
    /// Percentage of annual income.
    pub percentage: String,
}

/// A category/month spending record in a backup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupBudgetSpending {
    /// Owner username.
    pub owner_username: String,
    /// Year.
    pub year: i32,
    /// Month, 1–12.
    pub month: u32,
    /// Category identifier.
    pub category: String,
    /// Amount spent.
    pub amount: String,
    /// Currency code.
    pub currency: String,
    /// Optional note.
    pub note: Option<String>,
}

/// An aggregation link with provider provenance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupAggregationLink {
    /// Provider name.
    pub provider: String,
    /// Provider account id (stable with provider).
    pub provider_account_id: String,
    /// Local account name.
    pub account_name: String,
    /// Display name.
    pub display_name: String,
    /// Local account type identifier.
    pub account_type: String,
    /// Currency code.
    pub currency: String,
    /// RFC 3339 last successful sync, if any.
    pub last_synced_at: Option<String>,
    /// Provider balance observations.
    #[serde(default)]
    pub balances: Vec<BackupAggregationBalance>,
    /// Provider holdings.
    #[serde(default)]
    pub holdings: Vec<BackupAggregationHolding>,
}

/// A provider balance observation in a backup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupAggregationBalance {
    /// ISO observation date.
    pub as_of: String,
    /// Amount.
    pub amount: String,
    /// Currency code.
    pub currency: String,
    /// RFC 3339 sync time.
    pub synced_at: String,
}

/// A provider holding in a backup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupAggregationHolding {
    /// Symbol.
    pub symbol: String,
    /// Quantity.
    pub quantity: String,
    /// Market value.
    pub value: String,
    /// Currency code.
    pub currency: String,
    /// RFC 3339 sync time.
    pub synced_at: String,
}

/// The outcome of a restore.
#[derive(Debug, Default, Clone, Serialize)]
pub struct RestoreReport {
    /// Accounts created (existing names are reused).
    pub accounts_created: usize,
    /// Balances written.
    pub balances_saved: usize,
    /// Snapshots inserted.
    pub snapshots_restored: usize,
    /// Snapshots skipped because an identical one already existed.
    pub snapshots_skipped: usize,
    /// Securities created.
    pub securities_created: usize,
    /// Holdings restored.
    pub holdings_restored: usize,
    /// Holdings skipped because the account/symbol already existed.
    pub holdings_skipped: usize,
    /// Investment lots restored.
    pub lots_restored: usize,
    /// Investment sales restored.
    pub sales_restored: usize,
    /// Crypto assets created.
    pub crypto_assets_created: usize,
    /// Crypto wallets restored.
    pub crypto_wallets_restored: usize,
    /// Crypto wallets skipped because the asset/wallet identity already existed.
    pub crypto_wallets_skipped: usize,
    /// Crypto balances saved.
    pub crypto_balances_saved: usize,
    /// Crypto lots restored.
    pub crypto_lots_restored: usize,
    /// Crypto sales restored.
    pub crypto_sales_restored: usize,
    /// Budget income records saved.
    pub budget_incomes_saved: usize,
    /// Budget plans saved.
    pub budget_plans_saved: usize,
    /// Budget spending records saved.
    pub budget_spending_saved: usize,
    /// Aggregation links saved.
    pub aggregation_links_saved: usize,
}

/// Errors raised by backup/restore.
#[derive(Debug, thiserror::Error)]
pub enum BackupError {
    /// The backup is malformed or references missing users/accounts.
    #[error("{0}")]
    Validation(String),
    /// Persistence failed.
    #[error(transparent)]
    Repository(#[from] RepositoryError),
}

/// Export and restore complete backups.
pub struct BackupService {
    accounts: Arc<dyn AccountRepository>,
    networth: Arc<dyn NetWorthRepository>,
    identity: Arc<dyn IdentityRepository>,
    investments: Arc<dyn InvestmentRepository>,
    crypto: Arc<dyn CryptoRepository>,
    budget: Arc<dyn BudgetRepository>,
    aggregation: Arc<dyn AggregationRepository>,
}

impl BackupService {
    /// Build the service over all portable-data repositories.
    pub fn new(
        accounts: Arc<dyn AccountRepository>,
        networth: Arc<dyn NetWorthRepository>,
        identity: Arc<dyn IdentityRepository>,
        investments: Arc<dyn InvestmentRepository>,
        crypto: Arc<dyn CryptoRepository>,
        budget: Arc<dyn BudgetRepository>,
        aggregation: Arc<dyn AggregationRepository>,
    ) -> Self {
        Self {
            accounts,
            networth,
            identity,
            investments,
            crypto,
            budget,
            aggregation,
        }
    }

    /// Produce a complete backup.
    pub async fn export(&self) -> Result<Backup, BackupError> {
        let accounts = self.accounts.list_accounts(true).await?;
        let mut backup_accounts = Vec::with_capacity(accounts.len());
        let mut balances = Vec::new();

        for account in &accounts {
            let owner_username = match account.owner_user_id {
                Some(owner) => self
                    .identity
                    .find_user(owner)
                    .await?
                    .map(|user| user.username),
                None => None,
            };

            backup_accounts.push(BackupAccount {
                name: account.name.clone(),
                account_type: account.account_type.as_str().to_string(),
                institution: account.institution.clone(),
                currency: account.currency.to_string(),
                ownership_scope: account.ownership_scope.as_str().to_string(),
                archived: account.archived,
                owner_username,
            });

            for balance in self.accounts.list_balances(account.id).await? {
                balances.push(BackupBalance {
                    account_name: account.name.clone(),
                    as_of: balance.as_of.to_string(),
                    amount: balance.amount.amount_string(),
                    currency: balance.amount.currency().to_string(),
                    source: balance.source.as_str().to_string(),
                });
            }
        }

        let snapshots = self
            .networth
            .list_snapshots(&SnapshotFilter::default())
            .await?
            .into_iter()
            .map(|snapshot| BackupSnapshot {
                as_of: snapshot.as_of.to_string(),
                scope: snapshot.scope.as_str().to_string(),
                owner_user_id: snapshot.owner_user_id,
                owner_age: snapshot.owner_age,
                currency: snapshot.currency.to_string(),
                total: snapshot.total.amount_string(),
                liquid: snapshot.liquid.amount_string(),
                liquidity: snapshot.liquidity.amount_string(),
                accessible: snapshot.accessible.amount_string(),
                tax_rate: snapshot.policy.tax_rate.to_string(),
                penalty_rate: snapshot.policy.early_withdrawal_penalty_rate.to_string(),
                penalty_age: snapshot.policy.early_withdrawal_before_age,
                selling_rate: snapshot.policy.selling_cost_rate.to_string(),
                fixed_costs: snapshot.policy.fixed_costs.to_string(),
            })
            .collect();

        let users = self.identity.list_users().await?;
        let users_by_id: HashMap<Uuid, String> = users
            .into_iter()
            .map(|user| (user.id, user.username))
            .collect();
        let accounts_by_id: HashMap<Uuid, String> = accounts
            .iter()
            .map(|account| (account.id, account.name.clone()))
            .collect();

        let securities = self
            .investments
            .list_securities()
            .await?
            .into_iter()
            .map(|security| BackupSecurity {
                symbol: security.symbol,
                name: security.name,
                asset_class: security.asset_class.as_str().to_string(),
                sector: security.sector.map(|sector| sector.as_str().to_string()),
                currency: security.currency.to_string(),
            })
            .collect();

        let mut sales_by_holding: HashMap<Uuid, Vec<BackupSale>> = HashMap::new();
        for account in &accounts {
            for sale in self.investments.list_sales(account.id).await? {
                sales_by_holding
                    .entry(sale.holding_id)
                    .or_default()
                    .push(backup_sale(&sale));
            }
        }

        let holdings = self
            .investments
            .list_all_holdings()
            .await?
            .into_iter()
            .filter_map(|holding| {
                let account_name = accounts_by_id.get(&holding.account_id)?.clone();
                Some(BackupHolding {
                    account_name,
                    symbol: holding.security.symbol,
                    lots: holding.lots.iter().map(backup_lot).collect(),
                    sales: sales_by_holding.remove(&holding.id).unwrap_or_default(),
                })
            })
            .collect();

        let crypto_assets = self
            .crypto
            .list_assets()
            .await?
            .into_iter()
            .map(|asset| BackupCryptoAsset {
                symbol: asset.symbol,
                name: asset.name,
                chain: asset.chain.to_string(),
                contract_address: asset.contract_address,
                decimals: asset.decimals,
                currency: asset.currency.to_string(),
                price_provider_id: asset.price_provider_id,
            })
            .collect();

        let mut crypto_wallets = Vec::new();
        for wallet in self.crypto.list_wallets().await? {
            let owner_username = wallet
                .owner_user_id
                .and_then(|owner| owner_username(&users_by_id, owner));
            let balances = self
                .crypto
                .list_balances(wallet.id)
                .await?
                .into_iter()
                .map(|balance| BackupCryptoBalance {
                    as_of: balance.as_of.to_string(),
                    quantity: balance.quantity.to_string(),
                    source: balance.source.as_str().to_string(),
                    provider: balance.provider,
                    fetched_at: balance.fetched_at.map(format_datetime),
                })
                .collect();
            let lots = self
                .crypto
                .list_crypto_lots(wallet.id)
                .await?
                .iter()
                .map(backup_lot)
                .collect();
            let sales = self
                .crypto
                .list_crypto_sales(wallet.id)
                .await?
                .iter()
                .map(backup_sale)
                .collect();

            crypto_wallets.push(BackupCryptoWallet {
                asset_symbol: wallet.asset.symbol,
                kind: wallet.kind.as_str().to_string(),
                chain: wallet.chain.to_string(),
                address: wallet.address,
                label: wallet.label,
                ownership_scope: wallet.ownership_scope.as_str().to_string(),
                owner_username,
                archived: wallet.archived,
                balances,
                lots,
                sales,
            });
        }

        let budget_incomes = self
            .budget
            .list_incomes()
            .await?
            .into_iter()
            .filter_map(|income| {
                Some(BackupBudgetIncome {
                    owner_username: owner_username(&users_by_id, income.user_id)?,
                    year: income.year,
                    amount: income.amount.amount_string(),
                    currency: income.amount.currency().to_string(),
                })
            })
            .collect();

        let budget_plans = self
            .budget
            .list_plans()
            .await?
            .into_iter()
            .filter_map(|plan| {
                Some(BackupBudgetPlan {
                    owner_username: owner_username(&users_by_id, plan.user_id)?,
                    year: plan.year,
                    currency: plan.currency.to_string(),
                    allocations: plan
                        .allocations
                        .into_iter()
                        .map(|allocation| BackupBudgetAllocation {
                            category: allocation.category.as_str().to_string(),
                            percentage: allocation.percentage.to_string(),
                        })
                        .collect(),
                })
            })
            .collect();

        let budget_spending = self
            .budget
            .list_all_spending()
            .await?
            .into_iter()
            .filter_map(|spending| {
                Some(BackupBudgetSpending {
                    owner_username: owner_username(&users_by_id, spending.user_id)?,
                    year: spending.year,
                    month: spending.month,
                    category: spending.category.as_str().to_string(),
                    amount: spending.amount.amount_string(),
                    currency: spending.amount.currency().to_string(),
                    note: spending.note,
                })
            })
            .collect();

        let mut aggregation_links = Vec::new();
        for link in self.aggregation.links().await? {
            let account_name = accounts_by_id
                .get(&link.account_id)
                .cloned()
                .unwrap_or_default();
            let balances = self
                .aggregation
                .balances(link.id)
                .await?
                .into_iter()
                .map(|balance| BackupAggregationBalance {
                    as_of: balance.as_of.to_string(),
                    amount: balance.amount.amount_string(),
                    currency: balance.amount.currency().to_string(),
                    synced_at: format_datetime(balance.synced_at),
                })
                .collect();
            let holdings = self
                .aggregation
                .holdings(link.id)
                .await?
                .into_iter()
                .map(|holding| BackupAggregationHolding {
                    symbol: holding.symbol,
                    quantity: holding.quantity.to_string(),
                    value: holding.value.amount_string(),
                    currency: holding.value.currency().to_string(),
                    synced_at: format_datetime(holding.synced_at),
                })
                .collect();

            aggregation_links.push(BackupAggregationLink {
                provider: link.provider,
                provider_account_id: link.provider_account_id,
                account_name,
                display_name: link.display_name,
                account_type: link.account_type.as_str().to_string(),
                currency: link.currency.to_string(),
                last_synced_at: link.last_synced_at.map(format_datetime),
                balances,
                holdings,
            });
        }

        Ok(Backup {
            version: BACKUP_VERSION.to_string(),
            exported_at: format_datetime(OffsetDateTime::now_utc()),
            accounts: backup_accounts,
            balances,
            snapshots,
            securities,
            holdings,
            crypto_assets,
            crypto_wallets,
            budget_incomes,
            budget_plans,
            budget_spending,
            aggregation_links,
        })
    }

    /// Restore a backup. Validation failures write nothing.
    pub async fn restore(&self, backup: &Backup) -> Result<RestoreReport, BackupError> {
        if backup.version != V1_BACKUP_VERSION && backup.version != BACKUP_VERSION {
            return Err(BackupError::Validation(format!(
                "unsupported backup version '{}', expected {V1_BACKUP_VERSION} or {BACKUP_VERSION}",
                backup.version
            )));
        }

        let users = self.identity.list_users().await?;
        let mut users_by_name: HashMap<String, Uuid> = users
            .into_iter()
            .map(|user| (user.username.to_ascii_lowercase(), user.id))
            .collect();
        let implicit = crate::auth::implicit_user();
        users_by_name
            .entry(implicit.username.to_ascii_lowercase())
            .or_insert(implicit.id);

        let existing = self.accounts.list_accounts(true).await?;
        let mut known: HashMap<String, Account> = existing
            .into_iter()
            .map(|account| (account.name.to_ascii_lowercase(), account))
            .collect();

        let mut errors = Vec::new();

        // Parse and validate accounts.
        struct ParsedAccount {
            name: String,
            account_type: AccountType,
            institution: Option<String>,
            currency: Currency,
            ownership_scope: OwnershipScope,
            archived: bool,
            owner_user_id: Option<Uuid>,
        }

        let mut parsed_accounts = Vec::new();
        for account in &backup.accounts {
            let result = (|| -> Result<ParsedAccount, String> {
                let account_type = AccountType::from_str(&account.account_type)
                    .map_err(|error| error.to_string())?;
                let currency =
                    Currency::new(&account.currency).map_err(|error| error.to_string())?;
                let ownership_scope = OwnershipScope::from_str(&account.ownership_scope)
                    .map_err(|error| error.to_string())?;

                let owner_user_id = match &account.owner_username {
                    Some(username) => Some(
                        *users_by_name
                            .get(&username.to_ascii_lowercase())
                            .ok_or_else(|| format!("unknown owner username '{username}'"))?,
                    ),
                    None => None,
                };

                Ok(ParsedAccount {
                    name: account.name.clone(),
                    account_type,
                    institution: account.institution.clone(),
                    currency,
                    ownership_scope,
                    archived: account.archived,
                    owner_user_id,
                })
            })();

            match result {
                Ok(parsed) => parsed_accounts.push(parsed),
                Err(message) => errors.push(format!("account '{}': {message}", account.name)),
            }
        }

        let mut account_names: HashSet<String> = known.keys().cloned().collect();
        for account in &parsed_accounts {
            account_names.insert(account.name.to_ascii_lowercase());
        }

        // Parse and validate balances.
        let mut parsed_balances = Vec::new();
        for balance in &backup.balances {
            let result = (|| -> Result<(String, Date, Money, BalanceSource), String> {
                if !account_names.contains(&balance.account_name.to_ascii_lowercase()) {
                    return Err(format!("unknown account '{}'", balance.account_name));
                }
                let as_of = Date::parse(&balance.as_of, &Iso8601::DATE)
                    .map_err(|error| format!("invalid date: {error}"))?;
                let amount = Money::parse(&balance.amount, &balance.currency)
                    .map_err(|error| error.to_string())?;
                let source =
                    BalanceSource::from_str(&balance.source).map_err(|error| error.to_string())?;
                Ok((balance.account_name.clone(), as_of, amount, source))
            })();

            match result {
                Ok(parsed) => parsed_balances.push(parsed),
                Err(message) => {
                    errors.push(format!("balance '{}': {message}", balance.account_name))
                }
            }
        }

        // Parse and validate snapshots.
        let mut parsed_snapshots = Vec::new();
        for snapshot in &backup.snapshots {
            let result = (|| -> Result<NewSnapshot, String> {
                let as_of = Date::parse(&snapshot.as_of, &Iso8601::DATE)
                    .map_err(|error| format!("invalid date: {error}"))?;
                let scope = AggregationScope::from_str(&snapshot.scope)
                    .map_err(|error| error.to_string())?;
                let currency =
                    Currency::new(&snapshot.currency).map_err(|error| error.to_string())?;
                let parse =
                    |value: &str| BigDecimal::from_str(value).map_err(|error| error.to_string());

                Ok(NewSnapshot {
                    as_of,
                    scope,
                    owner_user_id: snapshot.owner_user_id,
                    owner_age: snapshot.owner_age,
                    currency: currency.clone(),
                    total: Money::new(parse(&snapshot.total)?, currency.clone()),
                    liquid: Money::new(parse(&snapshot.liquid)?, currency.clone()),
                    liquidity: Money::new(parse(&snapshot.liquidity)?, currency.clone()),
                    accessible: Money::new(parse(&snapshot.accessible)?, currency),
                    policy: RedemptionPolicy {
                        tax_rate: parse(&snapshot.tax_rate)?,
                        early_withdrawal_penalty_rate: parse(&snapshot.penalty_rate)?,
                        early_withdrawal_before_age: snapshot.penalty_age,
                        selling_cost_rate: parse(&snapshot.selling_rate)?,
                        fixed_costs: parse(&snapshot.fixed_costs)?,
                    },
                })
            })();

            match result {
                Ok(parsed) => parsed_snapshots.push(parsed),
                Err(message) => errors.push(format!("snapshot '{}': {message}", snapshot.as_of)),
            }
        }

        // Parse and validate securities.
        struct ParsedSecurity {
            symbol: String,
            name: String,
            asset_class: SecurityAssetClass,
            sector: Option<Sector>,
            currency: Currency,
        }
        let mut parsed_securities = Vec::new();
        for security in &backup.securities {
            let result = (|| -> Result<ParsedSecurity, String> {
                let asset_class = SecurityAssetClass::from_str(&security.asset_class)
                    .map_err(|error| error.to_string())?;
                let sector = security
                    .sector
                    .as_deref()
                    .map(Sector::from_str)
                    .transpose()
                    .map_err(|error| error.to_string())?;
                let currency =
                    Currency::new(&security.currency).map_err(|error| error.to_string())?;
                Ok(ParsedSecurity {
                    symbol: security.symbol.trim().to_ascii_uppercase(),
                    name: security.name.clone(),
                    asset_class,
                    sector,
                    currency,
                })
            })();

            match result {
                Ok(parsed) => parsed_securities.push(parsed),
                Err(message) => errors.push(format!("security '{}': {message}", security.symbol)),
            }
        }
        let security_currencies: HashMap<String, Currency> = parsed_securities
            .iter()
            .map(|security| (security.symbol.clone(), security.currency.clone()))
            .collect();

        // Parse and validate holdings, lots, and sales.
        struct ParsedHolding {
            account_name: String,
            symbol: String,
            lots: Vec<NewLot>,
            sales: Vec<NewSale>,
        }
        let mut parsed_holdings = Vec::new();
        for holding in &backup.holdings {
            let result = (|| -> Result<ParsedHolding, String> {
                if !account_names.contains(&holding.account_name.to_ascii_lowercase()) {
                    return Err(format!("unknown account '{}'", holding.account_name));
                }
                let symbol = holding.symbol.trim().to_ascii_uppercase();
                let currency = security_currencies
                    .get(&symbol)
                    .ok_or_else(|| format!("unknown security '{symbol}'"))?;
                if holding.lots.is_empty() {
                    return Err("holding has no lots".to_string());
                }
                let lots = holding
                    .lots
                    .iter()
                    .map(|lot| parse_lot(lot, currency))
                    .collect::<Result<Vec<_>, _>>()?;
                let mut sales = holding
                    .sales
                    .iter()
                    .map(|sale| parse_sale(sale, currency))
                    .collect::<Result<Vec<_>, _>>()?;
                sales.sort_by_key(|sale| sale.disposed_on);
                Ok(ParsedHolding {
                    account_name: holding.account_name.clone(),
                    symbol,
                    lots,
                    sales,
                })
            })();

            match result {
                Ok(parsed) => parsed_holdings.push(parsed),
                Err(message) => errors.push(format!(
                    "holding '{}:{}': {message}",
                    holding.account_name, holding.symbol
                )),
            }
        }

        // Parse and validate crypto assets.
        struct ParsedCryptoAsset {
            symbol: String,
            name: String,
            chain: Chain,
            contract_address: Option<String>,
            decimals: u32,
            currency: Currency,
            price_provider_id: Option<String>,
        }
        let mut parsed_crypto_assets = Vec::new();
        for asset in &backup.crypto_assets {
            let result = (|| -> Result<ParsedCryptoAsset, String> {
                let chain = Chain::from_str(&asset.chain).map_err(|error| error.to_string())?;
                let currency = Currency::new(&asset.currency).map_err(|error| error.to_string())?;
                Ok(ParsedCryptoAsset {
                    symbol: asset.symbol.trim().to_ascii_uppercase(),
                    name: asset.name.clone(),
                    chain,
                    contract_address: asset.contract_address.clone(),
                    decimals: asset.decimals,
                    currency,
                    price_provider_id: asset.price_provider_id.clone(),
                })
            })();

            match result {
                Ok(parsed) => parsed_crypto_assets.push(parsed),
                Err(message) => errors.push(format!("crypto asset '{}': {message}", asset.symbol)),
            }
        }
        let crypto_currencies: HashMap<String, Currency> = parsed_crypto_assets
            .iter()
            .map(|asset| (asset.symbol.clone(), asset.currency.clone()))
            .collect();

        // Parse and validate crypto wallets, balances, lots, and sales.
        struct ParsedCryptoBalance {
            as_of: Date,
            quantity: Quantity,
            source: CryptoBalanceSource,
            provider: Option<String>,
            fetched_at: Option<OffsetDateTime>,
        }
        struct ParsedCryptoWallet {
            asset_symbol: String,
            kind: CryptoWalletKind,
            chain: Chain,
            address: Option<String>,
            label: String,
            ownership_scope: OwnershipScope,
            owner_user_id: Option<Uuid>,
            balances: Vec<ParsedCryptoBalance>,
            lots: Vec<NewLot>,
            sales: Vec<NewSale>,
        }
        let mut parsed_crypto_wallets = Vec::new();
        for wallet in &backup.crypto_wallets {
            let result = (|| -> Result<ParsedCryptoWallet, String> {
                let asset_symbol = wallet.asset_symbol.trim().to_ascii_uppercase();
                let currency = crypto_currencies
                    .get(&asset_symbol)
                    .ok_or_else(|| format!("unknown crypto asset '{asset_symbol}'"))?;
                let kind =
                    CryptoWalletKind::from_str(&wallet.kind).map_err(|error| error.to_string())?;
                let chain = Chain::from_str(&wallet.chain).map_err(|error| error.to_string())?;
                let ownership_scope = OwnershipScope::from_str(&wallet.ownership_scope)
                    .map_err(|error| error.to_string())?;
                let owner_user_id = match &wallet.owner_username {
                    Some(username) => Some(
                        *users_by_name
                            .get(&username.to_ascii_lowercase())
                            .ok_or_else(|| format!("unknown owner username '{username}'"))?,
                    ),
                    None => None,
                };

                let mut balances = Vec::new();
                for balance in &wallet.balances {
                    let as_of = Date::parse(&balance.as_of, &Iso8601::DATE)
                        .map_err(|error| error.to_string())?;
                    let quantity =
                        Quantity::parse(&balance.quantity).map_err(|error| error.to_string())?;
                    let source = CryptoBalanceSource::from_str(&balance.source)
                        .map_err(|error| error.to_string())?;
                    balances.push(ParsedCryptoBalance {
                        as_of,
                        quantity,
                        source,
                        provider: balance.provider.clone(),
                        fetched_at: parse_optional_datetime(&balance.fetched_at)?,
                    });
                }

                let lots = wallet
                    .lots
                    .iter()
                    .map(|lot| parse_lot(lot, currency))
                    .collect::<Result<Vec<_>, _>>()?;
                let mut sales = wallet
                    .sales
                    .iter()
                    .map(|sale| parse_sale(sale, currency))
                    .collect::<Result<Vec<_>, _>>()?;
                sales.sort_by_key(|sale| sale.disposed_on);

                Ok(ParsedCryptoWallet {
                    asset_symbol,
                    kind,
                    chain,
                    address: wallet.address.clone(),
                    label: wallet.label.clone(),
                    ownership_scope,
                    owner_user_id,
                    balances,
                    lots,
                    sales,
                })
            })();

            match result {
                Ok(parsed) => parsed_crypto_wallets.push(parsed),
                Err(message) => errors.push(format!(
                    "crypto wallet '{}:{}': {message}",
                    wallet.asset_symbol, wallet.label
                )),
            }
        }

        // Parse and validate budgets.
        struct ParsedBudgetIncome {
            user_id: Uuid,
            year: i32,
            amount: Money,
        }
        let mut parsed_budget_incomes = Vec::new();
        for income in &backup.budget_incomes {
            let result = (|| -> Result<ParsedBudgetIncome, String> {
                let user_id = *users_by_name
                    .get(&income.owner_username.to_ascii_lowercase())
                    .ok_or_else(|| format!("unknown owner '{}'", income.owner_username))?;
                let amount = Money::parse(&income.amount, &income.currency)
                    .map_err(|error| error.to_string())?;
                Ok(ParsedBudgetIncome {
                    user_id,
                    year: income.year,
                    amount,
                })
            })();

            match result {
                Ok(parsed) => parsed_budget_incomes.push(parsed),
                Err(message) => errors.push(format!(
                    "budget income '{}:{}': {message}",
                    income.owner_username, income.year
                )),
            }
        }

        struct ParsedBudgetPlan {
            user_id: Uuid,
            year: i32,
            currency: Currency,
            allocations: Vec<BudgetAllocation>,
        }
        let mut parsed_budget_plans = Vec::new();
        for plan in &backup.budget_plans {
            let result = (|| -> Result<ParsedBudgetPlan, String> {
                let user_id = *users_by_name
                    .get(&plan.owner_username.to_ascii_lowercase())
                    .ok_or_else(|| format!("unknown owner '{}'", plan.owner_username))?;
                let currency = Currency::new(&plan.currency).map_err(|error| error.to_string())?;
                let allocations = plan
                    .allocations
                    .iter()
                    .map(|allocation| {
                        let category = BudgetCategory::from_str(&allocation.category)
                            .map_err(|error| error.to_string())?;
                        let percentage = BigDecimal::from_str(&allocation.percentage)
                            .map_err(|error| error.to_string())?;
                        Ok(BudgetAllocation {
                            category,
                            percentage,
                        })
                    })
                    .collect::<Result<Vec<_>, String>>()?;
                Ok(ParsedBudgetPlan {
                    user_id,
                    year: plan.year,
                    currency,
                    allocations,
                })
            })();

            match result {
                Ok(parsed) => parsed_budget_plans.push(parsed),
                Err(message) => errors.push(format!(
                    "budget plan '{}:{}': {message}",
                    plan.owner_username, plan.year
                )),
            }
        }

        struct ParsedBudgetSpending {
            user_id: Uuid,
            year: i32,
            month: u32,
            category: BudgetCategory,
            amount: Money,
            note: Option<String>,
        }
        let mut parsed_budget_spending = Vec::new();
        for spending in &backup.budget_spending {
            let result = (|| -> Result<ParsedBudgetSpending, String> {
                let user_id = *users_by_name
                    .get(&spending.owner_username.to_ascii_lowercase())
                    .ok_or_else(|| format!("unknown owner '{}'", spending.owner_username))?;
                let category = BudgetCategory::from_str(&spending.category)
                    .map_err(|error| error.to_string())?;
                let amount = Money::parse(&spending.amount, &spending.currency)
                    .map_err(|error| error.to_string())?;
                Ok(ParsedBudgetSpending {
                    user_id,
                    year: spending.year,
                    month: spending.month,
                    category,
                    amount,
                    note: spending.note.clone(),
                })
            })();

            match result {
                Ok(parsed) => parsed_budget_spending.push(parsed),
                Err(message) => errors.push(format!(
                    "budget spending '{}:{}:{}': {message}",
                    spending.owner_username, spending.year, spending.month
                )),
            }
        }

        // Parse and validate aggregation provenance.
        struct ParsedAggregationBalance {
            as_of: Date,
            amount: Money,
            synced_at: OffsetDateTime,
        }
        struct ParsedAggregationHolding {
            symbol: String,
            quantity: Quantity,
            value: Money,
            synced_at: OffsetDateTime,
        }
        struct ParsedAggregationLink {
            provider: String,
            provider_account_id: String,
            account_name: String,
            display_name: String,
            account_type: AccountType,
            currency: Currency,
            last_synced_at: Option<OffsetDateTime>,
            balances: Vec<ParsedAggregationBalance>,
            holdings: Vec<ParsedAggregationHolding>,
        }
        let mut parsed_aggregation_links = Vec::new();
        for link in &backup.aggregation_links {
            let result = (|| -> Result<ParsedAggregationLink, String> {
                if !account_names.contains(&link.account_name.to_ascii_lowercase()) {
                    return Err(format!("unknown account '{}'", link.account_name));
                }
                let account_type =
                    AccountType::from_str(&link.account_type).map_err(|error| error.to_string())?;
                let currency = Currency::new(&link.currency).map_err(|error| error.to_string())?;
                let mut balances = Vec::new();
                for balance in &link.balances {
                    balances.push(ParsedAggregationBalance {
                        as_of: Date::parse(&balance.as_of, &Iso8601::DATE)
                            .map_err(|error| error.to_string())?,
                        amount: Money::parse(&balance.amount, &balance.currency)
                            .map_err(|error| error.to_string())?,
                        synced_at: parse_datetime(&balance.synced_at)?,
                    });
                }
                let mut holdings = Vec::new();
                for holding in &link.holdings {
                    holdings.push(ParsedAggregationHolding {
                        symbol: holding.symbol.clone(),
                        quantity: Quantity::parse(&holding.quantity)
                            .map_err(|error| error.to_string())?,
                        value: Money::parse(&holding.value, &holding.currency)
                            .map_err(|error| error.to_string())?,
                        synced_at: parse_datetime(&holding.synced_at)?,
                    });
                }
                Ok(ParsedAggregationLink {
                    provider: link.provider.clone(),
                    provider_account_id: link.provider_account_id.clone(),
                    account_name: link.account_name.clone(),
                    display_name: link.display_name.clone(),
                    account_type,
                    currency,
                    last_synced_at: parse_optional_datetime(&link.last_synced_at)?,
                    balances,
                    holdings,
                })
            })();

            match result {
                Ok(parsed) => parsed_aggregation_links.push(parsed),
                Err(message) => errors.push(format!(
                    "aggregation link '{}:{}': {message}",
                    link.provider, link.provider_account_id
                )),
            }
        }

        if !errors.is_empty() {
            return Err(BackupError::Validation(errors.join("; ")));
        }

        let mut report = RestoreReport::default();

        for account in parsed_accounts {
            let key = account.name.to_ascii_lowercase();
            if known.contains_key(&key) {
                continue;
            }

            let created = self
                .accounts
                .insert_account(&NewAccount {
                    name: account.name.clone(),
                    account_type: account.account_type,
                    institution: account.institution,
                    currency: account.currency,
                    ownership_scope: account.ownership_scope,
                    owner_user_id: account.owner_user_id,
                })
                .await?;

            if account.archived {
                self.accounts.archive_account(created.id).await?;
            }

            report.accounts_created += 1;
            known.insert(key, created);
        }

        for (account_name, as_of, amount, source) in parsed_balances {
            let Some(account) = known.get(&account_name.to_ascii_lowercase()) else {
                return Err(BackupError::Validation(format!(
                    "unknown account '{account_name}'"
                )));
            };

            self.accounts
                .upsert_balance(&NewBalance {
                    account_id: account.id,
                    as_of,
                    amount,
                    source,
                })
                .await?;
            report.balances_saved += 1;
        }

        let existing_snapshots = self
            .networth
            .list_snapshots(&SnapshotFilter::default())
            .await?;
        let mut seen: HashSet<String> = existing_snapshots.iter().map(snapshot_key).collect();

        for snapshot in parsed_snapshots {
            let key = new_snapshot_key(&snapshot);
            if !seen.insert(key) {
                report.snapshots_skipped += 1;
                continue;
            }

            self.networth.insert_snapshot(&snapshot).await?;
            report.snapshots_restored += 1;
        }

        let mut securities_by_symbol: HashMap<String, Security> = self
            .investments
            .list_securities()
            .await?
            .into_iter()
            .map(|security| (security.symbol.to_ascii_uppercase(), security))
            .collect();

        for security in parsed_securities {
            if securities_by_symbol.contains_key(&security.symbol) {
                continue;
            }
            let created = self
                .investments
                .create_security(&NewSecurity {
                    symbol: security.symbol.clone(),
                    name: security.name,
                    asset_class: security.asset_class,
                    sector: security.sector,
                    currency: security.currency,
                })
                .await?;
            report.securities_created += 1;
            securities_by_symbol.insert(created.symbol.to_ascii_uppercase(), created);
        }

        for holding in parsed_holdings {
            let account = known
                .get(&holding.account_name.to_ascii_lowercase())
                .ok_or_else(|| {
                    BackupError::Validation(format!("unknown account '{}'", holding.account_name))
                })?;
            let security = securities_by_symbol.get(&holding.symbol).ok_or_else(|| {
                BackupError::Validation(format!("unknown security '{}'", holding.symbol))
            })?;

            let existing_holdings = self.investments.list_holdings(account.id).await?;
            if existing_holdings
                .iter()
                .any(|existing| existing.security.id == security.id)
            {
                report.holdings_skipped += 1;
                continue;
            }

            let mut current = self
                .investments
                .append_lots(account.id, security.id, &holding.lots)
                .await?;
            report.holdings_restored += 1;
            report.lots_restored += holding.lots.len();

            for sale in holding.sales {
                let allocations = allocate_fifo(&current.lots, &sale).map_err(|error| {
                    BackupError::Validation(format!(
                        "holding '{}:{}' sale {}: {error}",
                        holding.account_name, holding.symbol, sale.disposed_on
                    ))
                })?;
                self.investments
                    .record_sale(current.id, &sale, &allocations)
                    .await?;
                report.sales_restored += 1;
                current = self
                    .investments
                    .list_holdings(account.id)
                    .await?
                    .into_iter()
                    .find(|position| position.id == current.id)
                    .ok_or_else(|| {
                        BackupError::Validation(format!(
                            "holding '{}:{}' disappeared during restore",
                            holding.account_name, holding.symbol
                        ))
                    })?;
            }
        }

        let mut crypto_assets_by_symbol: HashMap<String, CryptoAsset> = self
            .crypto
            .list_assets()
            .await?
            .into_iter()
            .map(|asset| (asset.symbol.to_ascii_uppercase(), asset))
            .collect();

        for asset in parsed_crypto_assets {
            if crypto_assets_by_symbol.contains_key(&asset.symbol) {
                continue;
            }
            let created = self
                .crypto
                .create_asset(&NewCryptoAsset {
                    symbol: asset.symbol.clone(),
                    name: asset.name,
                    chain: asset.chain,
                    contract_address: asset.contract_address,
                    decimals: asset.decimals,
                    currency: asset.currency,
                    price_provider_id: asset.price_provider_id,
                })
                .await?;
            report.crypto_assets_created += 1;
            crypto_assets_by_symbol.insert(created.symbol.to_ascii_uppercase(), created);
        }

        let mut existing_wallets = self.crypto.list_wallets().await?;
        for wallet in parsed_crypto_wallets {
            let asset = crypto_assets_by_symbol
                .get(&wallet.asset_symbol)
                .ok_or_else(|| {
                    BackupError::Validation(format!(
                        "unknown crypto asset '{}'",
                        wallet.asset_symbol
                    ))
                })?
                .clone();

            let already_exists = existing_wallets.iter().any(|existing| {
                if existing.asset.id != asset.id || existing.kind != wallet.kind {
                    return false;
                }
                match wallet.kind {
                    CryptoWalletKind::Manual => existing.label.eq_ignore_ascii_case(&wallet.label),
                    CryptoWalletKind::Address => existing.address == wallet.address,
                }
            });
            if already_exists {
                report.crypto_wallets_skipped += 1;
                continue;
            }

            let created = self
                .crypto
                .create_wallet(&NewCryptoWallet {
                    asset_id: asset.id,
                    kind: wallet.kind,
                    chain: wallet.chain,
                    address: wallet.address,
                    label: wallet.label.clone(),
                    ownership_scope: wallet.ownership_scope,
                    owner_user_id: wallet.owner_user_id,
                })
                .await?;
            report.crypto_wallets_restored += 1;
            existing_wallets.push(created.clone());

            for balance in wallet.balances {
                self.crypto
                    .upsert_crypto_balance(&NewCryptoBalance {
                        wallet_id: created.id,
                        as_of: balance.as_of,
                        quantity: balance.quantity,
                        source: balance.source,
                        provider: balance.provider,
                        fetched_at: balance.fetched_at,
                    })
                    .await?;
                report.crypto_balances_saved += 1;
            }

            if !wallet.lots.is_empty() {
                let mut current_lots = self
                    .crypto
                    .append_crypto_lots(created.id, asset.id, &wallet.lots)
                    .await?;
                report.crypto_lots_restored += wallet.lots.len();

                for sale in wallet.sales {
                    let allocations = allocate_fifo(&current_lots, &sale).map_err(|error| {
                        BackupError::Validation(format!(
                            "crypto wallet '{}:{}' sale {}: {error}",
                            wallet.asset_symbol, wallet.label, sale.disposed_on
                        ))
                    })?;
                    self.crypto
                        .record_crypto_sale(created.id, asset.id, &sale, &allocations)
                        .await?;
                    report.crypto_sales_restored += 1;
                    current_lots = self.crypto.list_crypto_lots(created.id).await?;
                }
            } else if !wallet.sales.is_empty() {
                return Err(BackupError::Validation(format!(
                    "crypto wallet '{}:{}' has sales but no lots",
                    wallet.asset_symbol, wallet.label
                )));
            }
        }

        for income in parsed_budget_incomes {
            self.budget
                .upsert_income(&NewBudgetIncome {
                    user_id: income.user_id,
                    year: income.year,
                    amount: income.amount,
                })
                .await?;
            report.budget_incomes_saved += 1;
        }

        for plan in parsed_budget_plans {
            self.budget
                .upsert_plan(plan.user_id, plan.year, &plan.currency, &plan.allocations)
                .await?;
            report.budget_plans_saved += 1;
        }

        for spending in parsed_budget_spending {
            self.budget
                .upsert_spending(&NewBudgetSpending {
                    user_id: spending.user_id,
                    year: spending.year,
                    month: spending.month,
                    category: spending.category,
                    amount: spending.amount,
                    note: spending.note,
                })
                .await?;
            report.budget_spending_saved += 1;
        }

        for link in parsed_aggregation_links {
            let account = known
                .get(&link.account_name.to_ascii_lowercase())
                .ok_or_else(|| {
                    BackupError::Validation(format!("unknown account '{}'", link.account_name))
                })?;
            let saved = self
                .aggregation
                .upsert_link(
                    &link.provider,
                    &link.provider_account_id,
                    account.id,
                    &link.display_name,
                    link.account_type,
                    &link.currency,
                )
                .await?;
            for balance in link.balances {
                self.aggregation
                    .upsert_balance(saved.id, balance.as_of, &balance.amount, balance.synced_at)
                    .await?;
            }
            for holding in link.holdings {
                self.aggregation
                    .upsert_holding(
                        saved.id,
                        &holding.symbol,
                        &holding.quantity,
                        &holding.value,
                        holding.synced_at,
                    )
                    .await?;
            }
            if let Some(last_synced_at) = link.last_synced_at {
                self.aggregation
                    .touch_link(saved.id, last_synced_at)
                    .await?;
            }
            report.aggregation_links_saved += 1;
        }

        Ok(report)
    }
}

fn backup_lot(lot: &Lot) -> BackupLot {
    BackupLot {
        acquired_on: lot.acquired_on.to_string(),
        original_quantity: lot.original_quantity.to_string(),
        remaining_quantity: lot.remaining_quantity.to_string(),
        original_cost_basis: lot.original_cost_basis.amount_string(),
        remaining_cost_basis: lot.remaining_cost_basis.amount_string(),
        fees: lot.fees.amount_string(),
        source: lot.source.as_str().to_string(),
        basis_quality: lot.basis_quality.as_str().to_string(),
        basis_note: lot.basis_note.clone(),
    }
}

fn backup_sale(sale: &Sale) -> BackupSale {
    BackupSale {
        disposed_on: sale.disposed_on.to_string(),
        quantity: sale.quantity.to_string(),
        gross_proceeds: sale.gross_proceeds.amount_string(),
        fees: sale.fees.amount_string(),
        currency: sale.currency.to_string(),
        method: sale.method.as_str().to_string(),
        disposals: sale.disposals.iter().map(backup_disposal).collect(),
    }
}

fn backup_disposal(disposal: &Disposal) -> BackupDisposal {
    BackupDisposal {
        acquired_on: disposal.acquired_on.to_string(),
        quantity: disposal.quantity.to_string(),
        gross_proceeds: disposal.gross_proceeds.amount_string(),
        fees: disposal.fees.amount_string(),
        cost_basis: disposal.cost_basis.amount_string(),
        basis_quality: disposal.basis_quality.as_str().to_string(),
    }
}

fn owner_username(users_by_id: &HashMap<Uuid, String>, user_id: Uuid) -> Option<String> {
    users_by_id.get(&user_id).cloned().or_else(|| {
        let implicit = crate::auth::implicit_user();
        (implicit.id == user_id).then_some(implicit.username)
    })
}

fn parse_lot(lot: &BackupLot, currency: &Currency) -> Result<NewLot, String> {
    let acquired_on =
        Date::parse(&lot.acquired_on, &Iso8601::DATE).map_err(|error| error.to_string())?;
    let quantity = Quantity::parse(&lot.original_quantity).map_err(|error| error.to_string())?;
    let cost_basis = Money::parse(&lot.original_cost_basis, &currency.to_string())
        .map_err(|error| error.to_string())?;
    let fees = Money::parse(&lot.fees, &currency.to_string()).map_err(|error| error.to_string())?;
    let source = LotSource::from_str(&lot.source).map_err(|error| error.to_string())?;
    let basis_quality =
        BasisQuality::from_str(&lot.basis_quality).map_err(|error| error.to_string())?;

    Ok(NewLot {
        acquired_on,
        quantity,
        cost_basis,
        fees: Some(fees),
        source,
        basis_quality,
        basis_note: lot.basis_note.clone(),
    })
}

fn parse_sale(sale: &BackupSale, currency: &Currency) -> Result<NewSale, String> {
    let disposed_on =
        Date::parse(&sale.disposed_on, &Iso8601::DATE).map_err(|error| error.to_string())?;
    let quantity = Quantity::parse(&sale.quantity).map_err(|error| error.to_string())?;
    let gross_proceeds = Money::parse(&sale.gross_proceeds, &currency.to_string())
        .map_err(|error| error.to_string())?;
    let fees =
        Money::parse(&sale.fees, &currency.to_string()).map_err(|error| error.to_string())?;
    let sale_currency = Currency::new(&sale.currency).map_err(|error| error.to_string())?;
    if &sale_currency != currency {
        return Err(format!(
            "sale currency {} does not match position currency {}",
            sale_currency, currency
        ));
    }
    let method = SaleMethod::from_str(&sale.method).map_err(|error| error.to_string())?;

    Ok(NewSale {
        disposed_on,
        quantity,
        gross_proceeds,
        fees,
        currency: sale_currency,
        method,
    })
}

fn format_datetime(value: OffsetDateTime) -> String {
    value.format(&Rfc3339).unwrap_or_else(|_| value.to_string())
}

fn parse_datetime(value: &str) -> Result<OffsetDateTime, String> {
    OffsetDateTime::parse(value, &Rfc3339).map_err(|error| error.to_string())
}

fn parse_optional_datetime(value: &Option<String>) -> Result<Option<OffsetDateTime>, String> {
    value.as_deref().map(parse_datetime).transpose()
}

fn snapshot_key(snapshot: &Snapshot) -> String {
    format!(
        "{}|{}|{}|{}|{}|{}|{}",
        snapshot.as_of,
        snapshot.scope.as_str(),
        snapshot
            .owner_user_id
            .map(|id| id.to_string())
            .unwrap_or_default(),
        snapshot.total.amount_string(),
        snapshot.liquid.amount_string(),
        snapshot.liquidity.amount_string(),
        snapshot.accessible.amount_string(),
    )
}

fn new_snapshot_key(snapshot: &NewSnapshot) -> String {
    format!(
        "{}|{}|{}|{}|{}|{}|{}",
        snapshot.as_of,
        snapshot.scope.as_str(),
        snapshot
            .owner_user_id
            .map(|id| id.to_string())
            .unwrap_or_default(),
        snapshot.total.amount_string(),
        snapshot.liquid.amount_string(),
        snapshot.liquidity.amount_string(),
        snapshot.accessible.amount_string(),
    )
}
