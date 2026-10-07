#!/usr/bin/env bash
set -euo pipefail

root_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
baseline="$root_dir/eng/public-api-baseline.txt"
update="${1:-}"
projects=(
  "$root_dir/src/Platform.Core/Platform.Core.csproj"
  "$root_dir/src/Platform.Billing.Contracts/Platform.Billing.Contracts.csproj"
  "$root_dir/src/Platform.Identity.Contracts/Platform.Identity.Contracts.csproj"
  "$root_dir/src/Platform.Admin.Contracts/Platform.Admin.Contracts.csproj"
  "$root_dir/src/Platform.Eventing.Contracts/Platform.Eventing.Contracts.csproj"
  "$root_dir/src/Platform.Testing.AspNetCore/Platform.Testing.AspNetCore.csproj"
)

tmp_file="$(mktemp)"
trap 'rm -f "$tmp_file"' EXIT
for project in "${projects[@]}"; do
  project_name="$(basename "$project" .csproj)"
  printf '[%s]\n' "$project_name" >> "$tmp_file"
  rg -n '^[[:space:]]*public (abstract |sealed |static |readonly |partial )*(class|interface|record|struct|enum|delegate)|^[[:space:]]*public .*[({;]' "$(dirname "$project")" -g '*.cs' \
    | sed -E 's#^[^:]+:[0-9]+:[[:space:]]*##' | LC_ALL=C sort -u >> "$tmp_file" || true
done

if [[ "$update" == "--update" ]]; then
  mkdir -p "$(dirname "$baseline")"
  cp "$tmp_file" "$baseline"
  exit 0
fi

test -f "$baseline" || { echo "Missing API baseline: $baseline" >&2; exit 1; }
diff -u "$baseline" "$tmp_file"
