# Proposal: Make "work on `main`, no branch unless asked" canonical baseline text

## Why

An agent that decides to open a branch has made a decision nobody asked it to
make, and it will usually make it for a good reason — isolation, tidiness, a
cheap way to abandon a change. That is exactly the case the workspace does not
want, because a branch splits one change across two histories and both have to
be re-audited, reconciled and committed under the two-commit protocol in
`.ai-rules/completion.md`. The rule exists in this portfolio, but it exists
**informally, in a minority of repositories**, and it is absent from the one
place a canonical rule is supposed to live.

The measured shape, 2026-10-03:

- **81** `.project.json` declarations; **80** repositories have a
  `.ai-rules/workflow.md`.
- **9 of those 80** `.ai-rules/workflow.md` files carry any branch / `main` /
  worktree language at all: `argoscope`, `careBridge`, `chronicleaf`,
  `kairion`, `kairovia`, `openTeacherKit`, `retakeKit`, `therapist-commons`,
  `trailCrew`. The remaining 71 say nothing about where work happens.
- **15 of 80** `AGENTS.md` files state a work-on-`main` rule: `alethefy`,
  `argoscope`, `careBridge`, `chronicleaf`, `citylens`, `devloom`, `fotofy`,
  `kairion`, `kairovia`, `openTeacherKit`, `retakeKit`, `shopifireman`,
  `sutravia`, `therapist-commons`, `trailCrew`.

So 15 of 80 repositories carry the rule in the file an agent reads **first**,
and 9 of 80 carry anything resembling it in the workflow file the canonical
baseline is written to. `templates/project/.ai-rules/workflow.md` — the single
canonical source `scripts/init_project.py` copies through its `TEXT_TEMPLATES`
list — does not mention branches at all. A repository onboarded today is
therefore told nothing about git state, and an agent that asks no question
defaults to a branch.

**The template already half-states the rule, and the half it states is the
wrong half.** `templates/project/AGENTS.md` reads "Work directly on `main` by
default; do not create a branch or worktree without a designated purpose."
That names a condition — *without a designated purpose* — and leaves the
designation's source unstated. Under the shipped `rules-baseline` spec nothing
counts that sentence as the branch rule either: `rules_drift.py` `CLAUSES` for
`.ai-rules/workflow.md` carries exactly three clauses, and none of them is
about branches. The result is a rule that is real in the template, invisible to
the report, and stated without the exception that makes it usable.

**The exception is the load-bearing half.** A rule with no stated exception is
one an agent breaks for convenience: "do not create a branch" is disobeyed the
first time isolating a change looks cheaper than the two-commit protocol, and
the agent's justification is always reasonable. The rule only holds if it says
*unless the user asks for one or designates one* — which converts a preference
into a contract with a named authority.

