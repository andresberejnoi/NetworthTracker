# Tasks

Tasks are vertical slices. Every group that adds API capability also adds its frontend consumer and a
cross-boundary integration test in the same group. `backend/` and `frontend/` are decoupled: no
cross-project imports, integration only over the HTTP API, and each project ships its own container.

## 1. Monorepo, decoupling, and container scaffold

- [x] 1.1 Create the `backend/` Cargo workspace (`domain`, `application`, `adapters`, `api`) and verify `cargo build --workspace` succeeds
- [x] 1.2 Add `backend/Dockerfile` and verify the built image runs the API and answers `GET /health`
- [x] 1.3 Scaffold `frontend/` as a Vite + React + TypeScript app and verify `npm run build` succeeds
- [x] 1.4 Add `frontend/Dockerfile` and verify the built image serves the SPA
- [x] 1.5 Add root `docker-compose.yml` running `db`, `backend`, and `frontend` and verify all three start and the frontend can reach the backend
- [x] 1.6 Add a root `.gitignore`, a task runner, and a README documenting the layout and the no-cross-project-import rule, and verify the documented commands run as written
- [x] 1.7 Add CI that builds, tests, and lints both projects, runs a boundary check (backend must not reference `frontend/` and vice versa), and runs `openspec validate --strict`, and verify it passes on a trivial change
- [x] 1.8 Record the date-based versioning convention (`MAJOR.MINOR.yyyymmdd`) in AGENTS.md, the OpenSpec config, and the `release-versioning` spec, and verify `make version` prints the authoritative value
- [x] 1.9 Apply the date-based version to the backend, frontend, and OpenAPI contract and verify a backend test asserts the API reports it in the required format

## 2. Walking skeleton across the HTTP boundary

- [x] 2.1 Add `GET /health` and `GET /api/v1/ping` to the backend and verify them with API tests
- [x] 2.2 Configure the backend CORS allowlist and the frontend API base URL and verify a component test calls the API from the SPA origin
- [x] 2.3 Render backend data in the frontend and verify an integration test drives the running `db`+`backend`+`frontend` stack across the HTTP boundary
- [x] 2.4 Publish an OpenAPI contract for the skeleton endpoints and a frontend typed-client check, and verify the contract test passes against the running backend
- [x] 2.5 Document the development workflow (run backend, run frontend, run compose) in `docs/development.md` and verify the steps work as written

## 3. Exact money and core value objects (backend)

- [x] 3.1 Implement a `Money` value object (`bigdecimal` amount + currency) and verify unit tests covering addition, subtraction, and non-lossy large/precise values
- [x] 3.2 Implement `Quantity` for crypto-style assets with at least 18 decimals and verify a test storing `0.000000000000000001` round-trips unchanged
- [x] 3.3 Serialize all decimal amounts as JSON strings and verify a serde round-trip test proves no precision loss and that no float type appears in the money API
- [x] 3.4 Add property-based tests for money arithmetic (associativity, round-trip, no rounding drift) and verify they pass
- [x] 3.5 Document the money rules (no floats, exact decimals, currency tagging) in `docs/money.md` and verify the examples in it compile as doctests

## 4. Accounts and balances (backend + frontend)

- [x] 4.1 Implement the account type taxonomy with asset/liability and liquidity classification and verify tests cover every type from the `accounts` spec
- [x] 4.2 Write migrations for accounts, ownership scope, and dated balance records and verify they apply and roll back against Postgres
- [x] 4.3 Define the `AccountRepository` trait and implement the Postgres adapter, verified by repository integration tests
- [x] 4.4 Implement create-account and record-balance use cases and verify a test that runs them with no network access succeeds
- [x] 4.5 Implement account archival that preserves history and verify a test shows the account leaves current balances but remains in history
- [x] 4.6 Expose account and balance HTTP endpoints and verify them with API tests covering success and validation errors
- [x] 4.7 Add the account and balance endpoints to the OpenAPI contract and verify the contract test passes against the running backend
- [x] 4.8 Build the frontend account list and create/edit views against the typed client and verify component tests for success and validation errors
- [x] 4.9 Verify an integration test creates an account and records a balance through the UI against the real backend
- [x] 4.10 Document the account and balance API and its UI in `docs/api.md` and verify the documented requests work as written

## 5. Net worth engine and history (backend + frontend)

