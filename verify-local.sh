#!/bin/sh
# forge local CI rehearsal — mirrors .github/workflows/ci.yml fast suite.
# Run BEFORE pushing so local == remote CI. Then: git push && gh run watch
set -u
ROOT="$(cd "$(dirname "$0")" && pwd)"; cd "$ROOT" || exit 1
command -v rustup >/dev/null 2>&1 && rustup component add rustfmt clippy >/dev/null 2>&1 || true
FAILS=0
p() { printf '\033[32m✔\033[0m %s\n' "$1"; }
f() { printf '\033[31m✘\033[0m %s\n' "$1"; FAILS=$((FAILS+1)); }

echo "=== cargo fmt ==="
cargo fmt --all -- --check 2>log && p "fmt" || { f "fmt"; cat log; }; rm -f log
echo "=== cargo clippy (workspace, all targets, -D warnings) ==="
cargo clippy --workspace --all-targets --all-features -- -D warnings 2>log && p "clippy" || { f "clippy"; cat log; }; rm -f log
echo "=== fast test suite (3 native-scaffold tests skipped, like CI) ==="
bash scripts/check-tests.sh 2>log && p "test" || { f "test"; cat log; }; rm -f log
echo "=== gate (mirrors ci gate job; builds forge first) ==="
# Fail fast with a clear message if the gate runtime is absent, instead of
# letting `forge gate .` emit a cryptic error. This matches the CI `gate` job,
# which blocks loudly when driftwatchdog/driftwatch is not on PATH.
GATE_BIN=""
command -v driftwatchdog >/dev/null 2>&1 && GATE_BIN=driftwatchdog
command -v driftwatch >/dev/null 2>&1 && GATE_BIN=driftwatch
if [ -z "$GATE_BIN" ]; then
  echo "gate blocked: no gate runtime on PATH (tried driftwatchdog, driftwatch)" >&2
  f "gate"
else
  "$GATE_BIN" init >/dev/null 2>&1 || echo "gate: store init skipped (already initialised)" >&2
  cargo build -q 2>log && ./target/debug/forge gate . 2>>log && p "gate" || { f "gate"; cat log; }; rm -f log
fi

if [ "$FAILS" -eq 0 ]; then printf '\n\033[32mLocal checks passed.\033[0m Push and wait: gh run watch\n'; else printf '\n\033[31m%d failed.\033[0m Fix, then push + gh run watch\n' "$FAILS"; fi
exit "$FAILS"
