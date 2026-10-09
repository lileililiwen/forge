# Design: lifecycle-series rail (parts 1 + 2)

## 6. Part 2 decisions (as built)

- **Remediate preview is terminal-bound (chosen).**
  `remediatePlanCli(findingId) = forge remediate plan
  --finding <quoted>`; the copy note says to run it from the project
  directory. `--target` takes a directory (default `.`) which the
  browser never sends (path-free boundary), so it is omitted rather
  than invented. `remediate.plan` has no web execution row, hence no
  preview → confirm call — only the GitHub-pattern show + clipboard
  with select-the-text fallback. One `remediatePlanButton` builder
  serves both the doctor card (`renderHealth` blocking findings) and
  the status card (`renderProjectStatus` non-healthy checks).
  Alternative (deep-link a remediate web card) rejected: no such row
  exists and scope forbids server change.
- **Fleet shortcut is a deep link, not a fetch (chosen).**
  `fleetNeedsReconcile = conflict || freshness stale ||
  state unavailable || publish failed`; flagged rows append a
  `Refresh & reconcile` link to `/management?project=<id>`, reusing
  the existing tick-and-reconcile flow. Healthy rows keep exactly
  their Open/Manage control. Alternative (in-place refetch button)
  rejected: reconcile already has a view; a second writer risks
  divergence.
- **Hermora retry opens the card (chosen).** Shown only when a
  Hermora verb is recorded and not done. The button calls the
  existing `openActionCard("delivery.hermora-retry")` (typed
  `deployment_url` + `secret_ref` fields keep their preview →
  confirm discipline); the adjacent CLI is the `catalogInvocation`
  base + project id. Missing card degrades to a notice carrying the
  terminal string. Alternative (bespoke retry form) rejected: it
  would duplicate the card's validation.
- **Coexistence fix at the redirect (chosen).** The management
  registered-id redirect fires only when
  `viewForPath(pathname) === "management"` and targets
  `workbenchUrl(id)` (step-preserving). Rationale: discovery calls
  the matcher on every view; without the guard every workbench deep
  link lost its step ~2.5s after load. Found by the part-2
  Playwright oracle, not by inspection.
- **Roving + labels on the rail (chosen).** One Tab stop (the
  current step; `tabindex` re-synced on every re-render), delegated
  arrows/Home/End on the `<ol>` (survives re-render), `li`
  `aria-label="<Label>: <done|current step|upcoming>"` with the
  visible state word `aria-hidden` (no double announcement).
  Alternative (all-8 tabbable) rejected: 8 stops for onestrip is
  hostile; arrows are the expected pattern.
- **860px topbar + sticky headers + wrapping panel heads
  (chosen).** `@media(max-width:860px)` collapses the grid to one
  column with the sidebar as a topbar row (superset of the untouched
  850px tablet rule); `thead th` sticks at `top:0` with panel
  background over `--z-sticky`; `.panel-heading` wraps so the
  workbench tool cluster (selector + Open) no longer forces 425px
  page scroll at 390px (measured). Rail/nav/table scrollers keep
  their own internal overflow — the gate is page-level scrollWidth
  only. All additive; part-1 and slice rules untouched.
- **Error summaries: pin, don't rebuild (chosen).** All six
  containers already carry `tabindex="-1"` and the helper already
  focuses on every render; part 2 adds the contract test, no
  behavior change. Alternative (per-form focus code) rejected:
  the shared helper is already the single path.

## 1. Decisions

- **Derive, never fetch (chosen).** The rail, Next and CLI strings are
  pure functions of the four payloads the workbench already holds:
  `GET /v1/admin/projects/{id}` (manifest + health + operations),
  `.../status` (checks), `.../delivery/status` (phase + verbs +
  next), maintain (untouched). Alternative (new "series" endpoint)
  rejected: scope forbids server change; every fact needed is
  already on the wire.
- **8 fixed steps, first-incomplete-is-current (chosen).**
  `LIFECYCLE_RAIL_STEPS = idea/scaffold/spec/code/test/release/
  deploy/operate`. Evidence mapping: idea ← maturity declared;
  scaffold ← maturity ≥ L1; spec ← maturity ≥ L2 or journal kind
  contains `spec`; code ← features non-empty or journal contains
  `feature` or maturity ≥ L2; test ← health `healthy` or all status
  checks healthy; release ← any delivery verb done or phase past
  `draft` or maturity ≥ L3; deploy ← phase in
  `production/healthy/degraded/hermora-*` or maturity ≥ L4; operate ←
  phase `healthy` and maturity L4. Missing projections degrade to
  `todo`, never to invented `done`. Exactly one step carries
  `aria-current="step"` (first non-done; all-done → operate).
  Alternative (maturity-only mapping) rejected: the task requires
  doctor/status/delivery/journal evidence, not just the manifest.
