# Design: work-on-main-baseline

## 1. Implementation boundary

**Repository:** `/home/paul/code/workspace-governance` — Python 3.11, standard
library only. No sibling repository source is edited.

**Files to change:**

- `templates/project/.ai-rules/workflow.md` — add the rule sentence.
- `templates/project/AGENTS.md` — replace the existing partial branch sentence
  with the canonical one, keeping its second sentence about unrelated
  uncommitted work.
- `AGENTS.md` (this repository) — add the canonical sentence as one bullet in
  the top-level instruction list.
- `.ai-rules/workflow.md` (this repository) — add the canonical sentence, so
  this repository satisfies the clause it ships (§4.6).
- `scripts/rules_drift.py` — one tuple appended to
  `CLAUSES[".ai-rules/workflow.md"]`. No other edit to the file.
- `skills/workspace-governance/SKILL.md` — one routing-table row and one
  boundaries bullet.
- `skills/workspace-governance/references/rules-baseline.md` — state the rule
  and the carrier surfaces.
- `tests/test_rules_drift.py` — extend the `WORKFLOW` fixture with the clause
  and add the regression tests.
- `ROADMAP.md`, `decisions.json` — record the named gate-wiring follow-up
  (R6), so the gap is tracked in the repository and not only in this change.

**Must not change:** `RULE_FILES`, `drift()`, `check()`, `scaffold_handoff()`,
`read_text()`, `OPT_OUT`, `RULES_DRIFT`, `RULES_UNREADABLE`,
`HANDOFF_SCAFFOLD`, the report's exit code, `init_project.py`,
`TEXT_TEMPLATES`, `workspace_check.py`, `auto_gate.py`,
`.github/workflows/governance.yml`, `profile_requirements`, any
`.project.json`, and every sibling repository.

## 2. Language and runtime

Python **3.11**, standard library only.

- `python3 -m unittest discover -s tests -v`
- `python3 scripts/rules_drift.py --root .. --check`
- `python3 scripts/check_skill_refs.py`
- `python3 scripts/workspace_check.py --self-check`
- `python3 scripts/auto_gate.py --self-check`
- `node scripts/check-openspec-change-names.mjs`
- `openspec validate work-on-main-baseline --strict --no-interactive`
- `openspec validate --all --strict --no-interactive`
- `git diff --check`

## 3. Ownership and shared code

- **Owner:** `workspace-governance`. No sibling owns the shared rules text, so
  no extraction is authorized.
- **Exported contract:** the canonical rule sentence, in
  `templates/project/**`; the clause tuple in `rules_drift.CLAUSES`.
- **Consumers:** `init_project.py` (onboarding, unchanged code path), an agent
  reading `AGENTS.md` or `.ai-rules/workflow.md`, an agent entering through the
  front-door skill, and the drift-report reader.
- **Dependency direction:** products → governance. The rule travels outward
  from the templates; no product repository is asked to host it.
- **Release boundary:** ships with this repository. A product repository
  receives the rule the next time it is onboarded or re-baselined.

## 4. The propagation chain

The rule is one sentence with five carriers. The chain is deliberately
one-directional: the canonical text is authored once, and every other surface
either copies it or links to it.

```
templates/project/.ai-rules/workflow.md   (authored here, R1)
        │
        ├── scripts/init_project.py TEXT_TEMPLATES  → every new project
        │
        └── needles: rules_drift.CLAUSES  (R4)
                     └── scripts/rules_drift.py --check  → triage report

templates/project/AGENTS.md               (authored here, R1 + R3)
        ├── scripts/init_project.py TEXT_TEMPLATES  → every new project
        └── this repository's own AGENTS.md          (R3)
             └── an agent reads AGENTS.md first

skills/workspace-governance/SKILL.md routing + boundaries   (R7)
        └── references/rules-baseline.md  → the canonical templates + the spec
```

### 4.1 The exact intended wording

One sentence, byte-identical in all four text carriers:

