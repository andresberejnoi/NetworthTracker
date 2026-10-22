//! Account and balance HTTP handlers.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use networth_application::{BatchMode, BatchRow, CreateAccount, RecordBalance, RowError};
use networth_domain::{AccountType, BalanceSource, Currency, Money, OwnershipScope};
use serde::Deserialize;
use time::{format_description::well_known::Iso8601, Date};
use uuid::Uuid;

use crate::dto::{
    AccountDto, BalanceDto, BatchBalanceRow, BatchSaveDto, CreateAccountRequest,
    RecordBalanceRequest, RecordBalancesRequest,
};
use crate::error::ApiError;
use crate::AppState;

/// Query parameters for listing accounts.
#[derive(Debug, Deserialize)]
pub struct ListAccountsQuery {
    /// Include archived accounts.
    #[serde(default)]
    pub include_archived: Option<bool>,
}

/// `GET /api/v1/accounts`
pub async fn list_accounts(
    State(state): State<AppState>,
    Query(query): Query<ListAccountsQuery>,
) -> Result<Json<Vec<AccountDto>>, ApiError> {
    let accounts = state
        .accounts
        .list_accounts(query.include_archived.unwrap_or(false))
        .await?;
    Ok(Json(accounts.into_iter().map(AccountDto::from).collect()))
}

/// `POST /api/v1/accounts`
pub async fn create_account(
    State(state): State<AppState>,
    Json(request): Json<CreateAccountRequest>,
) -> Result<(StatusCode, Json<AccountDto>), ApiError> {
    let account_type: AccountType = request.account_type.parse().map_err(
        |error: networth_domain::accounts::AccountTypeParseError| {
            ApiError::BadRequest(error.to_string())
        },
    )?;
    let currency =
        Currency::new(request.currency).map_err(|error| ApiError::BadRequest(error.to_string()))?;
    let ownership_scope: OwnershipScope = request.ownership_scope.parse().map_err(
        |error: networth_domain::accounts::OwnershipScopeParseError| {
            ApiError::BadRequest(error.to_string())
        },
    )?;

    let account = state
        .accounts
        .create_account(CreateAccount {
            name: request.name,
            account_type,
            institution: request.institution,
            currency,
            ownership_scope,
            owner_user_id: request.owner_user_id,
        })
        .await?;

    Ok((StatusCode::CREATED, Json(AccountDto::from(account))))
}

/// `GET /api/v1/accounts/{id}`
pub async fn get_account(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<AccountDto>, ApiError> {
    let account = state.accounts.get_account(id).await?;
    Ok(Json(AccountDto::from(account)))
}

/// `POST /api/v1/accounts/{id}/archive`
pub async fn archive_account(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    state.accounts.archive_account(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `GET /api/v1/accounts/{id}/balances`
pub async fn list_balances(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<BalanceDto>>, ApiError> {
    let balances = state.accounts.list_balances(id).await?;
    Ok(Json(balances.into_iter().map(BalanceDto::from).collect()))
}

/// `POST /api/v1/accounts/{id}/balances`
pub async fn record_balance(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(request): Json<RecordBalanceRequest>,
) -> Result<(StatusCode, Json<BalanceDto>), ApiError> {
    let as_of = parse_date(&request.as_of)?;

    let account = state.accounts.get_account(id).await?;
    let currency = match request.currency {
        Some(code) => Currency::new(code).map_err(|e| ApiError::BadRequest(e.to_string()))?,
        None => account.currency.clone(),
    };

    let amount = Money::parse(&request.amount, currency.as_str())
        .map_err(|error| ApiError::BadRequest(error.to_string()))?;

    let source = match request.source {
        Some(value) => value.parse::<BalanceSource>().map_err(
            |error: networth_domain::accounts::BalanceSourceParseError| {
                ApiError::BadRequest(error.to_string())
            },
        )?,
        None => BalanceSource::Manual,
    };

    let balance = state
        .accounts
        .record_balance(RecordBalance {
            account_id: id,
            as_of,
            amount,
            source,
        })
        .await?;

    Ok((StatusCode::CREATED, Json(BalanceDto::from(balance))))
}

pub(crate) fn parse_date(value: &str) -> Result<Date, ApiError> {
    Date::parse(value, &Iso8601::DATE)
        .map_err(|error| ApiError::BadRequest(format!("invalid date '{value}': {error}")))
}

/// `POST /api/v1/accounts/{id}/balances/batch`
///
/// Saves a grid of balances in one action. Rows that cannot be parsed are
/// reported with their original index; the mode decides whether valid rows are
/// still saved (`partial`) or the batch is rejected (`atomic`, the default).
pub async fn record_balances(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(request): Json<RecordBalancesRequest>,
) -> Result<Json<BatchSaveDto>, ApiError> {
    let account = state.accounts.get_account(id).await?;
    let mut rows = Vec::new();
    let mut errors = Vec::new();

    for (index, raw) in request.rows.into_iter().enumerate() {
        match parse_batch_row(raw, id, &account.currency) {
            Ok(balance) => rows.push(BatchRow { index, balance }),
            Err(message) => errors.push(RowError { index, message }),
        }
    }

    let mode = parse_batch_mode(request.mode.as_deref())?;
    let result = state
        .accounts
        .record_balances(id, rows, errors, mode)
        .await?;
    Ok(Json(BatchSaveDto::from(result)))
}

fn parse_batch_row(
    raw: BatchBalanceRow,
    account_id: Uuid,
    default_currency: &Currency,
) -> Result<RecordBalance, String> {
    let as_of = Date::parse(&raw.as_of, &Iso8601::DATE)
        .map_err(|error| format!("invalid date '{}': {error}", raw.as_of))?;

    let currency = match raw.currency {
        Some(code) => Currency::new(code).map_err(|error| error.to_string())?,
        None => default_currency.clone(),
    };

    let amount = Money::parse(&raw.amount, currency.as_str()).map_err(|error| error.to_string())?;

    let source = match raw.source {
        Some(value) => value.parse::<BalanceSource>().map_err(
            |error: networth_domain::accounts::BalanceSourceParseError| error.to_string(),
        )?,
        None => BalanceSource::Manual,
    };

    Ok(RecordBalance {
        account_id,
        as_of,
        amount,
        source,
    })
}

fn parse_batch_mode(value: Option<&str>) -> Result<BatchMode, ApiError> {
    match value.map(|mode| mode.trim().to_ascii_lowercase()) {
        None => Ok(BatchMode::Atomic),
        Some(mode) if mode == "atomic" => Ok(BatchMode::Atomic),
        Some(mode) if mode == "partial" => Ok(BatchMode::Partial),
        Some(other) => Err(ApiError::BadRequest(format!(
            "unknown batch mode '{other}'"
        ))),
    }
}
