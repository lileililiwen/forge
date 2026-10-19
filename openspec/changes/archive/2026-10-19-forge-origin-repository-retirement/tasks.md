## 1. BFS — Baseline and impact coverage

- [x] 1.1 Verify the exact migrated-origin allowlist against Forge's imported source paths, import commits, and current GitHub identities.
- [x] 1.2 Inventory all maintained consumers, workflows, package metadata, source verifiers, and historical-only origin references.
- [x] 1.3 Capture Forge main/worktree state and status of the three existing sibling checkouts, including their untracked files.
- [x] 1.4 Query and classify every allowlisted remote as archived, active, confirmed absent, or unavailable; keep auth/network failures unresolved.

## 2. DFS — Requirement-by-requirement retirement

- [x] 2.1 Run Workspace Governance, .NET and Rust import verification and prove each source-only history set remains reachable in Forge.
- [x] 2.2 Verify maintained Cargo, module Gate, CI, Governance and publisher consumers use only Forge-owned paths or declared external runtimes.
- [x] 2.3 Migrate active source/homepage/registry/provider/provenance/clone pointers to Forge while retaining historical release asset URLs required by existing installers.
- [x] 2.4 Re-query each active allowlisted origin immediately before action; archive only the exact confirmed active repository.
- [x] 2.5 Read back every attempted archive and record archived, already-archived, absent, or unresolved status separately.
- [x] 2.6 Preserve all sibling checkout statuses and untracked files; do not alter unrelated repositories or publish/push Forge.

## 3. BFS — Cross-surface regression and completeness

- [x] 3.1 Re-run import provenance checks and inspect source-only commit ancestry after the remote operations.
- [x] 3.2 Re-run maintained consumer checks and verify the archived origins are not required at build, test, Gate or publication time.
- [x] 3.3 Re-capture the three sibling worktree statuses and confirm user-owned files were not modified.
- [x] 3.4 Verify every allowlisted origin has a verified terminal state; unresolved state blocks retirement completion.
- [x] 3.5 Verify every out-of-scope repository received no operation and no source import or remote history was rewritten.
- [x] 3.6 Verify all active ownership, registry and provider-preset pointers target Forge and retained release URLs are only historical distribution endpoints.

## 4. Verification

- [x] 4.1 Run all import verifiers, affected consumer checks, changed-file format checks and applicable tests.
- [x] 4.2 Run name preflight, selected status/instructions, strict OpenSpec validation, and `git diff --check`.
- [x] 4.3 Run Forge Gate dry-run and bounded full Gate after source and remote state verification; record actual results.
- [x] 4.4 Archive without `--skip-specs`, promote canonical requirements, and verify the archive.
- [x] 4.5 Commit only related implementation, tests, archive and promoted specs; record delivery evidence in the required HANDOFF-only follow-up commit.
