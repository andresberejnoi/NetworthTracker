//! Income, spending plans, spending records, and variance endpoints.

use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use bigdecimal::BigDecimal;
use networth_application::{BudgetAllocationView, BudgetView};
use networth_domain::{
    BudgetAllocation, BudgetCategory, BudgetSpending, BudgetVariance, Currency, Money,
};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::auth::CurrentUser;
use crate::error::ApiError;
use crate::AppState;

fn require_writer(current: &CurrentUser) -> Result<(), ApiError> {
    if current.user.role.can_write() {
        Ok(())
    } else {
        Err(ApiError::Forbidden("your role is read-only".to_string()))
    }
}

fn default_year() -> i32 {
    OffsetDateTime::now_utc().year()
}

/// Query for the budget view.
#[derive(Debug, Deserialize)]
pub struct BudgetQuery {
    /// Year; defaults to the current UTC year.
    #[serde(default)]
    pub year: Option<i32>,
}

/// A plan allocation in the API.
#[derive(Debug, Serialize)]
pub struct BudgetAllocationDto {
    /// Category.
    pub category: String,
    /// Percentage.
    pub percentage: String,
    /// Planned amount.
    pub planned: String,
}

impl From<BudgetAllocationView> for BudgetAllocationDto {
    fn from(value: BudgetAllocationView) -> Self {
        Self {
            category: value.category.as_str().to_string(),
            percentage: value.percentage.to_string(),
            planned: value.planned.amount_string(),
        }
    }
}

/// A spending record in the API.
#[derive(Debug, Serialize)]
pub struct BudgetSpendingDto {
    /// Identifier.
    pub id: uuid::Uuid,
    /// Year.
    pub year: i32,
    /// Month.
    pub month: u32,
    /// Category.
    pub category: String,
    /// Amount.
    pub amount: String,
    /// Currency.
    pub currency: String,
    /// Note.
    pub note: Option<String>,
}

impl From<BudgetSpending> for BudgetSpendingDto {
    fn from(value: BudgetSpending) -> Self {
        Self {
            id: value.id,
            year: value.year,
            month: value.month,
            category: value.category.as_str().to_string(),
            amount: value.amount.amount_string(),
            currency: value.amount.currency().to_string(),
            note: value.note,
        }
    }
}

/// A variance row in the API.
#[derive(Debug, Serialize)]
pub struct BudgetVarianceDto {
    /// Category.
    pub category: String,
    /// Planned amount.
    pub planned: String,
    /// Actual amount.
    pub actual: String,
    /// Planned minus actual.
    pub variance_amount: String,
    /// Difference as a percentage of planned.
    pub variance_percent: Option<String>,
}

impl From<BudgetVariance> for BudgetVarianceDto {
    fn from(value: BudgetVariance) -> Self {
        Self {
            category: value.category.as_str().to_string(),
            planned: value.planned.amount_string(),
            actual: value.actual.amount_string(),
            variance_amount: value.variance_amount.amount_string(),
            variance_percent: value.variance_percent.map(|value| value.to_string()),
        }
    }
}

/// The complete budget view.
#[derive(Debug, Serialize)]
pub struct BudgetViewDto {
    /// Year.
    pub year: i32,
    /// Currency.
    pub currency: String,
    /// Annual income.
    pub income: Option<String>,
    /// Allocations with planned amounts.
    pub allocations: Vec<BudgetAllocationDto>,
    /// Total allocated percentage.
    pub total_percentage: String,
    /// Exactly 100% allocated.
    pub balanced: bool,
    /// More than 100% allocated.
    pub over_allocated: bool,
    /// Spending records.
    pub spending: Vec<BudgetSpendingDto>,
    /// Plan versus actual.
    pub variance: Vec<BudgetVarianceDto>,
}

impl From<BudgetView> for BudgetViewDto {
    fn from(value: BudgetView) -> Self {
        Self {
            year: value.year,
            currency: value.currency.to_string(),
            income: value.income.map(|income| income.amount_string()),
            allocations: value
                .allocations
                .into_iter()
                .map(BudgetAllocationDto::from)
                .collect(),
            total_percentage: value.total_percentage.to_string(),
            balanced: value.balanced,
            over_allocated: value.over_allocated,
            spending: value
                .spending
                .into_iter()
                .map(BudgetSpendingDto::from)
                .collect(),
            variance: value
                .variance
                .into_iter()
                .map(BudgetVarianceDto::from)
                .collect(),
        }
    }
}

