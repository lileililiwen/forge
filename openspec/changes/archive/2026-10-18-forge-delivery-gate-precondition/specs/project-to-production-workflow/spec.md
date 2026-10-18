## MODIFIED Requirements

### Requirement: Evidence-gated staged delivery

Forge SHALL coordinate project checks, provider preflight, stage publication, and health verification through the existing publish provider contract (`forge-publish-provider/0.1.0`). Before invoking a delivery preflight provider, Forge SHALL require that the registered 40-character source revision equals the source repository's current Git `HEAD` and that persisted Gate evidence has the supported Gate evidence contract, matches the registered project, is a real non-dry-run record, is classified as `fresh` against that revision, and has aggregate `passed`. These checks SHALL complete before provider resolution, operation reservation, or provider invocation. Forge SHALL NOT run Gate implicitly.

#### Scenario: Preflight succeeds

- **WHEN** the project is registered at the current Git `HEAD`, Gate evidence for that project is fresh and has aggregate `passed`, and the provider's `preflight` returns `status=succeeded`
- **THEN** Forge records terminal provider evidence under `kind=delivery.preflight` and advances `phase` to `preflighted`

#### Scenario: Preflight refuses missing or non-passing Gate evidence

- **WHEN** the user invokes preflight and the Gate evidence is absent, stale, blocked, failed, or unknown
- **THEN** Forge refuses with `delivery-conflict`, gives the Gate state and recovery command, and writes no delivery operation row or provider call

#### Scenario: Preflight refuses a changed registered revision

- **WHEN** the repository `HEAD` differs from the registered source revision or cannot be resolved
- **THEN** Forge refuses with a typed delivery error before reading provider configuration, reserving an operation, or invoking the provider; the error instructs the operator to register the current source and run Gate again

#### Scenario: Gate evidence is invalid

- **WHEN** the Gate evidence file exists but cannot be read or parsed
- **THEN** Forge preserves the typed Gate error and does not reserve a delivery operation or invoke the provider

#### Scenario: Preflight fails

- **WHEN** current source and fresh passing Gate evidence satisfy preflight checks but the provider returns `status != succeeded` or the binary is missing
- **THEN** Forge records the failure under `delivery.preflight` with `state=failed`, surfaces the existing provider error, and refuses the next stage

#### Scenario: Stage is healthy

- **WHEN** a `delivery.preflight` row is terminal for the same `(project, revision)`, the user confirms the stage with `--confirm-operation-id <op_id>` (API: `confirm_operation_id`), and the provider's `publish` operation returns terminal `build_status=succeeded` and `run_status=succeeded`
- **THEN** Forge records the immutable revision, the provider's `container_identity`, and the terminal stage health evidence; `phase` advances to `stage-healthy`

#### Scenario: Stage health is unhealthy

- **WHEN** the provider's `publish` operation reports `build_status=failed` or `run_status=failed`, or the stage verify returns unhealthy
- **THEN** Forge records `state=failed` under `delivery.stage`, surfaces `delivery-unavailable`, transitions `phase` to `stage-failed`, and refuses promotion

#### Scenario: Required preflight missing

- **WHEN** the user invokes `stage` without a prior terminal `delivery.preflight` row for the same revision, or with a stale `confirm_operation_id`
- **THEN** Forge refuses with `delivery-invalid` and writes 0 bytes to stdout

## Traceability

| Requirement/scenarios | Proposal outcome | UX flow | Design | Implementation boundary | Tasks | Verification oracle |
|---|---|---|---|---|---|---|
| Evidence-gated staged delivery; current PASS success, missing/non-pass refusal, source mismatch, invalid evidence, provider failure, stage regression | Gate and source become explicit preflight preconditions | Existing CLI/API/workbench preflight journey with actionable refusal recovery | Ordered precondition table; shared freshness classifier; no implicit Gate | `src/delivery/handlers.rs::run_preflight`, `src/gate/engine.rs::load_latest_evidence` and `evidence_freshness` | 1.1–1.3, 2.1–2.4, 3.1–3.3, 4.1–4.4 | Delivery contract, cross-surface/API/browser tests; `cargo fmt --check`; local Gate |
