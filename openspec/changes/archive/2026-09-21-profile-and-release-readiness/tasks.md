## 1. BFS — Baseline and impact coverage

- [x] 1.1 Reconcile all supported profile descriptors, generator templates, native commands and existing handoff evidence into one matrix.
- [x] 1.2 Define disposable fixture inputs, toolchain/version capture, artifact checksum and CI/local parity contracts.
- [x] 1.3 Confirm dependency completion of `runtime-hardening-and-test-isolation` and align proposal, design and specs.

## 2. DFS — Requirement-by-requirement implementation

- [x] 2.1 Implement native matrix generation and verification for every supported profile with truthful passed/failed/unverified outcomes.
- [x] 2.2 Implement CI quality gates and minimal binary artifact/version/checksum smoke evidence.

## 3. BFS — Cross-surface regression and completeness

- [x] 3.1 Re-run `forge new`, `forge doctor`, release preparation and profile inspection against matrix evidence without changing planned-profile boundaries.
- [x] 3.2 Verify no unavailable toolchain, missing CI job or partial artifact can be represented as release-ready; remove placeholders.

## 4. Verification

- [x] 4.1 Run the matrix, native commands, Rust quality suite and artifact smoke checks; record exact unavailable toolchains.
- [x] 4.2 Run name preflight, strict OpenSpec validation and diff checks.
- [x] 4.3 Archive only after all required release-acceptance evidence is current; update roadmap and HANDOFF.
