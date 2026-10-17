#!/usr/bin/env bash
#
# Boundary check: the backend and frontend are decoupled projects in one
# monorepo. Neither may import the other's code. They integrate only over the
# HTTP API (see contracts/openapi.yaml). Shared, non-runtime assets live at the
# repository root.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
fail=0

check() {
  local label="$1" pattern="$2"
  shift 2
  local hits
  hits="$(grep -rInE "$pattern" "$@" 2>/dev/null || true)"
  if [[ -n "$hits" ]]; then
    echo "ERROR: $label"
    echo "$hits"
    fail=1
  fi
}

# Backend source must not reach into the frontend project.
check "backend references the frontend project" \
  '(\.\./)+frontend/|(^|/)frontend/(src|package\.json|tsconfig|vite\.config)' \
  "$root/backend/crates" "$root/backend/Cargo.toml"

# Frontend source must not reach into the backend project.
check "frontend references the backend project" \
  '(\.\./)+backend/|(^|/)backend/(crates|Cargo\.toml)' \
  "$root/frontend/src" "$root/frontend/package.json" "$root/frontend/vite.config.ts" "$root/frontend/tsconfig.json"

if [[ "$fail" -eq 0 ]]; then
  echo "Boundary check passed: no cross-project imports."
fi
exit "$fail"
