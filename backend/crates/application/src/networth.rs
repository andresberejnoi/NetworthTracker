//! Net worth use cases.

use std::sync::Arc;

use networth_domain::{
    compute_net_worth, AggregationScope, ComputeParams, Currency, NetWorthBreakdown, NewSnapshot,
    RedemptionPolicy, Snapshot, SnapshotFilter,
};
use time::Date;
use uuid::Uuid;

use crate::ports::{AccountRepository, NetWorthRepository, RepositoryError};

/// Errors raised by net worth use cases.
#[derive(Debug, thiserror::Error)]
pub enum NetWorthError {
    /// Input failed validation.
    #[error("{0}")]
    Validation(String),
    /// Persistence failed.
    #[error(transparent)]
    Repository(#[from] RepositoryError),
}

/// Inputs for computing or snapshotting net worth.
#[derive(Debug, Clone)]
pub struct NetWorthRequest {
    /// The valuation date.
    pub as_of: Date,
    /// Household or personal view.
    pub scope: AggregationScope,
    /// The user for a personal view.
    pub owner_user_id: Option<Uuid>,
    /// The owner's age, used for early-withdrawal penalties.
    pub owner_age: Option<u32>,
    /// The reductions applied to accessible net worth.
    pub policy: RedemptionPolicy,
    /// The reporting currency.
    pub currency: Currency,
}

/// Net worth computation and snapshot persistence.
pub struct NetWorthService {
    accounts: Arc<dyn AccountRepository>,
    networth: Arc<dyn NetWorthRepository>,
}

impl NetWorthService {
    /// Build the service over the account and net worth repositories.
    pub fn new(
        accounts: Arc<dyn AccountRepository>,
        networth: Arc<dyn NetWorthRepository>,
    ) -> Self {
        Self { accounts, networth }
    }

    /// Compute net worth as of the request's date.
    pub async fn compute(
        &self,
        request: NetWorthRequest,
    ) -> Result<NetWorthBreakdown, NetWorthError> {
        validate(&request)?;

        let accounts = self.accounts.list_accounts(true).await?;
        let balances = self.networth.latest_balances(request.as_of).await?;

        let params = ComputeParams {
            as_of: request.as_of,
            scope: request.scope,
            owner_user_id: request.owner_user_id,
            owner_age: request.owner_age,
            policy: request.policy,
            currency: request.currency,
        };

        Ok(compute_net_worth(&accounts, &balances, &params))
    }

    /// Compute net worth and persist an immutable snapshot.
    pub async fn take_snapshot(&self, request: NetWorthRequest) -> Result<Snapshot, NetWorthError> {
        let breakdown = self.compute(request.clone()).await?;

        let new = NewSnapshot {
            as_of: request.as_of,
            scope: request.scope,
            owner_user_id: request.owner_user_id,
            owner_age: request.owner_age,
            currency: request.currency,
            total: breakdown.total,
            liquid: breakdown.liquid,
            liquidity: breakdown.liquidity,
            accessible: breakdown.accessible,
            policy: request.policy,
        };

        Ok(self.networth.insert_snapshot(&new).await?)
    }

    /// List stored snapshots, oldest first.
    pub async fn list_snapshots(
        &self,
        filter: SnapshotFilter,
    ) -> Result<Vec<Snapshot>, NetWorthError> {
        Ok(self.networth.list_snapshots(&filter).await?)
    }
}

fn validate(request: &NetWorthRequest) -> Result<(), NetWorthError> {
    if request.scope == AggregationScope::Personal && request.owner_user_id.is_none() {
        return Err(NetWorthError::Validation(
            "a personal view requires owner_user_id".to_string(),
        ));
    }
    Ok(())
}
