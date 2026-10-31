//! Crypto assets, wallets, balances, and positions.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use networth_application::{CryptoError, CryptoPosition};
use networth_domain::{
    Chain, CryptoAsset, CryptoBalance, CryptoWallet, CryptoWalletKind, Currency, NewCryptoAsset,
    NewCryptoWallet, OwnershipScope, Quantity,
};
use serde::{Deserialize, Serialize};
use time::Date;
use uuid::Uuid;

use crate::accounts::parse_date;
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

fn household_only(scope: Option<&str>) -> Result<OwnershipScope, ApiError> {
    match scope {
        Some("user") => Err(ApiError::BadRequest(
            "private crypto wallets are not supported yet; use household scope".to_string(),
        )),
        Some("household") | None => Ok(OwnershipScope::Household),
        Some(other) => Err(ApiError::BadRequest(format!(
            "unknown ownership scope: {other}"
        ))),
    }
}

/// A crypto asset in the API.
#[derive(Debug, Serialize)]
pub struct CryptoAssetDto {
    /// Identifier.
    pub id: Uuid,
    /// Ticker.
    pub symbol: String,
    /// Name.
    pub name: String,
    /// Chain.
    pub chain: String,
    /// Token contract, if any.
    pub contract_address: Option<String>,
    /// Base-unit decimals.
    pub decimals: u32,
    /// Quote currency.
    pub currency: String,
    /// Provider-specific coin id.
    pub price_provider_id: Option<String>,
}

impl From<CryptoAsset> for CryptoAssetDto {
    fn from(asset: CryptoAsset) -> Self {
        Self {
            id: asset.id,
            symbol: asset.symbol,
            name: asset.name,
            chain: asset.chain.as_str().to_string(),
            contract_address: asset.contract_address,
            decimals: asset.decimals,
            currency: asset.currency.to_string(),
            price_provider_id: asset.price_provider_id,
        }
    }
}

/// Request body for creating a crypto asset.
#[derive(Debug, Deserialize)]
pub struct CreateCryptoAssetRequest {
    /// Ticker.
    pub symbol: String,
    /// Name.
    pub name: String,
    /// `bitcoin` or `ethereum`.
    pub chain: String,
    /// Token contract, if any.
    #[serde(default)]
    pub contract_address: Option<String>,
    /// Base-unit decimals; defaults to 18.
    #[serde(default)]
    pub decimals: Option<u32>,
    /// Quote currency; defaults to USD.
    #[serde(default)]
    pub currency: Option<String>,
    /// Provider-specific coin id.
    #[serde(default)]
    pub price_provider_id: Option<String>,
}

/// A crypto wallet in the API.
#[derive(Debug, Serialize)]
pub struct CryptoWalletDto {
    /// Identifier.
    pub id: Uuid,
    /// Asset identifier.
    pub asset_id: Uuid,
    /// Ticker.
    pub symbol: String,
    /// Asset name.
    pub name: String,
    /// Chain.
    pub chain: String,
    /// `address` or `manual`.
    pub kind: String,
    /// Public address, if any.
    pub address: Option<String>,
    /// Label.
    pub label: String,
    /// Ownership scope.
    pub ownership_scope: String,
    /// Owner, for private wallets.
    pub owner_user_id: Option<Uuid>,
    /// Whether archived.
    pub archived: bool,
}

impl From<CryptoWallet> for CryptoWalletDto {
    fn from(wallet: CryptoWallet) -> Self {
        Self {
            id: wallet.id,
            asset_id: wallet.asset.id,
            symbol: wallet.asset.symbol,
            name: wallet.asset.name,
            chain: wallet.chain.as_str().to_string(),
            kind: wallet.kind.as_str().to_string(),
            address: wallet.address,
            label: wallet.label,
            ownership_scope: wallet.ownership_scope.as_str().to_string(),
            owner_user_id: wallet.owner_user_id,
            archived: wallet.archived,
        }
    }
}

/// Request body for creating a crypto wallet.
#[derive(Debug, Deserialize)]
pub struct CreateCryptoWalletRequest {
    /// Asset identifier.
    pub asset_id: Uuid,
    /// `address` or `manual`.
    pub kind: String,
    /// `bitcoin` or `ethereum`.
    pub chain: String,
    /// Public address for address wallets.
    #[serde(default)]
    pub address: Option<String>,
    /// Label.
    pub label: String,
    /// Ownership scope; only household is supported for now.
    #[serde(default)]
    pub ownership_scope: Option<String>,
}

