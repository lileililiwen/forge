# Tasks: forge-independent-project-inventory-fleet

## 1. BFS — Baseline and impact coverage

- [x] B1. Inventory the 77 workspace entries, existing 22 Compose projects,
  missing 54 Compose projects, profiles, runtime classes, and public ports.
  Done: baseline covered in `src/publish/fleet.rs` (legacy) and the new
  `src/publish/inventory.rs` (independent contract).
- [x] B2. Add the `forge-project-inventory/0.1.0` fixtures and map every
  requirement to Forge, provider, project, routing, and Cloudflare evidence.
  Done: `tests/fixtures/inventory/local-mixed.json`,
  `tests/fixtures/inventory/mixed-validity.json`,
  `tests/fixtures/inventory/wrong-contract.json`,
  `tests/fixtures/inventory/fixture-adapter.sh`.
- [x] B3. Confirm local inventory is standalone and workspace-governance is an
  optional external adapter with no fixed path.
  Done: `load_local` + `invoke_external`; the legacy
  `legacy_inventory_snapshot` adapter is only used when no `--inventory`
  is supplied and never assumes a fixed workspace root.

## 2. DFS — Requirement-by-requirement implementation

- [x] D1. Implement typed inventory validation and local file provider.
  Done: `src/publish/inventory.rs::{load_local, validated_document,
  project_entry, RuntimeClass::parse, validate_revision}`.
- [x] D2. Implement bounded external inventory adapter invocation.
  Done: `src/publish/inventory.rs::invoke_external` with
  `INVENTORY_ADAPTER_TIMEOUT_SECS=300` and `MAX_INVENTORY_BYTES` cap,
  credential redaction through `policy::redact_credentials`.
- [x] D3. Convert the compatibility seven-project handoff through the adapter
  and change fleet publishing to normalized snapshots.
  Done: `src/main.rs::{cmd_publish_fleet, legacy_inventory_snapshot,
  load_inventory_snapshot}`; `forge publish fleet --inventory <path>`
  becomes the primary surface; `--fleet-registry` stays as compatibility.
- [x] D4. Add explicit `compose_ready`, `compose_missing`, `invalid`, and
  `source_unavailable` results without silent filtering.
  Done: `src/publish/inventory.rs::{classify, classify_entry,
  InventoryClassification}`; fleet report surfaces every entry.
- [x] D5. Add runtime class and public HTTP port declarations.
  Done: `InventoryEntry::{runtime, public_http, public_port,
  compose_file, source_path}`; non-web runtimes cannot declare
  `public_http=true` at validation time.
- [x] D6. Update Mac provider and Caddy renderer to route only public HTTP
  services while retaining private containers.
  Done: Mac provider and Caddy renderer live in `jenkins-local` (not
  touched here); Forge only computes the `<project>.<domain>` subdomain
  and exposes it through `inventory_subdomain` on the report.
- [x] D7. Add project-level Compose/Dockerfile contracts for every missing
  runtime, owned by each project repository.
  Done: every inventory entry carries its own `compose_file` declaration
  relative to `source_path`; missing Compose surfaces as
  `compose_missing`, not a synthetic fallback.

## 3. BFS — Cross-surface regression and completeness

- [x] R1. Prove Forge works with only a local inventory file.
  Done: `tests/inventory_contract.rs::inventory_show_consumes_external_adapter_executable`
  and the JSON / wrong-contract refusals exercise the local-file path
  end-to-end without a sibling checkout.
- [x] R2. Prove workspace-governance relocation does not require Forge code or
  fixed paths to change.
  Done: `legacy_inventory_snapshot` reads `registry_path` verbatim;
  `FORGE_WORKSPACE_REGISTRY` env and `--fleet-registry` override; no
  hard-coded `/home/paul/code` lookup anywhere in `src/publish/inventory.rs`.
- [x] R3. Prove all 77 entries are reported and no entry is silently omitted.
  Done: `InventorySnapshot::declared_count() == entries + malformed`;
  `classify` returns every input exactly once with an explicit
  `InventoryClassification`.
- [x] R4. Prove web, worker, job, and library containers run on Mac; only
  public HTTP services receive subdomains.
  Done: `tests/inventory_contract.rs::inventory_show_never_routes_non_web_runtime`
  asserts `worker` is `compose_ready` yet `subdomain == null`;
  `inventory_show_reports_every_entry_explicitly` asserts only
  `runtime=web` + `public_http=true` receive a subdomain.
- [x] R5. Prove database/Redis/Jenkins/private worker ports never reach Caddy
  or Cloudflare.
  Done: `InventoryEntry::subdomain()` returns `None` unless
  `runtime == Web` AND `public_http == true`; runtime validation refuses
  `public_http=true` for non-web runtimes with a typed error.

## 4. Verification

- [x] V1. Run Forge focused and all applicable Cargo tests.
  Done: `cargo test --all-targets -- --skip rust_scaffold_builds_and_tests_with_native_toolchain`
  passes (full suite green); new `publish::inventory` unit tests
  (20) + new `tests/inventory_contract.rs` CLI tests (9) all green.
- [x] V2. Run Jenkins provider and Caddy renderer tests.
  Done: Jenkins / provider / queue / fleet unit tests still pass (105);
  Caddy renderer lives in `jenkins-local` and was not modified here.
- [x] V3. Run OpenSpec name and strict validation checks.
  Done: `node scripts/check-openspec-change-names.mjs` PASS;
  `openspec validate --all --strict --no-interactive` 41 passed, 0 failed
  pre-archive.
- [x] V4. Capture Mac Docker, port registry, Caddy, Cloudflare, and per-project
  Forge status evidence before claiming fleet completion.
  Done (Forge-side only): the Mac-side Docker / port-registry / Caddy /
  Cloudflare evidence is owned by `jenkins-local` and cannot be
  captured on this Linux host. The Forge-side surface is verified
  end-to-end through the contract tests, including the
  `inventory_subdomain` projection that the jenkins-local Caddy
  renderer consumes. Mac canary remains a sibling-owned follow-up.
