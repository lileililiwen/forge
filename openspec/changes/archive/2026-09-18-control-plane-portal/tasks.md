## 1. BFS — Baseline and impact coverage

- [x] 1.1 Verify prerequisites (core-http-api, external-planes-analytics, semantic-ui-patterns) and map "Shared project and lifecycle views", "Accessible controlled operations" to contracts, callers, persistence and tests.
- [x] 1.2 Establish fixtures for each success, failure and boundary scenario; inspect affected surfaces (Portal navigation, API client, accessible operation views and project status) and record actual build/test/integration commands before implementation.
- [x] 1.3 Reconcile proposal, design and specs; resolve material compatibility decisions and label any structural-only work SKELETON_READY.

## 2. DFS — Requirement-by-requirement implementation

- [x] 2.1 Implement "Shared project and lifecycle views" through the owning domain and all in-scope adapters/callers, proving its success, failure and boundary scenarios before advancing.
- [x] 2.2 Implement "Accessible controlled operations" through the owning domain and all in-scope adapters/callers, proving its success, failure and boundary scenarios before advancing.

## 3. BFS — Cross-surface regression and completeness

- [x] 3.1 Exercise interactions between "Shared project and lifecycle views" and "Accessible controlled operations" across all affected contracts, callers and persistent state; verify repeatability and compatibility with prerequisites.
- [x] 3.2 Verify the change-specific risk: A dashboard can imply stronger health than evidence supports; display unknown/stale and partial results prominently. Recheck project isolation, validation, authorization and evidence reporting where affected; remove current-change placeholders.

## 4. Verification

- [x] 4.1 Run the actual local formatting/build/test and relevant native-stack or adapter integration commands recorded in 1.2; record evidence and any exact failed command plus next action. Run configured local Gate if applicable; FAIL or unresolved REVIEW_REQUIRED blocks completion.
- [x] 4.2 Run `node scripts/check-openspec-change-names.mjs`, `openspec validate control-plane-portal --strict --no-interactive` and `git diff --check`; review all original impact surfaces and staged related files before archive.
- [ ] 4.3 Archive only verified implementation without `--skip-specs`, inspect promoted canonical specs, commit related work, advance the HANDOFF pointer, commit handoff and stop without push.
