//! PostgreSQL-backed account and net worth repositories.

use std::str::FromStr;

use async_trait::async_trait;
use bigdecimal::BigDecimal;
use networth_application::ports::{
    AccountRepository, CryptoRepository, FxRepository, IdentityRepository, InvestmentRepository,
    NetWorthRepository, PriceRepository, RepositoryError, StoredPrice, StoredRate, UserWithHash,
};
use networth_domain::{
    Account, AggregationScope, Balance, BalanceSource, CryptoAsset, CryptoBalance, CryptoWallet,
    Currency, Disposal, DisposalAllocation, Holding, Lot, Money, NewAccount, NewBalance,
    NewCryptoAsset, NewCryptoBalance, NewCryptoWallet, NewLot, NewSale, NewSecurity, NewSnapshot,
    NewUser, OwnershipScope, PriceSource, RedemptionPolicy, Sale, Security, Snapshot,
    SnapshotFilter, User, UserRole,
};
use sqlx::PgPool;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

/// Account and net worth persistence backed by PostgreSQL.
#[derive(Clone)]
pub struct PostgresAccountRepository {
    pool: PgPool,
}

impl PostgresAccountRepository {
    /// Build the repository over a connection pool.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

/// Apply the embedded migrations to the given pool.
pub async fn run_migrations(pool: &PgPool) -> Result<(), sqlx::migrate::MigrateError> {
    sqlx::migrate!("../../migrations").run(pool).await
}

const ACCOUNT_COLUMNS: &str =
    "id, name, account_type, institution, currency, ownership_scope, owner_user_id, archived";
// `amount` is read as text so the exact stored scale is preserved (the NUMERIC
// binary decoder can pad the scale, e.g. 250.50 -> 250.5000).
const BALANCE_COLUMNS: &str = "id, account_id, as_of, amount::text AS amount, currency, source";
const SNAPSHOT_COLUMNS: &str = "id, as_of, scope, owner_user_id, owner_age, currency, \
    total::text AS total, liquid::text AS liquid, liquidity::text AS liquidity, \
    accessible::text AS accessible, \
    tax_rate::text AS tax_rate, penalty_rate::text AS penalty_rate, penalty_age, \
    selling_rate::text AS selling_rate, fixed_costs::text AS fixed_costs";

#[derive(sqlx::FromRow)]
struct AccountRow {
    id: Uuid,
    name: String,
    account_type: String,
    institution: Option<String>,
    currency: String,
    ownership_scope: String,
    owner_user_id: Option<Uuid>,
    archived: bool,
}

impl TryFrom<AccountRow> for Account {
    type Error = RepositoryError;

    fn try_from(row: AccountRow) -> Result<Self, Self::Error> {
        Ok(Account {
            id: row.id,
            name: row.name,
            account_type: row.account_type.parse().map_err(
                |e: networth_domain::accounts::AccountTypeParseError| {
                    RepositoryError::Storage(e.to_string())
                },
            )?,
            institution: row.institution,
            currency: Currency::new(row.currency)
                .map_err(|e| RepositoryError::Storage(e.to_string()))?,
            ownership_scope: row.ownership_scope.parse::<OwnershipScope>().map_err(
                |e: networth_domain::accounts::OwnershipScopeParseError| {
                    RepositoryError::Storage(e.to_string())
                },
            )?,
            owner_user_id: row.owner_user_id,
            archived: row.archived,
        })
    }
}

#[derive(sqlx::FromRow)]
struct BalanceRow {
    id: Uuid,
    account_id: Uuid,
    as_of: Date,
    amount: String,
    currency: String,
    source: String,
}

impl TryFrom<BalanceRow> for Balance {
    type Error = RepositoryError;

    fn try_from(row: BalanceRow) -> Result<Self, Self::Error> {
        Ok(Balance {
            id: row.id,
            account_id: row.account_id,
            as_of: row.as_of,
            amount: Money::new(
                parse_decimal(&row.amount)?,
                Currency::new(row.currency).map_err(|e| RepositoryError::Storage(e.to_string()))?,
            ),
            source: row.source.parse::<BalanceSource>().map_err(
                |e: networth_domain::accounts::BalanceSourceParseError| {
                    RepositoryError::Storage(e.to_string())
                },
            )?,
        })
    }
}

#[derive(sqlx::FromRow)]
struct SnapshotRow {
    id: Uuid,
    as_of: Date,
    scope: String,
    owner_user_id: Option<Uuid>,
    owner_age: Option<i32>,
    currency: String,
    total: String,
    liquid: String,
    liquidity: String,
    accessible: String,
    tax_rate: String,
    penalty_rate: String,
    penalty_age: i32,
    selling_rate: String,
    fixed_costs: String,
}

impl TryFrom<SnapshotRow> for Snapshot {
    type Error = RepositoryError;

    fn try_from(row: SnapshotRow) -> Result<Self, Self::Error> {
        let currency = Currency::new(row.currency)
            .map_err(|error| RepositoryError::Storage(error.to_string()))?;

        Ok(Snapshot {
            id: row.id,
            as_of: row.as_of,
            scope: row.scope.parse::<AggregationScope>().map_err(
                |e: networth_domain::networth::AggregationScopeParseError| {
                    RepositoryError::Storage(e.to_string())
                },
            )?,
            owner_user_id: row.owner_user_id,
            owner_age: row.owner_age.map(|age| age as u32),
            currency: currency.clone(),
            total: Money::new(parse_decimal(&row.total)?, currency.clone()),
            liquid: Money::new(parse_decimal(&row.liquid)?, currency.clone()),
            liquidity: Money::new(parse_decimal(&row.liquidity)?, currency.clone()),
            accessible: Money::new(parse_decimal(&row.accessible)?, currency),
            policy: RedemptionPolicy {
                tax_rate: parse_decimal(&row.tax_rate)?,
                early_withdrawal_penalty_rate: parse_decimal(&row.penalty_rate)?,
                early_withdrawal_before_age: row.penalty_age as u32,
                selling_cost_rate: parse_decimal(&row.selling_rate)?,
                fixed_costs: parse_decimal(&row.fixed_costs)?,
            },
        })
    }
}

fn parse_decimal(value: &str) -> Result<BigDecimal, RepositoryError> {
    BigDecimal::from_str(value).map_err(|error| RepositoryError::Storage(error.to_string()))
}

fn upsert_balance_sql() -> String {
    format!(
        "INSERT INTO account_balances (account_id, as_of, amount, currency, source) \
         VALUES ($1, $2, CAST($3 AS numeric), $4, $5) \
         ON CONFLICT (account_id, as_of) DO UPDATE \
         SET amount = EXCLUDED.amount, currency = EXCLUDED.currency, \
             source = EXCLUDED.source, updated_at = now() \
         RETURNING {BALANCE_COLUMNS}"
    )
}

fn map_sqlx_error(error: sqlx::Error) -> RepositoryError {
    if let sqlx::Error::Database(db) = &error {
        if matches!(db.code().as_deref(), Some("23505") | Some("23503")) {
            return RepositoryError::Conflict(db.message().to_string());
        }
    }
    RepositoryError::Storage(error.to_string())
}

#[async_trait]
impl AccountRepository for PostgresAccountRepository {
    async fn insert_account(&self, new: &NewAccount) -> Result<Account, RepositoryError> {
        let sql = format!(
            "INSERT INTO accounts (name, account_type, institution, currency, ownership_scope, owner_user_id) \
             VALUES ($1, $2, $3, $4, $5, $6) RETURNING {ACCOUNT_COLUMNS}"
        );

        let row: AccountRow = sqlx::query_as(&sql)
            .bind(&new.name)
            .bind(new.account_type.as_str())
            .bind(new.institution.as_deref())
            .bind(new.currency.as_str())
            .bind(new.ownership_scope.as_str())
            .bind(new.owner_user_id)
            .fetch_one(&self.pool)
            .await
            .map_err(map_sqlx_error)?;

        row.try_into()
    }

