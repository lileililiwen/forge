# Design: Reconcile DriftWatchdog Gate regression tests

## Context

The Forge Gate implementation was consolidated in `d7c5f96` and its
canonical contract is `openspec/specs/forge-native-gate/spec.md`. The planner
now injects one required `source-file-size` row for every resolved manifest;
`governance-quality` is an explicit project extension, not a profile default.
The direct `driftwatchdog::commands::gate::gate` API persists to DriftWatchdog
storage. Forge calls `gate_for_forge`, which executes with an in-memory store
so Forge alone owns public evidence persistence. The ten suite failures retain
expectations from before these changes.

## Goals / Non-Goals

**Goals:**

- Make the in-tree tests assert the existing Forge baseline, extension, dry-run,
  and persistence contracts.
- Preserve a direct assertion that the Forge adapter does not create or write
  DriftWatchdog-owned persistent state.
- Bring the canonical `forge-native-gate` spec into alignment with the
  baseline's explicit extension-selection behavior.
- Make the existing serial workspace test script and full Forge Gate pass.
- Keep the existing 390px workbench browser oracle passing when intrinsic
  content includes wide evidence tables and the lifecycle rail.

**Non-Goals:**

- No change to baseline rules, line limit, manifest schema, Gate result
  contract, CLI output, or production execution path unless a corrected test
  exposes a real mismatch.
- No edits to Workspace Governance or sibling repositories.
- No change to portal behavior outside the constrained mobile workbench grid
  and empty workspace-summary badge.
- No binary distribution or release workflow changes.

## Decisions

### Ownership and code boundary

Work in the Forge Rust workspace. Update assertions and fixtures in:

- `crates/driftwatchdog/src/commands/gate_tests.rs`
- `crates/driftwatchdog/src/gate/concerns.rs` test module
- `crates/driftwatchdog/src/gate/aigate.rs` test module
- `crates/driftwatchdog/src/gate/dry_run_plan_tests.rs`
- `crates/driftwatchdog/tests/gate_deployable.rs`
- `crates/driftwatchdog/tests/gate_dry_run_json.rs`
- `crates/driftwatchdog/tests/gate_exemption_cap.rs`
- `crates/driftwatchdog/tests/gate_governance_quality.rs`
- `crates/driftwatchdog/tests/gate_platform_result.rs`
- `crates/driftwatchdog/tests/gate_source_file_size.rs`
- `crates/driftwatchdog/tests/packaging/test_agent_examples.sh` (workspace
  target directory lookup used by the packaging-suite integration test)
- `src/api/command_catalog/rows_platform.rs` for the user-facing `use` row
- `src/cli/inventory.rs` command parity oracle, excluding hidden internal
  dispatch commands
- `tests/gate_contract/main.rs` for Forge-native Gate CLI, evidence, journal,
  status, rehearsal, and refusal behavior; remove obsolete subprocess-runtime
  fixtures and assertions
- `src/gate/evidence.rs` for resolving an optional DriftWatchdog executable
  solely for the separate release-evidence export operation
- `tests/gate_cross_surface.rs` for public runtime attribution and evidence
  export consumption across Gate, doctor, checker, and release surfaces

Update the canonical requirement in
`openspec/specs/forge-native-gate/spec.md`. Do not change resolver,
aggregation, persistence, or Forge CLI production code unless the tests prove
actual behavior diverges from that spec.

### Language, runtime, dependencies

Rust 2021, workspace member `driftwatchdog` at version `0.1.11`, built with
the workspace's locked Rust toolchain and existing `tempfile`/SQLite
test utilities. Add no dependencies. Verify with focused Cargo tests,
`RUSTC_WRAPPER= CARGO_INCREMENTAL=0 timeout 600 scripts/check-tests.sh`,
and the local Forge Gate dry-run/full run.

### Test contract decisions

| Surface | Required assertion |
|---|---|
| TOML/YAML and domain profiles | Resolved plans contain `source-file-size` once as required, selected by `forge-baseline`, and do not infer `governance-quality` from profile names or command-map entries. |
| Explicit governance extension | A project that explicitly selects and binds `governance-quality` retains it alongside the baseline. |
| Required unbound extension | A deliberately declared required check without a command is the unbound row and the dry-run's `plan-forced-block` reason. The in-process baseline is never listed as unbound. |
| Deployable profile | `compose-contract` and `ci-contract` are required project checks; `source-file-size` is in-process and never reports `:command` debt; `governance-quality` is absent unless explicitly selected. |
| JSON and human dry-run | The planned rows include the mandatory baseline; a blocking fixture declares an actually unbound required extension; the human golden snapshot reflects the current Forge baseline. |
| Unscheduled concerns | Assert an actually unscheduled concern such as `a11y`; the mandatory `source-file-size` row cannot be listed as unscheduled. |
| Direct DriftWatchdog `gate()` | A successful no-manifest or foreign-runtime fallback runs the Forge baseline and persists one passing run to the initialized DriftWatchdog store. |
| Forge adapter `gate_for_forge()` | A successful no-manifest baseline run uses in-memory storage and creates no `.driftwatch/state.db`. |

