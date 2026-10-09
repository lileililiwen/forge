# Design: Web lifecycle execution

## Ownership

- Backend: `src/api/lifecycle_exec/` (new: `mod` + `graduation,intent,remediate,studio,delivery`) owns the 9 handlers (7 lifecycle + 2 studio admin wrappers); `src/api/model.rs` owns the 7 `Route` variants; `src/api/router.rs` owns path→route + dispatch + permission; `src/api/command_catalog/{routes,catalog,rows_project}.rs` own the 6 `web_exec` rows.
- Frontend: `frontend/index.html` owns the idea form shell (`#wb-idea-entry` fields); `frontend/app.js` owns `renderIdeaEntry` (real form), refine revision display, `renderDeliveryNextIdea` journal write, and maintain intent/remediate cards (catalog-driven, no bespoke fetch).
- Tests: `tests/web_lifecycle_execution_contract.rs` (new) owns backend contract; `tests/portal_ui_contract.rs` (+tokens) owns static structure; `tests/browser/lifecycle-rail-check.mjs` + `tests/lifecycle_rail_browser.rs` own the click oracle.

## Contracts

- Digest: `hex(sha256(canonical_json(preview)))`; preview = `{kind, project_id?, artifact_digest|plan}` without timestamps; `apply` recomputes fresh preview bytes and compares in constant shape (`supplied != fresh` → 409 + fresh preview + fresh digest, `effect:none`, nothing written).
- Preview (all): `200 {contract, effect:"none", preview, plan_digest, confirmation:{requires:["confirm","plan_digest"], note}}`; no registry/journal/file write.
- Apply (all): requires `confirm:true` (else 409 `workbench-confirm-required`-family message) and non-empty `plan_digest` (else 400); idempotency via `Idempotency-Key` header through `run_with_operation` where a journaled op exists (upgrade/intent/remediate), else journal row + 200/202 with `{accepted:true, operation_id?, outcome}`; replay returns the recorded row, never a second write.
- Graduation preview in: `{artifact_json: string(1..1MiB)}`; out: `{brief title, requirements count, evidence count, source contract/revision, proposed id}` + digest over `(artifact_json, profile, id?)`. Import in: `{artifact_json, profile, id?, confirm, plan_digest}`; destination = `<FORGE_ADMIN_PROJECTS_ROOT>/<resolved id>` resolved server-side (never from browser); runs `parse_artifact → validate_graduation → build_proposal → adopt_graduation`; journal `graduation.import`; out: `{imported id, receipt, operation}`.
- Intent resolve in: `{action:create_project|extend_project, profile?, required?:string[], forbidden?:string[], constraints?:{key,value}[]}` scoped to `{id}` (project dir resolved server-side); out: `{plan (plan_id, steps, intent_hash, catalog_hash), plan_digest}`; receipt written only on apply. Apply in: `{plan_id, plan_digest, confirm}`; server re-resolves + revalidates + compares digest of the resolve output; then `write_plan_receipt` + `apply_plan(confirm=true)`; journal `intent.apply`.
- Remediate plan in: `{finding: string}`; target = registry-resolved project dir (browser never sends `--target`); out: `{plan (plan_id, actions), plan_digest}` via `remediation::build_plan`. Apply in: `{finding?, plan_id?, plan_digest, confirm}`; recompute + compare; then `remediation::apply(&plan, true, registry_path)`; journal `remediate.apply`.
- Delivery next-idea in: `{note?: string(≤2000), confirm:true}`; writes journal row kind `delivery.next-idea` with the note (credential-shaped notes refused); out: `{accepted:true, operation_id}`; rendered in `#delivery-next-idea` and the operations table.
- Studio refine: unchanged route; frontend displays `preview.spec_revision`/`app_revision` deltas and reloads `GET .../detail` operations.

## Failures

- Invalid artifact/intent/finding → typed 400 with the Core code (`graduation-invalid`, `intent-invalid`, `remediation-invalid`, `studio-invalid-spec`) and a path-free message; nothing written; error-summary takes focus; `role=alert` on the card result.
- Missing confirm → 409 refusal, nothing written. Stale/forged digest → 409 + fresh preview + fresh digest, nothing written; the card re-arms on the fresh digest (operator re-confirms).
- Unknown/unmanaged id → 404 unmanaged-project reason (id never echoed); absent project dir → 404 path-unavailable; registry unavailable → 503; all path-free.
- Credential-shaped artifact/refine/note → refused before any journal/file write.

## Migrations

None. Additive routes, additive catalog rows, additive journal kinds, additive DOM ids. CLI, registry shape, receipt formats, and existing web routes byte-identical.

## Alternatives

- Reusing `POST /v1/admin/workspace/onboard` for graduation: rejected — onboarding resolves workspace directories, not artifact bytes; mixing would leak the projects-root join and the artifact trust boundary.
- Writing the intent receipt on resolve (CLI parity): rejected — the web preview must change nothing; receipt+apply happen atomically on confirmed apply.
- Sending `--target` from the browser for remediate: rejected — path-free boundary; the server resolves the target from the registry id.
