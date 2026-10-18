# Tasks: rules-drift-gate-wiring

## 0. Caveats and confirmations

- [x] **R1–R7 are decided, not open.** R1 baseline data file; R2 block on new
  drift only; R3 nag when the baseline can shrink; R4 the detailed report stays
      put; R5 fail closed; R6 no escape hatches; R7 growth is argued, tightening
      is the default. `proposal.md` records them. Do not re-derive one, and do
      not quietly implement a variant.
- [x] **The baseline is a data file, never a script constant** (D16). No
      constant in `workspace_check.py` may hold, default, or regenerate a
      finding key. If a code path can extend the baseline, R1 is violated.
- [x] **No regeneration command ships.** The baseline is refreshed by a human
      running `rules_drift.py --check` and committing the diff (design.md §10e).
- [x] **`MUST NOT modify any project file` is inviolable.** The ratchet reads
      project files only. No write path is added anywhere, and 3.4 asserts it.
- [x] **Zero gate movement on day one is the acceptance criterion.** Fleet audit
      `errors=197 warnings=24`, `--self-check` `errors=0 warnings=0`,
      `auto_gate.py --self-check --no-deployment` `PASS`. Any other outcome means
      the baseline or the key derivation is wrong — do not adjust the code to
      reach a number.
- [x] **The file holds 140 keys, not 148.** 8 `HANDOFF_SCAFFOLD` findings have
      no `(file, label)` pair and stay in the report only (design.md §2).
- [x] **Do not use the baseline to silence a repository.** The legitimate escape
      is the `<!-- workspace-governance:rules-customized -->` marker in that
      repository. No task below may add an exemption registry, an environment
      variable, a skip flag, or a category escape.
- [x] **No sibling repository is edited, and this repository's own single
      finding is not fixed.** `.ai-rules/completion.md` / `evidence boundary`
      becomes a baseline entry like any other.
- [x] **Stage only this change's files.** Do not sweep unrelated working-tree
      edits into the change's commits.

## 1. BFS — Baseline and impact coverage

- [x] 1.1 Record the pre-change state verbatim: `drift_findings=148` = 140
      clause-level + 8 `HANDOFF_SCAFFOLD`; clause breakdown `work-on-main rule`
      68, `two-commit rule` 37, `evidence boundary` 28, `OpenSpec name
      preflight` 6, `BFS workflow model` 1, across 72 projects; fleet
      `errors=197 warnings=24`; `--self-check` `errors=0 warnings=0`;
      `auto_gate.py --self-check --no-deployment` `PASS`; 380 unit tests.
- [x] 1.2 Snapshot the byte content of every scanned rule file before the edit,
      so "no project file changed" is checkable afterwards (3.4).
- [x] 1.3 Map the propagation chain onto existing code before touching it:
      `rules_drift.drift()` / `check()` / `OPT_OUT` (`scripts/rules_drift.py`),
      `validate_handoff_quality` (`scripts/workspace_check.py:2126`),
      `validate_project` (`:2258`), `run` (`:2273`), `_vocabulary_for` (`:57`),
      `run_gate` and the `status` expression (`scripts/auto_gate.py:19`, `:38`).
- [x] 1.4 Confirm the additive shape: `rules_drift.py` and `auto_gate.py` are not
      modified at all, and the change is confined to one new data file plus
      `workspace_check.py` and its tests.
- [x] 1.5 Add the failing tests first (2.6), and confirm they fail against the
      unedited tree for the right reason — a missing `RULES_DRIFT_NEW` code, not
      an import error.

## 2. DFS — Requirement-by-requirement implementation

- [x] 2.1 R1: create `rules-drift-baseline.json` at the governance root with
      `schema_version`, `recorded_at`, `source` and a `findings` array of
      `{"project", "file", "label"}` objects, sorted by `(project, file, label)`
      (design.md §2). Populate it from the current report:
      `python3 scripts/rules_drift.py --root .. --check --json`, taking the
      `RULES_DRIFT` findings only. **Assert the file holds exactly 140 entries**
      and that the first key is
      `{"project": "actoria", "file": ".ai-rules/workflow.md", "label": "work-on-main rule"}`.
- [x] 2.2 R1: assert the 140 keys are exactly the current `RULES_DRIFT` set, so
      the shipped file is a snapshot of reality and not a hand-curated list. A
      mismatch here is a hard stop, not a value to edit.
- [x] 2.3 R5: add the baseline loader in `workspace_check.py`, beside
      `_vocabulary_for` so the fail-closed precedent is visible. All five
      conditions in design.md §5 — missing, unparseable, non-object, empty
      `findings`, malformed entry — yield ERROR `RULES_DRIFT_BASELINE` naming the
      file, and the run returns non-zero. **No empty-set fallback on any path.**
      Reject unknown keys in an entry rather than ignoring them.
- [x] 2.4 R2: in `validate_handoff_quality`, after the existing
      `HANDOFF_SCAFFOLD` check, iterate `rules_drift.drift(project_path)` and emit
      ERROR `RULES_DRIFT_NEW` for every `RULES_DRIFT` finding whose
      `(project, file, label)` is absent from the baseline. Pass `RULES_UNREADABLE`
      through untouched — it is not a drift key. Do not change the existing
      `HANDOFF_SCAFFOLD` finding.
- [x] 2.5 R3: after the per-project loop in `run`, compute
      `baseline - current_keys` and, when non-empty, emit exactly one WARNING
      `RULES_DRIFT_BASELINE_STALE` naming the redundant count. It must never
      affect the exit code.
