# Tasks

Tasks are ordered by dependency and phased so a usable vertical slice (manual accounts → net worth →
history chart) ships before any external integration. Design open questions are all deferrable behind
adapter traits and do not change this breakdown.

## 1. Repository, toolchain, and deployment scaffold

- [ ] 1.1 Create the Cargo workspace (`crates/domain`, `crates/application`, `crates/adapters`, `crates/api`) and verify `cargo build --workspace` succeeds
- [ ] 1.2 Add an Axum service exposing `GET /health` and verify the endpoint returns 200 by running the binary and curling it
- [ ] 1.3 Add `docker-compose.yml` with the API service and a Postgres service plus a named volume, and verify `docker compose up` starts both and `/health` responds
- [ ] 1.4 Wire SQLx with a migrations directory and a startup migration runner, and verify a first migration applies cleanly against a fresh Postgres container
- [ ] 1.5 Add CI (build, `cargo test`, `cargo clippy -D warnings`, `cargo fmt --check`, and `openspec validate --strict`) and verify the pipeline passes on a trivial PR

## 2. Exact money and core value objects

- [ ] 2.1 Implement a `Money` value object (`bigdecimal` amount + currency) and verify unit tests covering addition, subtraction, and non-lossy large/precise values
- [ ] 2.2 Implement `Quantity` for crypto-style assets with at least 18 decimals and verify a test storing `0.000000000000000001` round-trips unchanged
- [ ] 2.3 Serialize all decimal amounts as JSON strings and verify a serde round-trip test proves no precision loss and that no float type appears in the money API
- [ ] 2.4 Add property-based tests for money arithmetic (associativity, round-trip, no rounding drift) and verify they pass
- [ ] 2.5 Document the money rules (no floats, exact decimals, currency tagging) in `docs/money.md` and verify the examples in it compile as doctests

## 3. Account registry and balance persistence

- [ ] 3.1 Implement the account type taxonomy with asset/liability and liquidity classification in `crates/domain` and verify tests cover every type from the `accounts` spec
- [ ] 3.2 Write migrations for accounts, ownership scope, and dated balance records and verify they apply and roll back against Postgres
- [ ] 3.3 Define the `AccountRepository` trait in `application` and implement the Postgres adapter, verified by repository integration tests
- [ ] 3.4 Implement create-account and record-balance use cases and verify a test that runs them with no network access succeeds
- [ ] 3.5 Implement account archival that preserves history and verify a test shows the account leaves current balances but remains in history
- [ ] 3.6 Expose account and balance HTTP endpoints and verify them with API-level tests covering success and validation errors
- [ ] 3.7 Document the account and balance API in `docs/api.md` and verify the documented requests against a running instance

## 4. Net worth engine and history

- [ ] 4.1 Implement total net worth (assets minus liabilities, reporting currency) and verify tests for the `networth` spec scenarios
- [ ] 4.2 Implement liquid net worth filtering by liquidity class and verify tests prove illiquid and retirement assets are excluded
- [ ] 4.3 Implement the `RedemptionPolicy` model with percentage and fixed-amount cost components, a single user-set tax rate, and age-banded penalties, and verify tests for the retirement-discount and composited-selling-cost scenarios
- [ ] 4.4 Implement accessible net worth from redemption policies and verify tests for the accessible-net-worth scenarios
- [ ] 4.5 Implement what-if scenario modeling that projects accessible net worth without persisting until applied, and verify tests for the model-without-saving and apply scenarios
- [ ] 4.6 Implement household and per-user aggregation scope with no double counting of shared accounts and verify tests for the shared-account scenario
- [ ] 4.7 Persist snapshots capturing total, liquid, and accessible values with the policies and inputs used, and verify tests that a later data or rule change does not alter an existing snapshot until an explicit recompute
- [ ] 4.8 Expose snapshot and net worth calculation endpoints and verify API tests for each net worth form

## 5. Bulk data entry grid

- [ ] 5.1 Implement a reusable grid component with one row per record, inline fields, and an add-row control, and verify a component test enters several rows without opening a form
- [ ] 5.2 Implement keyboard-first navigation and add-row-on-enter and verify a test completes a run of rows using only the keyboard
- [ ] 5.3 Implement clipboard paste that splits tab-separated data into rows and columns and verify a test pasting twenty rows yields twenty populated grid rows
- [ ] 5.4 Implement inline per-row validation using the exact-decimal parser and verify invalid rows are flagged with reasons without affecting valid rows
- [ ] 5.5 Implement single-action bulk save with an atomic-or-partial policy and a per-row result report, and verify tests for all-valid and partial-failure saves
- [ ] 5.6 Wire the grid to balances, holdings, and wallet addresses and verify each entity type can be bulk entered end to end
- [ ] 5.7 Document bulk entry and paste in `docs/bulk-entry.md` and verify the documented steps reproduce a twenty-wallet paste

## 6. Spreadsheet import, export, backup, and analytics access

- [ ] 6.1 Define the versioned CSV template schema and implement a validating parser and verify row-level error tests (unknown type, bad amount) with no partial writes
- [ ] 6.2 Implement idempotent apply keyed on a stable column and verify a test importing the same file twice produces no duplicates
- [ ] 6.3 Implement XLSX read and write for the same schema and verify an XLSX round-trip test matches the CSV result
- [ ] 6.4 Implement full export of accounts, balances, holdings, and configuration and verify exported files re-import without change
- [ ] 6.5 Implement local backup and restore and verify restoring on a clean instance with networking disabled reproduces all data and history
- [ ] 6.6 Verify exact-decimal round-trip through import/export/backup with a test using many-decimal and very large values
- [ ] 6.7 Document an optional read-only analytics workflow (SQL or Python) against the database or exported CSV and verify a sample query runs against a seeded database
- [ ] 6.8 Publish the template and import rules in `docs/import-export.md` and verify the documented template imports as written

