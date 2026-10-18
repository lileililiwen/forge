## ADDED Requirements

### Requirement: Every selected check has a command or a written exemption

A repository's gate manifest MUST NOT leave a required check selected with no
command bound for it, because a selected check with no command is a check the
gate reports as `UNBOUND` and counts as a failure. Every such check MUST be
closed by one of exactly two written outcomes: a command under `commands:`, or
a deliberate statement that the check does not apply — `checks: <id>: false`
or a `not_applicable:` entry carrying a reason. A check MUST NOT be closed by
silence, by an absent file, or by a script that exits `0` without checking
anything.

#### Scenario: A check is closed by a command

- **WHEN** a required check has a real command written under `commands:`
- **THEN** the gate executes it and no longer reports it as `UNBOUND`

#### Scenario: A check is closed on the record

- **WHEN** a check genuinely has no surface in this product and the manifest
  records `checks: <id>: false` or a `not_applicable:` entry with a reason
- **THEN** the gate stops failing on it, and the reason remains on the record
  as the statement of why

#### Scenario: A check that is only nominally closed is not closed

- **WHEN** a command is bound for a check that does not verify its subject
- **THEN** the gap is still open, because a bound check that runs nothing is
  the same defect as an unbound one wearing a passing result

### Requirement: An ambiguous entry point is decided by an owner, in writing

Where a repository contains more than one entry point that can answer the same
concern, the concern MUST NOT be bound until an owner has recorded which entry
point the gate is to run. A tool MUST NOT pick one, MUST NOT prefer one
toolchain over another, and MUST NOT infer the choice from a declared `stack`
label. The reason for the refusal MUST name every candidate together with the
on-disk evidence that made it a candidate, so the decision can be made without
re-deriving anything.

#### Scenario: Two systems can each answer the concern

- **WHEN** a repository has both a .NET solution and a Node manifest that can
  each answer the same concern
- **THEN** the concern stays unbound, and the record names both candidates and
  the evidence for each, until an owner writes one of them under `commands:`

#### Scenario: The ambiguity is removed instead of decided

- **WHEN** a repository adds one script that runs every candidate in sequence
- **THEN** exactly one entry point answers the concern and the command can be
  derived without a decision

### Requirement: A refused derivation is a recorded gap, never a silent one

When a command cannot be derived, the gap MUST be recorded with the specific
cause — the file and the key whose absence caused the refusal — and the
reason MUST be durable and reviewable rather than held in a tool's exit code.
A reason that names only a class of problem MUST be treated as incomplete,
because a class tells the next reader nothing they can act on.

#### Scenario: The reason names the file and the key

- **WHEN** a concern has no command because the repository declares no
  dependency-audit tool
- **THEN** the reason names `pyproject.toml` or `requirements*.txt` and the
  dependency group to add the tool to

#### Scenario: The reason names the missing checker

- **WHEN** a concern has no command because no checker script exists
- **THEN** the reason names `scripts/check-<concern>.*` as the file to write

#### Scenario: The reason names every candidate

- **WHEN** a concern is refused because more than one entry point could answer
  it
- **THEN** the reason names every candidate, each with the path that made it
  one

### Requirement: Unbound gate checks are closed before product work proceeds

Unresolved gate bindings in a repository are **prerequisite** work. A
repository MUST NOT begin product work while a check its own manifest selects is
reported as `UNBOUND`, and each unresolved check MUST be tracked as a named
task with the file, key, or command that closes it, before product work
starts. A repository's product work MUST NOT be scheduled as though the gate
were passing, because an `UNBOUND` check makes every later gate result
ambiguous: a real regression and a pre-existing gap produce the same report.

#### Scenario: Product work is blocked by an open gap

- **WHEN** a repository has one or more checks its manifest selects but binds
  no command for
- **THEN** the repository's product work does not proceed until each is closed
  by a command or a written exemption

#### Scenario: The next reader is told what is owed

- **WHEN** a person or agent opens this repository with no memory of an
  earlier session
- **THEN** this change names every unbound concern, the reason for each, and
  the concrete edit that closes it, so no gap is inherited silently

### Requirement: The repository's own debt is stated in full and in place

The workspace-governance debt record SHALL contain every currently unbound
required check for this repository and SHALL contain no check the gate reports
as bound. The number of entries SHALL match the real gate's unbound set; the
record is allowed to be empty when no checks are unbound.

The recorded baseline for this closure is zero: the `security` check has a
written, scoped command and the real Gate passed it.

Every non-empty debt entry MUST retain the reason from the generated record,
not a restatement from memory.

#### Scenario: The list is complete when debt exists

- **WHEN** the gate reports an unbound check for this repository
- **THEN** this change names it, with the reason the record carries

#### Scenario: The list is empty when all checks are bound

- **WHEN** the gate reports no unbound checks for this repository
- **THEN** the debt record contains no entry for this repository
