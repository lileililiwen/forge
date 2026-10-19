#!/usr/bin/env bash
set -euo pipefail

root_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
dotnet_cmd="${DOTNET_CMD:-dotnet}"
configuration="${CONFIGURATION:-Release}"
nuget_source="${NUGET_SOURCE:-https://repo.huaweicloud.com/repository/nuget/v3/index.json}"

cd "$root_dir"
# D18 (owner ruling, overrides this change's own earlier recommendation): the UI
# token verifier runs in the repository gate, and so does the self-test that proves
# each of its named diagnostics actually fires. A verifier that no gate runs does
# not gate - that is this workspace's diagnosed failure mode, where
# forge/scripts/contract-parity.sh performs zero comparisons and still prints
# `contract-parity: OK`. Without these two lines a token regression (a dropped
# token, an orphan literal, an unresolvable alias, a missing canonical primitive)
# would not block the gate at all.
node ui/scripts/verify-ui.mjs
node ui/scripts/selftest-verify-ui.mjs
python3 -m unittest discover -s tests/scripts -v
"$dotnet_cmd" restore Platform.sln --source "$nuget_source" --ignore-failed-sources -p:NuGetAudit=false --nologo -m:1
"$dotnet_cmd" build Platform.sln -c "$configuration" --no-restore --nologo -m:1
"$dotnet_cmd" test Platform.sln -c "$configuration" --no-build --nologo -m:1
openspec validate --changes --strict --no-interactive
openspec validate --specs --strict --no-interactive
git diff --check
