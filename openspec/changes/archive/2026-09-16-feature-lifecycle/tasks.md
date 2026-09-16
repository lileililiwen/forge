## 1. BFS — Baseline and impact coverage

- [x] 1.1 Verify prerequisites (doctor-maturity-assessment) and map "Feature resolution and discovery", "Safe lifecycle operations" to contracts, callers, persistence and tests.
- [x] 1.2 Establish fixtures for each success, failure and boundary scenario; inspect affected surfaces (Features registry, resolver, generator, CLI, manifests and adapter validation) and record actual build/test/integration commands before implementation.
- [x] 1.3 Reconcile proposal, design and specs; resolve material compatibility decisions and label any structural-only work SKELETON_READY.

## 2. DFS — Requirement-by-requirement implementation

- [x] 2.1 Implement "Feature resolution and discovery" through the owning domain and all in-scope adapters/callers, proving its success, failure and boundary scenarios before advancing.
- [x] 2.2 Implement "Safe lifecycle operations" through the owning domain and all in-scope adapters/callers, proving its success, failure and boundary scenarios before advancing.

## 3. BFS — Cross-surface regression and completeness

- [x] 3.1 Exercise interactions between "Feature resolution and discovery" and "Safe lifecycle operations" across all affected contracts, callers and persistent state; verify repeatability and compatibility with prerequisites.
- [x] 3.2 Verify the change-specific risk: Shared file edits and reverse dependencies make removal hazardous; preserve user edits and stop on ownership conflicts. Recheck project isolation, validation, authorization and evidence reporting where affected; remove current-change placeholders.

## 4. Verification

- [x] 4.1 Run the actual local formatting/build/test and relevant native-stack or adapter integration commands recorded in 1.2; record evidence and any exact failed command plus next action. Run configured local Gate if applicable; FAIL or unresolved REVIEW_REQUIRED blocks completion.
- [x] 4.2 Run `node scripts/check-openspec-change-names.mjs`, `openspec validate feature-lifecycle --strict --no-interactive` and `git diff --check`; review all original impact surfaces and staged related files before archive.
- [x] 4.3 Archive only verified implementation without `--skip-specs`, inspect promoted canonical specs, commit related work, advance the HANDOFF pointer, commit handoff and stop without push.
