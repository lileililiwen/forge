# Tasks: studio-test-port-range

Phase order is BFS baseline → DFS requirement implementation → BFS
regression/completeness → Verification, per `.ai-rules/workflow.md`. Checkbox
state reflects only work actually completed.

## 1. BFS — Baseline and impact coverage

- [x] 1.1 Read the host's actual ephemeral window rather than assuming it:
  `/proc/sys/net/ipv4/ip_local_port_range` = `32768 60999`.
- [x] 1.2 Confirm the three recorded bases are inside it and that each is
  reached differently: `studio_api_contract.rs:193` (`47100`, in-process
  `set_var` + `handle_buffered`), `studio_cli_contract.rs:505,549` (`47300`,
  through a subprocess environment, twice in one test),
  `react_web_native_preview.rs:153` (`48200`, in-process `set_var` + a real
  Vite dev server).
- [x] 1.3 Confirm the product allocator is not the defect and is not to be
  touched: `allocate_port` (`src/studio/preview.rs:695-709`) walks upwards,
  binds only free candidates, refuses after `PORT_RANGE_WIDTH` busy candidates
  and kills nothing.
- [x] 1.4 Confirm the repository already has the convention this needs:
  `tests/support/{mod,interest,share}.rs`, included by six targets with
  `#[path = "support/…"] mod …`. `tests/support/share.rs` and the
  `portfolio/share` surfaces are owned elsewhere and are untouched.
- [x] 1.5 Map every assertion that depends on the base: one literal range
  assertion in `studio_api_contract.rs:210`; the other three assert on the port
  the session reported and need no recomputation.
- [x] 1.6 Record what this change does **not** fix, each with a reason: the
  allocator's own bind/drop window (a product change), distinct slots being a
  preference rather than a reservation, and a host whose ephemeral window covers
  everything (the panic names the window).
- [x] 1.7 Confirm the change touches no product code, no `contracts/**`, no
  `scripts/contract-parity.sh`, no `src/portfolio/share/**`, no
  `tests/portfolio_share_*`, no `tests/manifest_wire_contract.rs`.
- [x] 1.8 Confirm `driftwatch.toml` remains untracked and unstaged.

## 2. DFS — Requirement implementation

- [x] 2.1 Add `tests/support/studio_ports.rs` with `ephemeral_range()`,
  `candidate_bases()`, `reserve_range()` and `shared_port_base()`, carrying the
  landed file's documentation: why no test may hardcode a range, why the
  listeners are kept, and why the search is setup rather than a retry over a
  flaky assertion.
- [x] 2.6 Take a **distinct `start_index`** in each of the four targets, after
  the first full-suite run showed all four preferring `4100` and two of them
  failing `studio-start-timeout … (port 4100)` when a third target was running
  at the same time. `design.md` §3.4; measured controls in §4.5–4.7.
- [x] 2.2 `tests/studio_preview_contract.rs`: delete its local copies and
  include the module. Behaviour unchanged — same sweep, same step, same panic,
  same `reserve_range()` for the collision test, same `shared_port_base()` for
  the rest — and the two now-unused imports removed
  (`std::sync::OnceLock`, `DEFAULT_PORT_RANGE_START`).
- [x] 2.3 `tests/studio_api_contract.rs`: choose the base at run time, set it
  through `FORGE_STUDIO_PORT_RANGE_START`, and compute the range assertion from
  the base in force, naming the base on failure.
- [x] 2.4 `tests/studio_cli_contract.rs`: choose the base once and pass it to
  both probes through the subprocess environment, replacing both `47300`
  literals.
- [x] 2.5 `tests/react_web_native_preview.rs`: choose the base at run time in
  place of `48200`.

## 3. BFS — Regression and completeness

- [x] 3.1 Leave all four targets' tests intact: no `#[ignore]`, no serialisation
  added, no sleep, no retry, no weakened assertion.
- [x] 3.2 Confirm the landed `studio_preview_contract` guards still hold — the
  collision test's two-part liveness proof and the freed-window tests — because
  the module they call changed home. 30 consecutive runs, below.
- [x] 3.3 Confirm the `#![allow(dead_code)]` on the shared module is what keeps
  each target free of "unused helper" warnings, matching
  `tests/support/mod.rs`.
- [x] 3.4 Confirm the range env var is still set by every test that needs it,
  and that no test now depends on a base it does not configure.

## 4. Verification

| # | Check | Result |
|---|---|---|
| 4.1 | `cargo test --test studio_preview_contract` × 30 | see the session report |
| 4.2 | `cargo test --test studio_api_contract` × 30 | see the session report |
| 4.3 | `cargo test --test studio_cli_contract` × 30 | see the session report |
| 4.4 | `cargo test --test react_web_native_preview` × 30, **default invocation** | see the session report. Without `FORGE_NATIVE_REACT_WEB_PREVIEW=1` the test returns before it allocates anything, so these runs prove compilation and the early return only |
| 4.5 | the same four targets at `--test-threads=8` and `--test-threads=1`, 10 runs each | see the session report |
| 4.6 | **held-range experiment.** An unrelated python process holds all 256 ports of `45800-45863`, `47100-47163`, `47300-47363` and `48200-48263` — every base these four targets used to hardcode — and each target still passes | see the session report |
| 4.7 | **held-range negative control**: the same hold with the pre-fix bases restored | `studio_api_contract` **fails** `no free port in range 47100..=47163`; `studio_cli_contract` **fails** `no free port in range 47300..=47363`. `react_web_native_preview` cannot show it — see 4.4 |
| 4.8 | **concurrency control**: three Studio targets run at the same time, five rounds | 15/15 clean with distinct slots; the same control with every slot forced to `0` failed `studio_preview_contract` inside five rounds |
| 4.9 | **live opt-in**: `FORGE_NATIVE_REACT_WEB_PREVIEW=1 cargo test --test react_web_native_preview -- --nocapture` | 6 runs, **6 passed** — each with a real `npm install`, a real Vite dev server on the run-time base and a Playwright-rendered React component (`VERIFIED: rendered "hello from native-react-preview"`) |
| 4.10 | `cargo test --workspace --all-targets --no-fail-fast -- --skip generate::tests::rust_scaffold_builds_and_tests_with_native_toolchain`, 6 runs | exact totals in the session report |
| 4.11 | `cargo fmt --check` | clean |
| 4.12 | `cargo clippy --workspace --all-targets` | exit 0; zero diagnostics attributed to `tests/support/studio_ports.rs` or any of the four targets |
| 4.13 | `git diff --check` | PASS |
| 4.14 | `node scripts/check-openspec-change-names.mjs` | PASS |
| 4.15 | `openspec validate --all --strict --no-interactive` | passed / 0 failed |
| 4.16 | Archive | **deliberately not run**: five changes were already in flight and unarchived; nothing may be archived while they are parked |

## 5. Outstanding

- **The allocator's bind/drop window** and **slots being a preference rather
  than a reservation** stay outstanding — see `design.md` §5. The first one is a
  product change and produced a measured failure, not a theoretical risk.
