## 1. BFS — Baseline and impact coverage

- [x] 1.1 Record the actual archived-change and canonical-spec counts.
- [x] 1.2 Collect every count claim and its file and line.
- [x] 1.3 Confirm the three `.project.json` fields contradict observable facts.
- [x] 1.4 Confirm the Doctor/profile gap needs a product decision.

## 2. DFS — Requirement-by-requirement implementation

- [x] 2.1 Add the count rule to `scripts/check-spec-governance.mjs`.
- [x] 2.2 Update the four documents to the computed counts.
- [x] 2.3 Correct `visibility`, `evidence_status` and `gate_runtime`.
- [x] 2.4 Record the proposed `native-project-profile` follow-up in `ROADMAP.md`.

## 3. BFS — Cross-surface regression and completeness

- [x] 3.1 The governance check passes on the real counts.
- [x] 3.2 A wrong count fails naming the file and both numbers (mutate, restore).
- [x] 3.3 `.project.json` still parses and no test depends on the old values.
- [x] 3.4 The four claims are the only count claims in scope.

## 4. Verification

- [x] 4.1 Run the repository test suite.
- [x] 4.2 Run OpenSpec validation, the governance check, formatting and whitespace checks.
- [x] 4.3 Run `forge gate --dry-run`, then the full Gate.
- [x] 4.4 Record every command's exact result in the evidence table.
- [x] 4.5 Archive without `--skip-specs`, commit, then update the handoff pointer.

## Evidence

| Check | Result | Evidence |
|---|---|---|
| Count drift | Fixed | Before: `openspec/config.yaml` 118/94, `docs/requirements-coverage.md` 129/103, `ROADMAP.md` 155/113, `README.md` 167/121. Actual before this archive: **225 archived changes / 172 canonical specs**; after the archive: **226 / 172**. |
| Count rule | PASS | `scripts/check-spec-governance.mjs` now computes both counts from the directories and checks all four files; it passed with 225 and, after the archive and refresh, with 226. |
| Count rule catches drift | PASS | Rewriting README to `999 archived OpenSpec changes` made the check fail with `README.md:7: archived-change count 999 does not match the 225 directories under openspec/changes/archive`; restoring the file returned `PASS`. |
| `.project.json` truth | PASS | `visibility: "public"` (matches `gh repo view`), `evidence_status: "verified"` (suite and Gate run every cycle), `gate_runtime: "forge"` (bundled engine). The file parses as JSON. |
| Proposed follow-up | Recorded | `ROADMAP.md` names `native-project-profile` and the two decisions it must resolve; no implementation is claimed. |
| Repository test suite | PASS, both test-bearing Gate checks | The full Gate ran `declared-verification` and `tests` (the same fast suite) and both passed; `scripts/check-tests.sh` includes the Python import checks and the identity-provider workspace. |
| Repository Gate | PASS, 8/8 | `forge gate . --timeout-secs 1800` exited 0 with aggregate `passed` at `2026-10-30T14:21:42Z` on revision `582c3828`. |
| OpenSpec and whitespace | PASS | Name preflight `PASS`; strict validation `173 passed, 0 failed`; `check-spec-governance` `PASS`; `git diff --check` clean. |

