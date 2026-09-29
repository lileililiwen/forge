# Design: One reviewed project-to-production workflow

## 1. Implementation boundary

Repository `/home/paul/code/forge`, Rust 2021/MSRV 1.87, edition 2021.

**New module** — `src/delivery/` (single additive module, declared in `src/lib.rs`):

| File | Responsibility |
| --- | --- |
| `src/delivery/mod.rs` | Public API: contract name, error code mappings, the `DeliveryReport` value object, re-exports for CLI/API/UI |
| `src/delivery/state.rs` | `DeliveryPhase` closed enum (`draft`, `preflighted`, `awaiting-stage-confirmation`, `staging`, `stage-healthy`, `stage-failed`, `awaiting-production-approval`, `production`, `healthy`, `degraded`, `hermora-pending`, `hermora-connected`, `hermora-failed`); `DeliveryEnvironment` closed enum (`stage`, `production`); idempotency-key helpers (`preflight_key`, `stage_key`, `promote_key`, `hermora_key`) |
| `src/delivery/projection.rs` | Pure function `build_delivery_report(registry, project_id, now) -> DeliveryReport` reading the existing `operations` table (no new tables). Same shape is consumed by CLI, API, UI |
| `src/delivery/invoke.rs` | `invoke_publish_provider(action, request) -> PublishProviderResponse` thin wrapper that calls `publish::providers::invoke_provider` with the correct `ProviderOperation` (`Preflight`/`Publish`/`Verify`) and bounded queue ids `delivery-stage-<project>-<revision-12>` / `delivery-production-<project>-<revision-12>`. Wraps provider errors into `DeliveryUnavailable` |
| `src/delivery/hermora.rs` | `forge-delivery-hermora/0.1.0` contract: request envelope (JSON, stdin), response envelope (JSON, stdout), bounded `HermoraOutcome` (`Connected { site_id, environment_url }` / `Failed { reason }` / `Unavailable { reason }`). Resolves adapter via `$FORGE_HERMORA_BIN` (default `forge-hermora-adapter` on `PATH`); bounded wall-clock timeout (default 60s, `FORGE_HERMORA_TIMEOUT_SECS` clamped to 1..=600). No credential ever crosses Forge; secrets stay in the adapter's host environment |
| `src/delivery/handlers.rs` | `run_preflight`, `run_stage`, `run_promote`, `run_hermora_retry`, `status_for`. Every handler is a thin adapter: validate args, derive idempotency key, reserve/finalize the journal row, invoke provider, record phase evidence, return a typed outcome |
| `src/delivery/cli.rs` | `cmd_delivery_status`, `cmd_delivery_preflight`, `cmd_delivery_stage`, `cmd_delivery_promote`, `cmd_delivery_hermora_retry`. Each renders the human table and the JSON contract `forge-delivery-status/0.1.0` |

**No new database tables.** Every delivery state is encoded in the existing
`operations` table:

- `kind` ∈ {`delivery.preflight`, `delivery.stage`, `delivery.promote`, `delivery.hermora`}
- `idempotency_key` = `delivery.<verb>:<project_id>:<revision-12>` (deterministic, scoped)
- `request_hash` = SHA-256 hex of `(revision, environment, confirm_token)` — `confirm_token` is the operator-supplied `--confirm-revision <rev>` for promote and `--confirm-operation-id <op_id>` for stage
- `revision` = 40-hex SHA; `build_status`, `run_status`, `container_identity` are the provider's response, copied unchanged
- `detail` = bounded JSON `{ phase, evidence: [...], recovery: [...], hermora: {...} | null }` (under 4096 chars; credential-scrubbed before it lands)

**Other touched files**:

- `src/lib.rs` — add `pub mod delivery;`
- `src/main.rs` — add `Delivery { command: DeliveryCommands }` to the `Commands` enum, the five subcommand dispatchers, and the route/status wiring
- `src/api/mod.rs` — five new `Route::Delivery*` variants, five new arms in `route_request`, five new handlers, error-code mapping for `delivery-invalid`/`delivery-conflict`/`delivery-unavailable`
- `src/api/ui/data.rs` — extend `load_project_detail` with the delivery summary block (read-only projection)
- `src/core/mod.rs` — three new `ForgeError` variants: `DeliveryInvalid`, `DeliveryConflict`, `DeliveryUnavailable`
- `tests/delivery_contract.rs` and `tests/delivery_cross_surface.rs` — new test files

**What must NOT change**:

