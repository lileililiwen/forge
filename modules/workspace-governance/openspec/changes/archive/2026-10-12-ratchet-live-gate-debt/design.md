# Design: Block new work on live gate debt increases

## Implementation boundary

- Repository/project: `/home/paul/code/workspace-governance`.
- Runtime/module: Python 3, `scripts/workspace_check.py`.
- Entry point: `validate_new_change_with_gate_debt(project_path, findings, project_id, ...)`, called by `validate_project`.
- Shared parser: `scripts/bind_gate_commands.py::{read_manifest, read_gate_policy, gate_required, PROFILE_DEFAULTS, unquote_scalar}`.
- Tests: `tests/test_gate_debt_precondition.py`; requirements: `openspec/specs/workspace-check/spec.md`.
- Local Gate prerequisite: `.ai-gate/gate.yaml` receives the binder-derived `governance-quality` command because this repository is itself governed by the mandatory profile default.
- Do not modify Driftwatchdog or project repositories.

## Language and runtime

Python 3 standard library, matching the governance checker. Verification uses `python3 -m unittest tests.test_gate_debt_precondition -q`, the complete `python3 -m unittest discover -s tests -q`, strict OpenSpec validation, workspace self-check, auto-gate, and change-name validation.

## Ownership and shared code

Workspace Governance owns debt policy, the snapshot, and the binder. The live reader will reuse the binder parser and profile-selection mirror rather than introduce a second manifest interpretation. Driftwatchdog remains the runtime owner; its gate oracle test is the compatibility check for the mirrored required sets. No shared package extraction or sibling changes are needed.

## Behavioral model

1. Enumerate unarchived OpenSpec changes. If there are none, return without reading live gate state.
2. Preserve the existing behavior when `gate-debt.json` could not be loaded: its caller already reports a run-level error.
3. Read `.ai-gate/gate.yaml`. Parse declared selections, commands, disabled/exempt policy, and profile defaults through the binder helpers.
4. Define `live_unbound` as required concerns returned by `gate_required` whose command is absent or empty in the manifest.
5. Compare against the recorded concerns for this project. `live_unbound - recorded` is the increased debt set.
6. For each non-grandfathered, non-exempt, non-debt-closing change, emit the existing blocking finding when recorded debt exists or live debt increased. The message names the live increase and directs the owner to bind checks or regenerate the snapshot after a genuine repair.

The existing snapshot remains sticky for concerns it records; a decrease is not treated as a new failure and is cleared by normal debt-record regeneration. The live read only adds newly unrecorded debt. No checker path writes files or executes commands.

## Failure and boundary behavior

- No active changes: no live read and no finding.
- Missing, unreadable, malformed manifest or unknown profile the binder cannot resolve: fail closed with `QUALITY_NEW_CHANGE_WITH_GATE_DEBT` naming the read/resolution failure.
- Required command missing/empty: it is live unbound.
- New required command present: it is not live unbound, regardless of whether that command passes when executed.
- Existing baseline pair, exception, and `close-unbound-gate-checks` exclusions retain their current behavior.
- A live increase applies even when the project is absent from the recorded snapshot or recorded clean.

## Contract and compatibility

No persisted schema or CLI changes. The rule keeps code `QUALITY_NEW_CHANGE_WITH_GATE_DEBT`. Existing finding levels, baseline/exception records, and generated snapshot remain compatible. The binder mirror is the parser authority; an unsupported profile is an error rather than an empty required set.

## Verification oracle

Temporary project fixtures must establish: (1) clean snapshot plus a new unbound required check and new change emits one ERROR naming that concern; (2) newly added bound check is silent; (3) unchanged recorded debt still blocks; (4) an unreadable/unknown live manifest blocks fail-closed; and (5) baseline, exception, and debt-closing-change controls remain unchanged. Run focused and complete suites, then strict OpenSpec and workspace self-checks.

## Decision ledger

- Reuse the binder's parser and profile mirror to prevent a third independent policy implementation.
- Bind this repository's mandatory governance-quality concern using the binder output, not a hand-written command.
- Compare only commands declared in the current manifest; do not infer health from whether a command is derivable or whether it passed.
- Keep recorded debt sticky until the binder refreshes its generated snapshot; this change specifically prevents unrecorded increases.
- Fail closed on a current state that cannot be resolved; silently treating it as clean recreates the snapshot gap.
