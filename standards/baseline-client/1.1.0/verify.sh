#!/usr/bin/env sh
# Forge standard snapshot (baseline-client@1.1.0).
set -eu
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"
exec forge gate .
