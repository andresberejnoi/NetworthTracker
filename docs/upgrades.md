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
- `GET /api/v1/backup` also produces a portable JSON backup including snapshot
  history; `pg_dump` is the full database backup.

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
git fetch --tags && git checkout v0.1.20261028
# or: UPGRADE_GIT_PULL=1 make upgrade

make upgrade
```

`make upgrade`:

1. takes a pre-upgrade backup;
2. builds the images;
3. starts the stack;
4. lets the backend run pending SQLx migrations;
5. waits for `/health` and reports the new version.

If it fails, the backup path is printed. Restore it with:

```bash
make restore FILE=backups/networth-....sql.gz.gpg
```

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
- Prefer additive changes. Destructive changes should use expand/contract over
  two releases (add the new shape, migrate, then remove the old shape later).
- SQLx refuses to start if the database contains migrations the binary does not
  know about, which protects against accidentally running an older image
  against a newer schema.
- A repository test (`backend/crates/adapters/tests/migrations.rs`) applies all
  migrations except the latest, seeds accounts/balances/snapshots/users, applies
  the latest migration, and asserts the rows and exact values survived and the
  new schema is writable. It runs in `make backend-test-db`.
- Test a restore after every release.

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

## What is not automated yet

These are planned for the post-tag **robust upgrades** change:

- a pre-migration backup inside the container entrypoint (today it is in
  `scripts/upgrade.sh`);
- scheduled backups and retention outside the manual script;
- published versioned images (today the stack builds from source);
- a multi-release upgrade matrix (the current test covers the previous schema
  → current release);
- a documented compatibility policy for breaking schema changes;
- a guided data-migration tool for importing from other trackers.

## Commands

```bash
make backup     # ./backups/networth-<timestamp>.sql.gz.gpg (encrypted)
make upgrade    # backup + rebuild + restart + health check
make restore    # FILE=backups/... (confirmation required)
make destroy    # delete the volume (confirmation required)
```