## 7. Dashboard vertical slice

- [ ] 7.1 Scaffold the Vite + React dashboard and serve its build from the Axum process and verify the SPA loads from the API origin
- [ ] 7.2 Implement the summary view showing total, liquid, and accessible net worth with period selection and verify component tests for period changes
- [ ] 7.3 Implement the net worth history chart with switchable net worth form and verify it renders from seeded snapshots
- [ ] 7.4 Implement filtering and breakdown by account type and asset class/sector and verify tests that filters change summary and chart data
- [ ] 7.5 Add stale/manual data indicators and verify a test with a cached price shows the stale marker
- [ ] 7.6 Verify the dashboard loads and displays data with networking disabled and document it in `docs/dashboard.md`

## 8. Identity, authentication, and access control

- [ ] 8.1 Implement household and user models with ownership scoping and verify migrations plus model tests
- [ ] 8.2 Implement Argon2id password auth with local sessions and a documented auth-disabled single-user mode and verify tests for both modes
- [ ] 8.3 Enforce owner/editor/viewer roles in the application layer and verify a test that a viewer cannot modify data
- [ ] 8.4 Enforce per-user data scoping so private accounts are hidden from other members and verify tests for the shared and private scenarios
- [ ] 8.5 Document authentication and role setup in `docs/access-control.md` and verify the documented flow works against a running instance

## 9. Investments and market data

- [ ] 9.1 Implement securities, positions, and tax lots with asset-class and sector classification and verify model tests including the multi-lot scenario
- [ ] 9.2 Implement holding and account valuation plus unrealized gain/loss and verify tests for both valuation scenarios
- [ ] 9.3 Implement the `PriceProvider` trait with caching, timestamped prices, offline fallback marked stale, and manual override, verified by tests using a fake provider
- [ ] 9.4 Implement the price add-on loader and user-supplied credential configuration (own API key or subscription, stored locally, inert until enabled) and verify a test that with no key configured the system uses stored prices
- [ ] 9.5 Implement FX rate storage and multi-currency conversion recording the applied rate and verify conversion tests
- [ ] 9.6 Document the market data add-on contract, bring-your-own-key setup, and caching/staleness rules in `docs/market-data.md` and verify the documented setup against the fake provider

## 10. Cryptocurrency tracking

- [ ] 10.1 Implement wallet address registration per chain and verify input validation rejects private keys and seed phrases
- [ ] 10.2 Implement the `ChainDataProvider` trait with a fake backend and derive address balances, verified by adapter tests
- [ ] 10.3 Verify high-precision crypto quantities survive storage, net worth aggregation, and JSON with an 18-decimal test
- [ ] 10.4 Implement offline manual crypto balances and verify a disconnected test includes them in net worth
- [ ] 10.5 Implement aggregation of multiple addresses of the same asset and verify the two-wallets-one-position scenario
- [ ] 10.6 Implement chain add-on enablement with user-supplied API keys and verify it is optional and inert when unconfigured
- [ ] 10.7 Document adding a chain add-on in `docs/crypto.md` and verify the steps by adding the fake chain end to end

## 11. Budgeting

- [ ] 11.1 Implement income recording per user and period and verify model plus API tests
- [ ] 11.2 Implement spending plan allocation with over-allocation detection and verify tests for balanced and over-allocated plans
- [ ] 11.3 Implement category spending records and plan-versus-actual variance reporting and verify variance tests in amount and percentage
- [ ] 11.4 Verify the app works fully with no budget configured and document budgeting in `docs/budgeting.md`

## 12. Read-only aggregation adapter interface

- [ ] 12.1 Define the `AggregationProvider` trait and a fake read-only adapter and verify no write capability exists in the trait surface
- [ ] 12.2 Implement provenance-tagged mapping of provider accounts, balances, and holdings and verify tests that the source and sync time are stored
- [ ] 12.3 Implement optional, non-blocking sync with graceful provider-failure handling and verify tests for unconfigured and unreachable providers
- [ ] 12.4 Implement user-supplied provider subscription configuration (own account or API key) with local encrypted token storage and verify no intermediary service is used and tokens never leave the machine
- [ ] 12.5 Implement provenance-preserving updates so provider syncs never silently discard manual values and verify the manual-value-retained scenario
- [ ] 12.6 Document the aggregation add-on contract and bring-your-own-subscription setup in `docs/aggregation.md` and verify the documented steps against the fake adapter

## 13. End-to-end integration verification

- [ ] 13.1 Run the full spec-scenario suite against a containerized stack and verify every scenario has a passing test
- [ ] 13.2 Verify a cold, fully offline run (no outbound network) supports account creation, bulk/grid entry, import, net worth, history, and dashboard
- [ ] 13.3 Verify `openspec validate networth-tracker-v1 --strict` passes and every capability in the proposal has a corresponding spec file

## Workflow follow-up

- Archive the change after the project's review requirements are satisfied.
- Verify the archived result updates the main specs under `openspec/specs/`.
