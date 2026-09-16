## 1. BFS — Baseline and impact coverage

- [x] 1.1 Verify prerequisites (none) and map "Versioned manifest contract", "Registry and CLI identity", "Independent command contract" to contracts, callers, persistence and tests.
- [x] 1.2 Establish fixtures for each success, failure and boundary scenario; inspect affected surfaces (Core, Registry, CLI; manifest schema and local registry; future adapters and transports) and record actual build/test/integration commands before implementation.
- [x] 1.3 Reconcile proposal, design and specs; resolve material compatibility decisions and label any structural-only work SKELETON_READY.

## 2. DFS — Requirement-by-requirement implementation

- [x] 2.1 Implement "Versioned manifest contract" through the owning domain and all in-scope adapters/callers, proving its success, failure and boundary scenarios before advancing.
- [x] 2.2 Implement "Registry and CLI identity" through the owning domain and all in-scope adapters/callers, proving its success, failure and boundary scenarios before advancing.
- [x] 2.3 Implement "Independent command contract" through the owning domain and all in-scope adapters/callers, proving its success, failure and boundary scenarios before advancing.

## 3. BFS — Cross-surface regression and completeness

- [x] 3.1 Exercise interactions between "Versioned manifest contract" and "Registry and CLI identity" and "Independent command contract" across all affected contracts, callers and persistent state; verify repeatability and compatibility with prerequisites.
- [x] 3.2 Verify the change-specific risk: Database and manifest writes cannot share one transaction; journal reconciliation and never report a partial write as successful. Unsupported schema versions must not be rewritten. Recheck project isolation, validation, authorization and evidence reporting where affected; remove current-change placeholders.

## 4. Verification

- [x] 4.1 Run the actual local formatting/build/test and relevant native-stack or adapter integration commands recorded in 1.2; record evidence and any exact failed command plus next action. Run configured local Gate if applicable; FAIL or unresolved REVIEW_REQUIRED blocks completion.
- [x] 4.2 Run `node scripts/check-openspec-change-names.mjs`, `openspec validate core-manifest-registry --strict --no-interactive` and `git diff --check`; review all original impact surfaces and staged related files before archive.
- [ ] 4.3 Archive only verified implementation without `--skip-specs`, inspect promoted canonical specs, commit related work, advance the HANDOFF pointer, commit handoff and stop without push.
