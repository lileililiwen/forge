# Design: spec-purpose-backfill-gate

## 1. Implementation boundary

This change ships two things inside this repository and edits nothing else:

- `scripts/spec_purpose_backfill.py` — the shared backfill tool, promoted from
  `openlearning/scripts/fill_spec_purposes.py` and reshaped to the
  `scripts/axis_migration.py` contract.
- `SPEC_PURPOSE_PLACEHOLDER` plus the `SPEC_PURPOSE_AUDIT_ENABLED` fence in
  `scripts/workspace_check.py`.

The 1,234-file fleet write is **not** applied here. It is a cross-repo source
edit and is a separately authorized step, exactly as the axis migration was.

The prior art stays where it is. `openlearning/scripts/fill_spec_purposes.py` is
a record of the first application of this derivation in one repository; deleting
or editing it would rewrite that repository's history for no gain, and this
change has no authority over that repository's source.

## 2. What the marker is, exactly

Two generations exist in the fleet. Both are one line, immediately under
`## Purpose`, and both are matched by one pattern:

```
TBD - created by archiving change <name>. Update Purpose after archive.   # 1,229 files
TBD - created by archiving change <name>.                                 #     5 files
```

| Generation | Files | Matched by |
|---|---|---|
| current | 1,229 | the tail sentence is one alternative of the marker grammar |
| legacy | 5 | the same pattern — the tail is optional |

Total 1,234 of 1,958 spec files in the 79 repositories that carry an
`openspec/specs` tree, across 71 of them. 7 of the 1,234 carry a blank line
between the heading and the marker.

The `## Purpose` body is matched as a **section**, not as a line: the marker must
be the whole body, with only blank lines around it. That is what makes a
`## Purpose` holding real prose never a finding whatever its wording, and it is
why the change name is bound to `[\w.-]+` rather than `[^\n]*` — a change name
has no spaces, so a real sentence appended after the marker can never be
absorbed into the name. Measured: the section-strict pattern matches exactly the
same 1,234 files fleet-wide that a looser line pattern does, and no others, so
the strictness costs nothing on the real corpus.

The change name in the placeholder is the **bare** change name. The archive
directory is **date-prefixed**:
`openspec/changes/archive/2026-10-03-work-on-main-baseline-rule/`. Joining them
directly — the obvious implementation — resolves nothing. The resolution strips
`^\d{4}-\d{2}-\d{2}-` from each archive directory name and indexes the remainder.

## 3. The two sources, in order

**Source 1 — the archived change's own `## Why`.** Read from
`openspec/changes/archive/<date>-<name>/proposal.md`, the `## Why` section up to
the next `##` heading, whitespace-collapsed to one paragraph. This is real
prose written by the author at the time, so it is the honest Purpose. It
applies to 1,159 of the 1,234 markers, and 1,089 of those clear the 120-character floor below.

**Source 2 — the requirement-derived sentence.** Preserved byte-for-byte from
the prior art so the fleet write is reproducible against the result that script
already produced in `openlearning`:

- first `### Requirement: <topic>` heading in the file, trailing `.` stripped,
  first character lowercased;
- with a heading:
  `The {capability} specification covers {topic} and the related scenarios that govern this capability across the platform.`
- without a heading:
  `The {capability} specification defines the behaviors, contracts, and scenarios that govern this capability across the platform.`
- `capability` is the `# {capability} Specification` title when present, else the
  spec's own directory name.

The capability name is a `kebab-case` directory name on the fleet, and the prior
art interpolated it verbatim. That is preserved rather than prettified: changing
it now would make the shared tool's output differ from the string already
committed in `openlearning`, which is the one thing the reproducibility
requirement is for.

A `## Why` shorter than `MIN_WHY_CHARS` (120, whitespace-collapsed) is treated
as **absent** and the row falls back to source 2. The threshold is recorded on
the plan and asserted in a test, because the decision boundary is a judgment
call and must be visible rather than buried in a constant. At 120 the count is
1,091 usable; at 40 it would be 1,159, but the 12 shortest sections in the fleet
are fragments like a bare sentence fragment rather than a purpose. A reviewer
who disagrees changes one number in one place.

## 4. What changed when promoting `fill_spec_purposes.py`

Judged, not copied. Three changes, and an explicit statement of what was kept:

**Kept unchanged, deliberately.** The `### Requirement:` extraction; the capability-name resolution; both
derived sentences verbatim; the single-substitution rewrite; the
stdlib-only, no-third-party-import constraint. For any spec that has a
`### Requirement:` heading, the shared tool produces the same string the
`openlearning` script produced. That is the reproducibility property the fleet
write depends on.

