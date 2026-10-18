# Design: Gate delivery preflight on current quality evidence

## Implementation boundary

Repository `/home/paul/code/forge`, Rust 2021 workspace. Core implementation is `src/delivery/handlers.rs::run_preflight` and its project resolution helper. Use `src/gate/engine.rs::load_latest_evidence` and `evidence_freshness` and existing `ForgeError::DeliveryConflict` / `DeliveryUnavailable`. Tests are in `tests/delivery_contract.rs`, `tests/delivery_cross_surface.rs`, and `tests/forge_web_project_delivery_browser.rs`. Update `openspec/specs/project-to-production-workflow/spec.md`, `docs/ci-cd-system-model.md`, and existing UI behavior only if its error renderer fails to present the returned actionable reason. Do not change provider, persistence schema, or sibling projects.

## Language and runtime

- Rust 2021 with declared MSRV 1.88, stable toolchain, Clap CLI, Axum API, SQLite registry.
- Git is invoked read-only with `std::process::Command` argument arrays; working directory is the registered project root.
- Build/test commands: `cargo fmt --check`, `cargo test --test delivery_contract`, `cargo test --test delivery_cross_surface`, `cargo test --test forge_web_project_delivery_browser`, and the repository-required `forge gate` flow.

## Ownership and shared code

Gate evidence parsing and revision freshness semantics are owned by `src/gate/engine.rs::load_latest_evidence` and `evidence_freshness`; delivery consumes those functions and must not duplicate parsing or freshness rules. Delivery orchestration/error mapping remains in Forge Core. No shared sibling code is changed or added; concrete projects use Forge's existing Gate operation and don't require sibling sources.

## User experience and interface

Existing CLI command, project API, and `/workbench` Delivery card remain the entry points. On a refused preflight, CLI stderr and API's typed error detail name the failed precondition and next action. Workbench renders the API error in its existing announced error region; verify missing-evidence and stale-source states plus the successful browser flow. No new layout or controls. Keep focus and live-region behavior already provided by the Delivery view; if source inspection shows no such region, add minimal inline `role=alert`/status text adjacent to the preflight action.

## Behavioral model

| Order | Check/action | Failure result | Persistent effect |
|---|---|---|---|
| 1 | Resolve registered project directory and valid 40-hex `last_commit` | Existing delivery error | None |
| 2 | Run `git rev-parse HEAD`; require output to be a valid full SHA equal to `last_commit` | `delivery-conflict`, explain `forge register` recovery | None |
| 3 | Load persisted evidence with `load_latest_evidence(root)` | Preserve typed evidence read/parse errors | None |
| 4 | Require matching `project_id`, `evidence_freshness(evidence, Some(revision)) == fresh`, aggregate `passed`, and `dry_run == false` | `delivery-conflict`, identify freshness/aggregate and instruct `forge gate <path>` | None |
| 5 | Resolve provider, reserve idempotent operation, invoke provider, finalize evidence | Existing behavior | Existing journal behavior |

The check is read-only and precedes provider resolution, idempotency reservation, and invocation. A retry after running Gate at the same commit may proceed. A changed commit must be re-registered and Gate rerun against that commit. Do not run Gate implicitly. The existing Gate model binds to Git HEAD; dirty working-tree content remains outside this package's invariant.

## Contract and compatibility

No new transport fields or persistent records. Reuse stable `delivery-conflict` for revision/evidence precondition failures, and propagate existing Gate errors for malformed evidence. CLI/API error rendering uses the common `ForgeError` code/detail. A passing current Gate evidence preserves the existing provider request and operation key. Callers with no evidence must now run `forge gate` before delivery.

## Failure and boundary policy

Missing or invalid `HEAD`, registered revision mismatch, absent/stale evidence, or aggregate other than `passed` refuses with no provider call and no journal mutation. Git spawn/exit/invalid output refuses closed. Malformed Gate evidence preserves its existing typed Gate error and refuses. Provider errors occur only after all checks and retain existing journaling behavior. No retries, fallback, or automatic Gate invocation are added.

## Verification oracle

- Seed valid PASS evidence using Forge's `GateEvidence` and `save_evidence` test APIs at the actual repository SHA.
- Assert successful preflight calls the provider once and creates a terminal row.
- For missing, stale, blocked, failed, unknown evidence and registry/HEAD mismatch, assert typed refusal, zero provider marker calls, and no preflight row.
- Assert CLI/API typed errors match and include recovery detail.
- Run the Chromium workbench through missing-evidence refusal, verify its alert and recovery command, then add current PASS fixture evidence and complete preflight → stage → promote.
- Run targeted tests, formatting, name preflight, strict validation, diff check, Gate dry-run and bounded full Gate; record outcomes in HANDOFF before archive.

## Decision ledger

- Resolved: reuse persisted Gate evidence and shared freshness classifier; reject instead of auto-running Gate; validate actual HEAD against registry before interpreting evidence.
- Resolved: no clean-worktree rule in this package because current Gate freshness contract is HEAD-based and the requested delivery gate is evidence-bound; changing that source model would be a separate policy decision.
- Deferred: authenticated hosted evidence import, CI run identity, unified timeline, local/hosted check profiles, runtime version alignment, binary targets and multi-service topology. They have separate owners/contracts and remain in the CI/CD model gap register.
