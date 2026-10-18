# Tasks: derived-kind-canonical-form

## 0. Caveats and constraints

- [x] **This is a defect fix inside a landed change, not a re-litigation.**
  `identity-and-taxonomy-axis-split` committed `bb5ca6f` and `0933194` and its
  migration is applied. The axis split, the derivation, the frozen `kinds` list,
  and `TAXONOMY_LEGACY_VALUE` are all correct as landed. History is not
  rewritten, the change is not archived, and `identity-and-taxonomy-axis-split`
  is not edited.
- [x] **The fix is in the checker, not in the portfolio.** No `.project.json`,
  no `projects.json`, and no sibling repository is written. `errors=205` is
  unchanged because nothing here can move an ERROR.
- [x] **`KIND_UNKNOWN` stays WARNING and is not silenced.** The fix adds a
  question before reporting; it does not remove a report, rename the code, or
  narrow the rule away. A value that is neither legacy nor derived still
  reports, and §3 of `design.md` pins that on both sides.
- [x] **`kinds` stays 12 and no test is weakened.** Extending the vocabulary
  would be the smallest diff and is ruled out in `design.md` §2 for three
  reasons; extending it would also break
  `test_existing_arrays_are_unchanged`, which pins `len(raw["kinds"]) == 12`.
- [x] **No commit may sweep unrelated work.** The working tree carries a
  pre-existing uncommitted `.project.json` edit; it is not staged. `main`
  only, no branch, no `git add -A`, no attribution trailers.

## 1. BFS — Baseline and blast radius

- [x] 1.1 Record the baseline before any edit: suite `Ran 492 tests … OK`;
  fleet audit `errors=205 warnings=35` with 17 `KIND_UNKNOWN`; `--self-check`
  `errors=0 warnings=1`, the single warning being
  `KIND_UNKNOWN: unknown kind: 'python-governance'` on this repository.
- [x] 1.2 Confirm the diagnosis rather than re-derive it: the derivation space
  is 6 roles × 6 stacks = 36 values, `kinds` has 12, and **27 derived values
  are absent from `kinds`**. The 17 findings are the subset the fleet uses, so
  the defect is a vocabulary/rule mismatch and not 17 portfolio defects.
- [x] 1.3 Locate the single reporting site: `validate_kind`
  (`scripts/workspace_check.py`), one WARNING emission, called once per
  declaration from `validate_project`. No other site emits `KIND_UNKNOWN`.
- [x] 1.4 Map every consumer of `kinds` and of `validate_kind` before editing,
  so widening the rule cannot silently change a second caller's contract:
  `validate_project` (the audit path), `tests/test_vocabulary.py` (frozen-set
  calls), `tests/test_identity_and_taxonomy.py` (repo-root calls), and the
  `REMAPPED_KINDS` remap table.
- [x] 1.5 Confirm the axes stay descriptive. `validate_kind` reads two strings
  and writes a finding; it must not grow a read of a required file, a CI
  requirement, a verification command, or any evidence, deployment, or
  promotion state.

## 2. DFS — One coherent contract

- [x] 2.1 Add `_is_self_consistent_derived_kind`, which returns true only when
  both axes are present, both are canonical, and `kind` equals
  `derive_kind(role, stack)`. Each precondition is load-bearing: absent axes
  mean nothing is derived, and a non-canonical axis must not produce a
  derivation (`design.md` §4).
- [x] 2.2 Give `validate_kind` the derived-form escape **after** the legacy
  membership test, so a canonical `kind` reads no axis set and the change
  cannot introduce a vocabulary read on a path that did not make one.
- [x] 2.3 Add `canonical_roles` / `canonical_stacks` keyword arguments
  mirroring the `canonical_kinds` that was already there and mirroring
  `validate_taxonomy`'s existing shape, so the rule is testable against frozen
  axis values rather than the live `vocabulary.json`. Both default to `None` →
  resolve from `governance_root`; every existing call site is unchanged.
- [x] 2.4 Leave `vocabulary.json` alone. `kinds` is 12 before and after.
- [x] 2.5 Leave `validate_taxonomy` alone. It already compared against the
  derivation and already reported `TAXONOMY_LEGACY_VALUE` correctly; the two
  rules are now consistent rather than one of them changed.

## 3. BFS — Re-audit every consumer

