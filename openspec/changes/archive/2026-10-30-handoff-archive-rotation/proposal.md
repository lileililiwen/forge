# Change: handoff-archive-rotation

## Why

`HANDOFF.md` is 12,525 lines and about 1 MB. Every session reads it before it
does anything, and every completed change prepends another entry without ever
removing one. The file is now the largest tracked source in the repository and
its growth is unbounded: the most recent cycle added four entries, and nothing
in the workflow says when history stops being current state.

The content already separates: the newest deliveries, a `Purpose` section, a
`Current state` section, and roughly the second half is a historical change log
("Change 1" through "Change 50") that no longer describes the current tree. That
history is valuable evidence and must be retained; it does not need to be in the
file every agent reads first.

## What Changes

- **Historical cycles move to `HANDOFF-ARCHIVE.md`.** Everything from
  `## Seven in-flight changes consolidated, then archived` to the end is moved
  verbatim into a root-level archive file, where relative links keep resolving
  exactly as before. `HANDOFF.md` keeps the newest deliveries, Purpose and
  Current state, and links to the archive.
- **One documentation link is repointed.** `docs/ci-cd-system-model.md` links to
  `HANDOFF.md#remaining-consolidation-work`; that section moves, so the link
  follows it to `HANDOFF-ARCHIVE.md`.
- **The governance check bounds the file.** `scripts/check-spec-governance.mjs`
  fails when `HANDOFF.md` exceeds 8,000 lines, and requires a link to
  `HANDOFF-ARCHIVE.md` when that archive exists.

## BFS Impact Map

| Surface | Effect |
|---|---|
| `HANDOFF.md` | historical sections moved out; pointer added |
| `HANDOFF-ARCHIVE.md` | new root-level archive, verbatim history |
| `docs/ci-cd-system-model.md` | one anchor repointed |
| `scripts/check-spec-governance.mjs` | line budget and archive-link rule |
| `openspec/specs/specification-governance-refresh/spec.md` | one added requirement |

Persistence: none.

Failure boundaries: a cut that splits a section is avoided by cutting only at
the chosen top-level heading; a broken link fails the governance check; a
future file over 8,000 lines fails it too.

## Capabilities

| Capability | State | Notes |
|---|---|---|
| `specification-governance-refresh` | modified | bounded handoff, retained history |

## Non-Goals

- No rewriting of historical entries. They move byte-for-byte.
- No change to the archive/spec counts or to the promotion rules.
- No deletion of evidence.
