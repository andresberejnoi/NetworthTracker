//! Outbound ports implemented by adapters.

use async_trait::async_trait;
use networth_domain::{
    Account, Balance, NewAccount, NewBalance, NewSnapshot, NewUser, Snapshot, SnapshotFilter, User,
    UserRole,
};
use time::{Date, OffsetDateTime};
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

    /// Find an account by name (case-insensitive).
    async fn find_account_by_name(&self, name: &str) -> Result<Option<Account>, RepositoryError>;

    /// List accounts, optionally including archived ones, ordered by name.
    async fn list_accounts(&self, include_archived: bool) -> Result<Vec<Account>, RepositoryError>;

    /// Archive an account. Returns `false` if it does not exist.
    async fn archive_account(&self, id: Uuid) -> Result<bool, RepositoryError>;

    /// Insert or replace the balance for an account and date.
    async fn upsert_balance(&self, new: &NewBalance) -> Result<Balance, RepositoryError>;

    /// Insert or replace several balances in one operation.
    ///
    /// Implementations should apply the batch atomically where the store
    /// supports it, so a failure does not leave a half-written batch.
    async fn upsert_balances(
        &self,
        balances: &[NewBalance],
    ) -> Result<Vec<Balance>, RepositoryError>;

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

/// A user together with the stored password hash.
#[derive(Debug, Clone)]
pub struct UserWithHash {
    /// The user.
    pub user: User,
    /// Argon2 hash.
    pub password_hash: String,
}

/// Persistence for users and sessions.
#[async_trait]
pub trait IdentityRepository: Send + Sync {
    /// Number of users (used to detect first-run setup).
    async fn count_users(&self) -> Result<i64, RepositoryError>;

    /// Insert a user.
    async fn create_user(&self, new: &NewUser) -> Result<User, RepositoryError>;

    /// Find a user by username (case-insensitive), with the password hash.
    async fn find_user_by_username(
        &self,
        username: &str,
    ) -> Result<Option<UserWithHash>, RepositoryError>;

    /// Find a user by id.
    async fn find_user(&self, id: Uuid) -> Result<Option<User>, RepositoryError>;

    /// List all users, ordered by display name.
    async fn list_users(&self) -> Result<Vec<User>, RepositoryError>;

    /// Change a user's role.
    async fn update_user_role(&self, id: Uuid, role: UserRole) -> Result<User, RepositoryError>;

    /// Store a session keyed by the token hash.
    async fn create_session(
        &self,
        user_id: Uuid,
        token_hash: &str,
        expires_at: OffsetDateTime,
    ) -> Result<(), RepositoryError>;

    /// Look up a session, returning the user and expiry.
    async fn find_session(
        &self,
        token_hash: &str,
    ) -> Result<Option<(User, OffsetDateTime)>, RepositoryError>;

    /// Delete a session.
    async fn delete_session(&self, token_hash: &str) -> Result<(), RepositoryError>;
}