- `forge publish` / `forge deploy` CLI commands stay byte-identical. The workflow reuses the existing `publish::providers::invoke_provider` and `publish::providers::parse_request` paths; no new subprocess-adapter contract
- `Registry::open` / `Registry::open_read_only` and every existing journal helper stay unchanged — `record_operation` / `reserve_idempotent_operation` / `finalize_operation` / `update_operation_phase` already provide everything the workflow needs
- The UI for fleet list (`/ui`) and project detail (`/ui/projects/{id}`) keeps every existing field; only one additive delivery section joins the existing doctor/journal/share/interest summary
- `forge doctor`, `forge gate`, `forge upgrade`, `forge portfolio *`, and the catalog contract are untouched

## 2. Language and runtime

Rust 2021, MSRV 1.87, edition 2021 (matches `Cargo.toml`). No new dependency
added — `serde`, `serde_json`, `sha2`, `tempfile`, `chrono`, `rusqlite`,
`thiserror`, `serde_yaml`, `clap` are already declared.

Build/test commands:
- `cargo fmt --all -- --check`
- `cargo build`
- `cargo clippy --all-targets -- -D warnings` (must hold at the baseline: only the pre-existing 12 drift locations)
- `cargo test --workspace --all-targets --no-fail-fast -- --skip rust_scaffold_builds_and_tests_with_native_toolchain`
- `node scripts/check-openspec-change-names.mjs`
- `openspec validate --all --strict --no-interactive`
- `git diff --check`

## 3. Ownership and shared code

**Forge owns**: the workflow state machine, the journal projection, the CLI/API/UI surfaces, the `forge-delivery-hermora/0.1.0` contract definition, and the typed refusal vocabulary. Forge does **not** own deployment, rollback, health, or Hermora site registry state — those stay with their respective providers.

**OpenPanel owns**: the `forge-publish-provider/0.1.0` provider implementation. We consume its contract; we do not redefine it. `preflight`, `publish`, and `verify` are reused exactly as documented in `src/publish/providers.rs`.

**Hermora owns**: site registration, Management API access, SEO/content execution, and the actual `forge-delivery-hermora/0.1.0` adapter binary. Forge defines the wire contract because that is what makes the workflow portable across implementations; the implementation stays Hermora-owned.

**No shared `/lib` extraction.** The package is Forge-only; it does not extend a sibling `common`/`manager` project.

## 4. Behavioral model

### State machine (closed, encoded in `DeliveryPhase`)

```
              preflight-ok
draft  ─────────────────────►  preflighted
                                      │
                                      │ stage --confirm-operation-id
                                      ▼
                          awaiting-stage-confirmation
                                      │
                                      │ stage-publish invoked
                                      ▼
                                  staging
                                      │
                         ┌────────────┴────────────┐
                         │                         │
                  verify-healthy            verify-unhealthy
                         │                         │
                         ▼                         ▼
                  stage-healthy             stage-failed
                         │
                         │ promote --confirm-revision
                         ▼
                awaiting-production-approval
                         │
                         │ promote-publish invoked
                         ▼
                    production
                         │
                ┌────────┴────────┐
                │                 │
        verify-healthy     verify-unhealthy
                │                 │
                ▼                 ▼
            healthy           degraded

(draft/preflighted/staging/production/healthy/degraded —> hermora-pending
                  when --hermora-endpoint is configured;
 hermora-pending —invoke—> hermora-connected (record site_id + env_url)
                                   │
                       invoke-failed
                                   │
                                   ▼
                            hermora-failed
                                   │
                       hermora-retry (idempotent)
                                   │
                                   ▼
                       hermora-connected | hermora-failed
```

`stage-failed` and `degraded` are terminal until the operator changes revision and re-runs preflight; `hermora-failed` is retryable, never auto-retried, never silently redeploys.

### Idempotency keys

Deterministic per `(verb, project, revision)` so two calls with the same inputs collapse to one journal row and two calls with different confirmations surface as `IdempotencyKeyConflict`:

- `preflight_key = "delivery.preflight:" + project_id + ":" + revision_12`
- `stage_key    = "delivery.stage:"    + project_id + ":" + revision_12 + ":stage"`
- `promote_key  = "delivery.promote:"  + project_id + ":" + revision_12 + ":production"`
- `hermora_key  = "delivery.hermora:"  + project_id + ":" + revision_12 + ":production"`

`request_hash` is `sha256(revision || environment || confirm_token)` — the same `(verb, project, revision, confirm)` pair always reserves the same `op_id`; a different confirmation rejects as `IdempotencyKeyConflict`.

### Stage semantics