- [x] 2.6 Tests, new cases in `tests/test_rules_drift.py` (or a new
      `tests/test_rules_drift_gate.py`):
  - [x] 2.6.1 a fixture whose key is absent from the baseline yields exactly one
        ERROR `RULES_DRIFT_NEW` naming the project, file and clause;
  - [x] 2.6.2 the 140 shipped keys yield **no** `RULES_DRIFT_NEW`, so the
        baseline and the report agree on day one;
  - [x] 2.6.3 a key removed from a baseline copy yields one
        `RULES_DRIFT_BASELINE_STALE` naming the redundant count, and the exit
        code is unchanged;
  - [x] 2.6.4 each of the five §5 conditions yields a non-zero exit and an
        ERROR naming `rules-drift-baseline.json`, and **no** `RULES_DRIFT_NEW`;
  - [x] 2.6.5 `RULES_UNREADABLE` is passed through and produces no key;
  - [x] 2.6.6 a marker-bearing fixture still suppresses every clause and needs no
        baseline entry;
  - [x] 2.6.7 the shipped `rules-drift-baseline.json` parses, is a non-empty
        object, has no duplicate or malformed entries, and its key set equals the
        live report's `RULES_DRIFT` set — a guard so a future hand-edit that drifts
        from reality fails a test.
- [x] 2.7 Fix the stale docstring at `scripts/workspace_check.py:2129`: remove
      the reference to the nonexistent `scripts/rules_sync.py`, and rewrite the
      sentence so it names both surfaces — the detailed report in
      `scripts/rules_drift.py`, the blocking delta in `workspace_check.py` — and
      no longer claims the clause report is "rather than here" (design.md §8).
- [x] 2.8 R6: confirm no escape hatch was added — no environment variable, no
      `--skip-rules-drift`, no exemption registry, no category escape — and that
      `OPT_OUT` still short-circuits a file before any clause is evaluated.

## 3. BFS — Cross-surface regression and completeness

- [x] 3.1 Re-run `python3 scripts/rules_drift.py --root .. --check` and confirm
      it is byte-identical: still `drift_findings=148`, still the same 68/37/28/6/1
      breakdown. `rules_drift.py` was not touched, so any movement is a
      measurement error to investigate.
- [x] 3.2 Prove zero gate movement (0.5): fleet audit `errors=197 warnings=24`;
      `workspace_check.py --self-check` `errors=0 warnings=0`;
      `auto_gate.py --self-check --no-deployment` `PASS`; no new
      `RULES_DRIFT_NEW`; no `RULES_DRIFT_BASELINE_STALE` (the current set equals
      the baseline, which is not a *strict* subset). Record all four verbatim.
- [x] 3.3 Prove new drift blocks: construct a fixture with one key absent from
      the baseline and confirm `auto_gate.py` over it returns `FAIL` because
      `summary["errors"] != 0` — with **no** edit to `auto_gate.py` and no new
      flag.
- [x] 3.4 Prove the ratchet is still non-destructive: the 1.2 byte snapshots of
      every scanned rule file are identical after a `workspace_check` run, and no
      sibling repository shows a diff.
- [x] 3.5 Confirm the report remains the detailed surface: `workspace_check.py`
      emits only the delta and never re-emits the 140 baselined findings as
      warnings (R4). The audit line count stays 221.
- [x] 3.6 Confirm no requirement in
      `openspec/specs/rules-baseline/spec.md` was weakened: every pre-existing
      scenario in `Single canonical source` and `Drift is reported, never
      rewritten` appears in this change's `## MODIFIED Requirements` section,
      including the `MUST NOT modify any project file` guarantee, restated as
      still true.
- [x] 3.7 Confirm the one pre-existing scenario this change *does* change is
      restated as **stronger**, not weaker: "The clause does not block anything"
      keeps its title and its baselined-finding guarantee, and the new blocking
      behaviour is added as a separate scenario, "A clause finding outside the
      baseline does block". Record both texts side by side in the change's
      evidence so the direction of the change is reviewable, not asserted.
- [x] 3.8 Confirm the implementation-handoff gate: Python 3.11, standard library
      only, no new dependency, no shared-library boundary crossed, no sibling
      repository edited, no `auto_gate.py` or CI edit.

## 4. Verification

- [x] 4.1 `python3 -m unittest discover -s tests` — all 380 existing tests still
      pass with no existing expectation changed, plus the new ratchet cases.
- [x] 4.2 `python3 scripts/rules_drift.py --root .. --check` — record the exact
      `Summary:` line.
- [x] 4.3 `python3 scripts/check_skill_refs.py`,
      `python3 scripts/workspace_check.py --self-check`,
      `python3 scripts/auto_gate.py --self-check --no-deployment` — record exact
      output.
- [x] 4.4 `openspec validate rules-drift-gate-wiring --strict --no-interactive`,
      `openspec validate --all --strict --no-interactive`,
      `node scripts/check-openspec-change-names.mjs`, `git diff --check`.
- [x] 4.5 Record the two commits per `.ai-rules/completion.md`: the
      implementation/tests/archive commit, then the `HANDOFF.md`-only
      pointer/evidence commit. Stop after commit 2.
- [x] 4.6 Record actual outputs. Report any environment-blocked check with its
      exact next action rather than as passing, and state the measured before and
      after error/warning counts in the handoff text rather than claiming the gate
      tightened.
