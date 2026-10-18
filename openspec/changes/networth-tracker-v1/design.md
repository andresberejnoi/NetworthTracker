# Design

## Context

See `proposal.md` for motivation and the spec deltas under `specs/` for required behavior. This
document covers how the system is built.

Starting point: an empty repository. There is no existing code, data, or compatibility surface, so
the design optimizes for getting the domain model and persistence semantics right, because those are
the expensive things to change later.

Constraints carried from `openspec/config.yaml`:

- Rust backend, TypeScript dashboard, local/containerized deployment.
- Monorepo with decoupled `backend/` and `frontend/` subprojects: the frontend is only an API
  consumer, the backend never depends on frontend code, and each project carries its own container
  files. Shared assets live at the repository root.
- Exact decimal money, currency-tagged, with crypto precision (at least 18 places).
- Manual entry and import must work fully offline; every external integration is optional and
  user-funded.
- v1 targets USD but the model must not assume two decimal places or a single currency.

## Goals / Non-Goals

**Goals:**

- A domain layer that computes net worth correctly and is testable without a database or network.
- Storage that is exact for money, supports time-series history, and can aggregate in queries.
- A clean seam (`adapter` traits) so market data, chain data, and bank aggregation can be added or
  replaced as user-enabled add-ons without touching the domain.
- Fast bulk data entry (grid + paste) so hand-entering many records is not a form-per-record slog.
- A vertical slice that is usable end to end (manual accounts → net worth → dashboard) before any
  integration work begins.
- Backend and frontend that grow together: each change that adds API capability also adds its
  consumer UI and a test exercising both across the HTTP boundary.
- Container-first delivery that works with no internet access.

**Non-Goals:**

- **Live account aggregation in v1.** The adapter interface is designed and a fake/manual adapter is
  implemented; real providers come later as optional add-ons.
- A spreadsheet engine. The grid is an entry surface for the app's own data model, not a
  general-purpose spreadsheet with formulas or arbitrary cell references.
- Cloud hosting, mobile apps, and multi-device sync. The datastore is local to one machine in v1.
- Initiating any financial transaction. Every integration is strictly read-only.
- Tax filing accuracy. Accessible net worth is an estimate built from user-configured assumptions.
- Multi-currency UI. v1 records and reports USD; FX plumbing exists but is not exercised.
- Full marginal income-tax bracket modeling. v1 applies a single user-set tax rate; bracket stacking
  is planned as its own future capability.
- Shared runtime code between frontend and backend. There is no cross-project code sharing: the
  backend must not import frontend code and the frontend must not import backend code. They integrate
  only over the HTTP API, and only root-level shared, non-runtime assets may be common.

## Decisions

### D1. PostgreSQL as the primary datastore, behind a repository seam

**Decision:** Use PostgreSQL (as a companion container) via SQLx, with amounts in native `NUMERIC`.
Confirmed with the user.

**Rationale:** Net worth is an aggregation-heavy, exact-money, time-series workload, and the user's
hard requirement is correctness for dollar amounts (and many decimal places for crypto). Postgres
gives all three natively: arbitrary-precision `NUMERIC`, real `SUM`/`GROUP BY` for dashboard
breakdowns, and concurrent reads/writes for a family of users. "Offline" does not require "no
server": a local Postgres in Compose is still fully offline.

**Alternatives considered:**

- *SQLite.* Simplest backup story and a true single-file app, but it has no real decimal type:
  `NUMERIC` affinity silently becomes IEEE-754 `REAL`, and `INTEGER` is only 64-bit (max ≈
  9.2×10^18), which overflows 18-decimal crypto quantities for even modest holdings. We would store
  scaled integers or text and do all arithmetic in Rust, forfeiting SQL aggregation. Kept as a
  documented fallback for a future embedded mode; the repository seam keeps this open.
- *DuckDB.* Real decimals and embedded, but read-oriented concurrency makes it a poor transactional
  store for multi-user edits.
- *TimescaleDB.* Rejected as overkill; snapshot volumes here are tiny.

The decision is cheap to reverse early and expensive later, which is why the plan puts a
`Storage`/repository trait in front of it.

### D2. Exact decimals end to end; `bigdecimal` as the domain money type