> Work directly on `main` by default; do not create a branch or worktree
> unless the user asks for one or designates one.

Rendered as a single line in each file:

```
Work directly on `main` by default; do not create a branch or worktree unless the user asks for one or designates one.
```

Why this sentence and not a paraphrase per file:

- **`on `main`` is present**, so the canonical template satisfies the clause
  needle (§5) without the template and the report disagreeing on day one.
- **"by default"** states the normal case, so an agent that needs isolation is
  not reading it as a ban.
- **"do not create a branch or worktree"** names both escape hatches. A rule
  that names only "branch" is satisfied by a worktree, and a worktree is the
  cheaper way to violate it.
- **"unless the user asks for one or designates one"** is the exception, and
  it names its authority: the user. This is the part the current template
  sentence is missing, and it is the part that makes the rule survivable
  against convenience.
- It is one sentence, so a re-wrapping repository can carry it without
  restructuring its file, and it has no project-specific noun in it.

In `templates/project/AGENTS.md` the sentence replaces the first half of the
existing bullet; the trailing sentence "Preserve unrelated uncommitted work and
never sweep it into a commit." is kept unchanged. In this repository's
`AGENTS.md` it is added as a new bullet in the top-level list, next to the
BFS line, because an agent reads that list before the themed sections.

### 4.2 `init_project.py`

`TEXT_TEMPLATES` already contains `"AGENTS.md"` and
`".ai-rules/workflow.md"`, `load_templates()` reads both from disk, and
`render()` substitutes `__PROFILE__`, `__CURRENT_SPEC__`, `__PROJECT_ID__` and
`__VERIFICATION_COMMAND__`. The rule sentence contains none of those tokens, so
**no initializer change is required** — that is the shipped
`Single canonical source` requirement working as designed, and the
"no second copy" scenario is what proves it. The regression test asserts the
loaded template text, so a future edit that drops the sentence fails a test
rather than a code review.

### 4.3 `rules_drift.py` `CLAUSES`

```python
".ai-rules/workflow.md": (
    ("BFS workflow model", "BFS"),
    ("two-commit rule", "two commits"),
    ("OpenSpec name preflight", "check-openspec-change-names"),
    ("work-on-main rule", "on `main`"),
),
```

The label style matches its neighbours: a short noun phrase, no project noun.
Appending a tuple is the whole code change; `drift()` already iterates
`CLAUSES.items()` and appends one `RULES_DRIFT` finding per missing needle.

### 4.4 The front-door skill

`SKILL.md` gains a routing row so a git-state question has somewhere to land:

| Situation | Route |
|---|---|
| Where should this work happen — branch, worktree, or `main` | `skills/workspace-governance/references/rules-baseline.md` (`templates/project/AGENTS.md`) |

and a boundaries bullet carrying the rule itself, because a skill's boundaries
are read by an agent that never opens a repository. Every path cited in either
file must exist: `check_skill_refs.py` extracts each backticked span and each
`python3 scripts/*.py` command and exits non-zero on a missing one.

`references/rules-baseline.md` states the rule, names the two canonical
carriers, and keeps the existing "never copy a second set of the rules into a
script" instruction. It links the rule; it does not become a second source.

### 4.5 Per-repository `AGENTS.md`

Not edited in this change, by R3. The 15 repositories that already state the
rule are left byte-identical; the 65 that do not are reported by
`rules_drift.py` (§6) and worked down as product-owned follow-up. The
`AGENTS.md` clause itself is a named non-goal with a named precondition (R5).

### 4.6 Self-conformance

`workspace-governance` is itself `adoption: adopted` in `projects.json`, so
`check()` scans this repository's own rule files. Today it reports one
`RULES_DRIFT` finding for this repository
(`.ai-rules/completion.md` does not carry the evidence boundary) and **none**
for `.ai-rules/workflow.md`, which already carries all three needles. If the
new clause shipped without this repository adopting the rule, the report would
name this repository for the clause it invented. The rule therefore lands in
this repository's own `.ai-rules/workflow.md` in the same change, and the
regression includes an assertion that
`python3 scripts/rules_drift.py --root .. --check` reports no `work-on-main`
finding for `workspace-governance`. The alternative — marking this repository's
workflow file with the opt-out — is rejected: the marker suppresses every
clause in that file, including the three the file already satisfies.

