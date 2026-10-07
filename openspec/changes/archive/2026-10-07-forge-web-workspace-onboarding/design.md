# Design: Dynamic workspace onboarding

## 1. Implementation boundary

- **Repository / project:** this Forge repository, the `forge` crate. No
  sibling project is touched, and no concrete host folder appears in code,
  specs or tests — all filesystem locations in tests are throwaway tempdirs.
- **Modules changed:**
  - `src/api/mod.rs` — two `Route` variants, two router arms, admin
    short-circuit, permission, dispatch and unreachable-list entries.
  - `src/api/admin.rs` — two route consts, a candidate resolver
    (`workspace_root` → validated leaf → canonical descendant), a bounded
    live directory reader, per-candidate preview builders reusing
    `management_preview`, and `workspace_candidates` /
    `workspace_onboard_write` plus per-item apply helpers reusing
    `run_management_import` / `run_management_register`.
  - `src/api/command_catalog.rs` — the two new routes join
    `IMPLEMENTED_WEB_ROUTES` only. No row changes: the routes are web-only
    workflows with no CLI row, so the 227-row count is untouched.
  - `frontend/app.js` + `frontend/index.html` — a “Workspace onboarding”
    panel in the management area: discover/refresh, candidate table with
    selection, per-item profile/id overrides, preview-selected, confirm,
    per-item results, fleet refresh.
  - `tests/forge_web_command_catalog_contract.rs` — allowlist gains the two
    routes (no `web_ids` change: no catalog row points at them).
  - `README.md` — document the `FORGE_ADMIN_PROJECTS_ROOT` prerequisite and
    the onboarding flow in the portal section.
- **Modules reused unchanged:** `src/import` (`inspect_import`,
  `adopt_import`, `derive_project_id`), `src/registry` (`register`,
  `check_identity_available`, `record_operation`), the single-item
  management gate (`projects_root`, `authoring_digest`, `guarded`,
  `is_json`, `scrub_json`), `Registry::open`.
- **Must NOT change:** CLI import/register dispatch, bearer routes, journal
  schema, `API_CONTRACT_VERSION`, catalog rows/count, fleet aggregation
  (`src/api/fleet.rs` keeps reading the registry/database only), delivery
  or publish surfaces.

## 2. Language and runtime

- Rust 2021, toolchain floor `rustc 1.87` (per `Cargo.toml` `rust-version`).
- Build: `cargo build`. Test: `cargo test` (focused targets below).
  Format: `cargo fmt`.
- Frontend: plain ES modules under `frontend/`, served by `forge web serve`;
  no bundler, no new dependency.
- Browser oracle: pinned Playwright/Chromium under `tests/browser`;
  `UNVERIFIED` when unavailable, never a pass.
- Target platform: Linux loopback server; the operator configures the root
  at runtime via `FORGE_ADMIN_PROJECTS_ROOT`. Tests use tempdirs only.

## 3. Ownership and shared code

- `src/api/admin.rs` owns the session-gated confirm→digest discipline; the
  workspace routes join the management gate rather than opening a parallel
  mechanism. Descriptors are path-free; digests use `authoring_digest`.
- `src/import` owns detection/adoption semantics; the admin layer drives
  `inspect_import` / `adopt_import` per candidate and changes nothing in them.
- `src/registry` owns persistence; `register` / `check_identity_available`
  stay the single gates, and one `admin.workspace.onboard` parent row is
  appended with `record_operation` carrying counts only.
- The fleet stays database-driven: onboarding writes registry records, and
  `src/api/fleet.rs` reads those records. Discovery output never feeds the
  fleet directly.
- No shared `/lib` or sibling extraction; project-local extension of
  existing owners only.

## 4. User experience and interface

Actor: the authenticated global admin, whose goal is “make every sibling in
my workspace manageable here, including ones added later.” Entry point: a
“Workspace onboarding” panel in the dashboard management area, after sign-in.
Primary flow: Discover (or Refresh) → review candidate table → tick a subset
(or all) → Preview selection → review per-item plan and digest → Confirm →
read per-item results → fleet shows the new rows. Alternate flows: the
existing single-item new/import/register controls stay; an unconfigured root
shows a prerequisite notice naming `FORGE_ADMIN_PROJECTS_ROOT` and the README
step; ambiguous candidates show their blocker and stay unselected.

Content priority: leaf name, derived id, manifest/profile signal,
registration state, next action — then selection, then confirmation, then
results. The candidate table reuses the scrollable keyboard-navigable table
region pattern; checkboxes and override inputs are labelled; status uses the
existing live region; all rendering is `textContent`-only. Responsive and
focus behavior reuse the existing stylesheet hooks; new text meets the
measured-contrast check in the browser oracle.

## 5. Behavioral model

The root is re-read from the environment on every request: new, renamed or
removed sibling directories change the next discovery response with no code,
list or configuration change. Pagination is deterministic (sorted leaf
order, `limit` default 50/max 100, `cursor` offset); pages are a viewing
convenience only — confirmation binds the exact selected set.

**Candidate states** (computed live per directory): `unregistered`,
`registered`, `id-collision`, `path-collision`, `ambiguous` (needs explicit
profile), `undecidable` (no recognizable stack and no manifest),
`manifest-invalid`, `unreadable`. Only `unregistered` candidates are
selectable; the rest carry a safe reason.

