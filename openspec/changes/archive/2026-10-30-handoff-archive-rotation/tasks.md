## 1. BFS — Baseline and impact coverage

- [x] 1.1 Measure `HANDOFF.md` and locate its top-level regions.
- [x] 1.2 Choose a cut at a top-level heading with no split table or sentence.
- [x] 1.3 Find every link or anchor that points into the moved span.

## 2. DFS — Requirement-by-requirement implementation

- [x] 2.1 Move the historical span verbatim into `HANDOFF-ARCHIVE.md`.
- [x] 2.2 Add the pointer from `HANDOFF.md` to the archive.
- [x] 2.3 Repoint the `docs/ci-cd-system-model.md` anchor.
- [x] 2.4 Add the line-budget and archive-link rule to the governance check.

## 3. BFS — Cross-surface regression and completeness

- [x] 3.1 The governance check passes with the rotated file.
- [x] 3.2 `HANDOFF.md` is below the budget; the archive holds the moved heading.
- [x] 3.3 The repointed anchor resolves.
- [x] 3.4 No historical entry was edited; only moved.

## 4. Verification

- [x] 4.1 Run the repository test suite.
- [x] 4.2 Run formatting, OpenSpec validation, the governance check and whitespace checks.
- [x] 4.3 Run `forge gate --dry-run`, then the full Gate.
- [x] 4.4 Record every command's exact result in the evidence table.
- [x] 4.5 Archive without `--skip-specs`, commit, then update the handoff pointer.

## Evidence

| Check | Result | Evidence |
|---|---|---|
| Handoff size | 12,525 -> 6,362 lines | Before the rotation `wc -l HANDOFF.md` reported 12,525; after it reported 6,362, with 6,174 lines in the new `HANDOFF-ARCHIVE.md`. |
| History retained verbatim | PASS | `git show HEAD:HANDOFF.md` from the cut heading onward and `HANDOFF-ARCHIVE.md` from the same heading were byte-identical (534,162 bytes each). |
| Budget rule | PASS | `node scripts/check-spec-governance.mjs` passes at 6,362 lines; appending 2,000 padding lines made it fail with `HANDOFF.md:8363: handoff is 8363 lines; the budget is 8000 — rotate the oldest span into HANDOFF-ARCHIVE.md`; restoring returned `PASS`. |
| Archive link check | PASS | The same check requires `HANDOFF.md` to link `HANDOFF-ARCHIVE.md` when the archive exists; the pointer section does. |
| Anchor repointed | PASS | `docs/ci-cd-system-model.md` now links `../HANDOFF-ARCHIVE.md#remaining-consolidation-work`; the governance link check passes. |
| Repository test suite | PASS, both test-bearing Gate checks | The full Gate ran `declared-verification` and `tests` (the fast suite with its Python and identity-provider steps) and both passed. |
| Repository Gate | PASS, 8/8 | `forge gate . --timeout-secs 1800` exited 0 with aggregate `passed` at `2026-10-30T14:38:30Z` on revision `b8b3bb0a`. |
| OpenSpec and whitespace | PASS | Name preflight `PASS`; strict validation `173 passed, 0 failed`; `check-spec-governance` `PASS`; `git diff --check` clean. |