    async fn find_account(&self, id: Uuid) -> Result<Option<Account>, RepositoryError> {
        let sql = format!("SELECT {ACCOUNT_COLUMNS} FROM accounts WHERE id = $1");
        let row: Option<AccountRow> = sqlx::query_as(&sql)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx_error)?;
        row.map(TryInto::try_into).transpose()
    }

    async fn find_account_by_name(&self, name: &str) -> Result<Option<Account>, RepositoryError> {
        let sql = format!(
            "SELECT {ACCOUNT_COLUMNS} FROM accounts \
             WHERE lower(name) = lower($1) ORDER BY created_at LIMIT 1"
        );
        let row: Option<AccountRow> = sqlx::query_as(&sql)
            .bind(name)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx_error)?;
        row.map(TryInto::try_into).transpose()
    }

    async fn list_accounts(&self, include_archived: bool) -> Result<Vec<Account>, RepositoryError> {
        let sql = format!(
            "SELECT {ACCOUNT_COLUMNS} FROM accounts WHERE ($1 OR NOT archived) ORDER BY name"
        );
        let rows: Vec<AccountRow> = sqlx::query_as(&sql)
            .bind(include_archived)
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx_error)?;
        rows.into_iter().map(TryInto::try_into).collect()
    }

    async fn archive_account(&self, id: Uuid) -> Result<bool, RepositoryError> {
        let result =
            sqlx::query("UPDATE accounts SET archived = true, updated_at = now() WHERE id = $1")
                .bind(id)
                .execute(&self.pool)
                .await
                .map_err(map_sqlx_error)?;
        Ok(result.rows_affected() > 0)
    }

    async fn upsert_balance(&self, new: &NewBalance) -> Result<Balance, RepositoryError> {
        let row: BalanceRow = sqlx::query_as(&upsert_balance_sql())
            .bind(new.account_id)
            .bind(new.as_of)
            .bind(new.amount.amount_string())
            .bind(new.amount.currency().as_str())
            .bind(new.source.as_str())
            .fetch_one(&self.pool)
            .await
            .map_err(map_sqlx_error)?;

        row.try_into()
    }

    async fn upsert_balances(
        &self,
        balances: &[NewBalance],
    ) -> Result<Vec<Balance>, RepositoryError> {
        let sql = upsert_balance_sql();
        let mut tx = self.pool.begin().await.map_err(map_sqlx_error)?;
        let mut saved = Vec::with_capacity(balances.len());

        for balance in balances {
            let row: BalanceRow = sqlx::query_as(&sql)
                .bind(balance.account_id)
                .bind(balance.as_of)
                .bind(balance.amount.amount_string())
                .bind(balance.amount.currency().as_str())
                .bind(balance.source.as_str())
                .fetch_one(&mut *tx)
                .await
                .map_err(map_sqlx_error)?;
            saved.push(row.try_into()?);
        }

        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(saved)
    }

    async fn list_balances(&self, account_id: Uuid) -> Result<Vec<Balance>, RepositoryError> {
        let sql = format!(
            "SELECT {BALANCE_COLUMNS} FROM account_balances WHERE account_id = $1 \
             ORDER BY as_of DESC"
        );
        let rows: Vec<BalanceRow> = sqlx::query_as(&sql)
            .bind(account_id)
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx_error)?;
        rows.into_iter().map(TryInto::try_into).collect()
    }
}

#[async_trait]
impl NetWorthRepository for PostgresAccountRepository {
    async fn latest_balances(&self, as_of: Date) -> Result<Vec<Balance>, RepositoryError> {
        let sql = format!(
            "SELECT DISTINCT ON (account_id) {BALANCE_COLUMNS} \
             FROM account_balances WHERE as_of <= $1 \
             ORDER BY account_id, as_of DESC"
        );
        let rows: Vec<BalanceRow> = sqlx::query_as(&sql)
            .bind(as_of)
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx_error)?;
        rows.into_iter().map(TryInto::try_into).collect()
    }

    async fn insert_snapshot(&self, new: &NewSnapshot) -> Result<Snapshot, RepositoryError> {
        let sql = format!(
            "INSERT INTO net_worth_snapshots \
             (as_of, scope, owner_user_id, owner_age, currency, total, liquid, liquidity, \
              accessible, tax_rate, penalty_rate, penalty_age, selling_rate, fixed_costs) \
             VALUES ($1, $2, $3, $4, $5, CAST($6 AS numeric), CAST($7 AS numeric), \
                     CAST($8 AS numeric), CAST($9 AS numeric), CAST($10 AS numeric), \
                     CAST($11 AS numeric), $12, CAST($13 AS numeric), CAST($14 AS numeric)) \
             RETURNING {SNAPSHOT_COLUMNS}"
        );

        let row: SnapshotRow = sqlx::query_as(&sql)
            .bind(new.as_of)
            .bind(new.scope.as_str())
            .bind(new.owner_user_id)
            .bind(new.owner_age.map(|age| age as i32))
            .bind(new.currency.as_str())
            .bind(new.total.amount_string())
            .bind(new.liquid.amount_string())
            .bind(new.liquidity.amount_string())
            .bind(new.accessible.amount_string())
            .bind(new.policy.tax_rate.to_string())
            .bind(new.policy.early_withdrawal_penalty_rate.to_string())
            .bind(new.policy.early_withdrawal_before_age as i32)
            .bind(new.policy.selling_cost_rate.to_string())
            .bind(new.policy.fixed_costs.to_string())
            .fetch_one(&self.pool)
            .await
            .map_err(map_sqlx_error)?;

        row.try_into()
    }

    async fn list_snapshots(
        &self,
        filter: &SnapshotFilter,
    ) -> Result<Vec<Snapshot>, RepositoryError> {
        let sql = format!(
            "SELECT {SNAPSHOT_COLUMNS} FROM net_worth_snapshots \
             WHERE ($1::text IS NULL OR scope = $1) \
               AND ($2::uuid IS NULL OR owner_user_id = $2) \
             ORDER BY as_of ASC, created_at ASC"
        );

        let rows: Vec<SnapshotRow> = sqlx::query_as(&sql)
            .bind(filter.scope.map(|scope| scope.as_str()))
            .bind(filter.owner_user_id)
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx_error)?;

        rows.into_iter().map(TryInto::try_into).collect()
    }
}

