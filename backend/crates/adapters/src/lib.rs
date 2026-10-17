//! Adapter layer for NetworthTracker.
//!
//! Implementations of outbound ports live here: the Postgres repositories,
//! price/chain/aggregation providers, spreadsheet readers and writers, and the
//! HTTP transport. Adapters depend inward on the domain and application; the
//! domain never depends on them.
