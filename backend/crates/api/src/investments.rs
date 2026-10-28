//! Securities, holdings, prices, and FX endpoints.

use std::collections::HashMap;
use std::str::FromStr;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use bigdecimal::BigDecimal;
use networth_application::HoldingValuation;
use networth_domain::{
    Currency, Holding, Money, NewLot, NewSecurity, PriceQuote, Quantity, Sector, Security,
    SecurityAssetClass,
};
use serde::{Deserialize, Serialize};
use time::{format_description::well_known::Iso8601, Date};
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::error::ApiError;
use crate::AppState;

fn default_currency() -> String {
    "USD".to_string()
}

fn parse_currency(value: Option<&str>) -> Result<Currency, ApiError> {
    match value {
        Some(code) => Currency::new(code).map_err(|error| ApiError::BadRequest(error.to_string())),
        None => Ok(Currency::usd()),
    }
}

fn require_writer(current: &CurrentUser) -> Result<(), ApiError> {
    if current.user.role.can_write() {
        Ok(())
    } else {
        Err(ApiError::Forbidden("your role is read-only".to_string()))
    }
}

async fn visible_account(
    state: &AppState,
    current: &CurrentUser,
    account_id: Uuid,
) -> Result<(), ApiError> {
    let account = state.accounts.get_account(account_id).await?;
    if current
        .user
        .can_see(account.ownership_scope, account.owner_user_id)
    {
        Ok(())
    } else {
        Err(ApiError::NotFound(format!(
            "account not found: {account_id}"
        )))
    }
}

async fn modifiable_account(
    state: &AppState,
    current: &CurrentUser,
    account_id: Uuid,
) -> Result<(), ApiError> {
    let account = state.accounts.get_account(account_id).await?;
    if current
        .user
        .can_modify(account.ownership_scope, account.owner_user_id)
    {
        Ok(())
    } else {
        Err(ApiError::Forbidden(
            "you cannot modify this account".to_string(),
        ))
    }
}

/// A security in the API.
#[derive(Debug, Serialize)]
pub struct SecurityDto {
    /// Identifier.
    pub id: Uuid,
    /// Ticker symbol.
    pub symbol: String,
    /// Display name.
    pub name: String,
    /// Asset class.
    pub asset_class: String,
    /// Sector.
    pub sector: Option<String>,
    /// Currency.
    pub currency: String,
}

impl From<Security> for SecurityDto {
    fn from(security: Security) -> Self {
        Self {
            id: security.id,
            symbol: security.symbol,
            name: security.name,
            asset_class: security.asset_class.as_str().to_string(),
            sector: security.sector.map(|sector| sector.as_str().to_string()),
            currency: security.currency.to_string(),
        }
    }
}

/// Request body for creating a security.
#[derive(Debug, Deserialize)]
pub struct CreateSecurityRequest {
    /// Ticker symbol.
    pub symbol: String,
    /// Display name.
    pub name: String,
    /// `equity`, `fixed_income`, `real_estate`, `commodity`, `cash`, `other`.
    pub asset_class: String,
    /// Optional sector.
    #[serde(default)]
    pub sector: Option<String>,
    /// Currency; defaults to USD.
    #[serde(default = "default_currency")]
    pub currency: String,
}

/// A tax lot in the API.
#[derive(Debug, Serialize)]
pub struct LotDto {
    /// Identifier.
    pub id: Uuid,
    /// ISO acquisition date.
    pub acquired_on: String,
    /// Quantity as a decimal string.
    pub quantity: String,
    /// Cost basis as a decimal string.
    pub cost_basis: String,
}

/// A holding with valuation.
#[derive(Debug, Serialize)]
pub struct HoldingDto {
    /// Identifier.
    pub id: Uuid,
    /// Account identifier.
    pub account_id: Uuid,
    /// Ticker symbol.
    pub symbol: String,
    /// Security name.
    pub name: String,
    /// Asset class.
    pub asset_class: String,
    /// Sector.
    pub sector: Option<String>,
    /// Currency.
    pub currency: String,
    /// Total quantity.
    pub quantity: String,
    /// Total cost basis.
    pub cost_basis: String,
    /// Latest unit price, if available.
    pub price: Option<String>,
    /// When the price was observed.
    pub price_as_of: Option<String>,
    /// Whether the price is stale.
    pub price_stale: bool,
    /// Price provenance.
    pub price_source: Option<String>,
    /// Market value.
    pub market_value: Option<String>,
    /// Unrealized gain/loss.
    pub unrealized_gain: Option<String>,
    /// Tax lots.
    pub lots: Vec<LotDto>,
}