#[derive(sqlx::FromRow)]
struct UserRow {
    id: Uuid,
    username: String,
    display_name: String,
    role: String,
}

fn parse_role(value: &str) -> Result<UserRole, RepositoryError> {
    value
        .parse::<UserRole>()
        .map_err(|e: networth_domain::identity::UserRoleParseError| {
            RepositoryError::Storage(e.to_string())
        })
}

impl TryFrom<UserRow> for User {
    type Error = RepositoryError;

    fn try_from(row: UserRow) -> Result<Self, Self::Error> {
        Ok(User {
            id: row.id,
            username: row.username,
            display_name: row.display_name,
            role: parse_role(&row.role)?,
        })
    }
}

#[derive(sqlx::FromRow)]
struct UserWithHashRow {
    id: Uuid,
    username: String,
    display_name: String,
    role: String,
    password_hash: String,
}

impl TryFrom<UserWithHashRow> for UserWithHash {
    type Error = RepositoryError;

    fn try_from(row: UserWithHashRow) -> Result<Self, Self::Error> {
        Ok(UserWithHash {
            user: User {
                id: row.id,
                username: row.username,
                display_name: row.display_name,
                role: parse_role(&row.role)?,
            },
            password_hash: row.password_hash,
        })
    }
}

#[derive(sqlx::FromRow)]
struct SessionRow {
    id: Uuid,
    username: String,
    display_name: String,
    role: String,
    expires_at: OffsetDateTime,
}

impl TryFrom<SessionRow> for (User, OffsetDateTime) {
    type Error = RepositoryError;

    fn try_from(row: SessionRow) -> Result<Self, Self::Error> {
        Ok((
            User {
                id: row.id,
                username: row.username,
                display_name: row.display_name,
                role: parse_role(&row.role)?,
            },
            row.expires_at,
        ))
    }
}

#[async_trait]
impl IdentityRepository for PostgresAccountRepository {
    async fn count_users(&self) -> Result<i64, RepositoryError> {
        sqlx::query_scalar("SELECT COUNT(*) FROM users")
            .fetch_one(&self.pool)
            .await
            .map_err(map_sqlx_error)
    }

    async fn create_user(&self, new: &NewUser) -> Result<User, RepositoryError> {
        let row: UserRow = sqlx::query_as(
            "INSERT INTO users (username, display_name, password_hash, role) \
             VALUES ($1, $2, $3, $4) \
             RETURNING id, username, display_name, role",
        )
        .bind(&new.username)
        .bind(&new.display_name)
        .bind(&new.password_hash)
        .bind(new.role.as_str())
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx_error)?;

        row.try_into()
    }

    async fn find_user_by_username(
        &self,
        username: &str,
    ) -> Result<Option<UserWithHash>, RepositoryError> {
        let row: Option<UserWithHashRow> = sqlx::query_as(
            "SELECT id, username, display_name, role, password_hash \
             FROM users WHERE lower(username) = lower($1)",
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error)?;

        row.map(TryInto::try_into).transpose()
    }

    async fn find_user(&self, id: Uuid) -> Result<Option<User>, RepositoryError> {
        let row: Option<UserRow> =
            sqlx::query_as("SELECT id, username, display_name, role FROM users WHERE id = $1")
                .bind(id)
                .fetch_optional(&self.pool)
                .await
                .map_err(map_sqlx_error)?;

        row.map(TryInto::try_into).transpose()
    }

    async fn list_users(&self) -> Result<Vec<User>, RepositoryError> {
        let rows: Vec<UserRow> = sqlx::query_as(
            "SELECT id, username, display_name, role FROM users ORDER BY display_name",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error)?;

        rows.into_iter().map(TryInto::try_into).collect()
    }

    async fn update_user_role(&self, id: Uuid, role: UserRole) -> Result<User, RepositoryError> {
        let row: UserRow = sqlx::query_as(
            "UPDATE users SET role = $2 WHERE id = $1 \
             RETURNING id, username, display_name, role",
        )
        .bind(id)
        .bind(role.as_str())
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx_error)?;

        row.try_into()
    }

    async fn create_session(
        &self,
        user_id: Uuid,
        token_hash: &str,
        expires_at: OffsetDateTime,
    ) -> Result<(), RepositoryError> {
        sqlx::query("INSERT INTO sessions (user_id, token_hash, expires_at) VALUES ($1, $2, $3)")
            .bind(user_id)
            .bind(token_hash)
            .bind(expires_at)
            .execute(&self.pool)
            .await
            .map_err(map_sqlx_error)?;

        Ok(())
    }

    async fn find_session(
        &self,
        token_hash: &str,
    ) -> Result<Option<(User, OffsetDateTime)>, RepositoryError> {
        let row: Option<SessionRow> = sqlx::query_as(
            "SELECT u.id, u.username, u.display_name, u.role, s.expires_at \
             FROM sessions s JOIN users u ON u.id = s.user_id \
             WHERE s.token_hash = $1",
        )
        .bind(token_hash)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error)?;

        row.map(TryInto::try_into).transpose()
    }

    async fn delete_session(&self, token_hash: &str) -> Result<(), RepositoryError> {
        sqlx::query("DELETE FROM sessions WHERE token_hash = $1")
            .bind(token_hash)
            .execute(&self.pool)
            .await
            .map_err(map_sqlx_error)?;
        Ok(())
    }
}

#[derive(sqlx::FromRow)]
struct SecurityRow {
    id: Uuid,
    symbol: String,
    name: String,
    asset_class: String,
    sector: Option<String>,
    currency: String,
}

impl TryFrom<SecurityRow> for Security {
    type Error = RepositoryError;

