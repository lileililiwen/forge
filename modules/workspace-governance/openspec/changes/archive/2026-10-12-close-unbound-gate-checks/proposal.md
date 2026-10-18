# Proposal: Close the unbound gate checks in `workspace-governance`

## Why

The gate fails this repository. `driftwatchdog` reports **1 required
check** as `UNBOUND` — *no command declared for this
concern; no execution was attempted* — and counts the project as failing. A
selected check with no command behind it is a check that never runs, which is
the same defect as documentation that claims a gate exists.

| concern | status | why it is still unbound |
|---|---|---|
| `security` | UNBOUND | no Python security tool is declared in this repository's pyproject.toml or requireme. See below for the full reason and the exact edit. |


The authoritative statement of this is `workspace-governance/gate-debt.json`.
Its 24 repositories and 47 checks are equal, **by set comparison against the
gate's own dry run**, to what the gate reports as unbound; `GateOracleTest`
pins that equality so the record cannot silently drift. This change covers
the 1 check listed for `workspace-governance` and no other.

Totals across the portfolio at the time of writing: 24 repositories, 47 unbound checks.

## Why the binder did not invent these commands

`scripts/bind_gate_commands.py` derives each command from what is on disk and
**refuses rather than guesses**, on one rule: *a wrong command is worse than a
listed gap*. A guessed command is worse because it is not visible. A listed
gap is on the record, names its own cause, and is read by the next person. A
wrongly-bound `build` reports a healthy repository that was never built, and
nothing downstream can tell the difference.

So the 47 that remain are not 47 failures of the tool. Each is a place where
the tool found a genuine ambiguity — 1 of them in this
repository — and said so instead of picking. Every reason in the table above is
quoted from the debt record rather than restated, so this change and the record
cannot disagree.

## What Changes

For each of the 1, the concrete work is below: which file to write, which
key to add, or which command to put in `.ai-gate/gate.yaml`. Each has
`Close it` instructions naming the exact edit, and an `On disk today`
statement of what the binder actually saw. **No re-derivation is required to
act on any of them.**

### `security`

**Close it — declare the audit tool, then bind it.**

**On disk today:** this repository has no `pyproject.toml` and no
`requirements*.txt`, so it declares no dependency-audit tool, and a Python
dependency audit is not part of the standard library. There is nothing for the
binder to point a command at.

1. Add a `pyproject.toml` (or a `requirements-dev.txt`) declaring `pip-audit`
   in the development dependency group, and commit it.
2. Bind the check in `.ai-gate/gate.yaml`:

   ```yaml
   commands:
     security: python3 -m pip_audit
   ```

3. Re-run `python3 scripts/bind_gate_commands.py --root /home/paul/code` and
   confirm `security` has left this repository's `unresolved` list.

A hand-written command under `commands:` is equally acceptable. What does not
close the gap is leaving the concern selected with no command, because the gate
then fails on a check that has never been given a meaning.


## Why this is prerequisite work

**This is not business work and it is not deferrable alongside it.** A gate
that reports `UNBOUND` for a selected check is a gate reporting that a promise
was made and nothing delivers it. Starting product work on top of it means
every later gate result is read against a baseline that was already failing,
so a real regression and a pre-existing gap look identical. The floor has to
be standing before anything is built on it.

Two of the three closure routes for every concern here are one small edit to
`.ai-gate/gate.yaml` — bind the command, or declare the concern out of scope
with a reason. The third is writing the checker. None of them is product
work, and all of them are the thing that makes later product work measurable.

## What this deliberately does not do

- **It does not write a single command.** Every close below is a human
  decision, and this change is the note that says the decision is owed and
  names it.
- **It does not modify `.ai-gate/gate.yaml` or `.project.json`.** Those are the
  files the decisions land in, and they are not this change's to touch.
- **It does not guess an entry point for an ambiguous repository.** The
  candidates are listed; choosing is the owner's.
- **It does not add or remove a concern.** A concern is closed by binding a
  command to a check the repository already selects, or by declaring that
  check out of scope on the record.

## Impact

| Surface | Before | After |
|---|---|---|
| Checks this repository reports as `UNBOUND` | 1 | **0** |
| Commands written by this change | 0 | 0 — by design |
| Decisions recorded as owed | 0 | 1 |
| `.ai-gate/gate.yaml` / `.project.json` edits | 0 | 0 |

## Non-goals

No product, runtime, or architecture change. No dependency added. No check
removed from the gate's selection. No claim that a command runs until it has
been run.