- `preflight` invokes the provider with `ProviderOperation::Preflight`. Success is recorded with `build_status="succeeded"`. Failure (provider exit ≠ 0, unknown contract, missing fields) raises `DeliveryUnavailable` and refuses the next stage.
- `stage` requires `--confirm-operation-id <op_id>` whose `(project_id, kind)` matches a `delivery.preflight` row in `state=healthy|done` for the same revision. Provider invocation uses `ProviderOperation::Publish` and `queue_id="delivery-stage-<project>-<revision-12>"`. The journal row keeps the provider's `build_status`/`run_status`/`container_identity`.
- `promote` requires `--confirm-revision <revision>` and a previous `delivery.stage` row in terminal `healthy` state for the same revision. Provider invocation uses `ProviderOperation::Publish` with `queue_id="delivery-production-<project>-<revision-12>"`. A revised `--confirm-revision` mismatch raises `DeliveryConflict`.
- `verify` (called internally by `status` projection) invokes `ProviderOperation::Verify` with the matching queue_id and reflects the response without re-publishing.

### Hermora onboarding (separate child operation)

- The provider is resolved from `$FORGE_HERMORA_BIN`. Missing/unset → `HermoraOutcome::Unavailable` and the deployment is preserved as `healthy` (Hermora is **optional**, never auto-required).
- The adapter receives this JSON on stdin (`forge-delivery-hermora/0.1.0`):

```json
{
  "contract": "forge-delivery-hermora/0.1.0",
  "operation": "register",
  "project_id": "alpha",
  "environment": "production",
  "revision": "0123456789abcdef0123456789abcdef01234567",
  "deployment_url": "https://alpha.example.com",
  "secret_ref": "env:HERMORA_TOKEN_ALPHA"
}
```

- It must answer on stdout (`bounded to 16 KiB`, credential-scrubbed on receipt):

```json
{ "contract": "forge-delivery-hermora/0.1.0",
  "operation": "register",
  "site_id": "site_abc",
  "environment_url": "https://alpha.hermora.example",
  "status": "connected" }
```

- `status` ∈ {`connected`, `failed`}. Anything else → `DeliveryInvalid`. The adapter never receives a Forge session, a registry byte, or a credential value — only the `secret_ref` name and the public `deployment_url`.

### Concurrency

A single project delivery is serialised on `(project_id, revision)`. Two parallel preflights on the same revision collapse to one journal row via the idempotency key. Two parallel promotes on different revisions are allowed.

### Authorization

- CLI: every verb is operator-scoped; `--confirm-*` flags carry the explicit authority and refuse without them. No ambient authority.
- API: every delivery route sits behind `authorize()`; the existing `admin:access` permission applies (matches the readiness/share portfolio surface). A session minted for project A presented to project B is refused with `api-project-mismatch` before any handler runs.

### Persistence

Only the existing `operations` table. No new migration; no schema change.

## 5. Contract and compatibility

### Machine contract: `forge-delivery-status/0.1.0`

`GET /v1/projects/{id}/delivery` returns:

```json
{
  "contract": "forge-delivery-status/0.1.0",
  "project": "alpha",
  "phase": "awaiting-production-approval",
  "environment": "stage",
  "revision": "0123…",
  "updated_at": "2026-09-29T12:00:00Z",
  "preflight": { "operation_id": 12, "evidence": ["…"], "recovery": [] },
  "stage":     { "operation_id": 13, "status": "stage-healthy", "build_status": "succeeded", "run_status": "succeeded", "container_identity": "forge-alpha-0123abc456de", "operation_id_required_for_promote": 13 },
  "promote":   null,
  "hermora":   { "operation_id": null, "site_id": null, "environment_url": null, "status": null }
}
```

`phase` and `environment` are **closed** — only the values listed in §4 appear. The response never embeds `revision` from a provider claim that differs from the projection's `revision`; the project source revision wins.

### CLI grammar

```
forge delivery status <project>
forge delivery preflight <project> [--revision <sha>]
forge delivery stage <project> --confirm-operation-id <op_id> [--revision <sha>]
forge delivery promote <project> --confirm-revision <sha>
forge delivery hermora-retry <project> --deployment-url <url> --secret-ref <ref>
```

`--revision` defaults to the project's `last_commit` recorded in the registry. Refused without `--confirm-*` (CLI: `error[delivery-invalid]` with `0 bytes` on stdout, mirroring `forge portfolio share publish` and `forge gate`).

### Routes

| Method | Path | Verb | Confirmation gate |
| --- | --- | --- | --- |
| `GET` | `/v1/projects/{id}/delivery` | `Route::DeliveryStatus { id }` | none (read-only) |
| `POST` | `/v1/projects/{id}/delivery/preflight` | `Route::DeliveryPreflight { id }` | none |
| `POST` | `/v1/projects/{id}/delivery/stage` | `Route::DeliveryStage { id }` | `confirm_operation_id` field |
| `POST` | `/v1/projects/{id}/delivery/promote` | `Route::DeliveryPromote { id }` | `confirm_revision` field |
| `POST` | `/v1/projects/{id}/delivery/hermora/retry` | `Route::DeliveryHermoraRetry { id }` | `deployment_url` + `secret_ref` |

