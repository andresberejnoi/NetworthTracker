//! Outbound ports implemented by adapters.

use async_trait::async_trait;
use bigdecimal::BigDecimal;
use networth_domain::{
    Account, AccountType, Balance, BudgetAllocation, BudgetIncome, BudgetSpending, Chain,
    CryptoAsset, CryptoBalance, CryptoWallet, Currency, DisposalAllocation, Holding, Lot, Money,
    NewAccount, NewBalance, NewBudgetIncome, NewBudgetSpending, NewCryptoAsset, NewCryptoBalance,
    NewCryptoWallet, NewLot, NewSale, NewSecurity, NewSnapshot, NewUser, PriceSource, Quantity,
    Sale, Security, Snapshot, SnapshotFilter, SpendingPlan, User, UserRole,
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

    /// Append purchase lots to a position, creating the position if needed.
    async fn append_lots(
        &self,
        account_id: Uuid,
        security_id: Uuid,
        lots: &[NewLot],
    ) -> Result<Holding, RepositoryError>;

    /// List holdings for one account.
    async fn list_holdings(&self, account_id: Uuid) -> Result<Vec<Holding>, RepositoryError>;

    /// List every holding.
    async fn list_all_holdings(&self) -> Result<Vec<Holding>, RepositoryError>;

    /// Persist a sale and its per-lot disposals, reducing the lots' remaining
    /// quantities and cost bases atomically.
    async fn record_sale(
        &self,
        holding_id: Uuid,
        sale: &NewSale,
        allocations: &[DisposalAllocation],
    ) -> Result<Sale, RepositoryError>;

    /// List sales for an account, newest first.
    async fn list_sales(&self, account_id: Uuid) -> Result<Vec<Sale>, RepositoryError>;

    /// Delete a holding. Implementations must refuse if the holding has sales.
    async fn delete_holding(&self, holding_id: Uuid) -> Result<bool, RepositoryError>;
}

/// Persistence for crypto assets, wallets, and balances.
#[async_trait]
pub trait CryptoRepository: Send + Sync {
    /// Create a crypto asset.
    async fn create_asset(&self, new: &NewCryptoAsset) -> Result<CryptoAsset, RepositoryError>;

    /// Find an asset by symbol (case-insensitive).
    async fn find_asset_by_symbol(
        &self,
        symbol: &str,
    ) -> Result<Option<CryptoAsset>, RepositoryError>;

    /// List assets, ordered by symbol.
    async fn list_assets(&self) -> Result<Vec<CryptoAsset>, RepositoryError>;

    /// Create a wallet.
    async fn create_wallet(&self, new: &NewCryptoWallet) -> Result<CryptoWallet, RepositoryError>;

    /// Find a wallet by id.
    async fn find_wallet(&self, id: Uuid) -> Result<Option<CryptoWallet>, RepositoryError>;

    /// List wallets, ordered by label.
    async fn list_wallets(&self) -> Result<Vec<CryptoWallet>, RepositoryError>;

    /// Insert or replace a dated balance for a wallet/source.
    async fn upsert_crypto_balance(
        &self,
        new: &NewCryptoBalance,
    ) -> Result<CryptoBalance, RepositoryError>;

    /// List a wallet's balances, newest first.
    async fn list_balances(&self, wallet_id: Uuid) -> Result<Vec<CryptoBalance>, RepositoryError>;

    /// Latest balance per wallet on or before `as_of`, preferring manual over
    /// chain over import on the same date.
    async fn latest_balances(&self, as_of: Date) -> Result<Vec<CryptoBalance>, RepositoryError>;

    /// Append crypto purchase lots to the shared tax-lot ledger.
    async fn append_crypto_lots(
        &self,
        wallet_id: Uuid,
        asset_id: Uuid,
        lots: &[NewLot],
    ) -> Result<Vec<Lot>, RepositoryError>;

    /// List crypto lots for a wallet.
    async fn list_crypto_lots(&self, wallet_id: Uuid) -> Result<Vec<Lot>, RepositoryError>;

