# Upgrades and data safety

## Golden rules

- **Never run `docker compose down -v`** unless you intend to delete all data.
  Use `make down` (keeps the volume) or `make destroy` (asks for confirmation).
- **Back up before every upgrade.** `make upgrade` does this automatically.
- **Never edit a migration that has already been applied.** Add a new one.
- Treat `./backups` and `BACKUP_PASSPHRASE` like a wallet: together they unlock
  your financial data. Keep the passphrase somewhere safe.

## Backup

```bash
make backup
# → backups/networth-<UTC timestamp>.sql.gz.gpg
```

- **Encrypted by default** with GPG symmetric AES-256; the passphrase comes from
  `BACKUP_PASSPHRASE` in `.env`.
- `pg_dump` runs inside the `db` container; the dump is gzip-compressed, then
  encrypted. The script decrypts it once to verify the archive before keeping it.
- Keeps the last **14** backups (`BACKUP_KEEP=30 make backup` to change).
- Safe to run while the stack is up.
- `make backup-scheduled` runs the cron/systemd wrapper with locking and a
  timestamped log; see [`backup.md`](backup.md) for timer and cron setup.
- Uploaded documents are included as a separately encrypted
  `networth-documents-*.tar.gz.gpg` archive and restored with the database when
  present. See [`document-ingestion.md`](document-ingestion.md).
- `GET /api/v1/backup` also produces a portable JSON backup including snapshot
  history, investments, crypto, budgets, and aggregation provenance; `pg_dump`
  is the full database backup.

### Key management

- `BACKUP_PASSPHRASE` is required; the script refuses a missing or placeholder
  value.
- **Keep the passphrase out of `./backups`.** Store it in a password manager or
  on another machine. If you lose it, the encrypted backups are unrecoverable.
- Rotate the passphrase by taking a new backup with the new passphrase and
  deleting the old one (or re-encrypting it).
- Backups contain your full financial history; encryption is the default, not an
  option.

### Industry standard

Sensitive-data backups are normally **encrypted at rest and in transit**. Plain
dumps are common only in development or when the storage layer itself encrypts
(LUKS, cloud KMS/SSE). Common approaches are GPG/age-encrypted logical dumps,
`pgBackRest`/`WAL-G` with AES-256 for physical backups, or cloud-managed keys.
Key management is the hard part: keep the key separate from the backup, rotate
it, and test restores. This project uses GPG AES-256 with a passphrase in
`.env` as the pragmatic self-hosted default.

## Upgrade

```bash
# get the new code (choose one)
git fetch --tags && git checkout v0.1.20261108
# or: UPGRADE_GIT_PULL=1 make upgrade

make upgrade
```

`make upgrade`:

1. takes a pre-upgrade backup;
2. builds the images, or pulls them when `UPGRADE_PULL=1`;
3. starts the stack;
4. the backend entrypoint checks for pending SQLx migrations and, if any are
   pending, creates and verifies another encrypted backup before migrations run;
5. the backend runs pending migrations;
6. waits for `/health` and reports the new version.

The entrypoint backup is **fail closed**: if pending migrations are detected and
`BACKUP_PASSPHRASE` is missing, or the backup cannot be written/verified, the
container exits instead of migrating. Pre-migration backups are written to
`./backups` (mounted at `/backups`) and pruned by `BACKUP_KEEP` (default 14).

If it fails, the backup path is printed. Restore it with:

```bash
make restore FILE=backups/networth-....sql.gz.gpg
```

### Pinned release images

Release tags publish backend and frontend images to GHCR. Set `IMAGE_TAG` to a
release such as `v0.1.20261108`, then pull and start without rebuilding:

```bash
IMAGE_TAG=v0.1.20261108 docker compose pull
IMAGE_TAG=v0.1.20261108 docker compose up -d --no-build
```

`make upgrade` builds locally by default. For a pinned deployment, set
`UPGRADE_PULL=1` so it pulls the tagged images instead:

```bash
IMAGE_TAG=v0.1.20261108 UPGRADE_PULL=1 make upgrade
```

Do not use an older image tag as a schema rollback. Migrations are
forward-only; restore the pre-upgrade backup if the newer release applied one.
See `docs/deployment.md` for GHCR authentication and rollback details.

## Restore

```bash
make restore FILE=backups/networth-....sql.gz.gpg
```

The script decrypts the `.gpg` dump with `BACKUP_PASSPHRASE`, stops the backend,
drops and recreates the database, restores, starts the backend, and waits for
health. Legacy plain `.sql.gz` dumps are still accepted. The dump includes the
migration history, so any migrations added since the backup apply on startup.

Before dropping anything, the script decrypts and gzip-verifies the archive. A
wrong passphrase or corrupt file is rejected and the database is left untouched.

## Migration policy

- Migrations live in `backend/migrations/`, are versioned by timestamp, and run
  automatically at startup via SQLx.
