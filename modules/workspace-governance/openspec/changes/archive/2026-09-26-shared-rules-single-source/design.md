# Design: Single-source the shared rules baseline

## 1. Implementation boundary

**Repository:** `/home/paul/code/workspace-governance` — Python 3.11, standard
library only.

**Files to change (source):**

- `templates/project/**` — align the on-disk templates with the text that 45
  repositories actually carry (the embedded copy was the applied one), so the
  on-disk set is complete and canonical; add `README.md` and `ROADMAP.md`, which
  existed only in the embedded dict.
- `scripts/init_project.py` — read `templates/project/**` from disk; delete the
  embedded `TEMPLATES` dict.
- `scripts/rules_drift.py` (new) — clause drift report and `scaffold_handoff()`.
- `scripts/workspace_check.py` — surface a scaffold `HANDOFF.md` (`HANDOFF_SCAFFOLD`).
- `tests/test_rules_drift.py` (new).

**Sibling files changed (data only):** create `crossalheart/.ai-rules/*` and
`crossalheart/.ai-gate/gate.yaml` from the canonical template.

**Must not change:** any project's existing rule text, any other project's
`.ai-gate/gate.yaml`, any product source, and the two-commit process.

## 2. Language and runtime

Python **3.11**, standard library only. Commands:

- `python3 -m unittest discover -s tests -v`
- `python3 scripts/rules_drift.py --root .. --check`
- `python3 scripts/workspace_check.py --root .. --audit --json`
- `python3 scripts/workspace_check.py --self-check`
- `openspec validate --all --strict --no-interactive`

## 3. Ownership and shared code

Project-local to `workspace-governance`, the existing owner of templates and
policy. No extraction: no sibling implements a rules baseline.

- **Exported contract:** the canonical templates plus the invariant clause list
  in `rules_drift.CLAUSES`.
- **Consumers:** `init_project.py` (writes), `workspace_check.py` (scaffold
  signal), agents (read the report).
- **Dependency direction:** products → governance. Governance never imports a product.
- **Release boundary:** ships with this repository's templates and scripts.

## 4. Behavioral model

| Input state | Result |
|---|---|
| adopted project, rule file carries every clause | no finding |
| adopted project, rule file omits a clause | report WARNING naming project, file and clause |
| adopted project, rule file carries the opt-out marker | skipped |
| adopted project, rule file unreadable | report `RULES_UNREADABLE` |
| adopted project, `.ai-rules` absent and profile requires it | audit `REQUIRED_FILE` (existing rule), fixed here by creating the files |
| adopted project, `HANDOFF.md` is scaffold | audit WARNING `HANDOFF_SCAFFOLD` |
| non-adopted project | not checked |

**Invariant clauses** (`rules_drift.CLAUSES`), kept short so re-wrapping and
translation do not cause false drift:

- `.ai-rules/workflow.md`: `BFS`, `two commits`, `check-openspec-change-names`
- `.ai-rules/completion.md`: `two commits`, `release evidence`

**The tool never writes.** There is deliberately no `--apply`; §8 records why.

## 5. Contract and compatibility

- No `.project.json` or schema change.
- New audit code: `HANDOFF_SCAFFOLD` (WARNING). New report codes:
  `RULES_DRIFT`, `RULES_UNREADABLE` (report-level WARNING).
- **Compatibility:** the initializer keeps its CLI and file set; the generated
  `.ai-rules/workflow.md` gains the name-preflight sentence, which the
  majority of repositories already carry. Every other project file is untouched.
- `templates/project/.ai-rules/*` becomes the only editable source of the
  baseline text.

## 6. Failure and boundary policy

| Case | Result |
|---|---|
| Rule file unreadable / non-UTF-8 | reported `RULES_UNREADABLE`; never a pass |
| Adopted project missing rule files | audit `REQUIRED_FILE`; fixed by creating from template |
| Template file missing at run time | tool exits non-zero with the reason |
| Project not adopted | skipped |
| Project documents rules differently | opt-out marker suppresses the report for that file |
| Report run | exits non-zero when drift is found, but the workspace audit is unaffected |

## 7. Verification oracle

- `python3 -m unittest discover -s tests -v` — all pass, including: clause
  detection, opt-out suppression, scaffold detection, and a byte-identity check
  that running the report modifies nothing.
- `python3 scripts/rules_drift.py --root .. --check` — runs and reports; the
  remaining drift is triage, and every project file is unchanged afterwards.
- `python3 scripts/workspace_check.py --root .. --audit --json` — **0 errors**;
  the 2 `crossalheart` `REQUIRED_FILE` errors are cleared and its
  `HANDOFF_SCAFFOLD`/`DEPLOYMENT_PROMOTION_STALE` warnings are the only ones.
- `python3 scripts/workspace_check.py --self-check` — 0 errors, 0 warnings.
- `python3 scripts/maturity_audit.py --root .. --json` — counts match the audit.
- `openspec validate --all --strict --no-interactive` — pass.

## 8. Decision ledger

**Reversal recorded.** The first implementation made each canonical *paragraph*
mandatory and shipped `--apply`, which appends missing paragraphs. Running it
appended to **60 project files**, including well-maintained ones such as `mewo`
whose rule text already expressed the same rules in its own words. That was
measured, judged a regression, and reverted byte-for-byte. The evidence:

- 45 repositories match the embedded template exactly; 20+ document the rules
  with custom or non-English wording;
- paragraph-level containment flagged 143 issues where clause-level checks flag
  only genuinely absent rules;
- rewriting 60 files to satisfy a checker damages documentation quality.

**Resolved alternatives:**

- (a) Byte-identical files vs containment vs clause presence → chose
  **clause presence, reported as triage**; only objective gaps warn, nothing is
  rewritten.
- (b) Enforce drift in the workspace audit vs separate report → chose
  **separate report**, so legitimate per-project wording does not fail the build.
- (c) `--apply` rewriting vs report-only → chose **report-only**; the initializer
  remains the only writer.
- (d) Fail on scaffold `HANDOFF.md` vs warn → chose **warn**; real handoff bodies
  are product-owned.

**Assumptions:** the on-disk template is the source; the embedded copy was the
applied text and therefore the correct baseline text; project-specific rule
content is legitimate.

**Deferred:** authoring real `HANDOFF.md` bodies for the 11 scaffolded adopted
projects is a product-side task; the WARNING names them.

**Blockers:** none.

## Traceability

| Requirement | Proposal outcome | Design decision | Implementation boundary | Scenarios | Tasks | Oracle |
|---|---|---|---|---|---|---|
| R1 Single canonical source | one source for the baseline | §1, §3, §8 | `templates/project/**`, `init_project.py` | on-disk / no second copy | 1.2, 2.1 | generated file contains the preflight sentence |
| R2 Drift reported not rewritten | visible drift, intact files | §4, §6, §8(a),(c) | `rules_drift.py` | drift / opt-out / non-destructive | 1.3, 2.2 | report runs; files byte-identical |
| R3 Scaffold handoffs surfaced | boilerplate cannot pass as a handoff | §4, §6 | `rules_drift.py`, `workspace_check.py` | detected / not flagged | 2.3 | `HANDOFF_SCAFFOLD` fires only on boilerplate |
| R4 Baseline clears the errors | the last 2 errors clear | §1, §7 | `crossalheart/.ai-rules/*` | audit 0 errors | 2.4, 3.1 | `--audit` errors = 0 |
