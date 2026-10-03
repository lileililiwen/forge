# Tasks: contract-parity-gate-real-digests

Phase order is BFS baseline → DFS requirement implementation → BFS
regression/completeness → Verification, per `.ai-rules/workflow.md`. Checkbox
state reflects only work actually completed; `openspec archive` is **not** run
and no git operation is performed by this package.

## 1. BFS — Baseline and impact coverage

- [x] 1.1 Measure the defect before changing anything: confirm
  `scripts/contract-parity.sh` is 30 lines and that
  `grep -cE "sha256|cmp|diff" scripts/contract-parity.sh` returns `0`; record
  the unconditional `contract-parity: OK` line and the fact that the only
  reachable failure is an unresolvable source directory.
- [x] 1.2 Confirm the three compounding facts: `scripts/release-check.sh` runs
  the step only when a source resolves and otherwise notes-and-continues;
  `.github/workflows/ci.yml` job `contract-parity` hard-fails only on sibling
  absence; and `verify_manifest_digests()` (`src/contract/mod.rs:349`) compares
  `contracts/**` against `contracts/manifest.json`, never against upstream.
- [x] 1.3 Measure the actual drift between `contracts/` and
  `../platform-contracts/`: 11 of 12 mirrored files byte-identical,
  `contracts/registry.json` differing, and
  `contracts/schemas/public-portfolio-manifest.schema.json` absent from the
  mirror though upstream declares `platform.public-portfolio-manifest` at
  `current_version` `1.0.0`.
- [x] 1.4 Map the callers and the readers before writing the script:
  `scripts/release-check.sh` (invocation under `set -eu`),
  `.github/workflows/ci.yml` job `contract-parity`, and the Rust consumers of
  `contracts/` — `family_schema_path()`, `secret_field_substrings()`,
  `vendored_revision()` (`src/vocabulary.rs:291`) and
  `manifest_digests_match` (`src/contract/mod.rs:773`) — establishing that no
  Rust code reads `current_version` and that deleting the mirror is out of scope.
- [x] 1.5 Read the producer-side vocabulary from
  `platform-contracts/openspec/changes/contract-consumption-enforcement/` and
  the archived `2026-09-29-contracts-consumer-parity` package for the consumer
  contract-consumption wording, without editing that repository.

## 2. DFS — Requirement implementation

- [x] 2.1 `parity-source-unresolvable` / `parity-source-record-invalid`:
  preserve the current resolution order (variable when it names a directory,
  then the `../platform-contracts` sibling); keep the missing-source exit
  non-zero; additionally require the source's `manifest.json` to be readable and
  to declare at least one family.
- [x] 2.2 `parity-self-reference`: refuse a resolved source that is Forge's own
  `contracts/` directory, so the one input that could make the check pass
  trivially is closed.
- [x] 2.3 Anchor A — derive the compared set from both sides and hash each file
  on both sides: `contracts/registry.json` ↔ `<src>/schemas/registry.json` and
  `contracts/<rel>` ↔ `<src>/<rel>`, excluding `contracts/manifest.json` and
  `contracts/vocabulary/**`. Emit `parity-missing-mirror` for a source file with
  no vendored counterpart and `parity-undeclared-vendor` for the reverse.
- [x] 2.4 Anchor B — compare each declared family's mirror digest against the
  source manifest's `schema_digest` and report the source's
  `registry_revision`, so the digest authority is the contract's own record and
  never `contracts/manifest.json`.
- [x] 2.5 `parity-mismatch` / `parity-family-digest-mismatch`: accumulate every
  failure and print all of them before exiting, naming both digests.
- [x] 2.6 `parity-nothing-compared`: count the comparisons, and make a zero
  count a non-zero exit that states it verified nothing.
- [x] 2.7 Move the pass line to a single reachable site, gated on a non-zero
  comparison count and an empty failure list, so no path prints `OK` after
  comparing nothing.
- [x] 2.8 Re-sync the mirror: copy the source's `schemas/registry.json` to
  `contracts/registry.json` and vendor the missing
  `contracts/schemas/public-portfolio-manifest.schema.json`, then update the
  two affected `contracts/manifest.json` digests and add the new entry, keeping
  Forge's own manifest truthful for the offline check.
- [x] 2.9 Correct the `scripts/release-check.sh` comment and docstring so they no
  longer describe the step as an informational walk, keeping the invocation, the
  `set -eu` propagation and the documented local/CI divergence unchanged.
- [x] 2.10 `scripts/contract-parity.sh` stays `#!/bin/sh` with `set -eu`, as
  `AGENTS.md` mandates no other shell dialect and the file already is.

## 3. BFS — Regression and completeness

- [x] 3.1 Re-run the four acceptance cases against the implemented script:
  corrupted mirror (non-zero, no pass line), correct mirror (zero), no source
  (non-zero), and self-reference (non-zero).
- [x] 3.2 Confirm `cargo test --workspace --all-targets` still passes, including
  `manifest_digests_match` and `vendored_copy_loads_with_pinned_revision`,
  which read the re-synced mirror.
- [x] 3.3 Confirm no pre-existing CI job, check or failure path was removed or
  weakened, by reviewing the diff of `.github/workflows/ci.yml` and
  `scripts/release-check.sh`.
- [x] 3.4 Confirm `contracts/vocabulary/README.md` remains absent from
  `contracts/manifest.json`, as a pre-existing gap this package deliberately does
  not fold in, and that it causes no offline digest failure.
- [x] 3.5 Confirm the change adds no new CLI verb, transport, dependency,
  persistence or capability, and that `supported_families()` and
  `family_schema_path()` are untouched so the mirror update does not widen
  Forge's supported surface.

## 4. Verification

- [x] 4.1 `cargo test --workspace --all-targets` — **lib unit tests: 1171
  passed, 0 failed, 1 ignored** (1172 collected), identical to the 1171
  baseline, measured on two consecutive full runs. The clean run reports 0
  failures across all 105 targets, 2292 tests passed in total. An earlier run
  showed one spurious failure, `unknown_route_returns_404` in
  `tests/api_contract.rs`, with `ConnectionReset` under parallel load; it opens
  a raw `TcpStream`, touches no contract data, passes in isolation on repeated
  runs, and did not recur. Pre-existing flakiness, not a regression.
- [x] 4.2 `openspec validate --all --strict --no-interactive` — 64 passed, 0
  failed; `openspec validate contract-parity-gate-real-digests --strict
  --no-interactive` — valid. The delta is filed under
  `platform-contract-consumption`, which exists in `openspec/specs/`, so it will
  not abort at archive time.
- [x] 4.3 `node scripts/check-openspec-change-names.mjs` passes, and `HANDOFF.md`
  now carries exactly one `current_spec: contract-parity-gate-real-digests`
  line, which `check-spec-governance.mjs` does not object to.
- [ ] 4.4 `git diff --check` — clean. **But `check-spec-governance.mjs` still
  fails**, and it is not this package's to fix: it reports
  `openspec/specs/scaffold-prewires-shared-layer/spec.md:4` for a leftover
  `TBD - created by archiving change …` placeholder. That file is committed,
  untouched here (`git status openspec/specs/` is empty) and the failure
  reproduces without this change. Left for the change that owns that archive;
  recorded here so it is not mistaken for a regression from this work.
- [x] 4.5 `cargo fmt --all -- --check` and `cargo clippy` are unaffected: no Rust
  file changed in this package (`git status src/ Cargo.toml` is empty), and the
  change is therefore additive at the repository level.
- [ ] 4.6 `openspec archive` is **not** run and no commit is made; the parent
  session commits. The change stays active.
