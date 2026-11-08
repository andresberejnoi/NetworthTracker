#!/usr/bin/env bash
#
# Upgrade fixture databases from supported release tags to the current backend.
#
# For each tag (the newest four by default), this script creates a temporary
# git worktree, applies the migrations embedded in that tag, seeds representative
# rows, starts the current backend, and asserts that:
#   - the current migrations apply on top of the tag's schema;
#   - accounts, balances, snapshots, users, and tax lots survive exactly;
#   - the current schema (crypto, budgeting, aggregation) exists;
#   - GET /api/v1/ping reports the current version.
#
# The matrix is generated from release tags, not a hand-maintained list. Set
# UPGRADE_MATRIX_TAGS to test specific tags, or UPGRADE_MATRIX_LIMIT=0 for all
# tags. It uses throwaway Postgres containers and never touches .env or
# ./backups.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

current_version="$("$root/scripts/version.sh")"
mapfile -t current_migrations < <(find "$root/backend/migrations" -maxdepth 1 -name '*.sql' -print | sort)
if [[ ${#current_migrations[@]} -eq 0 ]]; then
  echo "No current migrations found" >&2
  exit 1
fi
current_count=${#current_migrations[@]}

release_tags() {
  local tag
  git tag --list 'v[0-9]*' --sort=version:refname | while read -r tag; do
    if git merge-base --is-ancestor "$tag" HEAD 2>/dev/null; then
      echo "$tag"
    fi
  done
}

if [[ -n "${UPGRADE_MATRIX_TAGS:-}" ]]; then
  read -r -a tags <<< "$UPGRADE_MATRIX_TAGS"
else
  limit="${UPGRADE_MATRIX_LIMIT:-4}"
  if [[ "$limit" == "0" ]]; then
    mapfile -t tags < <(release_tags)
  else
    mapfile -t tags < <(release_tags | tail -n "$limit")
  fi
fi

if [[ ${#tags[@]} -eq 0 ]]; then
  echo "No release tags found. Fetch tags with: git fetch --tags" >&2
  exit 1
fi

work_dir="$(mktemp -d "${TMPDIR:-/tmp}/networth-upgrade-matrix.XXXXXX")"
db_password="$(python3 -c 'import secrets; print(secrets.token_hex(16))')"
db_container=""
db_port=""
backend_pid=""
backend_port=""
url=""
tag_dir=""

pick_port() {
  python3 -c 'import socket; s=socket.socket(); s.bind(("127.0.0.1", 0)); print(s.getsockname()[1]); s.close()'
}

stop_backend() {
  if [[ -n "$backend_pid" ]]; then
    kill "$backend_pid" >/dev/null 2>&1 || true
    wait "$backend_pid" >/dev/null 2>&1 || true
    backend_pid=""
  fi
}

cleanup() {
  stop_backend
  if [[ -n "$db_container" ]]; then
    docker rm -f "$db_container" >/dev/null 2>&1 || true
    db_container=""
  fi
  if [[ -n "$tag_dir" ]]; then
    git worktree remove --force "$tag_dir" >/dev/null 2>&1 || true
    rm -rf "$tag_dir"
    tag_dir=""
  fi
  rm -rf "$work_dir"
}
trap cleanup EXIT

psql_db() {
  docker exec -i "$db_container" psql -v ON_ERROR_STOP=1 -U networth -d networth "$@"
}

start_db() {
  db_port="$(pick_port)"
  db_container="networth-matrix-db-$$-$1"
  docker run -d --rm --name "$db_container" \
    -e POSTGRES_USER=networth \
    -e POSTGRES_PASSWORD="$db_password" \
    -e POSTGRES_DB=networth \
    -p "127.0.0.1:${db_port}:5432" \
    postgres:17-alpine >/dev/null

  for _ in $(seq 1 240); do
    if docker exec "$db_container" pg_isready -U networth >/dev/null 2>&1; then
      return 0
    fi
    sleep 0.5
  done

  echo "Postgres did not become ready for ${1}" >&2
  docker logs "$db_container" >&2 || true
  exit 1
}

start_backend() {
  local log
  backend_port="$(pick_port)"
  while [[ "$backend_port" == "$db_port" ]]; do
    backend_port="$(pick_port)"
  done
  url="http://127.0.0.1:${backend_port}"
  log="$work_dir/backend-${1}.log"

  DATABASE_URL="postgres://networth:${db_password}@127.0.0.1:${db_port}/networth" \
    BACKEND_ADDR="127.0.0.1:${backend_port}" \
    AUTH_DISABLED=true \
    "$current_bin" >"$log" 2>&1 &
  backend_pid=$!

  for _ in $(seq 1 240); do
    if curl -sf "${url}/health" >/dev/null 2>&1; then
      return 0
    fi
    sleep 0.25
  done

  echo "Backend did not become healthy for ${1}; log follows:" >&2
  sed -n '1,120p' "$log" >&2
  exit 1
}

reported_version() {
  curl -s "${url}/api/v1/ping" \
    | python3 -c 'import sys, json; print(json.load(sys.stdin)["version"])'
}

assert_current_schema() {
  local accounts balances snapshots users applied securities lots remaining

  accounts="$(psql_db -tAc 'SELECT COUNT(*) FROM accounts')"
  balances="$(psql_db -tAc 'SELECT amount::text FROM account_balances')"
  snapshots="$(psql_db -tAc 'SELECT COUNT(*) FROM net_worth_snapshots')"
  users="$(psql_db -tAc 'SELECT COUNT(*) FROM users')"
  applied="$(psql_db -tAc 'SELECT COUNT(*) FROM _sqlx_migrations')"
  securities="$(psql_db -tAc 'SELECT COUNT(*) FROM securities')"
  lots="$(psql_db -tAc 'SELECT COUNT(*) FROM tax_lots')"
  remaining="$(psql_db -tAc 'SELECT remaining_quantity::text FROM tax_lots')"

  [[ "$accounts" == "1" ]] || { echo "accounts: expected 1, got ${accounts}" >&2; return 1; }
  [[ "$balances" == "1234.56" ]] || { echo "balance: expected 1234.56, got ${balances}" >&2; return 1; }
  [[ "$snapshots" == "1" ]] || { echo "snapshots: expected 1, got ${snapshots}" >&2; return 1; }
  [[ "$users" == "1" ]] || { echo "users: expected 1, got ${users}" >&2; return 1; }
  [[ "$applied" == "$current_count" ]] || {
    echo "migrations: expected ${current_count}, got ${applied}" >&2
    return 1
  }
  [[ "$securities" == "1" ]] || { echo "securities: expected 1, got ${securities}" >&2; return 1; }
  [[ "$lots" == "1" ]] || { echo "tax_lots: expected 1, got ${lots}" >&2; return 1; }
  [[ "$remaining" == "10" ]] || { echo "tax_lots: expected remaining 10, got ${remaining}" >&2; return 1; }
  [[ "$(psql_db -tAc "SELECT to_regclass('public.crypto_assets') IS NOT NULL")" == "t" ]] || {
    echo "crypto_assets table is missing" >&2
    return 1
  }
  [[ "$(psql_db -tAc "SELECT to_regclass('public.budget_plans') IS NOT NULL")" == "t" ]] || {
    echo "budget_plans table is missing" >&2
    return 1
  }
  [[ "$(psql_db -tAc "SELECT to_regclass('public.aggregation_links') IS NOT NULL")" == "t" ]] || {
    echo "aggregation_links table is missing" >&2
    return 1
  }
}

echo "Building current backend (${current_version}) ..."
(cd "$root/backend" && cargo build -p networth-api)
current_bin="$root/backend/target/debug/networth-api"

for index in "${!tags[@]}"; do
  tag="${tags[$index]}"
  tag_dir="$work_dir/tag-${index}"
  echo
  echo "== ${tag} =="

  git worktree add --detach "$tag_dir" "$tag" >/dev/null
  mapfile -t tag_migrations < <(find "$tag_dir/backend/migrations" -maxdepth 1 -name '*.sql' -print | sort)
  if [[ ${#tag_migrations[@]} -eq 0 ]]; then
    echo "${tag} has no migrations" >&2
    exit 1
  fi
  tag_count=${#tag_migrations[@]}

  start_db "$index"
  psql_db -c "
    CREATE TABLE _sqlx_migrations (
      version BIGINT PRIMARY KEY,
      description TEXT NOT NULL,
      installed_on TIMESTAMPTZ NOT NULL DEFAULT now(),
      success BOOLEAN NOT NULL,
      checksum BYTEA NOT NULL,
      execution_time BIGINT NOT NULL
    );" >/dev/null

  for file in "${tag_migrations[@]}"; do
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

DO $$
DECLARE
  account_uuid UUID;
  security_uuid UUID;
  holding_uuid UUID;
BEGIN
  IF to_regclass('public.users') IS NOT NULL THEN
    INSERT INTO users (username, display_name, password_hash, role)
    VALUES ('upgrade-owner', 'Upgrade Owner', 'hash', 'owner');
  END IF;

  IF to_regclass('public.securities') IS NOT NULL
     AND to_regclass('public.holdings') IS NOT NULL THEN
    SELECT id INTO account_uuid FROM accounts WHERE name = 'Upgrade checking';

    INSERT INTO securities (symbol, name, asset_class, currency)
    VALUES ('VTI', 'Total Market', 'equity', 'USD')
    RETURNING id INTO security_uuid;

    INSERT INTO holdings (account_id, security_id)
    VALUES (account_uuid, security_uuid)
    RETURNING id INTO holding_uuid;

    IF to_regclass('public.holding_lots') IS NOT NULL THEN
      INSERT INTO holding_lots (holding_id, acquired_on, quantity, cost_basis, currency)
      VALUES (holding_uuid, DATE '2026-01-01', 10, 1000.00, 'USD');
    ELSIF to_regclass('public.tax_lots') IS NOT NULL THEN
      INSERT INTO tax_lots
        (holding_id, acquired_on, original_quantity, remaining_quantity,
         original_cost_basis, remaining_cost_basis, currency, fees, source,
         basis_quality)
      VALUES
        (holding_uuid, DATE '2026-01-01', 10, 10, 1000.00, 1000.00, 'USD', 0,
         'import', 'actual');
    END IF;
  END IF;
END $$;
SQL

  if [[ "$(psql_db -tAc 'SELECT COUNT(*) FROM _sqlx_migrations')" != "$tag_count" ]]; then
    echo "${tag} migration count mismatch" >&2
    exit 1
  fi
  if [[ "$(psql_db -tAc 'SELECT amount::text FROM account_balances')" != "1234.56" ]]; then
    echo "${tag} fixture seed failed" >&2
    exit 1
  fi

  start_backend "$tag"
  version="$(reported_version)"
  if [[ "$version" != "$current_version" ]]; then
    echo "Version mismatch for ${tag}: expected ${current_version}, got ${version}" >&2
    exit 1
  fi
  if ! curl -s "${url}/api/v1/accounts" | grep -q "Upgrade checking"; then
    echo "The upgraded API did not return the seeded account for ${tag}" >&2
    exit 1
  fi
  assert_current_schema

  stop_backend
  docker rm -f "$db_container" >/dev/null 2>&1 || true
  db_container=""
  git worktree remove --force "$tag_dir" >/dev/null 2>&1 || true
  tag_dir=""

  echo "PASS ${tag} -> ${current_version} (${tag_count} -> ${current_count} migrations)"
done

echo
echo "Multi-release upgrade matrix passed: ${#tags[@]} release(s) -> ${current_version}."