**Decision:** A single canonical decimal type in the domain (`bigdecimal` for arbitrary precision),
mapped to `NUMERIC(38, 18)`-class columns. Binary floating point is banned everywhere, including JSON
serialization (amounts travel as strings).

**Rationale:** `rust_decimal` is faster but is 96-bit with ~28–29 significant digits; 18-decimal
crypto plus a large notional can approach that ceiling. `bigdecimal` removes the ceiling at the cost
of ergonomics, which is the right trade for a correctness-critical money engine. Every amount is a
`{ quantity, currency }` value object, never a bare number.

**Alternatives considered:** `rust_decimal` (rejected: precision ceiling); integer minor units
(rejected: per-asset scales, 128-bit overflow in SQLite, and awkward FX).

### D3. Hexagonal architecture: pure domain, adapter edges

**Decision:** Four layers — `domain` (pure types + net worth engine, no I/O), `application`
(use cases/orchestration, transactions), `adapters` (HTTP API, Postgres repositories, price/chain/
aggregation providers, CSV/XLSX), and the `frontend/` app. The domain crate has no
dependency on Axum, SQLx, or any provider. External capabilities are Rust traits
(`PriceProvider`, `ChainDataProvider`, `AggregationProvider`) with fakes for tests. The first three
layers live under `backend/`; the UI is the separate `frontend/` project (see D13).

**Rationale:** It isolates the two things that must be correct (money math and net worth semantics)
from the two that will churn (providers and UI), and it makes the spec scenarios directly testable.

### D4. Cash, liquidity, and accessibility are three distinct levels

**Decision:** Model three separate concepts instead of one "liquid" bucket:

1. **Liquidity** (cash on hand) — assets available *right now*: cash, checking, savings. This is the
   "how much cash do I have this minute" number.
2. **Liquid net worth** — assets that can be *quickly sold or bought*, which includes cash, deposit
   accounts, taxable brokerage, and **retirement accounts** (sellable, albeit with tax/penalty), and
   excludes illiquid assets such as real estate. Debt secured by an excluded asset does not reduce it.
3. **Accessible net worth** — the *net proceeds* after taxes, early-withdrawal penalties, and selling
   costs, across all assets.

An asset's `Liquidity` classification (`Immediate` / `Marketable` / `Illiquid`) drives the first two
numbers; accessibility is a separate computation over the same assets.

**Rationale:** These answer different questions — "what cash do I have now?" (liquidity), "what could
I raise quickly without touching the house?" (liquid net worth), and "what would I actually net if I
sold?" (accessible). Conflating them produces the misleading single number the product exists to
avoid. Notably, subtracting a mortgage from liquid net worth is wrong: the mortgage is secured by an
excluded asset, so it belongs in accessible net worth, not liquid.

**Alternatives considered:** One "liquid" metric (rejected: hides the cash-vs-sellable distinction);
gross liquid assets with no liability handling (rejected: ignores consumer debt, which reaches the
model later).

### D5. Redemption policies are configurable, composable, and versioned at snapshot time

**Decision:** Accessible net worth is computed from `RedemptionPolicy` records attached to account
types/assets. A policy is a set of cost components that may be expressed as a **percentage or a fixed
amount**, and it may reference:

- a tax component, expressed as a single user-set rate (effective or marginal);
- an early-withdrawal penalty that may depend on the owner's age;
- selling or closing costs (commission, fees, transfer taxes) as one or more percentage/fixed items;
- optional holding-period rules.

Policies are user-editable, and each snapshot stores the policy versions and rule inputs used. Users
can also run **what-if scenarios** — vary the tax rate or a cost — and see the projected accessible
values without persisting anything until they choose to apply it. Full marginal-bracket stacking is a
deliberate future capability, not part of v1 (see Non-Goals).

**Rationale:** Accessible net worth is an assumption-laden estimate, and the user explicitly wants to
control the assumptions. Composability (percentage *or* fixed, add or subtract taxes and fees) matches
how people actually reason about a property sale or a retirement withdrawal. Versioning keeps history
reproducible when assumptions change (spec: snapshot reproducibility), and scenarios let a user model
"what if I sell now" without committing to new assumptions.

**Alternatives considered:** A single global haircut percentage (rejected: loses per-asset reality);
hardcoded IRS formulas (rejected: state-dependent, changes yearly, and the user wants to model their
own assumptions); a full marginal-bracket engine (deferred: valuable but heavy, planned as its own
future capability).

