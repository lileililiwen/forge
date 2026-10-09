# Proposal: Portal feedback and session recovery

## Why

A UI/UX audit of `frontend/` (vanilla HTML/CSS/JS, no framework) measured four
defects on current `main`:

- **Action results are silent to assistive technology.** Every workbench,
  delivery, workspace-bulk and scoped-management preview/run/refuse result is
  written into a plain `<div>` with no live-region role. A screen-reader
  operator who activates "Run confirmed action" or "Publish approved preview"
  hears nothing whether the action succeeded or was refused —
  `frontend/index.html:134,135,190,195,214,220,329`;
  `frontend/app.js:1342-1409,1818,1863,2122,2206,2362,2961,3287`.
- **A mid-session 401 is unrecoverable.** The session is only checked on boot
  (`frontend/app.js:3338-3345`). Once the Forge-wide session expires or is
  revoked, every subsequent request throws the generic
  "Forge could not complete the request." (`frontend/app.js:6-11`) and the
  operator has no path back to sign-in short of manually reloading.
- **Essential text renders below the 12px floor.** Error text, digests, CLI
  hints and evidence chips are set at 10px and metadata/hints at 11px
  (`frontend/styles.css:14,16,65,80`), below the accessible body-text floor.
- **Disclosure buttons are not programmatically associated with their panel.**
  `buildActionControl` sets `aria-expanded` (`frontend/app.js:2274,2499`) but
  no `aria-controls`, so assistive technology cannot tie the button to the
  region it discloses.

## What Changes

- `frontend/index.html`: the seven `.wb-plan-result` containers gain
  `role="status"` so they are live regions before any script runs.
- `frontend/app.js`:
  - one shared `setResultRole(box, isError)` helper sets
    `role`/`aria-live` (`status`/`polite` for results, `alert`/`assertive` for
    refusals) and is wired into every result writer (workbench plan/apply,
    delivery action/lookup, workspace bulk preview/run, scoped management
    preview/run, and the per-action `showPreview`/`showError`/`showSuccess`),
    reusing the exact rendering operators already see — only the announcement
    role is added;
  - `request` and `requestStatus` detect a `401` on a non-login page, redirect
    to `login.html?next=<current path+query>` via a null-safe
    `sessionExpiredRedirect()`, and never fire that redirect from the login
    page itself (so a wrong-password `401` still renders the inline error);
  - `buildActionControl` gives each `.wb-action-body` a unique id and sets
    `aria-controls` on its `.wb-action-head`.
- `frontend/styles.css`: one appended slice-7 override block raises error,
  hint, digest, CLI-hint, evidence-chip, findings, workflow-reason, sources and
  metadata text to the 12px floor (`--text-md`), leaving uppercase micro-labels
  at their existing sizes.
- No API/registry/journal/CLI/catalog change. Deep-link, focus-on-route-change,
  contrast, touch and motion behaviors are unchanged.

## BFS Impact Map

- **Capabilities:** `portal-feedback-session-recovery` (new, change-scoped).
- **Users / flows:** screen-reader operators running or being refused any
  portal action; any operator whose session expires mid-task; operators reading
  error, digest and metadata text; keyboard/AT operators opening workbench
  action panels.
- **Contracts / data / persistence:** none. No JSON API, registry, journal,
  schema, catalog row/count, or CLI change; request payloads and endpoints are
  unchanged.
- **Integrations / configuration:** none. `scripts/web.sh` staging picks up
  `frontend/` unchanged.
- **Callers:** every `request`/`requestStatus` call site inherits the 401
  recovery; the result writers listed above gain the role helper; the login
  POST keeps its own non-redirecting 401 path because `page === "login"`.
- **Failure / boundary:** the redirect is skipped when `page === "login"` and
  is null-safe; result role is reset on every write so a reused box never keeps
  the previous urgency; a missing container is null-guarded; the 12px rules sit
  in one appended block so earlier cascade wins and prior tokens are untouched.
- **Tests:** existing `forge_web_navigation_contract`,
  `forge_web_command_catalog_contract`, `forge_web_workbench_deep_link_browser`
  and `web_login_credentials_contract` must stay green; the new tokens are
  verified by a static contract test over the shipped files (markup-only claims
  are not reported as browser verification).
- **Privacy / security:** the `next` value is the same-origin path already used
  by the login round-trip and is validated by `loginNextTarget`; no credential,
  token or hash enters the URL or any notice.

## Capabilities

- `portal-feedback-session-recovery`: every portal action result is a live
  region (polite for success, assertive for refusal); an expired mid-session
  credential returns the operator to sign-in with the current deep link instead
  of a dead error; essential portal text respects a 12px floor; and workbench
  disclosure buttons name the panel they control.

## Non-goals

- No light theme, no new module, no framework or build step, no visual
  redesign beyond the scoped live regions, redirect and 12px floor.
- No change to server-side 401 semantics, session storage, the static allowlist
  or the `next` allowlist.
- No new browser harness; rendered-behavior verification beyond the existing
  browser oracles stays future work.
