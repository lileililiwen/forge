#!/usr/bin/env bash
set -euo pipefail

dotnet_cmd="${DOTNET_CMD:-dotnet}"
audit_timeout="${AUDIT_TIMEOUT_SECONDS:-120}"
nuget_source="${NUGET_SOURCE:-https://repo.huaweicloud.com/repository/nuget/v3/index.json}"
if output=$(timeout "${audit_timeout}s" "$dotnet_cmd" restore Platform.sln \
  --source "$nuget_source" \
  -p:NuGetAudit=true -p:NuGetAuditMode=all -p:NuGetAuditLevel=high \
  --nologo -m:1 2>&1); then
  printf '%s\n' 'AUDIT_STATUS=VERIFIED'
  printf '%s\n' "$output"
  exit 0
fi

printf '%s\n' 'AUDIT_STATUS=UNVERIFIED'
printf '%s\n' "The vulnerability audit did not obtain evidence within ${audit_timeout}s; release publication must remain blocked." >&2
printf '%s\n' "$output" >&2
if [[ "${ALLOW_UNVERIFIED_AUDIT:-false}" == "true" ]]; then
  exit 0
fi
exit 2
