#!/usr/bin/env bash
#
# Safe in-place upgrade: take a backup, rebuild, restart, and verify health.
# The database volume is never removed.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

if [[ ! -f .env ]]; then
  echo "No .env found. Copy .env.example to .env first." >&2
  exit 1
fi

set -a
# shellcheck disable=SC1091
. ./.env
set +a

BACKEND_PORT="${BACKEND_PORT:-8080}"
old_version="$("$root/scripts/version.sh" 2>/dev/null || echo unknown)"

echo "==> Pre-upgrade backup"
"$root/scripts/backup.sh"

if [[ "${UPGRADE_GIT_PULL:-0}" == "1" ]]; then
  echo "==> git pull --ff-only"
  git pull --ff-only
fi

echo "==> Building images (${old_version})"
docker compose build

echo "==> Starting the stack"
docker compose up -d

echo "==> Waiting for the backend"
for _ in $(seq 1 90); do
  if curl -sf "http://localhost:${BACKEND_PORT}/health" >/dev/null 2>&1; then
    break
  fi
  sleep 1
done

if ! curl -sf "http://localhost:${BACKEND_PORT}/health" >/dev/null 2>&1; then
  echo "Upgrade failed: the backend did not become healthy." >&2
  echo "Your pre-upgrade backup is in ./backups." >&2
  echo "Restore it with: ./scripts/restore.sh backups/<file>.sql.gz.gpg" >&2
  exit 1
fi

new_version="$(curl -s "http://localhost:${BACKEND_PORT}/api/v1/ping" \
  | python3 -c 'import sys,json; print(json.load(sys.stdin).get("version","unknown"))' \
  2>/dev/null || echo unknown)"

echo "Upgrade complete: ${old_version} -> ${new_version}."
echo "Migrations ran automatically at startup. Backup kept in ./backups."
