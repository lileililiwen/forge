#!/bin/sh
# Contract parity: compare Forge's vendored contract mirror against the
# resolved platform-contracts source.
#
# This check answers one question — "are the bytes Forge ships still the bytes
# the contract owner publishes?" — by comparing bytes. A run that compares
# nothing is a failure, not a pass.
#
# What the two sides declare:
#   * the mirror declares what it retains (every file under contracts/ except
#     its own manifest and the derived/foreign vocabulary tree)
#   * the source's own manifest.json declares what it publishes (one entry per
#     published family, each with a schema path and a schema_digest, plus the
#     registry_revision reported below)
#
# Parity holds when both hold:
#   A. every retained file byte-matches its counterpart in the source
#   B. every family the source's record publishes is present in the mirror and
#      its digest equals that record's schema_digest
#
# The record is the authority, so uncommitted work in a source working tree is
# not required of the mirror: it is not published. Conversely Forge's own
# contracts/manifest.json is deliberately excluded from the parity decision — it
# is Forge's offline tamper record, not a statement of parity.
#
# `PLATFORM_CONTRACTS_DIR` locates the source and does nothing else. There is
# no flag or variable that skips, softens or downgrades a comparison, and a
# source resolving to the mirror itself is refused.
#
# Exit status: 0 only when every comparison passed and at least one comparison
# was made. Every other outcome is a non-zero exit and no pass line.

set -eu

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
MIRROR="$ROOT/contracts"

fail() {
  echo "contract-parity: $1" >&2
  exit 1
}

if command -v sha256sum >/dev/null 2>&1; then
  sha256() { sha256sum "$1" | cut -d' ' -f1; }
elif command -v shasum >/dev/null 2>&1; then
  sha256() { shasum -a 256 "$1" | cut -d' ' -f1; }
else
  fail "no sha256 tool found (tried sha256sum, shasum)"
fi

[ -d "$MIRROR" ] || fail "vendored mirror absent at $MIRROR"

# --- resolve the source: the variable when it names a directory, then the
# --- sibling checkout. Unresolvable stays a hard failure, never a skip.
SRC="${PLATFORM_CONTRACTS_DIR:-}"
if [ -z "$SRC" ] || [ ! -d "$SRC" ]; then
  SRC="$(realpath "$ROOT/../platform-contracts" 2>/dev/null || true)"
fi
if [ -z "$SRC" ] || [ ! -d "$SRC" ]; then
  fail "parity-source-unresolvable: set PLATFORM_CONTRACTS_DIR or place a sibling checkout at ../platform-contracts"
fi
SRC="$(cd "$SRC" && pwd)"

if [ "$SRC" = "$MIRROR" ]; then
  fail "parity-self-reference: the resolved source is the vendored mirror itself, which cannot detect drift"
fi

RECORD="$SRC/manifest.json"
[ -f "$RECORD" ] || fail "parity-source-record-invalid: no source record at $RECORD"

# --- the contract's own record: registry_revision plus, per published family,
# --- the schema path and schema_digest it publishes.
RECORD_DATA=""
if command -v python3 >/dev/null 2>&1; then
  RECORD_DATA="$(python3 - "$RECORD" <<'PY' 2>/dev/null || true
import json, sys
doc = json.load(open(sys.argv[1]))
families = doc.get("families")
if not isinstance(families, dict) or not families:
    raise SystemExit(1)
print("revision\t%s" % doc.get("registry_revision", "unknown"))
for name in sorted(families):
    f = families[name]
    if not isinstance(f, dict):
        raise SystemExit(1)
    schema, digest = f.get("schema"), f.get("schema_digest")
    if not isinstance(schema, str) or not isinstance(digest, str):
        raise SystemExit(1)
    print("family\t%s\t%s\t%s" % (name, schema, digest))
PY
)"
fi
[ -n "$RECORD_DATA" ] || fail "parity-source-record-invalid: $RECORD is unreadable, unparseable, or declares no family"

