#!/usr/bin/env bash
#
# Restore a database dump produced by scripts/backup.sh.
#
# This REPLACES the current database. It stops the backend, drops and recreates
# the database, restores the dump, and starts the backend (which re-runs any
# pending migrations).
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

file="${1:-}"
if [[ -z "$file" || ! -f "$file" ]]; then
  echo "Usage: $0 <backup.sql.gz>" >&2
  echo "Example: $0 backups/networth-20261028T120000Z.sql.gz" >&2
  exit 1
fi

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
BACKEND_PORT="${BACKEND_PORT:-8080}"

echo "This will REPLACE database '${POSTGRES_DB}' with:"
echo "  ${file}"
read -r -p "Type 'restore' to continue: " answer
if [[ "$answer" != "restore" ]]; then
  echo "Aborted."
  exit 1
fi

echo "Stopping the backend ..."
docker compose stop backend >/dev/null

echo "Recreating the database ..."
docker compose exec -T db psql -U "$POSTGRES_USER" -d postgres \
  -c "DROP DATABASE IF EXISTS \"$POSTGRES_DB\" WITH (FORCE);" >/dev/null
docker compose exec -T db psql -U "$POSTGRES_USER" -d postgres \
  -c "CREATE DATABASE \"$POSTGRES_DB\";" >/dev/null

echo "Restoring ..."
gunzip -c "$file" | docker compose exec -T db psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" >/dev/null

echo "Starting the backend ..."
docker compose start backend >/dev/null

for _ in $(seq 1 60); do
  if curl -sf "http://localhost:${BACKEND_PORT}/health" >/dev/null 2>&1; then
    echo "Restore complete; the backend is healthy."
    exit 0
  fi
  sleep 1
done

echo "Restore finished but the backend is not healthy." >&2
echo "Check 'docker compose logs backend'." >&2
exit 1
