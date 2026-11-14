#!/usr/bin/env bash
#
# Full encrypted backup from inside the backend container.
#
# The API calls this script with the container PG* connection variables. It
# writes the database dump and the encrypted document archive to BACKUP_DIR,
# verifies both, and applies local retention. The API chooses BACKUP_TIMESTAMP
# so it can return the exact filenames it created.
set -euo pipefail

umask 077

BACKUP_DIR="${BACKUP_DIR:-/backups}"
BACKUP_KEEP="${BACKUP_KEEP:-14}"
BACKUP_PASSPHRASE="${BACKUP_PASSPHRASE:-}"
DOCUMENT_DIR="${DOCUMENT_DIR:-/documents}"
BACKUP_TIMESTAMP="${BACKUP_TIMESTAMP:-$(date -u +%Y%m%dT%H%M%SZ)}"

if [[ -z "$BACKUP_PASSPHRASE" || "$BACKUP_PASSPHRASE" == "change-me" ]]; then
  echo "BACKUP_PASSPHRASE is not set." >&2
  exit 1
fi
if ! [[ "$BACKUP_KEEP" =~ ^[0-9]+$ ]] || [[ "$BACKUP_KEEP" -lt 1 ]]; then
  echo "BACKUP_KEEP must be a positive integer." >&2
  exit 1
fi
if ! command -v pg_dump >/dev/null 2>&1; then
  echo "pg_dump is required for a container backup." >&2
  exit 1
fi
if [[ -z "${PGHOST:-}" && -z "${DATABASE_URL:-}" ]]; then
  echo "PGHOST or DATABASE_URL is required for a container backup." >&2
  exit 1
fi
if ! command -v gpg >/dev/null 2>&1; then
  echo "gpg is required for encrypted backups." >&2
  exit 1
fi

gpg_encrypt() {
  gpg --batch --yes --pinentry-mode loopback --passphrase-fd 3 \
    --symmetric --cipher-algo AES256 --output "$1" \
    3< <(printf '%s' "$BACKUP_PASSPHRASE")
}

gpg_decrypt() {
  gpg --batch --yes --pinentry-mode loopback --passphrase-fd 3 \
    --decrypt "$1" \
    3< <(printf '%s' "$BACKUP_PASSPHRASE")
}

mkdir -p "$BACKUP_DIR"
file="$BACKUP_DIR/networth-${BACKUP_TIMESTAMP}.sql.gz.gpg"

echo "Backing up PostgreSQL to ${file} (GPG AES-256) ..."
if [[ -n "${PGHOST:-}" ]]; then
  dump=(pg_dump)
else
  dump=(pg_dump "$DATABASE_URL")
fi
if ! "${dump[@]}" | gzip | gpg_encrypt "$file"; then
  echo "Database backup failed." >&2
  rm -f "$file"
  exit 1
fi

if ! gpg_decrypt "$file" 2>/dev/null | gzip -t; then
  echo "Database backup verification failed." >&2
  rm -f "$file"
  exit 1
fi
if [[ ! -s "$file" ]]; then
  echo "Database backup is empty." >&2
  rm -f "$file"
  exit 1
fi

docs_file=""
if [[ -d "$DOCUMENT_DIR" ]] && find "$DOCUMENT_DIR" -maxdepth 1 -type f -print -quit | grep -q .; then
  docs_file="$BACKUP_DIR/networth-documents-${BACKUP_TIMESTAMP}.tar.gz.gpg"
  echo "Backing up encrypted documents to ${docs_file} ..."
  if ! tar -C "$(dirname "$DOCUMENT_DIR")" -czf - "$(basename "$DOCUMENT_DIR")" \
    | gpg_encrypt "$docs_file"; then
    echo "Document backup failed." >&2
    rm -f "$file" "$docs_file"
    exit 1
  fi
  if ! gpg_decrypt "$docs_file" 2>/dev/null | tar -tzf - >/dev/null; then
    echo "Document backup verification failed." >&2
    rm -f "$file" "$docs_file"
    exit 1
  fi
fi

mapfile -t old < <(ls -1t "$BACKUP_DIR"/networth-*.sql.gz.gpg 2>/dev/null | tail -n +$((BACKUP_KEEP + 1)) || true)
if [[ ${#old[@]} -gt 0 ]]; then
  rm -f "${old[@]}"
  echo "Pruned ${#old[@]} old database backup(s)."
fi
mapfile -t old < <(ls -1t "$BACKUP_DIR"/networth-documents-*.tar.gz.gpg 2>/dev/null | tail -n +$((BACKUP_KEEP + 1)) || true)
if [[ ${#old[@]} -gt 0 ]]; then
  rm -f "${old[@]}"
  echo "Pruned ${#old[@]} old document backup(s)."
fi

echo "Backup complete: ${file}"
if [[ -n "$docs_file" ]]; then
  echo "Document archive: ${docs_file}"
fi
