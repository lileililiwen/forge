# Proposal: Classify debt blocked by dirty gate manifests

## Why

The binder correctly refuses to overwrite a dirty `.ai-gate/gate.yaml`, but its recorded debt reason is currently reported by the legacy-debt driver as unknown. The operator needs a precise owner action while the driver's safety boundary remains intact.

## What Changes

Extend the existing governance-owned debt remediation driver with a known, non-mechanical class for dirty-manifest refusals. The report will explain that the manifest must be reviewed and committed before the binder is rerun. It will not alter sibling manifests. Refresh the generated workspace debt snapshot after authorized binder application so it reflects the measured current fleet.

## Package Boundary and Split Assessment

This package has one owner (`workspace-governance`), one runtime (Python 3), one record contract (`gate-debt.json`), and one acceptance boundary: a dirty-manifest reason is recognized and reported with zero target writes. Updating the generated debt snapshot is the output of the same binder operation and records the fleet state needed to exercise that behavior. No sibling repositories are edited by this package. A separate package is unnecessary because there is no additional lifecycle or acceptance oracle.

| Package | Single outcome | Owner/project and language | Boundary/contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `classify-dirty-gate-manifest-debt` | Report dirty-manifest debt as a safe owner action | `workspace-governance`, Python 3 | Binder reason in `gate-debt.json` to `close_gate_debt.py` class G | Existing binder and legacy debt driver | Unit fixture proves class G and unchanged target bytes; measured dry-run has no unknown dirty-manifest entries |

Dependency order: the binder's dirty-manifest debt reason and debt driver already exist. This package extends their classifier/report contract only.

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| Workspace Governance binder | `scripts/bind_gate_commands.py`, dirty-manifest refusal reason | Produces the canonical reason and protects user edits | Binder must not decide or overwrite a dirty manifest | Workspace Governance, same repository and Python runtime | **extend shared owner** |
| Driftwatchdog | `driftwatchdog/src/gate/manifest.rs` | Consumes the resulting manifest | Does not own debt repair reports or workspace writes | Driftwatchdog runtime release | **adopt** existing manifest behavior |

## BFS Impact Map

- The CLI and JSON report gain class G, a non-writing owner-action report.
- The debt schema and binder behavior do not change; the generated snapshot reflects the authorized full-workspace binder apply.
- Tests assert classifier behavior, owner-action text, no remedy dispatch, and no target mutation.
- No gate command is executed, and no sibling manifest is written by this package.
- Unknown reason handling remains `?`; other remedy classes and decisions remain unchanged.

## Capabilities

- **legacy-gate-debt-remediation** (existing): recognize binder refusals caused by a dirty manifest and direct the operator to review/commit it before rebinding.

## Non-goals

- Committing, restoring, or editing any product's dirty manifest.
- Choosing which changes in a dirty product manifest should be retained.
- Clearing debt before the binder successfully writes and records a binding.
- Applying or committing changes in sibling repositories.
