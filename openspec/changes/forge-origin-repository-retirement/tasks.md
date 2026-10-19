## 1. BFS — Baseline and impact coverage

- [ ] 1.1 Verify the exact migrated-origin allowlist against Forge's imported source paths, import commits, and current GitHub identities.
- [ ] 1.2 Inventory all maintained consumers, workflows, package metadata, source verifiers, and historical-only origin references.
- [ ] 1.3 Capture Forge main/worktree state and status of the three existing sibling checkouts, including their untracked files.
- [ ] 1.4 Query and classify every allowlisted remote as archived, active, confirmed absent, or unavailable; keep auth/network failures unresolved.

## 2. DFS — Requirement-by-requirement retirement

- [ ] 2.1 Run Workspace Governance, .NET and Rust import verification and prove each source-only history set remains reachable in Forge.
- [ ] 2.2 Verify maintained Cargo, module Gate, CI, Governance and publisher consumers use only Forge-owned paths or declared external runtimes.
- [ ] 2.3 Re-query each active allowlisted origin immediately before action; archive only the exact confirmed active repository.
- [ ] 2.4 Read back every attempted archive and record archived, already-archived, absent, or unresolved status separately.
- [ ] 2.5 Preserve all sibling checkout statuses and untracked files; do not alter unrelated repositories or publish/push Forge.

## 3. BFS — Cross-surface regression and completeness

- [ ] 3.1 Re-run import provenance checks and inspect source-only commit ancestry after the remote operations.
- [ ] 3.2 Re-run maintained consumer checks and verify the archived origins are not required at build, test, Gate or publication time.
- [ ] 3.3 Re-capture the three sibling worktree statuses and confirm user-owned files were not modified.
- [ ] 3.4 Verify every allowlisted origin has a verified terminal state; unresolved state blocks retirement completion.
- [ ] 3.5 Verify every out-of-scope repository received no operation and no source import or remote history was rewritten.

## 4. Verification

- [ ] 4.1 Run all import verifiers, affected consumer checks, changed-file format checks and applicable tests.
- [ ] 4.2 Run name preflight, selected status/instructions, strict OpenSpec validation, and `git diff --check`.
- [ ] 4.3 Run Forge Gate dry-run and bounded full Gate after source and remote state verification; record actual results.
- [ ] 4.4 Archive without `--skip-specs`, promote canonical requirements, and verify the archive.
- [ ] 4.5 Commit only related evidence/docs/spec artifacts, update HANDOFF, then make the HANDOFF-only commit.
