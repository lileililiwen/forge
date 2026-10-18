# Design: rules-drift-gate-wiring

## 1. Implementation boundary

**Repository:** `/home/paul/code/workspace-governance` — Python 3.11, standard
library only. No sibling repository is edited.

**Files to change:**

- `rules-drift-baseline.json` (**new**, governance root) — the committed
  baseline data file (R1).
- `scripts/workspace_check.py` — the baseline loader, the delta computation, the
  two new finding codes, and the corrected `validate_handoff_quality` docstring.
- `tests/test_rules_drift.py` or a new `tests/test_rules_drift_gate.py` — the
  ratchet regression cases.

**Must not change:** `scripts/rules_drift.py` (its `CLAUSES`, `RULE_FILES`,
`OPT_OUT`, `drift()`, `scaffold_handoff()`, `check()`, `main()`, its finding
shape and its exit code are all untouched), `scripts/auto_gate.py`,
`.github/workflows/governance.yml`, `scripts/vocabulary.py`, `projects.json`,
`profile_requirements`, every `.project.json`, and every sibling repository.

The load is local and follows the existing pattern: `validate_handoff_quality`
already does `import rules_drift  # local import to keep this module standalone`.

## 2. The baseline file's exact shape

`rules-drift-baseline.json` sits at the governance root, beside
`capabilities.json`, `decisions.json` and `vocabulary.json` — the repository's
other committed data files.

```json
{
  "schema_version": 1,
  "recorded_at": "2026-10-03",
  "source": "python3 scripts/rules_drift.py --root .. --check",
  "findings": [
    { "project": "actoria", "file": ".ai-rules/workflow.md", "label": "work-on-main rule" },
    { "project": "actoria", "file": ".ai-rules/workflow.md", "label": "two-commit rule" },
    { "project": "actoria", "file": ".ai-rules/completion.md", "label": "two-commit rule" },
    { "project": "alethefy", "file": ".ai-rules/workflow.md", "label": "work-on-main rule" }
  ]
}
```

The three keys shown are **real measured keys** from the current report. The
shipped file carries all **140** clause-level keys, sorted by `(project, file,
label)` so a diff is stable and a rename reads as one changed line.

- The top level is an **object** with `schema_version` and a `findings` **array**
  of flat key objects. No nesting by project, no per-clause grouping, no counts.
- `recorded_at` and `source` are provenance only; the loader **MUST NOT** read
  them for behaviour, so a stale `recorded_at` can never change a gate result.
- `HANDOFF_SCAFFOLD` findings are **not** in the file. The 8 of them are a
  separate check with no `(file, label)` pair; R4 keeps them in the detailed
  report only. This is why the file holds **140** keys while the report total is
  **148**, and it is stated here so a reviewer diffing the two numbers does not
  read it as a defect.

### 2.1 Why a flat set, and not per-clause counts

A count baseline would store `{"work-on-main rule": 68, "two-commit rule": 37,
…}` and compare totals. **Rejected**, because a count cannot say *which* finding
disappeared and *which* appeared. Concretely: `careBridge` fixes its
`work-on-main` drift while `vowra` loses it. The count is 68 before and 68
after, the ratchet reports nothing, and a genuinely new finding in a fourth
repository is invisible for exactly as long as somebody else is fixing theirs.
That is a hole of precisely the kind this workspace keeps finding — a check that
compares an aggregate and therefore cannot detect the thing it exists to detect —
and the flat set closes it: `current - baseline` names the new key and only the
new key, regardless of what else moved in the same run.

A flat set is also the shape a reviewer needs. Adding a key to silence one
finding is a visible one-line addition with a project and a clause in it (R7);
adding `1` to a number is not.

## 3. The ratchet algorithm

Two lines of intent, then the pseudocode.

```
current   = clause-level RULES_DRIFT keys across adopted projects   (the report)
new       = current - baseline   -> ERROR   RULES_DRIFT_NEW, one per key
redundant = baseline - current   -> WARNING RULES_DRIFT_BASELINE_STALE, if any
```

```
load_baseline(governance_root):
    # R5, fail closed on every path. No empty-set fallback, ever.
    if not path.is_file():                       -> error RULES_DRIFT_BASELINE "missing"
    if json.loads(text) raises:                  -> error RULES_DRIFT_BASELINE "unparseable"
    if not isinstance(doc, dict):                -> error RULES_DRIFT_BASELINE "not an object"
    entries = doc.get("findings")
    if not isinstance(entries, list) or not entries: -> error RULES_DRIFT_BASELINE "empty"
    return {(e["project"], e["file"], e["label"]) for e in entries}

ratchet(project_path, project_id, findings):
    baseline = load_baseline(governance_root)   # once per run, memoized
    for f in rules_drift.drift(project_path):
        if f["code"] != "RULES_DRIFT":
            continue                            # RULES_UNREADABLE stays as it is
        key = (project_id, f["file"], f["label"])
        if key not in baseline:
            finding(..., "ERROR", "RULES_DRIFT_NEW", f"{key} is not in the baseline", project_id)

# once, after the per-project loop:
redundant = baseline - current_keys
if redundant:
    finding(..., "WARNING", "RULES_DRIFT_BASELINE_STALE",
            f"{len(redundant)} baseline entries are redundant; shrink rules-drift-baseline.json", "-")
```

