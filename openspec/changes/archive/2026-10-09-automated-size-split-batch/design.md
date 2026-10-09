# Design: Automated size-split batch

## 1. Implementation boundary

- **Repository / project:** this Forge repository, the `forge`
  crate (library + binary). No sibling touched.
- **Modules changed:** the 45 `src/` files in the proposal
  table, each becoming a same-location module tree
  (`mod.rs` + domain-named submodules), plus the absorbed
  `src/registry/interest/` tree (kept as split).
- **Modules reused unchanged:** every `tests/` file and every
  `src/` file already under the cap. Parent `mod ...;`
  declarations are untouched: `mod.rs` inputs keep their
  module identity, and leaf→dir moves (`foo.rs` →
  `foo/mod.rs`) resolve under the existing declaration.
- **Must NOT change:** any public symbol path, item name,
  function body, contract string, refusal code, metric label,
  CLI output, JSON shape, journal column, route, catalog row,
  env var, `--version` output, or test name/count.

## 2. Language and runtime

- Rust 2021, `rustc 1.87` floor. Tool: `splitrs` 0.3.5.
- Per split: dry-run preview first, then the real run with
  `--rollback` (backups kept until the file verifies green,
  then dropped).
- Leaf files pass `--deepen-super` (modules sit one level
  deeper, so inherited `super::X` becomes
  `super::super::X`); `mod.rs` inputs do not (depth
  unchanged). Never `--parallel`; files processed one at a
  time, smallest-first.
- Constrained commands only: every cargo invocation prefixed
  `CARGO_BUILD_JOBS=2` with `-j2`; tests append
  `-- --test-threads=2`. Drop to `-j1` if thrash persists.

## 3. Ownership and shared code

Per-file shape after split + rename:

```
src/<area>/...        (mod.rs input: split inside its own dir)
  ├── mod.rs          (file doc, `pub mod <domain>;` decls, glob re-exports)
  ├── <domain_1>.rs   (one item family, bodies verbatim)
  └── ...

src/<leaf>/           (leaf input: new sibling dir of the same stem)
  ├── mod.rs          (file doc, decls, re-exports; original file removed)
  ├── <domain_1>.rs
  └── ...
```

`mod.rs` keeps the original file doc and re-exports every
moved name at its exact prior visibility, so every existing
caller path (`crate::<path>::<Item>` and onward re-exports)
resolves with zero call-site edits. New files carry only the
`use` lines their moved items need. The rename pass
(`git mv` of generic buckets → domain names + `mod.rs`
declaration/re-export update) inspects items first; no two
modules in one dir share a name; names stay snake_case.

Rollback strategy: splitrs `--rollback` backups restore the
input on tool failure; after each split group
(`cargo fmt` + constrained build + affected suites green)
the backups for verified files are dropped. Any
splitrs rewrite beyond move + imports/visibility is
reverted hunk-wise to the verbatim source. A file that will
not verify green keeps its backup until it does — the batch
never archives a red tree.

## 4. User experience and interface

`UI/UX: N/A`. No CLI change, no API change, no frontend
change, no journal change, no env change. The only
observable difference is the file-vs-directory shape on
disk, which the operator never sees.

## 5. Behavioral model

None — pure moves. Each item body is copied verbatim; the
only mechanical changes are the `mod` declarations, the
re-exports, per-file `use` lines, and `super::` deepening
for leaf splits.

## 6. Contract and compatibility

- No new or modified public symbol path. `cargo doc`
  output, `forge --help` output, every
  `forge <verb> --help`, every `GET /v1/admin/...`
  envelope, every catalog row, every registry/journal row,
  every CLI exit code — all byte-identical.
- No `cargo test` regression: pre/post
  `cargo test --workspace --all-targets` summaries are
  recorded; any delta is proven pre-existing via a stash
  baseline, otherwise it blocks the batch.

## 7. Failure and boundary policy

| Case | Behavior |
|---|---|
| A moved name misses its re-export | `cargo build` fails compile; export fixed, callers never edited |
| A new file misses an import / `super::` depth wrong | `cargo build` fails compile; fixed before moving on |
| splitrs emits a still-oversized file | re-split that output (or `--split-impl-blocks` for one giant impl / `--split-arrays` for one giant table) until every file is <1000 lines |
| A generated module collides with an existing sibling name | renamed in the rename pass before build |
| Breakage after a group | isolated to the last ≤3 splits; fixed or reverted before continuing |
| Gate FAIL / REVIEW_REQUIRED attributable to the batch | blocks archive; fixed first |

## 8. Verification oracle

- **Per group (1–3 files):** `cargo fmt`, constrained
  `cargo build -j2` 0 errors, affected test target(s)
  green with identical counts.
- **Final:** `cargo fmt --check` clean; constrained full
  build 0 errors; full constrained test run with recorded
  counts; every `src/**/*.rs` under 1000 lines (5 largest
  listed as proof); name preflight PASS; spec-governance
  PASS; strict validate clean; `git diff --check` clean.
- **Gate:** `forge gate --dry-run` renders a plan;
  `forge gate --timeout-secs 600` runs; verdict + file-size
  before/after recorded in HANDOFF with attribution.
  `source-file-size` is expected to go fully green (45 → 0
  oversized); the known pre-existing `project-runtime`
  adapter-timeout items must show 0 attributable.
- A task box is checked only with the command output for
  its assertion.

## 9. Follow-on roadmap

None for file size: after this batch no `src/` file
exceeds the cap, so the parent
`source-file-size-remediation` §9 table is fully drained.
Any file that regrows over the cap is a future single-file
change under the same verbatim-move rules.

## 10. Decision ledger

- **Resolved:** automate with splitrs (max-lines 900,
  domain-specific naming, rollback, one file at a time)
  instead of further manual splits — 45 files exceed the
  manual one-file-per-change budget.
- **Resolved:** absorb the in-flight manual
  `registry-interest-size-split` (retire its change package;
  keep its `mod.rs` + `model.rs` tree verbatim) as row 0 of
  this batch rather than archiving it separately.
- **Resolved:** keep the splitrs `glob` facade and
  `pub mod` declarations so historical paths resolve
  without caller edits (parent §10 rule).
- **Resolved:** rename pass is renames-of-files only
  (generic bucket → domain name from inspected content);
  item bodies and item names never change.
- **Blockers:** none.
