# Tasks: project-local-remediation-plans

Status: planning-only package. Every task is unchecked by design.

## 1. BFS — Baseline and impact coverage

- [ ] 1.1 Read `src/standard/`, `src/generate/` and `src/spec/mod.rs`; record the owned-asset receipt, stage/promote/cleanup discipline and finding routing to reuse.
- [ ] 1.2 Confirm the implementation-handoff gate: Rust 1.87+, `src/remediation/*`, CLI `forge remediate …`, dependency on the gap assessment and standard packs, no new dependency.
- [ ] 1.3 Map each requirement and scenario to its action kind, precondition, caller, failure case and named test.

## 2. DFS — Requirement-by-requirement implementation

- [ ] 2.1 Implement the `forge-remediation-plan/0.1.0` plan, action and outcome shapes with a closed action-kind set.
- [ ] 2.2 Implement finding selection restricted to `automatic`, with semantic/manual routing refused.
- [ ] 2.3 Implement the apply engine: confirmation, ownership check, digest/revision preconditions, stage/verify/promote, idempotent re-apply and rollback-on-failure.
- [ ] 2.4 Implement `scan|plan|diff|apply` with `--target`, `--finding`, `--confirm`, `--format`, consuming standard-pack assets only.

## 3. BFS — Cross-surface regression and completeness

- [ ] 3.1 Prove preview paths write nothing and apply preserves unrelated files.
- [ ] 3.2 Exercise unowned collision, stale revision, missing asset, invalid profile, partial write and rollback-failure boundaries.
- [ ] 3.3 Prove no secret is generated or disclosed and no remote mutation occurs.

## 4. Verification

- [ ] 4.1 Run `cargo fmt/build/clippy/test`, the two new suites, `openspec validate --all --strict --no-interactive`, the name preflight and `git diff --check`; record the exact output.
- [ ] 4.2 Record that no GitHub write, deployment or semantic generation was performed; those are separate packages.
