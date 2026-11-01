# Changelog

All notable changes to NetworthTracker are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Releases use date-based semantic versions in the form `MAJOR.MINOR.yyyymmdd`,
where the date is the UTC release date. See `AGENTS.md` for the release process.

## [Unreleased]

### Added

- A bulk purchase-lot grid for crypto wallets; multiple rows for one wallet
  become separate FIFO/tax lots.
- A wallet balance versus tax-lots reconciliation warning and a
  "Use balance as opening lot" helper in the crypto view.

### Changed

- README now acknowledges the earlier AssetPortfolio and CryptoTaxTools
  projects and reflects the completed budgeting/aggregation slices.

### Fixed

- Crypto bulk wallet totals now group rows by asset/label or address instead
  of creating a new wallet per row. Duplicate manual labels for the same asset
  are rejected, and repeated dates for one wallet are reported as row errors.
- Manual crypto wallet balances now follow the remaining tax-lot total after
  an opening lot exists; adding batches or selling updates the balance without
  overwriting a total-only position before its opening lot is created.

## [0.1.20261031] - 2026-10-31

This release adds the shared tax-lot ledger, native BTC/ETH crypto tracking,
optional budgeting, and optional read-only aggregation.

### Added

- Optional per-user budgeting: annual income, category spending plans,
  over-allocation flags, actual spending, and plan-versus-actual variance.
- Optional read-only account aggregation with a fake adapter, encrypted local
  tokens, provenance-tagged balances/holdings, and manual-value preservation.
- A Budget view and an owner-only Aggregation view.
- Cryptocurrency tracking with manual wallets/buckets and public-address
  wallets for native BTC/ETH, dated balances, address validation that rejects
  seed phrases/private keys, optional chain-provider sync, aggregated
  positions, and net worth integration as marketable crypto assets.
- Crypto purchase lots and FIFO sales on the shared tax-lot ledger, with
  wallet-scoped realized gains.
- Real free adapters: mempool.space/Blockscout for addresses, Etherscan with a
  user key, and keyless CoinPaprika for crypto prices.
- A Crypto view with asset/wallet/balance forms, manual prices, provider status,
  and a bulk wallet grid.
- A shared append-only tax-lot ledger for securities: original and remaining
  quantities/basis, estimated-basis markers, FIFO sales, per-lot disposals,
  realized gains, and configurable short/long-term holding-period rules.
- Lot-level investments UI with a purchase-lot form, sale form, estimated-basis
  badges, and a realized-gains table.
- Migration coverage for existing `holding_lots` and cross-boundary coverage
  for appending lots and selling FIFO.

### Fixed

- Adding or bulk-pasting a lot no longer deletes existing lots. Positions with
  recorded sales cannot be deleted.

## [0.1.20261029] - 2026-10-29

### Added

- `make upgrade-test`, an end-to-end test that starts at the previous database
  schema, seeds fake accounts/balances/snapshots/users, upgrades to the current
  schema, and verifies the data, exact `NUMERIC` values, new schema, and
  reported version.
- The same test verifies encrypted backup/restore, rejects a wrong passphrase,
  and rejects a corrupt archive without changing the database.
- A dedicated CI job runs the upgrade/backup/restore test.
- Upgrade-testing documentation and a roadmap note for a future guided
  data-migration tool.

### Changed

- Backup and restore scripts accept `DB_CONTAINER` and
  `RESTORE_SKIP_BACKEND` for isolated test databases.
- `make upgrade` reports the version that was actually running before the
  rebuild, instead of the version in the working tree.

### Fixed

- Restore now decrypts and verifies the archive before stopping the backend or
  dropping the database. A wrong passphrase or corrupt backup leaves existing
  data untouched.

### Security

- The backup passphrase is passed to GPG through a file descriptor instead of
  the process argument list.

## [0.1.20261028] - 2026-10-28

The first tagged release. It covers the accounts, net worth, history, bulk
entry, import/export, backup, family access, and investments slices. Crypto
wallet address tracking, budgeting, and read-only aggregation adapters are
planned for later releases.

### Added

- Accounts for 401(k), traditional/Roth IRA, taxable brokerage, checking,
  savings, cash, real estate, other assets, mortgages, and loans, with dated
  balances.
- Net worth dashboard with total, liquid, liquidity, and accessible figures;
  configurable tax, penalty, selling-cost, and fixed-cost assumptions.
- Immutable net worth snapshots and a history chart, plus filters by account
  type, asset class, and sector.
- Exact decimal money throughout; crypto quantities support at least 18
  decimal places in the domain.
- Spreadsheet-like bulk entry with clipboard paste, keyboard-first editing,
  inline validation, and atomic or partial batch saves.
- Versioned CSV/XLSX import and export with validation, idempotent re-imports,
  and sample files.
- JSON backup/restore and full database backups; safe backup, restore, upgrade,
  and destroy scripts.
- Family accounts with local login, Argon2id password hashing, hashed bearer
  sessions, owner/editor/viewer roles, per-user scoping, and an
  `AUTH_DISABLED` single-user mode.
- Investments: securities, holdings, tax lots, manual and cached market prices,
  FX conversion, and sector-aware filtering.
- Simple net worth mode: one account per category plus the Record a balance
  form, with no per-instrument tracking required.
- Responsive desktop/tablet/mobile UI, screenshots, and a quick-start README.
- Gitleaks pre-commit hook and CI scanning with a narrow allowlist.

### Security

- Backups are encrypted by default with GPG AES-256 and require
  `BACKUP_PASSPHRASE`.
- An incremental-migration test verifies that a schema upgrade preserves
  existing rows and exact `NUMERIC` values.

[Unreleased]: https://github.com/andresberejnoi/NetworthTracker/compare/v0.1.20261031...HEAD
[0.1.20261031]: https://github.com/andresberejnoi/NetworthTracker/compare/v0.1.20261029...v0.1.20261031
[0.1.20261029]: https://github.com/andresberejnoi/NetworthTracker/compare/v0.1.20261028...v0.1.20261029
[0.1.20261028]: https://github.com/andresberejnoi/NetworthTracker/releases/tag/v0.1.20261028
