#!/usr/bin/env bash
#
# Test document cleanup without a database: a fake container command returns the
# authoritative referenced hashes, and the script must only remove unreferenced
# encrypted files and temporary fragments after typed confirmation.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
work="$(mktemp -d "${TMPDIR:-/tmp}/networth-doc-cleanup.XXXXXX")"
trap 'rm -rf "$work"' EXIT

mkdir -p "$work/scripts" "$work/documents" "$work/bin"
cp "$root/scripts/cleanup-documents.sh" "$work/scripts/"
cat > "$work/bin/containers" <<'FAKE'
#!/usr/bin/env bash
printf '%s\n' "${FAKE_REFERENCED:?}"
FAKE
chmod +x "$work/bin/containers" "$work/scripts/cleanup-documents.sh"

referenced="aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
orphan="bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
printf 'referenced\n' > "$work/documents/${referenced}.enc"
printf 'orphan\n' > "$work/documents/${orphan}.enc"
printf 'temporary\n' > "$work/documents/upload.tmp"

cat > "$work/.env" <<EOF
DOCUMENT_DIR=$work/documents
DB_CONTAINER=fake
DOCKER_CMD=$work/bin/containers
FAKE_REFERENCED=$referenced
EOF

echo "== dry run lists only orphaned/temporary files =="
output="$("$work/scripts/cleanup-documents.sh")"
grep -q "upload.tmp" <<<"$output"
grep -q "${orphan}.enc" <<<"$output"
if grep -q "${referenced}.enc" <<<"$output"; then
  echo "referenced file was listed as an orphan" >&2
  exit 1
fi
[[ -f "$work/documents/${referenced}.enc" ]]
[[ -f "$work/documents/${orphan}.enc" ]]

echo "== apply removes only the candidates =="
printf 'cleanup\n' | "$work/scripts/cleanup-documents.sh" --apply >/dev/null
[[ -f "$work/documents/${referenced}.enc" ]]
[[ ! -e "$work/documents/${orphan}.enc" ]]
[[ ! -e "$work/documents/upload.tmp" ]]

echo "Document cleanup tests passed."
