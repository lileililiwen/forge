# Proposal: the Studio preview contract picks its test port range inside the OS ephemeral window

## Why

`tests/studio_preview_contract.rs::preview_port_collision_is_refused_without_killing_a_listener`
is the repository's remaining known flaky test. The reported failure is:

```
studio_preview_contract::preview_port_collision_is_refused_without_killing_a_listener
left: 63, right: 64
```

`tests/studio_preview_contract.rs:42` hardcodes

```rust
const TEST_PORT_BASE: u16 = 45800;
```

and `set_test_env()` points the Studio allocator's lower bound at it. The
allocator walks 64 candidates upwards (`src/studio/preview.rs:695-709`), so the
test's own range is **45800–45863**.

On this host that range sits inside the kernel's ephemeral port window:

```
$ cat /proc/sys/net/ipv4/ip_local_port_range
32768	60999
```

The kernel assigns ephemeral source ports from that window to **every outbound
connection by every process on the machine**. The kernel does not skip ports
that some other process is already listening on for inbound connections, so a
single unrelated outbound connection can take one of the test's 64 ports
between the moment the test chooses them and the moment it binds them. The
test then binds 63 of 64 and this assertion trips:

```rust
assert_eq!(listeners.len(), PORT_RANGE_WIDTH as usize);
```

The mechanism is **proved, not inferred**: with another process holding
ports from the range, the same assertion fails identically (`left: 63,
right: 64` with one port held, `left: 0, right: 64` with all of them). In
isolation the test always passes, which is exactly why it survived in the
suite and failed in a whole-suite run.

### The assertion that is not proving what it claims

The second half of the test — the part that carries the intent in its name,
*"without killing a listener"* — is:

```rust
// The listeners the test owns are all still alive: the allocator
// never killed an unrelated process.
assert_eq!(listeners.len(), PORT_RANGE_WIDTH as usize);
```

`listeners.len()` is a `Vec` length. Nothing can change it between the two
lines, so **this assertion cannot fail**. The test does hold real listeners,
so the scenario is real, but nothing verifies they survived. A future
regression that made the allocator kill a process holding the range would pass
this test.

## What Changes

- **The test chooses its range at run time, outside the host's ephemeral
  window.** On Linux the window is read from
  `/proc/sys/net/ipv4/ip_local_port_range`; elsewhere a conservative default
  is used. Candidates are tried in order and the first one that actually binds
  a full width-wide block is used, so a port already taken by an unrelated
  process is handled by *choosing another range*, never by retrying the test.
- **The occupied range is held, not re-bound.** The candidate search keeps the
  listeners it successfully bound and hands them to the test, so the test that
  needs the range busy never races itself between "find free range" and
  "occupy range".
- **"Without killing a listener" becomes an assertion that can fail.** After
  the refusal, every listener must still accept a connection and every one of
  its ports must still refuse to be re-bound. That is a real liveness proof:
  a dead socket accepts nothing and a released port re-binds.
- Nothing about the product changes. `allocate_port`, `PORT_RANGE_WIDTH`,
  `DEFAULT_PORT_RANGE_START`, `studio-port-unavailable` and the
  `studio.preview.start` / `studio.preview.stop` journal rows are untouched;
  the allocator's real invariant — walk upwards, bind only what the kernel
  reports free, never kill an unrelated process — is exactly what the
  strengthened guard now pins.

No `#[ignore]`, no retry loop, no sleep, no serialisation beyond the mutex the
target already uses for its process-global env, and no assertion weakened.

## BFS Impact Map

| Surface | State today | Action |
|---|---|---|
| `tests/studio_preview_contract.rs:42` `TEST_PORT_BASE` | fixed `45800`, inside `32768 60999` | replaced by a run-time chosen base outside the ephemeral window |
| `tests/studio_preview_contract.rs:44-47` `set_test_env` | points the allocator at that fixed base | takes the chosen base |
| `tests/studio_preview_contract.rs:171-205` the collision test | binds 64 fixed ports, asserts the count twice | holds listeners obtained from the range search and proves each is still alive |
| `tests/studio_preview_contract.rs:91` range assertion | asserts the reserved port is inside the fixed base's window | asserts it is inside the chosen base's window |
| other five tests in the target | use the same fixed base, but need the range **free** | use a shared, run-time chosen base |
| `src/studio/preview.rs:682-709` `port_range_start` / `allocate_port` | walks 64 candidates upwards, binds only free ones, refuses after 64, never kills | **unchanged** |
| `src/studio/mod.rs:67-74` env name, width, default | `FORGE_STUDIO_PORT_RANGE_START`, 64, 4100 | **unchanged** |
| `tests/studio_api_contract.rs:193`, `tests/studio_cli_contract.rs:505,549`, `tests/react_web_native_preview.rs:153` | hardcode `47100` / `47300` / `48200`, all inside `32768 60999` | **not touched** — same mechanism, different files; recorded in `design.md` §5 |
| `governance-adapter-bounded-process-run` | the other change in flight | **not touched** |
| `contracts/**`, `scripts/contract-parity.sh`, `src/portfolio/share/**`, `tests/portfolio_share_*`, `tests/manifest_wire_contract.rs` | owned elsewhere | **not touched** |

### Why the range is chosen at run time rather than moved to another constant

Hardcoding a different constant only moves the failure. The whole point is
that any port number can be taken by an unrelated process, and on this host
the ephemeral window covers 32768–60999 — over half the port space. A
constant outside it (4100–4163, say) would be *far* less exposed but would
still be a constant that silently becomes a flake the day something binds it.
Choosing at run time removes the class, not the instance.

### Why the product allocator is not changed

The allocator is correct: it walks upwards, binds only what the kernel reports
free, refuses after `PORT_RANGE_WIDTH` busy candidates, and never kills
anything. The defect is that its *test* picked a range the kernel is allowed
to hand out from under it. Changing `allocate_port` to, say, ask the kernel
for an ephemeral port would make the allocator *more* exposed to the very
window it is being protected from.

## Capabilities

- `site-studio-preview-refinement` — the port allocator's refusal never
  terminates a process it did not start, pinned by an assertion that can fail.
- `runtime-hardening-and-test-isolation` — tests that exercise a
  process-global port-range setting choose a run-time range that no unrelated
  outbound connection can take.

## Non-goals

- **Changing the allocator's algorithm, width or default range.**
- **The three sibling test files** that hardcode ephemeral-range bases. Same
  mechanism, different files, and verifying them means a Node toolchain and
  subprocess timing that this change does not otherwise need. Recorded in
  `design.md` §5 as remaining work for the owner to schedule.
- **Replacing `127.0.0.1` with a Unix socket** in the fake runner.
- **Archiving any in-flight change.**
- Any retry, sleep, `#[ignore]` or weakened assertion.
