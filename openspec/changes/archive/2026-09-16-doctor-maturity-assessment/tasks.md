## 1. BFS — Baseline and impact coverage

- [x] 1.1 Verify prerequisites (deterministic-project-generation) and map "Doctor finding inventory", "Evidence-based maturity levels" to contracts, callers, persistence and tests.
- [x] 1.2 Establish fixtures for each success, failure and boundary scenario; inspect affected surfaces (Doctor, profile checks, CLI, registry health observations; maturity policy descriptors) and record actual build/test/integration commands before implementation.
- [x] 1.3 Reconcile proposal, design and specs; resolve material compatibility decisions and label any structural-only work SKELETON_READY.

## 2. DFS — Requirement-by-requirement implementation

- [x] 2.1 Implement "Doctor finding inventory" through the owning domain and all in-scope adapters/callers, proving its success, failure and boundary scenarios before advancing.
- [x] 2.2 Implement "Evidence-based maturity levels" through the owning domain and all in-scope adapters/callers, proving its success, failure and boundary scenarios before advancing.

## 3. BFS — Cross-surface regression and completeness

- [x] 3.1 Exercise interactions between "Doctor finding inventory" and "Evidence-based maturity levels" across all affected contracts, callers and persistent state; verify repeatability and compatibility with prerequisites.
- [x] 3.2 Verify the change-specific risk: Configured maturity is an intent, not proof; stale observations must be shown as stale. Recheck project isolation, validation, authorization and evidence reporting where affected; remove current-change placeholders.

## 4. Verification

- [x] 4.1 Run the actual local formatting/build/test and relevant native-stack or adapter integration commands recorded in 1.2; record evidence and any exact failed command plus next action. Run configured local Gate if applicable; FAIL or unresolved REVIEW_REQUIRED blocks completion.
- [x] 4.2 Run `node scripts/check-openspec-change-names.mjs`, `openspec validate doctor-maturity-assessment --strict --no-interactive` and `git diff --check`; review all original impact surfaces and staged related files before archive.
- [ ] 4.3 Archive only verified implementation without `--skip-specs`, inspect promoted canonical specs, commit related work, advance the HANDOFF pointer, commit handoff and stop without push.
