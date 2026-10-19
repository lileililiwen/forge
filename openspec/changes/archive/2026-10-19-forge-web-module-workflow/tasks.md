## 1. BFS — Baseline and impact coverage

- [x] 1.1 Map CLI module discovery, Gate invocation/evidence, admin route/auth, command catalog, frontend navigation, and browser test harness.
- [x] 1.2 Define shared module catalog contract and preserve current CLI human/JSON/NDJSON output and exit semantics.
- [x] 1.3 Define API response/error envelope, root resolution, path redaction, authorization, persistence and concurrent-run policy.
- [x] 1.4 Define Modules page journeys, responsive/accessibility states and browser evidence matrix.

## 2. DFS — Requirement-by-requirement implementation

- [x] 2.1 Extract validated module discovery and ID resolution into a Forge library module; route the CLI facade through it.
- [x] 2.2 Add admin-session-gated module inventory and Gate status/run routes using the shared resolver and existing Gate evidence engine.
- [x] 2.3 Add bounded same-module execution guard, Registry journaling, relative-path projection, error redaction and typed failure mapping.
- [x] 2.4 Add module API route descriptors and map CLI module commands into the web command catalog without breaking catalog exhaustiveness.
- [x] 2.5 Add the Modules page, navigation, module list/detail, Fast/Full selection, run action, Gate check rendering and accessible/responsive states.

## 3. BFS — Cross-surface regression and completeness

- [x] 3.1 Verify CLI/API resolver parity, Gate evidence parity, admin auth, path containment, errors, concurrency, and existing project Gate routes.
- [x] 3.2 Verify module command catalog mapping and browser flows for populated, empty, unavailable, unauthorized, PASS, FAIL, running and recoverable error states.
- [x] 3.3 Verify keyboard interaction, announcements, visible focus, narrow viewport reflow and no path leakage in API/UI.

## 4. Verification

- [x] 4.1 Run focused Core/API/catalog/browser contracts, changed-file Rust formatting, JavaScript syntax checks and `scripts/check-tests.sh`.
- [x] 4.2 Run `node scripts/check-openspec-change-names.mjs`, selected status/instructions, strict validation and `git diff --check`. The active package passes strict validation; repository-wide strict validation currently reports 116/124 with eight pre-existing canonical specs failing SHALL/MUST wording.
- [x] 4.3 Run `forge gate --dry-run`, then bounded full `forge gate`; record the actual verdict. Dry-run resolved eight required checks with none unbound. Full Gate passed all eight checks at `2026-10-19T11:36:21Z`; source-size evaluated 433/433 candidates under the 1,000-line limit.
- [x] 4.4 Archive without `--skip-specs`, promote canonical requirements, and verify the archived package. The archive is `2026-10-19-forge-web-module-workflow`; the promoted canonical spec passes strict validation and `openspec list` reports no active changes.
- [x] 4.5 Commit related implementation/tests/archive/specs, then update HANDOFF and make the HANDOFF-only commit.