    /// Record a FIFO crypto sale and its disposals.
    async fn record_crypto_sale(
        &self,
        wallet_id: Uuid,
        asset_id: Uuid,
        sale: &NewSale,
        allocations: &[DisposalAllocation],
    ) -> Result<Sale, RepositoryError>;

    /// List crypto sales for a wallet.
    async fn list_crypto_sales(&self, wallet_id: Uuid) -> Result<Vec<Sale>, RepositoryError>;
}

/// A balance returned by a chain provider.
#[derive(Debug, Clone)]
pub struct ChainBalance {
    /// Exact native quantity.
    pub quantity: Quantity,
    /// When the provider observed it.
    pub as_of: OffsetDateTime,
    /// Provider name.
    pub provider: String,
}

/// A pluggable, user-funded chain data source.
#[async_trait]
pub trait ChainDataProvider: Send + Sync {
    /// Provider name for status reporting.
    fn name(&self) -> &'static str;

    /// Whether this provider handles a chain.
    fn supports(&self, chain: Chain) -> bool;

    /// Fetch the native balance for a public address.
    async fn balance(
        &self,
        chain: Chain,
        address: &str,
    ) -> Result<Option<ChainBalance>, ProviderError>;
}

/// Persistence for income, spending plans, and category spending.
#[async_trait]
pub trait BudgetRepository: Send + Sync {
    /// Insert or replace annual income.
    async fn upsert_income(&self, new: &NewBudgetIncome) -> Result<BudgetIncome, RepositoryError>;

    /// Annual income for a user/year.
    async fn income(
        &self,
        user_id: Uuid,
        year: i32,
    ) -> Result<Option<BudgetIncome>, RepositoryError>;

    /// Every annual income record, for portable backups.
    async fn list_incomes(&self) -> Result<Vec<BudgetIncome>, RepositoryError>;

    /// Insert or replace a spending plan and its allocations.
    async fn upsert_plan(
        &self,
        user_id: Uuid,
        year: i32,
        currency: &Currency,
        allocations: &[BudgetAllocation],
    ) -> Result<SpendingPlan, RepositoryError>;

    /// A user's plan for a year.
    async fn plan(&self, user_id: Uuid, year: i32)
        -> Result<Option<SpendingPlan>, RepositoryError>;

    /// Every spending plan and its allocations, for portable backups.
    async fn list_plans(&self) -> Result<Vec<SpendingPlan>, RepositoryError>;

    /// Insert or replace a category/month spending record.
    async fn upsert_spending(
        &self,
        new: &NewBudgetSpending,
    ) -> Result<BudgetSpending, RepositoryError>;

    /// All spending records for a user/year.
    async fn list_spending(
        &self,
        user_id: Uuid,
        year: i32,
    ) -> Result<Vec<BudgetSpending>, RepositoryError>;

    /// Every spending record, for portable backups.
    async fn list_all_spending(&self) -> Result<Vec<BudgetSpending>, RepositoryError>;
}

/// Stored aggregation configuration.
#[derive(Debug, Clone)]
pub struct AggregationConfig {
    /// Provider name.
    pub provider: String,
    /// Whether sync is enabled.
    pub enabled: bool,
    /// Encrypted token.
    pub token_ciphertext: Option<String>,
    /// Token nonce.
    pub token_nonce: Option<String>,
}

/// A provider account linked to a local account.
#[derive(Debug, Clone)]
pub struct AggregationLink {
    /// Identifier.
    pub id: Uuid,
    /// Provider name.
    pub provider: String,
    /// Provider account id.
    pub provider_account_id: String,
    /// Local account.
    pub account_id: Uuid,
    /// Display name.
    pub display_name: String,
    /// Local account type.
    pub account_type: AccountType,
    /// Currency.
    pub currency: Currency,
    /// Last successful sync.
    pub last_synced_at: Option<OffsetDateTime>,
}