    fn try_from(row: SecurityRow) -> Result<Self, Self::Error> {
        Ok(Security {
            id: row.id,
            symbol: row.symbol,
            name: row.name,
            asset_class: row.asset_class.parse().map_err(
                |e: networth_domain::investments::SecurityAssetClassParseError| {
                    RepositoryError::Storage(e.to_string())
                },
            )?,
            sector: row
                .sector
                .as_deref()
                .map(|value| value.parse())
                .transpose()
                .map_err(|e: networth_domain::investments::SectorParseError| {
                    RepositoryError::Storage(e.to_string())
                })?,
            currency: Currency::new(row.currency)
                .map_err(|e| RepositoryError::Storage(e.to_string()))?,
        })
    }
}

const SECURITY_COLUMNS: &str = "id, symbol, name, asset_class, sector, currency";
const HOLDING_COLUMNS: &str = "h.id, h.account_id, s.id AS security_id, s.symbol, s.name, \
    s.asset_class, s.sector, s.currency";

#[derive(sqlx::FromRow)]
struct HoldingRow {
    id: Uuid,
    account_id: Uuid,
    security_id: Uuid,
    symbol: String,
    name: String,
    asset_class: String,
    sector: Option<String>,
    currency: String,
}

impl TryFrom<HoldingRow> for (Holding, Uuid) {
    type Error = RepositoryError;

    fn try_from(row: HoldingRow) -> Result<Self, Self::Error> {
        let security = Security {
            id: row.security_id,
            symbol: row.symbol,
            name: row.name,
            asset_class: row.asset_class.parse().map_err(
                |e: networth_domain::investments::SecurityAssetClassParseError| {
                    RepositoryError::Storage(e.to_string())
                },
            )?,
            sector: row
                .sector
                .as_deref()
                .map(|value| value.parse())
                .transpose()
                .map_err(|e: networth_domain::investments::SectorParseError| {
                    RepositoryError::Storage(e.to_string())
                })?,
            currency: Currency::new(row.currency)
                .map_err(|e| RepositoryError::Storage(e.to_string()))?,
        };

        Ok((
            Holding {
                id: row.id,
                account_id: row.account_id,
                security,
                lots: Vec::new(),
            },
            row.security_id,
        ))
    }
}

#[derive(sqlx::FromRow)]
struct LotRow {
    id: Uuid,
    seq: i64,
    acquired_on: Date,
    original_quantity: String,
    remaining_quantity: String,
    original_cost_basis: String,
    remaining_cost_basis: String,
    currency: String,
    fees: String,
    source: String,
    basis_quality: String,
    basis_note: Option<String>,
}

impl TryFrom<LotRow> for Lot {
    type Error = RepositoryError;

    fn try_from(row: LotRow) -> Result<Self, Self::Error> {
        let currency =
            Currency::new(row.currency).map_err(|e| RepositoryError::Storage(e.to_string()))?;
        Ok(Lot {
            id: row.id,
            sequence: row.seq,
            acquired_on: row.acquired_on,
            original_quantity: networth_domain::Quantity::new(parse_decimal(
                &row.original_quantity,
            )?)
            .map_err(|e| RepositoryError::Storage(e.to_string()))?,
            remaining_quantity: networth_domain::Quantity::new(parse_decimal(
                &row.remaining_quantity,
            )?)
            .map_err(|e| RepositoryError::Storage(e.to_string()))?,
            original_cost_basis: Money::new(
                parse_decimal(&row.original_cost_basis)?,
                currency.clone(),
            ),
            remaining_cost_basis: Money::new(
                parse_decimal(&row.remaining_cost_basis)?,
                currency.clone(),
            ),
            fees: Money::new(parse_decimal(&row.fees)?, currency),
            source: row.source.parse().map_err(
                |e: networth_domain::investments::LotSourceParseError| {
                    RepositoryError::Storage(e.to_string())
                },
            )?,
            basis_quality: row.basis_quality.parse().map_err(
                |e: networth_domain::investments::BasisQualityParseError| {
                    RepositoryError::Storage(e.to_string())
                },
            )?,
            basis_note: row.basis_note,
        })
    }
}

#[derive(sqlx::FromRow)]
struct SaleRow {
    id: Uuid,
    account_id: Uuid,
    holding_id: Uuid,
    security_id: Uuid,
    symbol: String,
    name: String,
    asset_class: String,
    sector: Option<String>,
    currency: String,
    disposed_on: Date,
    quantity: String,
    gross_proceeds: String,
    fees: String,
    method: String,
}

#[derive(sqlx::FromRow)]
struct DisposalRow {
    id: Uuid,
    lot_id: Uuid,
    acquired_on: Date,
    quantity: String,
    gross_proceeds: String,
    fees: String,
    cost_basis: String,
    basis_quality: String,
}

#[derive(sqlx::FromRow)]
struct StoredPriceRow {
    price: String,
    currency: String,
    as_of: OffsetDateTime,
    source: String,
}

#[derive(sqlx::FromRow)]
struct StoredRateRow {
    rate: String,
    as_of: OffsetDateTime,
}

async fn load_lots(pool: &PgPool, holding_id: Uuid) -> Result<Vec<Lot>, RepositoryError> {
    let rows: Vec<LotRow> = sqlx::query_as(
        "SELECT id, seq, acquired_on, original_quantity::text AS original_quantity, \
                remaining_quantity::text AS remaining_quantity, \
                original_cost_basis::text AS original_cost_basis, \
                remaining_cost_basis::text AS remaining_cost_basis, \
                currency, fees::text AS fees, source, basis_quality, basis_note \
         FROM tax_lots WHERE holding_id = $1 ORDER BY acquired_on, seq",
    )
    .bind(holding_id)
    .fetch_all(pool)
    .await
    .map_err(map_sqlx_error)?;

    rows.into_iter().map(TryInto::try_into).collect()
}

#[async_trait]
impl InvestmentRepository for PostgresAccountRepository {
    async fn create_security(&self, new: &NewSecurity) -> Result<Security, RepositoryError> {
        let sql = format!(
            "INSERT INTO securities (symbol, name, asset_class, sector, currency) \
             VALUES ($1, $2, $3, $4, $5) RETURNING {SECURITY_COLUMNS}"
        );
        let row: SecurityRow = sqlx::query_as(&sql)
            .bind(&new.symbol)
            .bind(&new.name)
            .bind(new.asset_class.as_str())
            .bind(new.sector.map(|sector| sector.as_str()))
            .bind(new.currency.as_str())
            .fetch_one(&self.pool)
            .await
            .map_err(map_sqlx_error)?;
        row.try_into()
    }

