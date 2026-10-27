#!/usr/bin/env bash
#
# Run the backend's Postgres repository integration tests against a throwaway
# database, so they never depend on the developer's own Postgres.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
port="${TEST_DB_PORT:-15432}"
container="networth-test-db-$$"
db_password="$(python3 -c 'import secrets; print(secrets.token_hex(16))')"

cleanup() {
  docker rm -f "$container" >/dev/null 2>&1 || true
}
trap cleanup EXIT

echo "Starting throwaway Postgres on 127.0.0.1:${port} ..."
docker run -d --rm --name "$container" \
  -e POSTGRES_USER=postgres \
  -e POSTGRES_PASSWORD="$db_password" \
  -e POSTGRES_DB=postgres \
  -p "127.0.0.1:${port}:5432" \
  postgres:17-alpine >/dev/null

for _ in $(seq 1 60); do
  if docker exec "$container" pg_isready -U postgres >/dev/null 2>&1; then
    break
  fi
  sleep 0.5
done

if ! docker exec "$container" pg_isready -U postgres >/dev/null 2>&1; then
  echo "Postgres did not become ready" >&2
  exit 1
fi

export DATABASE_URL="postgres://postgres:${db_password}@127.0.0.1:${port}/postgres"
echo "Running repository integration tests ..."
(cd "$root/backend" && cargo test -p networth-adapters --features db-tests "$@")

echo "Repository integration tests passed."
