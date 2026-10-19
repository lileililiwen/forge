#!/usr/bin/env bash
# Generate a machine-readable package capability/dependency manifest for the
# platform libraries. Reads every *.csproj under src/ and emits a JSON file
# that downstream consumers (and the conformance suite) can diff against.
#
# Usage:
#   scripts/generate-package-manifest.sh                # writes eng/package-manifest.json
#   scripts/generate-package-manifest.sh --check        # verifies src matches the committed manifest
#   scripts/generate-package-manifest.sh --out <path>   # writes to a different path
#
# Exit codes:
#   0  manifest written or matches the source
#   1  invocation error
#   2  --check failed (source drifted from the manifest)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
DEFAULT_OUTPUT="$ROOT_DIR/eng/package-manifest.json"

mode="write"
output="$DEFAULT_OUTPUT"
while [[ $# -gt 0 ]]; do
  case "$1" in
    --check) mode="check"; shift ;;
    --out) output="$2"; shift 2 ;;
    -h|--help)
      sed -n '2,16p' "$0"
      exit 0
      ;;
    *) printf 'Unknown argument: %s\n' "$1" >&2; exit 1 ;;
  esac
done

tmp="$(mktemp)"
trap 'rm -f "$tmp"' EXIT

python3 - "$ROOT_DIR" "$tmp" <<'PY'
import json, os, re, sys, xml.etree.ElementTree as ET

root, tmp = sys.argv[1], sys.argv[2]

def local(tag: str) -> str:
    if "}" in tag:
        return tag.split("}", 1)[1]
    return tag

def find_text(node, candidates):
    for elem in node.iter():
        if local(elem.tag) in candidates and elem.text:
            value = elem.text.strip()
            if value:
                return value
    return ""

def find_all(node, target):
    return [child for child in node.iter() if local(child.tag) == target]

def find_props(path):
    if not os.path.isfile(path):
        return {}
    tree = ET.parse(path)
    values = {}
    for child in tree.getroot().iter():
        if child.text and local(child.tag) in {"VersionPrefix", "Version"}:
            values[local(child.tag)] = child.text.strip()
    return values

def find_inherited_version(start_dir):
    current = start_dir
    while True:
        candidate = os.path.join(current, "Directory.Build.props")
        if os.path.isfile(candidate):
            props = find_props(candidate)
            for key in ("Version", "VersionPrefix"):
                value = props.get(key, "")
                if value and not value.startswith("$("):
                    return value
        if current == root:
            break
        parent = os.path.dirname(current)
        if parent == current:
            break
        current = parent
    return ""

packages = []
for dirpath, _dirnames, filenames in os.walk(os.path.join(root, "src")):
    for filename in filenames:
        if not filename.endswith(".csproj"):
            continue
        full = os.path.join(dirpath, filename)
        if "/bin/" in full or "/obj/" in full:
            continue
        rel = os.path.relpath(full, root)
        tree = ET.parse(full)
        root_node = tree.getroot()
        target_framework = find_text(root_node, {"TargetFramework"})
        target_frameworks_raw = find_text(root_node, {"TargetFrameworks"})
        if target_frameworks_raw:
            target_frameworks = [t.strip() for t in re.split(r"[;|]", target_frameworks_raw) if t.strip()]
        elif target_framework:
            target_frameworks = [target_framework]
        else:
            target_frameworks = []
        description = find_text(root_node, {"Description"})
        version = find_text(root_node, {"Version", "VersionPrefix"})
        if not version or version.startswith("$("):
            version = find_inherited_version(dirpath)
        package_id = find_text(root_node, {"PackageId"})
        if not package_id:
            package_id = os.path.splitext(filename)[0]
        project_refs = []
        for ref in find_all(root_node, "ProjectReference"):
            include = (ref.attrib.get("Include") or "").strip()
            if include:
                project_refs.append(include.replace("\\", "/"))
        package_refs = []
        for ref in find_all(root_node, "PackageReference"):
            include = (ref.attrib.get("Include") or "").strip()
            version_value = (ref.attrib.get("Version") or "").strip()
            if include:
                package_refs.append({"name": include, "version": version_value})
        framework_refs = []
        for ref in find_all(root_node, "FrameworkReference"):
            include = (ref.attrib.get("Include") or "").strip()
            if include:
                framework_refs.append(include)
        is_packable_text = find_text(root_node, {"IsPackable"})
        is_packable = is_packable_text != "false"
        packages.append({
            "path": rel.replace("\\", "/"),
            "packageId": package_id,
            "version": version or "0.0.0",
            "targetFrameworks": target_frameworks,
            "description": description,
            "isPackable": is_packable,
            "projectReferences": project_refs,
            "packageReferences": package_refs,
            "frameworkReferences": framework_refs,
        })

manifest = {
    "schemaVersion": 1,
    "name": "platform-package-manifest",
    "packages": sorted(packages, key=lambda p: p["packageId"]),
}
with open(tmp, "w", encoding="utf-8") as handle:
    json.dump(manifest, handle, indent=2, sort_keys=True)
    handle.write("\n")
PY

if [[ "$mode" == "check" ]]; then
  if ! command -v diff >/dev/null 2>&1; then
    printf 'diff is required for --check\n' >&2
    exit 1
  fi
  if [[ ! -f "$output" ]]; then
    printf 'Manifest file not found: %s\n' "$output" >&2
    printf 'Run scripts/generate-package-manifest.sh to generate it.\n' >&2
    exit 2
  fi
  if diff -u "$output" "$tmp" >/dev/null; then
    printf 'Manifest matches source.\n'
    exit 0
  fi
  printf 'Manifest drift detected. Run scripts/generate-package-manifest.sh and commit the result.\n' >&2
  diff -u "$output" "$tmp" >&2 || true
  exit 2
fi

mkdir -p "$(dirname "$output")"
mv "$tmp" "$output"
printf 'Wrote %s with %d packages.\n' "$output" "$(grep -c '\"packageId\"' "$output" || true)"
