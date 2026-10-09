# Proposal: Automated size-split batch

## Why

45 `src/**/*.rs` files remain over the 1,000-physical-line cap
enforced by `forge gate` (`source-file-size`), so the Gate stays
red on that check no matter how many single-file manual changes
land. Five manual splits are archived (3 tests dirs +
`portfolio-interest` + `portfolio-mod`); a sixth manual split
(`registry-interest-size-split`, `src/registry/interest/mod.rs`
1,047 → 937 + new `model.rs` 120) was in flight when the operator
ordered automation. Manual splitting is too slow for 45 files, so
this ONE change splits all of them mechanically with `splitrs`
0.3.5 plus a readability rename pass over the generated bucket
names. The in-flight manual split is absorbed, not archived
separately: its tree changes (`mod.rs` + `model.rs`, already
under the cap, domain-named) are kept verbatim as the batch's
first row and verified with everything else.

Config: `splitrs --max-lines 900 --naming-strategy
domain-specific --rollback`, one file at a time (never
`--parallel`), smallest-first. `--deepen-super` for leaf files
split in place (`foo.rs` → `foo/`); unset for `mod.rs` inputs
split into their own module dir. Facade stays at the splitrs
default (`glob` re-exports) so historical `crate::x::Item`
paths keep resolving.

Rename policy (operator requirement): generic bucket names
(`functions.rs`, `types.rs`, `constants.rs`, `helpers.rs`,
`utils.rs`, `chunk_N.rs`) MUST be renamed to domain-meaningful
snake_case names derived from the actual items inside (inspected
before renaming), with the generated `mod.rs` declarations and
re-exports updated to match. No two modules in one dir share a
name. Bodies stay verbatim — if splitrs rewrites anything beyond
moving items + fixing imports/visibility, that hunk is reverted.

## What Changes

- All 45 remaining oversized `src/` files (table below) are
  split by splitrs into same-location module trees: `mod.rs`
  inputs split inside their own module dir; leaf files become
  same-stem sibling dirs (`foo.rs` → `foo/mod.rs` + modules)
  with the original removed per splitrs convention. Every
  resulting file is under 1,000 lines (target ≤900).
- Absorbed row 0: `src/registry/interest/mod.rs` (937) +
  `src/registry/interest/model.rs` (120, new) from the retired
  manual change, kept as-is, verified in this batch.
- Each generated `mod.rs` declares the submodules and
  re-exports moved names (glob facade, same visibility as
  before), so every `pub`/`pub(crate)` path resolves exactly
  as before; parent `mod` declarations are untouched except
  where Rust module resolution requires it (leaf → dir moves
  need no parent edit).
- No behavior change anywhere: no body edits, no renames of
  items, no contract-string/CLI/JSON/log/env change, no test
  logic change.

## BFS Impact Map

- Requirements/scenarios: the delta spec's two requirements
  (batch split with paths stable; counts identical).
- Concepts/modules: 45 module trees listed below + the
  absorbed registry-interest tree; ~120 new files total.
- Contracts: `pub` paths via re-exports; `forge --help`,
  `GET /v1/admin/...` envelopes, catalog rows,
  registry/journal rows byte-identical.
- Callers: none edited — call sites keep bare names through
  the re-exports; any compile break is fixed by export, not
  by caller edit.
- Persistence/integrations: untouched (pure moves).
- Tests: affected target(s) green after each split group;
  full `cargo test --workspace --all-targets` summary
  recorded before and after (any delta proven pre-existing
  via stash baseline).
- Compatibility: generated projects independent of Forge —
  untouched.
- Concerns: quality (verbatim discipline per parent
  design §10); no new filesystem/process/credential/remote
  boundary (security N/A beyond existing).
- Verification: fmt, constrained build, affected suites per
  group; final fmt-check, full build, full tests, ≤1000-line
  proof, name preflight, spec-governance, strict validate,
  diff-check, Gate dry-run + run.

## Capabilities

- `automated-size-split-batch`: split every remaining
  oversized `src/` file into a same-location module tree with
  verbatim bodies, stable public paths, and domain-meaningful
  module names.

## Non-goals

- No behavior, API, CLI, contract, or test-logic change.
- No `tests/` reshaping (tests-side splits are all archived).
- No harness-timeout or adapter investigation (pre-existing
  Gate `project-runtime` items stay recorded, not fixed).
- No per-file OpenSpec packages (this batch supersedes the
  one-file-per-change roadmap for the listed files only).

## Per-file table (smallest-first; lines at batch start)

| File | Lines | Result |
|---|---|---|
| `src/registry/interest/mod.rs` + `model.rs` (absorbed) | 937 + 120 | kept, verified here |
| `src/portfolio/interest/activation.rs` | 1054 | split |
| `src/publish/inventory.rs` | 1090 | split |
| `src/github/cli.rs` | 1099 | split |
| `src/release/mod.rs` | 1112 | split |
| `src/deploy/mod.rs` | 1121 | split |
| `src/kit/assets.rs` | 1122 | split |
| `src/semantic/review.rs` | 1134 | split |
| `src/api/fleet.rs` | 1180 | split |
| `src/doctor/gaps.rs` | 1200 | split |
| `src/publish/mod.rs` | 1230 | split |
| `src/standard/mod.rs` | 1237 | split |
| `src/github/adapter.rs` | 1291 | split |
| `src/upgrade/mod.rs` | 1300 | split |
| `src/fleet/mod.rs` | 1327 | split |
| `src/spec/mod.rs` | 1339 | split |
| `src/profile/mod.rs` | 1363 | split |
| `src/feature/mod.rs` | 1365 | split |
| `src/deploy/engine.rs` | 1388 | split |
| `src/governance.rs` | 1436 | split |
| `src/registry/mod.rs` | 1443 | split |
| `src/import/mod.rs` | 1469 | split |
| `src/publish/providers.rs` | 1494 | split |
| `src/distribution/mod.rs` | 1517 | split |
| `src/planner/mod.rs` | 1527 | split |
| `src/gate/mod.rs` | 1530 | split |
| `src/registry/portfolio.rs` | 1669 | split |
| `src/component/mod.rs` | 1672 | split |
| `src/procedure/mod.rs` | 1718 | split |
| `src/analytics/mod.rs` | 1789 | split |
| `src/docs/mod.rs` | 1911 | split |
| `src/publish/remote_compose.rs` | 1913 | split |
| `src/policy/mod.rs` | 1914 | split |
| `src/generate/mod.rs` | 1998 | split |
| `src/ui_pattern/mod.rs` | 2137 | split |
| `src/mcp/mod.rs` | 2157 | split |
| `src/portal/mod.rs` | 2191 | split |
| `src/agent/mod.rs` | 2204 | split |
| `src/identity/mod.rs` | 2207 | split |
| `src/release/engine.rs` | 2258 | split |
| `src/provider/mod.rs` | 2355 | split |
| `src/api/admin.rs` | 2648 | split |
| `src/api/command_catalog.rs` | 2945 | split |
| `src/doctor/mod.rs` | 3030 | split |
| `src/api/mod.rs` | 5400 | split |
| `src/main.rs` | 14924 | split |
