# Proposal: lifecycle-series rail (parts 1 + 2)

## Part 2 — observability shortcuts, mobile/a11y hardening, browser oracle

Part 1 (above, unchanged) left three gaps, closed here in the browser
only, sequentially after the part-1 files:

- **Observability.** A failed doctor/status row names its finding but
  offers no remediation path; a stale/failed fleet row leaves the
  operator on a dead Open/Manage state; an unfinished Hermora verb
  hides its retry in the release action group. Part 2 wires each to
  its existing surface: failed rows preview the exact
  `forge remediate plan --finding <id>` string (remediate.plan has no
  web row, so the preview is terminal-bound and `--target` stays out
  of the browser by the path-free boundary); stale/failed/conflict
  fleet rows gain a `Refresh & reconcile` deep link to the existing
  management reconcile flow; an unfinished Hermora verb gains an
  inline `Retry Hermora` control opening the existing
  `delivery.hermora-retry` card plus its exact CLI.
- **Mobile/a11y.** The 232px sidebar collapses to a topbar row under
  860px; table headers stay sticky inside their scroll regions; the
  rail `ol` gets roving focus (one Tab stop, arrows/Home/End) with a
  per-row text alternative (`<Label>: <done|current step|upcoming>`);
  panel headers wrap so tool clusters never force page-level scroll;
  error-summary focus on every form is pinned (all six containers
  carry `tabindex="-1"`; the shared helper moves focus on every
  render).
- **Coexistence fix.** Workspace discovery re-ran the management
  `?project=<registered>` redirect on every view, `replaceState`-ing
  workbench deep links down to `?project=` alone and dropping a valid
  `?step=`. The redirect now fires only on the management view and
  carries the step hint via `workbenchUrl`.
- **Browser oracle.** New `tests/browser/lifecycle-rail-check.mjs`
  (pinned playwright 1.63.0, no new frontend dep) driven by
  `tests/lifecycle_rail_browser.rs` (exit 2 → UNVERIFIED): 8 steps in
  order, single `aria-current`, single Tab stop, step/project
  coexistence across load/click/reload, bogus-step tolerance, a
  single Next, arrow roving, clipboard-equals-shown copy string,
  390px no page overflow, workbench contrast AA.

## BFS Impact Map (parts 1 + 2)

## Why

The workbench shows a project as five disconnected read-only cards
(Lifecycle definition list, Health/doctor, Project status, Delivery,
Journal evidence) plus a grouped wall of collapsed action buttons. An
operator opening a project cannot answer "where in Idea → Operate is
this project, what is the single next move, and what exact CLI
reproduces the action I am about to confirm" without reading every
card and every catalog summary. This change closes that UX gap in the
browser only: a series rail derived from projections the workbench
already fetches, one computed next-best-action, and a copyable exact
CLI string on every confirm action.

## What Changes

- Workbench lifecycle card gains `ol#lifecycle-rail` with the 8 series
  steps Idea / Scaffold / Spec / Code / Test / Release / Deploy /
  Operate. Step states derive from existing projections only
  (manifest maturity/target, doctor health, status checks, delivery
  phase + verbs + next, journal operation kinds); the first
  incomplete step carries `aria-current="step"`. Each step links to
  `/workbench?project=<id>&step=<key>`; `?step=` coexists with
  `?project=` and survives project switches and reload.
- Workbench gains a next-best-action card rendering the single Next
  computed from maturity/target/evidence with reason text, the exact
  CLI, and a button opening the matching action card when one exists.
- Every catalog-driven confirm action (`buildActionControl` cards in
  workbench, management and maintain views) gains a Copy-as-CLI
  button rendering the exact `forge ...` string built from the
  catalog `cli_invocation` + project positional + the card's own
  typed payload, reusing the GitHub preview pattern (validate, show
  full string, copy via clipboard with text fallback, secrets
  redacted as `<token>`/`<redacted>`).
- New `tests/portal_ui_contract.rs` asserts the static tokens
  (rail, `aria-current`, step param, next-action, copy-CLI builder).
  No new frontend dependency; no API/CLI/portal change.

## BFS Impact Map

- **Capabilities:** `portal-web-ui` (workbench views only).
- **Users / flows:** browser operators opening one managed project
  (`/workbench?project=<id>`), deep-linking a series step
  (`&step=<key>`), confirming catalog actions.
- **Contracts / data / persistence:** none. No endpoint, catalog row,
  portal section, registry write or journal shape change. The rail,
  Next and CLI strings are pure client-side derivations of payloads
  the workbench already fetches.
- **Integrations:** none (no provider probe, no new fetch).
- **Tests:** new `tests/portal_ui_contract.rs`; existing
  `forge_web_command_catalog_contract` assertions preserved
  (collapsed headers, `humanActionTitle`, `wb-action-cli`,
  `paramLabel`/`paramHint`, `lifecycleStageFor`, flat manifest read).
- **Compatibility:** rail/Next/copy controls are additive DOM inside
  existing cards; unknown `?step=` values are ignored; clipboard
  absence degrades to selecting the shown string.

## Capabilities

- Series rail with 8 derived steps, single `aria-current`, step deep links.
- Single computed next-best-action with reason and exact CLI.
- Copy-as-CLI on every confirm action with secret redaction.

## Non-goals

- No per-command positional-vs-flag CLI grammar beyond the documented
  construction rule (part 2 may promote a server-blessed invocation).
- No delivery-page bespoke confirm buttons (digest-bound allowlist /
  approve / publish / reconcile stay as-is in part 1).
- No server-side "next action" endpoint; no portal section change.
- No new frontend framework, dependency, or build step.
