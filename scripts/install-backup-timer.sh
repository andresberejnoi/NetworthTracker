#!/usr/bin/env bash
#
# Print or install a user-level systemd timer for encrypted backups.
#
# The timer runs scripts/scheduled-backup.sh, which logs to BACKUP_LOG_DIR and
# delegates encryption, verification, and retention to scripts/backup.sh.
# Run without --install to print the units and a cron example.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
unit_dir="${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user"
schedule="${BACKUP_TIMER_SCHEDULE:-daily}"
install=0
enable=1
systemctl="${SYSTEMCTL:-systemctl}"

usage() {
  cat <<EOF
Usage: $0 [--install] [--no-enable] [--schedule CALENDAR] [--unit-dir DIR]

Prints a user-level systemd service/timer and a cron example by default.
With --install, writes the units under:
  ${unit_dir}

Options:
  --install             Write the units and run systemctl --user daemon-reload
  --no-enable           With --install, do not enable/start the timer
  --schedule CALENDAR   systemd OnCalendar value (default: ${schedule})
  --unit-dir DIR        Override the systemd user unit directory
  -h, --help            Show this help

Examples:
  $0
  $0 --install
  $0 --install --schedule "*-*-* 03:15:00"
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --install) install=1 ;;
    --no-enable) enable=0 ;;
    --schedule)
      shift
      [[ $# -gt 0 ]] || { echo "--schedule needs a value" >&2; exit 1; }
      schedule="$1"
      ;;
    --unit-dir)
      shift
      [[ $# -gt 0 ]] || { echo "--unit-dir needs a value" >&2; exit 1; }
      unit_dir="$1"
      ;;
    -h|--help) usage; exit 0 ;;
    *) echo "Unknown option: $1" >&2; usage >&2; exit 1 ;;
  esac
  shift
done

service_file="$unit_dir/networth-backup.service"
timer_file="$unit_dir/networth-backup.timer"

render_service() {
  cat <<EOF
[Unit]
Description=NetworthTracker encrypted database backup
After=network-online.target
Wants=network-online.target

[Service]
Type=oneshot
WorkingDirectory=${root}
ExecStart=${root}/scripts/scheduled-backup.sh
Nice=10
IOSchedulingClass=best-effort
IOSchedulingPriority=7
EOF
}

render_timer() {
  cat <<EOF
[Unit]
Description=NetworthTracker encrypted database backup timer

[Timer]
OnCalendar=${schedule}
Persistent=true
RandomizedDelaySec=30m
AccuracySec=1m
Unit=networth-backup.service

[Install]
WantedBy=timers.target
EOF
}

if [[ "$install" -eq 0 ]]; then
  echo "# Service: ${service_file}"
  render_service
  echo
  echo "# Timer: ${timer_file}"
  render_timer
  echo
  echo "# Install the timer with:"
  echo "#   $0 --install"
  echo
  echo "# Or run from cron (the wrapper writes its own timestamped log):"
  echo "#   15 3 * * * ${root}/scripts/scheduled-backup.sh"
  exit 0
fi

if ! command -v "$systemctl" >/dev/null 2>&1; then
  echo "systemctl is not available; use the printed cron example instead." >&2
  echo "Run '$0' without --install to print it." >&2
  exit 1
fi

mkdir -p "$unit_dir"
render_service > "$service_file"
render_timer > "$timer_file"
chmod 644 "$service_file" "$timer_file"

echo "Wrote ${service_file}"
echo "Wrote ${timer_file}"

if ! "$systemctl" --user daemon-reload; then
  echo "systemctl --user daemon-reload failed. The units were written; reload them manually." >&2
  exit 1
fi

if [[ "$enable" -eq 1 ]]; then
  if ! "$systemctl" --user enable --now networth-backup.timer; then
    echo "Could not enable the timer. Enable it manually with:" >&2
    echo "  systemctl --user enable --now networth-backup.timer" >&2
    exit 1
  fi
  echo "Enabled networth-backup.timer."
  echo "Check it with: systemctl --user list-timers networth-backup.timer"
else
  echo "Timer not enabled (--no-enable). Enable it with:"
  echo "  systemctl --user enable --now networth-backup.timer"
fi
