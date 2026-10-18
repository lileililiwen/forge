# rules-baseline Specification

## Purpose

Give the workspace one canonical rules source and a non-destructive drift
report, so the shared baseline is unambiguous without overwriting the
project-specific guidance, wording and language that repositories legitimately
keep.
## Requirements
### Requirement: Single canonical source

The canonical rules text MUST live only in `templates/project/**`, and
`init_project.py` MUST read its output from those files rather than from an
embedded copy. The generated `.ai-rules/workflow.md` MUST contain the OpenSpec
name-preflight sentence. It MUST ALSO contain the work-on-`main` rule, naming
the user-asks exception, as canonical text rather than as a value computed by
or embedded in the initializer. The drift baseline MUST be a committed data file
at the governance root and MUST NOT be a script constant, a generated artifact,
or a second copy of the rules text: it records which clause findings exist, and
the canonical wording of a clause remains in `templates/project/**` only.

#### Scenario: Initializer uses the on-disk template

- **WHEN** `init_project.py` writes `.ai-rules/workflow.md` for a fresh project
- **THEN** the written file contains the OpenSpec name-preflight sentence taken
  from the on-disk template

#### Scenario: No second copy

- **WHEN** a canonical template changes
- **THEN** the initializer's output changes with it and no embedded duplicate
  needs editing

#### Scenario: The branch rule travels with the template

- **WHEN** the work-on-`main` sentence changes in
  `templates/project/.ai-rules/workflow.md`
- **THEN** the text `init_project.py` writes to a new project changes with it,
  and no initializer edit is required for the change to reach a new project

#### Scenario: The drift baseline is not a second source of the rules

- **WHEN** a clause's needle or wording changes in `templates/project/**`
- **THEN** the drift baseline is not edited to match, because it records
  finding keys and not clause text, and the new wording is decided in the
  templates alone

### Requirement: Drift is reported, never rewritten

`rules_drift.py` MUST report, for each adopted project, every invariant clause
that its `.ai-rules` files do not visibly carry, and MUST NOT modify any project
file. A rule file carrying the opt-out marker
`<!-- workspace-governance:rules-customized -->` MUST be skipped. The clause
set for `.ai-rules/workflow.md` MUST include the work-on-`main` rule, so that a
project which never states where its work happens is reported. The report MUST
remain the detailed, non-blocking surface for clause-level drift, and
`workspace_check.py` MUST NOT re-emit its findings as gate findings except for
the keys absent from the drift baseline. Adding a clause to the clause set MUST
NOT by itself change the exit code of any gate; only a clause finding that is new
relative to the committed baseline is an input to a gate.

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

#### Scenario: The work-on-`main` clause is reported like any other clause

- **WHEN** an adopted project's `.ai-rules/workflow.md` states no
  work-on-`main` rule
- **THEN** the report names the project, the file and the
  `work-on-main rule` clause, every scanned rule file is byte-identical
  afterwards, and the run's exit code is unchanged from its behaviour before
  the clause existed

#### Scenario: The clause does not block anything

- **WHEN** a clause is reported and its `(project, file, label)` key is present
  in `rules-drift-baseline.json`
- **THEN** the report names the finding, `workspace_check.py` reports no gate
  finding for it, and `workspace_check.py`, `auto_gate.py` and CI return the same
  result as before, because the report remains a triage report whose findings
  are reviewed rather than enforced, and because the ratchet is a baseline
  comparison rather than a report run

#### Scenario: A clause finding outside the baseline does block

- **WHEN** the same clause is reported for a project, file and label that
  `rules-drift-baseline.json` does not contain
- **THEN** `workspace_check.py` reports ERROR `RULES_DRIFT_NEW` naming the
  project, the file and the clause, and `auto_gate.py` returns `FAIL`

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

### Requirement: The work-on-`main` rule is canonical baseline text

