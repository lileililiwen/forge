# Design: Catalog-driven execution of project lifecycle writes

## 1. Implementation boundary

Repository/project: **forge** (this repository). Language: **Rust 2021**, min
rustc 1.87; standalone **HTML/CSS/JS** frontend served by the Rust web listener.

Files to change:
- `src/api/command_catalog.rs` — add an `execution` block to `CommandRow`; emit it
  for `web` rows via a new `web_exec` constructor (extends `web_at`); add the
  three lifecycle routes to `IMPLEMENTED_WEB_ROUTES`; convert `feature.remove`,
  `feature.upgrade`, `spec.apply` rows to executable `web` rows carrying their
  parameter schema; add the `feature.add`/`spec.generate` execution blocks.
- `src/api/admin.rs` — export `ROUTE_ADMIN_FEATURE_REMOVE`,
  `ROUTE_ADMIN_FEATURE_UPGRADE`, `ROUTE_ADMIN_SPEC_APPLY`; extend the
  `Authoring` enum and `authoring_descriptor`/`authoring_write` to cover the three
  commands and delegate on confirm to the Core handlers.
- `src/api/mod.rs` — add `Route::AdminProjectFeatureRemove`,
  `Route::AdminProjectFeatureUpgrade`, `Route::AdminProjectSpecApply`; add routing
  arms, permission-group entries, the `admin::handle` dispatch `matches!` arm, the
  404 exhaustiveness arm and the authorization arm (mirror the existing
  `AdminProjectFeature`/`AdminProjectSpec` sites exactly).
- `frontend/index.html` + `frontend/app.js` — render a generic confirm-gated
  action control per executable row of the current project, driven by
  `execution.parameters`.

Must NOT change: `src/feature/mod.rs`, `src/spec/mod.rs`, `src/main.rs` CLI
dispatch, the Core handler bodies, any `CONTRACT_VERSION`, the delivery/publish
pipeline, identity, or DriftWatch/PTY/analytics.

## 2. Language and runtime

- Build: `cargo build`. Test: `cargo test` (unit + `tests/*_contract.rs`).
- Format/lint: `cargo fmt`, `cargo clippy`.
- Frontend: no bundler; `frontend/` served statically by `forge web serve`; API
  JSON-only at the origin in `frontend/config.js` (`FORGE_API_BASE`).
- Crypto/hash for digest: `sha2::Sha256` (already used by `authoring_digest`).
- Times: `chrono::Utc::now()` passed into the Core handlers, as the CLI does.

## 3. Ownership and shared code

All code stays project-local in forge. The digest write-gate is owned by
`src/api/admin.rs` (from `forge-web-command-execution`) and is **extended**, not
duplicated: `authoring_descriptor`, `authoring_digest` and `authoring_write`
become parameterized over a wider `Authoring` set
(`{FeatureAdd, FeatureRemove, FeatureUpgrade, SpecGenerate, SpecApply}`) rather
than a parallel implementation. The three Core handlers are **adopted as-is** —
`remove_feature(&mut Registry, target, feature)`,
`upgrade_feature(&mut Registry, target, feature, Option<version>)`
(`src/feature/mod.rs:799,886`) and
`apply_routing(&SpecRequest, &FindingSource, now)` (`src/spec/mod.rs:1037`) —
called from the same dispatch path `handle_add_feature`/`handle_generate_spec`
use. There is no sibling `common`/`manager` owner; the extension point is the
`Authoring` enum + `execution` block so a later package adds a command by adding
one enum arm, one route const and one catalog row, not a new subsystem.

## 4. Behavioral model

Request flow for every executable lifecycle action (identical to the shipped
`feature.add` gate):

| Step | Input | Action | Output | Write? |
|---|---|---|---|---|
| JSON gate | any | require `content-type: application/json` | else `415` | no |
| Session gate | cookie | require valid `forge_admin_session` | else `401` | no |
| Id gate | `{id}` | `validate_project_id`; resolve project from registry | unknown/observed-only → `404` | no |
| Field gate | body | require the command's mandatory typed fields | missing/blank → `400` typed (input not echoed) | no |
| Preview | `confirm` absent/false | build path-free canonical descriptor → `plan_digest` | `200 {preview, plan_digest, confirmation}` | **no** |
| Confirm mismatch | `confirm:true`, `plan_digest` ≠ recomputed | recompute from current fields | `409 {error.code:"admin-digest-mismatch", preview, plan_digest}` | **no** |
| Confirm match | `confirm:true`, matching digest | call the named Core handler with the validated id + fields | `200` typed handler result | yes |

Per-command structured fields (all id/scalar, never a path/argv):
- `feature.remove`: `feature` (non-empty string). → `remove_feature`.
- `feature.upgrade`: `feature` (non-empty string), `version` (optional string).
  → `upgrade_feature`.
- `spec.apply`: `findings` (non-empty array of finding-id strings), `reason`
  (optional string). → build the same `SpecRequest`/`FindingSource` the CLI's
  `spec apply` builds (`finding_source_for` synthesizes the in-process source from
  the finding-name prefix; no caller path).

Idempotency: an idempotent replay that already applied re-runs the typed handler,
which is itself idempotent (manifest rewrite / routing record); no separate
journal write is added here. Concurrency: SQLite registry access is serialized by
`Registry::open` as elsewhere. Authorization: enforced by the dispatch layer
`required_permission` (admin session), independent of the route body.

