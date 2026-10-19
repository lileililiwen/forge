#!/usr/bin/env bash
# Consumer-baseline conformance harness.
#
# Proves the recipe in docs/consumer-baseline.md works: a consumer can
# resolve the platform crates, build and test under the pinned
# toolchain, and gets loud, named failures for lint, MSRV, and
# dependency-policy regressions.
#
# Usage: ./scripts/conformance.sh  (from the repository root)
#
# Exit code is 0 only if every proof below holds. Any unexpected
# result fails the harness with the fixture and the cause named.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

pass() { printf 'PASS: %s\n' "$1"; }
fail() { printf 'FAIL: %s\n' "$1" >&2; exit 1; }

# ---------------------------------------------------------------- 1. Pin.
# The workspace rustc MUST be the rust-toolchain.toml channel, and the
# channel MUST satisfy the declared workspace MSRV.

PINNED="$(sed -n 's/^channel *= *"\([^"]*\)".*/\1/p' rust-toolchain.toml | head -1)"
[ -n "$PINNED" ] || fail "toolchain pin: no channel in rust-toolchain.toml"
ACTUAL="$(rustc --version | awk '{print $2}')"
[ "$ACTUAL" = "$PINNED" ] || fail "toolchain pin: rustc $ACTUAL != pinned $PINNED"
pass "toolchain pin: rustc $ACTUAL matches rust-toolchain.toml"

MSRV="$(sed -n 's/^rust-version *= *"\([^"]*\)".*/\1/p' Cargo.toml | head -1)"
[ "$MSRV" = "1.75" ] || fail "MSRV: workspace rust-version is $MSRV, expected 1.75"
python3 - "$PINNED" "$MSRV" <<'EOF'
import sys
def parts(v):
    return [int(p) for p in v.split('.')]
pinned, msrv = sys.argv[1], sys.argv[2]
assert parts(pinned) >= parts(msrv), f"pinned {pinned} below MSRV {msrv}"
EOF
pass "MSRV: pinned $PINNED satisfies declared $MSRV"

# ------------------------------------------- 2. In-workspace consumer.
# tests/consumer-fixture MUST build and test green, and its manifest
# MUST use only workspace-relative path deps (no git, no registry, no
# sibling-relative escape).

cargo test -p consumer-fixture --offline -q \
    || fail "in-workspace consumer: cargo test -p consumer-fixture failed"
pass "in-workspace consumer: consumer-fixture builds and tests green"

grep -Eq '^\s*platform-[a-z-]+ = \{ path = "\.\./\.\./crates/' tests/consumer-fixture/Cargo.toml \
    || fail "in-workspace consumer: fixture manifest lacks workspace-relative path deps"
if grep -Eq '^\s*(platform-[a-z-]+|.*) = \{ [^}]*git *=' tests/consumer-fixture/Cargo.toml; then
    fail "in-workspace consumer: fixture manifest contains a git dependency"
fi
if grep -Eq 'path *= *"\.\./rust-platform-libs' tests/consumer-fixture/Cargo.toml; then
    fail "in-workspace consumer: fixture manifest escapes the workspace via sibling path"
fi
pass "in-workspace consumer: manifest uses workspace-relative paths only"

# ------------------------------------------ 3. Out-of-workspace consumer.
# A scratch crate OUTSIDE the workspace copies the four baseline files
# verbatim, resolves platform-core, and builds, tests, formats, and
# lints clean under the copied pin.
#
# OFFLINE STAND-IN, stated explicitly: the documented contract is a
# versioned git dependency, but this environment has no registry or
# git remote, so the probe uses an absolute path dependency into the
# workspace. Path-vs-git is a resolution detail; what is proven here
# is that an external crate resolves, builds, tests, and inherits the
# same format/lint outcomes from the copied baseline.

SCRATCH="${SCRATCH_DIR:-/tmp/opencode/rust-platform-conformance}"
rm -rf "$SCRATCH"
mkdir -p "$SCRATCH/outside"
cp rustfmt.toml clippy.toml deny.toml rust-toolchain.toml "$SCRATCH/"
(
    cd "$SCRATCH/outside"
    cargo new --lib consumer-outside -q --offline
    cd consumer-outside
    # cargo new already emits an empty [dependencies] table; append the
    # key to it rather than opening a second table.
    printf 'platform-core = { path = "%s/crates/platform-core" }\n' "$ROOT" >> Cargo.toml
    cat > src/lib.rs <<'EOF'
pub fn epoch_is_stable() -> bool {
    use platform_core::{Clock, FixedClock};
    let clock = FixedClock::at_epoch();
    let first = clock.now();
    let second = clock.now();
    first == second
}

#[cfg(test)]
mod tests {
    use platform_core::{Clock, CorrelationId, FixedClock};

    #[test]
    fn resolves_platform_crates() {
        let clock = FixedClock::at_epoch();
        assert_eq!(clock.now(), clock.now());
        let id = CorrelationId::new();
        assert_ne!(id.as_uuid().as_simple().to_string(), "");
        assert!(super::epoch_is_stable());
    }
}
EOF
    cargo test --offline -q \
        || { printf 'FAIL: out-of-workspace consumer: build/test failed\n' >&2; exit 1; }
    cargo fmt -- --check \
        || { printf 'FAIL: out-of-workspace consumer: copied rustfmt.toml does not hold\n' >&2; exit 1; }
    cargo clippy --all-targets --offline -q -- -D warnings \
        || { printf 'FAIL: out-of-workspace consumer: copied clippy baseline does not hold\n' >&2; exit 1; }
)
pass "out-of-workspace consumer: resolves, tests, formats, and lints clean under copied baseline"

# ------------------------------------------ 4. Lint regression is loud.
# A scratch crate with an unused variable MUST fail clippy -D warnings
# naming the lint.

mkdir -p "$SCRATCH/lintbreak"
(
    cd "$SCRATCH/lintbreak"
    cargo new --lib lintbreak -q --offline
    printf 'pub fn f() -> i32 { let unused = 1; 42 }\n' > lintbreak/src/lib.rs
    if cargo clippy --manifest-path lintbreak/Cargo.toml --offline -q -- -D warnings 2>clippy.err; then
        printf 'FAIL: lint-breaking fixture: clippy passed, expected failure\n' >&2
        exit 1
    fi
    grep -q "unused-variables" clippy.err \
        || { printf 'FAIL: lint-breaking fixture: failure did not name unused-variables\n' >&2; exit 1; }
)
pass "lint regression: clippy -D warnings fails naming unused-variables"

# ------------------------------------------ 5. MSRV enforcement is loud.
# A scratch crate declaring rust-version above the pinned toolchain
# MUST fail to build naming the required version.

mkdir -p "$SCRATCH/msrvbreak"
(
    cd "$SCRATCH/msrvbreak"
    cp "$ROOT/rust-toolchain.toml" .
    cargo new --lib msrvbreak -q --offline
    printf '[package]\nname = "msrvbreak"\nversion = "0.1.0"\nedition = "2021"\nrust-version = "1.99"\n' \
        > msrvbreak/Cargo.toml
    if cargo build --manifest-path msrvbreak/Cargo.toml --offline 2>msrv.err; then
        printf 'FAIL: msrv-breaking fixture: build passed, expected failure\n' >&2
        exit 1
    fi
    grep -q "requires rustc 1.99" msrv.err \
        || { printf 'FAIL: msrv-breaking fixture: failure did not name required version\n' >&2; exit 1; }
)
pass "MSRV enforcement: build fails naming required rustc 1.99"

# ------------------------------------------ 6. Denied dependency is loud.
# cargo deny against a probe config that bans serde MUST fail naming
# the crate. This proves the deny gate fails closed with a named cause.

printf '[bans]\ndeny = ["serde"]\n' > "$SCRATCH/deny-probe-serde.toml"
if cargo deny --config "$SCRATCH/deny-probe-serde.toml" check bans >"$SCRATCH/deny.out" 2>&1; then
    fail "deny-breaking fixture: cargo deny passed, expected failure"
fi
grep -q "serde" "$SCRATCH/deny.out" \
    || fail "deny-breaking fixture: failure did not name serde"
pass "denied dependency: cargo deny fails naming serde"

printf '\nConformance: all proofs hold.\n'