The warning is emitted **once per run**, not per project, because "the baseline
can shrink" is one fact about one file.

**A malformed entry is a hard error, not a skipped row.** An entry missing
`project`, `file` or `label`, or carrying an unknown key, is a data defect in a
file the gate trusts; a partially-parsed baseline would silently stop protecting
the rows it failed to read, which is the exact empty-set failure R5 exists to
prevent.

## 4. Blast radius

Every number below is measured on 2026-10-03 from
`/home/paul/code/workspace-governance` and `/home/paul/code`.

| Consumer | Before | After | Why it does not move |
|---|---|---|---|
| Fleet audit errors | **197** | **197** | `current - baseline` is empty: all 140 clause-level findings are baseline keys, so 0 new |
| Fleet audit warnings | **24** | **24** | `current == baseline` exactly, which is not a *strict* subset, so R3 does not fire on day one |
| Fleet audit lines | 221 | 221 | R4: the 140 are not re-emitted; only the delta is |
| `workspace_check.py --self-check` | `errors=0 warnings=0` | `errors=0 warnings=0` | this repository's own single finding (`.ai-rules/completion.md` / `evidence boundary`) is a baseline key, so it is not new |
| `auto_gate.py --self-check --no-deployment` | `PASS` | `PASS` | follows from the row above; `auto_gate.py` is not edited |
| `rules_drift.py --check` | `drift_findings=148` | `drift_findings=148` | the report is unchanged; R4 keeps it the detailed surface |
| Unit tests | 380 pass | 380 + new cases | no existing expectation changes |

**Zero movement is the acceptance criterion, not a side effect.** The ratchet
must ship with the fleet gate bit-for-bit identical, and any other outcome means
the baseline or the key derivation is wrong.

**Why an ERROR is enough.** `auto_gate.py:38` computes
`status = "PASS" if summary["errors"] == 0 and (not fail_on_warnings or
summary["warnings"] == 0) else "FAIL"`. An ERROR therefore fails the gate by
default, with no `auto_gate.py` edit and no new flag. `RULES_DRIFT_BASELINE_STALE`
is a WARNING, so it stays out of the default status computation entirely — the
decay mechanism cannot redden a gate even if a repository's warnings are later
made fatal.

**What does move:** the day a repository introduces drift that is not in the
baseline, `auto_gate.py` returns `FAIL` for the first time. That is the change's
entire point, and it is the first time the fleet has ever been able to say that.

## 5. The R5 failure path

R5 is decided on the vocabulary loader's precedent: `workspace_check.run` calls
`_vocabulary_for(governance_root)` and, on any exception, emits ERROR
`VOCABULARY` and returns non-zero — it never continues with an empty vocabulary.

| Condition | Detection | Result |
|---|---|---|
| File absent | `not path.is_file()` | ERROR `RULES_DRIFT_BASELINE`, message names `rules-drift-baseline.json` as missing |
| Unparseable | `json.JSONDecodeError` | ERROR `RULES_DRIFT_BASELINE`, message names the file and the parse failure |
| Top level not an object | `not isinstance(doc, dict)` — a bare array, string or number | ERROR `RULES_DRIFT_BASELINE`, message names the file |
| `findings` absent, not a list, or empty | `not isinstance(list)` / `len == 0` | ERROR `RULES_DRIFT_BASELINE`, message names the file as empty |
| Entry missing a required key, or carrying an unknown one | key check per entry | ERROR `RULES_DRIFT_BASELINE`, message names the offending entry |
| Any of the above | — | the run returns **non-zero**; no `RULES_DRIFT_NEW` and no `RULES_DRIFT_BASELINE_STALE` is computed from an untrusted baseline |

The reason this is an ERROR and not a warning is arithmetic. An empty baseline
means `current - baseline` is the whole current set, so all 140 findings become
`RULES_DRIFT_NEW` and the fleet turns red **for the wrong reason** — a missing
data file would present as 140 governance failures. Failing closed converts that
into one finding that names its own cause and its own fix.

## 6. No escape hatches (R6), and what the real one is

