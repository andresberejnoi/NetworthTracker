//! Application layer for NetworthTracker.
//!
//! Use cases and orchestration live here. It coordinates the domain through
//! repository and provider traits; it owns transactions but not transport or
//! persistence details. It must not depend on Axum or on a specific database.

pub mod accounts;
pub mod auth;
pub mod backup;
pub mod data_exchange;
pub mod investments;
pub mod market_data;
pub mod networth;
pub mod ports;

pub use accounts::{
    AccountError, AccountService, BatchMode, BatchRow, BatchSave, CreateAccount, RecordBalance,
    RowError,
};
pub use auth::{implicit_user, AuthError, AuthService, AuthStatus, Session};
pub use backup::{
    Backup, BackupAccount, BackupBalance, BackupError, BackupService, BackupSnapshot,
    RestoreReport, BACKUP_VERSION,
};
pub use data_exchange::{
    ImportError, ImportReport, ImportRowError, ImportService, Table, COLUMNS, SCHEMA_VERSION,
};
pub use investments::{HoldingValuation, InvestmentError, InvestmentsService};
pub use market_data::{
    FxError, FxService, PriceError, PriceService, ProviderConfig, DEFAULT_MAX_AGE_SECS,
};
pub use networth::{NetWorthError, NetWorthRequest, NetWorthService};
pub use ports::{
    AccountRepository, FxRepository, IdentityRepository, InvestmentRepository, NetWorthRepository,
    PriceProvider, PriceRepository, ProviderError, ProviderQuote, RepositoryError, StoredPrice,
    StoredRate, UserWithHash,
};

use networth_domain::APP_NAME;

/// Name of the service, exposed through the API's metadata endpoints.
pub fn service_name() -> &'static str {
    APP_NAME
}

/// Version of the backend, exposed through the API's metadata endpoints.
pub fn service_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
