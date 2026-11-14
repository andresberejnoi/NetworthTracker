# Design

## D1. Backend container runs the backup, not the browser

**Decision:** The API container executes a bundled container-safe backup script.
The browser calls authenticated JSON endpoints.

**Rationale:** The backend already has the PostgreSQL client, gpg, gzip, the
database connection, `/backups`, `/documents`, and `BACKUP_PASSPHRASE`. This
avoids a Docker socket, host shell access, or a separate agent.

**Alternatives considered:** Browser invoking host scripts (impossible safely);
Docker socket from frontend/backend (rejected: broad privilege); host sidecar
(later option, unnecessary now).

## D2. Reuse backup encryption/verification semantics

**Decision:** Add `scripts/backup-container.sh` modeled on `scripts/backup.sh`,
but it calls `pg_dump` directly using the container PG environment instead of
`docker compose exec`. It writes:

```text
$BACKUP_DIR/networth-<timestamp>.sql.gz.gpg
$BACKUP_DIR/networth-documents-<timestamp>.tar.gz.gpg  (when documents exist)
```

It verifies by decrypting and testing the gzip/tar stream, then prunes local
files using `BACKUP_KEEP`. The API supplies a fixed timestamp so it can return
the created filenames.

## D3. API endpoints

```text
POST /api/v1/admin/backups
GET  /api/v1/admin/backups
GET  /api/v1/admin/backups/{name}/download
```

- Owner-only.
- Create returns the database file and optional document-file metadata.
- List scans only recognized `networth-*.sql.gz.gpg` and
  `networth-documents-*.tar.gz.gpg` names.
- Download validates a basename and serves only matching files from
  `BACKUP_DIR`.
- No delete/restore endpoints.

## D4. Concurrency and failure behavior

Use a process-local async mutex around creation. A second request gets
`409 Conflict`. If the script fails or the resulting file is missing/empty, return
an error and leave no partial file. Missing/placeholder passphrase fails before
spawning the script.

## D5. UI

Settings → Backups becomes:

- Back up now button
- Last backup / error state
- Recent database and document archives, with size and download buttons
- A note that scheduled and off-site backups still use the host timer
- A note that restore remains host/CLI for now

Downloads use authenticated `fetch` as a Blob and a temporary object URL, since
the app uses bearer tokens rather than cookies.

## D6. Container image

Add the container backup script to the backend image and install `tar` explicitly
for the document archive. Reuse the existing `PG*`, `BACKUP_*`, and
`DOCUMENT_DIR` environment variables.

## D7. Testing

Use a fake backup runner in API tests to avoid pg_dump/gpg:
- owner create/list/download
- non-owner forbidden
- missing passphrase fails closed
- path traversal rejected
- fake documents file included
- concurrent creation rejected

The real container script is covered by shell-level command stubs or a
host-side smoke test; restore remains covered by the existing host restore
tests.
