# Upgrades and data safety

## Golden rules

- **Never run `docker compose down -v`** unless you intend to delete all data.
  Use `make down` (keeps the volume) or `make destroy` (asks for confirmation).
- **Back up before every upgrade.** `make upgrade` does this automatically.
- **Never edit a migration that has already been applied.** Add a new one.
- Treat `./backups` like a wallet: it contains your financial data. Encrypt it
  and keep it somewhere safe.

## Backup

```bash
make backup
# → backups/networth-<UTC timestamp>.sql.gz
```

- Uses `pg_dump` inside the `db` container, gzip-compressed and verified.
- Keeps the last **14** backups (`BACKUP_KEEP=30 make backup` to change).
- Safe to run while the stack is up.
- `GET /api/v1/backup` also produces a portable JSON backup including snapshot
  history; `pg_dump` is the full database backup.

## Upgrade

```bash
# get the new code (choose one)
git fetch --tags && git checkout v0.1.20261027
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
make restore FILE=backups/networth-....sql.gz
```

## Restore

```bash
make restore FILE=backups/networth-....sql.gz
```

The script stops the backend, drops and recreates the database, restores the
dump, starts the backend, and waits for health. The dump includes the migration
history, so any migrations added since the backup apply on startup.

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
- Test a restore after every release.

## What is not automated yet

These are planned for the post-tag **robust upgrades** change:

- a pre-migration backup inside the container entrypoint (today it is in
  `scripts/upgrade.sh`);
- scheduled backups and retention outside the manual script;
- encrypted backups;
- published versioned images (today the stack builds from source);
- an automated upgrade test matrix (previous release → new release);
- a documented compatibility policy for breaking schema changes.

## Commands

```bash
make backup     # ./backups/networth-<timestamp>.sql.gz
make upgrade    # backup + rebuild + restart + health check
make restore    # FILE=backups/... (confirmation required)
make destroy    # delete the volume (confirmation required)
```
