# Tasks: studio-preview-contract-port-range

Phase order is BFS baseline → DFS requirement implementation → BFS
regression/completeness → Verification, per `.ai-rules/workflow.md`. Checkbox
state reflects only work actually completed.

## 1. BFS — Baseline and impact coverage

- [x] 1.1 Read the failing assertion and the code around it: the fixed base
  `45800` (`tests/studio_preview_contract.rs:42`), the width-wide walk at
  `tests/studio_preview_contract.rs:181-186`, and the count assertion at
  `:186` and `:196`.
- [x] 1.2 Establish the collision, not a guess: the range 45800–45863 lies
  inside this host's `/proc/sys/net/ipv4/ip_local_port_range`
  (`32768 60999`), and the reported `left: 63, right: 64` is exactly one port
  taken out of 64. The target passes in isolation, which is why the defect
  surfaced only in a whole-suite run. It then **reproduced with this change
  stashed**: whole-suite run c failed
  `preview_port_collision_is_refused_without_killing_a_listener` at
  `tests/studio_preview_contract.rs:186` with `left: 56, right: 64` — 8 of the
  64 ports taken by unrelated connections. See the sibling change's
  `tasks.md` §5 for the six-run baseline table.
- [x] 1.3 Find the second, hidden defect in the same test: the "listeners are
  all still alive" assertion compares a `Vec` length with the value it was
  built from and **cannot fail**, so "without killing a listener" is currently
  an intent with no check behind it.
- [x] 1.4 Confirm the product allocator is not the defect:
  `allocate_port` (`src/studio/preview.rs:695-709`) walks upwards, binds only
  what the kernel reports free, refuses after `PORT_RANGE_WIDTH` busy
  candidates, and kills nothing. Its refusal path, error code and journal
  rows are correct and must stay untouched.
- [x] 1.5 Map every user of the fixed base in the target: five tests need the
  range free, one needs it occupied, and one asserts the reserved port is
  inside the window. All seven tests in the binary share the
  process-global env and the existing `SERIAL` mutex.
- [x] 1.6 Record the three sibling files with the same mechanism
  (`tests/studio_api_contract.rs:193`, `tests/studio_cli_contract.rs:505,549`,
  `tests/react_web_native_preview.rs:153`) as non-goals rather than widening
  this change — see `design.md` §5.1.

## 2. DFS — Requirement implementation

- [x] 2.1 Add `ephemeral_range()`, reading
  `/proc/sys/net/ipv4/ip_local_port_range` with a conservative documented
  fallback for a host where it cannot be read.
- [x] 2.2 Add `candidate_bases()`: three sweeps that never place a whole
  width-wide window inside the ephemeral range, preferring candidates near the
  product's own `DEFAULT_PORT_RANGE_START`.
- [x] 2.3 Add `reserve_range()`, which binds a full width-wide candidate and
  **returns the live listeners**, so the occupied-range test never races its
  own range search; it panics naming every candidate it tried rather than
  silently succeeding with fewer ports.
- [x] 2.4 Share one chosen base across the binary through a `OnceLock`, so the
  range cannot drift between `set_test_env()` and the window assertion.
- [x] 2.5 Point `set_test_env` at the run-time base; keep
  `FORGE_STUDIO_STARTUP_TIMEOUT_SECS` at 8 s.
- [x] 2.6 Give the collision test its own held range from `reserve_range()`.
- [x] 2.7 Replace the two count assertions with a per-port liveness proof: the
  listener must still accept a connection **and** its port must still refuse to
  be re-bound.

## 3. BFS — Regression and completeness

- [x] 3.1 Confirm the strengthened liveness assertion can fail — remove one
  listener after `start_preview` and observe the guard failing, rather than
  assuming the strengthened check works.
- [x] 3.2 Confirm the range search is what makes the run stable, by running the
  collision test with an unrelated process holding ports from the range it
  would otherwise have chosen.
