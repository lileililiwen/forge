# Tasks: work-on-main-baseline

## 0. Caveats and confirmations

- [x] **The wording is ruled, not open.** One sentence, byte-identical in all
  four text carriers (`design.md` §4.1):

  ```
  Work directly on `main` by default; do not create a branch or worktree unless the user asks for one or designates one.
  ```

  Do not paraphrase it per file. It must contain the clause needle
  `` on `main` `` **and** the exception phrase "unless the user asks for one or
  designates one"; both are asserted by 2.9.

- [x] **The needle is ruled: `` on `main` ``.** Not
  `do not create a branch` (0 of 80 files carry it — 100% red on day one), not
  `designated purpose`, not `work directly on`. See `design.md` §5 for the
  sensitivity table. If the needle is ever changed, the change is recorded as a
  decision; findings are never suppressed to make a needle look better.

- [x] **This change adds no enforcement, and no task may imply it.** A clause in
  `CLAUSES` is *reported*; `rules_drift.py` is wired into no gate
  (`design.md` §7). Every task below that says "the rule is stated" means
  stated in text. If a task cannot be phrased that way, it belongs to
  `rules-drift-gate-wiring`.

- [x] **This repository must satisfy its own clause.** `workspace-governance`
  is `adoption: adopted`, so 2.4 is not optional. Do **not** use the opt-out
  marker on this repository's `.ai-rules/workflow.md` to achieve it — the
  marker would also suppress the three clauses the file already satisfies.

- [x] **`MUST NOT modify any project file` is inviolable.** The clause is a
  tuple in an existing tuple. No write path is added anywhere in
  `rules_drift.py`, and 3.2 asserts it.

- [x] **No sibling repository is edited in this change.** Nine of 80
  `workflow.md` files already carry branch language and 15 of 80 `AGENTS.md`
  files already state the rule; leave all of them byte-identical, including the
  four the needle matches and the ones it misses for good reason
  (`design.md` §5).

- [x] **Stage only this change's files.** Do not sweep unrelated working-tree
  edits into the change's commits.

## 1. BFS — Baseline and impact coverage

- [x] 1.1 Record the baseline before any edit:
  `python3 scripts/rules_drift.py --root .. --check` reports
  `Summary: drift_findings=80`; the 9 repositories carrying branch language in
  `.ai-rules/workflow.md`; the 15 carrying a work-on-`main` rule in
  `AGENTS.md`; and the needle-sensitivity table in `design.md` §5.
- [x] 1.2 Snapshot the report output and the byte content of this
  repository's `.ai-rules/workflow.md` and `AGENTS.md`, so "no project file
  changed" and "the clause count moved by exactly 68" are both checkable after
  the edit.
- [x] 1.3 Map the propagation chain onto existing code before touching it:
  `TEXT_TEMPLATES` and `load_templates()` (`scripts/init_project.py:20`,
  `:97`), `CLAUSES` (`scripts/rules_drift.py:33`), `drift()` (`:58`),
  `check()` (`:86`), `OPT_OUT` (`:25`), `RULE_FILES` (`:23`), the skill's
  routing table and boundaries
  (`skills/workspace-governance/SKILL.md:16`, `:29`), and the reference's
  "Read first" list (`skills/workspace-governance/references/rules-baseline.md:3`).
- [x] 1.4 Confirm the additive shape: the clause is appended to
  `CLAUSES[".ai-rules/workflow.md"]` only. `RULE_FILES`, `drift()`,
  `check()`, `OPT_OUT`, the finding shape, the exit code and the `--json`
  document are untouched, so no existing scenario changes.
- [x] 1.5 Add the failing tests first (2.9), and confirm they fail against the
  unedited tree for the right reason: a missing clause entry, not an import
  error.
- [x] 1.6 Verify the implementation-handoff gate: Python 3.11,
  `workspace-governance`, no shared-library boundary crossed, no sibling
  repository edited, no dependency added.

## 2. DFS — Requirement-by-requirement implementation

- [x] 2.1 R1: add the canonical sentence to
  `templates/project/.ai-rules/workflow.md` as its own line, before the
  two-commit sentence so the file reads as: workflow model, git state,
  verification, commits.
- [x] 2.2 R1 + R2: in `templates/project/AGENTS.md`, replace the first
  sentence of the branch bullet with the canonical sentence and keep
  "Preserve unrelated uncommitted work and never sweep it into a commit."
  unchanged. The old "without a designated purpose" wording is removed, not
  kept alongside the new sentence.
- [x] 2.3 Confirm no initializer change is needed: `TEXT_TEMPLATES` already
  contains both files and `load_templates()` reads them from disk. If any
  initializer edit appears necessary, that is evidence the rule is being copied
  into a script — stop and re-read R1.
- [x] 2.4 R3 + `design.md` §4.6: add the canonical sentence to this
  repository's own `.ai-rules/workflow.md` as its own line, and to this
  repository's `AGENTS.md` as one bullet in the top-level instruction list.
  Byte-identical sentence in both.