impl From<HoldingValuation> for HoldingDto {
    fn from(value: HoldingValuation) -> Self {
        let holding = value.holding;
        let price = value.price;
        Self {
            id: holding.id,
            account_id: holding.account_id,
            symbol: holding.security.symbol.clone(),
            name: holding.security.name.clone(),
            asset_class: holding.security.asset_class.as_str().to_string(),
            sector: holding
                .security
                .sector
                .map(|sector| sector.as_str().to_string()),
            currency: holding.security.currency.to_string(),
            quantity: holding.quantity().to_string(),
            cost_basis: holding.cost_basis().amount_string(),
            price: price.as_ref().map(|quote| quote.price.amount_string()),
            price_as_of: price.as_ref().map(|quote| quote.as_of.to_string()),
            price_stale: price.as_ref().map(|quote| quote.stale).unwrap_or(false),
            price_source: price
                .as_ref()
                .map(|quote| quote.source.as_str().to_string()),
            market_value: value.market_value.map(|value| value.amount_string()),
            unrealized_gain: value.unrealized_gain.map(|value| value.amount_string()),
            lots: holding
                .lots
                .into_iter()
                .map(|lot| LotDto {
                    id: lot.id,
                    acquired_on: lot.acquired_on.to_string(),
                    quantity: lot.quantity.to_string(),
                    cost_basis: lot.cost_basis.amount_string(),
                })
                .collect(),
        }
    }
}

impl From<Holding> for HoldingDto {
    fn from(holding: Holding) -> Self {
        HoldingDto::from(HoldingValuation {
            holding,
            price: None,
            market_value: None,
            unrealized_gain: None,
        })
    }
}

/// One lot in a holding request.
#[derive(Debug, Deserialize)]
pub struct LotRequest {
    /// ISO acquisition date.
    pub acquired_on: String,
    /// Quantity as a decimal string.
    pub quantity: String,
    /// Cost basis as a decimal string.
    pub cost_basis: String,
    /// Currency; defaults to USD.
    #[serde(default)]
    pub currency: Option<String>,
}

/// Request body for creating or replacing a holding.
#[derive(Debug, Deserialize)]
pub struct CreateHoldingRequest {
    /// Ticker symbol.
    pub symbol: String,
    /// Lots.
    pub lots: Vec<LotRequest>,
}

/// One row of a holdings batch.
#[derive(Debug, Deserialize)]
pub struct HoldingBatchRow {
    /// Ticker symbol.
    pub symbol: String,
    /// ISO acquisition date.
    pub acquired_on: String,
    /// Quantity as a decimal string.
    pub quantity: String,
    /// Cost basis as a decimal string.
    pub cost_basis: String,
}

/// Request body for a holdings batch.
#[derive(Debug, Deserialize)]
pub struct HoldingBatchRequest {
    /// Rows.
    pub rows: Vec<HoldingBatchRow>,
}

/// A row-level error.
#[derive(Debug, Serialize)]
pub struct HoldingRowErrorDto {
    /// Zero-based row index.
    pub index: usize,
    /// Reason.
    pub error: String,
}

/// Result of a holdings batch.
#[derive(Debug, Serialize)]
pub struct HoldingBatchResultDto {
    /// Rows saved.
    pub saved: usize,
    /// Rows skipped.
    pub errors: Vec<HoldingRowErrorDto>,
}

/// A price quote in the API.
#[derive(Debug, Serialize)]
pub struct PriceDto {
    /// Ticker symbol.
    pub symbol: String,
    /// Unit price.
    pub price: String,
    /// Currency.
    pub currency: String,
    /// When observed.
    pub as_of: String,
    /// Whether stale.
    pub stale: bool,
    /// `provider` or `manual`.
    pub source: String,
}

impl From<PriceQuote> for PriceDto {
    fn from(quote: PriceQuote) -> Self {
        Self {
            symbol: quote.symbol,
            price: quote.price.amount_string(),
            currency: quote.price.currency().to_string(),
            as_of: quote.as_of.to_string(),
            stale: quote.stale,
            source: quote.source.as_str().to_string(),
        }
    }
}

/// Request body for a manual price.
#[derive(Debug, Deserialize)]
pub struct SetPriceRequest {
    /// Ticker symbol.
    pub symbol: String,
    /// Unit price.
    pub price: String,
    /// Currency; defaults to USD.
    #[serde(default)]
    pub currency: Option<String>,
}

/// Provider status.
#[derive(Debug, Serialize)]
pub struct ProviderStatusDto {
    /// Whether a provider is enabled.
    pub configured: bool,
    /// Provider name.
    pub name: Option<String>,
}

/// Request body for an FX rate.
#[derive(Debug, Deserialize)]
pub struct SetFxRateRequest {
    /// Base currency.
    pub base: String,
    /// Quote currency.
    pub quote: String,
    /// Rate.
    pub rate: String,
}

