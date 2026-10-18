# Design: Enforce sequential OpenSpec changes

## Context

`unarchived_changes(project_path)` already enumerates direct active change
directories while excluding `archive/` and hidden directories.
`validate_change_names()` runs for repositories whose profile requires
OpenSpec. Workspace Governance's own Gate invokes
`workspace_check.py --project workspace-governance --json`; `auto_gate.py`
fails on ERROR. The missing behavior is a seriality check for this repository
plus consistency between its one active change and `HANDOFF.md`.

## Goals / Non-Goals

**Goals:** block more than one active change in this repository; keep the
handoff pointer aligned with the active directory; retain the existing finding
and Gate contracts.

**Non-Goals:** inspect or constrain other project repositories; detect
unfinished task checkboxes inside a single change; alter the OpenSpec CLI;
perform rollback or archive operations automatically.

## Decisions

1. Add `validate_openspec_seriality(project_path, findings, project_id)` in
   `scripts/workspace_check.py`, reusing `unarchived_changes()` rather than
   duplicating directory rules. Call it from `validate_project()` only when
   `project_id == "workspace-governance"`. The existing Gate checks this
   project id, and this keeps the rule local rather than imposing it on the
   fleet.
2. Emit ERROR `OPENSPEC_CHANGE_SERIALITY` for any invalid state. More than one
   active directory yields one finding listing sorted names. Exactly one
   active directory requires exactly one line-anchored `current_spec:` entry
   whose unquoted value equals that directory name. Zero active directories
   accepts no pointer or one blank/`none` pointer; any named pointer or
   duplicate pointer is stale/ambiguous and fails.
3. Read `HANDOFF.md` when an active change exists or when a file is present
   and could contain a stale pointer. A missing handoff is allowed only when
   no change is active; an unreadable existing handoff fails closed. Do not
   parse prose mentions of `current_spec` as pointers.
4. Add isolated tests and a repository-state test to the existing required
   unit suite. Driftwatchdog owns the allow-list for `governance-quality`
   finding codes, and its current declaration does not include this new code;
   widening that list would require a sibling source edit. Keep scope local by
   making the existing `tests` Gate concern assert the real repository state.
   The checker still reports `OPENSPEC_CHANGE_SERIALITY` for JSON consumers.

Alternatives considered: a documented convention has no enforcement; a
preflight wrapper can be bypassed by direct OpenSpec CLI use; editing the
Driftwatchdog finding-code allow-list crosses the approved repository boundary;
applying the rule fleet-wide changes independent repository policies. The
local checker plus the already-required test Gate is the smallest enforced
boundary.

## Risks / Trade-offs

- **An in-progress change blocks creation of another:** this is the intended
  serial workflow; the diagnostic names the required recovery.
- **A parser could misread prose:** only anchored `current_spec:` lines are
  inspected, matching the repository's existing handoff convention.
- **The project id is a local scope discriminator:** registry/declaration
  identity validation already checks this id; other project ids are not
  subject to the new rule.

## Migration Plan

No data migration. The current repository has no active change and a
`current_spec: none` pointer, which remains valid. Archive the implementation
change after verification; rollback is a revert of its implementation/archive
commit and handoff-only commit.

## Open Questions

None.
