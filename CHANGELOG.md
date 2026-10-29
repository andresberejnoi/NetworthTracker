# Changelog

All notable changes to NetworthTracker are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Releases use date-based semantic versions in the form `MAJOR.MINOR.yyyymmdd`,
where the date is the UTC release date. See `AGENTS.md` for the release process.

## [Unreleased]

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

[Unreleased]: https://github.com/andresberejnoi/NetworthTracker/compare/v0.1.20261029...HEAD
[0.1.20261029]: https://github.com/andresberejnoi/NetworthTracker/compare/v0.1.20261028...v0.1.20261029
[0.1.20261028]: https://github.com/andresberejnoi/NetworthTracker/releases/tag/v0.1.20261028
