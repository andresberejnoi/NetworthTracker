#!/usr/bin/env bash
#
# Run an encrypted database backup from cron or a systemd timer.
#
# This wrapper serializes runs, writes a timestamped log, and delegates the
# actual dump, encryption, verification, and retention to scripts/backup.sh.
# It fails closed when the passphrase is missing/placeholder or when the backup
# command fails, so a timer never silently writes plaintext or skips protection.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

if [[ ! -f .env ]]; then
  echo "No .env found. Copy .env.example to .env first." >&2
  exit 1
fi

set -a
# shellcheck disable=SC1091
. ./.env
set +a

BACKUP_DIR="${BACKUP_DIR:-$root/backups}"
BACKUP_LOG_DIR="${BACKUP_LOG_DIR:-$BACKUP_DIR}"
BACKUP_PASSPHRASE="${BACKUP_PASSPHRASE:-}"

umask 077
mkdir -p "$BACKUP_DIR" "$BACKUP_LOG_DIR"
log="$BACKUP_LOG_DIR/backup.log"
lock="$BACKUP_LOG_DIR/.backup.lock"

exec >>"$log" 2>&1

timestamp() {
  date -u +%Y-%m-%dT%H:%M:%SZ
}

echo "== $(timestamp) scheduled backup starting =="

if [[ -z "$BACKUP_PASSPHRASE" || "$BACKUP_PASSPHRASE" == "change-me" ]]; then
  echo "ERROR: BACKUP_PASSPHRASE is missing or a placeholder; refusing to write a backup." >&2
  exit 1
fi

if ! command -v gpg >/dev/null 2>&1; then
  echo "ERROR: gpg is required for encrypted backups." >&2
  exit 1
fi

if command -v flock >/dev/null 2>&1; then
  exec 9>"$lock"
  if ! flock -n 9; then
    echo "Another backup is already running; skipping this run."
    exit 0
  fi
else
  # Portable fallback for systems without flock. Remove a stale lock left by a
  # process that no longer exists.
  lock_dir="$lock.d"
  if ! mkdir "$lock_dir" 2>/dev/null; then
    holder="$(cat "$lock_dir/pid" 2>/dev/null || true)"
    if [[ -n "$holder" ]] && kill -0 "$holder" 2>/dev/null; then
      echo "Another backup is already running; skipping this run."
      exit 0
    fi
    echo "Removing stale backup lock ${lock_dir}."
    rm -rf "$lock_dir"
    if ! mkdir "$lock_dir" 2>/dev/null; then
      echo "Another backup is already running; skipping this run."
      exit 0
    fi
  fi
  echo "$$" > "$lock_dir/pid"
  trap 'rm -rf "$lock_dir"' EXIT
fi

if ! "$root/scripts/backup.sh"; then
  echo "ERROR: backup.sh failed; scheduled backup did not complete." >&2
  exit 1
fi

echo "== $(timestamp) scheduled backup complete =="