- **`?step=` is a view hint, not state (chosen).** Rail links carry
  both params (`/workbench?project=<id>&step=<key>`); project
  switches preserve an existing valid step and step clicks preserve
  the project. Applying `?step=` scrolls to the mapped card and
  marks the rail item `data-linked`, but never moves `aria-current`
  (which tracks derived state, not the URL). Unknown step values are
  ignored. Alternative (step drives `aria-current`) rejected: URL
  must not falsify derived state for AT users.
- **Next is one ranked pick (chosen).** Priority: no maturity →
  declare maturity; health blocking → run doctor; no target → pick
  target; delivery `next.blocked` → unblock delivery; delivery
  `next.action` → mapped delivery action; maturity gap → plan
  upgrade; at-target + healthy → sustain/retire watch. Each pick
  carries step, title, reason, CLI (resolved from `catalogCommands`
  `cli_invocation` when the mapped command exists) and the catalog
  id to open. Recomputed after every projection resolves via a
  shared `workbenchEvidence` store. Alternative (per-card hints)
  rejected: the gap is the absence of a single answer.
- **Copy-as-CLI construction rule (chosen).** `buildCliString =
  cli_invocation + (scoped ? " <projectId>" : "") + payload flags`,
  where string → `--kebab-name <quoted>`, string_array → repeated
  `--kebab-name <v>` (matches the repeatable `--feature`/`--finding`
  CLI convention), boolean true → `--kebab-name`, and secret-ish
  params (`confirm`, tokens, `secret_ref`) render as `<token>` /
  `<redacted>` and are never copied literally (GitHub preview
  pattern). The string is shown in full in the card result box and
  copied via `navigator.clipboard` with a select-the-text fallback.
  Boundary (documented, part-2 work): per-command positional grammar
  (e.g. `feature add <FEATURE>`) is not reproduced; the string
  reproduces the card's typed payload against the catalog invocation.
- **One renderer owns all confirm cards (chosen).** The button is
  added once in `buildActionControl`'s `runWrap`, so workbench,
  management and maintain confirm actions all gain it with no
  bespoke wiring. Delivery-page digest-bound buttons are out of
  scope for part 1 (stated non-goal).

## 2. Implementation boundary

- **Files changed:**
  - `frontend/index.html` — rail `<ol id="lifecycle-rail">` shell +
    next-best-action card in the workbench lifecycle area.
  - `frontend/app.js` — `LIFECYCLE_RAIL_STEPS`,
    `workbenchEvidence` store, `railStepStates`,
    `renderLifecycleRail`, `workbenchStepParam` /
    `applyWorkbenchStepParam`, `computeNextBestAction` /
    `renderNextBestAction`, `buildCliString` + copy button in
    `buildActionControl`, evidence wiring in existing loaders.
  - `frontend/styles.css` — appended block only (rail layout,
    current-step marker, next card, copy button).
  - `tests/portal_ui_contract.rs` (new) — static token contract.
  - OpenSpec package (`proposal.md`, `design.md`, `tasks.md`,
    `specs/portal-web-ui/spec.md` delta).
- **Modules reused unchanged:** `request`/`requestStatus`, `el`,
  `setResultRole`, `renderErrorSummary`, catalog fetch + grouping,
  `navigateTo`/`renderRoute`, `scrollIntoViewRespectingMotion`,
  error-summary/focus helpers, GitHub preview builders.
- **Must NOT change:** catalog rows/routes, API handlers, portal
  sections, existing contract-test tokens (`wb-action-head`,
  `aria-expanded`, single-open behavior, `humanActionTitle`,
  `wb-action-cli`, `paramLabel`/`paramHint`, `lifecycleStageFor`,
  flat-manifest lifecycle read, `nextStep`), contrast/touch/motion
  tokens, `limit:1000` catalog query.

## 3. Language and runtime

Vanilla JS (ES2021, no build) + vanilla CSS in `frontend/`; Rust
`cargo test` for the static-token contract only. Verification:
`cargo test --test portal_ui_contract`, `cargo test --test
forge_web_command_catalog_contract`, `node scripts/check-
openspec-change-names.mjs`, `openspec validate --all --strict
--no-interactive`, `git diff --check`, `node --check
frontend/app.js`.

## 4. Failure modes

- Any projection fetch fails → rail steps from that family stay
  `todo`, Next falls back to manifest-only reasoning; nothing
  renders as done without evidence.
- Unknown `?step=` → ignored, rail still derives normally.
- Clipboard API absent/denied → full string stays visible and is
  selected for manual copy; no throw.
- Catalog not loaded → Next shows manifest/evidence reasoning
  without CLI; copy button still renders from the row id fallback.
- Secrets (`confirm*`, `*token*`, `secret_ref`) never enter the
  copied string literally.

## 5. Accessibility / responsive

Rail is a real `<ol>` with link steps; exactly one
`aria-current="step"`; step state also in text (`Done`/`Now`/
`Later` sr-only or visible suffix — color never the only signal).
Next card is a labelled section with `role="status"` result text.
Copy button is a native 44px `.button-quiet`; copied confirmation
announces via the card's existing live region. Rail scrolls
horizontally (`overflow-x:auto`) under 375px rules; no new motion.
