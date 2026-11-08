#!/usr/bin/env bash
#
# Restore the newest encrypted backup into a throwaway Postgres container and
# verify that the restored schema, rows, and decimal text are intact.
#
# This never touches the real database. Pass a backup path as the first
# argument to drill a specific file; otherwise the newest backup in BACKUP_DIR
# is used. Set RESTORE_DRILL_ENV_FILE to use an alternate .env file.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

env_file="${RESTORE_DRILL_ENV_FILE:-$root/.env}"
if [[ ! -f "$env_file" ]]; then
  echo "No .env found at ${env_file}. Copy .env.example to .env first." >&2
  exit 1
fi

set -a
# shellcheck disable=SC1090
. "$env_file"
set +a

POSTGRES_USER="${POSTGRES_USER:-networth}"
POSTGRES_DB="${POSTGRES_DB:-networth}"
BACKUP_DIR="${BACKUP_DIR:-$root/backups}"
BACKUP_PASSPHRASE="${BACKUP_PASSPHRASE:-}"
POSTGRES_IMAGE="${POSTGRES_IMAGE:-postgres:17-alpine}"
DOCKER_CMD="${DOCKER_CMD:-docker}"
RESTORE_DRILL_BACKUP="${RESTORE_DRILL_BACKUP:-}"

backup_file="${1:-$RESTORE_DRILL_BACKUP}"
if [[ -z "$backup_file" ]]; then
  backup_file="$(ls -1t "$BACKUP_DIR"/networth-*.sql.gz.gpg 2>/dev/null | head -1 || true)"
fi
if [[ -z "$backup_file" || ! -f "$backup_file" ]]; then
  echo "No backup found. Pass a backup path or create one with 'make backup'." >&2
  exit 1
fi
backup_file="$(cd "$(dirname "$backup_file")" && pwd)/$(basename "$backup_file")"

if [[ "$backup_file" == *.gpg && ( -z "$BACKUP_PASSPHRASE" || "$BACKUP_PASSPHRASE" == "change-me" ) ]]; then
  echo "BACKUP_PASSPHRASE is required to restore an encrypted backup." >&2
  exit 1
fi

if ! command -v "$DOCKER_CMD" >/dev/null 2>&1; then
  echo "The container command '${DOCKER_CMD}' is not available." >&2
  exit 1
fi

work_dir="$(mktemp -d "${TMPDIR:-/tmp}/networth-restore-drill.XXXXXX")"
container="networth-restore-drill-$$"
password="$(python3 -c 'import secrets; print(secrets.token_hex(16))')"

cleanup() {
  "$DOCKER_CMD" rm -f "$container" >/dev/null 2>&1 || true
  rm -rf "$work_dir"
}
trap cleanup EXIT

psql_db() {
  "$DOCKER_CMD" exec -i "$container" psql -v ON_ERROR_STOP=1 -U "$POSTGRES_USER" -d "$POSTGRES_DB" "$@"
}

echo "Starting throwaway Postgres (${POSTGRES_IMAGE}) ..."
"$DOCKER_CMD" run -d --rm --name "$container" \
  -e POSTGRES_USER="$POSTGRES_USER" \
  -e POSTGRES_PASSWORD="$password" \
  -e POSTGRES_DB="$POSTGRES_DB" \
  "$POSTGRES_IMAGE" >/dev/null

for _ in $(seq 1 240); do
  if "$DOCKER_CMD" exec "$container" psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" \
       -tAc 'select 1' >/dev/null 2>&1; then
    break
  fi
  sleep 0.5
done
if ! "$DOCKER_CMD" exec "$container" psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" \
     -tAc 'select 1' >/dev/null 2>&1; then
  echo "Throwaway Postgres did not become ready." >&2
  "$DOCKER_CMD" logs "$container" >&2 || true
  exit 1
fi

mkdir -p "$work_dir/scripts"
cp "$root/scripts/restore.sh" "$work_dir/scripts/restore.sh"
chmod +x "$work_dir/scripts/restore.sh"
cat > "$work_dir/.env" <<EOF
POSTGRES_USER=${POSTGRES_USER}
POSTGRES_DB=${POSTGRES_DB}
BACKUP_PASSPHRASE=${BACKUP_PASSPHRASE}
DB_CONTAINER=${container}
RESTORE_SKIP_BACKEND=1
EOF
chmod 600 "$work_dir/.env"

echo "Restoring ${backup_file} into the throwaway database ..."
printf 'restore\n' | "$work_dir/scripts/restore.sh" "$backup_file" >/dev/null

migrations="$(psql_db -tAc 'SELECT COUNT(*) FROM _sqlx_migrations')"
accounts="$(psql_db -tAc 'SELECT COUNT(*) FROM accounts')"
balances="$(psql_db -tAc 'SELECT COUNT(*) FROM account_balances')"
snapshots="$(psql_db -tAc 'SELECT COUNT(*) FROM net_worth_snapshots')"

if [[ ! "$migrations" =~ ^[0-9]+$ ]] || [[ "$migrations" -lt 1 ]]; then
  echo "Restore drill failed: migration history is missing." >&2
  exit 1
fi
if [[ ! "$accounts" =~ ^[0-9]+$ || ! "$balances" =~ ^[0-9]+$ || ! "$snapshots" =~ ^[0-9]+$ ]]; then
  echo "Restore drill failed: expected tables are missing." >&2
  exit 1
fi

plain_balances="$(psql_db -tAc \
  "SELECT COUNT(*) FROM account_balances WHERE amount::text ~ '^-?[0-9]+([.][0-9]+)?$'")"
if [[ "$plain_balances" != "$balances" ]]; then
  echo "Restore drill failed: balances contain non-decimal text." >&2
  exit 1
fi

sample="$(psql_db -tAc \
  'SELECT amount::text FROM account_balances ORDER BY as_of DESC, id LIMIT 1')"
echo "Restore drill passed: migrations=${migrations} accounts=${accounts} balances=${balances} snapshots=${snapshots} latest_amount=${sample:-none}"
