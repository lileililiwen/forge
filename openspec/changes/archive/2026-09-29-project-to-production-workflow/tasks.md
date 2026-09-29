# Tasks: project-to-production-workflow

## 1. BFS — Baseline and impact coverage

- [ ] 1.1 Map the existing `publish::providers::invoke_provider` / `parse_request` /
      `parse_response` / `ProviderOperation::{Preflight,Publish,Verify,Rollback}` surface, the
      `operations` table schema (`op_id`, `kind`, `project_id`, `state`, `started_at`,
      `finished_at`, `detail`, `idempotency_key`, `request_hash`, `queue_id`, `revision`,
      `build_status`, `run_status`, `container_identity`), the
      `Registry::{record_operation,reserve_idempotent_operation,finalize_operation,update_operation_phase,
      operation_by_idempotency,operations_for_project}` helpers, and the
      `forge-publish-provider/0.1.0` request/response envelope.
- [ ] 1.2 Add `delivery::state` (closed `DeliveryPhase`, `DeliveryEnvironment`,
      idempotency-key helpers) and `delivery::hermora` (the `forge-delivery-hermora/0.1.0`
      contract — request envelope on stdin, response envelope on stdout, bounded timeout)
      before any wiring. No filesystem side effects in this task.
- [ ] 1.3 Identify every transport entry point that must expose delivery: `forge delivery`
      (CLI), `GET/POST /v1/projects/{id}/delivery/*` (API), and the additive delivery
      block on `/ui/projects/{id}`. Confirm OpenPanel provider conformance and Hermora
      adapter conformance are **external** to this change; fakes substitute for live
      binaries in every test.

## 2. DFS — Requirement-by-requirement implementation

- [ ] 2.1 Implement `delivery::projection::build_delivery_report(registry, project_id,
      now)`. Pure function over the existing `operations` table; closed
      `DeliveryPhase::Draft` when no rows exist; deterministic ordering by `op_id`.
      Covered by `tests/delivery_state.rs` (in `src/delivery/state.rs`) and the cross-
      surface parity suite.
- [ ] 2.2 Implement `delivery::invoke::invoke_publish_provider` (preflight / publish /
      verify wrappers around `publish::providers::invoke_provider` with the bounded
      `delivery-stage-<project>-<revision-12>` and `delivery-production-<project>-
      <revision-12>` queue ids). Errors wrap into `DeliveryUnavailable`.
- [ ] 2.3 Implement `delivery::handlers::run_preflight` — derive idempotency key from
      `(project, revision)`, reserve the journal row, invoke provider preflight,
      finalize with `build_status="succeeded"` (or `failed`); surface recovery
      suggestions from the provider. Refuse when the registry has no source path or
      the revision is missing.
- [ ] 2.4 Implement `delivery::handlers::run_stage` — require
      `--confirm-operation-id <op_id>` (API: `confirm_operation_id` body field); verify
      the referenced preflight row is in `done`/`healthy` for the same revision; invoke
      provider publish with the stage queue_id; record `build_status`, `run_status`,
      `container_identity`; transition `awaiting-stage-confirmation → staging →
      stage-healthy|stage-failed` per the provider response.
- [ ] 2.5 Implement `delivery::handlers::run_promote` — require
      `--confirm-revision <revision>` (API: `confirm_revision` body field); verify the
      stage row for this revision is terminal `stage-healthy`; invoke provider publish
      with the production queue_id; transition to `production → healthy|degraded`.
- [ ] 2.6 Implement `delivery::handlers::run_hermora_retry` — invoke the Hermora
      adapter via the new `forge-delivery-hermora/0.1.0` subprocess boundary; record
      `site_id` + `environment_url` on success; preserve `healthy` on Hermora failure;
      idempotency-key reuse for retried identical inputs; no re-invocation of the
      publish provider.
- [ ] 2.7 Add the `forge delivery status|preflight|stage|promote|hermora-retry` CLI
      subcommand (`src/delivery/cli.rs`, dispatched from `src/main.rs`). Each verb
      renders the human table and the JSON contract
      `forge-delivery-status/0.1.0`; refusals are typed `delivery-invalid` /
      `delivery-conflict` / `delivery-unavailable` with empty stdout on the failure
      path. Stage / promote refuse without `--confirm-*`.
- [ ] 2.8 Add five new `Route::Delivery*` variants in `src/api/mod.rs`, the matching
      arms in `route_request`, and the per-route handlers. Every route sits behind the
      existing `authorize()`; `delivery-conflict` maps to `409`, `delivery-invalid` to
      `400`, `delivery-unavailable` to `503`; cross-project sessions refuse with the
      existing `api-project-mismatch` envelope.