### Compatibility

- Every existing transport (`forge publish`, `forge deploy`, `forge doctor`, `forge gate`, `forge portfolio *`) keeps its CLI grammar, exit code, and output shape byte-identical
- `forge api serve` keeps `/healthz` byte-identical
- `forge ui` (portal) gains one new section on `/ui/projects/{id}`; the existing sections (manifest, doctor, journal, share, interest, portfolio) stay byte-identical
- `forge mcp *` is unchanged (delivery is **not** an MCP tool yet — see §6 Non-goals)

## 6. Failure and boundary policy

| Case | Behaviour |
| --- | --- |
| Missing project | `unknown-project` (existing) |
| Missing `--confirm-*` | `delivery-invalid`; 0 bytes stdout |
| Stale confirm (different revision) | `delivery-conflict` (409) |
| Provider binary missing | `delivery-unavailable`; stage stays `awaiting-stage-confirmation` |
| Provider exit ≠ 0 | `delivery-unavailable`; journal row `state=failed`; transition to `stage-failed` |
| Verify after stage reports unhealthy | `degraded`; promote is refused until revision changes |
| Stale approval (different revision) | `delivery-conflict` |
| Hermora binary missing | `delivery-unavailable`; deployment stays `healthy` |
| Hermora timeout (> timeout) | `delivery-unavailable`; operation stays `hermora-pending`; retry is idempotent |
| Hermora response not the contract | `delivery-invalid`; operation stays `hermora-pending` |
| Duplicate idempotency key with different request hash | `idempotency-key-conflict` (existing) |
| Same idempotency key with the same request hash | re-used `op_id`; no second provider invocation |
| Empty registry / no journal rows | `DeliveryPhase::Draft`; `evidence: []` |
| `path-unavailable` (project source directory gone) | `delivery-invalid`; preflight refused |
| Credential value sneaking into any output | redacted by `policy::redact_credentials`; never echoed |

Hermora onboarding **never** re-invokes the provider. It is a separate child operation that retries only its own adapter.

## 7. Verification oracle

| Layer | Files | Coverage |
| --- | --- | --- |
| Unit | `src/delivery/state.rs`, `src/delivery/projection.rs`, `src/delivery/hermora.rs`, `src/delivery/invoke.rs` | closed `DeliveryPhase`; deterministic ordering; idempotency keys; projection over empty/non-empty journal; Hermora envelope parsing (success / failure / wrong contract / over-bound / non-JSON); provider invocation rejects on missing binary and wraps errors |
| CLI contract | `tests/delivery_contract.rs` | help; refusal without `--confirm-*`; successful preflight/stage/promote; stage-healthy→promote→healthy; stage-failed→promote-refused; hermora-pending→hermora-retry→hermora-connected; hermora timeout does not redeploy; duplicate idempotency collapses; stale-confirm rejected |
| Cross-surface | `tests/delivery_cross_surface.rs` | CLI/API parity for status + promote; `delivery-invalid` typed code across both transports; same JSON payload for the same operation_id; hermora-only credential (via adapter stdin capture) never reaches Forge logs |
| Live smoke | `cargo run -- forge delivery …` against the local registry | one happy-path promotion; one timeout recovery; one Hermora retry |

**Fake providers**: tests write a small shell script into `tempfile::tempdir()` and prepend it to `PATH`. The script answers the `forge-publish-provider/0.1.0` JSON contract and is killed at the `provider_timeout` budget. Hermora fakes live alongside.

**Blocked, honestly recorded**: the live 20/20 evidence against the actual OpenPanel provider binary and the actual Hermora adapter is **not claimed** by this change. It is a separate release qualification; this package's contract tests prove the orchestration, not the live integration.

## 8. Decision ledger

**Resolved**:

- New module `src/delivery/`, no new dependency, no new database table
- State machine encoded in the existing `operations` table with deterministic idempotency keys
- Hermora contract `forge-delivery-hermora/0.1.0` defined by Forge, implemented Hermora-side
- Promote is revision-bound; stage is operation-bound; both reject stale confirmations
- Hermora onboarding is a separate child operation; failure does not redeploy
- Provider failures surface as `delivery-unavailable`, never as `delivery-invalid`
- UI gains one additive section; no field is renamed or removed

**Deferred**: MCP surface (`forge mcp delivery *`); multi-cloud fan-out; DNS/TLS automation; automatic rollback; billing; portal write surfaces; automated preflight gate composition (this package reads `forge gate` evidence via the operations journal only).

**External blockers**: live OpenPanel provider binary (OpenPanel-owned, conformance-tested in their change) and live Hermora adapter binary (Hermora-owned). This package's live-provider check is gated on both, recorded as `not run` until they ship.