    async fn find_security_by_symbol(
        &self,
        symbol: &str,
    ) -> Result<Option<Security>, RepositoryError> {
        let sql =
            format!("SELECT {SECURITY_COLUMNS} FROM securities WHERE upper(symbol) = upper($1)");
        let row: Option<SecurityRow> = sqlx::query_as(&sql)
            .bind(symbol)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx_error)?;
        row.map(TryInto::try_into).transpose()
    }

    async fn list_securities(&self) -> Result<Vec<Security>, RepositoryError> {
        let sql = format!("SELECT {SECURITY_COLUMNS} FROM securities ORDER BY symbol");
        let rows: Vec<SecurityRow> = sqlx::query_as(&sql)
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx_error)?;
        rows.into_iter().map(TryInto::try_into).collect()
    }

    async fn append_lots(
        &self,
        account_id: Uuid,
        security_id: Uuid,
        lots: &[NewLot],
    ) -> Result<Holding, RepositoryError> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx_error)?;

        let (holding_id,): (Uuid,) = sqlx::query_as(
            "INSERT INTO holdings (account_id, security_id) VALUES ($1, $2) \
             ON CONFLICT (account_id, security_id) \
             DO UPDATE SET account_id = EXCLUDED.account_id RETURNING id",
        )
        .bind(account_id)
        .bind(security_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;

        for lot in lots {
            let fees = lot
                .fees
                .as_ref()
                .map(|fees| fees.amount_string())
                .unwrap_or_else(|| "0".to_string());
            sqlx::query(
                "INSERT INTO tax_lots \
                 (holding_id, acquired_on, original_quantity, remaining_quantity, \
                  original_cost_basis, remaining_cost_basis, currency, fees, source, \
                  basis_quality, basis_note) \
                 VALUES ($1, $2, CAST($3 AS numeric), CAST($3 AS numeric), \
                         CAST($4 AS numeric), CAST($4 AS numeric), $5, CAST($6 AS numeric), \
                         $7, $8, $9)",
            )
            .bind(holding_id)
            .bind(lot.acquired_on)
            .bind(lot.quantity.to_string())
            .bind(lot.cost_basis.amount_string())
            .bind(lot.cost_basis.currency().as_str())
            .bind(fees)
            .bind(lot.source.as_str())
            .bind(lot.basis_quality.as_str())
            .bind(lot.basis_note.as_deref())
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_error)?;
        }

        tx.commit().await.map_err(map_sqlx_error)?;

        let sql = format!(
            "SELECT {HOLDING_COLUMNS} FROM holdings h \
             JOIN securities s ON s.id = h.security_id WHERE h.id = $1"
        );
        let row: HoldingRow = sqlx::query_as(&sql)
            .bind(holding_id)
            .fetch_one(&self.pool)
            .await
            .map_err(map_sqlx_error)?;
        let (mut holding, _) = row.try_into()?;
        holding.lots = load_lots(&self.pool, holding.id).await?;
        Ok(holding)
    }

    async fn list_holdings(&self, account_id: Uuid) -> Result<Vec<Holding>, RepositoryError> {
        let sql = format!(
            "SELECT {HOLDING_COLUMNS} FROM holdings h \
             JOIN securities s ON s.id = h.security_id \
             WHERE h.account_id = $1 ORDER BY s.symbol"
        );
        let rows: Vec<HoldingRow> = sqlx::query_as(&sql)
            .bind(account_id)
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx_error)?;

        let mut holdings = Vec::with_capacity(rows.len());
        for row in rows {
            let (mut holding, _) = row.try_into()?;
            holding.lots = load_lots(&self.pool, holding.id).await?;
            holdings.push(holding);
        }
        Ok(holdings)
    }

    async fn list_all_holdings(&self) -> Result<Vec<Holding>, RepositoryError> {
        let sql = format!(
            "SELECT {HOLDING_COLUMNS} FROM holdings h \
             JOIN securities s ON s.id = h.security_id ORDER BY s.symbol"
        );
        let rows: Vec<HoldingRow> = sqlx::query_as(&sql)
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx_error)?;

        let mut holdings = Vec::with_capacity(rows.len());
        for row in rows {
            let (mut holding, _) = row.try_into()?;
            holding.lots = load_lots(&self.pool, holding.id).await?;
            holdings.push(holding);
        }
        Ok(holdings)
    }

    async fn record_sale(
        &self,
        holding_id: Uuid,
        sale: &NewSale,
        allocations: &[DisposalAllocation],
    ) -> Result<Sale, RepositoryError> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx_error)?;

        let holding: SaleRow = sqlx::query_as(
            "SELECT h.id, h.account_id, h.id AS holding_id, s.id AS security_id, \
                    s.symbol, s.name, s.asset_class, s.sector, s.currency, \
                    DATE '1970-01-01' AS disposed_on, \
                    '0' AS quantity, '0' AS gross_proceeds, '0' AS fees, 'fifo' AS method \
             FROM holdings h JOIN securities s ON s.id = h.security_id \
             WHERE h.id = $1",
        )
        .bind(holding_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;

        let (sale_id,): (Uuid,) = sqlx::query_as(
            "INSERT INTO tax_sales \
             (holding_id, disposed_on, quantity, gross_proceeds, fees, currency, method) \
             VALUES ($1, $2, CAST($3 AS numeric), CAST($4 AS numeric), CAST($5 AS numeric), $6, $7) \
             RETURNING id",
        )
        .bind(holding_id)
        .bind(sale.disposed_on)
        .bind(sale.quantity.to_string())
        .bind(sale.gross_proceeds.amount_string())
        .bind(sale.fees.amount_string())
        .bind(sale.currency.as_str())
        .bind(sale.method.as_str())
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;

        let mut disposals = Vec::with_capacity(allocations.len());
        for allocation in allocations {
            let (disposal_id,): (Uuid,) = sqlx::query_as(
                "INSERT INTO tax_disposals \
                 (sale_id, lot_id, quantity, gross_proceeds, fees, cost_basis) \
                 VALUES ($1, $2, CAST($3 AS numeric), CAST($4 AS numeric), \
                         CAST($5 AS numeric), CAST($6 AS numeric)) RETURNING id",
            )
            .bind(sale_id)
            .bind(allocation.lot_id)
            .bind(allocation.quantity.to_string())
            .bind(allocation.gross_proceeds.amount_string())
            .bind(allocation.fees.amount_string())
            .bind(allocation.cost_basis.amount_string())
            .fetch_one(&mut *tx)
            .await
            .map_err(map_sqlx_error)?;

            let result = sqlx::query(
                "UPDATE tax_lots \
                 SET remaining_quantity = remaining_quantity - CAST($2 AS numeric), \
                     remaining_cost_basis = remaining_cost_basis - CAST($3 AS numeric) \
                 WHERE id = $1 \
                   AND remaining_quantity >= CAST($2 AS numeric) \
                   AND remaining_cost_basis >= CAST($3 AS numeric)",
            )
            .bind(allocation.lot_id)
            .bind(allocation.quantity.to_string())
            .bind(allocation.cost_basis.amount_string())
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_error)?;

            if result.rows_affected() != 1 {
                return Err(RepositoryError::Conflict(
                    "a lot changed while the sale was being recorded".to_string(),
                ));
            }

            disposals.push(Disposal {
                id: disposal_id,
                lot_id: allocation.lot_id,
                acquired_on: allocation.acquired_on,
                quantity: allocation.quantity.clone(),
                gross_proceeds: allocation.gross_proceeds.clone(),
                fees: allocation.fees.clone(),
                cost_basis: allocation.cost_basis.clone(),
                basis_quality: allocation.basis_quality,
            });
        }

        tx.commit().await.map_err(map_sqlx_error)?;

        Ok(Sale {
            id: sale_id,
            account_id: holding.account_id,
            holding_id,
            security: Security {
                id: holding.security_id,
                symbol: holding.symbol,
                name: holding.name,
                asset_class: holding.asset_class.parse().map_err(
                    |e: networth_domain::investments::SecurityAssetClassParseError| {
                        RepositoryError::Storage(e.to_string())
                    },
                )?,
                sector: holding
                    .sector
                    .as_deref()
                    .map(|value| value.parse())
                    .transpose()
                    .map_err(|e: networth_domain::investments::SectorParseError| {
                        RepositoryError::Storage(e.to_string())
                    })?,
                currency: Currency::new(holding.currency)
                    .map_err(|e| RepositoryError::Storage(e.to_string()))?,
            },
            disposed_on: sale.disposed_on,
            quantity: sale.quantity.clone(),
            gross_proceeds: sale.gross_proceeds.clone(),
            fees: sale.fees.clone(),
            currency: sale.currency.clone(),
            method: sale.method,
            disposals,
        })
    }

    async fn list_sales(&self, account_id: Uuid) -> Result<Vec<Sale>, RepositoryError> {
        let rows: Vec<SaleRow> = sqlx::query_as(
            "SELECT s.id, h.account_id, s.holding_id, sec.id AS security_id, \
                    sec.symbol, sec.name, sec.asset_class, sec.sector, s.currency, \
                    s.disposed_on, \
                    s.quantity::text AS quantity, s.gross_proceeds::text AS gross_proceeds, \
                    s.fees::text AS fees, s.method \
             FROM tax_sales s \
             JOIN holdings h ON h.id = s.holding_id \
             JOIN securities sec ON sec.id = h.security_id \
             WHERE h.account_id = $1 ORDER BY s.disposed_on DESC, s.id DESC",
        )
        .bind(account_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error)?;

        let mut sales = Vec::with_capacity(rows.len());
        for row in rows {
            let disposals: Vec<DisposalRow> = sqlx::query_as(
                "SELECT d.id, d.lot_id, l.acquired_on, d.quantity::text AS quantity, \
                        d.gross_proceeds::text AS gross_proceeds, d.fees::text AS fees, \
                        d.cost_basis::text AS cost_basis, l.basis_quality \
                 FROM tax_disposals d JOIN tax_lots l ON l.id = d.lot_id \
                 WHERE d.sale_id = $1 ORDER BY l.acquired_on, l.seq",
            )
            .bind(row.id)
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx_error)?;

            sales.push(Sale {
                id: row.id,
                account_id: row.account_id,
                holding_id: row.holding_id,
                security: Security {
                    id: row.security_id,
                    symbol: row.symbol,
                    name: row.name,
                    asset_class: row.asset_class.parse().map_err(
                        |e: networth_domain::investments::SecurityAssetClassParseError| {
                            RepositoryError::Storage(e.to_string())
                        },
                    )?,
                    sector: row
                        .sector
                        .as_deref()
                        .map(|value| value.parse())
                        .transpose()
                        .map_err(|e: networth_domain::investments::SectorParseError| {
                            RepositoryError::Storage(e.to_string())
                        })?,
                    currency: Currency::new(row.currency.clone())
                        .map_err(|e| RepositoryError::Storage(e.to_string()))?,
                },
                disposed_on: row.disposed_on,
                quantity: networth_domain::Quantity::new(parse_decimal(&row.quantity)?)
                    .map_err(|e| RepositoryError::Storage(e.to_string()))?,
                gross_proceeds: Money::new(
                    parse_decimal(&row.gross_proceeds)?,
                    Currency::new(row.currency.clone())
                        .map_err(|e| RepositoryError::Storage(e.to_string()))?,
                ),
                fees: Money::new(
                    parse_decimal(&row.fees)?,
                    Currency::new(row.currency.clone())
                        .map_err(|e| RepositoryError::Storage(e.to_string()))?,
                ),
                currency: Currency::new(row.currency.clone())
                    .map_err(|e| RepositoryError::Storage(e.to_string()))?,
                method: row.method.parse().map_err(
                    |e: networth_domain::investments::SaleMethodParseError| {
                        RepositoryError::Storage(e.to_string())
                    },
                )?,
                disposals: disposals
                    .into_iter()
                    .map(|disposal| {
                        Ok(Disposal {
                            id: disposal.id,
                            lot_id: disposal.lot_id,
                            acquired_on: disposal.acquired_on,
                            quantity: networth_domain::Quantity::new(parse_decimal(
                                &disposal.quantity,
                            )?)
                            .map_err(|e| RepositoryError::Storage(e.to_string()))?,
                            gross_proceeds: Money::new(
                                parse_decimal(&disposal.gross_proceeds)?,
                                Currency::new(row.currency.clone())
                                    .map_err(|e| RepositoryError::Storage(e.to_string()))?,
                            ),
                            fees: Money::new(
                                parse_decimal(&disposal.fees)?,
                                Currency::new(row.currency.clone())
                                    .map_err(|e| RepositoryError::Storage(e.to_string()))?,
                            ),
                            cost_basis: Money::new(
                                parse_decimal(&disposal.cost_basis)?,
                                Currency::new(row.currency.clone())
                                    .map_err(|e| RepositoryError::Storage(e.to_string()))?,
                            ),
                            basis_quality: disposal.basis_quality.parse().map_err(
                                |e: networth_domain::investments::BasisQualityParseError| {
                                    RepositoryError::Storage(e.to_string())
                                },
                            )?,
                        })
                    })
                    .collect::<Result<Vec<_>, RepositoryError>>()?,
            });
        }

        Ok(sales)
    }

    async fn delete_holding(&self, holding_id: Uuid) -> Result<bool, RepositoryError> {
        let (sales,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM tax_sales WHERE holding_id = $1")
                .bind(holding_id)
                .fetch_one(&self.pool)
                .await
                .map_err(map_sqlx_error)?;
        if sales > 0 {
            return Err(RepositoryError::Conflict(
                "cannot delete a position with recorded sales".to_string(),
            ));
        }
        let result = sqlx::query("DELETE FROM holdings WHERE id = $1")
            .bind(holding_id)
            .execute(&self.pool)
            .await
            .map_err(map_sqlx_error)?;
        Ok(result.rows_affected() > 0)
    }
}

