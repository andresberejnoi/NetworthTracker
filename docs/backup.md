# Backups

Backups are encrypted with GPG AES-256 by default. `BACKUP_PASSPHRASE` is
required; missing, empty, or placeholder values are rejected rather than
writing a plaintext dump.

## Manual backup

```bash
make backup
# → backups/networth-<UTC timestamp>.sql.gz.gpg
```

`scripts/backup.sh` dumps the Compose `db` service, gzip-compresses it, encrypts
it, and decrypts it once to verify the archive. It keeps the newest
`BACKUP_KEEP` files (default 14). It is safe to run while the stack is up.

Restore with:

```bash
make restore FILE=backups/networth-....sql.gz.gpg
```

If `DOCUMENT_DIR` exists, `backup.sh` also writes a verified
`networth-documents-<timestamp>.tar.gz.gpg` archive containing the encrypted
document files. `restore.sh` restores the matching document archive alongside
the database dump. See [`document-ingestion.md`](document-ingestion.md).

See [`upgrades.md`](upgrades.md) for restore and pre-migration backup behavior.

## Scheduled backups

`scripts/scheduled-backup.sh` is the cron/systemd entry point. It:

- sources `.env` and refuses to run without a real `BACKUP_PASSPHRASE`;
- serializes runs with a lock, so a slow backup cannot overlap the next one;
- writes a timestamped log to `BACKUP_LOG_DIR/backup.log` (defaults to
  `BACKUP_DIR`);
- delegates dump, encryption, verification, and retention to `backup.sh`;
- exits nonzero if encryption or verification fails, so the scheduler records a
  failed unit/job instead of silently missing protection.

Run it once manually with:

```bash
make backup-scheduled
```

### systemd user timer

Print the generated units first:

```bash
make backup-timer
```

Install and enable a daily timer:

```bash
./scripts/install-backup-timer.sh --install
```

Use a specific schedule or skip enabling:

```bash
./scripts/install-backup-timer.sh --install --schedule "*-*-* 03:15:00"
./scripts/install-backup-timer.sh --install --no-enable
```

The units are written under `~/.config/systemd/user/`. Check them with:

```bash
systemctl --user list-timers networth-backup.timer
systemctl --user status networth-backup.service
journalctl --user -u networth-backup.service
```

The timer uses `Persistent=true`, so a missed run is attempted after the machine
comes back. To run while you are logged out, enable lingering once:

```bash
loginctl enable-linger "$USER"
```

The service runs as your user, so that user needs access to Docker (usually the
`docker` group) or a working rootless Podman setup. `COMPOSE_CMD` in `.env` is
used by the backup script.

### cron

Cron works too; the wrapper writes its own log, so no redirection is required:

```cron
15 3 * * * /absolute/path/to/NetworthTracker/scripts/scheduled-backup.sh
```

Use an absolute path and make sure cron's environment can reach `docker` or
`podman`. If it cannot, set `COMPOSE_CMD` to the absolute command in `.env`.
Check the log at `$BACKUP_DIR/backup.log` (or `$BACKUP_LOG_DIR/backup.log`).
The log is append-only; rotate or truncate it with logrotate if it grows.

## Configuration

| Variable | Purpose |
|---|---|
| `BACKUP_PASSPHRASE` | Required encryption passphrase; keep it separate from backups |
| `BACKUP_DIR` | Backup directory, default `./backups` |
| `BACKUP_LOG_DIR` | Scheduled-backup log directory, default `BACKUP_DIR` |
| `BACKUP_KEEP` | Number of local backups retained, default 14 |
| `BACKUP_REMOTE` | Optional rclone destination for encrypted off-site copies |
| `BACKUP_REMOTE_KEEP` | Optional number of off-site backups retained |
| `COMPOSE_CMD` | Compose command used by the scripts, e.g. `podman compose` |
| `DOCKER_CMD` | Container command for restore drills, e.g. `podman` |

