# Proposal: Let the portal execute the project's own Core lifecycle actions

## Why

requirement.md §36 says "The portal should consume the same Forge Core APIs as CLI
and MCP," and §35 lists the intended `POST /projects/{id}/features` and
`POST /projects/{id}/specs` routes. The delivered
`forge-web-command-execution` change made exactly two authoring commands —
`feature add` and `spec generate` — browser-executable behind a session-gated
preview→confirm→digest route. The user reported the portal still does not match
the requirement: the command center is a *list* that mostly says "run it in a
terminal," and the few web rows are hard-wired one at a time, so nothing about
the inventory is generically "doable." Two commands out of a 239-command surface
reads as decorative.

The honest gap is not breadth for its own sake — ROADMAP line 91 and
AGENTS.md forbid a generic-shell "run anything" button. The gap is structural:
the catalog carries no information the browser can act on (no route, no typed
parameter schema, no confirmation requirement per row), so the frontend cannot
render a runnable control from the inventory, and every newly-executable command
requires bespoke frontend + route wiring. This change closes that structural gap
and, through it, makes the next real batch of commands — the project's own
feature/spec lifecycle *writes* — executable: `feature remove`, `feature
upgrade`, `spec apply`. Each delegates to the same in-process Core handler the
CLI runs (`remove_feature`, `upgrade_feature`, `apply_routing`), keyed only by a
validated project id, behind the same preview→confirm→digest gate already
proven for `feature add`.

The user selected the **"Project actions (Core only)"** scope: no subprocess
(`build`/`test`), no provider (`deploy`/`publish`), no PTY (`agent`), no
hidden-input or transport commands. This package stays entirely inside that
boundary.

## What Changes

- **Self-describing catalog rows.** An executable (`web`) catalog row gains a
  structured `execution` block: the exact admin `route`, HTTP `method`, an
  ordered list of typed `parameters` (name, scalar kind, required flag, closed
  option set where applicable), whether a `confirm` flag is required, and the
  project-scope `risk` label. Non-executable rows carry no `execution` block.
  This makes the catalog the machine-readable contract the browser renders from,
  so a command is "in the list" and "doable" from the same source of truth.
- **Generic confirm-gated action control in the frontend.** The workbench renders
  a runnable preview→confirm→apply control for every executable row that belongs
  to the current project, generated from that row's `execution.parameters`, with
  no free-text shell/path/argv field and no hard-coded per-command widget.
- **Three new typed, session-gated execution routes** completing the lifecycle
  write set: `POST /v1/admin/projects/{id}/feature/remove`,
  `POST /v1/admin/projects/{id}/feature/upgrade`, and
  `POST /v1/admin/projects/{id}/spec/apply`. Each reuses the existing
  preview→confirm→digest guard and delegates, only on a matching digest, to the
  Core handler the CLI runs. No shell, no subprocess, no browser-supplied path.
- **Catalog reclassification.** The `feature.remove`, `feature.upgrade` and
  `spec.apply` rows move from `not_yet_web` (or `cli_only`) to `web` with those
  real routes, and the existing `feature.add`/`spec.generate` rows gain the
  `execution` block so the frontend renders them generically too.

No existing route, contract version, or already-archived capability is redefined;
this package consumes the `forge-web-command-execution` digest gate and the
`forge-web-command-catalog` row shape and extends both.

## Package Boundary and Split Assessment

Single outcome: **a signed-in operator can execute the project's feature/spec
lifecycle *write* actions from the portal, driven by self-describing catalog
rows, each gated by preview + confirm + matching digest, with no
path/argv/subprocess/provider.** One actor (authenticated admin), one capability
(portal execution of project lifecycle writes), one state model
(previewed descriptor → digest-bound confirm → Core mutation), one oracle
(the command becomes executable end-to-end and the catalog agrees).

Split signals considered and how they are handled:

- Read-only surfaces (`spec list`/`spec route`, `release list`/`inspect`,
  `standard check`/`diff`, `remediate scan`) are a different oracle (GET
  projection, no mutation gate) → **deferred to `forge-web-project-reads`.**