const CRYPTO_ASSET_COLUMNS: &str =
    "id, symbol, name, chain, contract_address, decimals, currency, price_provider_id";

#[derive(sqlx::FromRow)]
struct CryptoAssetRow {
    id: Uuid,
    symbol: String,
    name: String,
    chain: String,
    contract_address: Option<String>,
    decimals: i32,
    currency: String,
    price_provider_id: Option<String>,
}

impl TryFrom<CryptoAssetRow> for CryptoAsset {
    type Error = RepositoryError;

    fn try_from(row: CryptoAssetRow) -> Result<Self, Self::Error> {
        Ok(CryptoAsset {
            id: row.id,
            symbol: row.symbol,
            name: row.name,
            chain: row
                .chain
                .parse()
                .map_err(|e: networth_domain::crypto::ChainParseError| {
                    RepositoryError::Storage(e.to_string())
                })?,
            contract_address: row.contract_address,
            decimals: u32::try_from(row.decimals)
                .map_err(|_| RepositoryError::Storage("invalid decimals".to_string()))?,
            currency: Currency::new(row.currency)
                .map_err(|e| RepositoryError::Storage(e.to_string()))?,
            price_provider_id: row.price_provider_id,
        })
    }
}

#[derive(sqlx::FromRow)]
struct CryptoWalletRow {
    id: Uuid,
    kind: String,
    chain: String,
    address: Option<String>,
    label: String,
    ownership_scope: String,
    owner_user_id: Option<Uuid>,
    archived: bool,
    asset_id: Uuid,
    symbol: String,
    name: String,
    contract_address: Option<String>,
    decimals: i32,
    currency: String,
    price_provider_id: Option<String>,
}

