# Design: studio-test-port-range

## 1. The defect, precisely

The same two properties, in three more binaries.

1. **The hardcoded window is inside the ephemeral range.**
   `/proc/sys/net/ipv4/ip_local_port_range` is `32768 60999` on this host.
   `47100`, `47300` and `48200` are all inside it, as `45800` was. The kernel
   assigns outbound *source* ports from that window for every connection by
   every process on the machine, and it does not skip a port because something
   is already listening on it inbound.
2. **The consequence is a failed assertion, not a skip.**
   `allocate_port` binds only what the kernel reports free and refuses after
   `PORT_RANGE_WIDTH` consecutive busy candidates, so a lost port either walks
   the assertion to the next candidate or refuses the whole range — an
   environment collision reported as a product or test failure.

How each site reached the range:

| Site | Base | How it is used |
|---|---|---|
| `tests/studio_api_contract.rs:193` | `47100` | `std::env::set_var` in-process, then `handle_buffered` drives the real `start_preview`; the assertion was the literal `assert!((47100..47164).contains(&port))` |
| `tests/studio_cli_contract.rs:505,549` | `47300` | passed to a **subprocess** through `run_env`, twice in one test (a first start and a second start with the same stub) |
| `tests/react_web_native_preview.rs:153` | `48200` | `std::env::set_var` in-process, then a real `ProcessRunner::react_web()` Vite dev server is started on the allocated port |

## 2. Ownership

The tests. The product allocator is correct and is not touched: it walks
upwards, binds only what the kernel reports free, refuses after
`PORT_RANGE_WIDTH` busy candidates, and kills nothing. The defect is that the
*tests* chose windows the kernel may hand out from under them.

## 3. The change

### 3.1 One shared module, four binaries

`tests/support/studio_ports.rs` is the search that
`studio-preview-contract-port-range` wrote, lifted out of that one target:

```
tests/support/studio_ports.rs
├── ephemeral_range()      /proc/sys/net/ipv4/ip_local_port_range, (49152, u16::MAX) fallback
├── candidate_bases()      three sweeps, every candidate wholly outside the window
├── reserve_range()        (u16, Vec<TcpListener>) — binds all 64 and KEEPS the listeners
└── shared_port_base()     OnceLock: one base per test binary
```

Included with the repository's existing convention:

```rust
#[path = "support/studio_ports.rs"]
mod studio_ports;
use studio_ports::shared_port_base;
```

**Why a shared module and not a fourth copy.** The search is ~100 lines whose
whole value is *where it is not*. Four copies is four places to forget the
ephemeral check, and the three unfixed copies are the evidence of exactly
that. `tests/support/` already exists for this purpose and is already used by
six targets this way.

`tests/studio_preview_contract.rs` keeps every behaviour and loses its
duplicates: same sweep order, same step, same panic that names every candidate
tried, same `reserve_range()` for the collision test that needs the listeners,
same `shared_port_base()` for the tests that need the window free. The only
edit inside it is the removal of the local copies plus the include.

### 3.2 The assertion follows the base in force

`studio_api_contract.rs` asserted a literal window, which is why a run-time
base needs its assertion recomputed:

```rust
let port_base = shared_port_base();
std::env::set_var("FORGE_STUDIO_PORT_RANGE_START", &port_base.to_string());
...
assert!(
    (u64::from(port_base)..u64::from(port_base) + u64::from(PORT_RANGE_WIDTH)).contains(&port),
    "{port} outside {port_base}"
);
```

Strength is unchanged: it still fails if the allocated port leaves the
configured window, and it now names the window that was configured.

`studio_cli_contract.rs` and `react_web_native_preview.rs` assert on the port
the session reports, so nothing there needed recomputing — only the base.

### 3.3 The listeners that verified the range stay held

Unchanged from the landed fix and the reason the search is honest: for a test
that needs its range *occupied*, `reserve_range()` hands back the very sockets
it verified, so nothing can change between choosing and occupying. For a test
that needs the range *free*, `shared_port_base()` drops them again so the
allocator can bind one. Neither is a retry over a flaky assertion — it answers
"where on this host is a 64-wide window free?", which has no constant answer.

### 3.4 One base per binary, and one base per *slot*

