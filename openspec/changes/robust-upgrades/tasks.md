# Tasks

## 1. Upgrade safety

- [x] 1.1 Add a pending-migration check and verified encrypted pre-migration backup to the container entrypoint; fail closed if the backup fails
- [x] 1.2 Add tests for entrypoint backup on pending migrations, no backup when current, and startup refusal on backup failure
- [x] 1.3 Publish versioned backend/frontend images and support a pinned image tag in Compose
- [ ] 1.4 Document the N-1 expand/contract compatibility policy and unknown-migration behavior
- [ ] 1.5 Add a tag-driven multi-release upgrade matrix in CI with fixture data and version/data assertions

## 2. Backup operations

- [ ] 2.1 Add scheduled backup support suitable for cron/systemd timers, with retention and fail-closed encryption
- [ ] 2.2 Add optional off-site destination support using the user's own storage/credentials
- [ ] 2.3 Add a restore-drill command that restores the newest backup into a throwaway database and checks rows/exact decimals
- [ ] 2.4 Document key separation, rotation, retention, off-site storage, and restore drills

## 3. Portable JSON backup v2

- [ ] 3.1 Extend the backup format with securities, holdings, tax lots, sales/disposals, crypto assets/wallets/balances, budgets, and aggregation provenance
- [ ] 3.2 Implement idempotent restore with stable keys and v1 backward compatibility
- [ ] 3.3 Exclude plaintext tokens/passwords/sessions and reject unknown future backup versions
- [ ] 3.4 Add round-trip tests for investments, crypto, lots/sales, budgets, provenance, and v1 compatibility

## 4. Guided data migration

- [ ] 4.1 Define the migration pipeline and source-adapter interface with a generic CSV/JSON adapter
- [ ] 4.2 Implement field mapping, row validation, and a dry-run preview
- [ ] 4.3 Implement atomic/partial apply with stable keys, idempotency, and provenance tags
- [ ] 4.4 Require a verified backup before apply
- [ ] 4.5 Build the guided migration UI or CLI flow and verify it against documented sample exports
- [ ] 4.6 Add cross-boundary/integration tests for a realistic portfolio migration and re-run

## 5. Document ingestion

- [ ] 5.1 Add the encrypted content-addressed document store and metadata/audit schema with SHA-256 dedupe and size limits
- [ ] 5.2 Add upload/list/download/tag API and UI with encryption at rest and authorized access
- [ ] 5.3 Implement content-based format detection and generic CSV/XLSX/XML/OFX/PDF parser adapters
- [ ] 5.4 Implement institution and document-type classification adapters for Fidelity, Charles Schwab, Coinbase, Binance, and generic sources with confidence and user confirmation
- [ ] 5.5 Add dry-run preview, field mapping, idempotent apply, and document-to-record provenance links through the migration pipeline
- [ ] 5.6 Add audit events for upload/download/classification/import and include encrypted documents in backups/restore
- [ ] 5.7 Add a cleanup command/script for orphaned/temporary files with dry-run and confirmation, explicitly excluding computed financial data
- [ ] 5.8 Add parser/classification/import tests, docs, and a future HSA/medical-receipt design note

## 6. Verification and release

- [ ] 6.1 Run the full release matrix, including upgrade/restore/migration/document tests
- [ ] 6.2 Update `docs/upgrades.md`, add `docs/backup.md`, `docs/data-migration.md`, and `docs/document-ingestion.md`, and update the README/CHANGELOG
- [ ] 6.3 Tag the release and verify the tag points at the release commit
