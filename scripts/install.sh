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

mkdir -p "$PREFIX/bin"
TMPD="$(mktemp -d)"
trap 'rm -rf "$TMPD"' EXIT INT TERM
tar -xzf "$ARCHIVE" -C "$TMPD"
BIN="$(find "$TMPD" -name forge -type f | head -n 1)"
if [ -z "$BIN" ]; then
  echo "install blocked: archive $ARCHIVE contains no forge binary" >&2
  exit 1
fi
cp "$BIN" "$PREFIX/bin/forge"
chmod +x "$PREFIX/bin/forge"
echo "install: $PREFIX/bin/forge"
