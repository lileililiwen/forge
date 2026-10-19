#!/usr/bin/env sh
# Install Forge from a packaged archive into a prefix (`artifact-and-ci-baseline`).
#
# Usage: scripts/install.sh --archive <file> --prefix <dir>
#
# Writes only under the given prefix. Refuses a missing archive, an unknown
# (nonexistent) prefix, or a digest mismatch before writing anything.
# Never fetches anything from the network.

set -eu

ARCHIVE=""
PREFIX=""

while [ "$#" -gt 0 ]; do
  case "$1" in
    --archive)
      [ "$#" -ge 2 ] || { echo "install: --archive requires a file" >&2; exit 2; }
      ARCHIVE="$2"; shift 2 ;;
    --prefix)
      [ "$#" -ge 2 ] || { echo "install: --prefix requires a directory" >&2; exit 2; }
      PREFIX="$2"; shift 2 ;;
    -h|--help)
      echo "usage: scripts/install.sh --archive <file> --prefix <dir>" >&2
      exit 0 ;;
    *)
      echo "install: unknown argument '$1'" >&2
      exit 2 ;;
  esac
done

if [ -z "$ARCHIVE" ]; then
  echo "install blocked: --archive <file> is required" >&2
  exit 2
fi
if [ -z "$PREFIX" ]; then
  echo "install blocked: --prefix <dir> is required" >&2
  exit 2
fi
if [ ! -f "$ARCHIVE" ]; then
  echo "install blocked: archive $ARCHIVE does not exist" >&2
  exit 1
fi
if [ ! -d "$PREFIX" ]; then
  echo "install blocked: prefix $PREFIX does not exist; create it first" >&2
  exit 1
fi

sh scripts/checksum.sh --verify "$ARCHIVE"

TMPD="$(mktemp -d)"
trap 'rm -rf "$TMPD"' EXIT INT TERM
tar -xzf "$ARCHIVE" -C "$TMPD"
BIN="$(find "$TMPD" -name forge -type f | head -n 1)"
if [ -z "$BIN" ]; then
  echo "install blocked: archive $ARCHIVE contains no forge binary" >&2
  exit 1
fi
RESOURCE_DIR="$(dirname "$BIN")/share/forge/runtime-templates"
CONTRACTS_RESOURCE="$(dirname "$BIN")/share/forge/contracts"
if [ ! -d "$RESOURCE_DIR" ]; then
  echo "install blocked: archive is missing share/forge/runtime-templates" >&2
  exit 1
fi
if [ ! -f "$CONTRACTS_RESOURCE/manifest.json" ] || [ ! -f "$CONTRACTS_RESOURCE/family-support.json" ]; then
  echo "install blocked: archive is missing share/forge/contracts" >&2
  exit 1
fi
GOVERNANCE_RESOURCE="$(dirname "$BIN")/share/forge/governance/check_concern.py"
QUALITY_RESOURCE="$(dirname "$BIN")/share/forge/governance/product_code_quality.py"
QUALITY_CLAIMS_RESOURCE="$(dirname "$BIN")/share/forge/governance/product_code_quality_claims.py"
QUALITY_SCOPE_RESOURCE="$(dirname "$BIN")/share/forge/governance/product_code_quality_scope.py"
VOCABULARY_MODULE="$(dirname "$BIN")/share/forge/governance/vocabulary.py"
VOCABULARY_DATA="$(dirname "$BIN")/share/forge/vocabulary.json"
if [ ! -f "$GOVERNANCE_RESOURCE" ]; then
  echo "install blocked: archive is missing share/forge/governance/check_concern.py" >&2
  exit 1
fi
if [ ! -f "$QUALITY_RESOURCE" ]; then
  echo "install blocked: archive is missing share/forge/governance/product_code_quality.py" >&2
  exit 1
fi
if [ ! -f "$QUALITY_CLAIMS_RESOURCE" ] || [ ! -f "$QUALITY_SCOPE_RESOURCE" ]; then
  echo "install blocked: archive is missing split Forge product-quality checker resources" >&2
  exit 1
fi
if [ ! -f "$VOCABULARY_MODULE" ] || [ ! -f "$VOCABULARY_DATA" ]; then
  echo "install blocked: archive is missing Forge governance vocabulary resources" >&2
  exit 1
fi
mkdir -p "$PREFIX/bin" "$PREFIX/share/forge"
cp "$BIN" "$PREFIX/bin/forge"
chmod +x "$PREFIX/bin/forge"
mkdir -p "$PREFIX/share/forge/runtime-templates"
cp -R "$RESOURCE_DIR/." "$PREFIX/share/forge/runtime-templates/"
mkdir -p "$PREFIX/share/forge/contracts"
cp -R "$CONTRACTS_RESOURCE/." "$PREFIX/share/forge/contracts/"
mkdir -p "$PREFIX/share/forge/governance"
cp "$GOVERNANCE_RESOURCE" "$PREFIX/share/forge/governance/check_concern.py"
cp "$QUALITY_RESOURCE" "$PREFIX/share/forge/governance/product_code_quality.py"
cp "$QUALITY_CLAIMS_RESOURCE" "$PREFIX/share/forge/governance/product_code_quality_claims.py"
cp "$QUALITY_SCOPE_RESOURCE" "$PREFIX/share/forge/governance/product_code_quality_scope.py"
cp "$VOCABULARY_MODULE" "$PREFIX/share/forge/governance/vocabulary.py"
cp "$VOCABULARY_DATA" "$PREFIX/share/forge/vocabulary.json"
echo "install: $PREFIX/bin/forge"