- [x] 2.5 R7: add the routing row and the boundaries bullet to
  `skills/workspace-governance/SKILL.md`, and state the rule, the two canonical
  carriers and the per-file opt-out in
  `skills/workspace-governance/references/rules-baseline.md`. Every path cited
  must exist, because `scripts/check_skill_refs.py` proves it. Keep the
  reference's "never copy a second set of the rules into a script" instruction
  and keep it a link, not a second source.
- [x] 2.6 R7: run `python3 scripts/check_skill_refs.py` and record
  `skill refs ok`. A missing cited path is a failure of this task, not a
  follow-up.
- [x] 2.7 R4: append `("work-on-main rule", "on `main`")` to
  `CLAUSES[".ai-rules/workflow.md"]` in `scripts/rules_drift.py`. No other
  edit to that file.
- [x] 2.8 R6: record the `rules-drift-gate-wiring` follow-up in
  `ROADMAP.md` and in `decisions.json`, naming `workspace_check.run` and
  `auto_gate.py --fail-on-warnings` as the wire and the blocking decision, and
  stating that this clause is reported only. The gap must be tracked in the
  repository, not only in this change directory.
- [x] 2.9 Tests in `tests/test_rules_drift.py`:
  - [ ] 2.9.1 extend the `WORKFLOW` fixture with the canonical sentence, so
    `test_complete_rules_have_no_drift` keeps passing for the right reason;
  - [ ] 2.9.2 `test_work_on_main_clause_reported_when_absent` — a fixture
    carrying only `BFS` reports the label `work-on-main rule`;
  - [ ] 2.9.3 `test_work_on_main_clause_present_is_not_reported` — the
    canonical sentence produces no `work-on-main` finding;
  - [ ] 2.9.4 `test_opt_out_marker_suppresses_the_work_on_main_clause` — the
    marker suppresses the new clause as it does the existing three, and the
    file's bytes are unchanged;
  - [ ] 2.9.5 `test_templates_carry_the_work_on_main_rule` — both
    `load_templates()` outputs contain `` on `main` `` **and** the exception
    phrase, so a future template edit that drops either fails a test.

## 3. BFS — Cross-surface regression and completeness

- [x] 3.1 Re-run `python3 scripts/rules_drift.py --root .. --check` and prove
  the arithmetic in `design.md` §6: `drift_findings=80` before, `148` after,
  a delta of exactly 68 `work-on-main rule` findings, and **no** such finding
  for `workspace-governance`. Record both totals verbatim.
- [x] 3.2 Prove the report is still non-destructive: the `before`/`after` byte
  snapshots from 1.2 of every scanned rule file are identical, and the existing
  `test_report_is_non_destructive` still passes unchanged.
- [x] 3.3 Prove the opt-out still short-circuits the whole file before any
  clause is evaluated, and that no new switch was added anywhere: no
  environment variable, no exemption registry, no category escape.
- [x] 3.4 Prove no gate result moved: `workspace_check.py --self-check` and
  `auto_gate.py --self-check` return the same `errors=0 warnings=0` as the
  baseline, and `.github/workflows/governance.yml` needs no edit — the report
  is not in any gate path.
- [x] 3.5 Confirm the fleet is untouched: no sibling `AGENTS.md`,
  `.ai-rules/workflow.md`, `.project.json` or README differs from its
  pre-change bytes; the four needle-matching repositories are still silent; and
  `argoscope` / `kairion` are still silent for the right reason.
- [x] 3.6 Confirm `init_project.py` still refuses a conflicting `--id` and
  still derives the same ids, that an existing project's local `AGENTS.md` is
  not replaced, and that `test_no_embedded_template_copy` still passes — the
  rule is text in a template, never a second copy in a script.
- [x] 3.7 Confirm no requirement in
  `openspec/specs/rules-baseline/spec.md` was weakened: every pre-existing
  scenario in `Single canonical source` and `Drift is reported, never
  rewritten` appears in this change's `## MODIFIED Requirements` section,
  including the `MUST NOT modify any project file` guarantee.

## 4. Verification

- [x] 4.1 `python3 -m unittest discover -s tests -v` — all pass, including the
  new `tests/test_rules_drift.py` cases, with no existing test changing
  expectation.
- [x] 4.2 `python3 scripts/rules_drift.py --root .. --check` — record the
  before and after totals and the 68-finding delta.
- [x] 4.3 `python3 scripts/check_skill_refs.py`,
  `python3 scripts/workspace_check.py --self-check`,
  `python3 scripts/auto_gate.py --self-check` — record exact output.
- [x] 4.4 `openspec validate work-on-main-baseline --strict --no-interactive`
  and `openspec validate --all --strict --no-interactive`,
  `node scripts/check-openspec-change-names.mjs`, `git diff --check`.
- [x] 4.5 Record the two commits per `.ai-rules/completion.md`: the
  implementation/tests/archive commit, then the `HANDOFF.md`-only
  pointer/evidence commit. Stop after commit 2.
- [x] 4.6 Record actual outputs. Report any environment-blocked check with
  its exact next action rather than as passing, and report the gate-wiring
  follow-up as **not delivered here** in the handoff text.
