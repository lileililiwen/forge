# Project Completion

Completion is a security check, not a feeling: three gates plus targeted special
checks. Every gate item must be passable with named evidence; an item you cannot
point to is a failed item, and incomplete or blocked work is reported as
blocked, never complete.

## Gate 1 — entry (before code)

- `HANDOFF.md` read; `openspec list` shows at most one active change. Complete and archive it before beginning another.
- The single `current_spec:` pointer in `HANDOFF.md` set to that change.
- Impact mapped (BFS): actors, journeys, interface states, contracts, callers, persistence, tests, and applicable concerns.
- UI/UX assessment recorded, naming the human-facing surface or evidence for `N/A`.
- Shared infrastructure assessed by name before any local implementation.

## Gate 2 — in-flight (during work)

- Implement only the selected change; placeholders stay confined to it.
- Each requirement/scenario is verified as it is implemented, not batched at the end.
- A structural pass is labeled `SKELETON_READY`, never DONE.

## Gate 3 — exit (before archive, completion claims, or commits)

- `node scripts/check-openspec-change-names.mjs` passes; strict OpenSpec validation passes; the change is archived without `--skip-specs`.
- The declared verification command and the local Gate runtime ran, with the verdict recorded in `HANDOFF.md`; CI is a second layer, not the first.
- Human-facing changes carry real interface evidence (browser/interaction, responsive, accessibility); API or unit success cannot waive it.
- `AGENTS.md` Project facts, the directory tree, and README claims are re-checked against the tree and updated in the same commit if anything changed.
- The diff adds no secrets, edits no generated directories, and hand-edits no lockfiles; files are staged by explicit path.
- Requirements, scenarios, migrated callers, and the original impact surface are rechecked.
- Build/test/skeleton success is not runtime or release evidence.
- Each completed OpenSpec spec/change requires exactly two commits: first the implementation/tests/archive commit, then a HANDOFF.md-only pointer/evidence commit. Stop after commit 2; do not push.

## Special check

Changes touching security or privacy, data migration, public contracts, or
dependency upgrades load the matching `.ai-rules/concerns/` pack and add its
targeted evidence item to Gate 3 — migration reversibility, contract fixture
validation, provider-conflict audit, secret scan, as applicable. This is
heightened screening in addition to the standard gates, never instead of it.