### D6. Append-only facts plus persisted, immutable snapshots

**Decision:** Store dated balance facts, price facts, and holding facts append-only. Snapshots are
computed from facts and persisted; they are never mutated by later data changes and are recomputed
only on explicit user action. Snapshots store both gross and accessible values per asset.

**Rationale:** Directly satisfies "history does not change retroactively" while allowing cheap
dashboard reads. Alternatives: pure derived-on-read (rejected: history drifts when prices/rules
change) and full event sourcing (rejected: complexity beyond the requirement).

### D7. Decoupled backend API and frontend, each its own container

**Decision:** `backend/` is an Axum + Tokio + SQLx JSON API. `frontend/` is a Vite + React
single-page app using TanStack Query/Table and a time-series-capable chart library. They are separate
deployables with separate container files; the frontend is a pure consumer of the backend's HTTP API
and is configured with the API base URL (with the backend allowing it via CORS). The backend never
serves the SPA or references frontend assets, and the frontend never imports backend code. Local
Compose runs `db`, `backend`, and `frontend`.

**Rationale:** Decoupling keeps the two projects independently buildable and replaceable, and makes
the API the single integration contract — which is what lets a feature be exercised end to end in one
change. It keeps the Rust surface where learning value is highest (domain + API) and uses mature
tooling for the dense UI work (grids, tables, charts, filters).

**Alternatives considered:** Serving the built SPA from the Rust process (rejected: couples the
release artifacts and blurs the boundary the user asked to keep); Rust full-stack (Leptos/Dioxus —
rejected per stack decision: smaller charting/table ecosystem, slower iteration); Next.js (rejected:
adds a Node server runtime unnecessary for a local SPA).

### D8. Bulk entry is a spreadsheet-like grid, not a form per record

**Decision:** The primary hand-entry surface is a **grid**: one row per record, keyboard-first
navigation, an "add row" affordance that keeps focus in the grid, and **clipboard paste** that splits
tab-separated blocks into rows and columns. Validation is inline and per-row; saving is a single bulk
action. CSV/XLSX import and export remain, as interchange and backup, but they are not the only path
to efficient entry.

**Rationale:** The user's actual requirement is throughput: adding twenty wallet addresses or a page
of holdings must not mean twenty modal forms. A paste-friendly grid matches the spreadsheet muscle
memory they already have while writing into the app's validated data model, so referential integrity
and exact decimals hold. The CSV contract stays symmetric with export, and re-import stays idempotent.

**Alternatives considered:** Form-per-record only (rejected: the exact friction the user called out);
embedding a real spreadsheet engine such as a formula-evaluating sheet (rejected: out of scope and a
correctness risk for money).

### D9. Local auth with Argon2id and RBAC; optional auth for single-user

**Decision:** Local password auth with Argon2id and server-side sessions; roles (owner/editor/viewer)
enforced in the application layer; a documented single-user mode with auth disabled. Provider tokens,
when added, are stored locally and encrypted with a local key.

**Rationale:** The threat model is a shared home machine, not the internet. No external identity
provider keeps the app offline-capable; encryption of tokens limits damage from a stolen disk.

### D10. Integration adapters are optional, cached, and never authoritative

**Decision:** Every outbound integration (prices, FX, chain data, aggregation) is a trait adapter that
degrades to cached/manual data and marks values stale. Aggregation is read-only and provenance-tagged;
provider values never silently overwrite manual entries.

**Rationale:** This is the only way "works fully offline" and "can link accounts" coexist, and it
keeps a provider outage or a licensing change from breaking the core product.

### D11. Providers are user-funded, config-enabled built-in adapters

**Decision:** External data sources are compiled into the single application artifact and **enabled by
user configuration**, and the **individual user supplies their own API key or subscription**. The
product ships no bundled provider credentials and operates no central relay or aggregator service;
requests go from the user's machine directly to the provider they configured. Adapters are inert until
the user enables and configures one. This is a config-enabled adapter, not a runtime-loaded plugin:
the `PriceProvider`/`ChainDataProvider`/`AggregationProvider` trait boundary keeps a future dynamic
loader possible without changing the domain.

