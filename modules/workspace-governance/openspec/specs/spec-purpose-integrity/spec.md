# spec-purpose-integrity Specification

## Purpose
Defines fail-closed detection of archive-generated Purpose placeholders and a dry-run-first backfill that records its source and requires an explicit, reviewable apply.
## Requirements
### Requirement: An unresolved archived Purpose is a finding

Workspace Governance SHALL report a spec file whose `## Purpose` section still
holds the archiver's placeholder — `TBD - created by archiving change <name>.`,
with or without the trailing `Update Purpose after archive.` sentence — as ERROR
`SPEC_PURPOSE_PLACEHOLDER`, naming the spec file relative to its repository and
the change the placeholder records. The check SHALL detect both placeholder
generations the archiver has emitted, SHALL read the section body rather than
assume a single line, SHALL NOT fire on a `## Purpose` holding any real prose
whatever its wording, and SHALL skip a repository with no
`openspec/specs/<capability>/spec.md` tree entirely.

#### Scenario: A placeholder survives the archive

- **WHEN** a repository holds `openspec/specs/retention/spec.md` whose
  `## Purpose` body is
  `TBD - created by archiving change add-retention-policy. Update Purpose after archive.`
- **THEN** the audit reports ERROR `SPEC_PURPOSE_PLACEHOLDER` naming
  `openspec/specs/retention/spec.md` and the change `add-retention-policy`

#### Scenario: The legacy placeholder form

- **WHEN** a `## Purpose` body is
  `TBD - created by archiving change add-admin-ip-allowlist.` with no trailing
  sentence
- **THEN** the audit reports the same ERROR, because both generations are the
  same defect

#### Scenario: A real purpose is not a finding

- **WHEN** a `## Purpose` body is `TBD - created by archiving change
  add-retention-policy. Update Purpose after archive. Yes, this sentence is real
  prose that happens to start with the word.` — or any other real prose
- **THEN** the audit reports no `SPEC_PURPOSE_PLACEHOLDER`, because the section
  holds a purpose and the wording is not what is being decided

#### Scenario: A repository with no specs tree

- **WHEN** a repository has no `openspec/specs` directory
- **THEN** the audit reports no `SPEC_PURPOSE_PLACEHOLDER` for it and does not
  report any other finding for the absent tree

#### Scenario: The rule is not a requirements driver

- **WHEN** a repository's profile does not require OpenSpec but it carries an
  `openspec/specs` tree with a placeholder
- **THEN** the audit still reports ERROR `SPEC_PURPOSE_PLACEHOLDER`, because the
  rule is descriptive and is not gated on `profile_requirements`

### Requirement: The Purpose is derived mechanically, never by judgment

A backfill tool owned by Workspace Governance SHALL resolve each placeholder
from one of exactly two sources, in order, and SHALL record on every plan row
which source it used. The first source SHALL be the `## Why` section of the
change's own archived `proposal.md`, resolved by stripping the `YYYY-MM-DD-`
prefix from the archive directory name and matching the placeholder's bare
change name, and used only when the whitespace-collapsed section is at least
`MIN_WHY_CHARS` long. The second source SHALL be a sentence derived from the
file's first `### Requirement:` heading, or from the capability name when the
file has no such heading, and SHALL be byte-for-byte the sentence the
`openlearning` prior art produces. A change name matching more than one archive
directory SHALL refuse the plan and name both directories rather than choose.

#### Scenario: The archived proposal has a real Why

- **WHEN** a spec's placeholder names `add-retention-policy` and
  `openspec/changes/archive/2026-09-18-add-retention-policy/proposal.md` has a
  `## Why` section longer than `MIN_WHY_CHARS`
- **THEN** the plan row records `source: archived-change-why` with the evidence
  path, and the resolved Purpose is that section's prose

#### Scenario: No recoverable Why, so the requirement derives it

- **WHEN** the archived proposal exists but has no `## Why` section
- **THEN** the plan row records `source: first-requirement`, and the resolved
  Purpose is
  `The retention specification covers <first requirement topic, lowercased> and the related scenarios that govern this capability across the platform.`

#### Scenario: No requirement heading either

- **WHEN** the spec has no `### Requirement:` heading
- **THEN** the plan row records `source: capability-name` and the resolved
  Purpose is
  `The retention specification defines the behaviors, contracts, and scenarios that govern this capability across the platform.`

#### Scenario: Two archive directories share a change name

- **WHEN** a change name resolves to `2026-09-08-platform-testing-toolkit` and
  `2026-09-10-platform-testing-toolkit`
- **THEN** the tool refuses to plan, names both directories, and writes nothing

#### Scenario: The derivation matches the prior art

- **WHEN** a spec with a `### Requirement:` heading and no recoverable `## Why`
  is resolved
- **THEN** the resolved Purpose is the exact string
  `openlearning/scripts/fill_spec_purposes.py` produces for the same file

### Requirement: A dry run writes nothing and an apply is the only write

`scripts/spec_purpose_backfill.py` SHALL compute a plan and write nothing when
invoked with no write flag, and SHALL print `Re-run with --apply under explicit
authorization.` on that path. It SHALL write only under `--apply`, and the
`--apply` path SHALL print the reminder to set `SPEC_PURPOSE_AUDIT_ENABLED =
True` in `scripts/workspace_check.py` in the same commit. `--check` SHALL report
whether every planned row has been applied and exit non-zero while any row is
outstanding. Re-running after a successful apply SHALL produce an empty plan, so
a human-edited Purpose is never silently re-derived.

#### Scenario: The dry run is byte-identical