### Compatibility and failure policy

No manifest, valid manifest, and foreign-runtime declaration must continue to
use the mandatory Forge baseline. A direct DriftWatchdog API run retains its
own evidence lifecycle; a Forge invocation does not create DriftWatchdog
history. Project extension command text remains explicit and is never inferred
from a command map alone. An unbound required extension still blocks. Dry-run
still predicts a block only when the resolved plan contains a real blocking
input and never writes evidence.

`forge gate evidence` is a separate optional release-evidence consumer. It may
resolve a DriftWatchdog producer through an explicit override or PATH for the
`gate evidence-export` verb, but that resolver is not shared with the
Forge-native Gate execution path.

The serial suite also exposed two stale generation byte-equality assertions:
the standard receipt deliberately carries the actual `generated_at` value.
Equivalence tests normalize only that provenance field and compare every
other generated byte and receipt field. Its existing Chromium lifecycle
oracle then exposed a real 390px overflow: the one-column workbench grid's
automatic minimum track retained wide card content, while the empty workspace
badge added intrinsic width to the top bar. Use a zero-minimum mobile grid
track, shrinkable grid cards, and hide that nonessential empty badge at small
phone widths; keep tables and lifecycle steps in their existing bounded
scroll containers.

The next serial run exposed a third stale fixture in
`standard_cross_surface`: it asserted that default `forge new` omitted the
standard subtree, contradicting the canonical requirement that supported
service profiles receive `baseline-service@1.3.0`. Update that assertion to
check the current receipt while retaining its registry, list and profile
contract coverage; no standard-pack behavior changes.

The later web assurance failure also assumed that the first pack descriptor
supports `rust-web`. Descriptor order begins with the client pack, so the
remediation endpoint correctly refused that mismatch. The fixture now chooses
a selectable, verified pack compatible with its profile, keeping the endpoint
validation intact.

The workspace-metadata contract tests had the same age problem: their pinned
pre-metadata digests predated default standard-pack snapshots, and their JSON
shape assertion rejected the expected standard-pack note. Replace those
historical snapshots with paired generations that share project identity,
exclude exactly the declaration and receipt, normalize only the real standard
receipt timestamp, and compare all remaining bytes. The JSON assertion now
allows the existing standard note while ensuring metadata itself adds none.

The full Gate's mandatory source-size check then found that the optional
evidence-export resolver had brought `src/gate/evidence.rs` to 1,011 lines.
Move that small resolution boundary into private `src/gate/evidence_export.rs`;
Forge Gate execution remains in its existing engine and the evidence adapter
retains its separate optional producer lookup. The owning evidence module is
now below the 1,000-line baseline limit.

### UI/UX

N/A: this is a regression-suite reconciliation with no new user workflow or
screen. Existing CLI outputs are exercised as test inputs; no layout,
accessibility, responsive, or browser surface changes.

## Risks / Trade-offs

- **Risk:** Updating expected values could hide a real policy regression.
  **Mitigation:** Keep source-level resolver behavior and canonical Forge
  requirements as the oracle; assert the mandatory row, explicit extension,
  and persistent/non-persistent callers separately.
- **Risk:** Test-only reconciliation could pass targeted tests while other
  callers retain obsolete expectations. **Mitigation:** Run the full serial
  workspace test script and full Forge Gate before archive.
- **Risk:** Local linker or temp filesystem limits can obscure results.
  **Mitigation:** Do not skip the required full suite; record exact command,
  failure, and retry action if the environment prevents completion.

## Migration Plan

No data or deployment migration. Update tests first, verify the full suite,
promote the amended canonical spec, archive without `--skip-specs`, commit the
implementation/spec/archive, then update and commit HANDOFF separately.
Rollback is a normal Git revert of the single implementation commit; no
stored data format changes.

## Open Questions

None. The distinction between Forge's in-memory adapter and the crate's direct
persistent API is established in `commands/gate.rs` and the Forge native-gate
design.

## Traceability

| Requirement | Design decision | Implementation/test boundary | Verification oracle |
|---|---|---|---|
| `forge-native-gate`: project Gate profiles preserve baseline and explicit extensions | Test contract decisions, first four rows | manifest, YAML adapter, concern and dry-run tests | focused module tests and `scripts/check-tests.sh` |
| `forge-native-gate`: Forge owns public evidence persistence | Test contract decisions, last two rows | direct `gate()` and `gate_for_forge()` unit tests | command gate tests plus full Forge Gate |
