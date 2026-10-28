# NetworthTracker

A local-first, offline-capable personal and family net worth tracker. Your
financial data stays on your machine, manual entry works with no network, and
net worth is reported in the forms that actually matter: **total**, **liquid**,
**liquidity** (cash on hand), and **accessible** (after taxes, penalties, and
selling costs).

![Net worth dashboard](docs/screenshots/networth.png)

| Mobile | Tablet |
|--------|--------|
| <img src="docs/screenshots/mobile-networth.png" alt="Mobile net worth" width="320" /> | <img src="docs/screenshots/tablet-networth.png" alt="Tablet net worth" width="520" /> |

## What it does

- **Accounts across everything** — 401(k), Traditional/Roth IRA, taxable
  brokerage, checking, savings, cash, real estate, other assets, mortgages, and
  loans.
- **Exact money** — arbitrary-precision decimals, never binary floats. Crypto
  amounts keep at least 18 decimal places.
- **Four net worth figures** — total, liquid (cash + brokerage + retirement),
  liquidity (cash available now), and accessible (net proceeds after estimated
  taxes, penalties, and selling costs).
- **History** — immutable snapshots and a chart; changing assumptions never
  rewrites the past.
- **Fast bulk entry** — a spreadsheet-like grid with keyboard-first entry and
  clipboard paste. Paste twenty rows instead of opening twenty forms.
- **Spreadsheet import/export** — versioned CSV/XLSX with validation before any
  write, idempotent re-imports, and sample files in `samples/`.
- **Full backup/restore** — JSON with accounts, balances, and snapshot history.
- **Family accounts** — local login, owner/editor/viewer roles, private
  accounts, and per-user visibility.
- **Offline first** — the core app needs no internet. External providers are
  optional, user-funded add-ons behind read-only interfaces.

| Accounts | Bulk entry |
|----------|------------|
| <img src="docs/screenshots/accounts.png" alt="Accounts" width="520" /> | <img src="docs/screenshots/bulk-entry.png" alt="Bulk entry" width="520" /> |

| Import / export | Login |
|-----------------|-------|
| <img src="docs/screenshots/data.png" alt="Data import and export" width="520" /> | <img src="docs/screenshots/login.png" alt="Login" width="320" /> |

## Simple net worth (no per-instrument tracking)

You do not need holdings. Create one account per category and record its total
value:

1. **Accounts** → add e.g. `Stocks (total)` as `taxable_brokerage`,
   `Real estate (total)` as `real_estate`, `Bonds (total)` as `other_asset`.
2. **Accounts → Record a balance** → enter the date and total value.

Net worth, history, filters, and accessible value work the same. Add securities
and tax lots later only if you want per-instrument detail.

## Quick start (containers)

```bash
git clone git@github.com:andresberejnoi/NetworthTracker.git
cd NetworthTracker

cp .env.example .env
# edit .env: set a strong POSTGRES_PASSWORD and BACKUP_PASSPHRASE

docker compose up --build
```

Open **http://localhost:3000**. On first run, create the household owner
(username + password). The backend runs on `http://localhost:8080`.

If port 3000 is already in use:

```bash
FRONTEND_PORT=8081 docker compose up --build
```

Stop the stack:

```bash
docker compose down       # keep data
docker compose down -v    # also delete the database volume
```

### Single-user offline mode

Set `AUTH_DISABLED=true` in `.env` to skip the login screen. Everything runs as
an implicit local owner. Do not use this on a shared machine.

## Local development

Prerequisites: Rust stable, Node 22+, Docker or Podman with Compose, and
[gitleaks](https://github.com/gitleaks/gitleaks) for the pre-commit secret gate.

```bash
# database
docker compose up -d db

# enable the pre-commit secret scan (once per clone)
make hooks

# backend (reads DATABASE_URL and AUTH_DISABLED)
cd backend
DATABASE_URL='postgres://networth:YOUR_DB_PASSWORD@localhost:5432/networth' \
  AUTH_DISABLED=true cargo run -p networth-api

# frontend (separate terminal)
cd frontend
npm ci
npm run dev        # http://localhost:5173
```

Common checks:

```bash
make help
make test              # backend + frontend + secret + boundary checks
make backend-test-db   # Postgres repository tests (throwaway container)
make integration-test  # real backend + frontend client over HTTP
make hooks             # enable the gitleaks pre-commit hook (once per clone)
make secrets           # fast scan + full-history gitleaks scan
make validate          # openspec validate --strict
```

## Architecture

```
backend/     Rust API (Axum + SQLx)         -> own Dockerfile
frontend/    TypeScript app (Vite + React)  -> own Dockerfile
contracts/   openapi.yaml: the only integration surface
docs/        guides and screenshots
samples/     importable CSV/XLSX examples
openspec/    spec-driven change proposals and specs
```

The two projects are **decoupled**: the frontend is a pure HTTP consumer and
neither side imports the other's code. `contracts/openapi.yaml` is shared at the
root; the backend is validated against it and the frontend's typed client is
generated from it. `make boundaries` enforces this.

## Data, privacy, and security

- Data lives in a local PostgreSQL database in a Docker volume.
- Passwords are hashed with **Argon2id**; session tokens are stored only as
  SHA-256 hashes and expire after 30 days.
- Local secrets live in `.env`, which is gitignored. There is no committed
  database password. `make secrets` scans tracked files for credentials and
  runs in CI.
- Provider API keys (prices, chains, aggregation) are supplied by the individual
  user and stored locally; the product bundles none and runs no central relay.
- For a shared or portable machine, use full-disk encryption.

## Data safety and upgrades

Your data lives in the `db-data` volume. **Never run `docker compose down -v`**
unless you intend to delete it.

```bash
make backup                  # ./backups/networth-<timestamp>.sql.gz.gpg (encrypted)
make upgrade                 # backup + rebuild + restart + health check
make restore FILE=backups/…  # replace the database from a backup
make destroy                 # wipe the volume (confirmation required)
```

See `docs/upgrades.md` for the full procedure, migration policy, and what is
still planned.

## Versioning

Date-based semantic versioning: **`MAJOR.MINOR.yyyymmdd`** (UTC release date).
The current version is `0.1.20261027`. The authoritative value lives in
`backend/Cargo.toml`; `make version` prints it. See `AGENTS.md`.

## Documentation

- `docs/access-control.md` — users, roles, sessions, scoping
- `docs/api.md` — HTTP API examples
- `docs/security.md` — secret handling, gitleaks, pre-commit hook, sessions
- `docs/upgrades.md` — backups, upgrades, restore, migration policy
- `docs/import-export.md` — spreadsheet format, samples, backups
- `docs/bulk-entry.md` — grid, paste, batch saves
- `docs/dashboard.md` — frontend views
- `docs/analytics.md` — read-only SQL/Python analysis
- `openspec/changes/networth-tracker-v1/` — proposal, specs, design, tasks

## Roadmap

The v1 design also covers investment holdings and sectors, cryptocurrency wallet
tracking, budgeting, and read-only aggregation adapters. These are specified in
OpenSpec and land as their own slices; the current release covers the accounts,
net worth, history, bulk entry, import/export, backups, and family access.

## License

MIT