- **WHEN** the tool is run with no write flag over the real fleet
- **THEN** every spec file is byte-identical afterwards and the plan lists every
  placeholder with its source

#### Scenario: The apply names the flag

- **WHEN** the tool is run with `--apply`
- **THEN** each written file is named and the output ends with the reminder to
  set `SPEC_PURPOSE_AUDIT_ENABLED = True` in `scripts/workspace_check.py` in the
  same commit

#### Scenario: Applied state is answerable

- **WHEN** `--check` runs after the apply
- **THEN** it reports the backfill applied and exits 0; before the apply it exits
  3 and names the outstanding rows; and re-running after a successful apply is
  idempotent and still exits 0

#### Scenario: `--check` is not refused for drift

- **WHEN** `--check` runs while a committed plan exists and the fleet is fully
  applied, so every planned marker is legitimately absent
- **THEN** it reports the backfill applied and exits 0, because the drift refusal
  guards writes and a backfilled fleet is not drift

### Requirement: The spec-purpose audit is fenced off the fleet by a committed constant

Because 1,234 spec files hold the marker, `SPEC_PURPOSE_PLACEHOLDER` SHALL
be ERROR and the whole spec-purpose audit SHALL be held off the fleet by the
committed, repo-relative constant `SPEC_PURPOSE_AUDIT_ENABLED = False` in
`scripts/workspace_check.py`. The severity SHALL NOT be softened to WARNING to
achieve that hold. The constant SHALL NOT be readable from an environment
variable, and no skip flag or exemption registry SHALL exist for it. Flipping
the constant to `True` before the backfill is applied SHALL report ERROR on the
unmigrated corpus and turn the gate red; it SHALL NOT pass quietly. This is the
one and only fencing mechanism for this rule.

#### Scenario: The fence is closed on the fleet

- **WHEN** the audit runs against the un-backfilled fleet
- **THEN** no repository reports `SPEC_PURPOSE_PLACEHOLDER` and the fleet error
  count is unchanged by this rule

#### Scenario: The fence open turns the same corpus red

- **WHEN** the same corpus is audited with `SPEC_PURPOSE_AUDIT_ENABLED = True`
- **THEN** every un-backfilled spec reports ERROR `SPEC_PURPOSE_PLACEHOLDER` and
  the gate fails

#### Scenario: A real sentence after the marker is a purpose, not a defect

- **WHEN** a `## Purpose` body is the marker line followed on the same line by a
  real sentence, or followed by any further prose in the section
- **THEN** the audit reports no `SPEC_PURPOSE_PLACEHOLDER`, because the marker
  must be the whole section body to be a finding and the change name may not
  absorb trailing words

#### Scenario: The fence is not an environment variable

- **WHEN** the environment sets `SPEC_PURPOSE_AUDIT_ENABLED=1`
- **THEN** the audit reads the committed constant and the fence is unchanged

### Requirement: The check must be carried by a gate that blocks

A new ERROR code is not gated until a gate that blocks reports it. The
`governance-quality` concern in `driftwatchdog` runs `workspace_check.py` and
parses this checker's JSON envelope, but its effective code scope is the closed
allow-list `GOVERNANCE_QUALITY_DECLARED_CODES`; a code outside that list is
filtered out of the concern's verdict, and a manifest naming an undeclared code
is refused rather than trusted. `SPEC_PURPOSE_PLACEHOLDER` SHALL therefore be
added to that declared set in a separately authorized cross-repo step before
`SPEC_PURPOSE_AUDIT_ENABLED` is set to `True`. This change SHALL record the
unmet requirement as a blocker and SHALL NOT assert that the rule is gated.

#### Scenario: The declared set excludes the code

- **WHEN** the checker emits `SPEC_PURPOSE_PLACEHOLDER` and the
  `governance-quality` concern runs against the declared set as it stands
- **THEN** the finding is filtered out and the concern reports `PASS` while the
  checker exits 1, which is recorded as a blocker rather than as wiring

#### Scenario: A project cannot opt in from its own manifest

- **WHEN** a bound repository's manifest names `SPEC_PURPOSE_PLACEHOLDER` in its
  `governance_quality` code scope
- **THEN** the concern refuses it as `REVIEW_REQUIRED` naming the undeclared
  code, because a scope may shrink and never widen

### Requirement: The gate-wiring dependency is a recorded blocker, not a footnote

The unmet half of D18 SHALL be recorded as an explicit cross-repo dependency
naming the exact change required and where it lives, and `SPEC_PURPOSE_AUDIT_ENABLED`
SHALL remain `False` while it is unmet. Enabling the rule before the dependency
is met SHALL be treated as a defect of this change, because the checker would
exit non-zero on a fleet corpus the bound gate reports as `PASS`. This change
SHALL NOT edit the gate runtime, SHALL NOT assert that the rule is gated, and
SHALL NOT add a test in this repository that claims the wiring works.

#### Scenario: The dependency is named precisely

- **WHEN** this change is read for the follow-up it requires
- **THEN** it names adding `"SPEC_PURPOSE_PLACEHOLDER"` to
  `GOVERNANCE_QUALITY_DECLARED_CODES` in the gate runtime, and widening any
  bound manifest that names codes explicitly, as a separately authorized step

#### Scenario: The declared-code list is a known manual maintenance point

- **WHEN** a new finding code is added to the checker and not to the gate's
  declared set
- **THEN** the finding is filtered out and the gate reports `PASS`, which is
  recorded as a structural weakness with a named candidate guard and an explicit
  statement of what that guard does not fix
