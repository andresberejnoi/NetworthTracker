#!/usr/bin/env bash
#
# Cross-boundary integration test: build and start the backend, then run the
# frontend's integration test against it over HTTP.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
port="${INTEGRATION_PORT:-18080}"
url="http://127.0.0.1:${port}"

echo "Building backend..."
(cd "$root/backend" && cargo build -p networth-api)

echo "Starting backend on ${url} ..."
BACKEND_ADDR="127.0.0.1:${port}" "$root/backend/target/debug/networth-api" &
backend_pid=$!
trap 'kill "$backend_pid" 2>/dev/null || true' EXIT

for _ in $(seq 1 50); do
  if curl -sf "${url}/health" >/dev/null 2>&1; then
    break
  fi
  sleep 0.2
done

if ! curl -sf "${url}/health" >/dev/null 2>&1; then
  echo "Backend did not become healthy at ${url}" >&2
  exit 1
fi

echo "Running frontend integration test..."
(cd "$root/frontend" \
  && VITE_API_BASE_URL="$url" INTEGRATION_TESTS=1 \
     npx vitest run src/api/client.integration.test.ts)

echo "Cross-boundary integration test passed."
