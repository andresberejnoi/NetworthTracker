#!/usr/bin/env bash
#
# Guided generic CSV/JSON migration through the HTTP API.
#
# Preview is the default. --apply first takes a verified encrypted backup, then
# submits the migration with backup_confirmed=true. Use AUTH_TOKEN (or --token)
# when authentication is enabled.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

if [[ -f .env ]]; then
  set -a
  # shellcheck disable=SC1091
  . ./.env
  set +a
fi

file=""
source=""
mapping_file=""
mode="preview"
partial=0
token="${AUTH_TOKEN:-}"
backend_url="${BACKEND_URL:-http://localhost:${BACKEND_PORT:-8080}}"

usage() {
  cat <<EOF
Usage: $0 --file FILE [--source generic_csv|generic_json] [--mapping FILE.json]
          [--preview|--apply] [--partial] [--token TOKEN]

Preview is the default and writes nothing. --apply runs scripts/backup.sh first
(fail closed), then applies atomically unless --partial is given.

Examples:
  $0 --file samples/migration-portfolio.json
  $0 --file samples/migration-portfolio.csv --apply
  $0 --file export.json --mapping mapping.json --apply
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --file) shift; file="${1:-}" ;;
    --source) shift; source="${1:-}" ;;
    --mapping) shift; mapping_file="${1:-}" ;;
    --preview) mode="preview" ;;
    --apply) mode="apply" ;;
    --partial) partial=1 ;;
    --token) shift; token="${1:-}" ;;
    --backend-url) shift; backend_url="${1:-}" ;;
    -h|--help) usage; exit 0 ;;
    *) echo "Unknown option: $1" >&2; usage >&2; exit 1 ;;
  esac
  shift
done

if [[ -z "$file" || ! -f "$file" ]]; then
  echo "A readable --file is required." >&2
  usage >&2
  exit 1
fi
if [[ -z "$source" ]]; then
  case "${file##*.}" in
    json) source="generic_json" ;;
    csv) source="generic_csv" ;;
    *) echo "Cannot infer --source from ${file##*.}; pass it explicitly." >&2; exit 1 ;;
  esac
fi

if [[ "$mode" == "apply" ]]; then
  echo "==> Taking a verified backup before migration"
  "$root/scripts/backup.sh"
fi

request_file="$(mktemp "${TMPDIR:-/tmp}/networth-migration-request.XXXXXX.json")"
response_file="$(mktemp "${TMPDIR:-/tmp}/networth-migration-response.XXXXXX.json")"
cleanup() { rm -f "$request_file" "$response_file"; }
trap cleanup EXIT

python3 - "$file" "$source" "$mapping_file" "$mode" "$partial" "$request_file" <<'PY'
import json
import sys

file_name, source, mapping_file, mode, partial, output = sys.argv[1:]
content = open(file_name, encoding="utf-8").read()
mapping = {}
if mapping_file:
    with open(mapping_file, encoding="utf-8") as handle:
        mapping = json.load(handle)
request = {"source": source, "mapping": mapping, "content": content}
if mode == "apply":
    request["mode"] = "partial" if partial == "1" else "atomic"
    request["backup_confirmed"] = True
with open(output, "w", encoding="utf-8") as handle:
    json.dump(request, handle)
PY

endpoint="/api/v1/migrations/preview"
if [[ "$mode" == "apply" ]]; then
  endpoint="/api/v1/migrations/apply"
fi

headers=(-H 'content-type: application/json')
if [[ -n "$token" ]]; then
  headers+=(-H "authorization: Bearer ${token}")
fi

status="$(curl -sS -o "$response_file" -w '%{http_code}' \
  "${headers[@]}" \
  --data-binary "@${request_file}" \
  "${backend_url}${endpoint}")"

if [[ "$status" -lt 200 || "$status" -ge 300 ]]; then
  echo "Migration request failed with HTTP ${status}." >&2
  cat "$response_file" >&2
  echo >&2
  exit 1
fi

python3 - "$response_file" <<'PY'
import json
import sys

try:
    print(json.dumps(json.load(open(sys.argv[1], encoding="utf-8")), indent=2))
except Exception:
    print(open(sys.argv[1], encoding="utf-8").read())
PY

if [[ "$mode" == "apply" ]]; then
  echo "Migration applied. The pre-migration backup is in ${BACKUP_DIR:-./backups}."
else
  echo "Preview only; nothing was written. Re-run with --apply after reviewing."
fi