Workspace Governance SHALL state, in
`templates/project/.ai-rules/workflow.md` and in
`templates/project/AGENTS.md`, that work happens directly on `main` by default
and that a branch or worktree MUST NOT be created **unless the user asks for
one or designates one**. The rule SHALL name both the branch and the worktree
as things that require that exception, because a rule that names only a branch
is satisfied by a worktree. The canonical text SHALL live only in
`templates/project/**` and MUST NOT be embedded in a script. The canonical
source is the single source for the wording, so the same sentence MUST appear
in the initializer's output and in this repository's own rule files.

#### Scenario: The canonical source carries the rule and its exception

- **WHEN** `templates/project/.ai-rules/workflow.md` or
  `templates/project/AGENTS.md` is read
- **THEN** it states that work is done directly on `main` by default, that
  neither a branch nor a worktree is created without the user's request or
  designation, and it names the user as the authority that can grant the
  exception

#### Scenario: The rule is not copied into a script

- **WHEN** the canonical templates are searched for the rule
- **THEN** the only occurrence is under `templates/project/**`, and no script
  embeds a second copy of the sentence

#### Scenario: This repository satisfies its own rule

- **WHEN** this repository's own `AGENTS.md` and `.ai-rules/workflow.md` are
  read
- **THEN** both carry the same canonical sentence, and the drift report names
  no work-on-`main` finding for this repository

### Requirement: A new project receives the rule from the initializer

`init_project.py` SHALL deliver the work-on-`main` rule to every project it
onboards, by copying the canonical templates from disk. The rule MUST NOT
require a new entry in the initializer's template list, and onboarding MUST NOT
replace a project's existing `AGENTS.md` text. No project file may be rewritten
by this delivery.

#### Scenario: Initializer output carries the rule

- **WHEN** `init_project.py` writes `.ai-rules/workflow.md` and `AGENTS.md` for
  a fresh project
- **THEN** both written files contain the work-on-`main` rule taken from the
  on-disk templates, and the initializer's own source is unchanged

#### Scenario: An existing project's own text survives

- **WHEN** `init_project.py` is run against a project that already has its own
  `AGENTS.md`
- **THEN** the existing local content is preserved and no project file is
  rewritten in order to deliver the rule

### Requirement: The branch rule is machine-reported as a baseline clause

`rules_drift.py` SHALL carry the work-on-`main` rule as an invariant clause for
`.ai-rules/workflow.md`, labelled `work-on-main rule`, with a needle chosen for
stability across re-wording. An adopted project's `.ai-rules/workflow.md` that
does not visibly carry the clause SHALL be reported as WARNING `RULES_DRIFT`
naming the project, the file and the clause. The clause SHALL be appended to
the existing clause set only: the report's finding shape, its exit code and its
other three clauses MUST be unchanged. The clause MUST be reported as triage;
because no gate runs the report, nothing in this requirement SHALL be
represented as blocking, failing, or enforcing a commit.

#### Scenario: The clause fires when the rule is absent

- **WHEN** an adopted project's `.ai-rules/workflow.md` states no
  work-on-`main` rule
- **THEN** the report names the project, the file, and the
  `work-on-main rule` clause, and the same run names that project for no other
  reason than the clause's absence

#### Scenario: The clause is silent when the rule is present

- **WHEN** an adopted project's `.ai-rules/workflow.md` carries the canonical
  work-on-`main` sentence
- **THEN** the report names no `work-on-main rule` finding for that project

#### Scenario: The needle is not the template's own phrasing

- **WHEN** the clause needle is selected
- **THEN** it is a span that the canonical sentence itself contains, and it is
  not a phrase that zero of the workspace's rule files carry, so that the
  clause discriminates on the first run instead of reporting every project

#### Scenario: The report is triage, not a gate

- **WHEN** a work-on-`main` finding is reported
- **THEN** it is a WARNING in the drift report's own output, and no gate
  exit code changes because of it, because no gate invokes the report

### Requirement: A repository that documents the rules in its own words stays silent

The per-file opt-out marker `<!-- workspace-governance:rules-customized -->`
SHALL continue to suppress every invariant clause for the file that carries it,
including the work-on-`main` clause, and the marker MUST NOT gain a
scope, category or global form. The clause's needle MAY be a span that
recognises one phrasing of the rule over another, because the per-file marker is
the designed resolution for a repository that states the rule in its own words.
`AGENTS.md` SHALL NOT be added to the report's rule files in this change: the
opt-out marker does not yet exist in those files, so the resulting findings
would have no honest per-file suppression route.