**Changed — the tool's reach.** The prior art derives `ROOT` from its own
location and walks exactly one `openspec/specs` tree. The shared tool takes
`--root`, enumerates every registered sibling repository, and resolves each
spec's archive directory. It also stops globbing `*/spec.md` and reads the
registry, so a repository with a different capability-directory depth is still
counted rather than silently skipped.

**Changed — the safety contract.** The prior art writes unless `--dry-run` is
passed, which makes the dangerous mode the default. The shared tool follows
`axis_migration.py` instead: dry run is the **only** default, `--apply` is the
sole write path, a dry run writes nothing at all, and `--check` reports whether
the backfill has been applied. Every plan row records the spec, the change, the
source used, the resolved sentence, and the evidence path.

**Changed — refusal instead of a guess.** The prior art has no notion of the
archive and therefore cannot be ambiguous. The shared tool refuses to plan when a
change name resolves to more than one archive directory, and names both. Two
specs on the fleet are in this state: `dotnet-platform-libs`
`platform-testing-toolkit` (archived 2026-09-08 and 2026-09-10) and
`rust-ecommerce` `audit-log-immutability` (2026-08-02 and 2026-08-26). Choosing
between them is a guess about authorship, and the output is a published
sentence.

**Changed — idempotence is explicit.** A spec already resolved reports no row, so
re-running after `--apply` yields an empty plan rather than re-deriving over a
sentence a human has since edited.

## 5. The plan, the drift refusal, and why the plan is not committed

`--json` and the printed plan are one row per spec needing resolution, with
`project`, `spec`, `change`, `source`, `purpose`, `marker`, and the `evidence`
path. `--check` compares the plan against the live fleet and exits 3 while any
row is outstanding, so "is it applied?" is answerable without re-deriving and
without trusting a stale record.

**The generated plan is not committed**, which is a deliberate difference from
`axis_migration.py`'s `migration/identity-and-taxonomy-axis-split.json`. Measured:
the fleet plan is **954 KB across 1,234 rows**, because 1,089 rows carry the
archived `## Why` prose verbatim and the longest is 3,398 characters. That is 16×
the 58 KB axis plan and it is fully regenerable in about a second, so committing
it would add a megabyte of generated content to the repository and require
re-committing it on every unrelated archive in the fleet, without recording a
single decision. The tool is the artifact; the plan is its output. The reviewer
reads it from a dry run, and the operator can still commit one with
`--write-plan` at apply time. This is a re-affirmation of the decision reached
when the tool was written, on a fresh measurement of the current fleet.

### 5.1 `--check` is exempt from the drift refusal, and must be

The drift refusal exists so a *write* never lands on a fleet nobody inspected.
`--check` is not a write path, and applying the refusal there made the tool
unable to answer the one question it exists to answer: on a fully applied fleet
every planned marker is legitimately absent, which the refusal reports as 1,234
rows of drift, so `--check` exited 1 in all three states and never once reported
"applied".

`build_plan` therefore takes `verify_committed`, and `--check` passes `False`.
The one combination that still wants the refusal is `--check --write-plan`,
because that refreshes the record the refusal guards, and it keeps it. All three
states are asserted: not applied exits 3 naming the outstanding rows, applied
exits 0, and re-applied is idempotent at 0.

The drift refusal is unaffected: `_assert_matches_committed_plan` compares
against `migration/spec-purpose-backfill-gate.json` **whenever that file exists**.
With no plan committed it is inert, which is why the refusal is covered by a
fixture test rather than by fleet data — an inert mechanism tested only against
the fleet would be an untested mechanism.

## 6. D18 verification — the gate does **not** yet carry this code

Owner ruling D18: *a verifier that no gate runs does not gate*. This change was
required to verify, not assert, whether adding the rule to `workspace_check.py`
is sufficient. It is not. Read-only; no file in `driftwatchdog` was modified.

**The concern exists and is bound.** `governance-quality` runs a
project-bound command, parses the JSON envelope this change writes, and is
listed in `NON_EXEMPTABLE_CONCERNS` (`manifest.rs:281-287`), so no project may
exempt it. That part of the premise holds.

**The concern's scope is a closed allow-list and does not contain the new code.**