/// A dated crypto balance in the API.
#[derive(Debug, Serialize)]
pub struct CryptoBalanceDto {
    /// Identifier.
    pub id: Uuid,
    /// Wallet.
    pub wallet_id: Uuid,
    /// ISO date.
    pub as_of: String,
    /// Exact quantity.
    pub quantity: String,
    /// `manual`, `chain`, or `import`.
    pub source: String,
    /// Provider name.
    pub provider: Option<String>,
    /// Fetch timestamp.
    pub fetched_at: Option<String>,
}

impl From<CryptoBalance> for CryptoBalanceDto {
    fn from(balance: CryptoBalance) -> Self {
        Self {
            id: balance.id,
            wallet_id: balance.wallet_id,
            as_of: balance.as_of.to_string(),
            quantity: balance.quantity.to_string(),
            source: balance.source.as_str().to_string(),
            provider: balance.provider,
            fetched_at: balance.fetched_at.map(|value| value.to_string()),
        }
    }
}

/// Request body for recording a manual crypto balance.
#[derive(Debug, Deserialize)]
pub struct RecordCryptoBalanceRequest {
    /// ISO date.
    pub as_of: String,
    /// Exact quantity.
    pub quantity: String,
}

/// A crypto position aggregated across wallets.
#[derive(Debug, Serialize)]
pub struct CryptoPositionDto {
    /// Asset identifier.
    pub asset_id: Uuid,
    /// Ticker.
    pub symbol: String,
    /// Name.
    pub name: String,
    /// Chain.
    pub chain: String,
    /// Total quantity.
    pub quantity: String,
    /// Contributing wallets.
    pub wallet_count: usize,
    /// Unit price.
    pub price: Option<String>,
    /// Price timestamp.
    pub price_as_of: Option<String>,
    /// Whether the price is stale.
    pub price_stale: bool,
    /// Price source.
    pub price_source: Option<String>,
    /// Market value.
    pub value: Option<String>,
    /// Reporting currency.
    pub currency: String,
}

impl CryptoPositionDto {
    fn from_position(position: CryptoPosition, currency: &Currency) -> Self {
        let price = position.price.as_ref();
        Self {
            asset_id: position.asset.id,
            symbol: position.asset.symbol,
            name: position.asset.name,
            chain: position.asset.chain.as_str().to_string(),
            quantity: position.quantity.to_string(),
            wallet_count: position.wallet_count,
            price: price.map(|quote| quote.price.amount_string()),
            price_as_of: price.map(|quote| quote.as_of.to_string()),
            price_stale: price.map(|quote| quote.stale).unwrap_or(false),
            price_source: price.map(|quote| quote.source.as_str().to_string()),
            value: position.value.map(|value| value.amount_string()),
            currency: currency.to_string(),
        }
    }
}

/// Chain provider status.
#[derive(Debug, Serialize)]
pub struct ChainProviderStatusDto {
    /// Whether a provider is configured.
    pub configured: bool,
    /// Provider name.
    pub name: Option<String>,
}

/// Query for positions.
#[derive(Debug, Deserialize)]
pub struct PositionQuery {
    /// ISO valuation date; defaults to today.
    #[serde(default)]
    pub as_of: Option<String>,
}

/// `GET /api/v1/crypto/assets`
pub async fn list_assets(
    State(state): State<AppState>,
    _current: CurrentUser,
) -> Result<Json<Vec<CryptoAssetDto>>, ApiError> {
    let assets = state.crypto.list_assets().await?;
    Ok(Json(assets.into_iter().map(CryptoAssetDto::from).collect()))
}

/// `POST /api/v1/crypto/assets`
pub async fn create_asset(
    State(state): State<AppState>,
    current: CurrentUser,
    Json(request): Json<CreateCryptoAssetRequest>,
) -> Result<(StatusCode, Json<CryptoAssetDto>), ApiError> {
    require_writer(&current)?;
    let chain = request
        .chain
        .parse::<Chain>()
        .map_err(|error| ApiError::BadRequest(error.to_string()))?;
    let currency = match request.currency {
        Some(value) => {
            Currency::new(value).map_err(|error| ApiError::BadRequest(error.to_string()))?
        }
        None => Currency::usd(),
    };
    let asset = state
        .crypto
        .create_asset(NewCryptoAsset {
            symbol: request.symbol,
            name: request.name,
            chain,
            contract_address: request.contract_address,
            decimals: request.decimals.unwrap_or(18),
            currency,
            price_provider_id: request.price_provider_id,
        })
        .await?;
    Ok((StatusCode::CREATED, Json(CryptoAssetDto::from(asset))))
}

/// `GET /api/v1/crypto/wallets`
pub async fn list_wallets(
    State(state): State<AppState>,
    current: CurrentUser,
) -> Result<Json<Vec<CryptoWalletDto>>, ApiError> {
    let wallets = state.crypto.list_wallets().await?;
    Ok(Json(
        wallets
            .into_iter()
            .filter(|wallet| {
                current
                    .user
                    .can_see(wallet.ownership_scope, wallet.owner_user_id)
            })
            .map(CryptoWalletDto::from)
            .collect(),
    ))
}

