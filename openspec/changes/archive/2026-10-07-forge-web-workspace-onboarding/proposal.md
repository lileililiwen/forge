# Proposal: Onboard workspace sibling projects from the browser

## Why

The operator maintains an expanding set of sibling project directories, but
the portal fleet shows only the locally registered projects. The shipped
browser management routes (`new`, `import`, `register`) require the operator
to already know each exact kebab-case name, act one project at a time, and
have a server-side project root configured — which is commonly unset, with
no documented setup step. There is no discovery step, no bulk flow, and no
way to address directories whose leaf names are not valid project ids. The
result is a portal that describes project management instead of performing
it. Nothing in this change may depend on any concrete host folder: the
workspace root is strictly runtime operator configuration, and the mechanism
must keep working as siblings are added, renamed or removed.

## What Changes

- Add a read-only, session-gated discovery route,
  `GET /v1/admin/workspace/candidates`, that re-reads the operator-configured
  project root on every request — so newly added siblings appear with no code,
  list or configuration change — and returns a bounded, path-free candidate
  view per directory: leaf name, derived id, manifest presence, suggested
  profile/confidence, registration state and next action. No concrete host
  folder is assumed anywhere in code or specs; the root is runtime-only
  operator configuration.
- Add a confirm- and digest-bound bulk route,
  `POST /v1/admin/workspace/onboard`, that previews a selected candidate set
  and, on confirmation with the matching digest, imports or registers each
  candidate through the same in-process Core functions the CLI runs,
  reporting honest per-item success/failure.
- Accept one validated single-segment `directory` leaf plus optional typed
  `id`/`profile` overrides per item, so non-kebab leaves can be onboarded to
  explicit kebab ids without the browser ever naming a path.
- Add a task-oriented “Workspace onboarding” panel to the dashboard that
  discovers, reviews, selects, previews, confirms and reports onboarding —
  reusing generic preview/confirm controls and text-only rendering.
- Document the `FORGE_ADMIN_PROJECTS_ROOT` prerequisite in the README portal
  section so a normal operator can enable browser project management.

## Package Boundary and Split Assessment

This package has one independently verifiable outcome: a signed-in operator
can discover every sibling directory under the configured root and onboard a
selected set through preview → confirm → per-item results, ending with the
new projects visible in the fleet. Discovery, selection, preview, confirm,
apply and results share one lifecycle, one owner (`src/api` + `src/import` +
`src/registry` + `frontend/`), one journal contract and one browser oracle;
splitting read from write would ship an unusable half. A broader dashboard
reorganization around task-oriented command execution is a real but separable
concern and becomes the follow-on package.

| Package | Single outcome | Owner / language | Boundary / contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `forge-web-workspace-onboarding` (**this**) | Discover sibling directories and bulk import/register a selected set from the browser through preview → confirm → per-item results | Forge `src/api` + `src/import` + `src/registry` / Rust, `frontend/` | `GET …/workspace/candidates`, `POST …/workspace/onboard`; `inspect_import` / `adopt_import` / `Registry::register` reused unchanged | `forge-web-project-management` (single-item gate shape) | `tests/forge_web_workspace_onboarding_contract.rs` + Playwright onboarding drive |
| `forge-web-command-workflows` (follow-on) | Reorganize the dashboard around runnable lifecycle workflows and demote the command catalog to reference | Forge `frontend/` + `src/api/command_catalog.rs` | Navigation/task ordering, no new mutations | this package (full project set must exist first) | browser task-flow + accessibility checks |

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path / symbol | Reusable code / contract | Compatibility gap | Owner / release boundary | Decision |
|---|---|---|---|---|---|
| Single-item management gate | `src/api/admin.rs` `management_descriptor` / `management_destination` / `management_preview` / `management_write`, `ADMIN_PROJECTS_ROOT_ENV` | Preview → confirm → digest, root confinement, path scrubbing | Single `project` name only; leaf must already be a valid id; no discovery, no bulk | `src/api/admin.rs` owns the gate | **extend shared owner** — add discovery + bulk routes to the same gate |
| Import Core | `src/import/mod.rs` `inspect_import` / `adopt_import` / `derive_project_id` | Read-only detection, manifest adoption, kebab derivation | Called per directory by CLI; needs a bounded server-side driver | `src/import` owns detection/adoption | **extend shared owner** — drive existing functions per candidate, change nothing in them |
| Registration Core | `src/registry/mod.rs` `register` / `check_identity_available` | Manifest validation, id/path collision rules | Per directory; unchanged | `src/registry` owns persistence | **reuse unchanged** |
| Workspace registry document | `src/fleet/mod.rs` `observe`, `/home/paul/code/workspace-governance/projects.json` (81 entries) | Read-only fleet observation | Entry paths resolve inside the document's own directory, so it cannot represent siblings of that directory; observation never onboards | Workspace Governance owns the document; `src/fleet` owns observation | **keep local** — do not repurpose observation as onboarding; discovery reads the configured management root directly |
| Portable inventory | `src/publish/inventory.rs` `resolve_source` | Explicit inventory files | Optional, operator-supplied, observed-only; not a discovery mechanism | Provider-owned documents | **keep local** — onboarding never scans an inventory as a directory source |

