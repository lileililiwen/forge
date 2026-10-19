# Tasks: consumer-conformance-matrix

## 1. BFS — Baseline and impact coverage

- [x] Read `openspec/specs/rust-platform-foundation/spec.md`,
  `crates/platform-contracts/src/payload/{mod,readiness,identity_subject}.rs`,
  `src/{validate,registry}.rs`, and
  `tests/fixture_parity.rs`; confirm the §4 field/enum table in
  design.md transcribes the sibling `schemas/<family>.schema.json`
  files 1:1.
- [x] Confirm the sibling repo is reachable (`../platform-contracts`
  or `PLATFORM_CONTRACTS_DIR`) and each of the 5 families has a
  12-fixture `manifest.json`; record actual counts.
- [x] Verify the implementation-handoff gate: language (Rust, edition
  2021, toolchain 1.95.0), crate boundary (`platform-contracts`
  only), shared owner (in-place extension, no extraction), and the
  must-not-change list (design §1) are all actionable without further
  decisions.
- [x] Add the 5 walker tests first and run them against the unchanged
  generic validator to confirm the no-validator-change decision
  before writing payload code.

## 2. DFS — Requirement-by-requirement implementation

- [x] 2.1 `payload/gate_result.rs`: `GateResult`, `GateCheck`,
  `GateResultValue` per design §4, with round-trip unit test
  mirroring `valid.basic.json`.
- [x] 2.2 `payload/job_outcome.rs`: `JobOutcome`, `JobResult` per
  design §4, with round-trip unit test mirroring `valid.basic.json`.
- [x] 2.3 `payload/release_evidence.rs`: `ReleaseEvidence`,
  `EvidenceKind` per design §4, with round-trip unit test mirroring
  `valid.basic.json`.
- [x] 2.4 `payload/capability.rs`: `Capability`, `Stability` per
  design §4, with round-trip unit test mirroring `valid.basic.json`.
- [x] 2.5 `payload/mod.rs` + `lib.rs`: register the 4 modules,
  re-exports, and `ContractPayload` impls with canonical family ids.
- [x] 2.6 `tests/fixture_parity.rs`: 5 walker tests
  (`readiness`, `gate_result`, `job_outcome`, `release_evidence`,
  `capability`) via `walk_family`; `cargo test -p platform-contracts`
  green (60 new fixture cases + round-trips).
- [x] 2.7 Spec delta
  `specs/rust-platform-foundation/spec.md`: ADDED requirement pinning
  the 5-family payload + walker coverage.

## 3. BFS — Cross-surface regression and completeness

- [x] `cargo test --workspace` fully green; existing 4-family walkers,
  e2e test, and `platform-arch-tests` unchanged in outcome.
- [x] Re-inspect `platform-core|identity|audit|testing` public APIs
  for zero drift; `git status` shows only the design §1 file list.
- [x] Confirm no sibling repository file was modified and no new
  dependency entered any `Cargo.toml`.
- [x] Confirm no placeholder/TBD text remains in the change artifacts
  and every README/ROADMAP count touched by this change reflects 9
  of 9 roadmap families.

## 4. Verification

- [x] `cargo test -p platform-contracts`
- [x] `cargo test --workspace`
- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --workspace --all-targets -- -D warnings`
- [x] `node scripts/check-openspec-change-names.mjs`
- [x] `openspec validate --changes --strict --no-interactive`
- [x] `openspec validate --specs --strict --no-interactive`
- [x] `git diff --check`
- [x] Record all outputs in `HANDOFF.md`; completion follows the
  repo's two-commit cadence (implementation+archive, then HANDOFF
  pointer).
