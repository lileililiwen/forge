# Design: A derived `kind` is known by its own derivation

## 1. The shape of the defect

`kind` is a **derived, deprecated** field. It has exactly one authority: the
pair `role` and `stack`.

```
derive_kind(role, stack) = role              if stack == "none"
                          = f"{stack}-{role}" otherwise
```

`vocabulary.json` `kinds` predates that derivation. It is a hand-authored list
of 12 values written when `kind` *was* the classification. It was left frozen on
purpose by `identity-and-taxonomy-axis-split` — deliberately, so the original
vocabulary is not rewritten as a side effect of an axis split.

The two are now different things, and the checker was still treating the frozen
one as the authority for the derived one:

| | `kinds` | `derive_kind(role, stack)` |
|---|---|---|
| cardinality | 12, fixed | 6 × 6 = 36 |
| authored by | hand | computed |
| changes when | a role or stack is added | nothing |
| relationship | — | 27 of 36 values **absent** from `kinds` |

So `validate_kind` — "a `kind` outside the canonical set is `KIND_UNKNOWN`" —
was rejecting values that the current rule requires. `KIND_UNKNOWN` was not
wrong about its own wording; its input was incomplete.

## 2. Why not extend the vocabulary

Extending `kinds` to the 36 values would be the smallest diff. It is also wrong,
for three reasons.

**It is unbounded.** `kinds` would have to be re-extended whenever a role or a
stack is added: 7 roles × 6 stacks, then 7 × 7. A list that must track the
Cartesian product of two other lists is a manual-maintenance liability with no
terminal state. This is the same class of defect as
`driftwatchdog`'s `GOVERNANCE_QUALITY_DECLARED_CODES` allow-list, which was
found in the sibling audit: a hand-kept set standing in for something the code
can already compute.

**It creates a second source of truth.** `kind` would then be constrained by two
independent lists that can disagree. When they do, nothing says which is
authoritative — and the answer ("the axes, always") would have to be
rediscovered. The fix removes the ambiguity instead of institutionalising it.

**It would break an existing test on purpose.** `kinds` is frozen, and
`test_existing_arrays_are_unchanged` pins `len(raw["kinds"]) == 12` and
`raw["kinds"][6] == "shared-library"`. Changing the list means weakening a test
to make a code change go green, which this repository treats as a defect in its
own right.

## 3. Why not rewrite the 17 declarations

The pre-migration hand-authored values were bare role words (`forge` was
`platform`, `driftwatchdog` was `tooling`). Reverting the 17 findings to that
shape would:

- reintroduce the role/stack conflation the axis split exists to remove — a
  Rust platform would again be indistinguishable from a `.NET` platform;
- make `kind` disagree with its own axes again, so all 17 become
  `TAXONOMY_LEGACY_VALUE` warnings — 17 warnings traded for 17 warnings;
- write into 17 sibling repositories, which the cross-repo boundary in
  `AGENTS.md` gates behind separate authorization, for no gain.

## 4. The rule

`validate_kind` asks, in order, and stops at the first that holds:

1. `kind` is not a non-empty string → report `KIND_UNKNOWN`.
2. `kind` is in the legacy `kinds` list → **known**, return.
3. both `role` and `stack` are present, both canonical, and
   `kind == derive_kind(role, stack)` → **known**, return.
4. otherwise → report `KIND_UNKNOWN`.

Clause 3 is the whole change. Its preconditions are what make it safe:

- **both axes present** — a bare `kind` cannot say whether the stack was `none`
  or simply unstated, so nothing is derived and nothing is measurable. This is
  the same guard `validate_taxonomy` already applies before comparing against a
  derivation, and the two rules now agree on it.
- **both axes canonical** — a non-canonical axis is already
  `TAXONOMY_AXIS_UNKNOWN`; deriving from it would bless a value the audit has
  already rejected. A `role` of `library` must not make `rust-platform` known.
- **equality with the derivation** — this is what keeps the hand-authored case
  reported. A value that is in neither the list nor the derived form is exactly
  what `KIND_UNKNOWN` is for.