/// A provider balance observation.
#[derive(Debug, Clone)]
pub struct AggregationBalance {
    /// Link.
    pub link_id: Uuid,
    /// Observation date.
    pub as_of: Date,
    /// Amount.
    pub amount: Money,
    /// Sync time.
    pub synced_at: OffsetDateTime,
}

/// A provider holding observation.
#[derive(Debug, Clone)]
pub struct AggregationHolding {
    /// Link.
    pub link_id: Uuid,
    /// Symbol.
    pub symbol: String,
    /// Quantity.
    pub quantity: Quantity,
    /// Value.
    pub value: Money,
    /// Sync time.
    pub synced_at: OffsetDateTime,
}

/// An account returned by an aggregation provider.
#[derive(Debug, Clone)]
pub struct ProviderAccount {
    /// Provider account id.
    pub provider_id: String,
    /// Display name.
    pub name: String,
    /// Suggested local account type.
    pub account_type: AccountType,
    /// Currency.
    pub currency: Currency,
    /// Balance, if provided.
    pub balance: Option<Money>,
    /// Holdings, if provided.
    pub holdings: Vec<ProviderHolding>,
}

/// A holding returned by an aggregation provider.
#[derive(Debug, Clone)]
pub struct ProviderHolding {
    /// Symbol.
    pub symbol: String,
    /// Quantity.
    pub quantity: Quantity,
    /// Market value.
    pub value: Money,
}

/// A provider snapshot.
#[derive(Debug, Clone)]
pub struct ProviderSnapshot {
    /// Accounts.
    pub accounts: Vec<ProviderAccount>,
}

/// A read-only aggregation provider. It cannot initiate writes at the provider.
#[async_trait]
pub trait AggregationProvider: Send + Sync {
    /// Provider name.
    fn name(&self) -> &'static str;

    /// Fetch accounts using the user's token.
    async fn fetch(&self, token: &str) -> Result<ProviderSnapshot, ProviderError>;
}

/// Persistence for aggregation config, links, balances, and holdings.
#[async_trait]
pub trait AggregationRepository: Send + Sync {
    /// Current config.
    async fn config(&self) -> Result<Option<AggregationConfig>, RepositoryError>;

    /// Save config and encrypted token.
    async fn save_config(
        &self,
        provider: &str,
        enabled: bool,
        token_ciphertext: Option<&str>,
        token_nonce: Option<&str>,
    ) -> Result<AggregationConfig, RepositoryError>;

    /// All links.
    async fn links(&self) -> Result<Vec<AggregationLink>, RepositoryError>;

    /// Create or find a link for a provider account.
    async fn upsert_link(
        &self,
        provider: &str,
        provider_account_id: &str,
        account_id: Uuid,
        display_name: &str,
        account_type: AccountType,
        currency: &Currency,
    ) -> Result<AggregationLink, RepositoryError>;

    /// Upsert a provider balance.
    async fn upsert_balance(
        &self,
        link_id: Uuid,
        as_of: Date,
        amount: &Money,
        synced_at: OffsetDateTime,
    ) -> Result<AggregationBalance, RepositoryError>;

    /// Upsert a provider holding.
    async fn upsert_holding(
        &self,
        link_id: Uuid,
        symbol: &str,
        quantity: &Quantity,
        value: &Money,
        synced_at: OffsetDateTime,
    ) -> Result<AggregationHolding, RepositoryError>;

    /// Latest provider balance for a link.
    async fn latest_balance(
        &self,
        link_id: Uuid,
    ) -> Result<Option<AggregationBalance>, RepositoryError>;

    /// Provider holdings for a link.
    async fn holdings(&self, link_id: Uuid) -> Result<Vec<AggregationHolding>, RepositoryError>;

    /// All provider balances for a link, for portable backups.
    async fn balances(&self, link_id: Uuid) -> Result<Vec<AggregationBalance>, RepositoryError>;

    /// Mark a link as synced.
    async fn touch_link(
        &self,
        link_id: Uuid,
        synced_at: OffsetDateTime,
    ) -> Result<(), RepositoryError>;
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
