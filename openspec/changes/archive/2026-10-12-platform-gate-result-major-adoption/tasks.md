# Tasks: adopt `platform.gate-result` 1.0.0

## Phase 1 — BFS baseline

- [x] 1.1 `node scripts/check-openspec-change-names.mjs` PASS before
      selection; `openspec list` reports no active change.
- [x] 1.2 Read `AGENTS.md`, `.ai-rules/workflow.md`,
      `.ai-rules/completion.md`, `.ai-rules/concerns/quality.md`,
      `.ai-rules/concerns/security.md`, and the canonical specs
      `platform-contract-consumption`, `gate-runtime-evidence`,
      `gate-evidence-export-consumption`.
- [x] 1.3 Measure the drift before deciding anything:
      `scripts/contract-parity.sh` exits 1 with exactly three findings —
      `registry.json` byte mismatch, `schemas/gate-result.schema.json` byte
      mismatch, and the `platform.gate-result` family digest mismatch — and
      no finding names another family.
- [x] 1.4 Map every affected surface: `src/contract/mod.rs`
      (`map_gate_aggregate`, the inline per-check `match` in
      `emit_gate_result`, `contract_discriminator`, `build_envelope`,
      `validate_envelope`), `src/contract/contract_tests.rs`,
      `src/cli/functions_13.rs` (`forge contract inspect` human line),
      `contracts/**`, the canonical spec
      `openspec/specs/platform-contract-consumption/spec.md`.
- [x] 1.5 Confirm the callers: `forge contract emit gate-result`
      (`src/cli/functions_13.rs:779`) and the read-only admin route
      `GET /v1/admin/contracts/emit?family=platform.gate-result`
      (`src/api/admin/assurance.rs:742`) both route through
      `emit_gate_result`; `forge contract validate` and
      `forge contract inspect` call `validate_envelope` /
      `build_envelope`'s siblings directly.
- [x] 1.6 Confirm the untouched boundaries: `src/gate/**`
      (`GateAggregate`, `GateCheckState`, `GateEvidence`,
      `classify_aggregate`), `src/gate/evidence.rs`, the
      `GATE_CONTRACT_VERSION` constant, `forge gate` / `--dry-run`, the MCP
      registry, the API route table, portal sections and the command
      catalog are not in the diff.
- [x] 1.7 Set `current_spec: platform-gate-result-major-adoption` in
      HANDOFF.

## Phase 2 — DFS requirement implementation

- [x] 2.1 Re-vendor through the sanctioned writer only:
      `node scripts/sync-contracts.mjs` with `GOVERNANCE_DIR` pinned at
      `workspace-governance@0301ee8…` (the revision the mirror already
      records) so no unrelated governance vocabulary bytes enter the diff;
      confirm `contracts/registry.json` differs from its pre-sync copy only
      inside `contracts["platform.gate-result"]`.
- [x] 2.2 Confirm the manifest records the new upstream revision and fresh
      per-file sha256 values, and that `contracts/manifest.json`'s
      governance entry is byte-identical to before.
- [x] 2.3 Register the per-family wire version in
      `src/contract/mod.rs`: `PLATFORM_FAMILY_VERSIONS`,
      `platform_family_version`, `contract_discriminator` reading the
      table (no literal), and the named
      `UNREGISTERED_FAMILY_VERSION` fallback for the envelope builder.
- [x] 2.4 Replace `map_gate_aggregate` with `map_gate_result(&GateEvidence)`
      implementing design §3a, including the two-branch `Blocked`
      decomposition; keep the `Result<&'static str, ContractUnmappable>`
      shape so no existing refusal is weakened, and record in design
      §3a-bis why no gate-result row needs the `Err` arm.
- [x] 2.5 Add `map_gate_check_state(GateCheckState)` implementing design
      §3b and route `emit_gate_result`'s per-check loop through it,
      propagating the refusal as `ForgeError::ContractInvalid`.
- [x] 2.6 Rewrite `validate_envelope`'s major check to consult the
      vendored `contracts/registry.json` `supported_major_versions`, failing
      closed on an unreadable registry or a missing family record.
- [x] 2.7 `forge contract inspect`: render the registered version through
      `contract_discriminator` instead of the `/0.1.0` literal.
- [x] 2.8 Document the mapping, its evidence and every rejected
      alternative in `design.md` (§3a, §3b, §3c, §3d).

## Phase 3 — BFS regression/completeness

- [x] 3.1 Rewrite `map_gate_rows` for `map_gate_result`: every
      `GateAggregate`, both `Blocked` shapes, and the proof that no
      aggregate renders `errored`.
- [x] 3.2 Add check-level rows: every `GateCheckState`, the proof that
      `unresolved` never renders `errored`/`unbound`/`skipped`, and that
      `not_applicable` still renders `skipped`.
- [x] 3.3 Add `blocked_without_an_observed_failure_is_an_undecided_outcome`
      covering every reachable blocked shape that carries no failed row.
- [x] 3.4 Add an emit-level proof that reads the **vendored** schema and
      asserts every value the mapping can produce is inside both
      `result` and `checks[].status`, so a future upstream bump fails a
      test instead of shipping an invalid document.
- [x] 3.5 Add version-registry tests: the table covers exactly
      `supported_families()`, has no duplicate family, every value matches
      `CONTRACT_PATTERN`, `platform.gate-result` resolves to `1.0.0`, and
      `build_envelope` stamps that discriminator.
- [x] 3.6 Rewrite `validate_refuses_major_mismatch`: a declared major (1)
      is accepted, an undeclared major (2) refuses, a version-less
      discriminator refuses, and a family-less registry refuses rather
      than skipping.
- [x] 3.7 Confirm the canonical spec scenarios that pinned the old
      behaviour (`platform.gate-result/0.1.0`, `passed`/`failed`/`errored`)
      are updated, and that `gate-runtime-evidence` is untouched.
- [x] 3.8 Confirm no other contract family moved: `git diff --stat
      contracts/` names only `registry.json`,
      `schemas/gate-result.schema.json` and `manifest.json`; every
      unchanged file keeps its sha256, and the two `contracts/` entries
      the sync added or rewrote are accounted for in `design.md` §5.

## Phase 4 — Verification

- [x] 4.1 `cargo build`; `cargo fmt --check`;
      `cargo clippy` (default targets).
- [x] 4.2 `cargo test --lib contract`; `cargo test --lib`; the touched
      integration suites (`api_contract` for the admin emit route).
- [x] 4.3 `node scripts/check-openspec-change-names.mjs` PASS.
- [x] 4.4 `node scripts/check-spec-governance.mjs` PASS.
- [x] 4.5 `scripts/contract-parity.sh` exit 0 with the compared counts.
- [x] 4.6 `openspec validate --all --strict --no-interactive` PASS.
- [x] 4.7 `git diff --check` clean; reviewed the full staged set including
      newly added files.
- [x] 4.8 Archive `platform-gate-result-major-adoption` **without**
      `--skip-specs`; commit 1 = implementation + tests + specs + archive,
      explicit paths only; commit 2 = HANDOFF evidence and pointer removal.
      No push.
- [x] 4.9 Heavy Gate runtime (`forge gate --dry-run`, then bounded
      `forge gate`) is the orchestrator's step for this cycle; it is not
      run here by instruction, and the HANDOFF evidence table records that
      delegation rather than claiming a verdict.