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
| `BACKUP_KEEP` | Number of backups retained, default 14 |
| `COMPOSE_CMD` | Compose command used by the scripts, e.g. `podman compose` |

Keep the passphrase in a password manager or another machine, **not** in
`./backups`. An encrypted backup and its passphrase in the same directory are
not protection.

## Key separation and rotation

- Store `BACKUP_PASSPHRASE` separately from the backup files.
- To rotate, take a new backup with the new passphrase, verify it, then remove
  or re-encrypt the old backups.
- Without the passphrase, encrypted backups cannot be restored. There is no
  recovery path.

## Not automated yet

Off-site copies and an automated restore drill are tracked in the
`robust-upgrades` change. Until they land, copy encrypted backups to storage you
control and periodically test a restore with `make restore`.
