# Proposal: Block new work on live gate debt increases

## Why

`QUALITY_NEW_CHANGE_WITH_GATE_DEBT` currently trusts `gate-debt.json` as a snapshot. If a repository adds a required unbound check to its manifest before the debt record is regenerated, the next OpenSpec change is not blocked. That permits debt to increase between binder runs.

## What Changes

For a repository opening an unarchived change, read its current `.ai-gate/gate.yaml` using the same required-check resolution as the binder, then compare current unbound required checks with the recorded set. Any current check absent from the record blocks the new change. An unreadable or unresolvable live manifest fails closed. Existing baseline, exception, and debt-closing-change rules continue to apply.

## Package Boundary and Split Assessment

This package changes one Python 3 governance rule in `scripts/workspace_check.py`, its focused tests, and the `workspace_check` capability spec. Its acceptance boundary is a recorded-clean repository whose manifest gains an unbound required check: the next new change must produce the existing blocking finding. The governance repository's own manifest also receives the binder-derived command for the newly mandatory `governance-quality` concern; this prerequisite was missing and blocked the required self-check. This is independent from scaffold sequencing and legacy repair; it needs no additional project, persistence format, or runtime deployment package.

| Package | Single outcome | Owner/project and language | Boundary/contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `ratchet-live-gate-debt` | Block a new OpenSpec change when the live manifest has unrecorded unbound checks | `workspace-governance`, Python 3 | `workspace_check.py` compares required check commands in `.ai-gate/gate.yaml` with `gate-debt.json` | Existing gate binder profile mirror and new-change rule | Temporary repository adds an unbound check after a clean snapshot; checker emits `QUALITY_NEW_CHANGE_WITH_GATE_DEBT` naming it |

Dependency order: `drive-legacy-gate-debt-to-zero` and the binder's profile resolution are already committed. This package consumes their debt vocabulary and does not alter either package.

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| Governance binder | `scripts/bind_gate_commands.py::read_manifest`, `read_gate_policy`, `gate_required`, `PROFILE_DEFAULTS` | Existing manifest parsing and profile selection mirror; `GateOracleTest` checks the mirror against Driftwatchdog | Binder classifies derivable commands, while this rule must test commands actually present in the live manifest | Workspace Governance; same Python release and no sibling edit | **extend shared owner** |
| Driftwatchdog runtime | `driftwatchdog/src/gate/aigate.rs`, `src/gate/manifest.rs` | Authoritative required-check and profile behavior; the binder oracle already compares its output | Runtime subprocess invocation per project is not needed for this bounded precondition; current unbound status is exactly required concerns without manifest commands | Driftwatchdog owns runtime; no changes in this package | **adopt** the binder's existing tested mirror |

## BFS Impact Map

- `scripts/workspace_check.py`: computes live unbound required concerns only when a project has unarchived work, compares them with the snapshot, and emits the existing blocking rule.
- `.ai-gate/gate.yaml`: bind the mandatory governance-quality check with the exact command derived by the binder, enabling this repository's own Gate/self-check.
- `tests/test_gate_debt_precondition.py`: pins snapshot-clean/live-dirty, live-clean negative control, fail-closed manifest errors, and preservation of baseline/exception/closing-change behavior.
- `openspec/specs/workspace-check/spec.md`: defines the live comparison and failure scenarios.
- `HANDOFF.md`: records exact verification and two commit hashes.
- Unchanged: `gate-debt.json` schema/generator, Driftwatchdog source, scaffold behavior, product repositories, and debt closure decisions.

## Capabilities

- **workspace-check** (existing): extend `QUALITY_NEW_CHANGE_WITH_GATE_DEBT` so the check reads both the recorded snapshot and current manifest state.

## Non-goals

- Regenerating or writing `gate-debt.json` from the checker.
- Removing debt that remains in the snapshot after a repository repairs its manifest; the workspace binder remains responsible for refreshing that record.
- Running gate commands or claiming that a bound command passes.
- Changing one-time baseline, recorded-exception, or debt-closing-change policy.
