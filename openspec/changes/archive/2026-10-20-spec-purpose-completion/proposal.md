# Change: spec-purpose-completion

## Why

Every capability the OpenSpec archive promotes starts with the same line:

```
## Purpose
TBD - created by archiving change <name>. Update Purpose after archive.
```

The archive writes it because it cannot know what the capability is for — it
only knows the requirements the change declared. Somebody has to write the
real purpose, and nobody did: fourteen canonical capabilities still carry the
placeholder.

That is not cosmetic. `openspec/specs/` is the repository's only durable
design source. A capability whose Purpose says "TBD" tells a reader — a
person or an agent opening the spec to learn what a surface guarantees —
nothing at all. The requirements below it are precise, and the sentence above
them is missing.

The placeholder also recurs: every archive writes one, so a change that adds
a capability lands a new placeholder unless somebody remembers to replace it.
Two changes in this programme (`runtime-kind-shared-host` and
`lifecycle-observation-closure`) had theirs replaced by hand; the next one
would not be noticed at all.

## What Changes

- **The fourteen placeholder purposes are written for real.** Each one says
  what the capability is for, in the product's own terms, in the present
  tense, drawn from that capability's own requirements and nothing else.
- **The requirements are not touched.** Only the `## Purpose` paragraph
  changes. No requirement header, no scenario, no wording moves.
- **The placeholder can no longer survive unnoticed.** A repository contract
  test reads every canonical capability and refuses one whose Purpose is the
  archive placeholder, is missing, or says nothing. The test names the
  capability it refuses.

## Package Boundary

| Capability | Owner | Type | State | Notes |
|---|---|---|---|---|
| `spec-purpose-completion` | forge | Markdown + Rust | implemented | this package |
| The fourteen placeholder capabilities | forge | Markdown | modified | their `## Purpose` paragraph only |
| Every other canonical capability | forge | Markdown | read only | their purposes are already real |
| The OpenSpec tool | its own package | TypeScript | read only | not edited; the placeholder it writes is the symptom, not the cause |

## Non-Goals

- **No requirement is rewritten.** A purpose states the shape of the
  capability; the requirements stay the contract.
- **The OpenSpec tool is not patched.** The tool is a published package
  outside this repository. The guard catches what it writes instead.
- **No capability is merged, split or renamed.** The fourteen keep their
  names, their requirements and their scenarios.
- **No new prose is invented for the other 119 capabilities.** Their purposes
  are already written; this change does not rewrite them.
