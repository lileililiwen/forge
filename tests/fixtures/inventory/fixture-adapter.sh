#!/usr/bin/env bash
# hermes-fixture-inventory-adapter.sh — staged external inventory
# adapter used by tests/inventory_contract.rs. Reads the standard
# inventory adapter envelope from stdin and writes a complete
# forge-project-inventory/0.1.0 document to stdout. The adapter
# materialises the `__STAGE__` directory alongside itself so the
# fixture entry resolves to compose_ready without external setup.
set -euo pipefail

# Read the request envelope (unused; kept for contract conformance).
cat >/dev/null

stage_dir="$(dirname "$(readlink -f "$0")")/__STAGE__"
mkdir -p "$stage_dir"
echo "services: {}" > "$stage_dir/docker-compose.yml"

cat <<JSON
{"contract":"forge-project-inventory/0.1.0","provider":"fixture-adapter","generated_at":"2026-09-28T00:00:00Z","projects":[{"id":"adapter-web","repository":"https://example.invalid/adapter-web.git","revision":"0123456789abcdef0123456789abcdef01234567","profile":"rust-product","runtime":"web","compose_file":"docker-compose.yml","source_path":"$stage_dir","public_http":true,"public_port":8080}]}
JSON
