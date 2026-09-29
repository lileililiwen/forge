# Tasks: project-evidence-gap-assessment

Status: planning-only package. Every task is unchecked by design.

## 1. BFS — Baseline and impact coverage

- [ ] 1.1 Read `src/doctor/mod.rs` and `src/spec/mod.rs`; record the existing verdict vocabulary, finding IDs, evidence attribution and remediation classes to reuse.
- [ ] 1.2 Confirm the implementation-handoff gate: Rust 1.87+, `src/doctor/gaps.rs`, CLI `forge project gaps`, dependency on the catalog contract, no new dependency.
- [ ] 1.3 Map each requirement and scenario to its rule, category, verdict, callers, failure cases and named tests.

## 2. DFS — Requirement-by-requirement implementation

- [ ] 2.1 Implement the closed category, verdict and remediation-class vocabularies and the stable finding id.
- [ ] 2.2 Implement missing/stale/invalid/unavailable/unverified rules over catalog records, with profile-derived applicability.
- [ ] 2.3 Add `forge project gaps` with filters and JSON/NDJSON output; redact credentials on every path.
- [ ] 2.4 Reuse the doctor and specification-remediation classification rather than introducing a second routing table.

## 3. BFS — Cross-surface regression and completeness

- [ ] 3.1 Prove assessment is read-only (no file, registry byte, table or journal write).
- [ ] 3.2 Exercise empty, duplicate, stale, unavailable, not-applicable and conflicting-source fixtures.
- [ ] 3.3 Prove existing doctor findings are unchanged by the new inspector.

## 4. Verification

- [ ] 4.1 Run `cargo fmt/build/clippy/test`, the two new suites, `openspec validate --all --strict --no-interactive`, the name preflight and `git diff --check`; record the exact output.
- [ ] 4.2 Record that no repair was performed and no provider was contacted; repair and provider writes are separate packages.