#### Scenario: The marker suppresses the new clause

- **WHEN** a project documents the rules in its own words and its
  `.ai-rules/workflow.md` carries the opt-out marker
- **THEN** no finding is reported for that file, including no
  `work-on-main rule` finding, and no other clause is lost

#### Scenario: A different phrasing is resolved deliberately

- **WHEN** a repository states the work-on-`main` rule in wording the clause
  needle does not match
- **THEN** the report names that one file, and the resolution is a marker line
  in that file recording the deliberate wording, not a change to the needle
  and not a global suppression

#### Scenario: No new switch is introduced

- **WHEN** the clause's opt-out behaviour is inspected
- **THEN** it is the existing per-file marker and nothing else — no
  environment variable, no exemption registry, and no category-wide escape

### Requirement: New clause drift blocks the gate, and baselined clause drift does not

`workspace_check.py` MUST compare the current clause-level `RULES_DRIFT` findings
of every adopted project against the committed drift baseline, keyed on the
triple `(project, file, label)`. A finding whose key is **absent** from the
baseline MUST be reported by the workspace audit as ERROR `RULES_DRIFT_NEW`,
naming the project, the file and the clause, so that `auto_gate.py` returns
`FAIL`. A finding whose key is **present** in the baseline MUST NOT be reported
as a gate finding. `rules_drift.py --check` SHALL remain the detailed report
that names every clause-level finding, and `workspace_check.py` MUST NOT
re-emit the baselined findings, because a delta-only audit is what keeps the
shipped 197 errors and 24 warnings readable.

#### Scenario: Drift absent from the baseline blocks

- **WHEN** an adopted project reports a clause-level finding whose
  `(project, file, label)` triple is not in the drift baseline
- **THEN** the workspace audit reports ERROR `RULES_DRIFT_NEW` naming the
  project, the file and the clause, and `auto_gate.py` returns `FAIL`

#### Scenario: Drift present in the baseline does not block

- **WHEN** an adopted project reports a clause-level finding whose
  `(project, file, label)` triple is in the drift baseline
- **THEN** the workspace audit reports no gate finding for it, `auto_gate.py`
  returns the same result as before the finding existed, and the finding is still
  named in full by `rules_drift.py --check`

#### Scenario: The fleet gate does not move when the ratchet lands

- **WHEN** the drift baseline holds exactly the clause-level findings the fleet
  reports today
- **THEN** the audit's error and warning counts are unchanged, no
  `RULES_DRIFT_NEW` finding is reported, and no project file is modified

#### Scenario: The detailed report stays the report

- **WHEN** the workspace audit runs with a fully matching baseline
- **THEN** the audit emits only the delta and does not re-emit the baselined
  clause findings as audit lines, and `rules_drift.py --check` still prints every
  clause-level finding it printed before

### Requirement: The baseline nags when it can shrink, and never fails

When the current clause-level finding set is a **strict subset** of the drift
baseline, the workspace audit SHALL report exactly one WARNING
`RULES_DRIFT_BASELINE_STALE` naming how many baseline entries are now redundant.
The warning MUST NOT change the audit's exit code or the gate's status, so that
a repository that fixes its drift never turns the fleet red. A repository that
SHALL be made silent by a baseline entry instead of a fix is not the mechanism
this warning provides: the warning exists so the baseline is reduced to the
drift that still exists.

#### Scenario: A shrunken baseline is reported

- **WHEN** one or more baseline entries no longer match any current finding
- **THEN** the audit reports a single WARNING `RULES_DRIFT_BASELINE_STALE`
  naming the number of redundant entries and the baseline file to shrink

#### Scenario: The warning never fails the gate

- **WHEN** `RULES_DRIFT_BASELINE_STALE` is reported
- **THEN** the audit's error count and exit code are unchanged, and `auto_gate.py`
  returns the same status as before the warning existed

#### Scenario: An exactly matching baseline is silent

- **WHEN** the current finding set equals the baseline and is not a strict
  subset of it