- [x] 5.1 Implement total net worth (assets minus liabilities, reporting currency) and verify tests for the `networth` spec scenarios
- [x] 5.2 Implement liquid net worth (cash, deposits, brokerage, and retirement accounts) plus a separate liquidity (cash on hand) figure, and verify tests prove real estate is excluded and secured debt does not reduce liquid net worth
- [x] 5.3 Implement the `RedemptionPolicy` model with percentage and fixed-amount cost components, a single user-set tax rate, and age-banded penalties, and verify tests for the retirement-discount and composited-selling-cost scenarios
- [x] 5.4 Implement accessible net worth from redemption policies and verify tests for the accessible-net-worth scenarios
- [x] 5.5 Implement what-if scenario modeling that projects accessible net worth without persisting until applied, and verify tests for the model-without-saving and apply scenarios
- [x] 5.6 Implement household and per-user aggregation scope with no double counting of shared accounts and verify tests for the shared-account scenario
- [x] 5.7 Persist snapshots capturing total, liquid, and accessible values with the policies and inputs used, and verify tests that a later data or rule change does not alter an existing snapshot until an explicit recompute
- [x] 5.8 Expose net worth and snapshot endpoints, add them to the OpenAPI contract, and verify API and contract tests
- [x] 5.9 Build the frontend summary view showing total, liquid, liquidity, and accessible net worth with period selection and verify component tests for period changes
- [x] 5.10 Build the frontend history chart with switchable net worth form and verify it renders from seeded snapshots
- [x] 5.11 Build frontend filtering and breakdown by account type and asset class/sector and verify tests that filters change summary and chart data
- [x] 5.12 Add stale/manual data indicators in the UI and verify a test with a cached price shows the stale marker
- [x] 5.13 Verify an integration test exercises the summary, history, and filters across the HTTP boundary, and document the dashboard in `docs/dashboard.md`

## 6. Bulk data entry (backend batch endpoints + frontend grid)

- [x] 6.1 Implement a reusable grid component with one row per record, inline fields, and an add-row control, and verify a component test enters several rows without opening a form
- [x] 6.2 Implement keyboard-first navigation and add-row-on-enter and verify a test completes a run of rows using only the keyboard
- [x] 6.3 Implement clipboard paste that splits tab-separated data into rows and columns and verify a test pasting twenty rows yields twenty populated grid rows
- [x] 6.4 Implement inline per-row validation using the exact-decimal parser and verify invalid rows are flagged with reasons without affecting valid rows
- [x] 6.5 Implement backend bulk-save endpoints with an atomic-or-partial policy and a per-row result, and verify API tests for all-valid and partial-failure saves
- [x] 6.6 Implement single-action bulk save in the UI with a per-row result report and verify tests for all-valid and partial-failure saves
- [x] 6.7 Wire the grid to balances and verify a cross-boundary integration test; the same component is extended to holdings and wallet addresses in Groups 9 and 10
- [x] 6.8 Document bulk entry and paste in `docs/bulk-entry.md` and verify the documented steps reproduce a twenty-wallet paste

## 7. Spreadsheet import/export, backup, and analytics access (backend + frontend)

- [x] 7.1 Define the versioned CSV template schema and implement a validating parser and verify row-level error tests (unknown type, bad amount) with no partial writes
- [x] 7.2 Implement idempotent apply keyed on a stable column and verify a test importing the same file twice produces no duplicates
- [x] 7.3 Implement XLSX read and write for the same schema and verify an XLSX round-trip test matches the CSV result
- [x] 7.4 Implement full export of accounts, balances, holdings, and configuration and verify exported files re-import without change
- [x] 7.5 Implement local backup and restore and verify restoring on a clean instance with networking disabled reproduces all data and history
- [x] 7.6 Verify exact-decimal round-trip through import/export/backup with a test using many-decimal and very large values
- [x] 7.7 Add the import/export/backup endpoints to the OpenAPI contract and verify the contract test passes
- [x] 7.8 Build the frontend import/export UI with row-level error display and verify a cross-boundary integration test imports a file and surfaces errors
- [x] 7.9 Document an optional read-only analytics workflow (SQL or Python) against the database or exported CSV and verify a sample query runs against a seeded database
- [x] 7.10 Publish the template and import rules in `docs/import-export.md` and verify the documented template imports as written

## 8. Identity, authentication, and access control (backend + frontend)

- [x] 8.1 Implement household and user models with ownership scoping and verify migrations plus model tests
- [x] 8.2 Implement Argon2id password auth with local sessions and a documented auth-disabled single-user mode and verify tests for both modes
- [x] 8.3 Enforce owner/editor/viewer roles in the application layer and verify a test that a viewer cannot modify data
- [x] 8.4 Enforce per-user data scoping so private accounts are hidden from other members and verify tests for the shared and private scenarios
- [x] 8.5 Add the auth endpoints to the OpenAPI contract and verify the contract test passes
- [x] 8.6 Build the frontend login, session handling, and protected routes and verify component tests for signed-in and signed-out states
- [x] 8.7 Verify an integration test shows a viewer is blocked from modifying data through the UI
- [x] 8.8 Document authentication and role setup in `docs/access-control.md` and verify the documented flow works against a running instance

## 9. Investments and market data (backend + frontend)