**The report that could see this does not gate anything.** This is the fact
that decides the shape of the whole change, so it is stated before anything
else. `scripts/rules_drift.py` is **not wired into any gate**:
`scripts/workspace_check.py` imports only `rules_drift.scaffold_handoff` (see
`workspace_check.py:2133`), and neither the `drift()` function nor the
`--check` report is invoked by `workspace_check.py`, by
`scripts/auto_gate.py`, or by `.github/workflows/governance.yml`. The shipped
`rules-baseline` spec itself designates the tool a **triage report** ("Drift is
reported, never rewritten"). Adding a clause to a report that no gate runs
**does not enforce anything.** This workspace has already ruled on that
pattern: decision **D18** in `ui-shared-layer-reachability/design.md` — "A
verifier that no gate runs does not gate; wire every check into the gate that
blocks, or it is documentation" — decided in favour of wiring. This change
therefore cannot present a new `CLAUSES` entry as enforcement, and does not
try to. What it delivers is stated exactly in §4 and the follow-up is named.

## What Changes

- **One canonical sentence, in the canonical source.**
  `templates/project/.ai-rules/workflow.md` and `templates/project/AGENTS.md`
  both carry the identical rule text, and the rule is **not** embedded in any
  script. `init_project.py` already reads both files from disk through
  `TEXT_TEMPLATES`; nothing new is added to the initializer.
- **The rule names the exception.** The wording is a contract, not a
  preference: work on `main` by default, and do not create a branch or
  worktree **unless the user asks for one or designates one**.
- **The repository dogfoods it.** This repository's own `AGENTS.md` and its own
  `.ai-rules/workflow.md` carry the same sentence. The second is not optional:
  `workspace-governance` is itself an adopted project in `projects.json`, so
  the report scans this repository too, and a clause this repository does not
  satisfy is a finding against this repository.
- **The clause becomes machine-reported.** `rules_drift.py` `CLAUSES` for
  `.ai-rules/workflow.md` gains one entry, label `work-on-main rule`, needle
  `` on `main` `` — chosen for stability from a measured sensitivity table, and
  explicitly **not** `do not create a branch`.
- **The skill is the global carrier.** `skills/workspace-governance/SKILL.md`
  gains the rule in its routing table and its boundaries, and
  `skills/workspace-governance/references/rules-baseline.md` states it, so an
  agent that enters through the front door receives it without first reading
  the templates.
- **The gate gap is named, not papered over.** `rules_drift.check()` is wired
  into the gate that blocks as a **separate follow-up change**, recorded in
  `ROADMAP.md` and `decisions.json` by this change's implementation. This
  change does not claim enforcement and does not add a second unwired check
  silently — it adds one clause to an already-existing, already-designated
  report, and states in the shipped spec that the report is triage.

## Package Boundary and Split Assessment

This is one package because the rule, its canonical location, its detection
needle and its carrier are one text with one oracle: after implementation, a
project either visibly carries the work-on-`main` rule or it is named. Splitting
the canonical text from the clause would ship a rule no report can see; splitting
the clause from the skill would ship a rule no agent entering through the front
door receives.

| Package | Single outcome | Owner / language | Boundary and contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `work-on-main-baseline-rule` (this) | Every agent entering the workspace by any surface is told the work-on-`main` rule, and the rule is machine-reported by the existing drift report | `workspace-governance`, Python 3.11 stdlib | `templates/project/**`; `rules_drift.CLAUSES`; `SKILL.md`; `references/rules-baseline.md`; two `AGENTS.md` files | shipped `rules-baseline` (`Single canonical source`, `Drift is reported, never rewritten`) | the clause fires on a fixture that omits it and is silent on the canonical template; the opt-out marker still suppresses it; no project file is rewritten |
| `rules-drift-gate-wiring` (**follow-up, not this change**) | `RULES_DRIFT` becomes a finding the audit emits and a gate can block on | `workspace-governance`, Python 3.11 stdlib | `workspace_check.run` calls `rules_drift.check()`; severity and blocking decided there | this change's clause set | `auto_gate.py` and `.github/workflows/governance.yml` fail on a drifted adopted project |

The follow-up is **not** bundled here for one concrete reason: wiring
`rules_drift.check()` into `workspace_check.run` makes **every** adopted
project's rule drift a candidate audit finding at once, and the fleet currently
carries `drift_findings=80` before this clause exists. Wiring today would turn
a triage report into a red gate in the same commit that widens the report. The
ordering matters: the clause set stabilizes first, the fleet is worked down,
and the wiring lands against a report whose findings are actionable.

## BFS Impact Map

- **Actors:** an agent onboarding a new project (`init_project.py`); an agent
  reading `AGENTS.md` in an existing repository; an agent entering through
  `skills/workspace-governance/SKILL.md`; the drift-report reader.
- **State:** a repository's rule files move `unstated → stated` (this
  repository and the canonical template), and a repository's `.ai-rules/
  workflow.md` moves `unreported → reported` for the new clause. No project
  file's bytes change.
- **Contracts:** `CLAUSES[".ai-rules/workflow.md"]` gains
  `("work-on-main rule", "on `main`")`. `RULE_FILES` does **not** change.
- **Data integrity:** the report is read-only by construction — it opens rule
  files for reading and never writes. The regression test asserts
  byte-equality of the rule files across a report run, and that assertion is
  extended, not replaced.
- **Failure and boundary:** an unreadable rule file keeps reporting
  `RULES_UNREADABLE`; the opt-out marker keeps short-circuiting the whole file
  before any clause is evaluated. A repository that carries the rule in its own
  words and carries the marker is silent.
- **Compatibility:** the clause raises the report's finding count. That count is
  a report total, not an audit error, and no existing consumer parses it —
  `maturity_audit.py` reports `workspace_check.py` counts only, and CI runs
  `unittest`, `auto_gate.py --self-check` and `git diff --check`. No gate's exit
  code changes in this change.
- **Security:** no project command is executed, no network call, no sibling
  repository is written. The change edits two template files, two files in
  this repository, one Python constant, the skill, one reference, and the
  test file.
- **Tests:** clause fires when absent; clause silent when the canonical
  template text is present; the opt-out marker still suppresses the new clause;
  the report remains byte-identical; the templates on disk carry both the rule
  and its exception.
- **Dependencies:** Python 3.11 standard library only.

## Capabilities

- **New:** `work-on-main-baseline`.
- **Modified:** `rules-baseline` — the shipped `Single canonical source` and
  `Drift is reported, never rewritten` requirements are **extended, not
  weakened**. Every existing scenario of both is preserved verbatim in the
  delta's `## MODIFIED Requirements` section, including the
  `MUST NOT modify any project file` guarantee and the opt-out scenario.