- **THEN** no `RULES_DRIFT_BASELINE_STALE` warning is reported

### Requirement: The drift baseline is a committed data file that fails closed

The drift baseline SHALL be a committed JSON data file at the governance root
holding a flat set of `{"project", "file", "label"}` keys, and MUST NOT be a
script constant, a generated artifact, or a value any code path can extend. A
missing, unparseable, non-object, empty, or malformed baseline MUST be reported
as a hard non-zero ERROR naming the file, and the audit MUST NOT fall back to an
empty set, because an empty baseline would present every known finding as new
drift and fail the fleet for the wrong reason. The baseline records which clause
findings exist; it MUST NOT carry rules text, and the canonical wording of a
clause SHALL remain only in `templates/project/**`.

#### Scenario: The baseline is data, not a constant

- **WHEN** the baseline's location and format are inspected
- **THEN** it is one committed JSON data file at the governance root, and no
  script constant, default value, or code path writes a finding key into it

#### Scenario: A missing baseline fails closed

- **WHEN** `rules-drift-baseline.json` does not exist
- **THEN** the audit reports a non-zero ERROR naming the file as missing, and
  reports no `RULES_DRIFT_NEW`

#### Scenario: An unparseable baseline fails closed

- **WHEN** `rules-drift-baseline.json` is not valid JSON, or its top level is
  not a JSON object
- **THEN** the audit reports a non-zero ERROR naming the file, and reports no
  `RULES_DRIFT_NEW`

#### Scenario: An empty baseline fails closed

- **WHEN** `rules-drift-baseline.json` has no `findings` list, or an empty one
- **THEN** the audit reports a non-zero ERROR naming the file as empty rather
  than treating the baseline as an empty set and failing every project

#### Scenario: A malformed entry fails closed

- **WHEN** a baseline entry omits `project`, `file` or `label`, or carries a key
  the format does not define
- **THEN** the audit reports a non-zero ERROR naming the offending entry rather
  than silently dropping the rows it could not read

#### Scenario: The baseline carries no rules text

- **WHEN** the baseline file is read
- **THEN** it holds only project, file and clause-label keys, and the canonical
  wording of a clause is present only under `templates/project/**`

### Requirement: The ratchet has no escape hatch, and growth is an argued diff

The clause-level ratchet MUST NOT gain an environment variable, a skip flag, a
per-project exemption registry, or a category-wide escape. The per-file opt-out
marker `<!-- workspace-governance:rules-customized -->` honoured by
`rules_drift.OPT_OUT` SHALL remain the only per-repository escape, and it
suppresses a clause at source so that no baseline entry is required. Adding a
key to the baseline SHALL be permitted, and MUST be a review-visible diff that is
argued on its merits exactly as the legacy-divergent list is argued. Silencing a
repository that legitimately documents the shared rules in its own words MUST be
done with the marker in that repository and MUST NOT be done by growing the
baseline. The baseline is a ratchet snapshot of the drift that exists, not a
permanent allowance, and it can only be tightened by removing keys that no longer
match.

#### Scenario: No switch turns the ratchet off

- **WHEN** the ratchet's opt-out behaviour is inspected
- **THEN** there is no environment variable, no skip flag, no exemption registry
  and no category escape, and the only per-repository resolution is the
  per-file marker

#### Scenario: The marker needs no baseline entry

- **WHEN** a project documents the rules in its own words and its rule file
  carries the opt-out marker
- **THEN** the clause is suppressed before any key is computed, and the baseline
  contains no entry for that project, file and clause

#### Scenario: Growth is visible in review

- **WHEN** a key is added to the baseline so that a new drift finding does not
  block
- **THEN** the addition is a one-line diff naming the project, the file and the
  clause, and it is expected to be argued rather than accepted as a
  configuration change

#### Scenario: The baseline tightens by removing keys

- **WHEN** a repository fixes a clause finding so that its key matches nothing
  any more
- **THEN** the key is removed from the baseline, the audit reports
  `RULES_DRIFT_BASELINE_STALE` until it is, and the removal is the only
  direction the baseline moves without a governance argument