## 5. The needle choice

`CLAUSES` needles are "short and stable so they survive re-wrapping". A needle
is a detector for *presence of a rule*, and it has two failure modes: too
specific, so a repository that states the rule in its own words is reported; too
generic, so a repository that never states the rule passes. Measured over the
**80** `workflow.md` files, 2026-10-03:

| Candidate needle | Files lacking it | Files carrying it | Day-one finding rise | Verdict |
|---|---|---|---|---|
| `do not create a branch` | 80 of 80 | 0 | +80 | **rejected** — 100% red on day one. It discriminates nothing: every file is a finding, so the clause carries no information and the 80 new lines are noise that buries the existing findings. Ruled out by R4. |
| `designated purpose` | 79 of 80 | 1 | +79 | **rejected** — it detects only the exception *clause*, and only in the exact phrase the template happens to use. A repository that states the rule without the "designated purpose" wording is a false positive; a repository that copies the phrase without the rule is a false negative. It measures wording, not the rule. |
| `work directly on` | 78 of 80 | 2 | +78 | **rejected** — an adverb phrase is the part of the sentence a re-writer is most likely to change, and it misses `therapist-commons`, which says "Work is directly on `main` by default". |
| `` on `main` `` | 76 of 80 | 4 | +76 | **chosen** |

**Chosen needle: `` on `main` ``.** It is keyed to the *subject* of the rule —
the default branch, in the code form every one of the four matching files uses
— rather than to an adverb or an exception phrase. It is the shortest span that
identifies "work here by default", it is the highest-carry of the stable
candidates (4 files, double the next), and it is a substring of the canonical
sentence in §4.1, so the template and the report cannot disagree.

**Known misses, recorded rather than hidden.** The needle matches
`careBridge`, `openTeacherKit`, `therapist-commons` and `trailCrew`. It does
**not** match the other repositories named in the 9-file list, and the reasons
differ:

- `chronicleaf` — "On main, run name preflight then openspec list." (no code
  form, and capitalised sentence-initial).
- `kairovia` — "Use main directly."
- `retakeKit` — "Use `main` directly by default." (code form, but no `on`).
- `argoscope` and `kairion` — their only match is
  `git ls-remote --heads origin main` inside an initialisation sentence, which
  is a verification command and not a work-on-`main` rule. These are correct
  misses: the needle is right to be silent about them.

So the clause over-reports on a repository that already carries the rule in
another form. That is the **designed** case: `rules-baseline` already provides
the per-file escape, and the honest use of it is one deliberate marker line in
the file the finding names, recording that the repository documents the rules
in its own words. It is not a new switch, it is not global, and it cannot hide
a file the owner did not open. The per-file opt-out is what makes a
phrase-sensitive needle acceptable at all; without it, a stability-oriented
needle would have to be so generic that it also matches `origin main`.

## 6. Blast radius of the finding count

`python3 scripts/rules_drift.py --root .. --check` reports
`Summary: drift_findings=80` today (47 `RULES_DRIFT` on
`.ai-rules/completion.md`, 25 on `.ai-rules/workflow.md`, 8
`HANDOFF_SCAFFOLD`). The clause adds at most one finding per scanned
`workflow.md` that lacks it, so the briefing's ceiling is +80.

The measured rise is smaller, and the exact figure matters because it is the
number a reviewer will read. The report scans **registry-adopted** projects
only, not every directory with a rule file:

| Quantity | Value | How it was measured |
|---|---|---|
| Workspace repositories with a `.ai-rules/workflow.md` | 80 | file census |
| Adopted projects in `projects.json` | 75 | `rules_drift.adopted_projects` |
| Adopted projects whose `workflow.md` the report reads | 73 | `check()` with the clause in memory |
| Of those, already carrying `` on `main` `` | 4 | `careBridge`, `openTeacherKit`, `therapist-commons`, `trailCrew` |
| Of those, `workspace-governance`, which §4.6 brings into conformance | 1 | its own clause tuple |
| New findings added by the clause | 68 | 73 − 4 − 1 |
| Post-change total | **148** | 80 + 68, from `drift_findings=80` |

**148 is a report total, not a gate outcome.** No gate runs the report (§7), no
consumer parses the total, and `auto_gate.py` computes its status from
`workspace_check.run` plus the Jenkins manifest. The consequence to state
plainly: this change makes 68 more repositories *visible* on a question they
were previously invisible on, and it makes none of them fail.

## 7. Gate-wiring analysis (R6)

**The fact.** `rules_drift.py` is wired into no gate.
`scripts/workspace_check.py:2133` imports `rules_drift` for exactly one
function, `scaffold_handoff`; neither `drift()` nor `check()` is called by
`workspace_check.py`, `scripts/auto_gate.py`, or
`.github/workflows/governance.yml` (which runs `unittest`, `auto_gate.py
--self-check`, `git diff --check`). The shipped `rules-baseline` spec
designates the tool a **triage report** in its own requirement title, "Drift
is reported, never rewritten".

**The ruling that applies.** D18 in
`dotnet-platform-libs/openspec/changes/ui-shared-layer-reachability/design.md`
— "A verifier that no gate runs does not gate; wire every check into the gate
that blocks, or it is documentation" — was decided in favour of wiring, and
this repository's own `AGENTS.md` carries the same sentence as a standing
rule. Under D18, **a new `CLAUSES` entry in an unwired report is
documentation.**

**What this change therefore does not claim.** It does not claim the clause
blocks anything, prevents a branch, or fails a commit. It adds one tuple to an
existing, already-designated triage report. The spec delta says "reported" in
every requirement it adds, and never "enforced", "gated", or "blocks".

**What this change does deliver, stated exactly:**

1. The rule becomes **binding by instruction** on the two surfaces an agent
   reads first — `AGENTS.md` and `.ai-rules/workflow.md` — in this repository
   and in every project `init_project.py` onboards from now on. That is
   enforcement by instruction, not by gate, and it is labelled as such.
2. The clause becomes **machine-reported and reviewable**: 68 repositories
   that were invisible on this question become named, with a per-file
   remediation path that already exists.
3. The gap is **named and tracked in this repository**, not only in the change:
   the implementation records the follow-up in `ROADMAP.md` and
   `decisions.json`.

**The follow-up, named.** `rules-drift-gate-wiring`: call `rules_drift.check()`
from `workspace_check.run` so `RULES_DRIFT` appears in the audit, then decide
the blocking level there — `auto_gate.py` already exposes
`--fail-on-warnings`, and CI already runs `auto_gate.py --self-check`, so the
wire and the block are two edits in the same place. It is deliberately **not**
bundled here: wiring first would convert `drift_findings=80` into a red gate,
and this change widens the report to 148. The clause set stabilises first, the
fleet is worked down, the wiring lands against findings that are actionable.

## 8. Contract and compatibility

- `CLAUSES` gains one tuple. The report's finding shape
  (`level`/`code`/`project`/`message`), its exit code, its `--json` document
  and its stdout format are unchanged.
- `RULE_FILES` is unchanged, so `AGENTS.md` is still not scanned (R5).
- `OPT_OUT` is unchanged and still short-circuits the whole file before any
  clause is evaluated — the new clause is suppressed by the same marker as the
  existing three, and the regression proves it.
- `MUST NOT modify any project file` is unchanged. The report opens rule files
  for reading only; the new clause adds no write path.
- `init_project.py` is unchanged; onboarding a project that already has an
  `AGENTS.md` keeps its local metadata, so the rule reaches new projects
  without touching an existing one.
- No change to `profile_requirements`, the registry, any `.project.json`, any
  evidence state, or any sibling repository.
