//! Optional read-only aggregation config, sync, and provenance.

use axum::{extract::State, Json};
use networth_application::{AggregationLinkView, AggregationStatus, AggregationView};
use serde::{Deserialize, Serialize};

use crate::auth::CurrentUser;
use crate::error::ApiError;
use crate::AppState;

fn require_owner(current: &CurrentUser) -> Result<(), ApiError> {
    if current.user.role.is_owner() {
        Ok(())
    } else {
        Err(ApiError::Forbidden(
            "only the household owner can manage aggregation".to_string(),
        ))
    }
}

/// Aggregation status in the API.
#[derive(Debug, Serialize)]
pub struct AggregationStatusDto {
    /// Provider.
    pub provider: Option<String>,
    /// Enabled.
    pub enabled: bool,
    /// Whether a token is stored.
    pub has_token: bool,
    /// Whether the adapter is available.
    pub provider_available: bool,
}

impl From<AggregationStatus> for AggregationStatusDto {
    fn from(value: AggregationStatus) -> Self {
        Self {
            provider: value.provider,
            enabled: value.enabled,
            has_token: value.has_token,
            provider_available: value.provider_available,
        }
    }
}

/// A provider holding in the API.
#[derive(Debug, Serialize)]
pub struct AggregationHoldingDto {
    /// Symbol.
    pub symbol: String,
    /// Quantity.
    pub quantity: String,
    /// Value.
    pub value: String,
    /// Currency.
    pub currency: String,
    /// Sync time.
    pub synced_at: String,
}

/// A linked account in the API.
#[derive(Debug, Serialize)]
pub struct AggregationLinkDto {
    /// Link.
    pub id: uuid::Uuid,
    /// Provider.
    pub provider: String,
    /// Provider account id.
    pub provider_account_id: String,
    /// Local account.
    pub account_id: uuid::Uuid,
    /// Display name.
    pub display_name: String,
    /// Account type.
    pub account_type: String,
    /// Currency.
    pub currency: String,
    /// Last sync.
    pub last_synced_at: Option<String>,
    /// Latest provider balance.
    pub provider_balance: Option<String>,
    /// Provider balance date.
    pub provider_as_of: Option<String>,
    /// Latest local balance.
    pub local_balance: Option<String>,
    /// Local balance source.
    pub local_source: Option<String>,
    /// Which value is in effect.
    pub in_effect: String,
    /// Provider holdings.
    pub holdings: Vec<AggregationHoldingDto>,
}

impl From<AggregationLinkView> for AggregationLinkDto {
    fn from(value: AggregationLinkView) -> Self {
        Self {
            id: value.link.id,
            provider: value.link.provider,
            provider_account_id: value.link.provider_account_id,
            account_id: value.link.account_id,
            display_name: value.link.display_name,
            account_type: value.link.account_type.as_str().to_string(),
            currency: value.link.currency.to_string(),
            last_synced_at: value.link.last_synced_at.map(|value| value.to_string()),
            provider_balance: value
                .provider_balance
                .as_ref()
                .map(|balance| balance.amount.amount_string()),
            provider_as_of: value
                .provider_balance
                .as_ref()
                .map(|balance| balance.as_of.to_string()),
            local_balance: value
                .local_balance
                .as_ref()
                .map(|balance| balance.amount_string()),
            local_source: value.local_source.map(|source| source.as_str().to_string()),
            in_effect: value.in_effect,
            holdings: value
                .holdings
                .into_iter()
                .map(|holding| AggregationHoldingDto {
                    symbol: holding.symbol,
                    quantity: holding.quantity.to_string(),
                    value: holding.value.amount_string(),
                    currency: holding.value.currency().to_string(),
                    synced_at: holding.synced_at.to_string(),
                })
                .collect(),
        }
    }
}

/// The aggregation view.
#[derive(Debug, Serialize)]
pub struct AggregationViewDto {
    /// Status.
    pub status: AggregationStatusDto,
    /// Links.
    pub links: Vec<AggregationLinkDto>,
}

impl From<AggregationView> for AggregationViewDto {
    fn from(value: AggregationView) -> Self {
        Self {
            status: value.status.into(),
            links: value
                .links
                .into_iter()
                .map(AggregationLinkDto::from)
                .collect(),
        }
    }
}

/// Config request.
#[derive(Debug, Deserialize)]
pub struct AggregationConfigRequest {
    /// Provider name.
    pub provider: String,
    /// Whether sync is enabled.
    pub enabled: bool,
    /// Provider token; omit to keep the existing token.
    #[serde(default)]
    pub token: Option<String>,
}

/// `GET /api/v1/aggregation`
pub async fn get_aggregation(
    State(state): State<AppState>,
    current: CurrentUser,
) -> Result<Json<AggregationViewDto>, ApiError> {
    require_owner(&current)?;
    Ok(Json(state.aggregation.view().await?.into()))
}

/// `POST /api/v1/aggregation/config`
pub async fn save_config(
    State(state): State<AppState>,
    current: CurrentUser,
    Json(request): Json<AggregationConfigRequest>,
) -> Result<Json<AggregationStatusDto>, ApiError> {
    require_owner(&current)?;
    let status = state
        .aggregation
        .save_config(&request.provider, request.enabled, request.token.as_deref())
        .await?;
    Ok(Json(status.into()))
}

/// `POST /api/v1/aggregation/sync`
pub async fn sync(
    State(state): State<AppState>,
    current: CurrentUser,
) -> Result<Json<AggregationViewDto>, ApiError> {
    require_owner(&current)?;
    Ok(Json(state.aggregation.sync().await?.into()))
}
