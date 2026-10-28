#!/usr/bin/env bash
#
# Back up the local database to ./backups as a compressed SQL dump.
# Safe to run while the stack is up.
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

POSTGRES_USER="${POSTGRES_USER:-networth}"
POSTGRES_DB="${POSTGRES_DB:-networth}"
BACKUP_DIR="${BACKUP_DIR:-$root/backups}"
BACKUP_KEEP="${BACKUP_KEEP:-14}"

if ! docker compose exec -T db true >/dev/null 2>&1; then
  echo "The db container is not running. Start the stack with 'make up' first." >&2
  exit 1
fi

mkdir -p "$BACKUP_DIR"
timestamp="$(date -u +%Y%m%dT%H%M%SZ)"
file="$BACKUP_DIR/networth-${timestamp}.sql.gz"

echo "Backing up database '${POSTGRES_DB}' to ${file} ..."
docker compose exec -T db pg_dump -U "$POSTGRES_USER" -d "$POSTGRES_DB" | gzip > "$file"

# Verify the archive is valid and non-empty.
gzip -t "$file"
if [[ ! -s "$file" ]]; then
  echo "Backup failed: the archive is empty." >&2
  rm -f "$file"
  exit 1
fi

echo "Backup OK ($(du -h "$file" | cut -f1))."

mapfile -t old < <(ls -1t "$BACKUP_DIR"/networth-*.sql.gz 2>/dev/null | tail -n +$((BACKUP_KEEP + 1)) || true)
if [[ ${#old[@]} -gt 0 ]]; then
  rm -f "${old[@]}"
  echo "Pruned ${#old[@]} old backup(s); keeping the last ${BACKUP_KEEP}."
fi
