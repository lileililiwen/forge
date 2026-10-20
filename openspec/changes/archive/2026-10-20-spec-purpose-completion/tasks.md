# Tasks: spec-purpose-completion

## 1. Plan

- [x] 1.1 Read the mandatory context: `AGENTS.md`, `.ai-rules/workflow.md`, `.ai-rules/completion.md`.
- [x] 1.2 Find every canonical capability whose `## Purpose` is the archive placeholder, and confirm how many there are.
- [x] 1.3 Read the capabilities that already state a purpose, so the fourteen match their voice and shape.
- [x] 1.4 Confirm how the OpenSpec archive writes the placeholder, and that it writes it only for a new capability.
- [x] 1.5 Decide where the guard lives: the Gate's check commands are generated and the product-quality checker is vendored, so a bound checker is not available; a repository contract test is.

## 2. Design

- [x] 2.1 Decide the shape of a purpose: one paragraph, 35–70 words, the present tense, the capability's own vocabulary, no mention of the archive.
- [x] 2.2 Decide the rule that a purpose must be supported by that capability's own requirements, and that nothing is invented.
- [x] 2.3 Decide the guard's rules: the purpose must exist, must not be the archive placeholder, and must be at least 60 characters.
- [x] 2.4 Decide what the guard does when a specification cannot be read: it fails, never skips.
- [x] 2.5 Decide that only the `## Purpose` paragraph changes, and that no requirement, scenario or wording moves.

## 3. Implement

- [x] 3.1 Write the real purpose for each of the fourteen capabilities.
- [x] 3.2 `tests/spec_purpose_contract.rs` (new): the guard.
- [x] 3.3 The guard's own cases: a real purpose passes; a placeholder fails; a missing purpose fails; a label fails.

## 4. Verify

- [x] 4.1 The guard fails while a placeholder is in place and passes once the fourteen purposes are written.
- [x] 4.2 The repository test suite passes.
- [x] 4.3 The repository Gate passes, locally, before the archive.
- [x] 4.4 `openspec validate --all --strict --no-interactive`, `node scripts/check-openspec-change-names.mjs` and `git diff --check` pass.
- [x] 4.5 The diff shows only the `## Purpose` paragraphs changed in the fourteen capabilities.

## 5. Archive

- [x] 5.1 Archive without `--skip-specs`, so the new capability is promoted.
- [x] 5.2 Replace the archive's `TBD` Purpose with the real one.
- [x] 5.3 Commit the implementation with its specs, then update the handoff pointer and commit that separately.

## 6. Verification evidence

Recorded by the main agent on 2026-10-20, before the archive.

| Check | Result | Evidence |
|---|---|---|
| The guard fails on a placeholder | PASS | With `openspec/specs/ci-cd-system-model-documentation/spec.md` temporarily holding the archive placeholder again, `cargo test --test spec_purpose_contract` reported `test result: FAILED. 1 passed; 1 failed` and the failure named the capability: `1 of 133 canonical capabilities state no real purpose: - ci-cd-system-model-documentation: the purpose is still the archive placeholder — replace it with what the capability is for`. The file was restored and the same command reported `2 passed; 0 failed`. |
| The guard passes on the written purposes | PASS | `cargo test --test spec_purpose_contract` → `test result: ok. 2 passed; 0 failed`. Every one of the 133 canonical capabilities now holds a real purpose; a scan for the archive placeholder across `openspec/specs/` returns 0 matches. |
| Only the purpose changed | PASS | `git diff --stat openspec/specs/` shows 14 files, 96 insertions and 14 deletions — one purpose paragraph per file. No requirement header, scenario or wording moved. |
| Repository test suite | PASS | `cargo test --workspace --all-targets -- --skip rust_scaffold_builds_and_tests_with_native_toolchain --skip rust_scaffold_builds_with_native_toolchain_without_forge --skip declaration_is_inert_for_native_builds_present_and_absent --test-threads=1` → exit 0; **4004 passed, 0 failed, 1 ignored**, with `grep -c "^failures:"` reporting 0. The two new tests are the guard's. |
| Repository Gate | PASS | `forge gate --dry-run` resolved all 8 required checks with none unbound. The full `forge gate --timeout-secs 1800` run exited 0; `.forge/gate/forge/evidence.json` records `"aggregate": "passed"` at `"observed_at": "2026-10-20T20:52:25Z"` on revision `3feff6d`, with `build`, `declared-verification`, `placeholder-threshold`, `product-code-boundary`, `repository`, `security`, `source-file-size` and `tests` all in state `pass`. The `source-file-size` check reported "evaluated 456 of 456 candidate file(s); maximum 1000 physical line(s)". |
| OpenSpec, change names, whitespace | PASS | `openspec validate --all --strict --no-interactive` → `Totals: 134 passed, 0 failed (134 items)`; `node scripts/check-openspec-change-names.mjs` → `PASS`; `git diff --check` → clean. |
