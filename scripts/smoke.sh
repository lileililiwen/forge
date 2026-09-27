#!/usr/bin/env sh
# Smoke-test an installed or built Forge binary (`artifact-and-ci-baseline`).
#
# Usage: scripts/smoke.sh [--bin <path>]
#
# Builds a scratch project with the binary under test, then runs the core
# read surfaces against it: --version, --help, list, doctor, readiness
# artifact, and a checker document parseability check. Exit 0 only when
# every surface returns a parseable result; anything else names the failed
# surface. Persists and journals nothing outside the scratch directory.

set -eu

BIN="./target/debug/forge"
while [ "$#" -gt 0 ]; do
  case "$1" in
    --bin)
      [ "$#" -ge 2 ] || { echo "smoke: --bin requires a path" >&2; exit 2; }
      BIN="$2"; shift 2 ;;
    -h|--help)
      echo "usage: scripts/smoke.sh [--bin <path>]" >&2
      exit 0 ;;
    *)
      echo "smoke: unknown argument '$1'" >&2
      exit 2 ;;
  esac
done

if [ ! -x "$BIN" ]; then
  echo "smoke blocked: binary $BIN is not executable" >&2
  exit 1
fi

step() {
  echo "==> $*" >&2
  "$@"
}

SCRATCH="$(mktemp -d)"
trap 'rm -rf "$SCRATCH"' EXIT INT TERM
export FORGE_REGISTRY="$SCRATCH/registry.db"

step "$BIN" --version
step "$BIN" --help > /dev/null
step "$BIN" list > /dev/null
step "$BIN" new "$SCRATCH/smoke-probe" --profile rust-web > /dev/null
step "$BIN" doctor "$SCRATCH/smoke-probe" > /dev/null
step "$BIN" readiness artifact > /dev/null
step "$BIN" check "$SCRATCH/smoke-probe" --format json | python3 -c 'import json,sys; d=json.load(sys.stdin); assert "alerts" in d, sorted(d.keys())'
echo "smoke: OK ($BIN)"
