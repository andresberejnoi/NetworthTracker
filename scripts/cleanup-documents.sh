#!/usr/bin/env bash
#
# Remove only orphaned encrypted document files and temporary fragments.
#
# The dry run lists candidates and changes nothing. --apply requires an explicit
# typed confirmation. This command never deletes documents rows, balances,
# holdings, lots, snapshots, or any computed financial data.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

if [[ -f .env ]]; then
  set -a
  # shellcheck disable=SC1091
  . ./.env
  set +a
fi

POSTGRES_USER="${POSTGRES_USER:-networth}"
POSTGRES_DB="${POSTGRES_DB:-networth}"
DOCUMENT_DIR="${DOCUMENT_DIR:-$root/documents}"
DB_CONTAINER="${DB_CONTAINER:-}"
COMPOSE_CMD="${COMPOSE_CMD:-docker compose}"
DOCKER_CMD="${DOCKER_CMD:-docker}"
apply=0

usage() {
  cat <<EOF
Usage: $0 [--apply]

Without --apply, lists orphaned/temporary document files and changes nothing.
With --apply, asks for the typed confirmation "cleanup" before removing them.

The database metadata rows and all computed financial data are never deleted.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --apply) apply=1 ;;
    -h|--help) usage; exit 0 ;;
    *) echo "Unknown option: $1" >&2; usage >&2; exit 1 ;;
  esac
  shift
done

if [[ ! -d "$DOCUMENT_DIR" ]]; then
  echo "Document directory does not exist: ${DOCUMENT_DIR}"
  exit 0
fi

db_psql() {
  if [[ -n "$DB_CONTAINER" ]]; then
    $DOCKER_CMD exec -i "$DB_CONTAINER" psql "$@"
  else
    $COMPOSE_CMD exec -T db psql "$@"
  fi
}

referenced_file="$(mktemp "${TMPDIR:-/tmp}/networth-doc-refs.XXXXXX")"
cleanup() { rm -f "$referenced_file"; }
trap cleanup EXIT

# Fail closed: never delete files if we cannot read the authoritative metadata.
if ! db_psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" -tAc \
  'SELECT sha256 FROM documents' > "$referenced_file"; then
  echo "Could not read document metadata; refusing to clean up." >&2
  exit 1
fi
sort -u -o "$referenced_file" "$referenced_file"

orphans=()
while IFS= read -r name; do
  [[ -n "$name" ]] || continue
  case "$name" in
    *.tmp)
      orphans+=("$name")
      ;;
    *.enc)
      hash="${name%.enc}"
      if ! grep -Fxq "$hash" "$referenced_file"; then
        orphans+=("$name")
      fi
      ;;
  esac
done < <(find "$DOCUMENT_DIR" -maxdepth 1 -type f -printf '%f\n' | sort)

if [[ ${#orphans[@]} -eq 0 ]]; then
  echo "No orphaned document files found."
  exit 0
fi

echo "Orphaned/temporary document files (${#orphans[@]}):"
for name in "${orphans[@]}"; do
  echo "  ${DOCUMENT_DIR}/${name}"
done

if [[ "$apply" -eq 0 ]]; then
  echo "Dry run only; nothing was removed. Re-run with --apply to remove these files."
  exit 0
fi

read -r -p "Type 'cleanup' to remove these files: " answer
if [[ "$answer" != "cleanup" ]]; then
  echo "Aborted."
  exit 1
fi

# Re-read metadata after confirmation so a file referenced by a concurrent
# upload is not removed.
db_psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" -tAc \
  'SELECT sha256 FROM documents' > "$referenced_file"
sort -u -o "$referenced_file" "$referenced_file"

removed=0
for name in "${orphans[@]}"; do
  case "$name" in
    *.tmp) ;;
    *.enc)
      hash="${name%.enc}"
      if grep -Fxq "$hash" "$referenced_file"; then
        continue
      fi
      ;;
  esac
  rm -f -- "${DOCUMENT_DIR}/${name}"
  removed=$((removed + 1))
done
echo "Removed ${removed} orphaned/temporary file(s). Computed data was untouched."
