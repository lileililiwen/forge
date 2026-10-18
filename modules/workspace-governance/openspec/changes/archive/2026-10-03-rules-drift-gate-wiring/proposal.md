# Proposal: Wire clause-level rules drift into the blocking gate as a ratchet

## Why

`scripts/rules_drift.py` reports **148** findings across the fleet and is wired
into **no gate**. `scripts/workspace_check.py` imports only
`rules_drift.scaffold_handoff`; the clause report is invoked by neither
`workspace_check.py`, `scripts/auto_gate.py`, nor
`.github/workflows/governance.yml`. Under ruling **D18** — recorded in
`openspec/changes/ui-shared-layer-reachability/design.md` — "a verifier that no
gate runs does not gate; wire every check into the gate that blocks, or it is
documentation." Decision **D5** in `decisions.json` and the
`rules-drift-gate-wiring` follow-up in `ROADMAP.md` (lines 76–86) already name
this work. Today it is named and unstarted.

The measured state, 2026-10-03:

- `python3 scripts/rules_drift.py --root .. --check` → `Summary:
  drift_findings=148`.
- That total is **140 clause-level `RULES_DRIFT` findings plus 8
  `HANDOFF_SCAFFOLD` findings**. The 140 break down by clause as
  `work-on-main rule` **68**, `two-commit rule` **37**, `evidence boundary`
  **28**, `OpenSpec name preflight` **6**, `BFS workflow model` **1**, spread
  over **72** projects. The ratchet keys on the 140, because
  `HANDOFF_SCAFFOLD` is a separate check with no `file`/`label` pair to key on.
- The fleet audit reports `errors=197 warnings=24`;
  `workspace_check.py --self-check` reports `errors=0 warnings=0`;
  `auto_gate.py --self-check --no-deployment` reports `PASS`.
- **380** unit tests pass, including 14 cases in `tests/test_rules_drift.py`.

**The second defect is a lie in a docstring.** `scripts/workspace_check.py:2129`
states that clause-level drift "is reported by `scripts/rules_sync.py`". **No
such file exists** — the module is `scripts/rules_drift.py`. The same docstring
is now wrong in substance as well: it says the report is "a triage report rather
than here", which is exactly what this change reverses. A docstring that names a
nonexistent file and describes the wrong architecture is how a gap stays
unnoticed, so the correction is part of this change rather than a follow-up.

## What Changes

- **One committed data file**, `rules-drift-baseline.json` at the governance
  root, holding a flat set of `{"project", "file", "label"}` keys — the 140
  clause-level findings that exist today. It is a **data file, not a script
  constant** (R1, ruling D16), so it is reviewable as a diff and no code path can
  extend it in a review-blind edit.
- **`workspace_check.py` emits ERROR `RULES_DRIFT_NEW`** for any finding whose
  `(project, file, label)` key is absent from the baseline (R2). The measured
  new-finding count is **zero**, so the fleet gate does not move; new drift
  blocks. No change to `auto_gate.py` is needed: its `status` is
  `"PASS" if summary["errors"] == 0`, so an ERROR already blocks by default.
- **`workspace_check.py` emits WARNING `RULES_DRIFT_BASELINE_STALE`** naming how
  many baseline entries are now redundant, whenever the current finding set is a
  strict subset of the baseline (R3). The warning is the decay mechanism and it
  never fails. Repositories that fix their drift shrink the baseline.
- **The detailed 148-finding report stays where it is** (R4).
  `rules_drift.py --check` remains the triage surface, and `workspace_check.py`
  emits only the delta — re-emitting all 140 would bury the 197 real errors and
  24 warnings in a quarter of a thousand lines of known noise.
- **Fail closed on the baseline** (R5). A missing, unparseable, non-object, or
  empty `rules-drift-baseline.json` is a hard non-zero ERROR naming the file —
  the same rule the vocabulary loader established, because an empty baseline
  would make all 140 findings "new" and red the fleet for the wrong reason.
- **No escape hatches** (R6). No environment variable, no
  `--skip-rules-drift` flag, no per-project exemption registry, no category
  escape. The legitimate per-repository escape already exists and is repo-owned:
  the `<!-- workspace-governance:rules-customized -->` marker honoured by
  `rules_drift.OPT_OUT`.
- **Baseline growth is a governance action, tightening is the default** (R7).
  Growth is permitted and is a review-visible diff that is expected to be
  argued; the baseline is a ratchet snapshot, not a permanent allowance, and it
  is never the way to silence a legitimate repository.
