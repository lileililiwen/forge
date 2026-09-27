#!/usr/bin/env sh
# Regenerate or verify sha256 digests for Forge artifacts (`artifact-and-ci-baseline`).
#
# Usage:
#   scripts/checksum.sh <archive>            # print "<sha256>  <basename>"
#   scripts/checksum.sh --verify <archive>   # verify against <archive>.sha256
#
# Refuses a missing archive or a missing/mismatching digest before any install.
# Exit status: 0 on match; 1 on missing input; 2 on digest mismatch.

set -eu

VERIFY=0
if [ "${1:-}" = "--verify" ]; then
  VERIFY=1
  shift
fi

if [ "$#" -ne 1 ]; then
  echo "usage: scripts/checksum.sh [--verify] <archive>" >&2
  exit 2
fi

ARCHIVE="$1"
if [ ! -f "$ARCHIVE" ]; then
  echo "checksum blocked: archive $ARCHIVE does not exist" >&2
  exit 1
fi

if [ "$VERIFY" -eq 1 ]; then
  DIGEST_FILE="$ARCHIVE.sha256"
  if [ ! -f "$DIGEST_FILE" ]; then
    echo "checksum blocked: digest file $DIGEST_FILE does not exist" >&2
    exit 1
  fi
  EXPECTED="$(awk '{print $1}' "$DIGEST_FILE")"
  ACTUAL="$(sha256sum "$ARCHIVE" | awk '{print $1}')"
  if [ "$EXPECTED" != "$ACTUAL" ]; then
    echo "checksum mismatch: expected $EXPECTED, actual $ACTUAL ($ARCHIVE)" >&2
    exit 2
  fi
  echo "checksum: OK $ARCHIVE ($ACTUAL)"
else
  sha256sum "$ARCHIVE" | awk '{print $1 "  " $2}' | sed "s| .*\/|  |"
fi
