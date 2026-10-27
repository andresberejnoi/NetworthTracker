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
    Account, AccountType, AggregationScope, BalanceSource, Currency, Money, NewAccount, NewBalance,
    NewSnapshot, OwnershipScope, RedemptionPolicy, Snapshot, SnapshotFilter,
};
use serde::{Deserialize, Serialize};
use time::{format_description::well_known::Iso8601, Date, OffsetDateTime};
use uuid::Uuid;

use crate::ports::{AccountRepository, IdentityRepository, NetWorthRepository, RepositoryError};

/// The backup format version.
pub const BACKUP_VERSION: &str = "1";

/// A complete household backup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Backup {
    /// Format version.
    pub version: String,
    /// When the backup was produced (RFC 3339).
    pub exported_at: String,
    /// Accounts.
    pub accounts: Vec<BackupAccount>,
    /// Dated balances.
    pub balances: Vec<BackupBalance>,
    /// Immutable net worth snapshots.
    pub snapshots: Vec<BackupSnapshot>,
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
}

impl BackupService {
    /// Build the service over the three repositories.
    pub fn new(
        accounts: Arc<dyn AccountRepository>,
        networth: Arc<dyn NetWorthRepository>,
        identity: Arc<dyn IdentityRepository>,
    ) -> Self {
        Self {
            accounts,
            networth,
            identity,
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

        Ok(Backup {
            version: BACKUP_VERSION.to_string(),
            exported_at: OffsetDateTime::now_utc().to_string(),
            accounts: backup_accounts,
            balances,
            snapshots,
        })
    }

    /// Restore a backup. Validation failures write nothing.
    pub async fn restore(&self, backup: &Backup) -> Result<RestoreReport, BackupError> {
        if backup.version != BACKUP_VERSION {
            return Err(BackupError::Validation(format!(
                "unsupported backup version '{}', expected {BACKUP_VERSION}",
                backup.version
            )));
        }

        let users = self.identity.list_users().await?;
        let users_by_name: HashMap<String, Uuid> = users
            .into_iter()
            .map(|user| (user.username.to_ascii_lowercase(), user.id))
            .collect();

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

        Ok(report)
    }
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
