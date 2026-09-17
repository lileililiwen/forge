## 1. BFS — Baseline and impact coverage

- [ ] 1.1 Verify prerequisites (feature-lifecycle, project-upgrade-orchestration) and map "Semantic reusable unit contracts", "Quality classification and selection" to contracts, callers, persistence and tests.
- [ ] 1.2 Establish fixtures for each success, failure and boundary scenario; inspect affected surfaces (Components, capability contracts, catalog provenance and resolver ranking) and record actual build/test/integration commands before implementation.
- [ ] 1.3 Reconcile proposal, design and specs; resolve material compatibility decisions and label any structural-only work SKELETON_READY.

## 2. DFS — Requirement-by-requirement implementation

- [ ] 2.1 Implement "Semantic reusable unit contracts" through the owning domain and all in-scope adapters/callers, proving its success, failure and boundary scenarios before advancing.
- [ ] 2.2 Implement "Quality classification and selection" through the owning domain and all in-scope adapters/callers, proving its success, failure and boundary scenarios before advancing.

## 3. BFS — Cross-surface regression and completeness

- [ ] 3.1 Exercise interactions between "Semantic reusable unit contracts" and "Quality classification and selection" across all affected contracts, callers and persistent state; verify repeatability and compatibility with prerequisites.
- [ ] 3.2 Verify the change-specific risk: Certification can become stale or conflict with version constraints; compatibility and explicit policy precede quality preference. Recheck project isolation, validation, authorization and evidence reporting where affected; remove current-change placeholders.

## 4. Verification

- [ ] 4.1 Run the actual local formatting/build/test and relevant native-stack or adapter integration commands recorded in 1.2; record evidence and any exact failed command plus next action. Run configured local Gate if applicable; FAIL or unresolved REVIEW_REQUIRED blocks completion.
- [ ] 4.2 Run `node scripts/check-openspec-change-names.mjs`, `openspec validate semantic-component-registry --strict --no-interactive` and `git diff --check`; review all original impact surfaces and staged related files before archive.
- [ ] 4.3 Archive only verified implementation without `--skip-specs`, inspect promoted canonical specs, commit related work, advance the HANDOFF pointer, commit handoff and stop without push.
