#!/usr/bin/env sh
# Refuse a release tag that does not identify this exact Cargo package version.
set -eu

if [ "$#" -ne 1 ]; then
  echo "release-tag blocked: usage: scripts/validate-release-tag.sh v<version>" >&2
  exit 2
fi

VERSION="$(sed -n 's/^version *= *"\([^"]*\)".*/\1/p' Cargo.toml | head -n 1)"
if [ -z "$VERSION" ]; then
  echo "release-tag blocked: cannot read package version from Cargo.toml" >&2
  exit 1
fi

EXPECTED="v$VERSION"
if [ "$1" != "$EXPECTED" ]; then
  echo "release-tag blocked: tag '$1' does not match package version '$EXPECTED'" >&2
  exit 1
fi

echo "release-tag: $EXPECTED matches Cargo.toml version $VERSION"
