#!/usr/bin/env bash
# scripts/check-tests.sh — the Gate's `tests` command.
#
# `bind_gate_commands.py` reads `scripts/check-tests.*` off disk and binds
# the `tests` concern to it, so this file is the repository's own declaration
# of what "tests" means. Openpanel binds its concerns the same way.
#
# It runs the same fast suite CI's `test` job runs. CI draws this line on
# purpose: the three native-scaffold tests below compile and test a freshly
# generated Rust project, and
# `generate::generate_tests::tests::rust_scaffold_builds_and_tests_with_native_toolchain`
# alone measures ~2220s on a developer machine. Binding the full suite here
# made one Gate run cost roughly 46 minutes.
#
# The excluded three are NOT dropped coverage. CI runs exactly them in its
# dedicated `native-scaffold` job; this repository's CI matrix is where that
# evidence lives. Do not add or remove a skip without changing
# `.github/workflows/ci.yml` in the same commit, or the two silently diverge
# and the Gate stops being a local rehearsal of CI.
#
# `--test-threads=1`: this sandbox's tmpfs refuses to exec freshly-written
# fixture scripts under concurrent load (ETXTBSY / "Text file busy"), which
# makes the gate-engine tests flake. Serialising the suite is deterministic
# and matches what CI's `test` job requires of the code; CI runners do not
# exhibit the tmpfs quirk, but single-threading keeps the local Gate rehearsal
# reliable.
set -euo pipefail

cd "$(dirname "$0")/.."

cargo test --workspace --all-targets -- \
  --skip rust_scaffold_builds_and_tests_with_native_toolchain \
  --skip rust_scaffold_builds_with_native_toolchain_without_forge \
  --skip declaration_is_inert_for_native_builds_present_and_absent \
  --test-threads=1 \
  "$@"