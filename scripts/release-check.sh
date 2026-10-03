#!/usr/bin/env sh
# Local release check with CI parity (`artifact-and-ci-baseline`).
#
# Runs the same gates CI runs: Rust formatting/build/tests/clippy, dependency
# and licence policy (`cargo deny check`, `cargo audit`), strict OpenSpec
# validation, the full native profile matrix (evidence for every supported
# profile), the readiness gate over the runner-qualified profiles, and the
# contract parity comparison when the platform-contracts source is present.
# That comparison blocks on any mismatch. Usage:
#
#   scripts/release-check.sh [--gate-profile <id>]...
#
# "Gate" below always means the readiness gate (`forge readiness check`),
# never the shared gate runtime (`forge gate .`); the latter runs only in
# CI's `gate` job and via `forge gate` locally. With no `--gate-profile`,
# the readiness gate covers every supported profile, so a host missing a
# toolchain (or a template with a failing native command) blocks with the
# exact failed or unavailable check named. CI qualifies a subset (see
# `.github/workflows/ci.yml` `readiness` job); pass the same
# `--gate-profile` flags locally to reproduce the CI verdict.
#
# The parity step runs when `PLATFORM_CONTRACTS_DIR` or the
# `../platform-contracts` sibling checkout resolves, and a mismatch there blocks
# this run. Only an absent source prints a note and continues (CI's
# `contract-parity` job blocks instead — the one documented local/CI
# divergence).
#
# Exit status: 0 only when every required check and every selected matrix
# row passes. Anything else (including an unavailable `openspec`,
# `cargo-deny` or `cargo-audit` CLI) is a block, never a silent pass.

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
step cargo test --workspace --all-targets
step cargo clippy --workspace --all-targets --all-features -- -D warnings

if command -v cargo-deny >/dev/null 2>&1; then
  step cargo deny check
else
  echo "release-check blocked: cargo-deny is unavailable; install it to run dependency and licence policy" >&2
  exit 1
fi

if command -v cargo-audit >/dev/null 2>&1; then
  step cargo audit
else
  echo "release-check blocked: cargo-audit is unavailable; install it to run advisory checks" >&2
  exit 1
fi

if command -v node >/dev/null 2>&1; then
  step node scripts/check-openspec-change-names.mjs
  step node scripts/check-spec-governance.mjs
else
  echo "release-check blocked: node is unavailable; install it to run OpenSpec name and governance checks" >&2
  exit 1
fi

if command -v openspec >/dev/null 2>&1; then
  step openspec validate --all --strict --no-interactive
else
  echo "release-check blocked: openspec CLI is unavailable; install it to run strict validation" >&2
  exit 1
fi

# Contract parity: runs when the platform-contracts source resolves. The step
# compares the vendored contract bytes against the source and exits non-zero on
# any mismatch, so under `set -eu` a mismatch aborts this run — it is a block,
# not a note. Only an absent source prints a note and continues, which is the
# one documented local/CI divergence: CI's `contract-parity` job blocks without
# it instead.
PARITY_SRC="${PLATFORM_CONTRACTS_DIR:-}"
if [ -z "$PARITY_SRC" ] && [ -d "../platform-contracts" ]; then
  PARITY_SRC="../platform-contracts"
fi
if [ -n "$PARITY_SRC" ] && [ -d "$PARITY_SRC" ]; then
  step env PLATFORM_CONTRACTS_DIR="$PARITY_SRC" sh scripts/contract-parity.sh
else
  echo "release-check note: platform-contracts source absent, parity not run (CI contract-parity job blocks without it)" >&2
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
