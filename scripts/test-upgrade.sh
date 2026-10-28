#!/usr/bin/env bash
#
# End-to-end upgrade, encrypted backup, and restore test.
#
# It builds a database at the *previous* schema (every migration except the
# latest), seeds fake data, then starts the current backend. The backend applies
# the remaining migration. The test asserts:
#   - the schema upgrade preserved the fake data and exact NUMERIC values;
#   - the upgraded schema is writable;
#   - GET /api/v1/ping reports the authoritative version;
#   - an encrypted backup can be decrypted and restored;
#   - a wrong passphrase is rejected without changing the database.
#
# It uses a throwaway Postgres container and temporary copies of the backup and
# restore scripts, so it never touches your real database or ./backups.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

backend_port="${UPGRADE_TEST_BACKEND_PORT:-18081}"
db_port="${UPGRADE_TEST_DB_PORT:-15434}"
db_container="networth-upgrade-db-$$"
db_password="$(python3 -c 'import secrets; print(secrets.token_hex(16))')"
backup_passphrase="$(python3 -c 'import secrets; print(secrets.token_hex(24))')"
work_dir="$(mktemp -d "${TMPDIR:-/tmp}/networth-upgrade-test.XXXXXX")"
backup_dir="$work_dir/backups"
url="http://127.0.0.1:${backend_port}"
expected_version="$("$root/scripts/version.sh")"
backend_pid=""

cleanup() {
  if [[ -n "$backend_pid" ]]; then
    kill "$backend_pid" >/dev/null 2>&1 || true
    wait "$backend_pid" >/dev/null 2>&1 || true
  fi
  docker rm -f "$db_container" >/dev/null 2>&1 || true
  rm -rf "$work_dir"
}
trap cleanup EXIT

psql_db() {
  docker exec -i "$db_container" psql -v ON_ERROR_STOP=1 -U networth -d networth "$@"
}

start_backend() {
  DATABASE_URL="postgres://networth:${db_password}@127.0.0.1:${db_port}/networth" \
    BACKEND_ADDR="127.0.0.1:${backend_port}" \
    AUTH_DISABLED=true \
    "$root/backend/target/debug/networth-api" &
  backend_pid=$!

  for _ in $(seq 1 120); do
    if curl -sf "${url}/health" >/dev/null 2>&1; then
      return 0
    fi
    sleep 0.25
  done

  echo "Backend did not become healthy at ${url}" >&2
  exit 1
}

stop_backend() {
  if [[ -n "$backend_pid" ]]; then
    kill "$backend_pid" >/dev/null 2>&1 || true
    wait "$backend_pid" >/dev/null 2>&1 || true
    backend_pid=""
  fi
}

reported_version() {
  curl -s "${url}/api/v1/ping" \
    | python3 -c 'import sys, json; print(json.load(sys.stdin)["version"])'
}

assert_data() {
  local accounts balances snapshots users applied securities security_rows

  accounts="$(psql_db -tAc 'SELECT COUNT(*) FROM accounts')"
  balances="$(psql_db -tAc 'SELECT amount::text FROM account_balances')"
  snapshots="$(psql_db -tAc 'SELECT COUNT(*) FROM net_worth_snapshots')"
  users="$(psql_db -tAc 'SELECT COUNT(*) FROM users')"
  applied="$(psql_db -tAc 'SELECT COUNT(*) FROM _sqlx_migrations')"
  securities="$(psql_db -tAc "SELECT to_regclass('public.securities') IS NOT NULL")"
  security_rows="$(psql_db -tAc 'SELECT COUNT(*) FROM securities')"

  [[ "$accounts" == "1" ]] || { echo "accounts: expected 1, got ${accounts}" >&2; return 1; }
  [[ "$balances" == "1234.56" ]] || { echo "balance: expected 1234.56, got ${balances}" >&2; return 1; }
  [[ "$snapshots" == "1" ]] || { echo "snapshots: expected 1, got ${snapshots}" >&2; return 1; }
  [[ "$users" == "1" ]] || { echo "users: expected 1, got ${users}" >&2; return 1; }
  [[ "$applied" == "${#migrations[@]}" ]] || {
    echo "migrations: expected ${#migrations[@]}, got ${applied}" >&2
    return 1
  }
  [[ "$securities" == "t" ]] || { echo "securities table is missing" >&2; return 1; }
  [[ "$security_rows" == "1" ]] || {
    echo "securities: expected 1 row, got ${security_rows}" >&2
    return 1
  }
}

