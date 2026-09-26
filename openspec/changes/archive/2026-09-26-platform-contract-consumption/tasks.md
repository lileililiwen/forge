# Tasks: Consume the portfolio contract source instead of re-declaring it

## 1. BFS — Baseline and impact coverage

- [ ] Inventory the contract source before vendoring anything: confirm the nine
  registered families, each `$id`, required fields and status enums in
  `platform-contracts/schemas/*.schema.json`, the
  `secret_field_substrings` list in `schemas/registry.json`, the envelope
  required keys and `contract` pattern, and the producer/consumer rules in
  `docs/ownership.md` and `docs/migration.md`. Record the source revision being
  consumed in `contracts/manifest.json` rather than paraphrasing it.
- [ ] Inventory Forge's own versioned surfaces: every `pub const … = "0.1.0"`
  (28 across 27 modules) and every namespaced discriminator
  (`forge-checker/0.1.0`, `forge-deploy-executor/0.1.0`, the inbound
  `driftwatch-checker/0.1.0`), with the module, the constant name and the
  document each one versions.
- [ ] Capture the vocabulary divergences that the mapping must honour, with
  source lines: `GateAggregate` (`src/gate/mod.rs:89-94`),
  `ReadinessStatus` (`src/readiness/mod.rs:44-48`), `FindingStatus`
  (`src/doctor/mod.rs:34-39`), the free-form `operations.state` column
  (`src/registry/mod.rs:51`, `record_operation` at `:336`), and the checker
  severity mapping in `src/checker/mod.rs`.
- [ ] Read the two existing consumption precedents and decide what Forge
  borrows and what it must not: `rust-platform-libs`
  `crates/platform-contracts/tests/fixture_parity.rs` (resolution order
  `PLATFORM_CONTRACTS_DIR` → `../platform-contracts` → fail loudly) versus
  Forge's no-sibling-required invariant.
- [ ] Define `ContractSpec` and the refusal type `ContractUnmappable` before
  writing any projection, and state the mapping tables in design.md so a
  reviewer can disagree with a row instead of discovering it in code.
- [ ] Add failing test skeletons for every branch: digest agreement, inventory
  completeness, inventory drift, each mapping row, each refusal, envelope
  pattern enforcement, secret-field refusal, byte-identity of the four existing
  surfaces.

## 2. DFS — Requirement-by-requirement implementation

- [ ] Vendor the set: `contracts/schemas/*.schema.json` (envelope plus the nine
  families), `contracts/registry.json`,
  `contracts/vocabulary/secret-field-substrings.json` and
  `contracts/manifest.json` with the pinned revision and per-file sha256;
  implement `scripts/sync-contracts.mjs` with the documented resolution order,
  verbatim copy behaviour (no rewriting, no reordering) and refusal to write
  when a requested file is missing or unparseable.
- [ ] Implement the offline digest test (re-hash every manifest entry, name file
  and expected/actual digest on mismatch). It must pass with no sibling checkout
  and no network access.
- [ ] Implement `src/contract/`: `envelope.rs` (type mirroring
  `schemas/envelope.schema.json` exactly), `families.rs` (family ids plus
  supported majors read from the vendored registry), `map.rs` (total functions
  that refuse, one function per family), `inventory.rs` (the `CONTRACTS` table),
  and register the module in `src/lib.rs` without reordering existing entries.
- [ ] Implement the completeness and agreement tests over `src/**/*.rs` using
  `CARGO_MANIFEST_DIR`, proving a newly added versioned constant fails until it
  is registered, and that an inventory row contradicting its live constant
  fails.
- [ ] Implement `forge contract list` and `forge contract inspect <family>` as
  renderers of the inventory plus the vendored schema facts (required fields,
  enums, supported majors); unknown family refuses by name and lists supported
  families.
- [ ] Implement `forge contract emit <family> [TARGET]` for `gate-result`,
  `readiness`, `release-evidence`, `capability` and `audit-event`, each from the
  record Forge already persists or computes; strictly read-only — no file write,
  no journal row, no mutation; a missing source record yields a named refusal
  and no envelope.
