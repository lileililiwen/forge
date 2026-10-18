# Proposal: Enforce sequential OpenSpec changes

## Why

Several active changes accumulated in this repository, leaving partially
completed tasks to overlap with later work and making ownership and rollback
unclear. The existing Gate checks change names and gate debt but does not
prevent a second active change from starting.

## What Changes

- Add a blocking audit rule that permits at most one active OpenSpec change in
  the `workspace-governance` repository.
- Require the active change name to match the single `current_spec` pointer in
  `HANDOFF.md`; with no active changes, allow no pointer or `current_spec: none`
  and reject a stale named pointer.
- Add focused regression tests and document the sequential workflow.

## Package Boundary and Split Assessment

| Package | Single outcome | Owner/project and language | Boundary/contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `enforce-openspec-seriality` | Block overlapping OpenSpec work in this repository | Workspace Governance, Python | `workspace_check.py` ERROR from the existing project audit | none | focused tests plus the real Gate |

This is one outcome with one owner, one state model (active change count and
pointer), and one acceptance oracle. CLI preflight-only behavior could be
bypassed, while a separate workflow-documentation package could ship without
enforcement, so they are not independent deliverables. It does not change
OpenSpec behavior for sibling repositories or modify their files.

## Sibling and Shared Architecture Reconnaissance

| Candidate | Evidence path/symbol | Reusable code/config/architecture | Compatibility gap | Owner and release boundary | Decision |
|---|---|---|---|---|---|
| Workspace Governance | `scripts/workspace_check.py::unarchived_changes` and `validate_change_names`; `.ai-gate/gate.yaml::governance-quality` | Existing active-directory enumeration and the blocking project audit | No seriality or pointer consistency check exists | Local repository policy; no package or sibling release required | **keep local** |
| Driftwatchdog | `../driftwatchdog` Gate runtime, invoked by this repository's `.ai-gate/gate.yaml` | Existing Gate executes the declared governance-quality command and blocks ERROR | It owns Gate execution, not OpenSpec lifecycle policy | Driftwatchdog runtime release is unchanged | **adopt** the existing Gate path |

The Workspace Governance checker remains the policy owner. Driftwatchdog is
reused as the existing execution boundary; no Driftwatchdog source or config
is changed. Forge's adapter consumes the existing findings list, so this rule
adds only a finding and no adapter shape.

## User Experience and Interface Impact

The actor is the repository maintainer or coding agent using the OpenSpec CLI
and Gate output. A clean state passes; a second active directory produces a
plain ERROR listing the active names and directing the actor to finish/archive
the current change before opening another. A single change with a missing,
duplicate, stale, or mismatched `current_spec` produces an ERROR that names
the expected pointer. Recovery is to archive/complete the current change or
correct `HANDOFF.md`. This terminal workflow has no screens, navigation, or
responsive layout. Diagnostics use plain text and do not depend on color,
mouse, or terminal width; browser/device evidence is not applicable because
the interface is a CLI and repository Gate.

## BFS Impact Map

- **Checker and callers:** `validate_project()` adds one local governance
  invariant; `auto_gate.py` already fails on ERROR. Existing JSON finding
  shape and Forge adapter observation keys remain unchanged.
- **Data and integrations:** reads direct child directories under
  `openspec/changes/` and the `current_spec` line in `HANDOFF.md`; writes no
  registry or project declaration and invokes no OpenSpec subprocess.
- **Failure behavior:** multiple active changes or inconsistent pointer state
  is ERROR; directory enumeration or required handoff read failure is also
  ERROR, never a silent pass.
- **Tests:** isolated temporary repositories cover zero, one, and multiple
  active changes, pointer mismatch/duplication, and the audited project scope.
- **Security and compatibility:** no network, subprocess, external path read,
  or sibling edit; no new dependency or schema migration.
- **Shared ownership:** the policy is specific to this repository; using a
  generic rule over every registered repository would expand scope and could
  block independently governed consumers.

## Capabilities

- Modified: `workspace-check` — add the one-active-change and pointer
  consistency requirement.

## Non-goals

- Do not enforce this policy on sibling repositories.
- Do not make unchecked, explicitly deferred tasks block archiving.
- Do not add a CLI wrapper, branch policy, approval service, rollback command,
  or change OpenSpec's own CLI.
- Do not edit pre-existing dirty user files or any sibling repository.
