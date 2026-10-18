# rules-baseline Specification

## Purpose

Give the workspace one canonical rules source and a non-destructive drift
report, so the shared baseline is unambiguous without overwriting the
project-specific guidance, wording and language that repositories legitimately
keep.

## ADDED Requirements

### Requirement: Single canonical source

The canonical rules text MUST live only in `templates/project/**`, and
`init_project.py` MUST read its output from those files rather than from an
embedded copy. The generated `.ai-rules/workflow.md` MUST contain the OpenSpec
name-preflight sentence.

#### Scenario: Initializer uses the on-disk template

- **WHEN** `init_project.py` writes `.ai-rules/workflow.md` for a fresh project
- **THEN** the written file contains the OpenSpec name-preflight sentence taken
  from the on-disk template

#### Scenario: No second copy

- **WHEN** a canonical template changes
- **THEN** the initializer's output changes with it and no embedded duplicate
  needs editing

### Requirement: Drift is reported, never rewritten

`rules_drift.py` MUST report, for each adopted project, every invariant clause
that its `.ai-rules` files do not visibly carry, and MUST NOT modify any project
file. A rule file carrying the opt-out marker
`<!-- workspace-governance:rules-customized -->` MUST be skipped.

#### Scenario: Drift reported

- **WHEN** an adopted project's `workflow.md` omits the two-commit rule
- **THEN** the report names the project and the clause, and the file is
  byte-identical afterwards

#### Scenario: Customized but deliberate

- **WHEN** a project documents the rules in its own words and carries the
  opt-out marker
- **THEN** no finding is reported for that file

#### Scenario: Non-destructive run

- **WHEN** the report runs across the workspace
- **THEN** no project file changes

### Requirement: Scaffold handoffs are surfaced

A `HANDOFF.md` that still carries the initializer's scaffold boilerplate MUST be
reported by the workspace audit as WARNING `HANDOFF_SCAFFOLD`; a handoff with
real content MUST NOT be reported.

#### Scenario: Scaffold detected

- **WHEN** an adopted project's `HANDOFF.md` still contains the initializer
  boilerplate
- **THEN** the audit reports WARNING `HANDOFF_SCAFFOLD`

#### Scenario: Real handoff not flagged

- **WHEN** a project's `HANDOFF.md` contains project-specific content
- **THEN** no scaffold finding is reported

### Requirement: The baseline clears the last audit errors

An adopted project that is missing its `.ai-rules` files MUST be brought up to
the baseline by creating them from the canonical template, so the workspace
audit's `REQUIRED_FILE` errors clear.

#### Scenario: Baseline clears the audit

- **WHEN** every adopted project that requires rule files has them
- **THEN** the audit reports no `REQUIRED_FILE` finding and zero errors