REG_REV="$(printf '%s\n' "$RECORD_DATA" | sed -n '1s/^revision\t//p')"
FAMILY_ROWS="$(printf '%s\n' "$RECORD_DATA" | sed -n 's/^family\t//p')"

echo "contract-parity: source  $SRC"
echo "contract-parity: mirror  $MIRROR"
echo "contract-parity: source registry_revision $REG_REV"

COMPARED=0
MISMATCH=0
ORPHAN=0
MISSING=0
FAMILY_COMPARED=0
FAMILY_MISMATCH=0

# Mirror path -> source path. `contracts/registry.json` is the vendored form of
# the source's `schemas/registry.json` (the rename scripts/sync-contracts.mjs
# performs); every other file keeps its path.
source_rel_for() {
  case "$1" in
    registry.json) printf 'schemas/registry.json' ;;
    *) printf '%s' "$1" ;;
  esac
}

# --- Anchor A: every file the mirror retains must still match the source.
for mrel in $(find "$MIRROR" -type f ! -name manifest.json ! -path "$MIRROR/vocabulary/*" \
  | sed "s|^$MIRROR/||" | sort); do
  srel="$(source_rel_for "$mrel")"
  if [ ! -f "$SRC/$srel" ]; then
    echo "  UNDECLARED  $mrel  (the source publishes no $srel)" >&2
    ORPHAN=$((ORPHAN + 1))
    continue
  fi
  sd="$(sha256 "$SRC/$srel")"
  md="$(sha256 "$MIRROR/$mrel")"
  COMPARED=$((COMPARED + 1))
  if [ "$sd" != "$md" ]; then
    echo "  MISMATCH  $mrel  source $sd != mirror $md" >&2
    MISMATCH=$((MISMATCH + 1))
  fi
done

# --- Anchor B: the source's own record decides, not Forge's manifest.
# Family names, schema paths and digests carry no whitespace, so splitting on
# tab alone is exact. No family schema is the renamed registry, so a family's
# mirror path is its source path.
TAB="$(printf '\t')"
while IFS="$TAB" read -r name mrel digest; do
  [ -n "$name" ] || continue
  if [ ! -f "$MIRROR/$mrel" ]; then
    echo "  MISSING   $mrel  (the source publishes family $name, the mirror does not retain it)" >&2
    MISSING=$((MISSING + 1))
    continue
  fi
  FAMILY_COMPARED=$((FAMILY_COMPARED + 1))
  md="$(sha256 "$MIRROR/$mrel")"
  if [ "$md" != "$digest" ]; then
    echo "  FAMILY-DIGEST  $name  source record $digest != mirror $md" >&2
    FAMILY_MISMATCH=$((FAMILY_MISMATCH + 1))
  fi
done <<EOF
$FAMILY_ROWS
EOF

TOTAL=$((COMPARED + FAMILY_COMPARED))
echo ""
echo "contract-parity: $COMPARED retained file(s) compared byte-for-byte, $FAMILY_COMPARED published family digest(s) checked against the source record"

# The specific diagnosis wins over the generic one, so a mirror that retained
# nothing still names what the source publishes rather than only "nothing
# compared".
if [ "$MISMATCH" -ne 0 ] || [ "$ORPHAN" -ne 0 ] || [ "$MISSING" -ne 0 ] || [ "$FAMILY_MISMATCH" -ne 0 ]; then
  echo "contract-parity: FAIL  $MISMATCH byte mismatch(es), $ORPHAN undeclared vendored file(s), $MISSING missing vendored file(s), $FAMILY_MISMATCH family digest mismatch(es)" >&2
  echo "  re-sync with: node scripts/sync-contracts.mjs" >&2
  exit 1
fi

if [ "$TOTAL" -eq 0 ]; then
  fail "parity-nothing-compared: performed 0 comparisons, so this run verified nothing"
fi

echo "contract-parity: OK  ($COMPARED files and $FAMILY_COMPARED family digests match the resolved source)"