The result: `KIND_UNKNOWN` still fires for anything an owner could act on. It
stops firing only for values that are correct.

## 5. Implementation

One helper, one new early return, two optional keyword arguments:

```python
def _is_self_consistent_derived_kind(declaration, kind, *, canonical_roles, canonical_stacks) -> bool
```

`validate_kind` gains `canonical_roles` / `canonical_stacks` keyword arguments
mirroring the `canonical_kinds` it already had, and mirroring
`validate_taxonomy`'s existing shape. This keeps the rule testable against a
frozen set of axis values rather than the live `vocabulary.json`, and lets a
caller override one set without silently getting the other from disk. Both
default to `None` → resolve from `governance_root`, so every existing call site
is unchanged.

**Ordering matters for cost and for failure mode.** The axis lookup happens
*inside* the branch where a finding would otherwise be emitted, after the
membership test has already failed. A canonical `kind` — the common case —
therefore reads no axis set at all, so the change cannot introduce a vocabulary
read on a path that previously did not make one. Vocabulary loads are already
cached per resolved root (`vocabulary._CACHE`), so the two lookups on the
reporting path are a dict lookup each.

## 6. What is deliberately unchanged

- **`KIND_UNKNOWN` stays WARNING.** Not promoted, not demoted, not renamed, not
  suppressed. A non-canonical, non-derived `kind` is still a finding.
- **`kinds` stays 12.** No vocabulary edit, no test weakened.
- **No declaration is written.** 0 sibling files touched.
- **`validate_taxonomy` is untouched.** It already had the derivation
  comparison and already reported `TAXONOMY_LEGACY_VALUE` correctly; the two
  rules are now consistent rather than one of them changed.
- **`profile_requirements` is untouched.** The axes remain descriptive and
  drive no requirement, file, CI command, or promotion state.

## 7. Measurement

| Measurement | Before | After |
|---|---|---|
| Fleet audit errors | 205 | 205 |
| Fleet audit warnings | 35 | **18** |
| `KIND_UNKNOWN` on the fleet | 17 | **0** |
| `--self-check` | 0 errors / 1 warning | **0 / 0** |
| Test suite | 492 | 501 |

The 17-finding drop is the acceptance target: if warnings do not land on 18,
either a finding outside `KIND_UNKNOWN` moved or a derived value is still being
rejected, and both are defects in this fix rather than something to tune away.

## 8. The tests, and why each side is pinned

Nine tests, in `tests/test_vocabulary.py::DerivedKindKnownTests`. Both sides of
every branch, because a fix that only proves the happy path is not a fix.

| Test | Pins |
|---|---|
| `test_a_self_consistent_derived_value_produces_no_finding` | the new rule, and asserts `rust-platform` is genuinely not in `kinds` so it cannot pass by accident |
| `test_every_derived_value_outside_the_legacy_vocabulary_is_known` | the whole 36-value space against the real axis sets, and asserts the unlisted count is `> 0` — the sweep cannot pass on a vocabulary that already covered it |
| `test_a_legacy_value_that_disagrees_with_its_own_derivation_still_reports` | `rust-governance` under `governance`/`python` still reports, at WARNING |
| `test_a_value_in_neither_form_still_reports` | `quality-gate` still reports, message names it |
| `test_absent_axes_behave_as_before` | no axes → no derivation → still reports |
| `test_a_half_declared_axis_pair_behaves_as_before` | role without stack → still reports |
| `test_non_canonical_axes_behaves_as_before` | a bad `role` **and** a bad `stack` → still reports |
| `test_an_absent_or_non_string_kind_still_reports` | `{}`, `{"kind": None}`, `{"kind": 7}` → still reports |
| `test_a_canonical_kind_stays_known_with_unrelated_axes` | a legacy-list value whose axes say something else is still known |

Verified to bite: against the pre-fix body the two new-behaviour tests fail and
the seven unchanged-behaviour tests pass. The fix widens knowledge only where it
should, and the seven are guards against a future edit widening it further.
