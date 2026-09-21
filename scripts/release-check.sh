#!/usr/bin/env sh
# Local release check with CI parity (`profile-and-release-readiness`).
#
# Runs the same gates CI runs: Rust formatting/build/tests/clippy, strict
# OpenSpec validation, the full native profile matrix (evidence for every
# supported profile), and the release gate over the runner-qualified
# profiles. Usage:
#
#   scripts/release-check.sh [--gate-profile <id>]...
#
# With no `--gate-profile`, the gate covers every supported profile, so a
# host missing a toolchain (or a template with a failing native command)
# blocks with the exact failed or unavailable check named. CI qualifies a
# subset (see `.github/workflows/ci.yml`); pass the same
# `--gate-profile` flags locally to reproduce the CI verdict.
#
# Exit status: 0 only when every required check and every selected matrix
# row passes. Anything else (including an unavailable `openspec` CLI) is a
# block, never a silent pass.

set -eu

cd "$(dirname "$0")/.."

GATE_PROFILES=""

while [ "$#" -gt 0 ]; do
  case "$1" in
    --gate-profile)
      if [ "$#" -lt 2 ]; then
        echo "release-check: --gate-profile requires a profile id" >&2
        exit 2
      fi
      GATE_PROFILES="$GATE_PROFILES $2"
      shift 2
      ;;
    -h|--help)
      echo "usage: scripts/release-check.sh [--gate-profile <id>]..." >&2
      exit 0
      ;;
    *)
      echo "release-check: unknown argument '$1'" >&2
      exit 2
      ;;
  esac
done

step() {
  echo "==> $*" >&2
  "$@"
}

step cargo fmt --check
step cargo build
step cargo test
step cargo clippy --all-targets -- -D warnings

if command -v openspec >/dev/null 2>&1; then
  step openspec validate --all --strict --no-interactive
else
  echo "release-check blocked: openspec CLI is unavailable; install it to run strict validation" >&2
  exit 1
fi

# Full matrix: evidence for every supported profile. Informational on its
# own (exit 0 with failed/unverified rows); the gate below decides.
step ./target/debug/forge readiness matrix

GATE_ARGS=""
for profile in $GATE_PROFILES; do
  GATE_ARGS="$GATE_ARGS --profile $profile"
done
# Word-splitting $GATE_ARGS intentionally: it holds only --profile pairs.
# shellcheck disable=SC2086
step ./target/debug/forge readiness check $GATE_ARGS
