#!/usr/bin/env bash
#
# Prove the contract and boundary guards actually fail on violations, then
# restore the tree. Also verify every proposed capability has a spec file.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"
contract="contracts/openapi.yaml"
contract_backup="$(mktemp)"
probe="frontend/src/__boundary_probe.ts"

cleanup() {
  cp "$contract_backup" "$contract"
  rm -f "$contract_backup" "$probe"
}
trap cleanup EXIT

echo "== Contract guard =="
cp "$contract" "$contract_backup"
sed -i '0,/        decimals:/{s/          type: integer/          type: string/}' "$contract"
if (cd frontend && npm run check:api >/dev/null 2>&1); then
  echo "ERROR: contract check passed with a deliberate schema mismatch" >&2
  exit 1
fi
cp "$contract_backup" "$contract"
echo "Contract guard failed as expected."

echo "== Boundary guard =="
printf 'import "../../backend/crates/domain/src/lib";\n' > "$probe"
if ./scripts/check-boundaries.sh >/dev/null 2>&1; then
  echo "ERROR: boundary check passed with a deliberate cross-project import" >&2
  exit 1
fi
rm -f "$probe"
echo "Boundary guard failed as expected."

echo "== Capability specs =="
capabilities="$(sed -n '/### New Capabilities/,/### Modified Capabilities/p' \
  openspec/changes/networth-tracker-v1/proposal.md \
  | grep -oE '^- `[^`]+`' | sed 's/^- `//; s/`$//' | sort -u)"
for capability in $capabilities; do
  spec="openspec/changes/networth-tracker-v1/specs/${capability}/spec.md"
  if [[ ! -f "$spec" ]]; then
    echo "ERROR: missing spec for capability '${capability}'" >&2
    exit 1
  fi
done
echo "All proposed capabilities have spec files."

echo "== OpenSpec =="
openspec validate networth-tracker-v1 --strict
echo "Guards and capability coverage passed."
