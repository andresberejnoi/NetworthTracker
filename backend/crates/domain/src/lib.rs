#![doc = include_str!("../../../../docs/money.md")]

//! Pure domain layer for NetworthTracker.
//!
//! This crate holds the domain types and the net worth engine. It must not
//! depend on Axum, SQLx, any provider SDK, or the filesystem: everything here
//! is deterministic and testable without I/O.
//!
//! See the change design (`openspec/changes/networth-tracker-v1/design.md`,
//! decision D3) for the layering rules and `docs/money.md` for the money rules.

pub mod accounts;
pub mod currency;
pub mod money;
pub mod networth;
pub mod quantity;

mod decimal;

pub use accounts::{
    Account, AccountClassification, AccountType, Balance, BalanceSource, Liquidity, NewAccount,
    NewBalance, OwnershipScope,
};
pub use currency::Currency;
pub use money::{Money, MoneyError};
pub use networth::{
    compute_net_worth, AggregationScope, AssetClass, BreakdownRow, ComputeParams,
    NetWorthBreakdown, NetWorthKind, NewSnapshot, RedemptionPolicy, Snapshot, SnapshotFilter,
};
pub use quantity::{Quantity, QuantityError};

/// Human-readable application name used in service metadata.
pub const APP_NAME: &str = "NetworthTracker";