/// `POST /api/v1/crypto/wallets`
pub async fn create_wallet(
    State(state): State<AppState>,
    current: CurrentUser,
    Json(request): Json<CreateCryptoWalletRequest>,
) -> Result<(StatusCode, Json<CryptoWalletDto>), ApiError> {
    require_writer(&current)?;
    let kind = request
        .kind
        .parse::<CryptoWalletKind>()
        .map_err(|error| ApiError::BadRequest(error.to_string()))?;
    let chain = request
        .chain
        .parse::<Chain>()
        .map_err(|error| ApiError::BadRequest(error.to_string()))?;
    let ownership_scope = household_only(request.ownership_scope.as_deref())?;
    let wallet = state
        .crypto
        .create_wallet(NewCryptoWallet {
            asset_id: request.asset_id,
            kind,
            chain,
            address: request.address,
            label: request.label,
            ownership_scope,
            owner_user_id: None,
        })
        .await?;
    Ok((StatusCode::CREATED, Json(CryptoWalletDto::from(wallet))))
}

async fn visible_wallet(
    state: &AppState,
    current: &CurrentUser,
    wallet_id: Uuid,
) -> Result<CryptoWallet, ApiError> {
    let wallet = state
        .crypto
        .list_wallets()
        .await?
        .into_iter()
        .find(|wallet| wallet.id == wallet_id)
        .ok_or_else(|| ApiError::NotFound(format!("wallet not found: {wallet_id}")))?;
    if !current
        .user
        .can_see(wallet.ownership_scope, wallet.owner_user_id)
    {
        return Err(ApiError::NotFound(format!("wallet not found: {wallet_id}")));
    }
    Ok(wallet)
}

/// `GET /api/v1/crypto/wallets/{id}/balances`
pub async fn list_balances(
    State(state): State<AppState>,
    Path(wallet_id): Path<Uuid>,
    current: CurrentUser,
) -> Result<Json<Vec<CryptoBalanceDto>>, ApiError> {
    visible_wallet(&state, &current, wallet_id).await?;
    let balances = state.crypto.list_balances(wallet_id).await?;
    Ok(Json(
        balances.into_iter().map(CryptoBalanceDto::from).collect(),
    ))
}

/// `POST /api/v1/crypto/wallets/{id}/balances`
pub async fn record_balance(
    State(state): State<AppState>,
    Path(wallet_id): Path<Uuid>,
    current: CurrentUser,
    Json(request): Json<RecordCryptoBalanceRequest>,
) -> Result<(StatusCode, Json<CryptoBalanceDto>), ApiError> {
    require_writer(&current)?;
    visible_wallet(&state, &current, wallet_id).await?;
    let as_of = parse_date(&request.as_of)?;
    let quantity = Quantity::parse(&request.quantity)
        .map_err(|error| ApiError::BadRequest(error.to_string()))?;
    let balance = state
        .crypto
        .record_balance(wallet_id, as_of, quantity)
        .await?;
    Ok((StatusCode::CREATED, Json(CryptoBalanceDto::from(balance))))
}

/// `POST /api/v1/crypto/wallets/{id}/sync`
pub async fn sync_wallet(
    State(state): State<AppState>,
    Path(wallet_id): Path<Uuid>,
    current: CurrentUser,
) -> Result<Json<CryptoBalanceDto>, ApiError> {
    require_writer(&current)?;
    visible_wallet(&state, &current, wallet_id).await?;
    let balance = state.crypto.sync_wallet(wallet_id).await?;
    Ok(Json(CryptoBalanceDto::from(balance)))
}

/// `GET /api/v1/crypto/positions`
pub async fn list_positions(
    State(state): State<AppState>,
    Query(query): Query<PositionQuery>,
    _current: CurrentUser,
) -> Result<Json<Vec<CryptoPositionDto>>, ApiError> {
    let as_of = match query.as_of.as_deref() {
        Some(value) => parse_date(value)?,
        None => time::OffsetDateTime::now_utc().date(),
    };
    let currency = Currency::usd();
    let positions = state.crypto.positions(as_of, &currency).await?;
    Ok(Json(
        positions
            .into_iter()
            .map(|position| CryptoPositionDto::from_position(position, &currency))
            .collect(),
    ))
}

/// `GET /api/v1/crypto/provider`
pub async fn provider_status(
    State(state): State<AppState>,
    _current: CurrentUser,
) -> Json<ChainProviderStatusDto> {
    let name = state.crypto.provider_name();
    Json(ChainProviderStatusDto {
        configured: name.is_some(),
        name: name.map(str::to_string),
    })
}