- [ ] 9.1 Implement securities, positions, and tax lots with asset-class and sector classification and verify model tests including the multi-lot scenario
- [ ] 9.2 Implement holding and account valuation plus unrealized gain/loss and verify tests for both valuation scenarios
- [ ] 9.3 Implement the `PriceProvider` trait with caching, timestamped prices, offline fallback marked stale, and manual override, verified by tests using a fake provider
- [ ] 9.4 Implement the price add-on loader and user-supplied credential configuration (own API key or subscription, stored locally, inert until enabled) and verify a test that with no key configured the system uses stored prices
- [ ] 9.5 Implement FX rate storage and multi-currency conversion recording the applied rate and verify conversion tests
- [ ] 9.6 Add the holdings and price endpoints to the OpenAPI contract and verify the contract test passes
- [ ] 9.7 Build the frontend holdings view (positions, allocation by class/sector, unrealized P/L) and verify component tests
- [ ] 9.8 Verify a cross-boundary integration test exercises holdings and valuation end to end
- [ ] 9.9 Document the market data add-on contract and bring-your-own-key setup in `docs/market-data.md` and verify the documented setup against the fake provider
- [ ] 9.10 Wire the bulk-entry grid to holdings and verify a cross-boundary integration test

## 10. Cryptocurrency tracking (backend + frontend)

- [ ] 10.1 Implement wallet address registration per chain and verify input validation rejects private keys and seed phrases
- [ ] 10.2 Implement the `ChainDataProvider` trait with a fake backend and derive address balances, verified by adapter tests
- [ ] 10.3 Verify high-precision crypto quantities survive storage, net worth aggregation, and JSON with an 18-decimal test
- [ ] 10.4 Implement offline manual crypto balances and verify a disconnected test includes them in net worth
- [ ] 10.5 Implement aggregation of multiple addresses of the same asset and verify the two-wallets-one-position scenario
- [ ] 10.6 Implement chain add-on enablement with user-supplied API keys and verify it is optional and inert when unconfigured
- [ ] 10.7 Add the wallet endpoints to the OpenAPI contract and verify the contract test passes
- [ ] 10.8 Build the frontend wallet view with manual balance editing and verify component tests
- [ ] 10.9 Verify a cross-boundary integration test adds wallets and sees them in net worth
- [ ] 10.10 Document adding a chain add-on in `docs/crypto.md` and verify the steps by adding the fake chain end to end
- [ ] 10.11 Wire the bulk-entry grid to wallet addresses and verify a cross-boundary integration test

## 11. Budgeting (backend + frontend)

- [ ] 11.1 Implement income recording per user and period and verify model plus API tests
- [ ] 11.2 Implement spending plan allocation with over-allocation detection and verify tests for balanced and over-allocated plans
- [ ] 11.3 Implement category spending records and plan-versus-actual variance reporting and verify variance tests in amount and percentage
- [ ] 11.4 Add the budgeting endpoints to the OpenAPI contract and verify the contract test passes
- [ ] 11.5 Build the frontend spending-plan editor and variance views and verify component tests
- [ ] 11.6 Verify a cross-boundary integration test exercises budgeting and that the app works fully with no budget configured, then document `docs/budgeting.md`

## 12. Read-only aggregation adapter interface (backend + config UI)

- [ ] 12.1 Define the `AggregationProvider` trait and a fake read-only adapter and verify no write capability exists in the trait surface
- [ ] 12.2 Implement provenance-tagged mapping of provider accounts, balances, and holdings and verify tests that the source and sync time are stored
- [ ] 12.3 Implement optional, non-blocking sync with graceful provider-failure handling and verify tests for unconfigured and unreachable providers
- [ ] 12.4 Implement user-supplied provider subscription configuration (own account or API key) with local encrypted token storage and verify no intermediary service is used and tokens never leave the machine
- [ ] 12.5 Implement provenance-preserving updates so provider syncs never silently discard manual values and verify the manual-value-retained scenario
- [ ] 12.6 Add the aggregation endpoints to the OpenAPI contract and verify the contract test passes
- [ ] 12.7 Build the frontend add-on configuration UI (enable a provider, enter your own key) and verify component tests
- [ ] 12.8 Document the aggregation add-on contract and bring-your-own-subscription setup in `docs/aggregation.md` and verify the documented steps against the fake adapter

## 13. Cross-boundary and end-to-end verification

- [ ] 13.1 Run the full spec-scenario suite against the containerized stack (`db`+`backend`+`frontend`) and verify every scenario has a passing test
- [ ] 13.2 Verify a cold, fully offline run (no outbound network) supports account creation, bulk/grid entry, import, net worth, history, and dashboard
- [ ] 13.3 Verify the frontend typed client matches the running backend contract and that the contract test fails on a deliberate mismatch, then revert
- [ ] 13.4 Verify the boundary check fails on a deliberately introduced cross-project import, then remove it
- [ ] 13.5 Verify `openspec validate networth-tracker-v1 --strict` passes and every capability in the proposal has a corresponding spec file

## Workflow follow-up

- Archive the change after the project's review requirements are satisfied.
- Verify the archived result updates the main specs under `openspec/specs/`.
