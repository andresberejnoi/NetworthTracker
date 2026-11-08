#!/usr/bin/env bash
#
# Test the scheduled-backup wrapper and the systemd timer generator without
# touching the real database. A fake backup.sh records invocations, so this
# verifies locking, logging, fail-closed configuration, and unit rendering.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
work="$(mktemp -d "${TMPDIR:-/tmp}/networth-backup-schedule.XXXXXX")"
first_pid=""

cleanup() {
  if [[ -n "$first_pid" ]]; then
    kill "$first_pid" >/dev/null 2>&1 || true
    wait "$first_pid" >/dev/null 2>&1 || true
  fi
  rm -rf "$work"
}
trap cleanup EXIT

mkdir -p "$work/scripts" "$work/backups" "$work/logs"
cp "$root/scripts/scheduled-backup.sh" "$work/scripts/"
cat > "$work/scripts/backup.sh" <<'FAKE'
#!/usr/bin/env bash
set -euo pipefail
printf 'fake backup\n' >> "${FAKE_BACKUP_CALLS:?}"
sleep "${FAKE_BACKUP_SLEEP:-0}"
FAKE
chmod +x "$work/scripts/scheduled-backup.sh" "$work/scripts/backup.sh"

write_env() {
  cat > "$work/.env" <<EOF
BACKUP_DIR=$work/backups
BACKUP_LOG_DIR=$work/logs
BACKUP_PASSPHRASE=test-only-not-a-secret
BACKUP_KEEP=2
EOF
  chmod 600 "$work/.env"
}

run_scheduled() {
  (cd "$work" && "$work/scripts/scheduled-backup.sh")
}

export FAKE_BACKUP_CALLS="$work/calls"
write_env

echo "== scheduled run writes a log and calls backup.sh =="
run_scheduled
[[ "$(wc -l < "$work/calls")" == "1" ]]
grep -q "scheduled backup complete" "$work/logs/backup.log"

echo "== missing passphrase fails closed before backup.sh =="
sed -i 's/^BACKUP_PASSPHRASE=.*/BACKUP_PASSPHRASE=change-me/' "$work/.env"
: > "$work/calls"
if run_scheduled; then
  echo "scheduled backup accepted a placeholder passphrase" >&2
  exit 1
fi
[[ ! -s "$work/calls" ]]
grep -q "BACKUP_PASSPHRASE" "$work/logs/backup.log"

echo "== overlapping runs are skipped =="
write_env
: > "$work/calls"
FAKE_BACKUP_SLEEP=2 run_scheduled &
first_pid=$!
for _ in $(seq 1 50); do
  [[ -s "$work/calls" ]] && break
  sleep 0.1
done
run_scheduled
wait "$first_pid"
first_pid=""
[[ "$(wc -l < "$work/calls")" == "1" ]]
grep -q "already running" "$work/logs/backup.log"

echo "== timer generator renders the current checkout =="
output="$("$root/scripts/install-backup-timer.sh" --schedule "*-*-* 03:15:00")"
grep -q "ExecStart=${root}/scripts/scheduled-backup.sh" <<<"$output"
grep -q "OnCalendar=\*-\*-\* 03:15:00" <<<"$output"
grep -q "scheduled-backup.sh" <<<"$output"

echo "== timer install writes units without enabling =="
SYSTEMCTL=true "$root/scripts/install-backup-timer.sh" \
  --install --no-enable --unit-dir "$work/units" >/dev/null
[[ -s "$work/units/networth-backup.service" ]]
[[ -s "$work/units/networth-backup.timer" ]]
grep -q "ExecStart=${root}/scripts/scheduled-backup.sh" \
  "$work/units/networth-backup.service"

echo "Scheduled backup tests passed."