impl TryFrom<CryptoWalletRow> for CryptoWallet {
    type Error = RepositoryError;

    fn try_from(row: CryptoWalletRow) -> Result<Self, Self::Error> {
        Ok(CryptoWallet {
            id: row.id,
            asset: CryptoAsset {
                id: row.asset_id,
                symbol: row.symbol,
                name: row.name,
                chain: row.chain.parse().map_err(
                    |e: networth_domain::crypto::ChainParseError| {
                        RepositoryError::Storage(e.to_string())
                    },
                )?,
                contract_address: row.contract_address,
                decimals: u32::try_from(row.decimals)
                    .map_err(|_| RepositoryError::Storage("invalid decimals".to_string()))?,
                currency: Currency::new(row.currency)
                    .map_err(|e| RepositoryError::Storage(e.to_string()))?,
                price_provider_id: row.price_provider_id,
            },
            kind: row.kind.parse().map_err(
                |e: networth_domain::crypto::CryptoWalletKindParseError| {
                    RepositoryError::Storage(e.to_string())
                },
            )?,
            chain: row
                .chain
                .parse()
                .map_err(|e: networth_domain::crypto::ChainParseError| {
                    RepositoryError::Storage(e.to_string())
                })?,
            address: row.address,
            label: row.label,
            ownership_scope: row.ownership_scope.parse().map_err(
                |e: networth_domain::accounts::OwnershipScopeParseError| {
                    RepositoryError::Storage(e.to_string())
                },
            )?,
            owner_user_id: row.owner_user_id,
            archived: row.archived,
        })
    }
}

#[derive(sqlx::FromRow)]
struct CryptoBalanceRow {
    id: Uuid,
    wallet_id: Uuid,
    as_of: Date,
    quantity: String,
    source: String,
    provider: Option<String>,
    fetched_at: Option<OffsetDateTime>,
}

impl TryFrom<CryptoBalanceRow> for CryptoBalance {
    type Error = RepositoryError;

    fn try_from(row: CryptoBalanceRow) -> Result<Self, Self::Error> {
        Ok(CryptoBalance {
            id: row.id,
            wallet_id: row.wallet_id,
            as_of: row.as_of,
            quantity: networth_domain::Quantity::new(parse_decimal(&row.quantity)?)
                .map_err(|e| RepositoryError::Storage(e.to_string()))?,
            source: row.source.parse().map_err(
                |e: networth_domain::crypto::CryptoBalanceSourceParseError| {
                    RepositoryError::Storage(e.to_string())
                },
            )?,
            provider: row.provider,
            fetched_at: row.fetched_at,
        })
    }
}

const CRYPTO_WALLET_COLUMNS: &str = "w.id, w.kind, w.chain, w.address, w.label, \
    w.ownership_scope, w.owner_user_id, w.archived, a.id AS asset_id, a.symbol, a.name, \
    a.contract_address, a.decimals, a.currency, a.price_provider_id";

fn wallet_sql() -> String {
    format!(
        "SELECT {CRYPTO_WALLET_COLUMNS} FROM crypto_wallets w \
         JOIN crypto_assets a ON a.id = w.asset_id"
    )
}

#[async_trait]
impl CryptoRepository for PostgresAccountRepository {
    async fn create_asset(&self, new: &NewCryptoAsset) -> Result<CryptoAsset, RepositoryError> {
        let sql = format!(
            "INSERT INTO crypto_assets \
             (symbol, name, chain, contract_address, decimals, currency, price_provider_id) \
             VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING {CRYPTO_ASSET_COLUMNS}"
        );
        let row: CryptoAssetRow = sqlx::query_as(&sql)
            .bind(&new.symbol)
            .bind(&new.name)
            .bind(new.chain.as_str())
            .bind(new.contract_address.as_deref())
            .bind(i32::try_from(new.decimals).unwrap_or(18))
            .bind(new.currency.as_str())
            .bind(new.price_provider_id.as_deref())
            .fetch_one(&self.pool)
            .await
            .map_err(map_sqlx_error)?;
        row.try_into()
    }

