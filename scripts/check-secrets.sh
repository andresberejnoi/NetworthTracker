#!/usr/bin/env bash
#
# Fail if a tracked file contains credential material or a tracked .env file.
# This is intentionally high-signal: it should catch real secrets, not every
# occurrence of the word "password".
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"
fail=0

mapfile -t files < <(git ls-files | grep -vE '\.(png|jpg|jpeg|gif|ico|xlsx|woff2?)$' || true)
if [[ ${#files[@]} -eq 0 ]]; then
  echo "Secret scan passed: no tracked files."
  exit 0
fi

check() {
  local label="$1" pattern="$2"
  local hits
  hits="$(grep -nEI "$pattern" "${files[@]}" 2>/dev/null || true)"
  if [[ -n "$hits" ]]; then
    echo "ERROR: $label"
    echo "$hits"
    fail=1
  fi
}

check "private key material" '-----BEGIN (RSA|OPENSSH|EC|PGP|DSA) PRIVATE KEY-----'
check "AWS access key" 'AKIA[0-9A-Z]{16}'
check "GitHub token" 'gh[pousr]_[A-Za-z0-9]{36,}'
check "GitHub fine-grained token" 'github_pat_[A-Za-z0-9_]{20,}'
check "Slack token" 'xox[baprs]-[A-Za-z0-9-]{10,}'
check "generic API key assignment" '(api[_-]?key|secret[_-]?key)[[:space:]]*[:=][[:space:]]*["'"'"'][A-Za-z0-9_-]{16,}["'"'"']'

# A database URL with an inline password. Placeholders and runtime
# interpolation are allowed.
db_hits="$(grep -nEI 'postgres(ql)?://[^:/@[:space:]]+:[^@/[:space:]]+@' "${files[@]}" 2>/dev/null \
  | grep -vE 'choose-a-password|USER:PASSWORD|YOUR_DB_PASSWORD|<password>|ci-only-not-a-secret|\$\{' || true)"
if [[ -n "$db_hits" ]]; then
  echo "ERROR: database URL with an inline password"
  echo "$db_hits"
  fail=1
fi

# Secret files must never be tracked.
tracked_env="$(git ls-files | grep -E '(^|/)\.env($|\.)' | grep -vE '\.env\.example$' || true)"
if [[ -n "$tracked_env" ]]; then
  echo "ERROR: environment files are tracked"
  echo "$tracked_env"
  fail=1
fi

tracked_keys="$(git ls-files | grep -E '(^|/)(id_rsa|id_ed25519|.*\.pem|.*\.key|.*\.p12)$' || true)"
if [[ -n "$tracked_keys" ]]; then
  echo "ERROR: private key or certificate files are tracked"
  echo "$tracked_keys"
  fail=1
fi

if [[ "$fail" -eq 0 ]]; then
  echo "Secret scan passed: no credentials found in tracked files."
fi
exit "$fail"
