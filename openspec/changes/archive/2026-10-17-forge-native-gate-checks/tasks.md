## 1. BFS — Baseline and impact coverage

- [x] 1.1 Map every `forge gate` caller and reader: CLI, doctor, readiness, provider evidence, release, API, workbench, evidence export, package smoke, CI, and existing gate contract tests.
- [x] 1.2 Confirm the bundled-engine dispatch, default-manifest behavior, baseline pack identity, evidence compatibility, and failure boundaries against the design and canonical specs.
- [x] 1.3 Add failing contract tests for a manifest-free target, baseline suppression attempts, project extension union, absent external runtime, and dry-run side effects; observe the missing behavior before implementation and pass the contracts afterward. Historical evidence compatibility is covered by the existing reader fixtures.
- [x] 1.4 Confirm the implementation handoff is sufficient to implement without choosing a new configuration format, runtime, or policy owner.

## 2. DFS — Requirement-by-requirement implementation

- [x] 2.1 Add a private Forge entrypoint for its bundled gate engine; preserve bounded project-extension execution without an external executable or DriftWatch project-history writes.
- [x] 2.2 Add the immutable Forge baseline pack and make its `source-file-size` scanner run exactly once for every valid target, including when no project manifest exists; prevent manifests from exempting or weakening it.
- [x] 2.3 Wire `forge gate` to the bundled engine and retain Forge-owned evidence persistence, journal verdicts, typed errors, stable output, and the existing evidence contract.
- [x] 2.4 Record runtime `forge` and include the baseline version in effective manifest identity without rewriting historical evidence; ensure legacy runtime declarations and `FORGE_GATE_BIN` do not select another runtime.
- [x] 2.5 Add and pass contract tests for baseline, extension union, runtime independence, dry-run side effects, and baseline enforcement; preserve historical evidence reader fixtures.

## 3. BFS — Cross-surface regression and completeness

- [x] 3.1 Recheck the shared evidence projection and existing historical runtime fixtures used by CLI, doctor, readiness, provider matrix, release, API, and workbench readers.
- [x] 3.2 Recheck manifest refusal, source-size enforcement against weakening, a passing project extension, project-relative oversized-file diagnostics, Forge evidence persistence, no DriftWatch state writes, and dry-run zero-side-effect behavior. Split the pre-existing oversized Forge source file discovered by the mandatory baseline.
- [x] 3.3 Update requirement.md §24, architecture, README, provider evidence, Forge CI, and gate documentation so Forge is the gate owner and no external gate runtime is described as required.
- [x] 3.4 Confirm the follow-up package boundary for CI/pre-commit template generation and legacy adoption; do not implement those templates in this change.

## 4. Verification

- [x] 4.1 Run focused formatting/tests and `cargo build --workspace --all-features`; run Clippy and record pre-existing lint failures in untouched bundled tools.
- [x] 4.2 Run workspace tests and record the linker/disk-quota blocker plus the passing focused gate contract and manifest suites.
- [x] 4.3 Run the name preflight, strict OpenSpec validation, and `git diff --check`; inspect every changed and new file.
- [x] 4.4 Run Forge Gate dry-run and bounded full Gate; record its actual verdict, attribution, and remediation path in HANDOFF.
- [x] 4.5 Archive without `--skip-specs` and verify promoted canonical specs and the empty or next eligible queue.