Keep the passphrase in a password manager or another machine, **not** in
`./backups`. An encrypted backup and its passphrase in the same directory are
not protection.

## Retention

- **Local:** `BACKUP_KEEP` (default 14) keeps the newest
  `networth-*.sql.gz.gpg` files. Pruning happens only after the new backup is
  written and verified. Logs and lock files are not counted.
- **Pre-migration:** the backend entrypoint uses the same `BACKUP_KEEP` for its
  `networth-pre-migration-*.sql.gz.gpg` files.
- **Off-site:** `BACKUP_REMOTE_KEEP` is optional. If unset, the remote keeps
  every copied backup. If set, it must be a positive integer; only
  `networth-*.sql.gz.gpg` files at `BACKUP_REMOTE` are considered. Pruning runs
  after the new copy is verified.
- Keep at least one backup locally and one off-site. A retention setting that
  leaves only the newest copy makes a corrupt or encrypted-wrong backup
  unrecoverable; test with `make restore-drill` before relying on it.

## Restore drill

Run a drill against the newest backup:

```bash
make restore-drill
```

Or drill a specific file:

```bash
./scripts/restore-drill.sh backups/networth-20261108T050447Z.sql.gz.gpg
```

The drill starts a throwaway Postgres container, restores the backup with a
temporary copy of `restore.sh`, and checks that:

- the migration history is present;
- accounts, balances, and snapshots tables are queryable;
- every balance's text form is a plain decimal (no float/exponent form);
- it prints row counts and the newest balance value.

The real database and `./backups` are not modified. Set `DOCKER_CMD=podman` if
you use Podman without the Docker shim. Run a drill after changing backup keys,
destinations, or retention settings.

## Off-site copies

Off-site copying uses [rclone](https://rclone.org/) and the user's own remote
credentials. Configure a remote once:

```bash
rclone config
# e.g. an S3-compatible bucket, SFTP host, WebDAV server, or another rclone remote
```

Then set the destination in `.env`:

```env
BACKUP_REMOTE=remote:networth-backups
# Optional: keep only the newest N backups at the remote destination.
BACKUP_REMOTE_KEEP=30
```

The local backup is encrypted and verified before it is copied, so the remote
only sees ciphertext. The copy is verified with `rclone check`; if copying or
verification fails, `backup.sh` exits nonzero and the local encrypted backup is
left in place. There is no NetworthTracker-operated relay or cloud service.

Test the destination with:

```bash
rclone lsf remote:networth-backups
```

## Key separation and rotation

Backups contain the full financial database, including account history and any
provider credentials stored locally. Treat them as sensitive even though the
files are encrypted.

- Store `BACKUP_PASSPHRASE` in a password manager or on another machine, not in
  `./backups`. Keep `.env` mode `600` and use full-disk encryption on the host.
- Keep at least one copy of the passphrase outside the host that holds the
  backups; losing it means the encrypted backups cannot be restored.
- To rotate the passphrase:
  1. Set the new passphrase in `.env`.
  2. Run `make backup` and verify it with `make restore-drill FILE=...`.
  3. Copy the new backup off-site.
  4. Keep the old passphrase until every backup encrypted with it is deleted;
     then remove the old backups and retire the old passphrase.
- Do not re-encrypt a backup in place without verifying the result. A new
  backup with the new key is simpler and leaves the old copy available until
  the rotation is confirmed.
- The same passphrase protects manual, scheduled, and pre-migration backups.
  Rotate it deliberately rather than changing it between runs.

## When to run a drill

Run `make restore-drill` after:

- the first scheduled backup;
- changing `BACKUP_PASSPHRASE`, `BACKUP_DIR`, or `BACKUP_REMOTE`;
- changing retention;
- upgrading the application or PostgreSQL image.

The drill restores into a throwaway database and does not touch the live stack.
Keep the printed row counts and decimal value with your release notes if you
want an audit trail.
