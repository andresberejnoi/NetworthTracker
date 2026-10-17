//! PostgreSQL-backed account repository.

use std::str::FromStr;

use async_trait::async_trait;
use bigdecimal::BigDecimal;
use networth_application::ports::{AccountRepository, RepositoryError};
use networth_domain::{
    Account, Balance, BalanceSource, Currency, Money, NewAccount, NewBalance, OwnershipScope,
};
use sqlx::PgPool;
use time::Date;
use uuid::Uuid;

/// A [`AccountRepository`] backed by PostgreSQL.
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
    "id, name, account_type, institution, currency, ownership_scope, archived";
// `amount` is read as text so the exact stored scale is preserved (the NUMERIC
// binary decoder can pad the scale, e.g. 250.50 -> 250.5000).
const BALANCE_COLUMNS: &str = "id, account_id, as_of, amount::text AS amount, currency, source";

#[derive(sqlx::FromRow)]
struct AccountRow {
    id: Uuid,
    name: String,
    account_type: String,
    institution: Option<String>,
    currency: String,
    ownership_scope: String,
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
        let amount = BigDecimal::from_str(&row.amount)
            .map_err(|e| RepositoryError::Storage(e.to_string()))?;
        Ok(Balance {
            id: row.id,
            account_id: row.account_id,
            as_of: row.as_of,
            amount: Money::new(
                amount,
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

fn map_sqlx_error(error: sqlx::Error) -> RepositoryError {
    if let sqlx::Error::Database(db) = &error {
        if db.code().as_deref() == Some("23505") {
            return RepositoryError::Conflict(db.message().to_string());
        }
        if db.code().as_deref() == Some("23503") {
            return RepositoryError::Conflict(db.message().to_string());
        }
    }
    RepositoryError::Storage(error.to_string())
}

#[async_trait]
impl AccountRepository for PostgresAccountRepository {
    async fn insert_account(&self, new: &NewAccount) -> Result<Account, RepositoryError> {
        let sql = format!(
            "INSERT INTO accounts (name, account_type, institution, currency, ownership_scope) \
             VALUES ($1, $2, $3, $4, $5) RETURNING {ACCOUNT_COLUMNS}"
        );

        let row: AccountRow = sqlx::query_as(&sql)
            .bind(&new.name)
            .bind(new.account_type.as_str())
            .bind(new.institution.as_deref())
            .bind(new.currency.as_str())
            .bind(new.ownership_scope.as_str())
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
