# Tasks: Single-source the shared rules baseline

## 1. BFS — Baseline and impact coverage

- [x] Measure drift across the 75 projects with `.ai-rules/workflow.md`:
  `workflow.md` carries BFS 74/75, the OpenSpec name preflight 70/75 and the
  two-commit rule 57/75; `completion.md` carries the two-commit rule 56/75 and
  the evidence boundary 46/75.
- [x] Establish which copy was actually applied: 45 repositories are
  byte-identical to the **embedded** `init_project.py` template, so the on-disk
  `templates/project/*` was the stale one.
- [x] Add tests for clause detection, opt-out suppression, unreadable files,
  scaffold detection and non-destructiveness under `tests/test_rules_drift.py`.
- [x] Verify the implementation-handoff gate: Python 3.11, project
  `workspace-governance`, no shared-library boundary.

## 2. DFS — Requirement-by-requirement implementation

- [x] R1: align `templates/project/**` with the applied text (add the
  name-preflight sentence; add `README.md` and `ROADMAP.md`, which existed only
  in the embedded dict); make `init_project.py` read the templates from disk and
  delete the embedded `TEMPLATES` dict.
- [x] R2: implement `scripts/rules_drift.py --check` — clause report, opt-out
  marker, no writes.
- [x] R3: surface a scaffold `HANDOFF.md` from
  `workspace_check.validate_handoff_quality` as WARNING `HANDOFF_SCAFFOLD`.
- [x] R4: create `crossalheart/.ai-rules/workflow.md`,
  `crossalheart/.ai-rules/completion.md` and `crossalheart/.ai-gate/gate.yaml`
  from the canonical template.
- [x] Record the reversal: a first implementation made whole paragraphs
  mandatory with an `--apply` that appended them; it appended to **60 files**
  (including `mewo`, whose text already expressed the same rules) and was
  reverted byte-for-byte. The design keeps the report-only clause check instead.

## 3. BFS — Cross-surface regression and completeness

- [x] The audit drops from 2 errors / 7 warnings to **0 errors / 18 warnings**;
  no other project changed level.
- [x] No project's existing rule content changed — reverted to a clean
  portfolio `git diff` before the baseline change was re-scoped.
- [x] `init_project.py` still creates the same eight files for a fresh project
  and now writes the name-preflight sentence into `.ai-rules/workflow.md`.
- [x] The scaffold warning fires for the 11 known stub handoffs and not for a
  handoff with real content.

## 4. Verification

- [x] `python3 -m unittest discover -s tests -v` — 122 tests, OK (10 new).
- [x] `python3 scripts/rules_drift.py --root .. --check` — runs; 81 triage
  findings, and every project file is byte-identical afterwards.
- [x] `python3 scripts/workspace_check.py --root .. --audit --json` — 0 errors,
  18 warnings (11 `HANDOFF_SCAFFOLD`, 7 `DEPLOYMENT_PROMOTION_STALE`).
- [x] `python3 scripts/workspace_check.py --self-check` — 0 errors, 0 warnings.
- [x] `python3 scripts/maturity_audit.py --root .. --json` — reports the same
  0 errors / 18 warnings as the audit.
- [x] `node scripts/check-openspec-change-names.mjs` and
  `openspec validate --all --strict --no-interactive` — valid.
- [x] `git diff --check` — clean.
