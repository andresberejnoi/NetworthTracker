#!/usr/bin/env bash
#
# Unit-test the container entrypoint's migration detection and fail-closed
# backup behavior without needing a live database. Fake psql/pg_dump binaries
# provide controlled migration state; real gpg/gzip verify encryption.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
work="$(mktemp -d "${TMPDIR:-/tmp}/networth-entrypoint.XXXXXX")"
fakebin="$work/bin"
backups="$work/backups"
mkdir -p "$fakebin" "$backups"

cleanup() { rm -rf "$work"; }
trap cleanup EXIT

cat > "$fakebin/psql" <<'FAKE'
#!/usr/bin/env bash
case "$*" in
  *"select 1"*) echo 1 ;;
  *"to_regclass"*) echo "${FAKE_HAS_TABLE:-t}" ;;
  *"select version"*) printf '%s\n' ${FAKE_APPLIED:-} ;;
  *) echo 1 ;;
esac
FAKE
cat > "$fakebin/pg_dump" <<'FAKE'
#!/usr/bin/env bash
echo '-- fake database dump'
FAKE
chmod +x "$fakebin/psql" "$fakebin/pg_dump"

migrations=("$root"/backend/migrations/*.sql)
all_versions=""
pending_versions=""
for i in "${!migrations[@]}"; do
  base="$(basename "${migrations[$i]}" .sql)"
  version="${base%%_*}"
  all_versions="$all_versions $version"
  if [[ "$i" -lt $((${#migrations[@]} - 1)) ]]; then
    pending_versions="$pending_versions $version"
  fi
done

run_entrypoint() {
  PATH="$fakebin:$PATH" \
  DATABASE_URL="postgres://fake/fake" \
  PGHOST="fake" \
  MIGRATIONS_DIR="$root/backend/migrations" \
  BACKUP_DIR="$backups" \
  BACKUP_KEEP=2 \
  BACKUP_PASSPHRASE="${BACKUP_PASSPHRASE-test-passphrase}" \
  FAKE_HAS_TABLE="${FAKE_HAS_TABLE:-t}" \
  FAKE_APPLIED="${FAKE_APPLIED:-}" \
  "$root/backend/docker-entrypoint.sh" true
}

echo "== pending migration creates a verified backup =="
rm -f "$backups"/*
FAKE_APPLIED="$pending_versions" run_entrypoint >/dev/null
file="$(ls -1t "$backups"/networth-pre-migration-*.sql.gz.gpg | head -1)"
[[ -s "$file" ]]
gpg --batch --yes --pinentry-mode loopback --decrypt \
  --passphrase "test-passphrase" "$file" 2>/dev/null | gzip -dc | grep -q "fake database dump"

echo "== current schema creates no backup =="
rm -f "$backups"/*
FAKE_APPLIED="$all_versions" run_entrypoint >/dev/null
[[ -z "$(ls -A "$backups")" ]]

echo "== fresh database creates no backup =="
rm -f "$backups"/*
FAKE_HAS_TABLE="f" FAKE_APPLIED="" run_entrypoint >/dev/null
[[ -z "$(ls -A "$backups")" ]]

echo "== pending migration without passphrase fails closed =="
rm -f "$backups"/*
if BACKUP_PASSPHRASE="" FAKE_APPLIED="$pending_versions" run_entrypoint >/dev/null 2>&1; then
  echo "entrypoint started without a backup passphrase" >&2
  exit 1
fi

echo "== unknown migration refuses startup =="
if FAKE_APPLIED="$all_versions 99999999999999" run_entrypoint >/dev/null 2>&1; then
  echo "entrypoint accepted an unknown migration" >&2
  exit 1
fi

echo "Entrypoint backup tests passed."