- `GOVERNANCE_QUALITY_DECLARED_CODES` (`src/gate/governance/mod.rs:73-78`) holds
  exactly four codes: `QUALITY_README_CLAIM_CONTRADICTED`,
  `QUALITY_README_CLAIM_UNSCOPED`, `RULES_DRIFT_BASELINE_STALE`,
  `RULES_DRIFT_NEW`. `SPEC_PURPOSE_PLACEHOLDER` is not among them.
- The effective scope defaults to that declared set
  (`default_governance_codes`, `manifest.rs:599-604`) and may only shrink: a
  manifest naming an undeclared code is refused as `REVIEW_REQUIRED` **before the
  command runs** (`mod.rs:292-313`). So a project cannot opt in from its own
  `gate.yaml` either.
- The adapter filters before it classifies: `if !scope.contains(finding.code)`
  `continue` (`mod.rs:453`). A `SPEC_PURPOSE_PLACEHOLDER` finding is discarded.

**The failure mode is worse than absence.** `EXPECTED_EXITS` is `[0, 1]`
(`mod.rs:84`), so the checker's exit 1 is an accepted outcome and is not read as
a verdict. With every new-code finding filtered out, `matched` is empty, and the
envelope mapping yields `PASS` with exit 0 (`mod.rs:273-278`). Enabling this rule
behind that concern alone would give a check that runs, fails, and is reported
green — the precise condition D18 exists to prevent.

**What actually blocks on it today.** `scripts/auto_gate.py` imports
`workspace_check.run` and counts findings directly (`auto_gate.py:16,28,36`); it
has no code scope, so any `ERROR` makes it `FAIL` with exit 1. It is scheduled
by `.github/workflows/governance.yml:22`. That is a real gate that reaches the
new rule at no wiring cost — but it runs `--self-check`, which scans this
repository only, so it cannot see the other 77.

**The minimum change that satisfies D18**, in `driftwatchdog` and not in this
change's authority: add `"SPEC_PURPOSE_PLACEHOLDER"` to
`GOVERNANCE_QUALITY_DECLARED_CODES`, and widen the `governance-quality` code
scope in each bound repository's manifest if that manifest names codes
explicitly. Verified: no `gate.yaml` outside `driftwatchdog`'s own declares
`governance_quality`, so the declared-set change is the whole of it today. Until
that lands, `SPEC_PURPOSE_AUDIT_ENABLED` must stay `False` — flipping it would
make this repository's own CI red while the fleet gate reports green.

### 6.1 The declared-code allow-list is a manual maintenance point

`GOVERNANCE_QUALITY_DECLARED_CODES` is a hand-maintained allow-list, and this
change is the second code to be added to it by hand. Nothing fails when
`workspace_check.py` declares a code the gate cannot see: the code is emitted,
the adapter filters it, and the concern reports `PASS`. The failure is silent,
which is the worst property a check can have, and it will recur on the next new
finding code.

**A guard cannot live in this repository.** Any check here runs inside
`workspace_check.py` and can only observe findings, so it cannot tell the
difference between "a code this concern does not declare" and "a code belonging
to a different concern". A fleet-wide run emits every code in the workspace —
`CAPABILITY_ADOPTION_*`, `STANDALONE_*`, `TAXONOMY_*`, `QUALITY_*` and the rest —
so "saw an undeclared code → `REVIEW_REQUIRED`" inside the concern would fire on
a perfectly healthy fleet. That rule is wrong, not merely blunt, and is recorded
here so the next person does not reach for it.

**The durable guard is a committed inventory, compared in driftwatchdog's own
CI.** Two lists to keep in step is still a manual step, but a step whose
omission fails a build instead of passing quietly:

1. `workspace_check.py` commits its own inventory of every code it can emit —
   a single `FINDING_CODES` tuple that the `finding()` helper validates each
   emission against. A code emitted but not in the inventory is a fail-closed
   `ERROR` in the checker's own run, which is cheap and belongs here.
2. `driftwatchdog`'s governance module gains a test asserting
   `GOVERNANCE_QUALITY_DECLARED_CODES` covers every code in that inventory which
   is a governance-quality code. The comparison is against the **inventory**,
   never against a run's findings, which is exactly what the constraint in the
   previous paragraph permits.

The residual weakness is stated rather than solved: an inventory is a second
hand-maintained list, so this converts a silent gap into a red build but does
not remove the manual step. A single shared source of truth — the checker
emitting its codes in a machine-readable endpoint the gate reads at resolution
time — would remove it, and is a larger cross-repo contract than this change's
authority. Recorded as a follow-up, not proposed as done.

