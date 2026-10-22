//! Versioned spreadsheet import and export.
//!
//! The file schema is a single table with a `record_type` column so accounts
//! and balances can travel together. Imports are validated in full before
//! anything is written, and re-importing the same file is idempotent: accounts
//! are matched by name and balances upsert per account/day.

use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;

use networth_domain::{
    Account, AccountType, BalanceSource, Currency, Money, NewAccount, NewBalance, OwnershipScope,
};
use time::{format_description::well_known::Iso8601, Date};

use crate::ports::{AccountRepository, RepositoryError};

/// The schema version accepted by the importer and emitted by the exporter.
pub const SCHEMA_VERSION: &str = "1";

/// The required columns, in export order.
pub const COLUMNS: [&str; 11] = [
    "schema_version",
    "record_type",
    "account_name",
    "account_type",
    "institution",
    "currency",
    "ownership_scope",
    "archived",
    "as_of",
    "amount",
    "source",
];

/// A parsed tabular file: a header row plus data rows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Table {
    /// Column names.
    pub headers: Vec<String>,
    /// Cell values, row-major.
    pub rows: Vec<Vec<String>>,
}

/// Errors raised by import/export.
#[derive(Debug, thiserror::Error)]
pub enum ImportError {
    /// The file is malformed or failed validation.
    #[error("{0}")]
    Validation(String),
    /// Persistence failed.
    #[error(transparent)]
    Repository(#[from] RepositoryError),
}

/// A row-level import problem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportRowError {
    /// Spreadsheet line number (the header is line 1).
    pub row: usize,
    /// Why the row was rejected.
    pub message: String,
}

/// The outcome of an import.
#[derive(Debug, Default, Clone)]
pub struct ImportReport {
    /// Accounts created (existing names are reused).
    pub accounts_created: usize,
    /// Balances written.
    pub balances_saved: usize,
    /// Rows that were rejected. If non-empty, nothing was written.
    pub errors: Vec<ImportRowError>,
}

struct HeaderIndices {
    map: HashMap<String, usize>,
}

impl HeaderIndices {
    fn new(headers: &[String]) -> Result<Self, ImportError> {
        let map: HashMap<String, usize> = headers
            .iter()
            .enumerate()
            .map(|(index, header)| (header.trim().to_ascii_lowercase(), index))
            .collect();

        let missing: Vec<&str> = COLUMNS
            .iter()
            .copied()
            .filter(|column| !map.contains_key(*column))
            .collect();

        if !missing.is_empty() {
            return Err(ImportError::Validation(format!(
                "missing required columns: {}",
                missing.join(", ")
            )));
        }

        Ok(Self { map })
    }

    fn get(&self, row: &[String], column: &str) -> String {
        self.map
            .get(column)
            .and_then(|&index| row.get(index))
            .map(|value| value.trim().to_string())
            .unwrap_or_default()
    }
}

struct ParsedAccount {
    name: String,
    account_type: AccountType,
    institution: Option<String>,
    currency: Currency,
    ownership_scope: OwnershipScope,
    archived: bool,
}

struct ParsedBalance {
    account_name: String,
    as_of: Date,
    amount: Money,
    source: BalanceSource,
}

/// Imports and exports the versioned spreadsheet format.
pub struct ImportService {
    accounts: Arc<dyn AccountRepository>,
}

impl ImportService {
    /// Build the service over the account repository.
    pub fn new(accounts: Arc<dyn AccountRepository>) -> Self {
        Self { accounts }
    }

    /// Validate and apply a table. If any row is invalid, nothing is written.
    pub async fn import(&self, table: &Table) -> Result<ImportReport, ImportError> {
        let indices = HeaderIndices::new(&table.headers)?;
        let existing = self.accounts.list_accounts(true).await?;

        let mut known: HashMap<String, Account> = existing
            .into_iter()
            .map(|account| (account.name.to_ascii_lowercase(), account))
            .collect();
        let mut currency_by_name: HashMap<String, Currency> = known
            .iter()
            .map(|(key, account)| (key.clone(), account.currency.clone()))
            .collect();

        let mut report = ImportReport::default();
        let mut accounts_to_create: Vec<ParsedAccount> = Vec::new();
        let mut balances_to_save: Vec<ParsedBalance> = Vec::new();

        // First pass: accounts (so balance rows can resolve a currency).
        for (row_index, row) in table.rows.iter().enumerate() {
            let line = row_index + 2;
            if row.iter().all(|cell| cell.trim().is_empty()) {
                continue;
            }

            let schema_version = indices.get(row, "schema_version");
            if schema_version != SCHEMA_VERSION {
                report.errors.push(ImportRowError {
                    row: line,
                    message: format!(
                        "unsupported schema_version '{schema_version}', expected {SCHEMA_VERSION}"
                    ),
                });
                continue;
            }

            match indices
                .get(row, "record_type")
                .to_ascii_lowercase()
                .as_str()
            {
                "account" => match parse_account_row(row, &indices) {
                    Ok(account) => {
                        currency_by_name
                            .insert(account.name.to_ascii_lowercase(), account.currency.clone());
                        accounts_to_create.push(account);
                    }
                    Err(message) => report.errors.push(ImportRowError { row: line, message }),
                },
                "balance" => {}
                other => report.errors.push(ImportRowError {
                    row: line,
                    message: format!("unknown record_type '{other}'"),
                }),
            }
        }

        // Second pass: balances (all account names are now known).
        for (row_index, row) in table.rows.iter().enumerate() {
            let line = row_index + 2;
            if row.iter().all(|cell| cell.trim().is_empty()) {
                continue;
            }
            if !indices
                .get(row, "record_type")
                .eq_ignore_ascii_case("balance")
            {
                continue;
            }

            match parse_balance_row(row, &indices, &currency_by_name) {
                Ok(balance) => balances_to_save.push(balance),
                Err(message) => report.errors.push(ImportRowError { row: line, message }),
            }
        }

        if !report.errors.is_empty() {
            return Ok(report);
        }

        for account in accounts_to_create {
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
                    owner_user_id: None,
                })
                .await?;

