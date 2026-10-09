#!/usr/bin/env bash
# Test the UI distribution contracts against fixtures: the conformance check, the
# feed verifier, and the portability claim the whole change exists to satisfy.
#
#   * scripts/check-ui-conformance.sh is proved to pass on repositories that are
#     conformant, vacuously conformant, exempt, or on the frozen legacy list;
#   * it is proved to fail with its named diagnostics on a repository that
#     declares a shared primitive itself, ships a user interface with no
#     declaration, or introduces a new non-scale breakpoint;
#   * ui/scripts/verify-ui-feed.mjs is proved to fail on digest, version, missing
#     and undeclared packages, and to refuse an absolute path, a `..`-escaping
#     path and an environment-variable path;
#   * a consumer tree is copied to an unrelated path and resolved with npm, with
#     no sibling checkout, no environment variable and an unreachable registry.
#
# Nothing here writes to a repository outside a temporary directory: the fixtures
# are copied into one, and the check is read-only by construction.
#
# Exit codes: 0 every case behaved as specified, 1 a case did not, 2 the toolchain
# needed for a case is absent.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
FIXTURES="$ROOT_DIR/ui/tests/conformance-fixtures"
FEED="$ROOT_DIR/ui/feed"

command -v node >/dev/null || { echo "ERROR: node is required" >&2; exit 2; }

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
passed=0
failed=0

report_pass() { printf '  ok  %s\n' "$1"; passed=$((passed + 1)); }
report_fail() { printf '  FAIL %s\n' "$1" >&2; failed=$((failed + 1)); }

# Runs the conformance check over a workspace containing only the named fixtures.
# usage: check_workspace <expectation: pass|fail> <expected diagnostic> <fixture[:repo-name]>...
check_workspace() {
  local expectation="$1" diagnostic="$2"
  shift 2
  local workspace="$work/ws-$passed-$failed-$RANDOM"
  mkdir -p "$workspace"
  local fixture name
  for fixture in "$@"; do
    name="$fixture"
    [[ "$fixture" == *:* ]] && name="${fixture##*:}"
    cp -R "$FIXTURES/${fixture%%:*}" "$workspace/$name"
  done

  local output status
  set +e
  output="$("$SCRIPT_DIR/check-ui-conformance.sh" --workspace "$workspace" --verbose 2>&1)"
  status=$?
  set -e

  if [[ "$expectation" == "pass" ]]; then
    if [[ $status -ne 0 ]]; then
      report_fail "expected the check to pass over: $*
$output"
      return
    fi
    if [[ -n "$diagnostic" && "$output" != *"$diagnostic"* ]]; then
      report_fail "expected the report to mention '$diagnostic' over: $*
$output"
      return
    fi
    report_pass "conformance check passes: $*"
  else
    if [[ $status -eq 0 ]]; then
      report_fail "expected the check to fail with $diagnostic over: $*
$output"
      return
    fi
    if [[ "$output" != *"$diagnostic"* ]]; then
      report_fail "expected $diagnostic over: $*
$output"
      return
    fi
    report_pass "conformance check fails with $diagnostic: $*"
  fi
}