- [ ] Implement `forge contract validate <file|->` against the vendored schemas:
  envelope pattern, `generated_at` RFC3339, unknown family and unsupported major
  refused, payload schema failures reported with their location, and any field
  name carrying a registered secret substring refused without echoing the value.
- [ ] Union the consumed `secret_field_substrings` into
  `policy::redact_credentials` as additional key-name coverage while keeping
  every existing value-shape pattern; keep the eleven importing modules calling
  one function; state in the module docs that the registry governs field names
  in shared documents and Forge's redactor governs captured runtime output.
- [ ] Implement `scripts/contract-parity.sh`: walk the vendored family list
  against a pinned checkout using the documented resolution order, print
  per-family fixture counts, and fail loudly with a next-action message when the
  source cannot be reached. Ship the live walk as an `#[ignore]`d test so
  default `cargo test` stays standalone-green.
- [ ] Add `jsonschema` under `[dev-dependencies]` only, and assert in a test
  that the runtime dependency set does not contain it.

## 3. BFS — Cross-surface regression and completeness

- [ ] Prove the additive boundary with pinned digests: `forge gate status .
  --format json`, `forge check .`, `forge fleet list` and `forge provider
  matrix` byte-identical before and after this capability, using the
  digest-pinning technique already established in
  `tests/workspace_metadata_contract.rs`.
- [ ] Prove read-only behaviour: contract calls leave the manifest, registry,
  operations journal row count, `.forge/**` tree and git HEAD untouched;
  repeated calls differ only in `generated_at`.
- [ ] Prove transport non-exposure: MCP `tools/list` snapshot unchanged, no API
  route added, `forge portal view contract` refuses `portal-invalid`, and the
  checker document gains no alert from a contract call.
- [ ] Re-check every surface that consumes `policy::redact_credentials` for
  regression after the union (policy, gate, checker, fleet, governance, deploy,
  release, distribution, docs, analytics, identity, provider, agent).
- [ ] Confirm the refusal rule holds everywhere: no mapping path can produce a
  contract `passed`/`ready`/`success` from Forge `unverified`, `unknown`,
  `unavailable`, `pending` or `partial`.
- [ ] Confirm generated projects are untouched: a `forge new` tree stays
  byte-identical with and without this capability, and `--no-workspace-metadata`
  still reproduces the pinned pre-change digests.
- [ ] Update documentation to reflect the new upstream without overstating it:
  `docs/architecture.md` integration boundary, `docs/adapter-contracts/` or a
  new `docs/contract-consumption.md` naming the vendored set, the pinned
  revision and which families Forge projects; `docs/provider-evidence.md` gains
  a `contract-parity` row that is honestly `not-run` by default; README gets a
  short "Contract consumption" pointer.

## 4. Verification

- [ ] `cargo fmt --all -- --check`; `cargo build`; `cargo clippy --all-targets
  -- -D warnings`; `cargo test --all-targets` with the long-running
  native-toolchain scaffold test excluded and then run separately, recording the
  exact command and result as previous cycles did.
- [ ] Run `scripts/contract-parity.sh` against the pinned checkout and record the
  per-family counts; if the source is unreachable, record the exact command and
  the next action instead of a pass.
- [ ] Live dogfood: against a scratch project with real gate evidence from the
  installed sibling, run `forge contract emit gate-result <project>` and pipe it
  into `forge contract validate -`; record the document summary. For this
  repository, `forge gate .` remains honestly `gate-runtime-unavailable` until
  the sibling-owned `.driftwatch` store exists, so no gate pass or gate-result
  envelope is claimed here.
- [ ] `node scripts/check-openspec-change-names.mjs`; `openspec validate --all
  --strict --no-interactive`; `git diff --check` plus review of newly added
  files; archive without `--skip-specs` only after every scoped scenario has
  current evidence.
- [ ] Record in HANDOFF which families project, which rows refuse, the pinned
  source revision, and the remaining companion work
  (`governance-vocabulary-consumption` for capability naming and
  `artifact-and-ci-baseline` for the CI job that clones the source).
