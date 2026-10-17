//! Pure domain layer for NetworthTracker.
//!
//! This crate holds the domain types and the net worth engine. It must not
//! depend on Axum, SQLx, any provider SDK, or the filesystem: everything here
//! is deterministic and testable without I/O.
//!
//! See the change design (`openspec/changes/networth-tracker-v1/design.md`,
//! decision D3) for the layering rules.

/// Human-readable application name used in service metadata.
pub const APP_NAME: &str = "NetworthTracker";
