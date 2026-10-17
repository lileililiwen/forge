# Proposal: Reconcile DriftWatchdog Gate regressions with Forge's baseline

## Why

Forge's own Gate is blocked by stale assertions in its in-tree `driftwatchdog`
crate and in the CLI integration suite. Many assertions still expect the former implicit `governance-quality`
default, while the active Forge contract injects mandatory `source-file-size`
and treats governance checks as explicit project extensions; two runtime tests
also expect no persistence after a successful baseline run. Fix the contract
drift now so Forge's required local Gate can report a trustworthy result.

## What Changes

- Update regression tests to assert that the Forge baseline is scheduled once,
  required, and not counted as an unbound command, while
  `governance-quality` is selected only when a project explicitly declares it.
- Correct dry-run fixtures so a required unbound extension actually exists
  and is the reason the plan predicts a block.
- Align direct `driftwatchdog::commands::gate` tests with its current behavior:
  it runs and records the baseline in DriftWatchdog's store when a manifest is
  absent or declares a foreign runtime. Keep Forge's separate
  `gate_for_forge` in-memory persistence boundary unchanged.
- Change production behavior only if a corrected contract test exposes a real
  deviation from the canonical Forge Gate spec; do not weaken or suppress the
  mandatory baseline to make the suite green.
- Move this remediation ahead of binary distribution in the consolidation
  roadmap because the current failures block Forge's own Gate.
- Reconcile packaging binary lookup with Cargo's workspace target directory,
  and keep the operator command catalog complete for the `forge use` facade
  while excluding the hidden internal gate-engine dispatch.
- Replace the retired external-runtime evidence integration tests with
  Forge-native CLI contract coverage for baseline and extension verdicts,
  evidence, status freshness, rehearsal purity, and refusals.
- Keep the optional DriftWatchdog `gate evidence-export` producer bound to
  `forge gate evidence` only; the bundled Forge gate engine remains the sole
  implementation of `forge gate <TARGET>`.
- Fix the existing 390px lifecycle browser failure by constraining mobile
  workbench grid tracks and keeping the empty workspace badge from widening
  the top bar; retain precise overflow diagnostics in the browser harness.
- Update the standard cross-surface test's stale assumption that `forge new`
  omits the default pack; assert the already specified service-profile pack
  and preserve the independent registry/profile checks.
- Make the web assurance remediation test select by pack compatibility and
  support state instead of assuming the first catalog descriptor fits its
  `rust-web` project.
- Replace stale pre-standard-pack workspace-metadata output snapshots with
  paired renders that prove `--no-workspace-metadata` removes only the
  declaration and receipt, and specify that current parity boundary.
- Keep the export-only resolver under the mandatory source-file-size cap by
  moving it to a focused private Gate module after the full Gate exposed the
  new over-limit file.
- Correct the two kit generation equivalence tests surfaced by the full
  serial suite: compare all generated content and receipt fields while
  treating the documented real generation timestamp as provenance metadata.

## Package Boundary and Split Assessment

| Package | Single outcome | Owner/project and language | Boundary/contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `forge-driftwatchdog-test-reconciliation` | Make the full serial workspace suite and Forge Gate reflect already delivered behavior | Forge Rust workspace; DriftWatchdog and Forge contract tests, packaging harness, command catalog and affected canonical specs | Required `source-file-size` baseline is injected once; project checks such as `governance-quality` are opt-in; direct DriftWatchdog persists its run, Forge uses its no-persistence adapter; generation equivalence excludes only real timestamp provenance | `forge-native-gate` (`2026-10-17-forge-native-gate-checks`, already archived) and deterministic project generation | All stale assertions pass; `scripts/check-tests.sh` and full Forge Gate pass |

The Gate failures shared one cause and owner: stale test assumptions about the
baseline resolution contract. The required full serial suite then surfaced two
additional stale byte-equality assertions: generated ownership receipts
intentionally carry the actual `generated_at` timestamp, while project
equivalence excludes provenance metadata. Both causes are test-contract drift
and are reconciled here without changing generation or Gate runtime behavior.
Portable binary distribution has an independent release lifecycle and no
prerequisite relationship to this test reconciliation.