            if account.archived {
                self.accounts.archive_account(created.id).await?;
            }

            report.accounts_created += 1;
            known.insert(key, created);
        }

        for balance in balances_to_save {
            let key = balance.account_name.to_ascii_lowercase();
            let Some(account) = known.get(&key) else {
                return Err(ImportError::Validation(format!(
                    "unknown account '{}'",
                    balance.account_name
                )));
            };

            self.accounts
                .upsert_balance(&NewBalance {
                    account_id: account.id,
                    as_of: balance.as_of,
                    amount: balance.amount,
                    source: balance.source,
                })
                .await?;
            report.balances_saved += 1;
        }

        Ok(report)
    }

    /// Export every account (including archived) and its balances.
    pub async fn export(&self) -> Result<Table, ImportError> {
        let accounts = self.accounts.list_accounts(true).await?;
        let mut rows = Vec::new();

        for account in &accounts {
            rows.push(vec![
                SCHEMA_VERSION.to_string(),
                "account".to_string(),
                account.name.clone(),
                account.account_type.as_str().to_string(),
                account.institution.clone().unwrap_or_default(),
                account.currency.to_string(),
                account.ownership_scope.as_str().to_string(),
                account.archived.to_string(),
                String::new(),
                String::new(),
                String::new(),
            ]);
        }

        for account in &accounts {
            for balance in self.accounts.list_balances(account.id).await? {
                rows.push(vec![
                    SCHEMA_VERSION.to_string(),
                    "balance".to_string(),
                    account.name.clone(),
                    String::new(),
                    String::new(),
                    String::new(),
                    String::new(),
                    String::new(),
                    balance.as_of.to_string(),
                    balance.amount.amount_string(),
                    balance.source.as_str().to_string(),
                ]);
            }
        }

        Ok(Table {
            headers: COLUMNS.iter().map(|column| column.to_string()).collect(),
            rows,
        })
    }
}

/// A starter table users can download and edit.
pub fn template() -> Table {
    let headers = COLUMNS.iter().map(|column| column.to_string()).collect();
    let rows = vec![
        vec![
            SCHEMA_VERSION.to_string(),
            "account".to_string(),
            "Everyday Checking".to_string(),
            "checking".to_string(),
            "Local Bank".to_string(),
            "USD".to_string(),
            "household".to_string(),
            "false".to_string(),
            String::new(),
            String::new(),
            String::new(),
        ],
        vec![
            SCHEMA_VERSION.to_string(),
            "balance".to_string(),
            "Everyday Checking".to_string(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            "2026-10-21".to_string(),
            "2500.00".to_string(),
            "manual".to_string(),
        ],
    ];
    Table { headers, rows }
}

fn parse_account_row(row: &[String], indices: &HeaderIndices) -> Result<ParsedAccount, String> {
    let name = indices.get(row, "account_name");
    if name.is_empty() {
        return Err("account_name is required for account rows".to_string());
    }

    let account_type = AccountType::from_str(&indices.get(row, "account_type"))
        .map_err(|error| error.to_string())?;

    let currency_code = indices.get(row, "currency");
    let currency = if currency_code.is_empty() {
        Currency::usd()
    } else {
        Currency::new(currency_code).map_err(|error| error.to_string())?
    };

    let scope_value = indices.get(row, "ownership_scope");
    let ownership_scope = if scope_value.is_empty() {
        OwnershipScope::Household
    } else {
        OwnershipScope::from_str(&scope_value).map_err(|error| error.to_string())?
    };

    let institution = {
        let value = indices.get(row, "institution");
        if value.is_empty() {
            None
        } else {
            Some(value)
        }
    };

    let archived = parse_bool(&indices.get(row, "archived"))?;

    Ok(ParsedAccount {
        name,
        account_type,
        institution,
        currency,
        ownership_scope,
        archived,
    })
}

fn parse_balance_row(
    row: &[String],
    indices: &HeaderIndices,
    currency_by_name: &HashMap<String, Currency>,
) -> Result<ParsedBalance, String> {
    let account_name = indices.get(row, "account_name");
    if account_name.is_empty() {
        return Err("account_name is required for balance rows".to_string());
    }

    let key = account_name.to_ascii_lowercase();
    let Some(account_currency) = currency_by_name.get(&key) else {
        return Err(format!("unknown account '{account_name}'"));
    };

    let currency_code = indices.get(row, "currency");
    let currency = if currency_code.is_empty() {
        account_currency.clone()
    } else {
        Currency::new(currency_code).map_err(|error| error.to_string())?
    };

    let as_of = Date::parse(&indices.get(row, "as_of"), &Iso8601::DATE)
        .map_err(|error| format!("invalid as_of date: {error}"))?;

    let amount = Money::parse(&indices.get(row, "amount"), currency.as_str())
        .map_err(|error| error.to_string())?;

    let source_value = indices.get(row, "source");
    let source = if source_value.is_empty() {
        BalanceSource::Manual
    } else {
        BalanceSource::from_str(&source_value).map_err(|error| error.to_string())?
    };

    Ok(ParsedBalance {
        account_name,
        as_of,
        amount,
        source,
    })
}

fn parse_bool(value: &str) -> Result<bool, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "" | "false" | "0" | "no" => Ok(false),
        "true" | "1" | "yes" => Ok(true),
        other => Err(format!("invalid boolean '{other}'")),
    }
}
