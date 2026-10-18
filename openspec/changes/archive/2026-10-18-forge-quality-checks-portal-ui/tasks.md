# Tasks: make project quality evidence the primary portal view

## 1. BFS — Baseline and impact coverage

- [x] 1.1 Confirm the Gate status response wrapper, existing per-check data,
  portal section compatibility, frontend entry points and current CSS states.
- [x] 1.2 Map every `settings` section use, Gate renderer caller, Assurance
  registry/read action, and browser or static contract that pins them.
- [x] 1.3 Preserve the existing API and `settings` wire id; confirm no sibling
  project, generated template or provider execution behavior is in scope.

## 2. DFS — Requirement implementation

- [x] 2.1 Add failing portal projection tests for the Quality label/deep link,
  absent Gate evidence, and provider/path privacy.
- [x] 2.2 Implement the Gate-backed legacy Quality summary through Forge's
  existing persisted evidence and freshness helpers.
- [x] 2.3 Add failing workbench contracts for a dedicated Quality card, correct
  response-envelope normalization, honest state-specific next steps and only
  the existing rerun action.
- [x] 2.4 Implement the Quality card and normalize both status and run responses
  before rendering them.
- [x] 2.5 Add the Advanced diagnostics disclosure and place Assurance registry
  reads, including governance observations, inside it.
- [x] 2.6 Update the portal convergence mapping/docs while preserving the
  serialized `settings` id and API contract version.

## 3. BFS — Regression and completeness

- [x] 3.1 Verify status and rerun responses use one normalized renderer; check
  every freshness/aggregate next-step branch and absence/error handling in code,
  with API route contracts and the real Chromium absent-evidence flow.
- [x] 3.2 Verify optional provider reads stay explicit and hidden until the
  disclosure is expanded; no provider runs on page load.
- [x] 3.3 Verify keyboard disclosure, screen-reader status text, 390px reflow,
  evidence privacy, portal human/JSON parity and unchanged route/auth behavior.
- [x] 3.4 Review every BFS impact row and changed file; confirm no new endpoint,
  write, dependency, sibling source change or fabricated health state.

## 4. Verification

- [x] 4.1 Run focused Rust contracts, `node --check frontend/app.js`, the
  signed-in Chromium flow, OpenSpec name preflight, strict validation and
  `git diff --check`.
- [x] 4.2 Run Gate dry-run and bounded full Forge Gate; record actual verdict
  (dry-run resolved 8/8 required checks; full run timed out after 600 seconds in
  declared workspace verification, recorded in HANDOFF with rerun remediation).
- [x] 4.3 Archive without `--skip-specs`, promote canonical specs, commit
  implementation/tests/archive, update HANDOFF and pointer, commit HANDOFF
  alone, and stop without pushing.
