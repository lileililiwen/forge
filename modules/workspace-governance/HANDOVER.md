# Workspace Governance Handover

## Current state (2026-10-12)

- Branch: `main`.
- Active OpenSpec changes: none. `HANDOFF.md` is the canonical detailed completion log; the current pointer is `current_spec: none`.
- The most recent governance change, `enforce-openspec-seriality`, is archived and committed as `5e6b4ce`, followed by the HANDOFF-only commit `b05e5d4`.
- The local checker blocks multiple active changes and invalid `current_spec` state for this repository. Other repositories receive the sequential workflow through templates; shared Gate runtime support for this finding remains follow-up work.

## Verification

The latest recorded checks are in `HANDOFF.md`. For a fresh checkout, run:

```bash
python3 -m unittest discover -s tests -v
python3 scripts/workspace_check.py --self-check
python3 scripts/auto_gate.py --self-check --no-deployment
openspec validate --all --strict --no-interactive
node scripts/check-openspec-change-names.mjs
openspec list
```

This repository has no human-facing application surface. Its documentation
evidence is a redacted terminal capture in `docs/assets/terminal-self-check.txt`;
the capture procedure is recorded in `docs/assets/capture-plan.md`.

## Historical context

The superseded 2026-10-05 workspace handover is preserved at
`docs/history/legacy-workspace-handover.md`. Its old fleet counts, uncommitted
file lists, and next-step instructions are historical only. Use the current
registry, Git state, `HANDOFF.md`, and OpenSpec archive as evidence.
