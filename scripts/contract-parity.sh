#!/bin/sh
set -eu
ROOT="$(dirname "$0")/.."
SRC="${PLATFORM_CONTRACTS_DIR:-}"
if [ -z "$SRC" ] || [ ! -d "$SRC" ]; then
  SRC="$(realpath "$ROOT/../platform-contracts" 2>/dev/null || true)"
fi
if [ -z "$SRC" ] || [ ! -d "$SRC" ]; then
  echo "contract-parity: cannot resolve platform-contracts source." >&2
  echo "  Set PLATFORM_CONTRACTS_DIR or place a sibling checkout at ../platform-contracts" >&2
  exit 1
fi
echo "contract-parity: source $SRC"
echo "vendored families (from contracts/registry.json):"
python3 - << 'PY'
import json, pathlib
reg = json.loads(pathlib.Path("contracts/registry.json").read_text())
for k in sorted(reg["contracts"].keys()):
    print(f"  {k} {reg['contracts'][k]['current_version']}")
PY
echo ""
echo "fixture counts per family in source:"
for d in "$SRC"/fixtures/*; do
  [ -d "$d" ] || continue
  name=$(basename "$d")
  count=$(ls "$d"/*.json 2>/dev/null | wc -l)
  echo "  $name: $count fixtures"
done
echo ""
echo "contract-parity: OK (next step: run ignored parity test with PLATFORM_CONTRACTS_DIR=$SRC)"