**Phase:** implementation. **Source requirement:** `requirement.md` §24,
Forge Gate.

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| Forge bundled gate engine | `src/main.rs` dispatches to `driftwatchdog::commands::gate::gate_for_forge`; `crates/driftwatchdog/src/gate/manifest.rs::resolve` injects the baseline | Existing baseline, planner, direct DriftWatchdog persistence path, and Forge-specific in-memory adapter | Failing tests still assert pre-baseline semantics | Forge workspace owns `forge-native-gate`; the crate remains an ordinary in-tree workspace member | **keep local** |
| Workspace Governance | `../workspace-governance/scripts/bind_gate_commands.py` and its `GateOracleTest` bind workspace checks and compare declarations to the actual Gate oracle | Existing command binding and profile-consistency tests | It configures project checks; it does not own Forge's mandatory baseline or the bundled engine's persistence split | Workspace Governance remains a separate sibling owner; this change does not edit it | **keep local** |

No shared implementation is needed: the defect is in Forge's internal test
fixtures and assertions. The sibling binder remains a consumer of declared
project checks, not the owner of Forge's gate contract. No sibling files will
be changed.

## User Experience and Interface Impact

**UI/UX: N/A.** This change adds no interactive capability or screen. The
operational actors are Forge maintainers and the local Gate runner; they use
existing CLI commands. The user-visible result is a truthful passing/blocked
Gate and readable per-check output. No route, browser, device, responsive, or
accessibility surface changes.

## BFS Impact Map

- **Requirements:** existing canonical `forge-native-gate` mandatory-baseline
  and project-extension requirements; clarify profile selection and runtime
  ownership scenarios without changing the source-size policy.
- **Modules:** `crates/driftwatchdog/src/commands/gate_tests.rs`,
  `src/gate/concerns.rs` tests, `src/gate/aigate.rs` tests,
  `src/gate/dry_run_plan_tests.rs`, `src/gate/manifest.rs` resolution tests,
  and the `gate_deployable.rs`, `gate_dry_run_json.rs`, and
  `gate_exemption_cap.rs`, `gate_governance_quality.rs`, and
  `gate_platform_result.rs`, and `gate_source_file_size.rs` integration
  tests, plus the workspace target-path resolution in
  `tests/packaging/test_agent_examples.sh` required by the serial suite, and
  `tests/kit_contract/tokens_and_assets.rs` where rendered trees normalize
  only `.standard/receipt.json::generated_at`; all other bytes and receipt
  fields remain part of the oracle.
  No production Gate behavior is expected to change.
- **Callers:** Forge `gate_for_forge`, DriftWatchdog's direct `gate`, TOML and
  YAML manifest resolution, built-in profile selection, and dry-run rendering.
- **Persistence:** direct `gate()` records one successful baseline run in the
  DriftWatchdog store; `gate_for_forge()` continues using an in-memory store
  so Forge alone persists public evidence.
- **Failure/boundaries:** an undeclared required extension remains unbound and
  blocking; the in-process baseline is not unbound; no manifest and a foreign
  runtime declaration still run the Forge baseline; dry-run never persists.
- **Compatibility:** no manifest schema, gate-result version, baseline
  threshold, or project-facing configuration changes.
- **Tests:** targeted failed cases, DriftWatchdog crate suite, workspace test
  script, kit equivalence, 390px Chromium lifecycle flow, Forge Gate dry-run
  and full run, strict OpenSpec, formatting and diff checks.
- **Security:** no command execution policy, credentials, filesystem boundary,
  network, or database schema changes.

## Capabilities

### New Capabilities

None. This reconciles tests and clarifies an already implemented capability.

### Modified Capabilities

- `forge-native-gate`: specify that the mandatory baseline remains present
  across profile selection and governance checks are project-owned extensions;
  preserve Forge's evidence ownership at the public entry point.
- `forge-web-command-catalog`: cover all user-facing Clap paths, including
  `forge use`, while omitting hidden internal dispatch paths.
- `gate-evidence-export-consumption`: keep optional release evidence export
  resolution separate from the Forge-native gate runner.
- `portal-web-ui`: retain the 390px no-page-overflow workbench contract when
  the workbench contains intrinsic-width content and lifecycle rail steps.

## Non-goals

- Changing Gate implementation, baseline contents, line threshold, or project
  extension semantics unless a corrected test proves a contract violation.
- Replacing the in-tree DriftWatchdog engine or reviving its archived origin
  repository.
- Editing Workspace Governance or any sibling repository.
- Publishing binaries, changing release workflows, migrating libraries, or
  changing generated-project templates.
- Treating a green test count as sufficient when the full Gate still fails.