## 5. Contract and compatibility

`CommandRow` gains an additive optional field serialized only for `web` rows:

```json
"execution": {
  "route": "POST /v1/admin/projects/{id}/feature/remove",
  "method": "POST",
  "risk": "local_write",
  "confirm_required": true,
  "digest_bound": true,
  "parameters": [
    { "name": "feature", "kind": "string", "required": true }
  ]
}
```

`kind` ∈ `string | string_array | boolean`; a closed option set MAY be expressed
via an added `options: [...]` (used by no row in this package). Non-executable
rows: `"execution": null`. `CONTRACT_VERSION` stays `0.1.0` (purely additive).
Response bodies of the three new routes reuse the exact shape the shipped
`/feature` and `/spec` routes return (`{preview, plan_digest, confirmation}` /
handler result), so the existing frontend result-rendering path is unchanged. No
bearer `/v1` route changes; only the session-admin surface gains routes.

## 6. Failure and boundary policy

- Non-JSON body → `415` (no Core call). Missing/invalid session → `401`.
- Unknown or observed-only project id → `404` (typed, path never serialized).
- Missing mandatory field / blank feature / empty findings → `400` typed error;
  the offending value is never echoed (mirrors the workbench hostile-input rule).
- Unconfirmed mutation → preview + digest, no write (success, not an error).
- Digest mismatch → `409 admin-digest-mismatch` + fresh digest, no write.
- Core handler returns `ForgeError` → surfaced through the existing typed error
  mapping; absolute path scrubbed. Never reported as success.
- No partial-write handling beyond what the Core handler already does (its own
  rollback); this layer adds none and claims none.

## 7. Verification oracle

Layer A — new `tests/forge_web_project_actions_contract.rs`, reusing the
`forge_web_command_execution_contract` harness (`request`/`with_cookie`/
`json_body`/`login_token`/`write_project`/`register_project`/`body_json`):
1. `anonymous_and_non_json_refused_before_core`: JSON-no-cookie → `401`;
   cookie-no-JSON → `415`, for all three routes.
2. `preview_writes_nothing_per_command`: preview returns `{preview, plan_digest}`
   and the on-disk manifest / `.forge` state is byte-identical afterward.
3. `wrong_digest_refused_per_command`: confirmed request with a stale/mismatched
   digest → `409 admin-digest-mismatch`, no mutation observed.
4. `confirmed_run_equals_core`: confirmed matching-digest request mutates
   identically to a direct `remove_feature` / `upgrade_feature` / `apply_routing`
   call on a sibling fixture (compare resulting `forge.yaml`/`.forge` state).
5. `no_path_or_argv_echo`: hostile `{id}` and field inputs return typed `400`/
   `404` whose body never contains the input or any absolute path.
6. `spec_apply_requires_findings`: empty/missing `findings` → `400`, no write.

Layer B — extend the in-source catalog test
`web_rows_only_point_at_implemented_routes`: add `feature.remove`,
`feature.upgrade`, `spec.apply` to the ordered web vec with their new routes;
add the three `ROUTE_ADMIN_*` consts to `IMPLEMENTED_WEB_ROUTES`.

Layer C — catalog serialization/agreement: a test asserting (a) every `web` row
has a non-null `execution` whose `route` ∈ `IMPLEMENTED_WEB_ROUTES` and whose
`parameters` names match that route's typed fields, (b) every non-`web` row has
`execution: null`, and (c) `problems()` returns empty.

Commands run before any task is checked: `cargo build`, `cargo test`,
`cargo fmt --check`, `node scripts/check-openspec-change-names.mjs`,
`openspec validate --all --strict --no-interactive`, `git diff --check`.
Browser evidence: a Playwright run against a throwaway temp registry (never the
user's registry) that logs in, opens a project, previews then confirms
`feature remove`/`feature upgrade`/`spec apply`, and asserts the manifest/`.forge`
change and zero console errors.

## 8. Decision ledger

- **Resolved:** scope = user-selected "Project actions (Core only)."
- **Resolved:** keep per-command typed routes (not a single argv `execute`
  endpoint) because the fixed command-id→Core-handler match IS the strict
  allowlist AGENTS.md requires; a generic body-driven dispatcher would risk
  re-introducing argv/path interpretation.
- **Resolved:** `execution` block is additive on `0.1.0`, not a contract bump.
- **Resolved:** `spec.apply` source is synthesized in-process from the finding
  prefix (matching the CLI), so no caller-supplied path or stdin is needed.
- **Excluded by design:** `feature.remove`/`upgrade`/`spec.apply` share the
  best-effort `git` capture that `Registry::register`/`finish()` already performs
  for the shipped `feature.add`/`apply` routes (`src/registry/mod.rs:1169,1181`).
  This is identical in kind to what is already in production behind these routes;
  it is not a new subprocess surface and stays within "Core only." Documented as
  accepted, not hidden.
- **Deferred (own packages):** read projections; describe/classify approve/reject;
  portfolio tag/relation removal; build/test; deploy/publish; agent/PTY.
- **Blockers:** none.

Implementation-handoff gate: an agent can implement without inventing
architecture — route consts, enum arms, handler calls, `execution` shape, failure
codes and every test oracle above are named.
