#!/usr/bin/env bash
#
# Restore a database dump produced by scripts/backup.sh.
#
# This REPLACES the current database. It stops the backend, drops and recreates
# the database, restores the dump, and starts the backend (which re-runs any
# pending migrations). Encrypted .gpg dumps are decrypted with
# BACKUP_PASSPHRASE; legacy plain .sql.gz dumps are still supported.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

file="${1:-}"
if [[ -z "$file" || ! -f "$file" ]]; then
  echo "Usage: $0 <backup.sql.gz.gpg>" >&2
  echo "Example: $0 backups/networth-20261028T120000Z.sql.gz.gpg" >&2
  exit 1
fi

if [[ ! -f .env ]]; then
  echo "No .env found. Copy .env.example to .env first." >&2
  exit 1
fi

set -a
# shellcheck disable=SC1091
. ./.env
set +a

POSTGRES_USER="${POSTGRES_USER:-networth}"
POSTGRES_DB="${POSTGRES_DB:-networth}"
BACKEND_PORT="${BACKEND_PORT:-8080}"
BACKUP_PASSPHRASE="${BACKUP_PASSPHRASE:-}"
DB_CONTAINER="${DB_CONTAINER:-}"
RESTORE_SKIP_BACKEND="${RESTORE_SKIP_BACKEND:-0}"

db_psql() {
  if [[ -n "$DB_CONTAINER" ]]; then
    docker exec -i "$DB_CONTAINER" psql "$@"
  else
    docker compose exec -T db psql "$@"
  fi
}

# Keep the passphrase out of the process argument list; GPG reads it from fd 3
# while the restore data uses stdin.
gpg_decrypt() {
  gpg --batch --yes --pinentry-mode loopback --passphrase-fd 3 \
    --decrypt "$1" \
    3< <(printf '%s' "$BACKUP_PASSPHRASE")
}

encrypted=0
if [[ "$file" == *.gpg ]]; then
  encrypted=1
  if [[ -z "$BACKUP_PASSPHRASE" || "$BACKUP_PASSPHRASE" == "change-me" ]]; then
    echo "BACKUP_PASSPHRASE is required to restore an encrypted backup." >&2
    exit 1
  fi
  if ! command -v gpg >/dev/null 2>&1; then
    echo "gpg is required to restore an encrypted backup." >&2
    exit 1
  fi
fi

echo "Verifying the backup before changing anything ..."
if [[ "$encrypted" -eq 1 ]]; then
  if ! gpg_decrypt "$file" 2>/dev/null | gzip -t; then
    echo "Cannot decrypt or verify the backup. The database was not changed." >&2
    exit 1
  fi
else
  if ! gzip -t "$file"; then
    echo "Cannot verify the backup. The database was not changed." >&2
    exit 1
  fi
fi

echo "This will REPLACE database '${POSTGRES_DB}' with:"
echo "  ${file}"
read -r -p "Type 'restore' to continue: " answer
if [[ "$answer" != "restore" ]]; then
  echo "Aborted."
  exit 1
fi

if [[ "$RESTORE_SKIP_BACKEND" == "1" ]]; then
  echo "Skipping backend stop/start (RESTORE_SKIP_BACKEND=1)."
else
  echo "Stopping the backend ..."
  docker compose stop backend >/dev/null
fi

echo "Recreating the database ..."
db_psql -U "$POSTGRES_USER" -d postgres \
  -c "DROP DATABASE IF EXISTS \"$POSTGRES_DB\" WITH (FORCE);" >/dev/null
db_psql -U "$POSTGRES_USER" -d postgres \
  -c "CREATE DATABASE \"$POSTGRES_DB\";" >/dev/null

echo "Restoring ..."
if [[ "$encrypted" -eq 1 ]]; then
  gpg_decrypt "$file" \
    | gunzip \
    | db_psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" >/dev/null
else
  gunzip -c "$file" \
    | db_psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" >/dev/null
fi

if [[ "$RESTORE_SKIP_BACKEND" == "1" ]]; then
  echo "Restore complete (backend left stopped)."
  exit 0
fi

echo "Starting the backend ..."
docker compose start backend >/dev/null

for _ in $(seq 1 60); do
  if curl -sf "http://localhost:${BACKEND_PORT}/health" >/dev/null 2>&1; then
    echo "Restore complete; the backend is healthy."
    exit 0
  fi
  sleep 1
done

echo "Restore finished but the backend is not healthy." >&2
echo "Check 'docker compose logs backend'." >&2
exit 1
