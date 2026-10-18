# Proposal: A derived `kind` is known by its own derivation

## Why

The fleet audit reports 17 `KIND_UNKNOWN` warnings, one per repository, and
every one of them is correct. This is a checker defect, not 17 portfolio
defects.

`identity-and-taxonomy-axis-split` made `kind` a **derived, deprecated**
composite: `derive_kind(stack, role)` returns `f"{stack}-{role}"`, or the bare
`role` when `stack == "none"`. The migration landed and the invariant holds —
`kind == derive_kind(role, stack)` for all 81 declarations, which is why
`TAXONOMY_LEGACY_VALUE` is 0.

`vocabulary.json` `kinds` was deliberately left **frozen** at its original 12
hand-authored values:

```
deployment, dotnet-product, governance, inventory, platform, product,
python-product, rust-product, shared-library, tooling,
typescript-monorepo, typescript-product
```

The derivation is a product of two enumerated axis sets — 6 roles × 6 stacks =
36 values — and **27 of those 36 are absent from `kinds`**. The fleet's 17
findings are simply the unlisted values that happen to be in use:

| Value | Repositories |
|---|---|
| `rust-platform` | `forge`, `labrys`, `platform-contracts`, `rust-platform-libs` |
| `rust-tooling` | `cargo-sonar`, `catchen`, `driftwatchdog`, `sisyphusfy` |
| `python-governance` | `workspace-governance` (this repository) |
| `python-tooling` | `alltools-platform`, `ariadex`, `cargo-audit-boost`, `cargo-matrix-boost`, `jenkins-local` |
| `python-platform`, `python-deployment`, `python-inventory`, `dotnet-*` | the remainder |

`validate_kind` fires on a value that is *correct by the current rule*. Three
repositories the rest of the workspace reasons from — `forge` (the platform),
`driftwatchdog` (the gate runtime), and this repository itself — carry a
permanent warning that no owner can clear. A warning nobody can clear is noise
that teaches people to ignore the checker.

The self-check is 1 error / 1 warning for exactly this reason: the
`KIND_UNKNOWN` on `workspace-governance` itself.

## What

Make `validate_kind` recognise the derived form. A `kind` outside the legacy
`kinds` list is **known** when it equals the value its own two axes derive, with
both axes present and canonical.

Everything else keeps reporting at WARNING:

- a hand-authored legacy value that disagrees with its own derivation
  (`role: platform, stack: rust, kind: rust-governance`);
- a one-off value in neither form (`kind: quality-gate`);
- any value whose axes are absent, half-declared, or non-canonical;
- an absent or non-string `kind`.

The severity stays WARNING, the code stays `KIND_UNKNOWN`, and the rule is not
silenced: it now asks one more question before reporting.

## What this deliberately does not do

**It does not add the 9 missing strings to `kinds`.** That relocates the
problem. The derivation space is the product of two axis sets, so the list
would need re-extending for every future axis combination — the same
unbounded manual-maintenance point already found in `driftwatchdog`'s
`GOVERNANCE_QUALITY_DECLARED_CODES` allow-list. It would also freeze a derived
field's value space into a second source of truth that can contradict the
first. `kinds` stays 12 and keeps its existing test
(`test_existing_arrays_are_unchanged` asserts `len(raw["kinds"]) == 12`).

**It does not edit the 17 declarations back to bare role words.** That
reintroduces precisely the role/stack conflation the change eliminated, and it
makes `kind` disagree with its own axes — trading a 0-count `TAXONOMY_LEGACY_VALUE`
for 17 new ones.

## Impact

| Surface | Before | After |
|---|---|---|
| Fleet audit | `errors=205 warnings=35` | `errors=205 warnings=18` |
| `--self-check` | `errors=0 warnings=1` | `errors=0 warnings=0` |
| Test suite | `Ran 492 tests … OK` | `Ran 501 tests … OK` |
| `vocabulary.json` | 12 kinds | 12 kinds — unchanged |
| Declarations written | — | none; 0 fleet files touched |

`errors=205` is unchanged: the rule is WARNING and nothing here can promote or
demote an existing finding. The 17 cleared warnings are exactly the 17
`KIND_UNKNOWN` findings; no other finding count moves.
