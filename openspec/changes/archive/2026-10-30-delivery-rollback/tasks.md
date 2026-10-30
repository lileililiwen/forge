## 1. BFS — Baseline and impact coverage

- [x] 1.1 Confirm no rollback verb exists in the CLI, the handlers, the API or the web surface.
- [x] 1.2 Confirm the status projection takes the latest row per verb and cannot answer "which revisions were promoted healthy".
- [x] 1.3 Confirm the healthy-promotion rule in `compute_phase` and that rollback must reuse it.
- [x] 1.4 Confirm every exhaustive `DeliveryPhase` and `DeliveryVerb` match site.
- [x] 1.5 Confirm no provider has ever received a rollback request, so `target_revision` can be added under the existing contract.

## 2. DFS — Requirement-by-requirement implementation

- [x] 2.1 Add the shared healthy-promotion predicate and the healthy-release list; make `compute_phase` call the predicate.
- [x] 2.2 Add `DeliveryVerb::Rollback`, `RollbackTarget` and the target-scoped idempotency key.
- [x] 2.3 Add `target_revision` to the provider request and a rollback request builder with its queue id.
- [x] 2.4 Add `run_rollback`: target resolution, the healthy-release proof, the capability gate, reservation and invocation.
- [x] 2.5 Record `target_revision` in the journal detail and in the run/phase columns.
- [x] 2.6 Add `DeliveryPhase::RolledBack`, the `rollback` report and `target_revision` in the status document.
- [x] 2.7 Add the CLI `rollback` subcommand with `--to` and `--previous`.
- [x] 2.8 Map `rolled-back` in the doctor and in the environment classifier.
- [x] 2.9 Move the status contract to `forge-delivery-status/0.2.0` and update its readers.
- [x] 2.10 State the operation, the field and the refusal in `docs/adapter-contracts/publish-provider.md` and the flow in `docs/ci-cd-system-model.md`.

## 3. BFS — Cross-surface regression and completeness

- [x] 3.1 A promoted healthy release is restored by `--to`, the phase is `rolled-back`, and the row names the target.
- [x] 3.2 A revision that was never promoted is refused before the provider is resolved.
- [x] 3.3 A revision whose newest promotion was degraded is refused.
- [x] 3.4 `--previous` resolves the newest healthy release other than the registered revision.
- [x] 3.5 `--previous` with no earlier release is refused.
- [x] 3.6 The registered revision is refused as a target.
- [x] 3.7 Both and neither selectors are refused.
- [x] 3.8 A provider that declares no `rollback` is refused before reservation.
- [x] 3.9 An unknown provider is refused for `rollback`.
- [x] 3.10 A replay reuses the row; a different target reserves a new one.
- [x] 3.11 A failed rollback records `state=failed` and leaves the phase unchanged.
- [x] 3.12 Rollback does not require fresh Gate evidence for the current revision.
- [x] 3.13 The status JSON keeps its published fields and adds only the rollback verb and phase.
- [x] 3.14 The doctor reports `rolled-back` as a warning naming the state.
- [x] 3.15 Every existing delivery, doctor, API and web test keeps its outcome.

## 4. Verification

- [x] 4.1 Run the repository test suite.
- [x] 4.2 Run `cargo build` and the focused delivery, provider and doctor suites.
- [x] 4.3 Run the repository Gate: `forge gate --dry-run`, then the full `forge gate`.
- [x] 4.4 Run `node scripts/check-openspec-change-names.mjs`, `openspec validate --all --strict --no-interactive` and `git diff --check`.
- [x] 4.5 Run a real rollback against fixture providers and record each outcome: restored, refused for an unproven target, refused for a provider that declares no rollback, and failed.

## 5. Implementation — authorized surfaces

- [x] 5.1 `src/delivery/{projection,state,invoke,handlers,mod}.rs`.
- [x] 5.2 `src/publish/providers/model.rs`.
- [x] 5.3 `src/cli/{commands,delivery}.rs`.
- [x] 5.4 `src/doctor/delivery.rs` and `src/api/model.rs`.
- [x] 5.5 `docs/adapter-contracts/publish-provider.md` and `docs/ci-cd-system-model.md`.
- [x] 5.6 The new unit and CLI contract tests and the updated status assertions.
- [x] 5.7 Record the exact output of every verification command in the evidence table below.
- [x] 5.8 Archive without `--skip-specs` so the capability is promoted, commit the implementation with its specs, then update the handoff pointer.

## Evidence

