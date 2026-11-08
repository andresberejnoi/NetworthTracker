#!/usr/bin/env bash
#
# Back up the database before pending migrations, then start the API.
#
# The backup is encrypted with the same GPG AES-256 scheme as scripts/backup.sh.
# If migrations are pending and the backup cannot be created/verified, startup
# stops so an upgrade cannot run without a recoverable copy.
set -euo pipefail

MIGRATIONS_DIR="${MIGRATIONS_DIR:-/usr/local/share/networth/migrations}"
BACKUP_DIR="${BACKUP_DIR:-/backups}"
BACKUP_KEEP="${BACKUP_KEEP:-14}"
DATABASE_URL="${DATABASE_URL:-}"

if [[ -z "$DATABASE_URL" ]]; then
  echo "DATABASE_URL is required" >&2
  exit 1
fi

psql_db() {
  if [[ -n "${PGHOST:-}" ]]; then
    psql "$@"
  else
    psql "$DATABASE_URL" "$@"
  fi
}

pg_dump_db() {
  if [[ -n "${PGHOST:-}" ]]; then
    pg_dump
  else
    pg_dump "$DATABASE_URL"
  fi
}

for _ in $(seq 1 60); do
  if psql_db -tAc 'select 1' >/dev/null 2>&1; then
    break
  fi
  sleep 1
done
if ! psql_db -tAc 'select 1' >/dev/null 2>&1; then
  echo "Database is not reachable; refusing to start" >&2
  exit 1
fi

has_migrations_table="$(psql_db -tAc \
  "select to_regclass('public._sqlx_migrations') is not null" | tr -d '[:space:]')"
if [[ "$has_migrations_table" != "t" ]]; then
  echo "Fresh database; no pre-migration backup needed."
  exec "$@"
fi

mapfile -t applied < <(psql_db -tAc \
  "select version from _sqlx_migrations order by version" | sed '/^[[:space:]]*$/d')

declare -A embedded=()
pending=0
for file in "$MIGRATIONS_DIR"/*.sql; do
  [[ -e "$file" ]] || continue
  base="$(basename "$file" .sql)"
  version="${base%%_*}"
  embedded["$version"]=1
done

for version in "${!embedded[@]}"; do
  found=0
  for current in "${applied[@]}"; do
    [[ "$current" == "$version" ]] && found=1
  done
  [[ "$found" -eq 1 ]] || pending=1
done

for current in "${applied[@]}"; do
  if [[ -z "${embedded[$current]:-}" ]]; then
    echo "Database contains migration ${current}, which this image does not know." >&2
    echo "Refusing to start an older image against a newer schema." >&2
    exit 1
  fi
done

if [[ "$pending" -eq 0 ]]; then
  echo "No pending migrations."
  exec "$@"
fi

if [[ -z "${BACKUP_PASSPHRASE:-}" || "$BACKUP_PASSPHRASE" == "change-me" ]]; then
  echo "Pending migrations require BACKUP_PASSPHRASE for an encrypted backup." >&2
  echo "Set a strong passphrase in .env and restart." >&2
  exit 1
fi
if ! command -v gpg >/dev/null 2>&1; then
  echo "gpg is required for the pre-migration backup." >&2
  exit 1
fi

mkdir -p "$BACKUP_DIR"
timestamp="$(date -u +%Y%m%dT%H%M%SZ)"
file="$BACKUP_DIR/networth-pre-migration-${timestamp}.sql.gz.gpg"

echo "Pending migrations found; writing encrypted pre-migration backup to ${file} ..."
if ! pg_dump_db \
  | gzip \
  | gpg --batch --yes --pinentry-mode loopback \
        --symmetric --cipher-algo AES256 \
        --passphrase-fd 3 \
        --output "$file" \
        3< <(printf '%s' "$BACKUP_PASSPHRASE"); then
  echo "Pre-migration backup failed; refusing to start." >&2
  rm -f "$file"
  exit 1
fi

if ! gpg --batch --yes --pinentry-mode loopback --decrypt \
      --passphrase-fd 3 "$file" 3< <(printf '%s' "$BACKUP_PASSPHRASE") \
      2>/dev/null | gzip -t; then
  echo "Pre-migration backup verification failed; refusing to start." >&2
  rm -f "$file"
  exit 1
fi

mapfile -t old < <(ls -1t "$BACKUP_DIR"/networth-pre-migration-*.sql.gz.gpg 2>/dev/null \
  | tail -n +$((BACKUP_KEEP + 1)) || true)
if [[ ${#old[@]} -gt 0 ]]; then
  rm -f "${old[@]}"
fi

echo "Pre-migration backup OK: ${file}"
exec "$@"
