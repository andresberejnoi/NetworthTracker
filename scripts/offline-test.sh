#!/usr/bin/env bash
#
# Cold offline smoke test: no price, chain, or aggregation provider is
# configured. Exercises account creation, single/bulk balances, CSV import,
# net worth, and snapshot history over HTTP.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
port="${OFFLINE_PORT:-18082}"
db_port="${OFFLINE_DB_PORT:-15435}"
db_container="networth-offline-db-$$"
db_password="$(python3 -c 'import secrets; print(secrets.token_hex(16))')"
url="http://127.0.0.1:${port}"
tmp="$(mktemp -d "${TMPDIR:-/tmp}/networth-offline.XXXXXX")"
backend_pid=""

cleanup() {
  [[ -n "$backend_pid" ]] && kill "$backend_pid" >/dev/null 2>&1 || true
  [[ -n "$backend_pid" ]] && wait "$backend_pid" >/dev/null 2>&1 || true
  docker rm -f "$db_container" >/dev/null 2>&1 || true
  rm -rf "$tmp"
}
trap cleanup EXIT

json() { python3 -c "import sys,json; data=json.load(sys.stdin); print(data$1)"; }

echo "Starting throwaway Postgres ..."
docker run -d --rm --name "$db_container" \
  -e POSTGRES_USER=networth \
  -e POSTGRES_PASSWORD="$db_password" \
  -e POSTGRES_DB=networth \
  -p "127.0.0.1:${db_port}:5432" \
  postgres:17-alpine >/dev/null
for _ in $(seq 1 60); do
  docker exec "$db_container" pg_isready -U networth >/dev/null 2>&1 && break
  sleep 0.5
done

echo "Building backend ..."
(cd "$root/backend" && cargo build -p networth-api)

DATABASE_URL="postgres://networth:${db_password}@127.0.0.1:${db_port}/networth" \
  BACKEND_ADDR="127.0.0.1:${port}" \
  AUTH_DISABLED=true \
  PRICE_PROVIDER=none \
  CHAIN_PROVIDER=none \
  AGGREGATION_PROVIDER=none \
  "$root/backend/target/debug/networth-api" >"$tmp/backend.log" 2>&1 &
backend_pid=$!
for _ in $(seq 1 60); do
  curl -sf "${url}/health" >/dev/null 2>&1 && break
  sleep 0.25
done
if ! curl -sf "${url}/health" >/dev/null; then
  echo "backend did not start" >&2
  cat "$tmp/backend.log" >&2
  exit 1
fi

account="$(curl -sf -X POST "${url}/api/v1/accounts" -H 'content-type: application/json' \
  -d '{"name":"Offline checking","account_type":"checking","currency":"USD"}')"
account_id="$(json '["id"]' <<<"$account")"

curl -sf -X POST "${url}/api/v1/accounts/${account_id}/balances" -H 'content-type: application/json' \
  -d '{"as_of":"2026-01-01","amount":"1000.00"}' >/dev/null

curl -sf -X POST "${url}/api/v1/accounts/${account_id}/balances/batch" -H 'content-type: application/json' \
  -d '{"mode":"atomic","rows":[{"as_of":"2026-02-01","amount":"1100.00"},{"as_of":"2026-03-01","amount":"1200.00"}]}' >/dev/null

cat > "$tmp/import.csv" <<'CSV'
schema_version,record_type,account_name,account_type,institution,currency,ownership_scope,archived,as_of,amount,source
1,account,Offline imported,savings,,USD,household,false,,,
1,balance,Offline imported,,,,,,2026-01-01,500.00,manual
CSV
curl -sf -X POST "${url}/api/v1/import?format=csv" -H 'content-type: text/csv' \
  --data-binary "@$tmp/import.csv" >/dev/null

networth="$(curl -sf "${url}/api/v1/networth?as_of=2026-03-01&owner_age=35")"
python3 - "$networth" <<'PY'
import json, sys
data = json.loads(sys.argv[1])
assert data["total"] == "1700.00", data
assert data["liquid"] == "1700.00", data
PY

curl -sf -X POST "${url}/api/v1/networth/snapshots" -H 'content-type: application/json' \
  -d '{"as_of":"2026-03-01","owner_age":35}' >/dev/null
snapshots="$(curl -sf "${url}/api/v1/networth/snapshots")"
python3 - "$snapshots" <<'PY'
import json, sys
data = json.loads(sys.argv[1])
assert len(data) == 1, data
assert data[0]["total"] == "1700.00", data
PY

echo "Offline smoke test passed (no providers configured)."
