## 1. BFS — Baseline and impact coverage

- [x] 1.1 Map the original Gate failures and stale profile, dry-run, exemption, platform-result, and source-size assertions surfaced by serial workspace runs to the current resolver, runtime, and canonical Forge Gate contract; retain the failing command and assertion evidence.
- [x] 1.2 Confirm the distinct persistence callers: direct DriftWatchdog `gate()` and Forge's `gate_for_forge()` adapter.
- [x] 1.3 Confirm TOML, YAML, built-in profiles, project-defined profiles, dry-run rendering, and workspace-governance's read-only Gate oracle are affected or explicitly out of scope.
- [x] 1.4 Finalize the `forge-native-gate` requirement delta and implementation boundary before editing tests.

## 2. DFS — Requirement-by-requirement implementation

- [x] 2.1 Update no-manifest and foreign-runtime tests to assert the direct DriftWatchdog API runs the Forge baseline and persists one passing run.
- [x] 2.2 Add or update a regression assertion that `gate_for_forge()` runs the baseline without creating DriftWatchdog persistent state.
- [x] 2.3 Align concern, YAML adapter, domain-profile, deployable-profile, governance-quality, platform-result, source-size, and exemption-cap integration tests with mandatory `source-file-size` and explicitly selected `governance-quality` semantics.
- [x] 2.4 Correct unit and CLI dry-run fixtures to include a genuinely unbound required extension and assert it—not a removed profile default—forces the block prediction.
- [x] 2.5 Add the `forge-native-gate` requirement delta for profile baseline and extension selection semantics.
- [x] 2.6 Reconcile the CLI catalog with all user-facing Clap paths (`use`) and exclude the hidden gate-engine dispatch; capture that contract in `forge-web-command-catalog`.
- [x] 2.7 Resolve the packaging example test's binary path from the Cargo workspace target directory.
- [x] 2.8 Replace the retired external-runtime CLI contract assertions with Forge-native baseline, extension, evidence, journal, freshness, rehearsal, and refusal coverage.
- [x] 2.9 Separate optional DriftWatchdog export-producer resolution from the Forge-native Gate runtime and update cross-surface runtime attribution expectations.
- [x] 2.10 Reconcile kit generation equivalence tests with the documented real `generated_at` provenance field while preserving byte checks for all other output and receipt content.
- [x] 2.11 Fix the serial suite's 390px lifecycle browser overflow in mobile workbench grid sizing and empty workspace summary layout; capture the existing responsive contract in `portal-web-ui`.
- [x] 2.12 Update the standard cross-surface contract test to assert the service-profile default pack already required by the canonical standard-pack spec.
- [x] 2.13 Update the second command-catalog row-count oracle for the newly documented user-facing `forge use` row.
- [x] 2.14 Select a verified pack compatible with the remediation test's `rust-web` fixture instead of relying on descriptor catalog order.
- [x] 2.15 Update the delivery-history catalog conversion oracle to include the newly documented `forge use` row.
- [x] 2.16 Reconcile workspace-metadata tests and canonical `deterministic-project-generation` opt-out scenario with default standard-pack output, comparing paired renders outside metadata and the timestamp provenance field.
- [x] 2.17 Move the export-only binary resolver out of `src/gate/evidence.rs` to restore the mandatory 1,000-line source-size limit without coupling it to Forge Gate execution.

## 3. BFS — Cross-surface regression and completeness

- [x] 3.1 Re-run the original failures and all stale integration assertions surfaced by the full suite; inspect plan rows, exit status, persistence, and dry-run predictions.
- [x] 3.2 Re-run the entire DriftWatchdog workspace suite and check for stale assertions in all touched callers and fixtures.
- [x] 3.3 Confirm no production Gate code or sibling repository was changed unless evidence proves a real spec violation; retain `source-file-size` as mandatory.

## 4. Verification

- [x] 4.1 Run focused DriftWatchdog Gate/manifest/YAML/dry-run tests, Rust formatting, and the workspace build; record exact results.
- [x] 4.2 Run `RUSTC_WRAPPER= CARGO_INCREMENTAL=0 timeout 600 scripts/check-tests.sh`; then rehearse and run the full Forge Gate with a bounded timeout. The serial workspace suite passed; Gate dry-run passed; full Gate aggregate passed with 419/419 source-size candidates.
- [x] 4.3 Run OpenSpec name preflight, strict validation, `git diff --check`, archive without `--skip-specs`, and review the promoted spec.
- [x] 4.4 Commit the related tests, promoted specs, and archive; record Gate evidence in HANDOFF, commit HANDOFF separately, then stop without pushing.