`shared_port_base(slot)` is memoised per slot, so `set_var` and the assertion
cannot disagree, and a binary does not print a different range per test.

The `slot` argument exists because of a defect this change introduced and then
fixed, found by its own evidence run rather than by review. With all four
targets sharing one preference list, all four preferred **4100**. `cargo test`
runs test binaries one at a time, but two `cargo test` invocations at once do
not — and the first full-suite run in this change overlapped a concurrent
`react_web_native_preview` run and failed:

```
studio_api_contract::api_owns_the_live_preview_between_start_and_stop
  studio-start-timeout: profile runner exited before binding the reserved port (port 4100)
studio_cli_contract::preview_start_probe_reaches_ready_and_leaves_no_live_process
  studio-start-timeout: profile runner exited before binding the reserved port (port 4100)
```

The mechanism is the allocator's own bind/drop window (§5.1): the test binds the
window to verify it is free, releases it, and the allocator then binds a single
port and drops it again. Between those steps any other process can take the
port — including another Forge test target's runner, which is a legitimate
holder, not a collision the allocator is allowed to clear.

So each target takes a different candidate:

| target | `start_index` | preferred base on this host |
|---|---|---|
| `studio_preview_contract` | 0 | 4100 |
| `studio_api_contract` | 1 | 4228 |
| `studio_cli_contract` | 2 | 4356 |
| `react_web_native_preview` | 3 | 4484 |

`start_index` **rotates** the preference list instead of filtering it, so a
target whose preferred window is taken still sees every other candidate.
Measured: `ss -ltn` while three Studio targets ran concurrently showed 4228 and
4356 in use at the same time, from two different targets.

## 4. Behaviour before and after

| | Before | After |
|---|---|---|
| base, this host | `45800` / `47100` / `47300` / `48200`, all inside `32768 60999` | run-time; first sweep starts at the product's own `DEFAULT_PORT_RANGE_START` (4100) and only while `base + 64 ≤ 32768` |
| an unrelated outbound connection takes a port | the test fails | no effect: no port of the window can be handed out |
| a candidate is partly taken | — | the search moves to the next candidate |
| every candidate is taken | — | the test panics naming every candidate it tried |
| product allocator | unchanged | unchanged |
| journal rows, error codes, persisted state | unchanged | unchanged |

## 5. Outstanding, deliberately not fixed here

1. **The allocator's own bind/drop window.** `allocate_port`
   (`src/studio/preview.rs:695-709`) binds a candidate, drops the listener and
   returns the number, so a runner that starts later can find it taken. §3.4
   shows this is a real, observable failure, not a theoretical one, whenever
   anything else on the host claims the port in between. Closing it means
   handing the bound listener to the runner — a product change to the preview
   session, not a test fix, and already recorded by
   `studio-preview-contract-port-range`.
2. **Distinct slots are a preference, not a reservation.** Two targets running
   at the same time now prefer different windows, but nothing prevents a third
   process from taking one. A reservation that survived until the runner bound
   the port would fix it properly, and that is §5.1 again.
3. **`/tmp` and the ephemeral window are host facts.** On a host whose
   ephemeral window starts at or below 1024 and ends above 60999, sweep 1 is
   empty and the last-resort sweep finds nothing; the panic names the window
   and every candidate tried, which is the honest outcome.

## 6. Verification strategy

| Check | Why it is honest |
|---|---|
| **30 consecutive runs of each of the four targets**, every run recorded | the fix is "this assertion stops failing", so a long clean run is the only meaningful evidence |
| the same targets at `--test-threads=8` and `--test-threads=1` | the shared base must not depend on thread timing |
| **the previously hardcoded ranges held by an unrelated process** while each target runs | proves the mechanism directly: with `47100`, `47300`, `48200` and `45800` occupied, the targets must still pass, because they no longer use them |
| **the same hold with the pre-fix bases restored** | the negative control: `studio_api_contract` then fails `no free port in range 47100..=47163` and `studio_cli_contract` fails `47300..=47363` |
| **three Studio targets run concurrently** | proves the slot fix (§3.4); with every slot set to 0 the same control failed within five rounds |
| the search with a candidate partly taken | the search moves on instead of failing |
| whole workspace suite × 6 | catches a base that collides with another target |
