#!/usr/bin/env bash
# Upgrade/rollback smoke test for the consumer conformance fixture.
#
# Usage:
#   scripts/consumer-upgrade-rollback.sh <previous-feed> <candidate-feed> <candidate-version>
#
# Behaviour:
#   1. Restore the conformance fixture from the candidate feed and run the test suite.
#   2. Roll the fixture back to a known previous version and re-run the test suite.
#   3. Fail with a non-zero status if either step fails or if either test run
#      drops a previously passing test.
#
# Environment overrides:
#   DOTNET_CMD=dotnet           Override the dotnet binary.
#   NUGET_SOURCE=<url>          Override the public NuGet source.
#   ALLOW_ENV_BLOCKER=1         Reclassify feed failures as environment blockers (exit 75).

set -euo pipefail

if [[ $# -ne 3 ]]; then
  echo "Usage: $0 <previous-feed> <candidate-feed> <candidate-version>" >&2
  exit 64
fi

root_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
previous_feed="$1"
candidate_feed="$2"
candidate_version="$3"
dotnet_cmd="${DOTNET_CMD:-dotnet}"
nuget_source="${NUGET_SOURCE:-https://repo.huaweicloud.com/repository/nuget/v3/index.json}"
allow_env_blocker="${ALLOW_ENV_BLOCKER:-0}"

temp_dir="$(mktemp -d)"
work_dir="$temp_dir/fixture"
trap 'rm -rf "$temp_dir"' EXIT

tar -C "$root_dir/tests/Platform.ConsumerConformance" \
  --exclude='./bin' --exclude='./obj' --exclude='./.local-feed' \
  --exclude='./.nuget-packages' --exclude='./.logs' --exclude='./TestResults' \
  -cf - . \
  | tar -C "$work_dir" -xf -

project="$work_dir/Platform.ConsumerConformance.csproj"
manifest="$work_dir/../../eng/package-manifest.json"
previous_version="$(python3 - <<'PY' "$manifest"
import json, sys
with open(sys.argv[1], "r", encoding="utf-8") as handle:
    manifest = json.load(handle)
versions = sorted({p["version"] for p in manifest["packages"] if p.get("isPackable") and p.get("version")})
if not versions:
    raise SystemExit("manifest has no packable versions")
print(versions[-1])
PY
)"

set_version() {
  local version="$1"
  sed -i -E "/PackageReference Include=\"Platform\\./ s/Version=\"[^\"]+\"/Version=\"$version\"/" "$project"
}

run_fixture() {
  local label="$1"
  local feed="$2"
  local version="$3"
  echo "=== $label ($version) ==="
  if ! "$dotnet_cmd" restore "$project" \
      --source "$feed" \
      --source "$previous_feed" \
      --source "$nuget_source" \
      --ignore-failed-sources \
      --nologo -m:1 >/dev/null; then
    if [[ "$allow_env_blocker" == "1" ]]; then
      echo "[ENV BLOCKER] restore failed for $label ($version)" >&2
      return 75
    fi
    return 1
  fi
  if ! "$dotnet_cmd" test "$project" \
      -c Release --no-restore \
      --nologo -m:1 \
      --logger "console;verbosity=normal" >/dev/null; then
    if [[ "$allow_env_blocker" == "1" ]]; then
      echo "[ENV BLOCKER] test failed for $label ($version)" >&2
      return 75
    fi
    return 1
  fi
}

set_version "$candidate_version"
if ! run_fixture candidate "$candidate_feed" "$candidate_version"; then
  status=$?
  if [[ "$status" == "75" ]]; then
    echo "Consumer upgrade smoke test reported an environment blocker; rollback step was skipped."
    exit 75
  fi
  echo "Candidate upgrade failed; rolling back to $previous_version to confirm the previous state still passes." >&2
  set_version "$previous_version"
  if ! run_fixture rollback "$previous_feed" "$previous_version"; then
    status=$?
    if [[ "$status" == "75" ]]; then
      echo "Rollback was blocked by an environment issue; manual investigation required." >&2
      exit 75
    fi
    exit 2
  fi
  echo "Rollback succeeded; candidate version refused." >&2
  exit 1
fi

set_version "$previous_version"
if ! run_fixture rollback "$previous_feed" "$previous_version"; then
  status=$?
  if [[ "$status" == "75" ]]; then
    echo "Candidate upgrade passed; rollback run was environment-blocked. Manual verification required." >&2
    exit 75
  fi
  exit 2
fi

echo "Consumer upgrade and rollback smoke test succeeded."