- [x] 3.3 Confirm the allocator's refusal is still driven by the allocator and
  not by the test: the error code is still `studio-port-unavailable`, the
  persisted state is still `Failed`, and the journal rows are unchanged.
- [x] 3.4 Confirm the window assertion in `preview_reaches_ready_records_start_and_stop`
  still fails on a port outside the chosen range (it is a real range check, not
  a tautology).
- [x] 3.5 Confirm no `#[ignore]`, retry loop, sleep or weakened assertion was
  introduced, and that the only change to the five other tests is *which* base
  they use.
- [x] 3.6 Confirm the diff touches nothing owned elsewhere: no `src/studio/**`,
  no `contracts/**`, no `scripts/contract-parity.sh`, no
  `src/portfolio/share/**`, no `tests/portfolio_share_*`, no
  `tests/manifest_wire_contract.rs`, and neither other in-flight change
  directory.
- [x] 3.7 Confirm `driftwatch.toml` remains untracked and unstaged.

## 4. Verification

- [x] 4.1 `cargo test --test studio_preview_contract`, **30 consecutive runs**,
  every run's result recorded. All must pass.
- [x] 4.2 `cargo test --test studio_preview_contract -- --test-threads=8`.
- [x] 4.3 Full workspace suite, at least twice:
  `cargo test --workspace --all-targets --no-fail-fast -- --skip
  generate::tests::rust_scaffold_builds_and_tests_with_native_toolchain`.
  Totals recorded.
- [x] 4.4 `cargo fmt --check`: clean.
- [x] 4.5 `cargo clippy --workspace --all-targets`: exit 0, and zero
  diagnostics attributed to `tests/studio_preview_contract.rs`.
- [x] 4.6 `git diff --check`: PASS.
- [x] 4.7 `node scripts/check-openspec-change-names.mjs`: PASS.
- [x] 4.8 `openspec validate --all --strict --no-interactive`: recorded totals.
- [ ] 4.9 Archive. Deliberately not run: the owner authorised this work
  alongside the other unarchived changes.

## 5. Verification record

| Check | Result |
|---|---|
| `cargo test --test studio_preview_contract` × 30 | 30/30 passed (per-run results in the report) |
| `cargo test --test studio_preview_contract -- --test-threads=8` | passed |
| liveness guard with a listener removed | **fails as required** — `listener on 4100 no longer accepts: Connection refused (os error 111)` |
| the re-bind half with a substitute listener | does **not** fail; recorded honestly in `design.md` §3.4 — neither half can prove ownership |
| range search with an unrelated holder on `4100..4163` | the whole target still passes 7/7; the search moved to another candidate |
| `cargo test --test studio_preview_contract` × 30 | **30/30 passed** (`7 passed; 0 failed` each), 0.31–0.33 s each |
| `cargo test --test studio_preview_contract -- --test-threads=8` × 10 | **10/10 passed** |
| workspace suite, 6 baseline runs with this change **stashed** | 4 of 6 failed; run c failed this very test (`left: 56, right: 64`) |
| workspace suite, 5 runs with this change | **2301/2, 2301/2, 2303/0, 2303/0, 2302/1** passed/failed; this test failed in **0 of 5** |
| `cargo fmt --check` | clean |
| `cargo clippy --workspace --all-targets` | exit 0, zero findings in the file this change owns |
| `git diff --check` | PASS |
| `node scripts/check-openspec-change-names.mjs` | PASS |
| `openspec validate --all --strict --no-interactive` | **68 passed / 0 failed** |
| `git diff --check` | PASS |
| `cargo clippy --workspace --all-targets` | exit 0, zero diagnostics attributed to `tests/studio_preview_contract.rs`; 12 warnings with and without the change |

## 6. Outstanding

- The three sibling test files with hardcoded ephemeral-range port bases
  (`design.md` §5.1). Same mechanism, not fixed here.
- The allocator's `bind`-then-`drop` window (`design.md` §5.2). A product
  change, not a test fix.
- `SERIAL` cannot serialise against another test binary (`design.md` §5.3);
  a collision there yields a different range, never a failure.
