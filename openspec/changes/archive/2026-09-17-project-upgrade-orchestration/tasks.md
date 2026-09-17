## 1. BFS — Baseline and impact coverage

- [ ] 1.1 Verify prerequisites (feature-lifecycle) and map "Deterministic upgrade planning", "Isolated fleet outcomes" to contracts, callers, persistence and tests.
- [ ] 1.2 Establish fixtures for each success, failure and boundary scenario; inspect affected surfaces (Upgrade, feature migrations, registry selection, migration state and later spec integration) and record actual build/test/integration commands before implementation.
- [ ] 1.3 Reconcile proposal, design and specs; resolve material compatibility decisions and label any structural-only work SKELETON_READY.

## 2. DFS — Requirement-by-requirement implementation

- [ ] 2.1 Implement "Deterministic upgrade planning" through the owning domain and all in-scope adapters/callers, proving its success, failure and boundary scenarios before advancing.
- [ ] 2.2 Implement "Isolated fleet outcomes" through the owning domain and all in-scope adapters/callers, proving its success, failure and boundary scenarios before advancing.

## 3. BFS — Cross-surface regression and completeness

- [ ] 3.1 Exercise interactions between "Deterministic upgrade planning" and "Isolated fleet outcomes" across all affected contracts, callers and persistent state; verify repeatability and compatibility with prerequisites.
- [ ] 3.2 Verify the change-specific risk: Database migrations may be irreversible; distinguish reversible file recovery from manual data recovery and require declared migration strategy. Recheck project isolation, validation, authorization and evidence reporting where affected; remove current-change placeholders.

## 4. Verification

- [ ] 4.1 Run the actual local formatting/build/test and relevant native-stack or adapter integration commands recorded in 1.2; record evidence and any exact failed command plus next action. Run configured local Gate if applicable; FAIL or unresolved REVIEW_REQUIRED blocks completion.
- [ ] 4.2 Run `node scripts/check-openspec-change-names.mjs`, `openspec validate project-upgrade-orchestration --strict --no-interactive` and `git diff --check`; review all original impact surfaces and staged related files before archive.
- [ ] 4.3 Archive only verified implementation without `--skip-specs`, inspect promoted canonical specs, commit related work, advance the HANDOFF pointer, commit handoff and stop without push.