**`GET /v1/admin/workspace/candidates?limit=&cursor=`** (read-only):
1. `guarded` session gate → 401.
2. `projects_root()` → 409 `admin-prerequisite` naming the variable.
3. Read immediate child directories (bounded, sorted, paginated); skip
   hidden entries and non-directories; resolve each leaf strictly
   server-side.
4. Return 200 with `{ candidates: [...], total, cursor, limit,
   contract }`. No provider call, no write, no absolute path.

**`POST /v1/admin/workspace/onboard`** (bulk preview → confirm → apply):
1. `is_json` → 415; `guarded` → 401.
2. Parse `items`: 1..=25 entries of `{ directory, id?, profile? }`.
   `directory` must be one validated single path segment (no separator,
   traversal, NUL, control, `.`/`..`); anything else is a static 400 with
   no echo. Unknown keys are ignored.
3. Resolve each leaf under the canonical root; symlink escapes refused.
4. Build the canonical descriptor `{ action: "workspace-onboard",
   root: <canonical root>, items: [sorted {directory, id?, profile?,
   action, revision-hint?}] }`; digest = `authoring_digest(descriptor)`.
   The root participates in the digest as a hash input only — it is never
   serialized.
5. No `confirm` → 200 with per-item previews (action, derived id, profile,
   confidence, blockers) + digest; **no write**.
6. Mismatched digest → 409 `admin-digest-mismatch` + fresh preview; **no
   write**. A changed directory set therefore refuses safely.
7. Matching `confirm: true` → apply items in order through
   `adopt_import` (no manifest) or `Registry::register` (manifest present),
   each under the shared operation journal; collect per-item
   `{directory, id, ok, code?, message?}` with scrubbed messages.
8. All ok → 202; any failure → 207 with the same shape; append one
   `admin.workspace.onboard` parent row with counts only. Partial results
   are reported as partial — never a blanket success.

## 6. Contract and compatibility

- `ROUTE_ADMIN_WORKSPACE_CANDIDATES =
  "GET /v1/admin/workspace/candidates"`,
  `ROUTE_ADMIN_WORKSPACE_ONBOARD =
  "POST /v1/admin/workspace/onboard"`.
- Discovery query: `limit` (default 50, clamp 1..=100), `cursor` (default
  0, non-negative integer); malformed values fall back to defaults, never
  400, because they only shape a read.
- Onboard body: `{ items: [{directory, id?, profile?}], confirm?,
  plan_digest? }`. Only these keys are read.
- Both routes join `IMPLEMENTED_WEB_ROUTES`; no catalog row points at them,
  so row count and `web_ids` are unchanged.
- Response shapes are additive; CLI output and existing responses unchanged.
- Validation ownership: session/JSON/leaf/digest gates in `admin.rs`;
  detection/adoption/registration semantics stay in `src/import` and
  `src/registry`.

## 7. Failure and boundary policy

| Case | Behavior |
|---|---|
| No / invalid session | 401, no filesystem read |
| Non-JSON onboard body | 415 |
| Root unset/blank/non-directory | 409 `admin-prerequisite`, names the variable, never the value |
| Leaf with separator/traversal/NUL/`.`/`..` | 400, no echo, no read |
| Symlink escaping the root | refused before any read |
| Empty selection or > 25 items | 400 naming the bound |
| Ambiguous/undecidable/manifest-invalid candidate | preview-blocked with reason; excluded from apply |
| No confirm | 200 preview + digest, no write |
| Digest mismatch (set changed) | 409 + fresh preview, no write |
| Per-item Core failure | 207 entry with typed code/message, journaled, others continue |
| Absolute path/secret in any response | scrubbed before return |

## 8. Verification oracle

- **New `tests/forge_web_workspace_onboarding_contract.rs`** against a
  throwaway root: anonymous → 401; non-JSON → 415; unset root → 409 naming
  the variable; hostile leaf → 400 no echo; discovery lists manifest,
  importable, ambiguous and garbage dirs with correct states and no
  absolute path; preview writes nothing; mismatch writes nothing;
  confirmed onboard imports+registers the selectable set (202) and reports
  per-item results; a failing item yields 207 with honest entries; a new
  directory added mid-test appears on re-discovery (dynamic proof).
- **New Playwright drive** (`tests/browser/workspace-onboarding-check.mjs`
  + `tests/forge_web_workspace_onboarding_browser.rs`): sign in, open the
  onboarding panel, discover, select, preview, confirm, see results, see the
  new rows in the fleet; keyboard + contrast + no-path-rendered checks.
- **Existing suites stay green:** management, catalog (allowlist only),
  execution, workbench, admin API, fleet, import/register unit tests,
  `--bin forge`, `--lib api::`.
- A task box is checked only with the command output for its assertion.

## 9. Decision ledger

- **Resolved:** discovery is live per request (dynamic for an expanding
  workspace) rather than a stored inventory; the fleet remains
  database-driven and never consumes discovery output.
- **Resolved:** one bulk route with per-item results instead of N single
  calls, because 70+ siblings make per-item round trips unusable; the
  25-item cap bounds request time and the digest binds the exact set.
- **Resolved:** `directory` is a validated single segment joined to the
  runtime-configured root — never a browser path — with canonical
  descendant checks per item.
- **Resolved:** partial success is 207 with per-item codes; retries are safe
  because `register`/import are identity-checked and journaled per item.
- **Deferred to `forge-web-command-workflows`:** dashboard-wide navigation
  and command-execution reorganization.
- **Blockers:** none.