/// Parse a date used by the crypto API.
#[allow(dead_code)]
fn parse_crypto_date(value: &str) -> Result<Date, ApiError> {
    parse_date(value)
}

#[allow(dead_code)]
fn _error_type_check(_: CryptoError) {}

/// `GET /api/v1/crypto/wallets/{id}/lots`
pub async fn list_lots(
    State(state): State<AppState>,
    Path(wallet_id): Path<Uuid>,
    current: CurrentUser,
) -> Result<Json<Vec<crate::investments::LotDto>>, ApiError> {
    visible_wallet(&state, &current, wallet_id).await?;
    let lots = state.crypto.list_lots(wallet_id).await?;
    Ok(Json(
        lots.into_iter()
            .map(crate::investments::LotDto::from)
            .collect(),
    ))
}

/// `POST /api/v1/crypto/wallets/{id}/lots`
pub async fn create_lot(
    State(state): State<AppState>,
    Path(wallet_id): Path<Uuid>,
    current: CurrentUser,
    Json(request): Json<crate::investments::LotRequest>,
) -> Result<(StatusCode, Json<Vec<crate::investments::LotDto>>), ApiError> {
    require_writer(&current)?;
    visible_wallet(&state, &current, wallet_id).await?;
    let lot = crate::investments::parse_lot(crate::investments::RawLot {
        acquired_on: &request.acquired_on,
        quantity: &request.quantity,
        cost_basis: &request.cost_basis,
        currency: request.currency.as_deref(),
        fees: request.fees.as_deref(),
        source: request.source.as_deref(),
        basis_quality: request.basis_quality.as_deref(),
        basis_note: request.basis_note.as_deref(),
    })
    .map_err(ApiError::BadRequest)?;
    let lots = state.crypto.append_lots(wallet_id, vec![lot]).await?;
    Ok((
        StatusCode::CREATED,
        Json(
            lots.into_iter()
                .map(crate::investments::LotDto::from)
                .collect(),
        ),
    ))
}

/// `GET /api/v1/crypto/wallets/{id}/sales`
pub async fn list_sales(
    State(state): State<AppState>,
    Path(wallet_id): Path<Uuid>,
    Query(query): Query<crate::investments::SaleQuery>,
    current: CurrentUser,
) -> Result<Json<Vec<crate::investments::SaleDto>>, ApiError> {
    visible_wallet(&state, &current, wallet_id).await?;
    let rule = crate::investments::long_term_rule(query.long_term_years, query.long_term_days)?;
    let sales = state.crypto.list_sales(wallet_id).await?;
    Ok(Json(
        sales
            .into_iter()
            .map(|sale| crate::investments::SaleDto::from_sale(sale, rule))
            .collect(),
    ))
}

/// `POST /api/v1/crypto/wallets/{id}/sales`
pub async fn create_sale(
    State(state): State<AppState>,
    Path(wallet_id): Path<Uuid>,
    current: CurrentUser,
    Json(request): Json<crate::investments::SaleRequest>,
) -> Result<(StatusCode, Json<crate::investments::SaleDto>), ApiError> {
    require_writer(&current)?;
    visible_wallet(&state, &current, wallet_id).await?;
    let rule = crate::investments::long_term_rule(request.long_term_years, request.long_term_days)?;
    let disposed_on = Date::parse(
        &request.disposed_on,
        &time::format_description::well_known::Iso8601::DATE,
    )
    .map_err(|error| ApiError::BadRequest(format!("invalid disposed_on date: {error}")))?;
    let quantity = Quantity::parse(&request.quantity)
        .map_err(|error| ApiError::BadRequest(error.to_string()))?;
    let currency = match request.currency {
        Some(code) => {
            Currency::new(code).map_err(|error| ApiError::BadRequest(error.to_string()))?
        }
        None => Currency::usd(),
    };
    let gross_proceeds = networth_domain::Money::parse(&request.gross_proceeds, currency.as_str())
        .map_err(|error| ApiError::BadRequest(error.to_string()))?;
    let fees = match request.fees.as_deref() {
        Some(fees) => networth_domain::Money::parse(fees, currency.as_str())
            .map_err(|error| ApiError::BadRequest(error.to_string()))?,
        None => networth_domain::Money::zero(currency.clone()),
    };
    let sale = state
        .crypto
        .record_sale(
            wallet_id,
            networth_domain::NewSale {
                disposed_on,
                quantity,
                gross_proceeds,
                fees,
                currency,
                method: networth_domain::SaleMethod::Fifo,
            },
        )
        .await?;
    Ok((
        StatusCode::CREATED,
        Json(crate::investments::SaleDto::from_sale(sale, rule)),
    ))
}
