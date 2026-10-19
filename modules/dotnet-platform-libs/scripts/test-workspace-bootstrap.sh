#!/usr/bin/env bash
# Verifies the workspace's consumer bootstrap import path: a project that
# opts in with PlatformConsumerBootstrap=true must receive a
# ProjectReference to Platform.Core from the local checkout; a project
# that opts out with PlatformConsumerOptOut=true must not.
#
# Set FORGE_WORKSPACE_ROOT to the workspace root whose Directory.Build files
# import this module. No sibling checkout or machine-specific path is inferred.

set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
platform_root="$(cd "${script_dir}/.." && pwd)"
workspace_root="${FORGE_WORKSPACE_ROOT:-}"
if [[ -z "${workspace_root}" ]]; then
  echo "set FORGE_WORKSPACE_ROOT to a workspace that imports this module's Platform.Consumer.props" >&2
  exit 2
fi
workspace_root="$(cd "${workspace_root}" && pwd)"
consumer_dir="$(mktemp -d "${TMPDIR:-/tmp}/platform-consumer-bootstrap.XXXXXX")"
consumer_project="${consumer_dir}/Consumer.csproj"
optout_project="${consumer_dir}/OptOut.csproj"

cleanup() {
  rm -rf -- "${consumer_dir}"
}
trap cleanup EXIT

test -f "${workspace_root}/global.json"
test -f "${workspace_root}/Directory.Build.props"
test -f "${workspace_root}/Directory.Build.targets"
test -f "${platform_root}/build/Platform.Consumer.props"

mkdir -p "${consumer_dir}"
printf '%s\n' \
  '<Project Sdk="Microsoft.NET.Sdk">' \
  '  <PropertyGroup>' \
  '    <TargetFramework>net10.0</TargetFramework>' \
  '    <PlatformConsumerBootstrap>true</PlatformConsumerBootstrap>' \
  '  </PropertyGroup>' \
  '</Project>' > "${consumer_project}"

evaluation="$(dotnet msbuild "${consumer_project}" -getProperty:TargetFramework -getItem:ProjectReference -nologo)"
grep -Fq '"TargetFramework": "net10.0"' <<<"${evaluation}"
grep -Fq 'Platform.Core.csproj' <<<"${evaluation}"
grep -Fq "${platform_root}" <<<"${evaluation}"
dotnet restore "${consumer_project}" --nologo -p:RestoreIgnoreFailedSources=true
dotnet build "${consumer_project}" --nologo --no-restore -p:RestoreIgnoreFailedSources=true

printf '%s\n' \
  '<Project Sdk="Microsoft.NET.Sdk">' \
  '  <PropertyGroup>' \
  '    <TargetFramework>net10.0</TargetFramework>' \
  '    <PlatformConsumerBootstrap>true</PlatformConsumerBootstrap>' \
  '    <PlatformConsumerOptOut>true</PlatformConsumerOptOut>' \
  '  </PropertyGroup>' \
  '</Project>' > "${optout_project}"
optout_evaluation="$(dotnet msbuild "${optout_project}" -getItem:ProjectReference -nologo)"
if grep -Fq 'Platform.Core.csproj' <<<"${optout_evaluation}"; then
  echo "workspace platform consumer bootstrap: opt-out failed" >&2
  exit 1
fi

echo "workspace platform consumer bootstrap: passed"
