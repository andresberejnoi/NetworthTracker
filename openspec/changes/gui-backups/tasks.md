# Tasks

## 1. Container backup runner

- [x] 1.1 Add a container-safe backup script using direct `pg_dump`, gpg encryption, document archive, verification, and retention
- [x] 1.2 Add the script and `tar` to the backend image; reuse existing `PG*`, `BACKUP_*`, and `DOCUMENT_DIR` settings
- [x] 1.3 Add shell-level coverage for success, missing passphrase, verification failure, and retention

## 2. Backend service and API

- [x] 2.1 Add a full-backup service with a process-local concurrency lock and fail-closed passphrase preflight
- [x] 2.2 Add owner-only create/list/download endpoints
- [x] 2.3 Validate download filenames and prevent path traversal
- [x] 2.4 Add backup metadata DTOs and update OpenAPI/frontend typed client
- [x] 2.5 Add API tests for owner access, non-owner denial, create, list, download, invalid name, missing passphrase, and concurrency

## 3. Settings UI

- [x] 3.1 Replace the static Backups page with create/list/download controls
- [x] 3.2 Show errors, empty states, sizes, timestamps, and document-archive downloads
- [x] 3.3 Keep scheduled/off-site and host-only restore guidance visible without exposing secrets
- [x] 3.4 Add frontend tests for backup list, create, download, and error states

## 4. Verification and docs

- [x] 4.1 Update `docs/backup.md`, README, CHANGELOG, and deployment notes
- [x] 4.2 Run the full frontend/backend/integration/upgrade matrix
- [x] 4.3 Run the release process and tag the next version
