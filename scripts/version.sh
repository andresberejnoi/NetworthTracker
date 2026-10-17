#!/usr/bin/env bash
#
# Print the authoritative product version.
#
# The single source of truth is `backend/Cargo.toml` ->
# `[workspace.package].version`, following MAJOR.MINOR.yyyymmdd (see AGENTS.md).
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

version="$(sed -n 's/^version = "\(.*\)"$/\1/p' "$root/backend/Cargo.toml" | head -1)"

if [[ -z "$version" ]]; then
  echo "Could not read the version from backend/Cargo.toml" >&2
  exit 1
fi

if [[ ! "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]{8}$ ]]; then
  echo "Version '$version' is not in MAJOR.MINOR.yyyymmdd form" >&2
  exit 1
fi

echo "$version"
