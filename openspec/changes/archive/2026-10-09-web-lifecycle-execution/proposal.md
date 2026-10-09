# Proposal: Web lifecycle execution (idea → maintain)

## Why

On `main` at `58d3963` the workbench rail renders idea→maintain but the ends are preview strings, not execution:

- **Gap1 — Idea has no web execution.** `#wb-idea-entry` previews `forge graduation preview/import` strings and links to studio spec entry. No typed artifact textarea, no profile/id fields, no preview digest, no confirm that runs the same in-process `validate_graduation`/`adopt_graduation` op as the CLI. Studio spec save exists as an endpoint, refine journals but the browser never shows the revision bump or the journal row.
- **Gap2 — Maintain has no web execution for intent/remediate.** Failed doctor/status rows preview `forge remediate plan --finding <id>` for the terminal only; no `remediate plan → apply` digest-bound web op. No `intent resolve → apply` digest-bound web op. Delivery publish/approve/stage are real but success renders only a link, writing no journal row for the next-idea transition.
- **Gap3 — Confirm buttons are not all clickable end-to-end.** The generic `buildActionControl` preview→confirm→run discipline exists for catalog `web` rows, but graduation/intent/remediate rows are `NotYetWeb`, so their cards render honest non-executable states. No harness clicks every lifecycle confirm in order, asserts console-error-free runs, error-summary focus, or `role=status` updates.

## What Changes

- **Idea execution (backend + web).** New session-gated admin routes `POST /v1/admin/graduation/preview` (artifact JSON text → validated brief + `plan_digest`, no write) and `POST /v1/admin/graduation/import` (same artifact + `profile` + optional `id` + `confirm:true` + `plan_digest` → stale-digest refused with fresh preview, else same in-process `parse_artifact`/`validate_graduation`/`build_proposal`/`adopt_graduation` as CLI, destination joined server-side under `FORGE_ADMIN_PROJECTS_ROOT`, journal row `graduation.import`). Web `#wb-idea-entry` becomes a real form (artifact textarea, profile select, id override, Preview → confirm checkbox → Run, `role=status` result, journal evidence reload). No shell, no browser path.
- **Maintain execution (backend + web).** New session-gated per-project routes `POST /v1/admin/projects/{id}/intent/resolve` (typed intent fields → plan + `plan_digest`, receipt not yet written) and `.../intent/apply` (`confirm:true` + `plan_digest` + `plan_id` → recompute, stale refused 409 with fresh plan, else `write_plan_receipt` + `apply_plan` same in-process op as CLI, journal `intent.apply`); `POST .../remediate/plan` (`finding` → plan + `plan_digest`, no write, target resolved server-side) and `.../remediate/apply` (`confirm:true` + `plan_digest` → recompute, stale refused, else same in-process `build_plan`/`apply(confirm=true)` as CLI, journal `remediate.apply`). Keep the existing terminal `forge remediate plan` string where shown. Delivery keeps approve/publish/stage; success additionally records journal row `delivery.next-idea` via `POST .../delivery/next-idea` and renders it in `#delivery-next-idea` plus the workbench operations table.
- **Studio refine wiring (web only).** Refine confirm reuses existing `POST .../studio/refine`; the card shows the returned `spec_revision`/`app_revision` bump in a `role=status` result and reloads journal evidence. No new endpoint.
- **Harness.** Extend `tests/browser/lifecycle-rail-check.mjs` to click every lifecycle confirm in order on a throwaway registry (idea preview→confirm, scaffold `new`, spec save, gate dry-run plan, delivery approve/publish dry paths with confirm-refused-without-digest, maintain refresh, remediate plan/apply, intent resolve/apply), asserting zero JS console errors, error-summary focus on invalid, `role=status` updates, per-step screenshots. `tests/lifecycle_rail_browser.rs` drives it (exit 2 → UNVERIFIED when toolchain absent). Fix all true bugs found by clicks.

## BFS Impact Map

- **Capabilities:** `portal-web-ui` (idea form, refine revision display, next-idea journal display), `forge-web-command-execution` (4 new web_exec rows + 2 global), `validated-intent-planner` + `project-local-remediation-plans` (web projection of existing Core ops, no Core semantics change).
- **Users/flows:** browser operators walking idea→scaffold→gate→publish→maintain entirely in the workbench; CLI flows byte-identical.
- **Contracts/data/persistence:** 7 new admin routes (additive, session-gated, path-free); catalog `web_exec` rows for `graduation.preview|import`, `intent.resolve|apply`, `remediate.plan|apply` (availability `web`, routes point at new endpoints); journal kinds additive (`graduation.import`, `intent.apply`, `remediate.apply`, `delivery.next-idea`); no registry shape change; digests are hex SHA-256 of canonical preview bytes.
- **Integrations:** Core reuse only (`graduation::{parse_artifact,validate_graduation,build_proposal,adopt_graduation}`, `planner::{validate_intent,resolve_plan,write_plan_receipt,apply_plan}`, `remediation::{build_plan,apply}`, `studio::record_refinement`); no shell, no provider, no adapter change.
- **Tests:** `portal_ui_contract` (+lifecycle-execution tokens), `lifecycle_rail_browser` (click-every-confirm harness), new `web_lifecycle_execution_contract` (digest binding, stale refusal, journal rows, catalog routes); pinned playwright 1.63.0 reused, no new dep.
- **Compatibility:** CLI byte-identical; `NotYetWeb` → `web` only for the six rows above; terminal remediate string kept; unknown/invalid inputs keep typed refusals.

## Capabilities

- Graduation preview/import web execution with typed fields and digest-bound confirm.
- Intent resolve/apply web execution with stale-plan refusal and journal evidence.
- Remediate plan/apply web execution with stale-plan refusal and journal evidence.
- Delivery next-idea transition writing a journal row.
- Studio refine confirm showing the revision bump plus journal evidence.
- Click-every-confirm browser oracle (console-error-free, error-summary focus, status updates, screenshots).

## Non-goals

- No new planner/remediation Core semantics, no receipt format change.
- No scaffold/gate native-toolchain execution in the browser (scaffold `new` dry preview only where already web; gate stays dry-run plan display).
- No provider writes beyond existing delivery approve/publish/stage.
- No new frontend framework or browser dependency.
