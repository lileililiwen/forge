#!/usr/bin/env sh
# Suggest the next version bump for Forge (`artifact-and-ci-baseline`).
#
# Usage: scripts/bump.sh [--to <semver>]
#
# Prints the version + changelog + tag suggestion. With --to it updates
# Cargo.toml and prepends a CHANGELOG.md entry; tag creation and any push
# remain explicit operator actions (never performed here).

set -eu

cd "$(dirname "$0")/.."

CURRENT="$(sed -n 's/^version *= *"\([^"]*\)".*/\1/p' Cargo.toml | head -n 1)"

if [ "$#" -eq 0 ]; then
  echo "current version: $CURRENT"
  echo "suggested tag: v$CURRENT"
  echo "to bump: scripts/bump.sh --to <semver> (then operator tags v<semver> explicitly)"
  exit 0
fi

if [ "${1:-}" != "--to" ] || [ "$#" -ne 2 ]; then
  echo "usage: scripts/bump.sh [--to <semver>]" >&2
  exit 2
fi

NEXT="$2"
case "$NEXT" in
  *.*.*) ;;
  *) echo "bump blocked: '$NEXT' is not semver x.y.z" >&2; exit 2 ;;
esac

DATE="$(date +%Y-%m-%d)"
sed -i "s/^version *= *\"$CURRENT\"/version = \"$NEXT\"/" Cargo.toml
TMPF="$(mktemp)"
{
  echo "# Changelog"
  echo ""
  echo "All notable changes to Forge are documented here. The format follows"
  echo "[Keep a Changelog](https://keepachangelog.com/en/1.0.0/) and this project"
  echo "adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html)."
  echo ""
  echo "## [$NEXT] - $DATE"
  echo ""
  echo "### Changed"
  echo ""
  echo "- Release $NEXT (bumped from $CURRENT)."
  echo ""
  tail -n +8 CHANGELOG.md
} > "$TMPF"
mv "$TMPF" CHANGELOG.md

echo "bump: $CURRENT -> $NEXT"
echo "suggested tag (operator action, not created here): v$NEXT"