/// FX rate response.
#[derive(Debug, Serialize)]
pub struct FxRateDto {
    /// Base currency.
    pub base: String,
    /// Quote currency.
    pub quote: String,
    /// Rate as a decimal string.
    pub rate: String,
}

/// Query for an FX rate.
#[derive(Debug, Deserialize)]
pub struct FxRateQuery {
    /// Base currency.
    pub base: String,
    /// Quote currency.
    pub quote: String,
}

/// `GET /api/v1/securities`
pub async fn list_securities(
    State(state): State<AppState>,
    _current: CurrentUser,
) -> Result<Json<Vec<SecurityDto>>, ApiError> {
    let securities = state.investments.list_securities().await?;
    Ok(Json(
        securities.into_iter().map(SecurityDto::from).collect(),
    ))
}

/// `POST /api/v1/securities`
pub async fn create_security(
    State(state): State<AppState>,
    current: CurrentUser,
    Json(request): Json<CreateSecurityRequest>,
) -> Result<(StatusCode, Json<SecurityDto>), ApiError> {
    require_writer(&current)?;

    let asset_class = request.asset_class.parse::<SecurityAssetClass>().map_err(
        |error: networth_domain::investments::SecurityAssetClassParseError| {
            ApiError::BadRequest(error.to_string())
        },
    )?;
    let sector = request
        .sector
        .as_deref()
        .map(Sector::from_str)
        .transpose()
        .map_err(|error: networth_domain::investments::SectorParseError| {
            ApiError::BadRequest(error.to_string())
        })?;

    let security = state
        .investments
        .create_security(NewSecurity {
            symbol: request.symbol,
            name: request.name,
            asset_class,
            sector,
            currency: Currency::new(request.currency)
                .map_err(|error| ApiError::BadRequest(error.to_string()))?,
        })
        .await?;

    Ok((StatusCode::CREATED, Json(SecurityDto::from(security))))
}

/// `GET /api/v1/accounts/{id}/holdings`
pub async fn list_holdings(
    State(state): State<AppState>,
    Path(account_id): Path<Uuid>,
    current: CurrentUser,
) -> Result<Json<Vec<HoldingDto>>, ApiError> {
    visible_account(&state, &current, account_id).await?;
    let holdings = state.investments.list_holdings(account_id).await?;
    Ok(Json(holdings.into_iter().map(HoldingDto::from).collect()))
}

/// `POST /api/v1/accounts/{id}/holdings`
pub async fn create_holding(
    State(state): State<AppState>,
    Path(account_id): Path<Uuid>,
    current: CurrentUser,
    Json(request): Json<CreateHoldingRequest>,
) -> Result<(StatusCode, Json<HoldingDto>), ApiError> {
    modifiable_account(&state, &current, account_id).await?;

    let lots = parse_lots(request.lots)?;
    let holding = state
        .investments
        .upsert_holding(account_id, &request.symbol, lots)
        .await?;

    Ok((StatusCode::CREATED, Json(HoldingDto::from(holding))))
}

