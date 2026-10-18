#!/usr/bin/env sh
# Forge standard snapshot (baseline-client@1.2.0).
set -eu
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"
exec forge gate .
