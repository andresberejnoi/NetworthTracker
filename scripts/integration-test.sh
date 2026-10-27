#!/usr/bin/env bash
#
# Cross-boundary integration test: start a throwaway Postgres, run the backend
# against it, then exercise the API through the frontend's own client over HTTP.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
port="${INTEGRATION_PORT:-18080}"
db_port="${INTEGRATION_DB_PORT:-15433}"
db_container="networth-integration-db-$$"
db_password="$(python3 -c 'import secrets; print(secrets.token_hex(16))')"
url="http://127.0.0.1:${port}"

cleanup() {
  kill "${backend_pid:-}" >/dev/null 2>&1 || true
  docker rm -f "$db_container" >/dev/null 2>&1 || true
}
trap cleanup EXIT

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

echo "Building backend ..."
(cd "$root/backend" && cargo build -p networth-api)

echo "Starting backend on ${url} ..."
DATABASE_URL="postgres://networth:${db_password}@127.0.0.1:${db_port}/networth" \
  BACKEND_ADDR="127.0.0.1:${port}" \
  AUTH_DISABLED=true \
  "$root/backend/target/debug/networth-api" &
backend_pid=$!

for _ in $(seq 1 60); do
  if curl -sf "${url}/health" >/dev/null 2>&1; then
    break
  fi
  sleep 0.25
done

if ! curl -sf "${url}/health" >/dev/null 2>&1; then
  echo "Backend did not become healthy at ${url}" >&2
  exit 1
fi

echo "Running frontend integration test ..."
(cd "$root/frontend" \
  && VITE_API_BASE_URL="$url" INTEGRATION_TESTS=1 \
     npx vitest run src/api/client.integration.test.ts)

echo "Cross-boundary integration test passed."
