# Tasks: lifecycle-series rail (parts 1 + 2)

## Phase 5 — part 2 implementation (sequentially after part 1)

- [x] 5.1 `frontend/app.js` observability: `remediatePlanCli` +
  `remediatePlanButton` (exact `forge remediate plan --finding`
  show + clipboard, `--target` omitted by the path-free boundary)
  wired into `renderHealth` blocking rows and `renderProjectStatus`
  non-healthy rows; `fleetNeedsReconcile` + `reconcileShortcut`
  (`Refresh & reconcile` → `/management?project=`) appended in
  `renderProjects`; inline `Retry Hermora` + exact CLI in
  `renderProjectDelivery` opening `delivery.hermora-retry` via
  `openActionCard`.
- [x] 5.2 `frontend/app.js` coexistence fix: the management
  registered-id redirect fires only on the management view and
  targets step-preserving `workbenchUrl(id)` (found by the oracle:
  discovery dropped `?step=` ~2.5s after load).
- [x] 5.3 `frontend/app.js` rail a11y: per-row `aria-label` text
  alternative, single-Tab-stop roving (`initRailRoving`,
  arrows/Home/End, `data-rail-key`, tabindex re-sync on re-render).
- [x] 5.4 `frontend/styles.css` (appended block only): 860px
  sidebar→topbar collapse, sticky `thead th`, wrapping
  `.panel-heading`, finding-row/reconcile/preview styles.
- [x] 5.5 `tests/portal_ui_contract.rs` +6 static tests (remediate
  preview, reconcile shortcut, Hermora retry, 860px + sticky,
  roving + labels, every summary focusable).
- [x] 5.6 Playwright oracle (pinned 1.63.0, no new dep):
  `tests/browser/lifecycle-rail-check.mjs` +
  `tests/lifecycle_rail_browser.rs` (exit 2 → UNVERIFIED).

## Phase 6 — part 2 verification

- [x] 6.1 `cargo test --test portal_ui_contract` (13/13) +
  `cargo test --test lifecycle_rail_browser` (VERIFIED, 8 notes).
- [x] 6.2 `node scripts/check-openspec-change-names.mjs` PASS;
  `openspec validate --all --strict --no-interactive` PASS;
  `git diff --check` clean; `node --check` on both JS files.
- [x] 6.3 `forge gate --dry-run` rehearsal + bounded
  `forge gate --timeout-secs 500`, verdict recorded in HANDOFF.

## Phase 7 — closeout

- [x] 7.1 `openspec archive lifecycle-series-rail --yes` WITHOUT
  `--skip-specs` (promote specs); exactly 2 commits on top of
  `ec512c4` (implementation+specs+tests+UI+playwright, then HANDOFF
  docs); never push; explicit paths only.

## Phase 1 — BFS baseline

- [x] 1.1 Preflight PASS (`node scripts/check-openspec-change-names.mjs`); `openspec new change lifecycle-series-rail`; confirm `main@ec512c4` clean.
- [x] 1.2 Map touch points: workbench loaders (`loadWorkbenchDetail`, `loadProjectStatus`, `loadProjectDelivery`, `renderOperations`), catalog `cli_invocation` + exec params, GitHub preview builders, existing contract tokens to preserve.
- [x] 1.3 Set `current_spec: lifecycle-series-rail` in HANDOFF.

## Phase 2 — DFS requirement implementation

- [x] 2.1 `frontend/index.html`: rail `<ol id="lifecycle-rail">` shell + next-best-action card (`wb-next-title`, `wb-next-body`, `wb-next-cli`) in the workbench lifecycle area.
- [x] 2.2 `frontend/app.js` rail: `LIFECYCLE_RAIL_STEPS`, `workbenchEvidence`, `railStepStates`, `renderLifecycleRail` (`aria-current="step"`, `?project=`+`?step=` links), `workbenchStepParam`/`applyWorkbenchStepParam` (scroll + preserve), wired into existing loaders.
- [x] 2.3 `frontend/app.js` Next: `computeNextBestAction` (ranked pick from maturity/target/evidence) + `renderNextBestAction` (title, reason, CLI, open-matching-action), recomputed on every projection resolve.
- [x] 2.4 `frontend/app.js` Copy-as-CLI: `buildCliString` (construction rule + quoting + secret redaction) + Copy button in `buildActionControl` `runWrap` (clipboard + text fallback, full string shown).
- [x] 2.5 `frontend/styles.css`: appended block only (rail, current marker, next card, copy button).
- [x] 2.6 `tests/portal_ui_contract.rs` (new): static tokens for rail/aria-current/step-link/Next/copy builder + `node --check` syntax gate.

## Phase 3 — BFS regression/completeness

- [x] 3.1 `cargo test --test portal_ui_contract` green.
- [x] 3.2 `cargo test --test forge_web_command_catalog_contract` green (no existing token broken).
- [x] 3.3 No new endpoint/route/catalog-row/portal-section; no new frontend dependency; secrets never copied literally; unknown `?step=` ignored.

## Phase 4 — Verification

- [x] 4.1 `cargo test --test portal_ui_contract` + `cargo test --test forge_web_command_catalog_contract`.
- [x] 4.2 `node scripts/check-openspec-change-names.mjs` PASS; `openspec validate --all --strict --no-interactive` PASS; `git diff --check` clean.
- [x] 4.3 Working tree left dirty for subagent 2: DO NOT commit, DO NOT archive.
