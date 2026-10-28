#!/usr/bin/env bash
#
# Permanently delete the database volume. This is the ONLY script that wipes
# data, and it requires an explicit confirmation.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

echo "This permanently deletes the database volume and ALL household data."
echo "Backups in ./backups are not removed."
read -r -p "Type 'destroy' to continue: " answer
if [[ "$answer" != "destroy" ]]; then
  echo "Aborted."
  exit 1
fi

docker compose down -v
echo "Stack and data removed."
