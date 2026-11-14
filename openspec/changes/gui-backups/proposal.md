# Proposal

## Why

Backups currently require SSH/host terminal access to run `scripts/backup.sh`.
Most household users will not use a terminal, so the GUI should provide the
same encrypted "back up now" and download workflow without granting the browser
host access.

## What Changes

- Add an owner-only backend service that creates a full encrypted PostgreSQL dump
  plus the encrypted document archive inside the backend container.
- Add owner-only endpoints to create, list, and download backups.
- Replace the informational Settings → Backups page with functional controls:
  create a backup, see recent backups, and download encrypted files.
- Keep backup encryption fail-closed: missing/placeholder passphrase blocks
  creation.
- Keep scheduled/off-site backups on the host timer; GUI scheduling and full
  restore are out of scope for this change.

No restore-from-GUI, no Docker socket, and no arbitrary host command execution.

## Capabilities

### New Capabilities

- `backup-operations`: owner-only in-container backup creation, retention,
  listing, encrypted download, and safety behavior.

## Impact

- Backend full-backup service and API endpoints; bundled container-safe backup
  script; Docker runtime package/script copy.
- Backend image and Compose environment (existing backup variables are reused).
- Settings → Backups UI and typed client/OpenAPI schemas.
- Backup documentation and API tests with a fake backup runner.
