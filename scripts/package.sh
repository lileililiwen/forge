#!/usr/bin/env sh
# Package Forge into a versioned, checksummed archive (`artifact-and-ci-baseline`).
#
# Builds `cargo build --release`, then archives the release binary plus
# LICENSE, README.md and CHANGELOG.md with repository-relative paths only
# (no absolute build-host path enters the archive). The target triple is
# discovered from `rustc -vV`, never hardcoded.
#
# Output: dist/forge-<version>-<target>.tar.gz + .sha256
# Exit status: 0 on success; non-zero naming the exact failed step.
# No network, no publish, no tag, no deploy.

set -eu

cd "$(dirname "$0")/.."

VERSION="$(sed -n 's/^version *= *"\([^"]*\)".*/\1/p' Cargo.toml | head -n 1)"
if [ -z "$VERSION" ]; then
  echo "package: cannot read version from Cargo.toml" >&2
  exit 1
fi

TARGET="$(rustc -vV | sed -n 's/^host: //p')"
if [ -z "$TARGET" ]; then
  echo "package: cannot discover target triple from rustc -vV" >&2
  exit 1
fi

for f in LICENSE README.md CHANGELOG.md; do
  if [ ! -f "$f" ]; then
    echo "package blocked: required file $f is absent" >&2
    exit 1
  fi
done

echo "package: cargo build --release" >&2
cargo build --release

BIN="target/release/forge"
if [ ! -x "$BIN" ]; then
  echo "package blocked: release binary $BIN missing after build" >&2
  exit 1
fi

mkdir -p dist
STAGE="dist/stage-forge-$VERSION-$TARGET"
ARCHIVE="dist/forge-$VERSION-$TARGET.tar.gz"
rm -rf "$STAGE" "$ARCHIVE" "$ARCHIVE.sha256"
mkdir -p "$STAGE/forge-$VERSION-$TARGET"
cp "$BIN" "$STAGE/forge-$VERSION-$TARGET/forge"
cp LICENSE README.md CHANGELOG.md "$STAGE/forge-$VERSION-$TARGET/"
tar -czf "$ARCHIVE" -C "$STAGE" "forge-$VERSION-$TARGET"
rm -rf "$STAGE"

sh scripts/checksum.sh "$ARCHIVE" > "$ARCHIVE.sha256"

echo "package: $ARCHIVE" >&2
cat "$ARCHIVE.sha256"
