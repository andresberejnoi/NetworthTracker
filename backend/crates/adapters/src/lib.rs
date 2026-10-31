//! Adapter layer for NetworthTracker.
//!
//! Implementations of outbound ports live here: the Postgres repositories,
//! price/chain/aggregation providers, spreadsheet readers and writers, and the
//! HTTP transport. Adapters depend inward on the domain and application; the
//! domain never depends on them.

pub mod aggregation_provider;
pub mod chain_provider;
pub mod coinpaprika;
pub mod memory;
pub mod postgres;
pub mod price_provider;
pub mod public_chain;
pub mod spreadsheet;

pub use aggregation_provider::FakeAggregationProvider;
pub use chain_provider::FakeChainProvider;
pub use coinpaprika::CoinPaprikaPriceProvider;
pub use memory::InMemoryAccountRepository;
pub use postgres::PostgresAccountRepository;
pub use price_provider::StaticPriceProvider;
pub use public_chain::PublicChainProvider;
pub use spreadsheet::{read_table, write_table, SpreadsheetError, SpreadsheetFormat};