/// Income request.
#[derive(Debug, Deserialize)]
pub struct BudgetIncomeRequest {
    /// Year.
    pub year: i32,
    /// Annual amount.
    pub amount: String,
    /// Currency; defaults to USD.
    #[serde(default)]
    pub currency: Option<String>,
}

/// One allocation request.
#[derive(Debug, Deserialize)]
pub struct BudgetAllocationRequest {
    /// Category.
    pub category: String,
    /// Percentage.
    pub percentage: String,
}

/// Plan request.
#[derive(Debug, Deserialize)]
pub struct BudgetPlanRequest {
    /// Year.
    pub year: i32,
    /// Currency; defaults to USD.
    #[serde(default)]
    pub currency: Option<String>,
    /// Allocations.
    pub allocations: Vec<BudgetAllocationRequest>,
}

/// Spending request.
#[derive(Debug, Deserialize)]
pub struct BudgetSpendingRequest {
    /// Year.
    pub year: i32,
    /// Month.
    pub month: u32,
    /// Category.
    pub category: String,
    /// Amount.
    pub amount: String,
    /// Currency; defaults to USD.
    #[serde(default)]
    pub currency: Option<String>,
    /// Optional note.
    #[serde(default)]
    pub note: Option<String>,
}

fn parse_currency(value: Option<&str>) -> Result<Currency, ApiError> {
    match value {
        Some(code) => Currency::new(code).map_err(|error| ApiError::BadRequest(error.to_string())),
        None => Ok(Currency::usd()),
    }
}

/// `GET /api/v1/budget`
pub async fn get_budget(
    State(state): State<AppState>,
    Query(query): Query<BudgetQuery>,
    current: CurrentUser,
) -> Result<Json<BudgetViewDto>, ApiError> {
    let year = query.year.unwrap_or_else(default_year);
    Ok(Json(state.budget.view(current.user.id, year).await?.into()))
}

/// `POST /api/v1/budget/income`
pub async fn set_income(
    State(state): State<AppState>,
    current: CurrentUser,
    Json(request): Json<BudgetIncomeRequest>,
) -> Result<StatusCode, ApiError> {
    require_writer(&current)?;
    let currency = parse_currency(request.currency.as_deref())?;
    let amount = Money::parse(&request.amount, currency.as_str())
        .map_err(|error| ApiError::BadRequest(error.to_string()))?;
    state
        .budget
        .set_income(current.user.id, request.year, amount)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `POST /api/v1/budget/plan`
pub async fn set_plan(
    State(state): State<AppState>,
    current: CurrentUser,
    Json(request): Json<BudgetPlanRequest>,
) -> Result<StatusCode, ApiError> {
    require_writer(&current)?;
    let currency = parse_currency(request.currency.as_deref())?;
    let mut allocations = Vec::with_capacity(request.allocations.len());
    for allocation in request.allocations {
        allocations.push(BudgetAllocation {
            category: allocation
                .category
                .parse::<BudgetCategory>()
                .map_err(|error| ApiError::BadRequest(error.to_string()))?,
            percentage: allocation
                .percentage
                .parse::<BigDecimal>()
                .map_err(|error| ApiError::BadRequest(format!("invalid percentage: {error}")))?,
        });
    }
    state
        .budget
        .set_plan(current.user.id, request.year, currency, allocations)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `POST /api/v1/budget/spending`
pub async fn record_spending(
    State(state): State<AppState>,
    current: CurrentUser,
    Json(request): Json<BudgetSpendingRequest>,
) -> Result<StatusCode, ApiError> {
    require_writer(&current)?;
    let currency = parse_currency(request.currency.as_deref())?;
    let amount = Money::parse(&request.amount, currency.as_str())
        .map_err(|error| ApiError::BadRequest(error.to_string()))?;
    let category = request
        .category
        .parse::<BudgetCategory>()
        .map_err(|error| ApiError::BadRequest(error.to_string()))?;
    state
        .budget
        .record_spending(
            current.user.id,
            request.year,
            request.month,
            category,
            amount,
            request.note,
        )
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
