#!/usr/bin/env bash
#
# Back up the local database to ./backups as an encrypted, compressed SQL dump.
# Encrypted by default with GPG symmetric AES-256; the passphrase comes from
# BACKUP_PASSPHRASE in .env. Safe to run while the stack is up.
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
BACKUP_PASSPHRASE="${BACKUP_PASSPHRASE:-}"

if [[ -z "$BACKUP_PASSPHRASE" || "$BACKUP_PASSPHRASE" == "change-me" ]]; then
  echo "BACKUP_PASSPHRASE is not set." >&2
  echo "Backups are encrypted by default; add a strong passphrase to .env." >&2
  exit 1
fi

if ! command -v gpg >/dev/null 2>&1; then
  echo "gpg is required for encrypted backups." >&2
  exit 1
fi

if ! docker compose exec -T db true >/dev/null 2>&1; then
  echo "The db container is not running. Start the stack with 'make up' first." >&2
  exit 1
fi

mkdir -p "$BACKUP_DIR"
timestamp="$(date -u +%Y%m%dT%H%M%SZ)"
file="$BACKUP_DIR/networth-${timestamp}.sql.gz.gpg"

echo "Backing up database '${POSTGRES_DB}' to ${file} (GPG AES-256) ..."
docker compose exec -T db pg_dump -U "$POSTGRES_USER" -d "$POSTGRES_DB" \
  | gzip \
  | gpg --batch --yes --pinentry-mode loopback \
        --symmetric --cipher-algo AES256 \
        --passphrase "$BACKUP_PASSPHRASE" \
        --output "$file"

# Verify by decrypting and testing the gzip stream.
if ! gpg --batch --yes --pinentry-mode loopback --decrypt \
      --passphrase "$BACKUP_PASSPHRASE" "$file" 2>/dev/null | gzip -t; then
  echo "Backup verification failed; removing ${file}." >&2
  rm -f "$file"
  exit 1
fi

if [[ ! -s "$file" ]]; then
  echo "Backup failed: the archive is empty." >&2
  rm -f "$file"
  exit 1
fi

echo "Encrypted backup OK ($(du -h "$file" | cut -f1))."

mapfile -t old < <(ls -1t "$BACKUP_DIR"/networth-*.sql.gz.gpg 2>/dev/null | tail -n +$((BACKUP_KEEP + 1)) || true)
if [[ ${#old[@]} -gt 0 ]]; then
  rm -f "${old[@]}"
  echo "Pruned ${#old[@]} old backup(s); keeping the last ${BACKUP_KEEP}."
fi