- **The stale `rules_sync.py` docstring is corrected**, and its substantive
  claim is rewritten to state the new split: the detailed report lives in
  `rules_drift.py`, the blocking delta lives in `workspace_check.py`.

## Package Boundary

One package, because the baseline file, the loader, the two finding codes and
the docstring are one contract: the file is meaningless without the ratchet that
reads it, and the ratchet is unsound without the file.

| Package | Single outcome | Owner / language | Boundary and contract | Depends on | Independent oracle |
|---|---|---|---|---|---|
| `rules-drift-gate-wiring` (this) | New clause-level rules drift blocks the fleet gate; drift that exists today does not | `workspace-governance`, Python 3.11 stdlib | `rules-drift-baseline.json`; `workspace_check.validate_handoff_quality` / `run`; the `validate_handoff_quality` docstring | shipped `rules-baseline` (`Drift is reported, never rewritten`); the `CLAUSES` clause set | a fixture with a key absent from the baseline yields ERROR `RULES_DRIFT_NEW`; the 140 shipped keys yield nothing; a deleted baseline yields a non-zero exit |

`scripts/rules_drift.py` keeps its clause set, its finding shape, its exit code
and its read-only guarantee. `auto_gate.py` needs no edit. No sibling
repository, no `.project.json`, and no `projects.json` entry is touched.

## Non-goals

- **No fixing the 140 findings.** No sibling repository's `.ai-rules` file is
  edited in this change. The ratchet is measured against the fleet as it stands;
  remediation is product-owned follow-up.
- **No fix to this repository's own single finding**
  (`.ai-rules/completion.md` / `evidence boundary`). It becomes a baseline entry
  like any other, so `--self-check` stays at `errors=0`. Fixing it is the first
  natural demonstration of R3 and belongs to its own change.
- **No new clause, needle, `RULE_FILES` entry or `OPT_OUT` form.** The clause
  set is not widened by this change; `AGENTS.md` remains un-scanned, for the
  named reason in the shipped spec.
- **No `--fail-on-warnings` policy change, no CI workflow edit.** The blocking
  comes from the ERROR level, which `auto_gate.py` already honours by default.
- **No `HANDOFF_SCAFFOLD` ratchet.** Those 8 findings stay reported only; they
  are not clause-level and have no `(file, label)` key.
- **No automatic baseline regeneration in any code path.** A tool that rewrites
  the baseline is a code path that edits it, which is the thing D16 forbids.

## Owner rulings — decided

These are **decided**, not proposed. They are recorded here so the change carries
no open question, and the implementation does not re-derive them.

- **R1 — a committed baseline data file.** `rules-drift-baseline.json` at the
  governance root, a flat set of `{"project", "file", "label"}` keys. Decided on
  ruling **D16** from the UI change: "A data file is reviewable as a diff; a
  script constant invites in-code edits."
- **R2 — the gate blocks only on NEW drift.** ERROR `RULES_DRIFT_NEW` for any
  key not in the baseline. Decided: a blanket failure would red the fleet
  overnight with 140 known findings and prove nothing about the future.
- **R3 — the gate nags when the baseline can shrink.** WARNING
  `RULES_DRIFT_BASELINE_STALE`, naming the redundant-entry count, on a strict
  subset. It never fails. Decided: the ratchet decays through attention, not
  through a deadline.
- **R4 — the detailed report stays where it is.** `rules_drift.py --check`
  remains the report; `workspace_check.py` emits only the delta. Decided: 140
  extra lines would bury the 197 real errors and 24 warnings.
- **R5 — fail closed on the baseline.** Missing, unparseable, non-object or
  empty is a hard non-zero ERROR naming the file; never an empty-set fallback.
  Decided on the vocabulary loader's precedent: a silently empty baseline turns
  140 known findings into 140 "new" ones and reds the fleet for the wrong
  reason.
- **R6 — no escape hatches.** No environment variable, no skip flag, no
  exemption registry, no category escape. Decided: the repo-owned
  `<!-- workspace-governance:rules-customized -->` marker already is the
  legitimate per-repository escape, and a second escape owned by governance
  would displace it.
- **R7 — the baseline tightens by diff, and growth is argued.** Adding a key to
  silence new drift is visible in review and is expected to be argued, exactly
  as the legacy-divergent list is argued in the UI change. Decided, and stated
  explicitly because it is the load-bearing review rule: baseline **growth** is
  permitted but is a review-visible governance action, whereas silencing a
  legitimate repository is done with the marker **in that repository**, never by
  growing the baseline.
