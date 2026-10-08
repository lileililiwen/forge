# Design: Portal form error feedback

## 1. Decisions

- **Visible stacked labels, same wrappers (chosen).** Fleet
  `.fleet-filter` labels switch from `sr-only` to a visible label span
  and become `flex-direction: column` (label above the 44px input); the
  row keeps its wrap layout. Delivery/portfolio `filter-box`/`search-box`
  wrappers on relabelled controls become `label.field` (column stack:
  visible label, control, hint, inline error) so the visible text never
  crowds the control inline. Alternative (inline visible text inside the
  existing row wrappers) rejected: label and control collide at 320px.
- **One shared summary helper (chosen).**
  `renderErrorSummary(container, heading, items)` builds
  `<h3>` + link list from `[{fieldId, label, message}]`, unhides the
  pre-declared `.error-summary` container (`tabindex="-1"`), and focuses
  it; empty items hide the container. `setFieldError(field, errorId,
  message)` sets the inline node, `aria-invalid="true"`, and appends the
  error id to `aria-describedby`; `clearFieldError` restores the base.
  Alternative (per-form bespoke markup) rejected: five surfaces need the
  identical pattern; one helper keeps heading/link/focus behavior
  uniform.
- **Inline errors retained, summaries added (chosen).** Server-refusal
  result rendering (`wb-plan-result`, delivery action result, notices)
  stays exactly where operators already look; the summary is the
  focus/movement target and the inline node is the field anchor. Nothing
  is removed, so existing operator habits keep working.
- **`role="alert"` for the fleet filter error (chosen).** A failed
  predicate read leaves a full table showing while claiming a filter —
  that is an error, not a status update, so it must interrupt. Single
  row-level error: no multi-field summary applies there.
- **Native toggle button (chosen).** `<button type="button">` with
  `aria-pressed` + `aria-controls="login-password"`, text Show/Hide,
  flips `type=password/text`. No custom checkbox, no value copying, no
  `maxlength`/`onpaste` changes — password managers and paste keep
  working because nothing about the input's identity changes.
- **Appended slice-3 CSS block (chosen).** Same convention as slice 2:
  one commented block at the end of `frontend/styles.css`,
  equal-specificity later rules, no minified-rule rewrites, 44px minima
  re-asserted for relabelled controls.

## 2. Implementation boundary

- **Repository / project:** this Forge repository, vendored `frontend/`
  only. No sibling touched.
- **Files changed:**
  - `frontend/index.html` — visible labels + hints + inline error nodes
    + summary containers (fleet row, portfolio metadata, delivery
    actions); `fleet-filter-error` role fix; inputs wired with
    `aria-describedby` to hint ids.
  - `frontend/login.html` — error summary container, password toggle
    button, required indicators + legend.
  - `frontend/app.js` — `renderErrorSummary`/`setFieldError`/
    `clearFieldError` helpers + wiring in login submit, portfolio
    add-tag/record-review, delivery set/approve/publish/reconcile/
    lookup, `buildActionControl` (`gatherPayload` + `showError`),
    workspace bulk and management scoped preview/run.
  - `frontend/styles.css` — appended slice-3 block only.
  - OpenSpec package (`proposal.md`, `design.md`, `tasks.md`,
    `specs/portal-web-ui/spec.md` delta).
- **Modules reused unchanged:** `request`/`requestStatus` payloads and
  endpoints, `loginNextTarget`, `renderRoute`/`lastRouteView` focus
  guard, scroll helper, deep-link applies, `next` guard, slice-1/2
  tokens.
- **Must NOT change:** JSON API, auth/session server behavior, registry
  or journal, `command_catalog` rows, static-server allowlist, typed
  payload shapes, slices 4–6 surfaces.

## 3. Language and runtime

Vanilla JS (ES2021, no build step) in `frontend/app.js`, served by
`forge web serve` and staged by `scripts/web.sh` symlinks. Vanilla CSS
in `frontend/styles.css` (no new custom properties; existing tokens
only). Verification: Rust integration oracles
(`cargo test --test <name>`) plus static token assertions over the
shipped files; no new browser harness in this slice.

## 4. User experience and interface

Actor: any operator filling a dashboard form or signing in, including
screen-reader users.

