## Implementation Boundary

Owners:

| Surface | Owner | In this package |
|---|---|---|
| The current handoff | `forge` | Yes |
| The historical archive | `forge` | Yes |
| The governance checker | `forge` | Yes |
| Archived change evidence | history | No — moved verbatim |

## The Cut

The file is read top to bottom by agents and has four regions:

| Region | Lines (before) | Kept |
|---|---|---|
| `## Latest delivery` and the newest entries | 1–390 | yes |
| `## Purpose` | 391–1615 | yes |
| `## Current state` | 1616–6357 | yes |
| Historical cycles, ending with `## Change 50` | 6358–12525 | moved |

The cut is the `## Seven in-flight changes consolidated, then archived` heading.
It is a top-level section boundary, so no sentence or table splits. Everything
from there to EOF is history.

## Why a Root-Level Archive

`HANDOFF-ARCHIVE.md` sits at the repository root rather than under `docs/` so
every relative link in the moved text resolves to the same target as before.
Moving it under `docs/` would require rewriting every `src/…`, `openspec/…` and
`docs/…` link by one directory level, which is exactly the kind of mechanical
edit that introduces silent breakage.

The archive keeps a heading that names what it is and the date of the cut; the
moved text is otherwise byte-identical.

## The Check

`scripts/check-spec-governance.mjs` already owns the pointer, placeholder, link
and count rules. It gains one rule:

```text
if HANDOFF.md has more than 8000 lines -> fail naming the line count
if HANDOFF-ARCHIVE.md exists and HANDOFF.md does not link to it -> fail
```

The budget is deliberately above the post-rotation size (~6,400 lines) and
leaves room for many cycles. When it is reached, the next rotation moves another
historical span instead of raising the number.

## The Repointed Link

`docs/ci-cd-system-model.md` ends with:

```text
[current HANDOFF](../HANDOFF.md#remaining-consolidation-work)
```

`remaining-consolidation-work` is in the moved span. The link becomes
`../HANDOFF-ARCHIVE.md#remaining-consolidation-work`, and the governance link
check proves it resolves.

## Failure Boundaries

- The cut is at a `##` heading; a test-free eyeball check confirms the first
  lines of the archive are the heading, not a paragraph.
- A missing archive link fails the governance check.
- A file over 8,000 lines fails the check.
- `current_spec` stays in `HANDOFF.md` (never in the archive).

## Verification Strategy

1. `node scripts/check-spec-governance.mjs` passes.
2. `wc -l HANDOFF.md` is below 8,000 and `HANDOFF-ARCHIVE.md` contains the moved
   heading and the last original line.
3. A deliberate `9000`-line padding makes the budget rule fail (mutate,
   restore) — or the rule is read back from the script.
4. `openspec validate --all --strict --no-interactive`, `git diff --check` and
   the repository Gate pass.