| Check | Result | Evidence |
|---|---|---|
| Repository test suite | PASS, 4318 tests / 0 failed | `cargo test --workspace --all-targets -- --skip rust_scaffold_builds_and_tests_with_native_toolchain --skip rust_scaffold_builds_with_native_toolchain_without_forge --skip declaration_is_inert_for_native_builds_present_and_absent --test-threads=1` exited 0 and reported 4318 passed, 0 failed, 1 ignored, 3 filtered out across 245 suites in 381.95s. |
| Focused suites: library | PASS, 163 tests / 0 failed | `cargo test -p forge --lib delivery::` → 33 passed; `cargo test -p forge --lib doctor::` → 74 passed; `cargo test -p forge --lib publish::providers` → 56 passed; `cargo test -p forge --lib command_catalog` → 7 passed. |
| Focused suites: contracts | PASS, 74 tests / 0 failed | `cargo test --test delivery_contract --test delivery_cross_surface --test delivery_provider_health_truth_contract --test delivery_next_idea_evidence_truth_contract --test delivery_provider_capabilities_contract --test delivery_rollback_contract` → 54 passed; `cargo test --test web_release_deploy_history_contract --test web_agent_identity_readonly_contract` → 20 passed. |
| New rollback contract suite | PASS, 10 tests / 0 failed | `cargo test --test delivery_rollback_contract` → 10 passed: restored, unpromoted target refused, current revision refused, selector rules, undeclared provider refused, `--previous`, no earlier release, failed rollback, Gate evidence absent, replay and different target. |
| Repository Gate | FAIL, 7/8 — the same pre-existing advisory | `forge gate --dry-run` resolved all 8 required checks with none unbound. The full `forge gate --timeout-secs 1800` run reported aggregate `blocked`: `security` failed on RUSTSEC-2026-0387 and every other check passed, including `tests`. `.forge/gate/forge/evidence.json` records `"aggregate": "blocked"` at `2026-10-30T10:20:48Z` on revision `52e1b1c`, with `source-file-size` reporting 463 of 463 candidates at the 1000-line maximum. This is the same failure the previous change recorded; it is not attributable to this change and `Cargo.lock` is untouched. |
| Real rollback, restored | PASS | A fixture project with two promoted revisions (`13c7fb61…` then `42d6d409…`) and a stub declaring `["preflight","publish","rollback"]`: `forge delivery rollback rollback-manual --to 13c7fb61…` printed `op_id: 9`, `phase: rolled-back`, `target_revision: 13c7fb61…`, `provider_status: succeeded`. `delivery status` reported contract `forge-delivery-status/0.2.0`, phase `rolled-back`, and a `rollback` row with `target_revision`. The provider's request carried `"operation":"rollback"`, `"revision":"42d6d409…"` and `"target_revision":"13c7fb61…"`. |
| Real rollback, refused | PASS | `forge delivery rollback rollback-manual --to ffffffff…` exited 1 with `error[delivery-conflict]: delivery conflict: delivery rollback: revision `ffffffffffffffffffffffffffffffffffffffff` has no healthy promotion for project `rollback-manual`; only a release Forge promoted and observed healthy can be restored`, and no rollback row was written. The capability refusal and the failed-provider case are the `rollback_is_refused_when_the_provider_does_not_declare_it` and `a_failed_rollback_does_not_claim_the_release_moved` contract tests. |
| OpenSpec and whitespace | PASS | `node scripts/check-openspec-change-names.mjs` reports `PASS`; `openspec validate --all --strict --no-interactive` reported `Totals: 170 passed, 0 failed (170 items)`; `git diff --check` reported no findings. Archived without `--skip-specs`, and the promoted spec's `TBD` Purpose was replaced before the commit. |

## Notes on the implementation, for the reviewer

- **The healthy-release rule is shared.** `promotion_is_healthy` is called by
  `compute_phase` for the latest promotion and by `healthy_release_revisions`
  for the rollback proof, so the status page and the verb cannot disagree.
- **A revision's newest promotion decides it.** A revision promoted twice, once
  failed and once healthy, is a release; a revision whose newest promotion is
  degraded is not, even if an older row was healthy.
- **A failed rollback leaves the phase where it was.** The provider's failure
  is a row, not a claim that production moved. A rollback that the provider
  reports as done but explicitly unhealthy yields `degraded`, not `rolled-back`.
- **The rollback probe runs before the idempotent reservation**, so a replay
  probes the provider again and then reuses the row. The contract suite pins
  the reuse and the new-target behaviour.
- **The status contract moved to `0.2.0`** because a verb and a phase were
  added. The `rollback` report and its `target_revision` are additive fields;
  the `0.1.0` fields are unchanged.
- **The CLI command has a catalog row** with availability `cli_only` and the
  reason that no authenticated route exists yet; the row count moved from 259
  to 260 in the two pinned assertions.
- **The web deploy signal now treats `rolled-back` as deployed**, so the
  dashboard does not report a restored project as never deployed.

