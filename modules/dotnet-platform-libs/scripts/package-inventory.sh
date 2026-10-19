#!/usr/bin/env bash
set -euo pipefail

root_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
find "$root_dir/src" -mindepth 2 -maxdepth 2 -name '*.csproj' -print | sort
printf '%s\n' 'Excluded from package inventory: samples/, tests/, templates/'