# Builds a consumer tree with the committed feed bytes and a declaration derived
# from ui/feed/feed.json, exactly as a real consumer would commit it.
make_consumer() {
  local target="$1"
  mkdir -p "$target"
  cp -R "$FIXTURES/feed-consumer/." "$target/"
  cp "$FEED"/*.tgz "$target/vendor/platform-ui/"
  node - "$FEED/feed.json" "$target" <<'NODE'
const [manifestPath, target] = process.argv.slice(2);
const { readFileSync, writeFileSync } = require('node:fs');
const { join } = require('node:path');
const feed = JSON.parse(readFileSync(manifestPath, 'utf8'));
writeFileSync(
  join(target, 'ui-conformance.json'),
  `${JSON.stringify(
    {
      schema: 1,
      state: 'consumes-feed',
      kitVersion: feed.kitVersion,
      feed: {
        path: 'vendor/platform-ui',
        packages: feed.packages.map(({ name, version, file, sha256 }) => ({ name, version, file, sha256 })),
      },
    },
    null,
    2,
  )}\n`,
);
NODE
}

# Runs the feed verifier on a consumer tree and compares the diagnostics.
feed_case() {
  local expectation="$1" diagnostic="$2" consumer="$3"
  local output status
  set +e
  output="$(node "$ROOT_DIR/ui/scripts/verify-ui-feed.mjs" "$consumer" 2>&1)"
  status=$?
  set -e
  if [[ "$expectation" == "pass" ]]; then
    if [[ $status -ne 0 ]]; then
      report_fail "expected the feed verifier to pass for $consumer
$output"
      return
    fi
  elif [[ $status -eq 0 || "$output" != *"$diagnostic"* ]]; then
    report_fail "expected $diagnostic for $consumer
$output"
    return
  fi
  report_pass "feed verifier ${expectation}${diagnostic:+ ($diagnostic)}: $(basename "$consumer")"
}

echo "conformance check (design.md V11, V13, V14):"
check_workspace pass "no user-interface signal" no-ui-repo
check_workspace pass "exempt" exempt-repo
check_workspace pass "known frozen divergence" frozen-known:openpanel
check_workspace pass "" exempt-repo no-ui-repo frozen-known:openpanel
check_workspace fail "ui-divergent-primitive" divergent-repo
check_workspace fail "ui-conformance-undeclared" undeclared-ui-repo
check_workspace fail "ui-breakpoint-undeclared" new-breakpoint-repo
check_workspace fail "ui-divergent-primitive" exempt-repo divergent-repo

echo "feed verification (design.md V8, V9):"
set +e
node "$ROOT_DIR/ui/scripts/verify-ui-feed.mjs" --self >/dev/null 2>&1
if [[ $? -eq 0 ]]; then report_pass "feed self-check: committed tarballs match ui/feed/feed.json"; else report_fail "feed self-check"; fi
set -e

consumer="$work/consumer"
make_consumer "$consumer"
feed_case pass "" "$consumer"

digest_consumer="$work/consumer-digest"
cp -R "$consumer" "$digest_consumer"
printf 'x' >> "$digest_consumer/vendor/platform-ui/platform-design-tokens-0.2.0.tgz"
feed_case fail "ui-feed-digest-mismatch" "$digest_consumer"

version_consumer="$work/consumer-version"
cp -R "$consumer" "$version_consumer"
node -e "const f='$version_consumer/ui-conformance.json';const fs=require('node:fs');const j=JSON.parse(fs.readFileSync(f,'utf8'));j.kitVersion='0.3.0';fs.writeFileSync(f,JSON.stringify(j,null,2))"
feed_case fail "ui-feed-version-mismatch" "$version_consumer"

missing_consumer="$work/consumer-missing"
cp -R "$consumer" "$missing_consumer"
mv "$missing_consumer/vendor/platform-ui/platform-react-shell-0.2.0.tgz" "$missing_consumer/"
feed_case fail "ui-feed-incomplete" "$missing_consumer"

undeclared_consumer="$work/consumer-undeclared"
cp -R "$consumer" "$undeclared_consumer"
node -e "const f='$undeclared_consumer/ui-conformance.json';const fs=require('node:fs');const j=JSON.parse(fs.readFileSync(f,'utf8'));j.feed.packages=j.feed.packages.filter(p=>p.name!=='@platform/react-shell');fs.writeFileSync(f,JSON.stringify(j,null,2))"
feed_case fail "ui-feed-undeclared" "$undeclared_consumer"

stray_consumer="$work/consumer-stray"
cp -R "$consumer" "$stray_consumer"
cp "$FEED/platform-design-tokens-0.2.0.tgz" "$stray_consumer/vendor/platform-ui/platform-unknown-ui-9.9.9.tgz"
feed_case fail "ui-feed-undeclared" "$stray_consumer"

absolute_consumer="$work/consumer-absolute"
cp -R "$consumer" "$absolute_consumer"
node -e "const f='$absolute_consumer/package.json';const fs=require('node:fs');const j=JSON.parse(fs.readFileSync(f,'utf8'));j.dependencies['@platform/design-tokens']='file:/opt/platform/platform-design-tokens-0.2.0.tgz';fs.writeFileSync(f,JSON.stringify(j,null,2))"
feed_case fail "ui-feed-path-invalid" "$absolute_consumer"

escaping_consumer="$work/consumer-escaping"
cp -R "$consumer" "$escaping_consumer"
node -e "const f='$escaping_consumer/package.json';const fs=require('node:fs');const j=JSON.parse(fs.readFileSync(f,'utf8'));j.dependencies['@platform/design-tokens']='file:../dotnet-platform-libs/ui/feed/platform-design-tokens-0.2.0.tgz';fs.writeFileSync(f,JSON.stringify(j,null,2))"
feed_case fail "ui-feed-path-invalid" "$escaping_consumer"

env_consumer="$work/consumer-env"
cp -R "$consumer" "$env_consumer"
node -e "const f='$env_consumer/package.json';const fs=require('node:fs');const j=JSON.parse(fs.readFileSync(f,'utf8'));j.dependencies['@platform/design-tokens']='file:\${PLATFORM_UI_FEED}/platform-design-tokens-0.2.0.tgz';fs.writeFileSync(f,JSON.stringify(j,null,2))"
feed_case fail "ui-feed-path-invalid" "$env_consumer"

echo "portability (design.md V10):"
portable="$work/unrelated-location/consumer"
mkdir -p "$(dirname "$portable")"
cp -R "$consumer" "$portable"

if ! command -v npm >/dev/null; then
  echo "  UNVERIFIED  npm is not on PATH; the portability proof needs a package manager" >&2
  failed=$((failed + 1))
else
  set +e
  # --legacy-peer-deps only suppresses npm's attempt to fetch the react peer dependency
  # from the registry; the registry itself is deliberately unreachable, so anything the
  # platform UI layer needs that is not in the committed tarballs fails loudly.
  npm_output="$(cd "$portable" && npm install --offline --no-audit --no-fund --ignore-scripts --legacy-peer-deps --cache "$work/npm-cache" --registry http://127.0.0.1:1/ 2>&1)"
  npm_status=$?
  set -e
  if [[ $npm_status -ne 0 ]]; then
    report_fail "portability: npm could not resolve the committed feed at an unrelated path
$npm_output"
  elif [[ ! -f "$portable/node_modules/@platform/design-tokens/dist/tokens.css" ]]; then
    report_fail "portability: npm resolved nothing
$npm_output"
  elif ! diff -q "$portable/node_modules/@platform/design-tokens/dist/tokens.css" "$ROOT_DIR/ui/packages/design-tokens/dist/tokens.css" >/dev/null; then
    report_fail "portability: the resolved artifact differs from the generated one"
  elif grep -qE '"/|file:/' "$portable/package.json"; then
    report_fail "portability: the consumer tree records an absolute path"
  else
    report_pass "portability: resolved from committed tarballs at an unrelated path, offline, registry unreachable, no sibling checkout"
  fi
fi

echo
if [[ $failed -gt 0 ]]; then
  printf 'ui-distribution test FAILED: %d passed, %d failed.\n' "$passed" "$failed" >&2
  exit 1
fi
printf 'ui-distribution test passed: %d cases.\n' "$passed"
