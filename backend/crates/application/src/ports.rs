//! Outbound ports implemented by adapters.

use async_trait::async_trait;
use bigdecimal::BigDecimal;
use networth_domain::{
    Account, Balance, Currency, Holding, Money, NewAccount, NewBalance, NewLot, NewSecurity,
    NewSnapshot, NewUser, PriceSource, Security, Snapshot, SnapshotFilter, User, UserRole,
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

/// A stored price observation.
#[derive(Debug, Clone)]
pub struct StoredPrice {
    /// Unit price.
    pub price: Money,
    /// When it was observed.
    pub as_of: OffsetDateTime,
    /// Where it came from.
    pub source: PriceSource,
}

/// A stored FX rate.
#[derive(Debug, Clone)]
pub struct StoredRate {
    /// Rate from base to quote.
    pub rate: BigDecimal,
    /// When it was observed.
    pub as_of: OffsetDateTime,
}

/// Persistence for securities, holdings, and tax lots.
#[async_trait]
pub trait InvestmentRepository: Send + Sync {
    /// Insert a security.
    async fn create_security(&self, new: &NewSecurity) -> Result<Security, RepositoryError>;

    /// Find a security by ticker symbol.
    async fn find_security_by_symbol(
        &self,
        symbol: &str,
    ) -> Result<Option<Security>, RepositoryError>;

    /// List securities, ordered by symbol.
    async fn list_securities(&self) -> Result<Vec<Security>, RepositoryError>;

    /// Create or replace a holding and its lots for an account/security pair.
    async fn upsert_holding(
        &self,
        account_id: Uuid,
        security_id: Uuid,
        lots: &[NewLot],
    ) -> Result<Holding, RepositoryError>;

    /// List holdings for one account.
    async fn list_holdings(&self, account_id: Uuid) -> Result<Vec<Holding>, RepositoryError>;

    /// List every holding.
    async fn list_all_holdings(&self) -> Result<Vec<Holding>, RepositoryError>;

    /// Delete a holding. Returns `false` if it does not exist.
    async fn delete_holding(&self, holding_id: Uuid) -> Result<bool, RepositoryError>;
}

/// Persistence for cached and manual prices.
#[async_trait]
pub trait PriceRepository: Send + Sync {
    /// Append a price observation.
    async fn insert_price(
        &self,
        symbol: &str,
        price: &Money,
        as_of: OffsetDateTime,
        source: PriceSource,
    ) -> Result<(), RepositoryError>;

    /// The latest price for a symbol.
    async fn latest_price(&self, symbol: &str) -> Result<Option<StoredPrice>, RepositoryError>;
}

/// Persistence for foreign-exchange rates.
#[async_trait]
pub trait FxRepository: Send + Sync {
    /// Append an FX rate.
    async fn insert_rate(
        &self,
        base: &Currency,
        quote: &Currency,
        rate: &BigDecimal,
        as_of: OffsetDateTime,
    ) -> Result<(), RepositoryError>;

    /// The latest rate for a currency pair.
    async fn latest_rate(
        &self,
        base: &Currency,
        quote: &Currency,
    ) -> Result<Option<StoredRate>, RepositoryError>;
}

/// A quote returned by a price provider.
#[derive(Debug, Clone)]
pub struct ProviderQuote {
    /// Unit price.
    pub price: Money,
    /// When the provider observed it.
    pub as_of: OffsetDateTime,
}

/// Errors raised by a price provider.
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct ProviderError(pub String);

/// A pluggable, user-funded price source.
#[async_trait]
pub trait PriceProvider: Send + Sync {
    /// Provider name for status reporting.
    fn name(&self) -> &'static str;

    /// Fetch a quote, or `None` if the provider does not know the symbol.
    async fn quote(&self, symbol: &str) -> Result<Option<ProviderQuote>, ProviderError>;
}