- [ ] 2.9 Extend `src/api/ui/data.rs::load_project_detail` with the additive delivery
      section. The existing manifest / doctor / journal / share / interest / portfolio
      fields stay byte-identical; only one new block joins them.
- [ ] 2.10 Add three new `ForgeError` variants in `src/core/mod.rs`:
      `DeliveryInvalid`, `DeliveryConflict`, `DeliveryUnavailable`, with stable machine
      codes `delivery-invalid`, `delivery-conflict`, `delivery-unavailable` returned
      by `code()`.
- [ ] 2.11 Add `pub mod delivery;` to `src/lib.rs`. No `Cargo.toml` change.

## 3. BFS — Cross-surface regression and completeness

- [ ] 3.1 Test every success/failure transition (preflight ok / preflight fail; stage
      ok / stage fail; promote after healthy / promote refused after degraded; hermora
      ok / hermora fail; hermora retry after fail). Covered in
      `tests/delivery_contract.rs`.
- [ ] 3.2 Test duplicate idempotency keys collapsing to one `op_id` (with same
      request_hash) and refusing (with different request_hash) across both transports.
- [ ] 3.3 Test stale `--confirm-revision` (CLI) and `confirm_revision` (API)
      rejected as `delivery-conflict` with the existing revision echoed in the error.
- [ ] 3.4 Test missing provider binary → `delivery-unavailable`; verify the stage row
      stays in `awaiting-stage-confirmation` and no second invocation occurs.
- [ ] 3.5 Test that a Hermora timeout / failure leaves the deployment at `healthy` and
      a subsequent `hermora-retry` does not invoke the publish provider. Captured via
      adapter stdin capture in the test.
- [ ] 3.6 Test CLI / API parity: identical JSON payload for the same `(project, revision,
      operation_id)` across `forge delivery status` and `GET /v1/projects/{id}/delivery`;
      identical `delivery-invalid` typed code across transports.
- [ ] 3.7 Test that no `forge publish` / `forge deploy` command changed: re-run the
      existing `tests/publish_contract.rs` and `tests/publish_queue_status_contract.rs`
      against this change and confirm zero regressions.
- [ ] 3.8 Test that no schema migration ran: a registry written before this change
      opens, projects and journal rows survive, and the `operations` table carries no
      new columns (covered by reading the row count and column set directly).
- [ ] 3.9 Test credential redaction: a fixture that returns a `ghp_…`-shaped value in
      the Hermora response is refused at the parser and the value never appears in
      `journal.detail` or any rendered output.

## 4. Verification

- [ ] 4.1 Run `cargo fmt --all -- --check` (touched files only; the pre-change
      formatting drift in `src/gate/evidence.rs`, `src/portfolio/share/validation.rs`,
      `src/publish/fleet.rs`, `src/api/ui/auth.rs`, and the related `tests/*` files is
      preserved exactly as prior cycles left it).
- [ ] 4.2 Run `cargo build`, `cargo clippy --all-targets -- -D warnings` (must hold
      at the recorded baseline — same 12 pre-existing locations; zero new clippy
      errors anywhere under `src/delivery/`).
- [ ] 4.3 Run `cargo test --workspace --all-targets --no-fail-fast -- --skip
      rust_scaffold_builds_and_tests_with_native_toolchain`. The
      `fleet_online_routes_to_local_listener_when_alethefy_is_up` failure is pre-
      existing and unrelated; reproduce it on the stashed baseline to confirm.
- [ ] 4.4 Run `node scripts/check-openspec-change-names.mjs`;
      `openspec validate --all --strict --no-interactive`;
      `git diff --check` (staged and unstaged).
- [ ] 4.5 Live binary smoke (CLI) against a two-project local registry:
      `forge delivery status alpha` (draft → preflighted → stage-healthy → healthy
      → hermora-connected); a `--confirm-revision` refusal prints 0 bytes to stdout.
      Document exact commands and outcomes.
- [ ] 4.6 Live binary smoke (HTTP): `forge api serve --bind 127.0.0.1 --port <eph>` —
      `GET /v1/projects/alpha/delivery` with and without an admin session;
      `POST /v1/projects/alpha/delivery/promote` without `confirm_revision` → 400
      `delivery-invalid`; with mismatched revision → 409 `delivery-conflict`.
- [ ] 4.7 Update HANDOFF with the verification evidence and the next active change in
      `openspec list`. Advance the existing pointer in place (per AGENTS.md).
- [ ] 4.8 Archive the change via `openspec archive project-to-production-workflow
      --skip-specs=false` (or equivalent) and promote the canonical specs.
      Commit only related implementation, tests, and the HANDOFF update; do not push.