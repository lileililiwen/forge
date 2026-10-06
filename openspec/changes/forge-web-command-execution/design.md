# Design: Browser execution of the handler-backed authoring commands

## Ownership and layering

Forge owns the Rust admin API, its cookie session, the catalog and the
standalone `frontend/`. This change adds no new Core operation and no new
persistence: it routes the browser to the in-process Core handlers that already
back the CLI. `src/api/admin.rs` stays the only place that speaks to `frontend/`
and delegates to the same `handle_add_feature` / `handle_generate_spec` functions
`/v1` uses, rather than duplicating their logic.

## Route contract

Two new paths, mirroring the existing admin shape and each with an `OPTIONS`
`AdminOptions` pair plus a session-gate-allowlist entry so they behave like
`AdminProjectApply`:

```text
POST /v1/admin/projects/{id}/feature   (feature add)
POST /v1/admin/projects/{id}/spec      (spec generate)
```

Each path string is a shared constant exported by a module (as
`workbench::ROUTE_PROJECT_APPLY` already is) and referenced by `src/api/mod.rs`,
`admin.rs` and `command_catalog.rs::IMPLEMENTED_WEB_ROUTES`, so the router and the
catalog can never name different paths.

## Two-step preview + confirm (digest) binding

The bearer handlers write immediately with no gate, so the admin layer adds the
same confirm+digest discipline the workbench uses, in two steps per action:

- **Preview** — `POST` the structured fields with no `confirm`. The admin branch
  runs the Core handler in a read-only/planning way where one exists (feature:
  resolve the descriptor + version; spec: validate findings), then returns a
  bounded summary plus a `plan_digest` computed with the existing `plan_digest`
  helper over the exact canonical action (project id + action + normalized
  fields). No project mutation happens on a preview.
- **Apply** — `POST` the same fields with `confirm: true` and the `plan_digest`
  the operator reviewed. The admin branch recomputes the digest from the current
  request; a mismatch (stale or forged) is refused with a fresh digest and writes
  nothing; only on a match does it delegate to `handle_add_feature` /
  `handle_generate_spec`. The digest binds confirmation to the exact project and
  field set, exactly as the workbench apply binds it to the reviewed plan.

If a safe read-only preview of a specific action is not available in Core, that
action is not exposed in this change rather than exposed without a digest gate.

## Session gate and failures

- Missing/expired session cookie → `401 api-unauthorized`, same as the other admin
  routes, before any Core call.
- Non-JSON content type on these mutations → `415`, refused before the gate.
- Unknown project / empty findings / missing feature id → the Core handler's
  existing typed safe error (no absolute path, no secret).
- No request field is ever interpreted as a shell command, argv vector, or
  filesystem path; fields are the same structured values the CLI accepts.

## Catalog reclassification

`feature add` and `spec generate` move from `not_yet_web` to `web`, each with its
real route added to `IMPLEMENTED_WEB_ROUTES`. The evidence-based web check in the
catalog contract test must still pass (only routes that really exist are `web`),
and the expected `web`-row count and the `not_yet_web` rows for these two commands
are updated. Every other command keeps its honest disposition.

## Frontend workflow

`frontend/app.js` gains, inside the existing project workbench panel, two action
forms: *Add feature* (feature id + optional version) and *Generate spec* (finding
ids + optional reason). Submitting requests the preview, renders the bounded
summary and digest, and offers a confirm control that re-submits with
`confirm: true` + the digest. Results render through the existing safe DOM
builders; admin errors surface their code/message without leaking paths or
secrets. No free-text shell field is introduced.

## Migration

Additive: two routes, two catalog availability changes, one frontend panel. No
data or wire-format breaks. Bearer `/v1` routes and CLI are untouched.
