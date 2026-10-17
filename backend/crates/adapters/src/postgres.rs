//! PostgreSQL-backed account and net worth repositories.

use std::str::FromStr;

use async_trait::async_trait;
use bigdecimal::BigDecimal;
use networth_application::ports::{AccountRepository, NetWorthRepository, RepositoryError};
use networth_domain::{
    Account, AggregationScope, Balance, BalanceSource, Currency, Money, NewAccount, NewBalance,
    NewSnapshot, OwnershipScope, RedemptionPolicy, Snapshot, SnapshotFilter,
};
use sqlx::PgPool;
use time::Date;
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
    total::text AS total, liquid::text AS liquid, accessible::text AS accessible, \
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
        let sql = format!(
            "INSERT INTO account_balances (account_id, as_of, amount, currency, source) \
             VALUES ($1, $2, CAST($3 AS numeric), $4, $5) \
             ON CONFLICT (account_id, as_of) DO UPDATE \
             SET amount = EXCLUDED.amount, currency = EXCLUDED.currency, \
                 source = EXCLUDED.source, updated_at = now() \
             RETURNING {BALANCE_COLUMNS}"
        );

        let row: BalanceRow = sqlx::query_as(&sql)
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
             (as_of, scope, owner_user_id, owner_age, currency, total, liquid, accessible, \
              tax_rate, penalty_rate, penalty_age, selling_rate, fixed_costs) \
             VALUES ($1, $2, $3, $4, $5, CAST($6 AS numeric), CAST($7 AS numeric), \
                     CAST($8 AS numeric), CAST($9 AS numeric), CAST($10 AS numeric), $11, \
                     CAST($12 AS numeric), CAST($13 AS numeric)) \
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