- They are **forward-only**; there is no downgrade path. To roll back, restore a
  backup.
- Each migration is applied in a transaction on PostgreSQL.
- **Never edit a migration that has already been applied.** SQLx records a
  checksum; changing the file makes startup fail with a checksum mismatch. Add
  a new migration instead.
- Prefer additive changes. Destructive changes use **expand/contract** over two
  releases:
  1. **Expand** release: add the new column/table with a safe default or
     nullable shape, dual-write or backfill, and keep the old shape readable.
  2. **Contract** release: only after the expand release has shipped and data
     is backfilled, remove the old shape and its dual-write path.
- A repository test (`backend/crates/adapters/tests/migrations.rs`) applies all
  migrations except the latest, seeds accounts/balances/snapshots/users plus a
  pre-ledger holding lot, applies the latest migration, and asserts the rows,
  exact values, and migrated tax lot survived and the new schema is writable.
  It runs in `make backend-test-db`.
- Test a restore after every release.

## Compatibility policy (N-1)

The project supports **direct upgrades from the previous release (N-1)** to the
current release. It does not promise that any older database can jump directly
to the newest release.

| From | To current `0.1.20261108` | Path |
|---|---|---|
| `0.1.20261101` | Supported directly | Pull the current image and upgrade |
| `0.1.20261031` or older | Not direct | Upgrade to `0.1.20261101` first, then current |

`N-1` means the previous **release**, not the previous migration: every
migration shipped between that release and the current one still runs in order.
The direct-upgrade promise is tested by `make upgrade-test`; the tag-driven
multi-release matrix extends that coverage to prior release tags.

For a breaking schema change, the expand release and the contract release must
both remain within this policy: an N-1 database must be able to reach the expand
release, and the contract release must only require data that the expand release
already backfilled. Do not combine expand and contract in one release.

## Unknown migrations

The container entrypoint compares the versions in `_sqlx_migrations` with the
migrations embedded in the image **before** taking a backup or starting the API.
If the database contains a migration version that the image does not know, it
prints:

```text
Database contains migration <version>, which this image does not know.
Refusing to start an older image against a newer schema.
```

and exits without running migrations. SQLx enforces the same rule when the API
starts, and it also rejects a known version whose checksum changed. This is
deliberate protection against running an older image against a newer schema.

To recover, deploy an image whose migration set includes the database version.
Do **not** delete rows from `_sqlx_migrations` or manually downgrade the schema.
If the matching image is unavailable or the upgrade is suspect, restore the
pre-upgrade backup with `make restore FILE=...`.

## Testing the upgrade path

`make upgrade-test` runs `scripts/test-upgrade.sh`. It builds a throwaway
Postgres at the previous schema (every migration except the latest), seeds fake
accounts, balances, snapshots, and users, then starts the current backend so it
applies the remaining migration. It verifies:

- the fake data and exact `NUMERIC` values survived;
- the new schema is writable;
- `GET /api/v1/ping` reports the authoritative version;
- an encrypted backup can be decrypted and restored;
- a wrong passphrase is rejected before the database is touched.

The test uses temporary copies of the backup and restore scripts, a throwaway
container, and a temporary backup directory, so it never touches your real
database or `./backups`.

`make upgrade-matrix` runs `scripts/test-upgrade-matrix.sh`. It selects the
newest release tags that are ancestors of the current commit (four by default),
checks each tag out into a temporary git worktree, applies that tag's migrations,
seeds accounts, balances, snapshots, users, securities, holdings, and a lot, then
starts the current backend. It asserts the data and exact decimals survived, the
current crypto/budgeting/aggregation tables exist, and `GET /api/v1/ping` reports
the current version. This is the tag-driven matrix that runs in CI.

Use `UPGRADE_MATRIX_TAGS="v0.1.20261031 v0.1.20261108"` to test specific tags, or
`UPGRADE_MATRIX_LIMIT=0` to test every ancestor release tag. The matrix uses
throwaway Postgres containers and temporary worktrees, so it does not touch the
real stack.

## Future work

The generic CSV/JSON migration pipeline is implemented. Institution-specific
source adapters, a future HSA/medical-receipt capability, and additional cloud
backup destinations remain follow-ups.

## Commands

```bash
make backup          # ./backups/networth-<timestamp>.sql.gz.gpg (encrypted)
make upgrade         # backup + rebuild + restart + health check
make upgrade-test    # previous schema -> current, backup/restore drill
make upgrade-matrix  # recent release tags -> current, fixture data
make restore         # FILE=backups/... (confirmation required)
make restore-drill   # newest backup -> throwaway DB, row/decimal check
make documents-cleanup # dry-run orphaned/temporary document files
./scripts/migrate.sh --file samples/migration-portfolio.json # migration preview
make destroy         # delete the volume (confirmation required)
```
