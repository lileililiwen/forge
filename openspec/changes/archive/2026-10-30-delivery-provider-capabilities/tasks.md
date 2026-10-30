## 1. BFS — Baseline and impact coverage

- [x] 1.1 Confirm the provider contract declares `capabilities`, `verify` and `rollback`, and that Forge constructs only `preflight` and `publish` outside tests.
- [x] 1.2 Confirm the response type has no field in which a provider can declare a capability set.
- [x] 1.3 Confirm the delivery verb order in `run_preflight`, `run_stage` and `run_promote`, and where a refusal can sit without writing a journal row.
- [x] 1.4 Confirm every existing delivery test stub answers without a `capabilities` field, so the unknown path is the compatibility path.
- [x] 1.5 Confirm the contract's additive-field rule allows a response field without a version bump.

## 2. DFS — Requirement-by-requirement implementation

- [x] 2.1 Give `ProviderOperation` `ALL`, `as_str` and `parse`, and remove the test-local `as_str`.
- [x] 2.2 Add `PublishProviderResponse.capabilities` as an optional, additive list.
- [x] 2.3 Validate the declaration: count, name shape, duplicates; unknown names ignored.
- [x] 2.4 Add `CapabilityDeclaration` with `Declared`/`Unknown`, the permission rule, the stable summary and the bounded detail value.
- [x] 2.5 Add `probe_capabilities` to the provider engine: build the request, run it, read a declaration, and treat a non-answer as unknown.
- [x] 2.6 Add `operation_for(verb)` to the delivery invoke module and reuse it in `build_request`.
- [x] 2.7 Probe, gate and record in `run_preflight`, `run_stage` and `run_promote`, refusing before the operation is reserved.
- [x] 2.8 Record the declaration on every verb's journal detail under `provider_capabilities`.
- [x] 2.9 State the operation, the field and the refusal in `docs/adapter-contracts/publish-provider.md` and the delivery seam in `docs/ci-cd-system-model.md`.

## 3. BFS — Cross-surface regression and completeness

- [x] 3.1 A provider declaring `preflight` and `publish` is served by preflight, stage and promote.
- [x] 3.2 A provider declaring only `preflight` is refused at stage, with the provider id, the required verb and the declared set in the sentence.
- [x] 3.3 A refused stage writes no `delivery.stage` row and leaves the phase at `awaiting-stage-confirmation`.
- [x] 3.4 A provider that answers no declaration is unknown and is served by preflight and stage.
- [x] 3.5 A provider that refuses the probe (non-zero exit) is unknown and is served.
- [x] 3.6 A malformed declaration is refused at preflight, names the reason, writes no row and does not run the requested verb.
- [x] 3.7 The probe is side-effect-free: it carries no `queue_id`, reserves nothing and changes no phase.
- [x] 3.8 Promote is gated by the declaration the provider makes now, and the refusal names promote.
- [x] 3.9 Every existing delivery, publish-provider and health test keeps its outcome.
- [x] 3.10 The response field, the request shape and the contract version are unchanged for a provider that does not declare anything.

## 4. Verification

- [x] 4.1 Run the repository test suite.
- [x] 4.2 Run `cargo build` and the focused suites for the providers module and the delivery lane.
- [x] 4.3 Run the repository Gate: `forge gate --dry-run`, then the full `forge gate`.
- [x] 4.4 Run `node scripts/check-openspec-change-names.mjs`, `openspec validate --all --strict --no-interactive` and `git diff --check`.
- [x] 4.5 Run a real delivery against fixture providers and record each outcome: declared-and-served, declared-without-publish refused, unknown served, malformed refused.

## 5. Implementation — authorized surfaces

- [x] 5.1 `src/publish/providers/{model,capabilities,engine,engine_tests}.rs`.
- [x] 5.2 `src/delivery/{capabilities,invoke,handlers}.rs`.
- [x] 5.3 `docs/adapter-contracts/publish-provider.md` and `docs/ci-cd-system-model.md`.
- [x] 5.4 The new unit and CLI contract tests.
- [x] 5.5 Record the exact output of every verification command in the evidence table below.
- [x] 5.6 Archive without `--skip-specs` so the capability is promoted, commit the implementation with its specs, then update the handoff pointer.

## Evidence