**Neither §6 nor §6.1 is implemented by this change.** Both live in
`driftwatchdog`, and `design.md` records them so the separately authorized
cross-repo step has a specification rather than a request. Nothing in this
change's own test suite asserts the wiring works, because this repository cannot
run the Rust gate and asserting it from here would be exactly the unverifiable
claim `AGENTS.md` rejects.

## 7. Contract and compatibility

- `SPEC_PURPOSE_PLACEHOLDER` is a new code in its own family. No existing code is
  renamed, narrowed, or reused, and the severity is not negotiable while the
  placeholders stand.
- The rule is **not** coupled to `profile_requirements` and is not gated on
  `requires_openspec`. A repository with an `openspec/specs` tree is checked
  whatever its profile, because a descriptive rule must not become a
  requirements driver.
- The check is skipped entirely for a repository with no `openspec/specs`
  directory, so the fleet's non-OpenSpec repositories are untouched.
- `MIN_WHY_CHARS`, the plan file, and the tool's flags are this change's only new
  surface. `workspace_check.py` gains one function, two constants, and one
  keyword argument on `validate_project`, defaulted to `None` so every existing
  caller is unaffected.
- Python 3.11 standard library only.

## 8. Verification oracle

- `python3 -m pytest tests` — 501 before, **553 after** (52 new tests), zero
  failures, no existing test weakened.
- `python3 scripts/workspace_check.py --self-check` — 0 errors, 0 warnings.
- `python3 scripts/workspace_check.py --root .. --audit` — unchanged
  `errors=205` / `warnings=18` with the fence closed, because the rule emits
  nothing.
- The same fleet audit with `SPEC_PURPOSE_AUDIT_ENABLED` flipped to `True`:
  **1,439 errors**, i.e. 205 + exactly the 1,234 markers and nothing else. This
  is the fence-open proof, measured on the real corpus rather than only on a
  fixture, and the constant is committed back at `False`.
- A dry run of `scripts/spec_purpose_backfill.py` over the real fleet, proving
  the fleet is byte-identical afterwards.
- A fixture-based test proving the same unmigrated corpus reports ERROR and turns
  the gate red with `SPEC_PURPOSE_AUDIT_ENABLED = True`, and reports nothing with
  it `False`.
- A test proving the environment variable cannot open the fence.
- A test proving the checker's pattern and the tool's pattern are the same
  pattern, and a test proving the section-strict form still matches exactly the
  1,234 fleet files.
- A test pinning all three `--check` states, including the applied state with a
  committed plan present.
- A fixture test for the drift refusal, which is inert against the live fleet
  because no plan is committed.
- `openspec validate spec-purpose-backfill-gate --strict`.

## 9. Decision ledger

| # | Decision | Why |
|---|---|---|
| 1 | The rule is ERROR and fenced by a committed constant | Ruling 1; the taxonomy rule's exact shape. Softening to WARNING would make a permanent warning nobody can clear. |
| 2 | The requirement-derived fallback is preserved verbatim | It is the only property that makes a 1,234-file write reviewable, and it keeps the shared tool's output identical to what `openlearning` already committed. |
| 3 | `## Why` below 120 characters is treated as absent | A recorded threshold beats an unrecorded judgment; 120 keeps fragments out and admits 1,089 real sections. |
| 4 | An ambiguous archive name refuses the plan row-level, while `apply` is all-or-nothing | The output is a published sentence, so a guess about which change authored it is not reviewable — but refusing the whole plan would make the tool unable to produce a reviewable plan at all. All-or-nothing at write time keeps a half-migrated fleet from being invisible to a closed fence. |
| 5 | D18 is reported unmet, with the file and line that must change | A verifier that no gate runs does not gate, and the check as it stands would be reported green while failing. Naming the fix is more useful than asserting success. |
| 6 | No sibling repository and no archiver is touched | `AGENTS.md` cross-repo boundary; the archiver is third-party and the durable fix is owned here. |
| 7 | The marker must be the whole `## Purpose` section, and the change name is bound to `[\w.-]+` | A `## Purpose` holding real prose is not a defect whatever its wording, and a change name has no spaces, so this is the only way trailing prose cannot be mistaken for a long name. Costs nothing on the real corpus: the same 1,234 files match either way. |
| 8 | `--check` is exempt from the drift refusal | The refusal guards writes. A backfilled fleet is exactly the state in which every planned marker is legitimately gone, so applying the refusal to `--check` made "is it applied?" unanswerable. |
| 9 | The generated plan is still not committed | Re-measured at 954 KB and regenerable in a second; it records no decision, and the drift refusal it would activate is covered by a fixture test instead. |
