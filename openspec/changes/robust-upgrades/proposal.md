# Proposal

## Why

The v1 app is now usable with real household data, but two operational gaps remain: upgrades still depend on a manually verified backup path, and there is no guided way to bring data in from other trackers. A user should be able to upgrade across releases without risking data and migrate an existing portfolio with a dry run before anything is written.

## What Changes

- Add an automatic pre-migration backup inside the container entrypoint, fail closed if it cannot be created, and print the restore path on failure.
- Add scheduled, encrypted, off-site backup support with retention and restore drills. The user supplies the destination and encryption key; there is no central backup service.
- Publish versioned container images and document pinning a release instead of `latest`.
- Define a compatibility policy for forward-only migrations: expand/contract, N-1 upgrade support, and explicit handling of unknown migrations.
- Add a multi-release upgrade test matrix that starts from prior releases with fixture data and upgrades to the current release.
- Extend the portable JSON backup to cover investments, tax lots, crypto wallets/balances, budgets, and aggregation provenance while keeping v1 backups readable.
- Add a guided data-migration tool with source adapters, field mapping, validation, dry-run preview, idempotent apply, and provenance tags. Generic CSV/JSON is the first adapter; tracker-specific adapters follow.
- Add a local encrypted document store with content-based format detection, institution/document-type classification, parser adapters for CSV/XLSX/XML/OFX/PDF, preview/import, provenance links, audit, and cleanup.
- Keep original statements locally for auditing; support future HSA/medical-receipt organization through the same document store.
- Document upgrade, backup, restore, and migration runbooks.

No **BREAKING** API changes are required; existing v1 JSON backups remain restorable.

## Capabilities

### New Capabilities

- `upgrade-safety`: pre-migration backups, scheduled/off-site encrypted backups, restore drills, versioned artifacts, compatibility policy, and multi-release upgrade verification.
- `data-migration`: guided import from other trackers with source adapters, dry-run mapping, validation, idempotent apply, and provenance.
- `document-ingestion`: encrypted local document storage, format detection, institution/type classification, parser adapters, preview/import, audit, and safe cleanup.

### Modified Capabilities

- `data-exchange`: portable JSON backup/restore now includes investments, tax lots, crypto, budgets, and aggregation provenance, with a versioned, backward-compatible format.

## Impact

- Container entrypoint and Compose configuration.
- Backup/restore scripts, scheduler integration, key-management docs, and restore drills.
- CI release/upgrade jobs and versioned image publishing.
- `backend/crates/application/src/backup.rs`, backup DTOs/API, and import/export UI.
- New migration/import tooling, CLI or frontend wizard, and source-adapter tests.
- Encrypted document store, parser adapters, metadata/audit tables, upload UI, and cleanup/backup integration.
- `docs/upgrades.md`, `docs/backup.md`, `docs/data-migration.md`, and `README.md`.