- `check_skill_refs.py` continues to pass: the paths added to `SKILL.md` and
  `references/rules-baseline.md` are `templates/project/AGENTS.md`,
  `skills/workspace-governance/references/rules-baseline.md`,
  `scripts/rules_drift.py` and `openspec/specs/rules-baseline/spec.md`, all
  of which exist.

## 9. Failure and boundary policy

| Case | Result |
|---|---|
| `workflow.md` omits the branch rule | WARNING `RULES_DRIFT`, label `work-on-main rule`, naming the project and the file; the file is byte-identical afterwards |
| `workflow.md` carries the rule in other words | the same finding; the per-file opt-out marker is the designed, deliberate resolution, recorded in that file |
| `workflow.md` carries the opt-out marker | no finding for any clause, including the new one |
| `workflow.md` unreadable | unchanged `RULES_UNREADABLE` |
| `workflow.md` absent | unchanged: the file is skipped, not reported |
| the rule removed from a canonical template | `test_templates_carry_the_work_on_main_rule` fails, and the report then reports this repository for its own clause |
| the needle later matches nothing in the fleet | 100% red, exactly the `do not create a branch` failure mode in §5; the fix is the needle, recorded as a decision, never a suppressed finding |
| a repository is onboarded with its own `AGENTS.md` | unchanged; the initializer adds missing metadata and does not replace local text |
| a repository really uses a branch, deliberately, under user instruction | correct outcome. The clause is a text check, and the honest record is the rule text or the marker — not a git-history inference |

## 10. Verification oracle

- `python3 -m unittest discover -s tests -v` — all pass, including the new
  work-on-`main` cases in `tests/test_rules_drift.py`, with no existing test
  changing expectation.
- `python3 scripts/rules_drift.py --root .. --check` — record the before
  (`drift_findings=80`) and the after (`drift_findings=148`) totals, and
  confirm the delta is exactly 68 `work-on-main rule` findings, none of them
  for `workspace-governance`.
- Opt-out proof: a fixture `workflow.md` carrying the marker and none of the
  four clauses reports `[]`, and its bytes are unchanged after the run.
- Template proof: `init_project.load_templates()` returns text containing
  `` on `main` `` **and** the exception phrase in both
  `.ai-rules/workflow.md` and `AGENTS.md`.
- `python3 scripts/check_skill_refs.py` — `skill refs ok`.
- `python3 scripts/workspace_check.py --self-check` and
  `python3 scripts/auto_gate.py --self-check` — unchanged from before this
  change; the report is not in either path, so **no gate result may move**.
- `openspec validate work-on-main-baseline --strict --no-interactive`,
  `openspec validate --all --strict --no-interactive`,
  `node scripts/check-openspec-change-names.mjs`, `git diff --check` — pass.
- Record actual outputs. Any environment-blocked check is reported with its
  exact next action, never as passing.

## 11. Decision ledger

**Assumptions:** a repository that documents the rules in its own words is
conformant even when a needle misses it; the report is a triage instrument and
its count is not a gate outcome; a text check can prove a rule is *stated* and
can never prove a branch was *not* created.

**Resolved alternatives:**

- (a) Rewrite the template's branch sentence vs add a second one → chose
  **replace**. Two sentences about git state in one file is two authorities,
  and the one being replaced conditions on an undesignated "purpose".
- (b) `do not create a branch` as the needle → **rejected**; 0 of 80 carry it
  (§5). Ruled out by R4 before it was measured, and the measurement confirms it.
- (c) A looser needle such as `` `main` `` → **rejected**: `argoscope` and
  `kairion` contain `origin main` in a verification command and would pass
  without a rule.
- (d) Add `AGENTS.md` to `RULE_FILES` too → **deferred**; R5, with a named
  precondition.
- (e) Wire `rules_drift.check()` into `workspace_check.run` in this change →
  **rejected**; §7. It would turn 148 report findings into a gate result in
  the same commit that creates them.
