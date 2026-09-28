# Tasks: portfolio-activation-readiness

Contract: `forge-portfolio-activation/0.1.0`. Read `design.md` for the
exact types, grammar, JSON shapes, reason order and named test list; this
file is the execution order.

## 1. BFS — Baseline and impact coverage

- [ ] 1.1 Capture the pre-change baselines before editing anything: `cargo fmt --all -- --check` drift locations and `cargo clippy --all-targets -- -D warnings` locations, using `git stash push -u -- src tests` and a `comm` diff. Record the exact pre-change test counts (`cargo test --workspace --all-targets -- --skip rust_scaffold_builds_and_tests_with_native_toolchain`).
- [ ] 1.2 Read `src/portfolio/interest/{mod,compare}.rs`, `src/portfolio/interest_report.rs`, `src/registry/interest/mod.rs`, the interest handlers in `src/api/mod.rs`, and `cmd_fleet_online` in `src/main.rs`. Confirm: `Freshness`, `PrivacyMode::is_exact`, `Coverage`, `InterestMetric`, `SnapshotState::is_current`, `bound_stale_after_days`, `normalize_timestamp`, `MAX_METRIC_VALUE`, `DEFAULT_STALE_AFTER_DAYS`, `interest_require_project`/`require_project`, `as_output`, and the `{"interest": {...}}` envelope.
- [ ] 1.3 Confirm the impact map: no schema change, no migration, no new dependency, no MCP tool, no portal change. Record the Forge/product ownership split and the `paid_interest_events` boundary in the change notes.
- [ ] 1.4 Do not leave a placeholder: `RequestedWindow`, `ActivationReport`, `ReadinessVerdict`, `ReadinessReason`, `ReadinessEvidence`, `NotReadyReason` and `Readiness` must all be implemented, never stubbed or deferred.

## 2. DFS — Requirement-by-requirement implementation

- [ ] 2.1 Add `src/portfolio/interest/activation.rs` with `ACTIVATION_CONTRACT_VERSION`, `Readiness`, `NotReadyReason` (declaration order = report order), `ReadinessReason`, `RequestedWindow`, `ReadinessEvidence`, `ReadinessVerdict`, `ActivationReport`, and `ActivationReport::is_ready` / `verdict_reason_labels`.
- [ ] 2.2 Implement `validate_threshold` (bound `0..=MAX_METRIC_VALUE`) and `parse_window` (split on `..`, `normalize_timestamp` both sides, require `end > start`) with their exact refusal strings.
- [ ] 2.3 Implement `build_readiness` with the documented candidate-selection rule (current snapshots reporting the metric, optional source and normalized-window filters, maximum by `(window_end, id)`, no fallback) and the eight-condition evaluation table reporting every held condition in the fixed order.
- [ ] 2.4 Add `Registry::interest_snapshot_counts` returning `(total, current)` by two `COUNT(*)` queries, so `no-evidence` is distinguishable from `superseded-only`.
- [ ] 2.5 Add `ForgeError::PortfolioActivationNotReady` with code `portfolio-activation-not-ready`, and its `err_status` row, documented as CLI-gate-only.
- [ ] 2.6 Add `activation_readiness` to `src/portfolio/interest_report.rs`: bound the staleness value, `require_project` every id before reading, refuse an empty id list, read counts then current snapshots, and build the report with verdicts ordered by project id.
- [ ] 2.7 Add `PortfolioActivationCommands::Readiness` and `PortfolioCommands::Activation` with the exact flags and defaults from the design, plus `cmd_portfolio_activation` implementing the `cmd_fleet_online` print-then-error gate pattern, the human layout and the JSON envelope.
- [ ] 2.8 Add `Route::InterestReadiness` (`GET /v1/interest/readiness`) with its matcher, `admin:access` permission arm, fleet `authorize` arm, dispatch arm, `handle_interest_readiness`, and strict query parsing (`project` xor `projects`, required `metric`, bounded `min-value` and `stale_after_days`, optional `source` and `window`, unknown parameter refused). Always answer `200`.
- [ ] 2.9 Add the unit tests named in `design.md` under "Verification oracle", including the `interest_snapshot_counts` test.
- [ ] 2.10 Add `tests/portfolio_activation_cli_contract.rs` with every named CLI and in-process-API test, including the gate exit code, the empty-stdout input refusals, the reason reachability sweep, the fleet ordering and the admin gating.

## 3. BFS — Cross-surface regression and completeness

- [ ] 3.1 Add `tests/portfolio_activation_cross_surface.rs`: prove every table's row counts, the journal count and the registry bytes are unchanged by a readiness run, and that a registry written before this package is read as-is.
- [ ] 3.2 Prove no activation or billing field reaches the public share manifest, the private portfolio projection, the fleet list or any readiness-independent projection, and that no price/plan/subscription/entitlement/checkout/revenue field exists anywhere in Forge.
- [ ] 3.3 Prove `paid_interest_events` remains an aggregate signal that grants no access, and that existing interest, share and portfolio contract suites pass without edits.
- [ ] 3.4 Exercise the boundary cases end to end: superseded-only, all-stale, `lower-bound`, `undeclared`, `partial`, no evidence, a value exactly at the threshold, a value one below it, two sources sharing the latest window, and a declared window that matches nothing.

## 4. Verification

- [ ] 4.1 Run formatting, build, clippy, focused unit tests, the two new suites, the full suite, the name preflight, strict OpenSpec validation and `git diff --check`. Diff `cargo fmt` and `cargo clippy` against the captured baselines and confirm zero new locations; restore any incidentally reformatted pre-existing file with `git checkout --`.
- [ ] 4.2 Run the live smoke paths and record them: the CLI `ready` case (exit 0), the CLI `not-ready` case (report on stdout, typed error on stderr, exit non-zero), each input refusal with empty stdout, and the loopback `forge api serve` route returning `200` for both verdicts plus `401` without a bearer.
- [ ] 4.3 Record that no analytics provider was contacted, no product was activated, and product conversion evidence stays external; this package claims no revenue outcome.
- [ ] 4.4 Confirm no billing, entitlement, checkout or payment record was added or began, and that Forge still declares `billing: blocked`.
- [ ] 4.5 Set the single `current_spec` pointer to this change before implementing, update it in `HANDOFF.md` with the evidence, and archive only after every requirement above is evidenced.