No sibling checkout is modified; every decision is an in-repo extension of an
existing owner.

## User Experience and Interface Impact

Actor: the single authenticated global admin, whose goal is “make all my
workspace projects manageable in the portal.” Entry point: a new “Workspace
onboarding” panel in the dashboard management area, visible after sign-in.
Primary flow: Discover → review candidate table → tick a subset → Preview
selection → review per-item plan and digest → Confirm → watch per-item results
→ new rows appear in the fleet. Alternate flows: single-item register/import
controls remain; unconfigured root shows a prerequisite notice naming the
exact variable and README step instead of a dead button.

Content priority: candidate directory leaf, derived id, manifest/profile
signal, registration state, next action — then selection, then confirmation,
then results. Responsive: the candidate table lives in the existing
scrollable, keyboard-navigable table region pattern. Accessibility: labelled
checkboxes and inputs, a live-region status line, visible focus via the
existing stylesheet hooks, text-only rendering, measured contrast on new text.
Browser evidence: a Playwright drive against a throwaway root performs
discover → select → preview → confirm → fleet-appears, plus keyboard and
contrast checks. `UI/UX: N/A` does not apply — this change exists for the
human operator.

## BFS Impact Map

- **Capabilities:** `forge-web-workspace-onboarding` (new). No catalog
  disposition changes: the new routes are web-only workflows with no CLI
  row; they join `IMPLEMENTED_WEB_ROUTES` without altering the 227-row count.
- **Users / flows:** global admin onboarding 0..N sibling directories per
  confirmed batch.
- **Contracts / data / persistence:** no schema change. Per-item `register`
  and `admin.project.import` rows as today, plus one
  `admin.workspace.onboard` parent row carrying counts only.
- **Integrations / configuration:** reads `FORGE_ADMIN_PROJECTS_ROOT`
  (unchanged semantics); the browser never supplies a path, only validated
  leaves and typed overrides.
- **Callers:** `src/api/mod.rs` router + admin lists; `src/api/admin.rs`
  discovery/onboard facades; `src/api/command_catalog.rs` allowlist;
  `frontend/app.js` + `frontend/index.html` onboarding panel;
  `README.md` prerequisite docs.
- **Failure / boundary behavior:** unconfigured root → 409 naming the
  variable; traversal/non-segment leaves → 400 no echo; ambiguous/undecidable
  candidates → preview-blocked with reason; per-item failures → 207 with
  per-item codes, never a blanket success.
- **Tests:** new API contract + Playwright drive; existing management,
  catalog, import and registry suites stay green.
- **Privacy / security:** leaf names and derived ids only; no absolute path,
  manifest content, credential or secret crosses the API.

## Capabilities

- `forge-web-workspace-onboarding`: discover and bulk-onboard sibling
  directories from the browser.

## Non-goals

- No dashboard-wide navigation reorganization (follow-on package).
- No remote/Mac-server onboarding; only the server-local configured root.
- No recursive scanning below immediate children; no symlink escapes from
  the root.
- No bulk `new` (creation stays single, deliberate).
- No cross-request bulk idempotency key; per-item journal rows make retries
  observable and safe.
- No automatic onboarding without explicit selection + confirmation.
- No change to CLI import/register, bearer routes, journal schema or
  `API_CONTRACT_VERSION`.