**Rationale:** This keeps the product local-first, ships one backend artifact, and lets each user
choose and pay for whatever service they trust, matching the user's stated intent. It also avoids taking on
provider terms-of-service, cost, and data-liability exposure, and means a provider going away degrades
to manual entry rather than breaking the app. Compiling adapters in avoids the sandboxing, ABI-stability,
and supply-chain risks of loading third-party code at runtime, which is not justified for v1.

**Alternatives considered:** Bundling the product's own API keys or running a hosted relay (rejected:
reintroduces cloud dependency, cost, and privacy questions); requiring one specific provider
(rejected: lock-in and uneven coverage); dynamically loaded third-party plugins (deferred: sandboxing
and ABI cost, revisited only if a real extension ecosystem emerges).

### D12. Python as an optional analytics companion, not part of the runtime

**Decision:** The shipped product is Rust + TypeScript. Python is welcome as a documented, optional
companion for ad-hoc analysis and data wrangling against the Postgres database or exported CSV, but it
is not a runtime dependency of the app.

**Rationale:** The user's strength is Python and it is the right tool for exploratory analysis of
exported data. Keeping it out of the runtime preserves a small, containerized deployment surface
while letting Python do what it is best at. A read-only database role for analytics scripts keeps
this safe.

### D13. Monorepo layout with enforced project boundaries

**Decision:** The repository is a monorepo: `backend/` (Cargo workspace with `domain`,
`application`, `adapters`, `api`), `frontend/` (Vite app), and root-level shared assets (`docs/`,
`docker-compose.yml`, task runner, CI, `openspec/`). Each project has its own container file
(`backend/Dockerfile`, `frontend/Dockerfile`). Root tooling orchestrates builds and tests but imports
no project code. A CI check fails the build if backend code references a frontend path or frontend
code references a backend path.

**Rationale:** Hard boundaries stop the monorepo from becoming a tangle, keep the two containers
genuinely independent, and encode the user's rule: no cross-subproject imports, while root-level
shared assets are fine.

**Alternatives considered:** One shared workspace/package pulling both in (rejected: couples builds
and breaks independent deployability); separate repositories (rejected for now: a monorepo keeps
cross-stack changes atomic).

### D14. API contract first, so features are tested in unison

**Decision:** The HTTP API is described by an OpenAPI document that is the source of truth. The
frontend's typed client is generated from or checked against it, and a contract test asserts the
running backend matches the spec. Feature work adds the endpoint, the spec entry, the frontend
consumer, and an integration test that drives the real API across the HTTP boundary — in the same
change.

**Rationale:** The user wants new API capabilities exercised together with the frontend. A checked
contract prevents silent drift between two decoupled projects, and cross-boundary integration tests
are what actually prove a feature works.

**Alternatives considered:** Hand-written duplicated request/response types on both sides (rejected:
drifts silently); a shared-types codegen that couples the two projects (rejected: reintroduces the
coupling D13 forbids).

### D15. Date-based semantic versioning: MAJOR.MINOR.yyyymmdd

**Decision:** The product version is `MAJOR.MINOR.yyyymmdd`, where the third segment is the UTC
release date (for example `0.1.20261017`). `MAJOR` and `MINOR` carry the usual compatibility meaning;
the date replaces the numeric patch. The authoritative value is `backend/Cargo.toml`
(`[workspace.package].version`), mirrored into `frontend/package.json` and
`contracts/openapi.yaml`; the API reports it via `GET /api/v1/ping`, and release images are tagged
`v<version>`. `make version` prints the authoritative value, and a backend test asserts the reported
version matches the format and is a real date.

**Rationale:** The user's preference, and it fits a continuously shipped local app: the date is
monotonic and human-readable, tells you how fresh a build is at a glance, and never collides the way
an incremented patch number can when tags are cut by hand. Keeping `MAJOR`/`MINOR` preserves a
compatibility signal that pure CalVer would lose, and single-sourcing it prevents the backend,
frontend, and contract from disagreeing.

**Alternatives considered:** plain semver `0.1.0` (rejected: the user's preference, and a bare patch
number conveys less about freshness); CalVer `2026.10.17` (rejected: drops the
compatibility signal and reads oddly as a three-part number); build metadata `0.1.0+20261017`
(rejected: not reliably carried into container tags and registries).

## Risks / Trade-offs

