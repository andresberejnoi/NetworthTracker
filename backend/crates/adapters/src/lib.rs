//! Adapter layer for NetworthTracker.
//!
//! Implementations of outbound ports live here: the Postgres repositories,
//! price/chain/aggregation providers, spreadsheet readers and writers, and the
//! HTTP transport. Adapters depend inward on the domain and application; the
//! domain never depends on them.

pub mod memory;
pub mod postgres;
pub mod price_provider;
pub mod spreadsheet;

pub use memory::InMemoryAccountRepository;
pub use postgres::PostgresAccountRepository;
pub use price_provider::StaticPriceProvider;
pub use spreadsheet::{read_table, write_table, SpreadsheetError, SpreadsheetFormat};