## Non-goals

- **No bulk edit of the other repositories.** This change does not touch any
  sibling's `AGENTS.md` or `.ai-rules/`. Nine of 80 already carry branch
  language and 15 of 80 already state the rule; reconciling the rest is
  per-repository work with its own authorization, and `AGENTS.md` forbids
  editing a sibling without scope authorization.
- **`AGENTS.md` is not added to `RULE_FILES`.** Adding it would surface roughly
  71 new findings at once across 80 files, and the per-file opt-out marker does
  not exist in those files yet, so those findings would have no honest
  suppression route. This is recorded as an explicit non-goal with a named
  precondition for the follow-up.
- **No gate wiring.** `rules_drift.check()` is not called from
  `workspace_check.py`, `auto_gate.py`, or CI in this change. See R6.
- **No change to `drift()`'s signature, the finding shape, the exit code, or
  the `RULES_DRIFT` / `RULES_UNREADABLE` codes.** The clause is a tuple in an
  existing tuple.
- **No new finding code** for the branch rule. `RULES_DRIFT` already says
  "this file does not carry a canonical clause", and the label says which one.
- **No new dependency, no network, no project command**, and no file-tree
  inference of whether a repository "really" works on `main`. The clause reads
  text; it cannot and does not inspect git history.
- **No promotion of any repository's evidence state** and no change to
  `profile_requirements`, the registry, or any `.project.json`.
- **No weakening of the opt-out marker, and no new opt-out mechanism.** The
  marker ships in `rules-baseline` today and is honoured per file; this change
  adds no switch of its own.

## Owner rulings — decided

The owner has ruled on all seven questions below. They are binding on the
wording, the file set, the needle, the severity claims and the follow-up
boundary. The same rulings in design terms are `design.md` §12.

1. **The rule is canonical baseline text (R1).** It lives in
   `templates/project/.ai-rules/workflow.md` — the single canonical source
   `scripts/init_project.py` copies via its `TEXT_TEMPLATES` list — and in
   `templates/project/AGENTS.md`. It is **not** embedded in any script. The
   shipped `Single canonical source` requirement already forbids a second
   embedded copy, and this change must not become the reason that rule is
   relaxed.
2. **The rule names an exception, not a preference (R2).** The wording must
   make the exception explicit: work directly on `main` by default; do **not**
   create a branch or worktree **unless the user asks for one or designates
   one**. Rationale: a rule with no stated exception is one an agent will
   break for convenience, and it will justify the break. The current template
   sentence conditions on "a designated purpose" without naming who designates
   it, which is why it is replaced rather than kept.
3. **The rule also lands in `AGENTS.md` files, not only `.ai-rules` (R3).** An
   agent reads `AGENTS.md` before it reads `.ai-rules/`, and 15 of 80
   repositories already state the rule there. This change updates
   `templates/project/AGENTS.md` and this repository's own `AGENTS.md`, and
   does **not** bulk-edit the other repositories.
4. **The clause is added to `rules_drift.py` `CLAUSES` with a needle chosen
   for stability (R4).** The measured sensitivity of every candidate needle
   over the 80 `workflow.md` files is recorded in `design.md` §5. The needle is
   **not** `do not create a branch`: **0 of 80** files carry it, so the clause
   would be 100% red on day one and discriminate nothing while adding 80
   findings of pure noise. A needle that is red everywhere is not a check — it
   is an alarm that is always sounding.
5. **`AGENTS.md` is not added to `RULE_FILES` in this change (R5).** Adding it
   would surface roughly 71 new findings at once across 80 files, and the
   per-file opt-out marker `<!-- workspace-governance:rules-customized -->`
   does not yet exist in those files, so the findings would have no honest
   per-file suppression route. **Named precondition for the follow-up:** the
   opt-out marker must be present in the files that legitimately document the
   rules in their own words, and the finding count must be worked down to a
   reviewable number *before* `AGENTS.md` enters `RULE_FILES`.
6. **Resolve the gate tension head-on (R6).** State plainly that adding a
   clause to a report that no gate runs does not, by itself, enforce anything,
   and that under D18 an unwired check is documentation. Then state what this
   change does about it. The honest answer: this change makes the clause
   **reported and reviewable**, keeps the tool's shipped designation as a triage
   report, and names full gate wiring as a **separate follow-up**. No claim of
   enforcement appears in this change, in its spec, or in its tasks.
7. **The skill is the global carrier (R7).**
   `skills/workspace-governance/SKILL.md` routes agents into this repository for
   governance situations, so the branch rule is added to that skill's routing
   table and boundaries, and
   `skills/workspace-governance/references/rules-baseline.md` is updated.
   `scripts/check_skill_refs.py` proves every cited path in those files exists,
   so any path added to a reference must exist at validation time.
