//! Application layer for NetworthTracker.
//!
//! Use cases and orchestration live here. It coordinates the domain through
//! repository and provider traits; it owns transactions but not transport or
//! persistence details. It must not depend on Axum or on a specific database.

pub mod accounts;
pub mod aggregation;
pub mod auth;
pub mod backup;
pub mod budget;
pub mod crypto;
pub mod data_exchange;
pub mod investments;
pub mod market_data;
pub mod networth;
pub mod ports;

pub use accounts::{
    AccountError, AccountService, BatchMode, BatchRow, BatchSave, CreateAccount, RecordBalance,
    RowError,
};
pub use aggregation::{
    AggregationError, AggregationLinkView, AggregationService, AggregationStatus, AggregationView,
    TokenCipher,
};
pub use auth::{implicit_user, AuthError, AuthService, AuthStatus, Session};
pub use backup::{
    Backup, BackupAccount, BackupAggregationBalance, BackupAggregationHolding,
    BackupAggregationLink, BackupBalance, BackupBudgetAllocation, BackupBudgetIncome,
    BackupBudgetPlan, BackupBudgetSpending, BackupCryptoAsset, BackupCryptoBalance,
    BackupCryptoWallet, BackupDisposal, BackupError, BackupHolding, BackupLot, BackupSale,
    BackupSecurity, BackupService, BackupSnapshot, RestoreReport, BACKUP_VERSION,
    V1_BACKUP_VERSION,
};
pub use budget::{BudgetAllocationView, BudgetError, BudgetService, BudgetView};
pub use crypto::{validate_public_address, CryptoError, CryptoPosition, CryptoService};
pub use data_exchange::{
    ImportError, ImportReport, ImportRowError, ImportService, Table, COLUMNS, SCHEMA_VERSION,
};
pub use investments::{HoldingValuation, InvestmentError, InvestmentsService};
pub use market_data::{
    FxError, FxService, PriceError, PriceService, ProviderConfig, DEFAULT_MAX_AGE_SECS,
};
pub use networth::{NetWorthError, NetWorthRequest, NetWorthService};
pub use ports::{
    AccountRepository, AggregationBalance, AggregationConfig, AggregationHolding, AggregationLink,
    AggregationProvider, AggregationRepository, BudgetRepository, ChainBalance, ChainDataProvider,
    CryptoRepository, FxRepository, IdentityRepository, InvestmentRepository, NetWorthRepository,
    PriceProvider, PriceRepository, ProviderAccount, ProviderError, ProviderHolding, ProviderQuote,
    ProviderSnapshot, RepositoryError, StoredPrice, StoredRate, UserWithHash,
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
