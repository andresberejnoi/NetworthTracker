#!/usr/bin/env bash
#
# Test the container backup runner without a database. pg_dump is stubbed; GPG,
# gzip, and tar are real so encryption, verification, and retention are exercised.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
work="$(mktemp -d "${TMPDIR:-/tmp}/networth-container-backup.XXXXXX")"
passphrase="test-only-container-backup"

cleanup() {
  rm -rf "$work"
}
trap cleanup EXIT

mkdir -p "$work/bin" "$work/backups" "$work/documents"
printf 'encrypted document fixture\n' > "$work/documents/fixture.txt"
cat > "$work/bin/pg_dump" <<'FAKE'
#!/usr/bin/env bash
printf 'CREATE TABLE accounts (id int);\n'
FAKE
chmod +x "$work/bin/pg_dump"

export PATH="$work/bin:$PATH"
export BACKUP_DIR="$work/backups"
export DOCUMENT_DIR="$work/documents"
export BACKUP_PASSPHRASE="$passphrase"
export BACKUP_KEEP=2
export PGHOST=test-db
export PGDATABASE=networth

run_backup() {
  BACKUP_TIMESTAMP="$1" "$root/scripts/backup-container.sh" >/dev/null
}

echo "== creates encrypted database and document archives =="
run_backup 20260101T000000Z
db_file="$work/backups/networth-20260101T000000Z.sql.gz.gpg"
docs_file="$work/backups/networth-documents-20260101T000000Z.tar.gz.gpg"
[[ -s "$db_file" ]]
[[ -s "$docs_file" ]]
gpg --batch --yes --pinentry-mode loopback --passphrase "$passphrase" \
  --decrypt "$db_file" 2>/dev/null | gzip -dc | grep -q 'CREATE TABLE accounts'
gpg --batch --yes --pinentry-mode loopback --passphrase "$passphrase" \
  --decrypt "$docs_file" 2>/dev/null | tar -tzf - | grep -q 'documents/fixture.txt'

echo "== missing passphrase fails closed =="
if BACKUP_PASSPHRASE= BACKUP_TIMESTAMP=20260104T000000Z \
  "$root/scripts/backup-container.sh" >/dev/null 2>&1; then
  echo "container backup accepted a missing passphrase" >&2
  exit 1
fi
[[ ! -e "$work/backups/networth-20260104T000000Z.sql.gz.gpg" ]]

echo "== retention keeps two database and document archives =="
run_backup 20260102T000000Z
run_backup 20260103T000000Z
[[ ! -e "$db_file" ]]
[[ ! -e "$docs_file" ]]
[[ -s "$work/backups/networth-20260102T000000Z.sql.gz.gpg" ]]
[[ -s "$work/backups/networth-20260103T000000Z.sql.gz.gpg" ]]
[[ -s "$work/backups/networth-documents-20260102T000000Z.tar.gz.gpg" ]]
[[ -s "$work/backups/networth-documents-20260103T000000Z.tar.gz.gpg" ]]

echo "== verification failure removes the partial archive =="
mkdir -p "$work/badbin"
cat > "$work/badbin/gpg" <<'FAKE'
#!/usr/bin/env bash
set -euo pipefail
if [[ " $* " == *" --decrypt "* ]]; then
  exit 1
fi
out=""
args=("$@")
for ((i = 0; i < ${#args[@]}; i++)); do
  if [[ "${args[$i]}" == "--output" ]]; then
    out="${args[$((i + 1))]}"
  fi
done
[[ -n "$out" ]]
cat > "$out"
FAKE
chmod +x "$work/badbin/gpg"
if PATH="$work/badbin:$PATH" BACKUP_TIMESTAMP=20260105T000000Z \
  "$root/scripts/backup-container.sh" >/dev/null 2>&1; then
  echo "container backup accepted a failed verification" >&2
  exit 1
fi
[[ ! -e "$work/backups/networth-20260105T000000Z.sql.gz.gpg" ]]

echo "Container backup tests passed."
