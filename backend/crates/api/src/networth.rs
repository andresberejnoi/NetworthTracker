//! Net worth and snapshot HTTP handlers.

use std::str::FromStr;

use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use bigdecimal::BigDecimal;
use networth_application::NetWorthRequest;
use networth_domain::{AggregationScope, Currency, RedemptionPolicy, SnapshotFilter};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::accounts::parse_date;
use crate::dto::{
    NetWorthDto, NetWorthQuery, PolicyParams, SnapshotDto, SnapshotListQuery, SnapshotRequest,
};
use crate::error::ApiError;
use crate::AppState;

/// `GET /api/v1/networth`
pub async fn get_net_worth(
    State(state): State<AppState>,
    Query(query): Query<NetWorthQuery>,
) -> Result<Json<NetWorthDto>, ApiError> {
    let request = build_request(
        query.as_of.as_deref(),
        query.scope.as_deref(),
        query.owner_user_id,
        query.owner_age,
        &query.policy,
    )?;

    let breakdown = state.networth.compute(request).await?;
    Ok(Json(NetWorthDto::from(breakdown)))
}

/// `POST /api/v1/networth/snapshots`
pub async fn create_snapshot(
    State(state): State<AppState>,
    Json(body): Json<SnapshotRequest>,
) -> Result<(StatusCode, Json<SnapshotDto>), ApiError> {
    let request = build_request(
        body.as_of.as_deref(),
        body.scope.as_deref(),
        body.owner_user_id,
        body.owner_age,
        &body.policy,
    )?;

    let snapshot = state.networth.take_snapshot(request).await?;
    Ok((StatusCode::CREATED, Json(SnapshotDto::from(snapshot))))
}

/// `GET /api/v1/networth/snapshots`
pub async fn list_snapshots(
    State(state): State<AppState>,
    Query(query): Query<SnapshotListQuery>,
) -> Result<Json<Vec<SnapshotDto>>, ApiError> {
    let filter = SnapshotFilter {
        scope: match query.scope.as_deref() {
            Some(value) => Some(value.parse::<AggregationScope>().map_err(
                |error: networth_domain::networth::AggregationScopeParseError| {
                    ApiError::BadRequest(error.to_string())
                },
            )?),
            None => None,
        },
        owner_user_id: query.owner_user_id,
    };

    let snapshots = state.networth.list_snapshots(filter).await?;
    Ok(Json(snapshots.into_iter().map(SnapshotDto::from).collect()))
}

fn build_request(
    as_of: Option<&str>,
    scope: Option<&str>,
    owner_user_id: Option<Uuid>,
    owner_age: Option<u32>,
    policy: &PolicyParams,
) -> Result<NetWorthRequest, ApiError> {
    let as_of = match as_of {
        Some(value) => parse_date(value)?,
        None => OffsetDateTime::now_utc().date(),
    };

    let scope = match scope {
        Some(value) => value.parse::<AggregationScope>().map_err(
            |error: networth_domain::networth::AggregationScopeParseError| {
                ApiError::BadRequest(error.to_string())
            },
        )?,
        None => AggregationScope::Household,
    };

    Ok(NetWorthRequest {
        as_of,
        scope,
        owner_user_id,
        owner_age,
        policy: policy_from_params(policy)?,
        currency: Currency::usd(),
    })
}

fn policy_from_params(params: &PolicyParams) -> Result<RedemptionPolicy, ApiError> {
    let default = RedemptionPolicy::default();

    fn parse(value: &Option<String>, fallback: &BigDecimal) -> Result<BigDecimal, ApiError> {
        match value {
            Some(raw) => BigDecimal::from_str(raw.trim())
                .map_err(|error| ApiError::BadRequest(format!("invalid decimal '{raw}': {error}"))),
            None => Ok(fallback.clone()),
        }
    }

    Ok(RedemptionPolicy {
        tax_rate: parse(&params.tax_rate, &default.tax_rate)?,
        early_withdrawal_penalty_rate: parse(
            &params.penalty_rate,
            &default.early_withdrawal_penalty_rate,
        )?,
        early_withdrawal_before_age: params
            .penalty_age
            .unwrap_or(default.early_withdrawal_before_age),
        selling_cost_rate: parse(&params.selling_rate, &default.selling_cost_rate)?,
        fixed_costs: parse(&params.fixed_costs, &default.fixed_costs)?,
    })
}