/// `DELETE /api/v1/accounts/{id}/holdings/{holding_id}`
pub async fn delete_holding(
    State(state): State<AppState>,
    Path((account_id, holding_id)): Path<(Uuid, Uuid)>,
    current: CurrentUser,
) -> Result<StatusCode, ApiError> {
    modifiable_account(&state, &current, account_id).await?;

    let holdings = state.investments.list_holdings(account_id).await?;
    if !holdings
        .iter()
        .any(|valuation| valuation.holding.id == holding_id)
    {
        return Err(ApiError::NotFound(format!(
            "holding not found: {holding_id}"
        )));
    }

    state.investments.delete_holding(holding_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `POST /api/v1/accounts/{id}/holdings/batch`
pub async fn record_holdings_batch(
    State(state): State<AppState>,
    Path(account_id): Path<Uuid>,
    current: CurrentUser,
    Json(request): Json<HoldingBatchRequest>,
) -> Result<Json<HoldingBatchResultDto>, ApiError> {
    modifiable_account(&state, &current, account_id).await?;

    let mut by_symbol: HashMap<String, (usize, Vec<NewLot>)> = HashMap::new();
    let mut errors = Vec::new();

    for (index, row) in request.rows.into_iter().enumerate() {
        let symbol = row.symbol.trim().to_ascii_uppercase();
        if symbol.is_empty() {
            errors.push(HoldingRowErrorDto {
                index,
                error: "symbol is required".to_string(),
            });
            continue;
        }

        match parse_lot(&row.acquired_on, &row.quantity, &row.cost_basis, None) {
            Ok(lot) => {
                by_symbol
                    .entry(symbol)
                    .or_insert_with(|| (index, Vec::new()))
                    .1
                    .push(lot);
            }
            Err(message) => errors.push(HoldingRowErrorDto {
                index,
                error: message,
            }),
        }
    }

    let mut saved = 0;
    for (symbol, (index, lots)) in by_symbol {
        let count = lots.len();
        match state
            .investments
            .upsert_holding(account_id, &symbol, lots)
            .await
        {
            Ok(_) => saved += count,
            Err(error) => errors.push(HoldingRowErrorDto {
                index,
                error: format!("{symbol}: {error}"),
            }),
        }
    }

    Ok(Json(HoldingBatchResultDto { saved, errors }))
}

/// `GET /api/v1/prices/provider`
pub async fn provider_status(
    State(state): State<AppState>,
    _current: CurrentUser,
) -> Json<ProviderStatusDto> {
    let name = state.prices.provider_name();
    Json(ProviderStatusDto {
        configured: name.is_some(),
        name: name.map(|name| name.to_string()),
    })
}

/// `GET /api/v1/prices/{symbol}`
pub async fn get_price(
    State(state): State<AppState>,
    Path(symbol): Path<String>,
    _current: CurrentUser,
) -> Result<Json<PriceDto>, ApiError> {
    match state.prices.quote(&symbol).await? {
        Some(quote) => Ok(Json(PriceDto::from(quote))),
        None => Err(ApiError::NotFound(format!("no price for '{symbol}'"))),
    }
}

/// `POST /api/v1/prices`
pub async fn set_price(
    State(state): State<AppState>,
    current: CurrentUser,
    Json(request): Json<SetPriceRequest>,
) -> Result<(StatusCode, Json<PriceDto>), ApiError> {
    require_writer(&current)?;

    let currency = parse_currency(request.currency.as_deref())?;
    let price = Money::parse(&request.price, currency.as_str())
        .map_err(|error| ApiError::BadRequest(error.to_string()))?;
    let quote = state.prices.set_manual(&request.symbol, price).await?;
    Ok((StatusCode::CREATED, Json(PriceDto::from(quote))))
}

/// `POST /api/v1/fx-rates`
pub async fn set_fx_rate(
    State(state): State<AppState>,
    current: CurrentUser,
    Json(request): Json<SetFxRateRequest>,
) -> Result<StatusCode, ApiError> {
    require_writer(&current)?;

    let base =
        Currency::new(request.base).map_err(|error| ApiError::BadRequest(error.to_string()))?;
    let quote =
        Currency::new(request.quote).map_err(|error| ApiError::BadRequest(error.to_string()))?;
    let rate = BigDecimal::from_str(request.rate.trim())
        .map_err(|error| ApiError::BadRequest(format!("invalid rate: {error}")))?;

    state.fx.set_rate(base, quote, rate).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `GET /api/v1/fx-rates?base=USD&quote=EUR`
pub async fn get_fx_rate(
    State(state): State<AppState>,
    Query(query): Query<FxRateQuery>,
    _current: CurrentUser,
) -> Result<Json<FxRateDto>, ApiError> {
    let base =
        Currency::new(query.base).map_err(|error| ApiError::BadRequest(error.to_string()))?;
    let quote =
        Currency::new(query.quote).map_err(|error| ApiError::BadRequest(error.to_string()))?;

    match state.fx.rate(&base, &quote).await? {
        Some(rate) => Ok(Json(FxRateDto {
            base: base.to_string(),
            quote: quote.to_string(),
            rate: rate.to_string(),
        })),
        None => Err(ApiError::NotFound(format!(
            "no FX rate from {base} to {quote}"
        ))),
    }
}

fn parse_lots(requests: Vec<LotRequest>) -> Result<Vec<NewLot>, ApiError> {
    if requests.is_empty() {
        return Err(ApiError::BadRequest(
            "a holding needs at least one lot".to_string(),
        ));
    }

    requests
        .into_iter()
        .map(|request| {
            parse_lot(
                &request.acquired_on,
                &request.quantity,
                &request.cost_basis,
                request.currency.as_deref(),
            )
            .map_err(ApiError::BadRequest)
        })
        .collect()
}

fn parse_lot(
    acquired_on: &str,
    quantity: &str,
    cost_basis: &str,
    currency: Option<&str>,
) -> Result<NewLot, String> {
    let acquired_on = Date::parse(acquired_on, &Iso8601::DATE)
        .map_err(|error| format!("invalid acquired_on date: {error}"))?;
    let quantity = Quantity::parse(quantity).map_err(|error| error.to_string())?;
    let currency = match currency {
        Some(code) => Currency::new(code).map_err(|error| error.to_string())?,
        None => Currency::usd(),
    };
    let cost_basis =
        Money::parse(cost_basis, currency.as_str()).map_err(|error| error.to_string())?;

    Ok(NewLot {
        acquired_on,
        quantity,
        cost_basis,
    })
}
