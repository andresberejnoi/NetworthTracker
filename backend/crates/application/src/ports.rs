//! Outbound ports implemented by adapters.

use async_trait::async_trait;
use networth_domain::{
    Account, Balance, NewAccount, NewBalance, NewSnapshot, Snapshot, SnapshotFilter,
};
use time::Date;
use uuid::Uuid;

/// Errors a repository can raise.
#[derive(Debug, thiserror::Error)]
pub enum RepositoryError {
    /// A uniqueness or integrity constraint was violated.
    #[error("conflict: {0}")]
    Conflict(String),
    /// The underlying store failed.
    #[error("storage error: {0}")]
    Storage(String),
}

/// Persistence for accounts and their dated balances.
#[async_trait]
pub trait AccountRepository: Send + Sync {
    /// Insert a new account.
    async fn insert_account(&self, new: &NewAccount) -> Result<Account, RepositoryError>;

    /// Find an account by id.
    async fn find_account(&self, id: Uuid) -> Result<Option<Account>, RepositoryError>;

    /// List accounts, optionally including archived ones, ordered by name.
    async fn list_accounts(&self, include_archived: bool) -> Result<Vec<Account>, RepositoryError>;

    /// Archive an account. Returns `false` if it does not exist.
    async fn archive_account(&self, id: Uuid) -> Result<bool, RepositoryError>;

    /// Insert or replace the balance for an account and date.
    async fn upsert_balance(&self, new: &NewBalance) -> Result<Balance, RepositoryError>;

    /// List an account's balances, newest first.
    async fn list_balances(&self, account_id: Uuid) -> Result<Vec<Balance>, RepositoryError>;
}

/// Persistence for net worth reads and snapshots.
#[async_trait]
pub trait NetWorthRepository: Send + Sync {
    /// The latest balance on or before `as_of` for every account that has one.
    async fn latest_balances(&self, as_of: Date) -> Result<Vec<Balance>, RepositoryError>;

    /// Persist a new immutable snapshot.
    async fn insert_snapshot(&self, new: &NewSnapshot) -> Result<Snapshot, RepositoryError>;

    /// List snapshots, oldest first, optionally filtered.
    async fn list_snapshots(
        &self,
        filter: &SnapshotFilter,
    ) -> Result<Vec<Snapshot>, RepositoryError>;
}
