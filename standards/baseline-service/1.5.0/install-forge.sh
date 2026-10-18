#!/usr/bin/env sh
# Install the exact Forge release pinned by baseline-service@1.5.0.
set -eu

FORGE_VERSION=0.1.0
TARGET=x86_64-unknown-linux-gnu
ARCHIVE="forge-${FORGE_VERSION}-${TARGET}.tar.gz"
RELEASE_TAG="v${FORGE_VERSION}"
BASE_URL="https://github.com/lileililiwen/forge/releases/download/${RELEASE_TAG}"

RUNNER_TEMP_DIR="${RUNNER_TEMP:-}"
GITHUB_PATH_FILE="${GITHUB_PATH:-}"
if [ -z "$RUNNER_TEMP_DIR" ] || [ ! -d "$RUNNER_TEMP_DIR" ]; then
  echo "forge-install blocked: RUNNER_TEMP must name an existing directory" >&2
  exit 1
fi
if [ -z "$GITHUB_PATH_FILE" ] || [ ! -f "$GITHUB_PATH_FILE" ] || [ ! -w "$GITHUB_PATH_FILE" ]; then
  echo "forge-install blocked: GITHUB_PATH must name a writable Actions path file" >&2
  exit 1
fi

INSTALL_DIR="$RUNNER_TEMP_DIR/forge-${FORGE_VERSION}/bin"
if [ -e "$INSTALL_DIR/forge" ] || [ -L "$INSTALL_DIR/forge" ]; then
  echo "forge-install blocked: destination already contains $INSTALL_DIR/forge" >&2
  exit 1
fi

TMP_DIR="$(mktemp -d "${RUNNER_TEMP_DIR%/}/forge-download.XXXXXX")"
trap 'rm -rf "$TMP_DIR"' EXIT HUP INT TERM
ARCHIVE_PATH="$TMP_DIR/$ARCHIVE"
CHECKSUM_PATH="$ARCHIVE_PATH.sha256"

curl --fail --location --silent --show-error --output "$ARCHIVE_PATH" "$BASE_URL/$ARCHIVE"
curl --fail --location --silent --show-error --output "$CHECKSUM_PATH" "$BASE_URL/$ARCHIVE.sha256"
[ -s "$ARCHIVE_PATH" ] || { echo "forge-install blocked: downloaded archive is empty" >&2; exit 1; }
[ -f "$CHECKSUM_PATH" ] || { echo "forge-install blocked: checksum sidecar is missing" >&2; exit 1; }

LINE_COUNT="$(wc -l < "$CHECKSUM_PATH" | awk '{print $1}')"
[ "$LINE_COUNT" = 1 ] || { echo "forge-install blocked: checksum sidecar must contain exactly one line" >&2; exit 1; }
EXPECTED="$(awk -v name="$ARCHIVE" 'NF == 2 && length($1) == 64 && $1 ~ /^[0-9a-f]+$/ && $2 == name { print $1 }' "$CHECKSUM_PATH")"
[ -n "$EXPECTED" ] || { echo "forge-install blocked: checksum sidecar must name exactly $ARCHIVE with a lowercase SHA-256 digest" >&2; exit 1; }
ACTUAL="$(sha256sum "$ARCHIVE_PATH" | awk '{print $1}')"
[ "$EXPECTED" = "$ACTUAL" ] || {
  echo "forge-install blocked: checksum mismatch for $ARCHIVE (expected $EXPECTED, actual $ACTUAL)" >&2
  exit 1
}

MEMBER="forge-${FORGE_VERSION}-${TARGET}/forge"
MEMBERS="$(tar -tzf "$ARCHIVE_PATH")" || { echo "forge-install blocked: archive is invalid" >&2; exit 1; }
MATCH_COUNT="$(printf '%s\n' "$MEMBERS" | awk -v expected="$MEMBER" '$0 == expected { count++ } END { print count + 0 }')"
[ "$MATCH_COUNT" = 1 ] || { echo "forge-install blocked: archive must contain exactly one $MEMBER" >&2; exit 1; }
mkdir -p "$TMP_DIR/extracted"
tar -xzf "$ARCHIVE_PATH" -C "$TMP_DIR/extracted" "$MEMBER"
EXTRACTED="$TMP_DIR/extracted/$MEMBER"
[ -f "$EXTRACTED" ] && [ ! -L "$EXTRACTED" ] && [ -x "$EXTRACTED" ] || {
  echo "forge-install blocked: $MEMBER must be an executable regular file" >&2
  exit 1
}

mkdir -p "$TMP_DIR/install"
cp "$EXTRACTED" "$TMP_DIR/install/forge"
chmod 755 "$TMP_DIR/install/forge"
mkdir -p "$INSTALL_DIR"
mv "$TMP_DIR/install/forge" "$INSTALL_DIR/forge"
printf '%s\n' "$INSTALL_DIR" >> "$GITHUB_PATH_FILE"
echo "forge-install: installed Forge $FORGE_VERSION ($TARGET) at $INSTALL_DIR/forge"