echo "Starting throwaway Postgres on 127.0.0.1:${db_port} ..."
docker run -d --rm --name "$db_container" \
  -e POSTGRES_USER=networth \
  -e POSTGRES_PASSWORD="$db_password" \
  -e POSTGRES_DB=networth \
  -p "127.0.0.1:${db_port}:5432" \
  postgres:17-alpine >/dev/null

for _ in $(seq 1 60); do
  if docker exec "$db_container" pg_isready -U networth >/dev/null 2>&1; then
    break
  fi
  sleep 0.5
done

if ! docker exec "$db_container" pg_isready -U networth >/dev/null 2>&1; then
  echo "Postgres did not become ready" >&2
  exit 1
fi

echo "Building backend ..."
(cd "$root/backend" && cargo build -p networth-api)

migrations=("$root"/backend/migrations/*.sql)
if [[ ${#migrations[@]} -lt 2 ]]; then
  echo "Need at least two migrations to test an upgrade" >&2
  exit 1
fi
last_index=$((${#migrations[@]} - 1))

echo "Creating the previous schema (${last_index} of ${#migrations[@]} migrations) ..."
psql_db -c "
  CREATE TABLE _sqlx_migrations (
    version BIGINT PRIMARY KEY,
    description TEXT NOT NULL,
    installed_on TIMESTAMPTZ NOT NULL DEFAULT now(),
    success BOOLEAN NOT NULL,
    checksum BYTEA NOT NULL,
    execution_time BIGINT NOT NULL
  );" >/dev/null

for i in "${!migrations[@]}"; do
  [[ "$i" -eq "$last_index" ]] && continue

  file="${migrations[$i]}"
  base="$(basename "$file" .sql)"
  version="${base%%_*}"
  description="${base#*_}"
  checksum="$(openssl dgst -sha384 -r "$file" | cut -d' ' -f1)"

  psql_db -f - < "$file" >/dev/null
  psql_db -c "
    INSERT INTO _sqlx_migrations
      (version, description, success, checksum, execution_time)
    VALUES
      (${version}, '${description}', true, decode('${checksum}', 'hex'), 0);" >/dev/null
done

echo "Seeding fake data ..."
psql_db <<'SQL' >/dev/null
INSERT INTO accounts (name, account_type, currency)
VALUES ('Upgrade checking', 'checking', 'USD');

INSERT INTO account_balances (account_id, as_of, amount, currency)
SELECT id, DATE '2026-01-01', 1234.56, 'USD'
FROM accounts
WHERE name = 'Upgrade checking';

INSERT INTO net_worth_snapshots
  (as_of, scope, currency, total, liquid, liquidity, accessible,
   tax_rate, penalty_rate, penalty_age, selling_rate, fixed_costs)
VALUES
  (DATE '2026-01-01', 'household', 'USD',
   1234.56, 1234.56, 1234.56, 1234.56, 0.2, 0.1, 59, 0.06, 0);

INSERT INTO users (username, display_name, password_hash, role)
VALUES ('upgrade-owner', 'Upgrade Owner', 'hash', 'owner');
SQL

if [[ "$(psql_db -tAc "SELECT to_regclass('public.securities') IS NOT NULL")" != "f" ]]; then
  echo "The previous schema unexpectedly already has securities" >&2
  exit 1
fi
if [[ "$(psql_db -tAc 'SELECT COUNT(*) FROM _sqlx_migrations')" != "$last_index" ]]; then
  echo "The previous schema has the wrong migration count" >&2
  exit 1
fi

echo "Starting the upgraded backend (applies the final migration) ..."
start_backend

version="$(reported_version)"
if [[ "$version" != "$expected_version" ]]; then
  echo "Version mismatch after upgrade: expected ${expected_version}, got ${version}" >&2
  exit 1
fi
echo "Version after upgrade: ${version}"

if ! curl -s "${url}/api/v1/accounts" | grep -q "Upgrade checking"; then
  echo "The upgraded API did not return the seeded account" >&2
  exit 1
fi

psql_db -c "INSERT INTO securities (symbol, name, asset_class, currency)
             VALUES ('VTI', 'Total Market', 'equity', 'USD');" >/dev/null
assert_data
echo "Schema upgrade preserved data and the new schema is writable."

# Use temporary copies so the test never reads or writes the real .env/backups.
mkdir -p "$work_dir/scripts"
cp "$root/scripts/backup.sh" "$root/scripts/restore.sh" "$work_dir/scripts/"
cat > "$work_dir/.env" <<EOF
POSTGRES_USER=networth
POSTGRES_DB=networth
BACKUP_PASSPHRASE=${backup_passphrase}
BACKUP_DIR=${backup_dir}
BACKUP_KEEP=2
DB_CONTAINER=${db_container}
RESTORE_SKIP_BACKEND=1
EOF
chmod 600 "$work_dir/.env"

echo "Taking an encrypted backup ..."
"$work_dir/scripts/backup.sh"
backup_file="$(ls -1t "$backup_dir"/networth-*.sql.gz.gpg | head -1)"
if [[ ! -s "$backup_file" ]]; then
  echo "Encrypted backup was not created" >&2
  exit 1
fi

stop_backend

echo "Checking that a wrong passphrase is rejected before changing data ..."
cp "$work_dir/.env" "$work_dir/.env.good"
sed -i 's/^BACKUP_PASSPHRASE=.*/BACKUP_PASSPHRASE=wrong-passphrase/' "$work_dir/.env"
if printf 'restore\n' | "$work_dir/scripts/restore.sh" "$backup_file" >/dev/null 2>&1; then
  echo "Restore with a wrong passphrase unexpectedly succeeded" >&2
  exit 1
fi
mv "$work_dir/.env.good" "$work_dir/.env"
if [[ "$(psql_db -tAc 'SELECT COUNT(*) FROM accounts')" != "1" ]]; then
  echo "A failed decryption changed the database" >&2
  exit 1
fi
echo "Wrong passphrase rejected; data was untouched."

echo "Checking that a corrupt backup is rejected before changing data ..."
head -c 100 "$backup_file" > "$work_dir/corrupt.sql.gz.gpg"
if printf 'restore\n' | "$work_dir/scripts/restore.sh" "$work_dir/corrupt.sql.gz.gpg" >/dev/null 2>&1; then
  echo "Restore of a corrupt backup unexpectedly succeeded" >&2
  exit 1
fi
if [[ "$(psql_db -tAc 'SELECT COUNT(*) FROM accounts')" != "1" ]]; then
  echo "A corrupt backup changed the database" >&2
  exit 1
fi
echo "Corrupt backup rejected; data was untouched."

echo "Restoring the encrypted backup ..."
printf 'restore\n' | "$work_dir/scripts/restore.sh" "$backup_file" >/dev/null

start_backend
version="$(reported_version)"
if [[ "$version" != "$expected_version" ]]; then
  echo "Version mismatch after restore: expected ${expected_version}, got ${version}" >&2
  exit 1
fi
assert_data
echo "Encrypted backup restored; version and data are correct."

echo "Upgrade, encrypted backup, and restore test passed."
