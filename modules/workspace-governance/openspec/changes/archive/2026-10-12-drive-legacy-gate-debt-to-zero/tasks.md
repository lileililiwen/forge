# Tasks: Drive legacy gate debt down with verified remedies

## 1. BFS — Baseline and impact coverage

- [x] Measure the existing debt driver before specifying behavior: 24 entries, A=10, B=7, C=3, D=2, E=1, F=1, `?`=0.
- [x] Inspect the current three `App.slnx` files; each already contains all project paths, so preserve and verify them.
- [x] Map governance driver/checker ownership, Driftwatchdog debt input, binder derivation, product launcher write boundary, and excluded profile/scaffold/ratchet changes in `proposal.md`.
- [x] Add tests for classifier families, dry-run, malformed debt input, classes A-D, decisions/unknown, binder build derivation, idempotency, and dirty-manifest reporting.
- [x] Confirm proposal, design, spec, test boundary and measured baseline align.
- **Done when** the mapped requirements have named implementation files, failure cases, and exact local test oracles.

## 2. DFS — Requirement-by-requirement implementation

### DFS-1 — Classify and escalate

- [x] Add `tests/test_close_gate_debt.py` covering A-F and `?`, malformed entries, dry-run no-write, and unknown reason no-write behavior.
- [x] Run the focused tests and observe expected failures for missing behavior.
- [x] Fix classifier, malformed-input validation, dirty-manifest state reporting, and escalation output.
- [x] Rerun focused tests; the original 24-entry baseline classified A-F with no `?`. The refreshed record now contains five entries, including three owner/working-tree decisions and two dirty HestiaLab bindings.
- **Done when** all classifier cases, including a new unknown negative control, are pinned.

### DFS-2 — Make remedies safe and idempotent

- [x] Add temp-directory tests for default dry-run no-write, existing checker launcher preservation, build-system/frontend addition, and `pip-audit` declaration repeat.
- [x] Add `.slnx` and `.sln` fixtures; the remedy adds only missing project paths and verifies complete membership.
- [x] Include the false-control `.slnx` with zero projects and assert it is rejected.
- [x] Pin the exit-zero `.NET` error message case; the remedy checks output as well as status.
- [x] Rerun remedy tests, including a second apply to prove idempotency.
- **Done when** each A-D outcome is either verified complete or explicitly remains unresolved with its reason.

### DFS-3 — Derive a real Python build command

- [x] Add binder tests for a Python package with `[build-system]` and declared `build`, and controls with missing build metadata/tooling and unrelated `build` configuration words.
- [x] Measure all three original class-C cases. Ariadex already declares its build frontend; Alethefy has backend metadata but not the frontend; jenkins-local has no package metadata.
- [x] Implement the narrow derivation `python -m build` only when parsed package metadata and the build frontend are declared.
- [x] Update remedy C to add the frontend for an existing package and to escalate when the repository has no package metadata.
- **Done when** binder output and an invocation against a temporary package agree, and the missing-tool control remains unresolved.

### DFS-4 — Validate central concern checkers

- [x] Add focused tests for positive/clean controls across all five concerns and run the launchers in the affected repositories.
- [x] Spot-check privacy and accessibility against source context; privacy passed for the sampled repositories, accessibility found only the Mewo `lang` issue in applicable HTML.
- [x] Confirm observed findings include a relative path and line and use exit 1; clean/not-applicable runs exit 0.
- [x] Confirm product launchers invoke the canonical checker using argv, without shell interpolation.
- **Done when** real fleet findings are surfaced as findings and no false clean is introduced.

## 3. BFS — Cross-surface regression and completeness

- [x] Run the driver dry-run against the original 24 entries and classify all as A-F, `?`=0, SKIP=0. The post-bind record has five items and no silently repaired dirty manifest.
- [x] Apply the remaining mechanical remedy for Alethefy; preserve decisions and report unavailable audit tooling/network results honestly.
- [x] Run every affected generated checker launcher; platform-contracts fixtures and schemas both report zero findings.
- [x] Run the binder with `--root /home/paul/code --apply`, then regenerate debt with `--debt-only --debt-out /home/paul/code/workspace-governance/gate-debt.json`.
- [x] Confirm the three `.slnx` files are populated; the class-B unit tests also verify project paths for both solution formats.
- [x] Surface findings: Mewo `apps/mewo_mobile/web/index.html:1` lacks `lang`; Actoria and trailCrew lack `THIRD-PARTY-NOTICES`; sampled privacy checks found no findings. No prose was changed.
- [x] Keep `sisyphusfy` build/security and `jenkins-local` package-build as decisions; HestiaLab's dirty manifest leaves build/security unbound and recorded.
- [x] Check the staged path list against pre-existing changes; exclude the unrelated `driftwatch.toml`.
- **Done when** every modified surface has an observable check and every remaining issue is named with evidence.

## 4. Verification

- [x] `python3 -m unittest tests.test_close_gate_debt tests.test_bind_gate_commands -q` — 160 passed.
- [x] `python3 -m unittest discover -s tests -q` — 838 passed, 0 failed.
- [x] `openspec validate --changes --strict` — 11 passed, 0 failed.
- [x] `python3 scripts/workspace_check.py --self-check` — 0 errors, 0 warnings.
- [x] `python3 scripts/auto_gate.py --self-check --no-deployment` — PASS, 0 errors, 0 warnings.
- [x] `node scripts/check-openspec-change-names.mjs` — valid.
- [x] `git diff --check` — passed.
- [x] Record exact pass/fail counts and remaining debt/findings in `HANDOFF.md`.
- **Commit protocol:** Commit 1 contains only the explicit spec, implementation, tests, debt snapshot, and `HANDOFF.md`; commit 2 contains only `HANDOFF.md`. Do not archive without user authorization.
- **Done when** each command has a recorded observed result and the two commits contain only their prescribed paths.
