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
use crate::auth::CurrentUser;
use crate::dto::{
    NetWorthDto, NetWorthQuery, PolicyParams, SnapshotDto, SnapshotListQuery, SnapshotRequest,
};
use crate::error::ApiError;
use crate::AppState;

/// `GET /api/v1/networth`
pub async fn get_net_worth(
    State(state): State<AppState>,
    Query(query): Query<NetWorthQuery>,
    current: CurrentUser,
) -> Result<Json<NetWorthDto>, ApiError> {
    let request = build_request(
        query.as_of.as_deref(),
        query.scope.as_deref(),
        query.owner_user_id,
        query.owner_age,
        &query.policy,
        &current,
    )?;

    let breakdown = state.networth.compute(request).await?;
    Ok(Json(NetWorthDto::from(breakdown)))
}

/// `POST /api/v1/networth/snapshots`
pub async fn create_snapshot(
    State(state): State<AppState>,
    current: CurrentUser,
    Json(body): Json<SnapshotRequest>,
) -> Result<(StatusCode, Json<SnapshotDto>), ApiError> {
    if !current.user.role.can_write() {
        return Err(ApiError::Forbidden(
            "you do not have permission to create snapshots".to_string(),
        ));
    }

    let request = build_request(
        body.as_of.as_deref(),
        body.scope.as_deref(),
        body.owner_user_id,
        body.owner_age,
        &body.policy,
        &current,
    )?;

    let snapshot = state.networth.take_snapshot(request).await?;
    Ok((StatusCode::CREATED, Json(SnapshotDto::from(snapshot))))
}

/// `GET /api/v1/networth/snapshots`
pub async fn list_snapshots(
    State(state): State<AppState>,
    Query(query): Query<SnapshotListQuery>,
    current: CurrentUser,
) -> Result<Json<Vec<SnapshotDto>>, ApiError> {
    let scope = match query.scope.as_deref() {
        Some(value) => Some(value.parse::<AggregationScope>().map_err(
            |error: networth_domain::networth::AggregationScopeParseError| {
                ApiError::BadRequest(error.to_string())
            },
        )?),
        None => Some(AggregationScope::Household),
    };

    if scope == Some(AggregationScope::Personal) {
        let target = query.owner_user_id.unwrap_or(current.user.id);
        if target != current.user.id && !current.user.role.is_owner() {
            return Err(ApiError::Forbidden(
                "you cannot view another user's snapshots".to_string(),
            ));
        }
    }

    let filter = SnapshotFilter {
        scope,
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
    current: &CurrentUser,
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

    let (target, viewer_user_id, viewer_is_owner) = match scope {
        AggregationScope::Household => (None, Some(current.user.id), current.user.role.is_owner()),
        AggregationScope::Personal => {
            let target = owner_user_id.unwrap_or(current.user.id);
            if target != current.user.id && !current.user.role.is_owner() {
                return Err(ApiError::Forbidden(
                    "you cannot view another user's net worth".to_string(),
                ));
            }
            (Some(target), Some(target), false)
        }
    };

    Ok(NetWorthRequest {
        as_of,
        scope,
        owner_user_id: target,
        owner_age,
        policy: policy_from_params(policy)?,
        currency: Currency::usd(),
        viewer_user_id,
        viewer_is_owner,
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
