# Tasks: Anchor the TypeScript test-code match on the qualifier

## 1. BFS — Measure the false positives before fixing anything

- [x] Run the real checker over the four repositories named in the report
- [x] Count: `argoset` 2, `crossify` 1, `fotofy` 6, `transitfy` 6
- [x] Read every one of the 15 lines and confirm each is a regex, a compiled
      `RegExp`, a `String` method, or a comment — **15 findings, 0 real**
- [x] Do not trust the task note's counts; they were taken from an earlier run
      and the tree had moved
- **Done when** the false-positive count is measured, not quoted.

## 2. DFS — One coherent contract

- [x] `proposal.md` — the measurement, the discriminator table, what is refused
- [x] `specs/product-code-quality-boundary/spec.md` — MODIFIED the product-scope
      requirement with six scenarios, including one that fails if the anchor is
      removed
- [x] `design.md` — why a name match cannot work, and why comments are excluded
      separately from the qualifier
- **Done when** `openspec validate --all --strict` passes.

### 2.1 Anchor on the qualifier

- [x] Add `_TS_TEST_CALL`, which rejects a preceding `.` or word character, so
      `contest(` and `latest` do not match
- [x] Add `_TS_MEMBER_CALL` and strip member calls **before** the final
      decision, so a line carrying both a member call and a bare call is still
      reported — the bare call is the test
- [x] Exclude comment prefixes (`//`, `/*`, `*`). A comment cannot call
      anything, and the rule's remediation makes no sense for prose
- [x] Keep the existing `import` exclusion
- **Done when** the scanner is one function with no file or line allow-list.

## 3. BFS — Re-audit every consumer

- [x] `_scan_typescript_test_code` serves both `typescript` and `javascript`
      (`LANGUAGE_TEST_SCANNERS`). One fix, two languages — confirmed in the
      dispatch table, not assumed
- [x] The Rust and C# scanners are separate functions and are untouched. The
      Rust one already reads the module tree
- [x] The finding's `rule_id`, level, severity and remediation string are
      unchanged, so the gate's declared code scope is unaffected
- **Done when** no other scanner shares the changed code path.

## 4. BFS — Tests, and the control that keeps the rule alive

- [x] `TypeScriptQualifierTests.test_no_method_or_comment_is_reported_as_test_code`
      — all 9 measured shapes, one `subTest` each, so a regression names the
      shape rather than "a test failed"
- [x] `TypeScriptQualifierTests.test_a_bare_global_call_is_still_reported` —
      the negative control. `describe(`, `it(` and `test(` on a bare identifier
      must each still be reported, or the anchor was dropped and the rule is
      dead
- [x] The pre-existing `test_describe_call_in_source_is_rejected` still passes
      unmodified, so the end-to-end path was not weakened
- [x] Add the two imports the new tests need (`pathlib`, `shutil`) — they were
      missing, and the first run was 12 errors for that reason alone
- **Done when** 50 tests pass in `test_product_code_quality.py`.

## 5. BFS — Verify against the fleet, not against the fixture

- [x] Re-run the four repositories: **15 → 0**
- [x] Sweep every repository with a `.project.json` and count
      `product-test-code` findings fleet-wide
- [x] The result is **1**, in `alltools-platform`, and it is a genuine `#[test]`
      in Rust source — not a TypeScript artefact, and not something to fix here
- [x] `python3 -m unittest discover -s tests` — **812 passed, 0 failed**
- [x] `scripts/workspace_check.py --self-check` — exit 0
- **Done when** the numbers are measurements.

## 6. BFS — Commit

- [x] Commit 1: spec, design, tasks, `product_code_quality.py`, the new tests
- [x] Commit 2: `HANDOFF.md` only
- [x] Explicit paths, never `git add -A`
- [x] On `main`. No branch, no worktree, no push.
- [x] Archive with the owner's authorization. `opendockify`'s
      `readme-claim-unscoped` is a checker-coverage gap, not this rule's
      subject, and remains recorded separately.