- Describe/classify proposal `approve`/`reject` carry their own state machine and
  confirm semantics → **deferred to `forge-web-review-actions`.**
- Portfolio *remove*-side metadata (`tag.remove`, `relation.remove`) live in the
  portfolio-controls owner (currently add-only), a different surface → **deferred
  to `forge-web-portfolio-mutation-completion`.**
- `build`/`test` (subprocess), `deploy`/`publish` (provider), `agent` (PTY),
  transport/`serve`, hidden-input identity commands are **excluded by the user's
  chosen scope and by ROADMAP line 91 / AGENTS.md**; this package does not
  weaken that.

This is the smallest independently verifiable unit that both fixes the
"decorative list" structure and adds real executable commands: the `execution`
block and the generic frontend control are useless without at least one wired
command, and the three commands would each need bespoke UI without the block. They
share owner, lifecycle, contract and acceptance, so they stay together.

Package map (this row is implemented now; others are follow-ups with the shown
dependency edge):

| Package | Single outcome | Owner/project & language | Boundary/contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| **forge-web-project-actions** (this) | Execute project feature/spec lifecycle writes from the portal via self-describing catalog rows | forge (Rust) — `src/api/command_catalog.rs`, `src/api/admin.rs`, `src/api/mod.rs`, `frontend/` | `execution` block on web rows; three new typed confirm/digest routes delegating to `remove_feature`/`upgrade_feature`/`apply_routing` | forge-web-command-execution (digest gate), forge-web-command-catalog (row shape) | contract test: each row executable, preview writes nothing, wrong digest refused, confirmed run equals CLI, catalog/`problems()` agree |
| forge-web-project-reads | Read-only project Core surfaces rendered from catalog rows | forge (Rust) — `src/api`, `frontend/` | GET typed id-scoped projections | forge-web-project-actions | read projection tests; no mutation |
| forge-web-review-actions | Describe/classify proposal approve/reject in the portal | forge (Rust) — `src/api`, `frontend/` | own confirm + Suggested→Approved state transition | forge-web-project-actions | review-decision tests |
| forge-web-portfolio-mutation-completion | Portfolio tag/relation removal in the portal | forge (Rust) — `src/api/portfolio.rs`, `frontend/` | remove actions on the existing portfolio POST surface | forge-web-command-catalog | removal round-trip tests |

## Sibling and Shared Architecture Reconnaissance

This is a follow-up inside the same repository and capability family, not a new
first spec, so the reconnaissance is a locate-the-existing-owner check rather than
a cross-project extraction.

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| Preview→confirm→digest write gate | `src/api/admin.rs` `authoring_descriptor`/`authoring_digest`/`authoring_write` | The whole two-step guard, JSON gate, path-free canonical descriptor, digest compare, delegate-on-match | None — same admin owner, same contract version | `forge-web-command-execution` canonical spec | **extend shared owner** (reuse the existing guard for the three new commands) |
| Typed Core lifecycle handlers | `src/feature/mod.rs:799,886`; `src/spec/mod.rs:1037` (`remove_feature`,`upgrade_feature`,`apply_routing`) | In-process, registry-id-scoped, rollback-safe writes already used by the CLI | None | `src/feature`, `src/spec` (unchanged) | **adopt existing capability** (call them; do not modify) |
| Id-only project scoping + path-free projection | `src/api/workbench.rs:140-154`, `validate_project_id` | Server resolves the stored path from the validated id; responses never serialize it | None | `forge-project-workbench` | **adopt** |
| Catalog row shape + `problems()` agreement | `src/api/command_catalog.rs` `push`/`web_at`/`IMPLEMENTED_WEB_ROUTES` | Route constants sourced from handler modules so router and catalog cannot diverge | `CommandRow` has no `execution` block yet | `forge-web-command-catalog` | **extend shared owner** (add `execution`, keep additive) |

