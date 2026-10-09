# Design: Portal feedback and session recovery

## 1. Decisions

- **One explicit role helper (chosen).** `setResultRole(box, isError)` sets
  `role="status"` + `aria-live="polite"` for results and `role="alert"` +
  `aria-live="assertive"` for refusals. It is called on every result write, so
  a box reused for a later success never keeps an earlier refusal's urgency.
  Alternative (annotate the static containers only) rejected: the containers
  are reused for both success and refusal, so a static role cannot express the
  difference an operator actually needs.
- **Roles set in both HTML and JS (chosen).** The seven static
  `.wb-plan-result` divs carry `role="status"` in markup so they are live
  regions from first paint; JS then flips the role per outcome. Alternative
  (JS-only) rejected: an empty live region that only becomes live after the
  first write can be missed by some AT.
- **401 recovery inside the request helpers (chosen).** `request` and
  `requestStatus` are the single choke points every admin call passes through.
  On `status === 401` and `page !== "login"` they call
  `sessionExpiredRedirect()`, which reuses the existing login round-trip
  (`login.html?next=`) that already validates the return path. Alternative
  (per-call-site handling) rejected: fifteen-plus callers, easy to miss one.
- **Login page excluded from the redirect (chosen).** A wrong-password login
  returns `401`; the login page must keep rendering its inline error and
  summary, so the guard is `page !== "login"`.
- **Appended slice-7 CSS block (chosen).** Same convention as slices 2/3/5: one
  commented block at the end of `frontend/styles.css`, later equal-specificity
  rules win, no earlier rule is rewritten.

## 2. Implementation boundary

- **Repository / project:** this Forge repository, vendored `frontend/` only.
  No sibling touched.
- **Files changed:**
  - `frontend/index.html` — `role="status"` on the seven `.wb-plan-result`
    containers.
  - `frontend/app.js` — `setResultRole` helper + wiring in `renderPlan`,
    `renderApplyError`, `renderApplySuccess`, `mgmtScopedPreview`,
    `mgmtScopedRun`, `wsPreview`, `wsRun`, `renderDeliveryActionResult`,
    `deliveryLookup`, and the per-action `showPreview`/`showError`/
    `showSuccess`; `sessionExpiredRedirect()` + `401` handling in `request`
    and `requestStatus`; `actionBodySeq` + `aria-controls` in
    `buildActionControl`.
  - `frontend/styles.css` — appended slice-7 block only.
  - OpenSpec package (`proposal.md`, `design.md`, `tasks.md`,
    `specs/portal-web-ui/spec.md` delta).
- **Modules reused unchanged:** every result writer's existing markup,
  `renderErrorSummary`/`setFieldError`/`clearFieldError`, `loginNextTarget`,
  `renderRoute`/`lastRouteView` focus guard, deep-link applies, slice-1/2/3/5
  tokens.
- **Must NOT change:** JSON API, auth/session server behavior, registry or
  journal, static-server allowlist, typed payload shapes, contrast tokens,
  touch targets, motion tokens.

## 3. Language and runtime

Vanilla JS (ES2021, no build step) in `frontend/app.js`, served by
`forge web serve` and staged by `scripts/web.sh`. Vanilla CSS in
`frontend/styles.css` (existing tokens only; no new custom properties).
Verification: a Rust static contract test over the shipped files plus the
existing `forge_web_*` regression suites.

## 4. User experience and interface

Actor: any operator running or being refused a portal action, and any operator
whose session ends mid-task.

| Action | Before | After |
|---|---|---|
| Run or get refused an action with a screen reader | Silent `<div>`; nothing announced | Success announced politely, refusal announced assertively |
| Session expires while the page is open | Generic "could not complete" with no recovery | Return to sign-in carrying the current deep link |
| Read an error, digest, hint or evidence chip | 10–11px text | At least 12px |
| Open a workbench action panel with AT | Button and panel unassociated | `aria-controls` ties the button to its panel |

## 5. Behavioral model

- Result: each writer calls `setResultRole(box, isError)` immediately before
  replacing its children; `hidden` is then cleared so the update is announced.
  `isError` is true for refusal branches (`renderApplyError`, the `!ok` paths,
  `showError`, a delivery result carrying an error) and false for previews,
  successes and lookups.
- Session: `request` throws after scheduling the redirect so a caller cannot
  continue rendering on a dead session; `requestStatus` returns the status
  unchanged (its callers already render an honest refused state) after
  scheduling the redirect.
- Type: the slice-7 selectors raise only content-bearing text; uppercase
  micro-labels (`th`, `.eyebrow`, `.field-label`, `.side-label`, badges) keep
  their sizes.

## 6. Contract and compatibility

No wire, storage, or CLI contract change. Browser-only deltas: live-region
roles, a 401 redirect, larger essential text, and one ARIA association. All
existing `app.js` token assertions (`setResultRole` is additive;
`history.pushState`, `VIEW_BY_PATH`, `loginNextTarget`, `renderErrorSummary`,
`other.querySelector(".wb-action-body").hidden`, `wb-action-cli`) remain
literally present; bare `/workbench`, `/management`, `login.html` and the
`next` allowlist behave as before.

## 7. Failure and boundary policy

| Case | Behavior |
|---|---|
| Result container missing (foreign shell) | `setResultRole` null-guards and returns; inline errors still render |
| Wrong-password login `401` | No redirect (`page === "login"`); inline error + summary render as before |
| Session-expiry `401` on the dashboard | Redirect to `login.html?next=<here>`; return path validated by `loginNextTarget` |
| Reused result box after a refusal | Role reset to `status` on the next success so urgency never leaks |
| Browser without JS disabled | Static `role="status"` containers remain valid empty live regions |
| Reduced motion / high contrast | No motion added; roles and text size carry no color-only meaning |

## 8. Verification oracle

- `cargo fmt --check`, `cargo build` clean (no Rust touched; confirms no
  collateral).
- New contract test `cargo test --test portal_feedback_session_contract`
  asserting the shipped tokens (roles, helper, 401 recovery, 12px selectors,
  `aria-controls`) and the surviving prior-slice tokens.
- Regression: `cargo test --test forge_web_navigation_contract`,
  `--test forge_web_command_catalog_contract`, and the deep-link browser
  suites stay green.
- Local Gate verdict recorded in HANDOFF before archive.
