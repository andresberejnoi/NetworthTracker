# Proposal

## Why

Household net worth is scattered across retirement accounts, brokerages, banks, cash, crypto, and
property, each with its own portal and no single accurate total. Existing tools are cloud-only,
require handing over credentials to an aggregator, and report a single misleading "net worth" number
that ignores taxes, penalties, and liquidation costs. We want a local-first tracker that keeps the
sensitive data on the user's own machine, works fully offline with hand-entered data, and reports
net worth in *useful* forms: total, liquid, and accessible.

## What Changes

This is the greenfield v1 design for **NetworthTracker**, a local, offline-capable
personal/family networth tracker.

- Introduce a versioned domain model for accounts, holdings, and balances across account types
  (401k, Traditional/Roth IRA, taxable brokerage, checking/savings, cash, real estate, crypto).
- Introduce exact decimal money handling (no binary floats) with scales sufficient for both fiat
  cents and high-precision crypto.
- Introduce a networth engine that computes **total**, **liquid**, and **accessible** net worth,
  where accessible value applies configurable haircuts (taxes, early-withdrawal penalties,
  real-estate selling costs) and persists point-in-time history snapshots.
- Introduce market/crypto price and FX retrieval behind a cache-first adapter so historical
  snapshots remain stable and offline use never breaks.
- Introduce read-only external account aggregation behind a provider-agnostic adapter interface;
  the fully-working path is manual entry and import.
- Introduce fast bulk data entry: a spreadsheet-like grid with keyboard-first navigation and
  clipboard paste, so many records (balances, holdings, wallet addresses) are entered without a
  form-per-row workflow, plus CSV/XLSX import/export for interchange and backup.
- Structure the work as a monorepo whose `backend/` and `frontend/` grow together: the frontend is a
  pure consumer of the backend HTTP API, the two share no code and ship as separate containers, and
  each change that adds API capability adds its consumer UI and a cross-boundary test.
- Treat every external provider (market data, chain data, aggregation) as an optional add-on the
  individual user enables with their own API key or subscription; nothing is bundled or required,
  and there is no central service operated by the product.
- Introduce multi-user support: a family/household with per-user profiles, sub-accounts, and
  role-based access levels.
- Introduce budgeting: yearly income, a conscious-spending-style allocation plan, and category
  tracking.
- Introduce a web dashboard for summary metrics, networth history, and filtering by account type
  and asset/sector class.
- No **BREAKING** changes (new project).

## Capabilities

### New Capabilities

- `identity`: users, households/families, sub-accounts, roles and access levels.
- `accounts`: the financial account and balance registry (account types, institutions, ownership, valuation source).
- `investments`: securities positions, cost basis and tax lots, sector/asset-class classification.
- `crypto-tracking`: cryptocurrency wallet addresses, on-chain balance derivation, high-precision amounts.
- `market-data`: quotes, historical prices, and FX rates with caching and offline fallback.
- `networth`: networth definitions (total/liquid/accessible), valuation adjustments, and historical snapshots.
- `account-sync`: read-only external aggregation adapter interface for linking real accounts.
- `bulk-entry`: fast spreadsheet-like grid entry for many records at once, including clipboard paste.
- `data-exchange`: CSV/XLSX import/export between the app and external spreadsheets, plus local backup/restore.
- `budgeting`: income, spending plan, and budget categories.
- `dashboard`: web UI for networth metrics, history charts, and filtering.
- `release-versioning`: the date-based `MAJOR.MINOR.yyyymmdd` product version, single-sourced and reported by the API.

### Modified Capabilities

None (greenfield project).

## Impact

- **Repository layout**: a monorepo with decoupled `backend/` (Rust) and `frontend/` (TypeScript)
  projects, each with its own container file; the frontend consumes the backend HTTP API and no
  cross-project imports are allowed.
- **New codebase**: Rust backend (HTTP/JSON API) + TypeScript web dashboard, packaged to run locally in a container.
- **Storage**: an embedded/local database chosen for exact decimal arithmetic and time-series history.
- **Data model**: versioned schema with migrations; every money value exact and currency-tagged.
- **Integrations**: optional, user-funded add-on adapters for prices/FX, on-chain data, and (later) account aggregation, each enabled with the user's own credentials; all offline-tolerant.
- **Security/privacy**: credentials and tokens, if ever added, stay local; no cloud dependency required for core use.
