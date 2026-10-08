# Proposal: Portal form error feedback (slice 3)

## Why

Prior UI/UX audit of `frontend/` (vanilla HTML/CSS/JS, no framework) found
six gap slices; slices 1–2 (`portal-focus-route-contrast`,
`portal-touch-responsive-targets`) are delivered and archived. This change
implements ONLY slice 3 (form labels and error feedback). Slices 4–6 stay
QUEUED and are not authored as active changes by this package.

Measured defects in slice 3 scope (current `main`):

- The fleet filter row (`frontend/index.html:68-73`) and the
  delivery/portfolio inputs (`index.html:289-320` plus the portfolio
  metadata controls at `237-247`) use the placeholder-only + `sr-only`
  pattern: a sighted operator sees only a grey placeholder that vanishes
  on typing, and there is no helper text explaining what values each
  field accepts.
- Failed submissions have no error summary: portfolio, delivery,
  workbench action cards, workspace onboarding, and login each render a
  single flat notice (or nothing field-scoped), so an operator must hunt
  for which field failed. Inline errors are not wired with
  `aria-describedby`, and `fleet-filter-error` uses `role="status"`,
  which never interrupts — an error live region must be `role="alert"`.
- `login.html` has no password show/hide toggle, no visible required
  indicators (only the `required` attribute), while keeping
  `autocomplete`/password-manager support intact.

## What Changes

- `frontend/index.html`: fleet filter labels gain visible text
  (stacked label-above-input, layout and 44px targets unchanged) plus one
  shared visible hint for the row; delivery/portfolio text inputs and
  selects are relabelled from `sr-only` to visible labels with per-field
  helper text; each validated field gets an inline error node and an
  error-summary container per form area; `fleet-filter-error`
  `role="status"` becomes `role="alert"`.
- `frontend/app.js`: one shared `renderErrorSummary` helper (heading +
  per-field links, moves focus, retains inline errors) plus
  `setFieldError`/`clearFieldError` (`aria-invalid`, `aria-describedby`
  management) wired into login, portfolio, delivery, workbench action
  cards (`buildActionControl`), and workspace/management onboarding
  preview/run validation and server-refusal paths. Payload gathering is
  otherwise byte-identical: validated single-segment names/typed fields
  only, no new endpoints.
- `frontend/login.html` + `app.js`: password show/hide toggle
  (`type=button`, `aria-pressed`, `aria-controls`, Show/Hide label),
  visible `*` required indicators with a legend line; `name`,
  `autocomplete="username"`/`"current-password"`, and paste behavior
  untouched (no `onpaste` prevention exists or is added).
- `frontend/styles.css`: one appended slice-3 override block (visible
  labels, hints, inline errors, error summary, invalid outline, password
  toggle layout); earlier rules untouched, 44px minima re-asserted for
  the relabelled controls.
- No API/registry/journal/CLI/catalog change. Deep-link,
  focus-on-route-change, contrast, and touch behaviors are unchanged.

## BFS Impact Map

- **Capabilities:** `portal-form-error-feedback` (new, slice-scoped).
- **Users / flows:** sighted keyboard, screen-reader, and
  password-manager operators filling fleet filters, portfolio metadata,
  delivery actions, workbench/management forms, and login; operators
  recovering from validation or server refusals.
- **Contracts / data / persistence:** none. No JSON API, registry,
  journal, schema, catalog row/count, or CLI change; payloads unchanged.
- **Integrations / configuration:** none. `scripts/web.sh` staging picks
  up `frontend/` unchanged.
- **Callers:** `gatherPayload`, portfolio/delivery/management submit
  handlers, login submit, `showFleetFilterError` route through the new
  helpers; `renderRoute`, `navigateTo`, `popstate`, param applies,
  `loginNextTarget`, focus guard, scroll helper untouched.
- **Failure / boundary:** summary focus is null-guarded; links target
  existing field ids; `aria-describedby` restores its base value on
  clear; server-refusal summaries retain the existing result rendering;
  toggle degrades to a plain password field without JS.
- **Tests:** existing `forge_web_navigation_contract` (token assertions
  retained), `forge_web_manage_deep_link_browser`,
  `forge_web_workbench_deep_link_browser` must stay green; label/summary/
  toggle verified by static token assertions over the shipped files (no
  new harness; markup-only claims are not reported as browser
  verification).
- **Privacy / security:** no credential, path, or hash material enters
  the URL or notices; `next` guard untouched; password value never copied
  to the toggle label; autocomplete tokens unchanged.

## Capabilities

- `portal-form-error-feedback`: every scoped form field carries a visible
  label (plus helper text where the field needs explanation); every
  failed submission shows a focusable error summary with per-field links
  alongside retained inline errors wired via `aria-describedby`; the
  fleet filter error interrupts as `role="alert"`; login offers a
  password show/hide toggle and visible required indicators while
  password-manager support stays intact.

## Non-goals

- Slices 4–6 (breakpoints, sidebar overflow, state preservation, empty
  route; SVG icons, typography, motion tokens; data-table
  virtualize/sort/export/skeleton) — separate future changes.
- No light theme, no new modules, no framework or build step, no visual
  redesign beyond the scoped labels/errors/toggle tokens.
- No workbench health latency or project-retire work (HANDOFF defects 1–2
  stand).