| Action | Before | After |
|---|---|---|
| Read a fleet filter or delivery/portfolio field | Grey placeholder only, vanishes on typing; no guidance | Persistent visible label above the control; helper text names accepted values |
| Submit portfolio/delivery/workbench/onboarding/login with a missing or refused value | One flat notice; field must be hunted | Focus moves to an error summary (heading + links); each field keeps its inline error wired via `aria-describedby` |
| Fleet catalog predicate read fails | `role="status"` notice may never be announced | `role="alert"` interrupts |
| Sign in | Password unreadable; required-ness invisible | Show/Hide toggle; `*` markers + legend; managers/paste unaffected |

## 5. Behavioral model

- Labels: every scoped input/select is wrapped by (or `for`-linked to) a
  visible label; placeholders stay as examples, never as the only name.
  Helper text sits in `.field-hint` with a stable id referenced by the
  control's `aria-describedby` from first render.
- Summary: on a failed submission the handler clears prior errors,
  calls `setFieldError` per failing field, calls `renderErrorSummary`
  with the same items, and focus lands on the summary container
  (`tabindex="-1"`). Activating a summary link moves focus to the field.
  A later successful submit (or input edit where wired) clears summary
  and inline nodes and restores base `aria-describedby`.
- Toggle: click flips `login-password` between `password`/`text`,
  swaps button text Show/Hide, and flips `aria-pressed`; focus stays on
  the toggle; the input value, `name`, and `autocomplete` never change.
- Payloads: `gatherPayload` returns the same object shape; delivery/
  portfolio/onboarding handlers send the same field names to the same
  routes; no new endpoint is called.

## 6. Contract and compatibility

No wire, storage, or CLI contract changes. Browser-only deltas: visible
labels/hints, summary + inline error nodes, alert role, toggle and
required markers. Backward compatible: all existing `app.js` token
assertions (`renderRoute`, `VIEW_BY_PATH`, `loginNextTarget`,
`mgmt-project-preview`, …) remain literally present; bare `/workbench`,
`/management`, `login.html`, and the `next` allowlist behave as before;
slice-1/2 rendering unchanged.

## 7. Failure and boundary policy

| Case | Behavior |
|---|---|
| Summary container missing (foreign shell) | Helper null-guards and returns; inline errors still set |
| Summary link target missing | Link omitted for that item; heading + remaining links still render |
| Browser without `:has` or JS disabled | Labels/hints/summaries render as static text; toggle absent without JS but the password field works |
| Forced-colors / high-contrast | Invalid outline uses `outline`, never color alone; summary links are real anchors |
| Screen reader on `aria-describedby` | Base hint always described; error id appended only while the error shows, removed on clear |
| Password managers | `name`, `autocomplete`, form structure unchanged; toggle is outside the credential fields |

## 8. Verification oracle

- `cargo fmt --check`, `cargo build` clean (no Rust touched; confirms no
  collateral).
- `cargo test --test forge_web_navigation_contract`,
  `--test forge_web_manage_deep_link_browser`,
  `--test forge_web_workbench_deep_link_browser` green (tokens + behavior
  intact).
- Static assertions by inspection over shipped files: no `sr-only`-only
  label remains on the scoped inputs; every scoped control has a visible
  label + `aria-describedby` hint; every form area owns an
  `.error-summary[tabindex="-1"]`; `role="status"` no longer on
  `fleet-filter-error`; helpers (`renderErrorSummary`,
  `setFieldError`, password toggle) present and wired at every listed
  call site; `autocomplete` values and input `name`s unchanged; no new
  fetch route.
- `node scripts/check-openspec-change-names.mjs` PASS;
  `openspec validate --all --strict --no-interactive` 0 failures;
  `git diff --check` clean.
- `forge gate --dry-run` then `forge gate --timeout-secs 600`; verdict
  recorded in HANDOFF with attribution.

## 9. Decision ledger

- **Resolved:** stacked visible labels over inline label text (no
  crowding at 320px).
- **Resolved:** shared summary helper over per-form markup (uniform
  heading/link/focus behavior).
- **Resolved:** summaries added alongside retained inline/result
  rendering (no operator habit broken).
- **Resolved:** native toggle button over custom control (manager/paste
  safe).
- **Resolved:** no new browser harness; browser automation stays the
  slices-4–6 / full-audit concern, not this slice's.
- **Deferred (non-goals):** slices 4–6; light theme; icon/typography/
  motion tokens; unscoped `sr-only` sites (global search, source filter,
  workbench selects, table headers) stay as found.
- **Blockers:** none.