- (f) Bulk-edit the 65 repositories whose `AGENTS.md` lacks the rule →
  **rejected**; R3, and `AGENTS.md` forbids editing a sibling without scope
  authorization.
- (g) Put the rule only in the skill → **rejected**. The skill is the widest
  carrier but the narrowest reach: a project onboarded today, or an agent
  working with no skill installed, never sees it.
- (h) Let this repository fail its own new clause → **rejected**; §4.6.

**Deferred:** `rules-drift-gate-wiring` (§7); `AGENTS.md` in `RULE_FILES`
(R5); reconciling the 65 sibling `AGENTS.md` files and the 71 sibling
`workflow.md` files that carry no branch language, as per-repository
product-owned work.

**Blockers:** none. R1–R7 are decided, so the wording, the needle, the file set
and the follow-up boundary are all closed. The one open item — whether
`RULES_DRIFT` blocks — is deliberately the follow-up's question and does not
block this change.

## 12. Owner rulings — decided

The owner has ruled on all seven questions. They are binding on the wording, the
needle, the file set and the enforcement claims. The rationale is recorded in
`proposal.md` "Owner rulings — decided"; this section records the same rulings
in design terms.

| Ruling | Decision | Effect on this design |
|---|---|---|
| R1 | Canonical baseline text in `templates/project/**`, never in a script | §1 file set; §4.1 the single sentence; §4.2 no initializer change; the existing `Single canonical source` requirement is extended, not relaxed |
| R2 | The rule names the user-asks exception | §4.1 wording; the template's "designated purpose" sentence is replaced |
| R3 | The rule also lands in `AGENTS.md` | §4 chain branch 2; two `AGENTS.md` files, no sibling edit (§4.5) |
| R4 | A `CLAUSES` entry with a stable needle, not `do not create a branch` | §4.3 the tuple; §5 the sensitivity table and the choice of `` on `main` ``, with the known misses recorded |
| R5 | `AGENTS.md` is not added to `RULE_FILES` | §1 "must not change `RULE_FILES`"; §8; the precondition is the follow-up's first task |
| R6 | The gate tension is stated, not resolved | §7 in full: the fact, D18, what is not claimed, what is delivered, the named follow-up |
| R7 | The skill is the global carrier | §4.4 routing row, boundaries bullet, and the reference update, with `check_skill_refs.py` as the existence proof |

## Traceability

| Requirement | Proposal outcome | Design decision | Implementation boundary | Scenarios | Tasks | Oracle |
|---|---|---|---|---|---|---|
| The rule is canonical baseline text and names the exception | What Changes §1–2 | §4.1, §4.2 | `templates/project/.ai-rules/workflow.md`, `templates/project/AGENTS.md` | rule present with exception / template carries it | 2.1, 2.2, 2.3 | `load_templates()` assertion |
| New projects receive the rule | What Changes §1 | §4.2 | `init_project.py` (unchanged) | initializer copies the sentence | 2.2 | template load + existing "no second copy" test |
| The rule reaches this repository and the skill | What Changes §3–4 | §4.4, §4.6 | two `AGENTS.md` files, this repo's `workflow.md`, `SKILL.md`, `references/rules-baseline.md` | self-conformance / front-door routing | 2.4, 2.5, 2.6 | `check_skill_refs.py`, no self finding in the report |
| The clause is machine-reported | What Changes §5 | §4.3, §5 | `rules_drift.CLAUSES` | fires when absent, silent when present | 2.7, 3.1 | `drift_findings` 80 → 148 |
| The report stays non-destructive and the opt-out still applies | Non-goals, R5 | §8, §9 | `rules_drift.py` (no write path) | opt-out suppresses the new clause; no file rewritten | 2.7, 3.2, 3.3 | byte-equality assertion, existing test extended |
| The report is triage, and the gate gap is named | Package Boundary, R6 | §7 | `ROADMAP.md`, `decisions.json` | — (not a behaviour) | 2.8 | follow-up named in both records |
