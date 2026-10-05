# Change: studio-test-port-range

## Why

`studio-preview-contract-port-range` proved the mechanism for
`tests/studio_preview_contract.rs`: that target hardcoded base `45800`, which
lies **inside** this host's ephemeral window
(`/proc/sys/net/ipv4/ip_local_port_range` = `32768 60999`), so one unrelated
outbound connection took a port and the test failed `left: 56, right: 64`. It
fixed that one target and recorded the other three in its `design.md` §5.1:

> `tests/studio_api_contract.rs:193` (`47100`), `tests/studio_cli_contract.rs:505,549`
> (`47300`) and `tests/react_web_native_preview.rs:153` (`48200`) hardcode
> bases inside this host's ephemeral range — the same mechanism as this fix, in
> different binaries, reached through a subprocess env instead of a
> process-global one. Fixing them means either a shared test support module or
> a repeated helper.

Owner instruction for this round: apply it to the three siblings, and do it
through **one** shared helper rather than a fourth copy of the search.

## What Changes

1. **A shared test support module**, `tests/support/studio_ports.rs`, holding
   the search that already exists in `studio_preview_contract.rs`:
   `ephemeral_range()`, `candidate_bases()`, `reserve_range()`,
   `shared_port_base()`. It follows the existing `#[path = "support/…"] mod …`
   convention already used by the portfolio suites.

2. **`tests/studio_preview_contract.rs` drops its local copies** (≈100 lines)
   and includes the module. Its behaviour is unchanged — same candidates, same
   order, same "keep the listeners" rule — so the landed fix and its two
   measured guards are not disturbed.

3. **`tests/studio_api_contract.rs`** — base chosen at run time; the assertion
   reflects the base actually chosen instead of the literal `47100..47164`.

4. **`tests/studio_cli_contract.rs`** — both probes use the chosen base,
   passed through the subprocess environment as before.

5. **`tests/react_web_native_preview.rs`** — the chosen base replaces `48200`.

6. **A distinct candidate per target**, added after this change's own evidence
   run failed. With one shared preference list all four targets preferred
   `4100`; two of them then failed `studio-start-timeout … (port 4100)` when a
   third target was running at the same time. Each target now starts the search
   at its own index (`0`, `1`, `2`, `3`), so it prefers `4100`, `4228`, `4356`,
   `4484`. The list is rotated, not filtered, so every candidate stays
   reachable. `design.md` §3.4.

## BFS Impact Map

| Surface | Finding |
|---|---|
| Requirement | a test that exercises a process-global port-range setting must not hardcode the range, and four targets must share one selection |
| Callers | `studio_preview_contract` (7 tests), `studio_api_contract` (1 test), `studio_cli_contract` (2 probes in 1 test), `react_web_native_preview` (1 test) |
| Product code | **untouched.** `allocate_port` / `port_range_start` (`src/studio/preview.rs`) already walk upwards and never kill an unrelated process |
| Env vars | unchanged names: `FORGE_STUDIO_PORT_RANGE_START`, `FORGE_STUDIO_STARTUP_TIMEOUT_SECS`. Only the *value* becomes run-time |
| Assertions | one literal range assertion replaced by one computed from the base in force. Strength unchanged: it still fails if the allocated port leaves the configured window |
| Contracts | none. No wire, CLI, registry or persisted shape is involved |
| Concurrency | the range env var is process-global, so it was already unsafe for these targets to run in parallel; one base per slot per binary makes the value identical rather than racy. Distinct slots also stop two binaries running at the same time from choosing the same window |
| Persistence / migrations | none |
| Tests | 4 targets; the change is entirely inside test code and test support |
| Compatibility | unchanged for any host whose ephemeral window does not contain the previously hardcoded bases; on this host the chosen base is below `32768` |
| Toolchain | `react_web_native_preview` is opt-in (`FORGE_NATIVE_REACT_WEB_PREVIEW=1`) and needs `npm`; the other three need no Node toolchain |

## Capabilities

- `runtime-hardening-and-test-isolation` — one ADDED requirement covering the
  shared, run-time window across all four targets.

## Non-goals

1. **The allocator's own bind/drop window.**
   `allocate_port` (`src/studio/preview.rs:695-709`) binds a candidate, drops
   the listener and returns the number, so a runner that starts later can find
   it taken. Closing that window means handing the bound listener to the
   runner — a product change, not a test fix. Already recorded by
   `studio-preview-contract-port-range`.
2. **Serialising the four binaries against each other.** Distinct slots make
   the four prefer different windows, but they are a preference, not a
   reservation: nothing stops another process taking a port after the window is
   released and before the runner binds it. That gap is the allocator's
   bind/drop window, below.
3. **Changing the product's default range** or any test's semantics beyond the
   base selection.
4. No test is ignored, serialised, slept, retried or given a weaker assertion.