- **Accessible net worth is inherently approximate, and tax estimation multiplies the assumptions** →
  Make every component explicit and editable, label the number as an estimate, version policies per
  snapshot, keep gross total prominent, and make scenarios clearly non-committal until applied.
- **Postgres adds a second container and ops surface** → Repository seam for a future SQLite mode;
  `pg_dump`-based backup is a spec requirement (`data-exchange`); Compose keeps startup to one command.
- **`bigdecimal` is slower and less ergonomic than `rust_decimal`** → Confine it to the domain and
  serialization boundaries; use property-based tests for arithmetic; revisit only if profiling demands.
- **Bulk paste and grid entry can inject bad data at scale** → Inline per-row validation, atomic or
  explicitly-partial saves with a per-row report, idempotency keys, and exact-decimal parsing that
  rejects ambiguous input rather than coercing it.
- **Provider fragility: rate limits, API keys, ToS, and data licensing** (prices, chains, banks) →
  Cache-first with staleness flags, manual override always available, one adapter per provider, no
  provider required for core functionality, and the user owns their credentials.
- **Monorepo boundary drift as the codebase grows** → Enforce the no-cross-project-import rule in
  CI, keep the HTTP API as the only integration surface, and require a cross-boundary integration
  test for every feature that spans the stack.
- **Two containers plus a database add moving parts for a local app** → Root Compose starts
  everything with one command, and per-project container files keep each build independent.
- **Scope creep across eleven capabilities** → Phase the tasks so a usable vertical slice (manual
  accounts → net worth → history chart) ships first; bulk entry and integrations follow.
- **Rust learning curve slows delivery** → Start with the narrow domain slice; keep the dashboard and
  analytics in TypeScript and Python respectively; avoid over-abstracting early.
- **Stored aggregation credentials are a real asset** → Local-only storage, encrypted at rest, never
  transmitted to a third party, easily rotatable, and never required.
- **Crypto by address invites abuse of scope** (chains, tokens, NFTs, staking) → v1 supports native
  balances for a small set of chains via a per-chain trait; tokens/DeFi are out of scope.

## Migration Plan

Greenfield, so there is no data migration. Delivery is phased as vertical slices, and every phase
that adds API capability also adds its frontend consumer and a cross-boundary integration test:

1. Monorepo scaffold: `backend/` Cargo workspace and `frontend/` Vite app, per-project container
   files, root Compose (`db`, `backend`, `frontend`), CI with the boundary check and
   `openspec validate`.
2. Walking skeleton: a trivial backend endpoint consumed and rendered by the frontend, proven by an
   integration test across the HTTP boundary, plus the OpenAPI contract and typed-client check.
3. Domain core + persistence: accounts, balances, exact-decimal value objects, migrations, and the
   accounts UI with its integration test.
4. Net worth engine: total/liquid/accessible, configurable redemption policies and snapshots, plus the
   summary, history chart, and filter UI with integration tests.
5. Bulk entry grid and spreadsheet import/export plus backup/restore.
6. Identity and RBAC, including the login UI and scoping tests.
7. Investments and market data (including a user-funded price add-on) with the holdings UI.
8. Cryptocurrency tracking (including a user-funded chain add-on) with the wallet UI.
9. Budgeting with its UI.
10. Aggregation adapter interface and a reference read-only, user-funded adapter, with a config UI.

Rollback is per-migration and per-PR; the only irreversible artifact is data, which is why backups
(`data-exchange`) land before integrations.

## Open Questions

Deferrable without changing the specs, approach, or task breakdown:

- Which concrete price/FX provider and which chains first? (Behind traits; a fake ships in v1, and
  several user-oriented options are on the table.)
- Exact keyboard/paste ergonomics of the bulk-entry grid, and whether rows can be pasted into any
  grid field or only into a dedicated paste target.
- Charting library selection within the React dashboard.
- Whether XLSX import needs formula handling or is treated as values-only.
- Reporting-currency selection UX and whether v1 exposes FX conversion at all.
- Whether a future embedded SQLite mode is worth the duplicate repository implementation.
- Whether the analytics companion is documented as raw SQL, a small Python package, or both.
- When to introduce the marginal tax-bracket capability and what inputs it would need (state tax,
  filing status).
- Whether the OpenAPI contract is authored by hand and checked against the backend, or generated from
  the backend with a typed client for the frontend.
