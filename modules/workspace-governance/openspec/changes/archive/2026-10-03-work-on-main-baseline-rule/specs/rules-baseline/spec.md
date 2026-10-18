# rules-baseline Specification (delta: work-on-main-baseline-rule)

## ADDED Requirements

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

## MODIFIED Requirements

### Requirement: Single canonical source

The canonical rules text MUST live only in `templates/project/**`, and
`init_project.py` MUST read its output from those files rather than from an
embedded copy. The generated `.ai-rules/workflow.md` MUST contain the OpenSpec
name-preflight sentence. It MUST ALSO contain the work-on-`main` rule, naming
the user-asks exception, as canonical text rather than as a value computed by
or embedded in the initializer.

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

### Requirement: Drift is reported, never rewritten

`rules_drift.py` MUST report, for each adopted project, every invariant clause
that its `.ai-rules` files do not visibly carry, and MUST NOT modify any project
file. A rule file carrying the opt-out marker
`<!-- workspace-governance:rules-customized -->` MUST be skipped. The clause
set for `.ai-rules/workflow.md` MUST additionally include the work-on-`main`
rule, so that a project which never states where its work happens is reported.
Adding that clause MUST increase only the report's finding count, and MUST NOT
change the exit code of any gate, because the report is not invoked by any
gate.

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

- **WHEN** the clause is added to the clause set
- **THEN** `workspace_check.py`, `auto_gate.py` and CI return the same result
  as before, and the report remains a triage report whose findings are
  reviewed rather than enforced