Nothing is added that can turn the ratchet off. No environment variable, no
`--skip-rules-drift` flag, no per-project exemption registry, no category-wide
escape — the same three things the shipped spec already forbids for
`OPT_OUT` ("no environment variable, no exemption registry, and no
category-wide escape").

The legitimate per-repository escape **already exists and is repo-owned**: a
`<!-- workspace-governance:rules-customized -->` marker line in the finding's own
file, honoured by `rules_drift.OPT_OUT`. A repository that documents the shared
rules in its own words suppresses the clause before it ever becomes a key, and
the suppression lands in that repository, where its owner can see it in review.

The two routes are not interchangeable, and the difference is the whole design:

- **Marker** — repo-owned, per-file, visible to the repository's owner, the
  designed resolution for a deliberate re-wording.
- **Baseline growth** — governance-owned, global, invisible to the repository
  whose finding was silenced, and available to anyone with a diff.

R7 states the rule that follows: growth is permitted, is a review-visible diff,
and is expected to be argued exactly as the legacy-divergent list is argued in
the UI change. But silencing a repository that is legitimately conformant is
done with the marker in that repository, **never** by growing the baseline. The
baseline is a ratchet snapshot of *known* drift; it is not a permanent
allowance, and it can only be tightened by removing keys that no longer match.

## 7. Contract and compatibility

- `rules_drift.py` is **not modified**. Its `MUST NOT modify any project file`
  guarantee is inviolable and unchanged: the ratchet reads project files through
  the same `read_text()` path, adds no write path, and the report still opens
  rule files for reading only.
- `OPT_OUT` still short-circuits the whole file before any clause is evaluated,
  so a marker-bearing file never produces a key and never needs a baseline entry.
- `RULES_UNREADABLE` is passed through untouched; an unreadable rule file is
  still a report finding and is **not** turned into a ratchet key, because an
  unreadable file is an environment fault rather than a drift finding.
- The `HANDOFF_SCAFFOLD` warning in `validate_handoff_quality` is unchanged.
  The new code is additive within that function and its neighbourhood.
- `Finding(level, code, message, project)` is unchanged, so `auto_gate.py`'s
  `asdict(item)` and the `--json` document are unchanged.
- No new dependency: `json` and `pathlib` are already imported by
  `workspace_check.py`.

## 8. The corrected docstring

`scripts/workspace_check.py:2129` currently reads:

> Clause-level `.ai-rules` drift is reported by `scripts/rules_sync.py` as a
> triage report rather than here, because repositories legitimately document the
> shared rules in their own words and language.

Both halves are wrong. `scripts/rules_sync.py` **does not exist** — the module is
`scripts/rules_drift.py`. And "rather than here" is exactly what this change
reverses. The replacement names both surfaces and the split between them, so the
docstring describes the architecture that ships: the detailed report in
`rules_drift.py`, the blocking delta in `workspace_check.py`, and the baseline
that separates them.

## 9. Verification oracle

- `python3 scripts/rules_drift.py --root .. --check` — still
  `drift_findings=148`, byte-identical to the recorded baseline.
- New drift blocks: a fixture whose `(project, file, label)` is absent from the
  baseline yields ERROR `RULES_DRIFT_NEW`, and `auto_gate.py` over that fixture
  returns `FAIL`.
- No movement: fleet audit stays `errors=197 warnings=24`;
  `workspace_check.py --self-check` stays `errors=0 warnings=0`;
  `auto_gate.py --self-check --no-deployment` stays `PASS`.
- Shrinkage nags: with one key removed from the baseline fixture, the run emits
  `RULES_DRIFT_BASELINE_STALE` naming the redundant count, and the exit code is
  unchanged.
- Fail closed: each of the five §5 conditions yields a non-zero exit and an
  ERROR naming the file; none yields a `RULES_DRIFT_NEW`.
- Non-destructive: every scanned project rule file is byte-identical across a
  `workspace_check` run.
- `python3 -m unittest discover -s tests` — 380 existing tests still pass, no
  existing expectation changed.
- `python3 scripts/check_skill_refs.py`, `openspec validate
  rules-drift-gate-wiring --strict --no-interactive`, `openspec validate --all
  --strict --no-interactive`, `node scripts/check-openspec-change-names.mjs` —
  pass. Record actual outputs; report any environment-blocked check with its
  exact next action rather than as passing.

## 10. Decision ledger

**Assumptions:** the 140 clause-level findings are *known* debt, not 140 defects
to be rediscovered every run; a repository that adopts the opt-out marker
suppresses the finding at source and needs no baseline entry; the fleet audit's
197 errors and 24 warnings are pre-existing debt unrelated to this change.

**Resolved alternatives:**

- (a) Wire all 140 findings as blocking → **rejected**. It reds the fleet
  overnight with findings nobody has triaged, and the resulting red is
  indistinguishable from a regression. A gate that is always failing trains
  everyone to ignore it.
- (b) Per-clause counts instead of a flat set → **rejected**; §2.1.
- (c) A `--fail-on-warnings` policy change so `RULES_DRIFT_BASELINE_STALE` can
  fail → **rejected**. The decay signal is advice; making it fatal would punish a
  repository for fixing its drift.
- (d) An empty-set fallback when the baseline is unreadable → **rejected**; §5.
- (e) A tool that regenerates the baseline from the current report → **rejected**.
  That is a code path that edits the baseline, which is the review-blind edit
  D16 rules out. The baseline is regenerated by a human running one command and
  committing the diff.
- (f) Keying on the report's 148 total, including `HANDOFF_SCAFFOLD` → **rejected**;
  those 8 findings have no `(file, label)` pair, so they would need a second,
  inconsistent key shape in the same file.

**Open questions:** none. R1–R7 are decided; see `proposal.md`.