| Check | Result | Evidence |
|---|---|---|
| Repository test suite | PASS, 4301 tests / 0 failed | `cargo test --workspace --all-targets -- --skip rust_scaffold_builds_and_tests_with_native_toolchain --skip rust_scaffold_builds_with_native_toolchain_without_forge --skip declaration_is_inert_for_native_builds_present_and_absent --test-threads=1` exited 0 and reported **4301 passed, 0 failed, 1 ignored, 3 filtered out** across 244 suites in 425.30s. |
| Focused suite: providers | PASS, 55 tests / 0 failed | `cargo test -p forge --lib publish::providers` → 55 passed, 0 failed (1434 filtered out). |
| Focused suite: delivery | PASS, 27 tests / 0 failed | `cargo test -p forge --lib delivery::` → 27 passed, 0 failed (1462 filtered out). |
| New CLI contract suite | PASS, 6 tests / 0 failed | `cargo test --test delivery_provider_capabilities_contract` → 6 passed: declared-and-served, declared-without-publish refused, absent served, probe-refused served, malformed refused, promote gated on the current declaration. |
| Existing delivery suites | PASS, 47 tests / 0 failed | `cargo test --test delivery_contract --test delivery_cross_surface --test delivery_provider_health_truth_contract` → 35 passed across 3 suites; `cargo test --test forge_web_project_delivery_contract` → 12 passed after its call counter was taught to separate the probe from a delivery operation. |
| Repository Gate | FAIL, 7/8 — one pre-existing advisory | `forge gate --dry-run` resolved all 8 required checks with none unbound. The first full `forge gate --timeout-secs 1800` run reported aggregate `blocked`: `tests` failed with exit 101 during the cold run whose build check was still compiling, and `security` failed on RUSTSEC-2026-0387. The second run, with a warm build, reported `tests` pass (and the same command run directly, `bash scripts/check-tests.sh`, exited 0) and `security` still failed, so the aggregate stayed `blocked`. `.forge/gate/forge/evidence.json` records `"aggregate": "blocked"` at `2026-10-30T08:55:31Z` on revision `295a692`, with `security` in state `fail` and every other check `pass`; `source-file-size` reported 463 of 463 candidates, maximum 1000 lines. |
| The Gate's one failure, attributed | PRE-EXISTING, not this change | `cargo audit` reports **RUSTSEC-2026-0387** (`crypto-bigint` 0.5.5, published 2026-10-23, "multiple timing variability issues in operations intended to be constant-time"). The dependency is transitive: `openidconnect` 4.0.1 → `elliptic-curve` 0.13.8 → `crypto-bigint` 0.5.5 (`cargo tree -i crypto-bigint`). This change does not touch `Cargo.toml` or `Cargo.lock`, and 4.0.1 is the latest `openidconnect` (`cargo search openidconnect`), so no compatible patched release exists. `cargo update -p crypto-bigint/elliptic-curve/openidconnect --dry-run` locks 0 packages. Remediation path: upstream `openidconnect` must move to `elliptic-curve` 0.14 / `crypto-bigint` ≥ 0.7.6, or a separate security-policy change must justify an `.cargo/audit.toml` ignore with the same reasoning the file already records for RUSTSEC-2023-0071. This change introduces no new failure. |
| Real delivery, declared and refused | PASS | A fixture project with a stub provider declaring `["preflight"]` only: `forge delivery preflight caps-manual` recorded op 2, `phase: awaiting-stage-confirmation`. `forge delivery stage caps-manual --confirm-operation-id 2` exited 1 with `error[delivery-invalid]: delivery invalid: delivery stage: publish provider `openpanel` does not declare the `publish` capability (declared: preflight); refused before the requested verb ran`. The stub's last request was `"operation":"capabilities"`, the journal held no `delivery.stage` row, and `delivery status` still reported `awaiting-stage-confirmation`. |
| Real delivery, declared and served | PASS | A second fixture project with a stub declaring `["preflight","publish"]`: preflight recorded op 4, stage recorded op 5, `delivery status` advanced to `awaiting-production-approval`, and the stage row's detail carried `{"operations": ["preflight", "publish"], "state": "declared"}`. |
| OpenSpec and whitespace | PASS | `node scripts/check-openspec-change-names.mjs` reports `PASS`; `openspec validate --all --strict --no-interactive` reported `Totals: 169 passed, 0 failed (169 items)`; `git diff --check` reported no findings. |

## Notes on the implementation, for the reviewer

- **The gate sits before the operation is reserved**, so a refused verb writes
  no row, changes no phase and runs no requested operation. The probe itself
  is a provider process, so the accurate claim is "the requested verb never
  ran", not "the provider never ran"; the spec and the refusal sentence say
  the former.
- **Unknown permits `preflight` and `publish`.** Those are the two operations
  every `0.1.0` provider has served; refusing them would turn a missing
  optional declaration into a broken deployment. `verify` and `rollback` are
  already refused for an unknown provider, which is the guard the next change
  needs.
- **Each verb probes for itself.** A declaration read from an earlier verb
  would be stale; the `caps-narrowing` fixture proves a provider that narrows
  its declaration between stage and promote is refused at promote.
- **A response that fails to parse is unknown, not malformed.** The malformed
  path is reserved for a response that parsed as a provider response and then
  carried a bad list (count, name shape, duplicate). A wrong-typed
  `capabilities` field makes the whole response unparseable and is therefore a
  non-claim, which the contract document states.
- **`ProviderOperation` now derives `Ord`** so the recorded list is stable
  and the `BTreeSet` summary cannot reorder between runs.
- **No wire request changed.** The probe sends the existing request shape with
  no `queue_id`; a provider that does not understand it answers non-zero and
  is unknown.