    async fn find_asset_by_symbol(
        &self,
        symbol: &str,
    ) -> Result<Option<CryptoAsset>, RepositoryError> {
        let sql = format!(
            "SELECT {CRYPTO_ASSET_COLUMNS} FROM crypto_assets WHERE upper(symbol) = upper($1)"
        );
        let row: Option<CryptoAssetRow> = sqlx::query_as(&sql)
            .bind(symbol)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx_error)?;
        row.map(TryInto::try_into).transpose()
    }

    async fn list_assets(&self) -> Result<Vec<CryptoAsset>, RepositoryError> {
        let sql = format!("SELECT {CRYPTO_ASSET_COLUMNS} FROM crypto_assets ORDER BY symbol");
        let rows: Vec<CryptoAssetRow> = sqlx::query_as(&sql)
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx_error)?;
        rows.into_iter().map(TryInto::try_into).collect()
    }

    async fn create_wallet(&self, new: &NewCryptoWallet) -> Result<CryptoWallet, RepositoryError> {
        let (wallet_id,): (Uuid,) = sqlx::query_as(
            "INSERT INTO crypto_wallets \
             (asset_id, kind, chain, address, label, ownership_scope, owner_user_id) \
             VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING id",
        )
        .bind(new.asset_id)
        .bind(new.kind.as_str())
        .bind(new.chain.as_str())
        .bind(new.address.as_deref())
        .bind(&new.label)
        .bind(new.ownership_scope.as_str())
        .bind(new.owner_user_id)
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx_error)?;

        self.find_wallet(wallet_id)
            .await?
            .ok_or_else(|| RepositoryError::Storage("wallet not found after insert".to_string()))
    }

    async fn find_wallet(&self, id: Uuid) -> Result<Option<CryptoWallet>, RepositoryError> {
        let sql = format!("{} WHERE w.id = $1", wallet_sql());
        let row: Option<CryptoWalletRow> = sqlx::query_as(&sql)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx_error)?;
        row.map(TryInto::try_into).transpose()
    }

    async fn list_wallets(&self) -> Result<Vec<CryptoWallet>, RepositoryError> {
        let sql = format!("{} ORDER BY w.label, w.id", wallet_sql());
        let rows: Vec<CryptoWalletRow> = sqlx::query_as(&sql)
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx_error)?;
        rows.into_iter().map(TryInto::try_into).collect()
    }

    async fn upsert_crypto_balance(
        &self,
        new: &NewCryptoBalance,
    ) -> Result<CryptoBalance, RepositoryError> {
        let row: CryptoBalanceRow = sqlx::query_as(
            "INSERT INTO crypto_balances \
             (wallet_id, as_of, quantity, source, provider, fetched_at) \
             VALUES ($1, $2, CAST($3 AS numeric), $4, $5, $6) \
             ON CONFLICT (wallet_id, as_of, source) DO UPDATE SET \
               quantity = EXCLUDED.quantity, provider = EXCLUDED.provider, \
               fetched_at = EXCLUDED.fetched_at \
             RETURNING id, wallet_id, as_of, quantity::text AS quantity, source, \
                       provider, fetched_at",
        )
        .bind(new.wallet_id)
        .bind(new.as_of)
        .bind(new.quantity.to_string())
        .bind(new.source.as_str())
        .bind(new.provider.as_deref())
        .bind(new.fetched_at)
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx_error)?;
        row.try_into()
    }

    async fn list_balances(&self, wallet_id: Uuid) -> Result<Vec<CryptoBalance>, RepositoryError> {
        let rows: Vec<CryptoBalanceRow> = sqlx::query_as(
            "SELECT id, wallet_id, as_of, quantity::text AS quantity, source, provider, fetched_at \
             FROM crypto_balances WHERE wallet_id = $1 \
             ORDER BY as_of DESC, CASE source WHEN 'manual' THEN 0 WHEN 'chain' THEN 1 ELSE 2 END",
        )
        .bind(wallet_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error)?;
        rows.into_iter().map(TryInto::try_into).collect()
    }

    async fn latest_balances(&self, as_of: Date) -> Result<Vec<CryptoBalance>, RepositoryError> {
        let rows: Vec<CryptoBalanceRow> = sqlx::query_as(
            "SELECT DISTINCT ON (wallet_id) id, wallet_id, as_of, \
                    quantity::text AS quantity, source, provider, fetched_at \
             FROM crypto_balances WHERE as_of <= $1 \
             ORDER BY wallet_id, as_of DESC, \
                      CASE source WHEN 'manual' THEN 0 WHEN 'chain' THEN 1 ELSE 2 END",
        )
        .bind(as_of)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error)?;
        rows.into_iter().map(TryInto::try_into).collect()
    }
}

#[async_trait]
impl PriceRepository for PostgresAccountRepository {
    async fn insert_price(
        &self,
        symbol: &str,
        price: &Money,
        as_of: OffsetDateTime,
        source: PriceSource,
    ) -> Result<(), RepositoryError> {
        sqlx::query(
            "INSERT INTO security_prices (symbol, price, currency, as_of, source) \
             VALUES ($1, CAST($2 AS numeric), $3, $4, $5)",
        )
        .bind(symbol)
        .bind(price.amount_string())
        .bind(price.currency().as_str())
        .bind(as_of)
        .bind(source.as_str())
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error)?;
        Ok(())
    }

    async fn latest_price(&self, symbol: &str) -> Result<Option<StoredPrice>, RepositoryError> {
        let row: Option<StoredPriceRow> = sqlx::query_as(
            "SELECT price::text AS price, currency, as_of, source \
             FROM security_prices WHERE upper(symbol) = upper($1) \
             ORDER BY as_of DESC LIMIT 1",
        )
        .bind(symbol)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error)?;

        row.map(|row| {
            Ok(StoredPrice {
                price: Money::new(
                    parse_decimal(&row.price)?,
                    Currency::new(row.currency)
                        .map_err(|e| RepositoryError::Storage(e.to_string()))?,
                ),
                as_of: row.as_of,
                source: row.source.parse().map_err(
                    |e: networth_domain::investments::PriceSourceParseError| {
                        RepositoryError::Storage(e.to_string())
                    },
                )?,
            })
        })
        .transpose()
    }
}

#[async_trait]
impl FxRepository for PostgresAccountRepository {
    async fn insert_rate(
        &self,
        base: &Currency,
        quote: &Currency,
        rate: &BigDecimal,
        as_of: OffsetDateTime,
    ) -> Result<(), RepositoryError> {
        sqlx::query(
            "INSERT INTO fx_rates (base_currency, quote_currency, rate, as_of) \
             VALUES ($1, $2, CAST($3 AS numeric), $4)",
        )
        .bind(base.as_str())
        .bind(quote.as_str())
        .bind(rate.to_string())
        .bind(as_of)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error)?;
        Ok(())
    }

    async fn latest_rate(
        &self,
        base: &Currency,
        quote: &Currency,
    ) -> Result<Option<StoredRate>, RepositoryError> {
        let row: Option<StoredRateRow> = sqlx::query_as(
            "SELECT rate::text AS rate, as_of FROM fx_rates \
             WHERE base_currency = $1 AND quote_currency = $2 \
             ORDER BY as_of DESC LIMIT 1",
        )
        .bind(base.as_str())
        .bind(quote.as_str())
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error)?;

        row.map(|row| {
            Ok(StoredRate {
                rate: parse_decimal(&row.rate)?,
                as_of: row.as_of,
            })
        })
        .transpose()
    }
}
