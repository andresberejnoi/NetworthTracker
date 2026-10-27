#!/usr/bin/env bash
#
# Run gitleaks from PATH or the per-user install location, with a clear error
# when it is missing.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

if command -v gitleaks >/dev/null 2>&1; then
  bin="$(command -v gitleaks)"
elif [[ -x "$HOME/.local/bin/gitleaks" ]]; then
  bin="$HOME/.local/bin/gitleaks"
else
  echo "gitleaks is not installed." >&2
  echo "Install it (see docs/security.md) and run 'make hooks' once." >&2
  exit 1
fi

cd "$root"
exec "$bin" "$@"