- [x] 3.1 Pin both sides of the rule with nine tests in
  `tests/test_vocabulary.py::DerivedKindKnownTests`: a self-consistent derived
  value produces no finding; the full 36-value derivation space produces no
  finding and the unlisted count is asserted non-zero so the sweep cannot pass
  on a vocabulary that already covered it; a legacy value that disagrees with
  its own derivation still reports; a value in neither form still reports;
  absent, half-declared, and non-canonical axes still report; an absent or
  non-string `kind` still reports; a legacy-list value with disagreeing axes
  stays known.
- [x] 3.2 Assert the severity is WARNING wherever a finding is expected, so the
  rule cannot be demoted or silenced by a later edit without a red test.
- [x] 3.3 Prove the new tests bite: reconstruct the pre-fix body behind a
  backup/restore and run only the new class. The two new-behaviour tests fail
  and the seven unchanged-behaviour tests pass — the fix widens knowledge only
  where it should. Restore verified byte-identical.
- [x] 3.4 Suite green: **`Ran 501 tests … OK`** (492 before, +9 new, 0
  weakened, 0 skipped-by-edit). The pre-existing `KIND_UNKNOWN` tests
  (`test_kind_unknown_keeps_its_own_meaning`,
  `test_non_canonical_kind_warns_without_driving_requirements`) still pass
  unmodified, which is the regression guard for the fleet path.
- [x] 3.5 Fleet audit: **`errors=205 warnings=18`** — errors unchanged, warnings
  down exactly 17, `KIND_UNKNOWN` **0**. The 18 remaining warnings are
  `PUBLICATION_UNVERIFIED` 0, `HANDOFF_SCAFFOLD` 8, `DEPLOYMENT_PROMOTION_STALE`
  7, `CAPABILITY_BLOCKED` 2, `DISCOVERABILITY_INVALID` 2, `CI_MISSING` 2,
  `QUALITY_README_CLAIM_UNSCOPED` 1. No `KIND_UNKNOWN` remains and no other
  finding count moved.
- [x] 3.6 `--self-check`: **`errors=0 warnings=0`**, clearing the
  `python-governance` warning the migration left behind.
- [x] 3.7 `docs/vocabulary.md` states the derived-form rule, records that
  `kinds` is a legacy vocabulary rather than the authority on a derived value,
  carries the 36-vs-12 arithmetic, and tabulates the five cases that still
  report. The `KIND_UNKNOWN` line and the `kind` / `profile` term-table row
  were corrected to match.

## 4. BFS — Spec and validation

- [x] 4.1 Spec delta at
  `specs/portfolio-vocabulary/spec.md`, MODIFYING the existing
  `Canonical kinds are enforced` requirement, which is the requirement
  `validate_kind` implements. `identity-and-taxonomy` is not a target: that
  capability spec is still unarchived, and its derived-kind requirement is
  unchanged by this fix.
- [x] 4.2 `openspec validate derived-kind-canonical-form --strict` passes.
- [x] 4.3 `openspec validate --changes --strict` passes for the whole tree.
  (`openspec list --changes --strict` is not valid on the pinned CLI v1.6.0;
  `openspec list --changes` is the available form.)
- [x] 4.4 Two commits on `main`: spec/tests/docs first, then `HANDOFF.md` only.
  No archive commit, no branch.

## 5. Deliberately not done

- [ ] **`kinds` is still 12, by decision, not by oversight.** The 27
  unlisted-but-derivable values are correctly known through the derivation and
  are deliberately not enumerated (`design.md` §2). If a future change needs
  the list itself to be exhaustive, that is a separate decision about a
  deprecated field, and it should be argued there rather than inherited from
  here. Not doing it is the point: a list that must track the Cartesian product
  of two other lists has no terminal state.
- [x] **The 17 warnings are cleared, not fixed, and that is the whole fix.**
  The underlying declarations were already correct — which is why this change
  writes no portfolio file and touches no sibling repository. There is no
  follow-up owner action outstanding on them, and none is created here.
- [x] **The other 18 fleet warnings are untouched and out of scope.**
  `HANDOFF_SCAFFOLD` 8, `DEPLOYMENT_PROMOTION_STALE` 7, `CAPABILITY_BLOCKED` 2,
  `DISCOVERABILITY_INVALID` 2, `CI_MISSING` 2,
  `QUALITY_README_CLAIM_UNSCOPED` 1. Each is a separate repository's
  declaration or document, and none is a `KIND_UNKNOWN`.
- [x] **`identity-and-taxonomy-axis-split` is not archived.** Its migration is
  applied and its work is committed, but archiving is not this change's to do
  and is not a task here.