No sibling `common`/`manager` project owns a portal-execution engine; the concern
is project-local (it depends on Forge's own Core handlers and the standalone
frontend). Decision: **keep the implementation in forge**, extending the existing
`src/api` admin/catalog owners. Nothing in `src/feature`, `src/spec`, or the CLI
dispatch is changed.

## BFS Impact Map

- **Capabilities touched:** `forge-web-command-catalog` (row shape gains
  `execution`; agreement test extended), `forge-web-command-execution`
  (digest-gate owner extended to more commands — referenced, not redefined), new
  `forge-web-project-actions` (execution of lifecycle writes + generic form).
- **Callers/flows:** `GET /v1/admin/commands` and `GET /v1/admin/projects/{id}`
  serialization (frontend consumes); the workbench action panel in
  `frontend/app.js`/`index.html`; the Clap-parity test in `src/main.rs` (ids
  `feature.remove`/`feature.upgrade`/`spec.apply` already exist, so no new ids).
- **Contracts/persistence:** catalog `CONTRACT_VERSION` stays `0.1.0` (additive
  field); new request bodies carry only structured scalars (feature id, optional
  version, finding id set, reason, confirm, plan_digest); registry + manifest and
  `.forge/` writes happen only inside the existing Core handlers; `spec.apply`
  writes spec/proposal files under `.forge/` via `apply_routing` only.
- **Integrations/config:** none new; no provider, no toolchain, no env key.
- **Failure/boundary:** unsigned → 401; non-JSON → 415; unknown project id →
  404; missing required parameter → 400 typed (never echoed as path/argv);
  unconfirmed → preview+digest, no write; digest mismatch → 409
  `admin-digest-mismatch`, fresh digest, no write; Core handler error → its typed
  failure surface, response never serializes the absolute path.
- **Tests:** extend the `forge_web_command_execution_contract`-style harness to
  the three new routes; assert preview-writes-nothing, mismatch refused, confirmed
  run equals a direct Core call, catalog `problems()` clean and rows executable;
  add a serialization test that every `web` row carries a well-formed `execution`
  block and every non-`web` row does not.
- **Dependencies:** Rust 2021 (≥1.87), existing `serde`/`serde_json`/`sha2`/`chrono`
  in `src/api`; no new crate.
- **Compatibility/privacy/security:** absolute paths stay redacted; no
  browser-supplied filesystem path (id-only, server-resolved); the allowlist is
  the fixed command-id→handler match (no generic shell/argv), satisfying
  `.ai-rules/concerns/security.md` ("invoke processes with argument arrays rather
  than interpolated shell" — here we invoke *no* process at all, only typed
  in-process Core fns) and AGENTS.md ("no generic shell/sh -c/interpolated argv").
  Unaffected and explicitly out of scope: delivery/publish pipeline, identity,
  DriftWatch, PTY agent manager, fleet/inventory reads.

## Capabilities

- New: `forge-web-project-actions` — portal execution of project feature/spec
  lifecycle writes, driven by self-describing catalog rows.
- Modified: `forge-web-command-catalog` — web rows declare an `execution` block
  (route, method, typed parameters, confirmation requirement, risk); the
  catalog/browser agreement invariant extends to that block.

Source requirement sections: §34 (CLI feature/spec surface), §35 (portal API
routes), §36 (portal consumes Core APIs like CLI/MCP).

## Non-goals

- No subprocess execution (`build`/`test`), no provider operations
  (`deploy`/`publish`/release write), no PTY (`agent`), no transport/`serve`, no
  hidden-input identity commands — excluded by the user's chosen scope and
  ROADMAP line 91 / AGENTS.md.
- No generic shell, argv, or browser-supplied filesystem path anywhere.
- No read-only projection surfaces, describe/classify approve/reject, or
  portfolio *remove* metadata — separate follow-up packages.
- No change to Core handlers, the CLI dispatch, or any contract version.
- No publish, deploy, push or archive of unrelated work; no forcing of higher
  maturity.
