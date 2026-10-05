# Design: studio-preview-contract-port-range

## 1. The defect, precisely

The test binary sets the Studio allocator's lower bound through a
process-global environment variable and then binds every port in the width-wide
window above it:

```rust
const TEST_PORT_BASE: u16 = 45800;              // window 45800..=45863

std::env::set_var(STUDIO_PORT_RANGE_START_ENV, TEST_PORT_BASE.to_string());
```

```rust
for offset in 0..PORT_RANGE_WIDTH {
    if let Ok(listener) = TcpListener::bind(("127.0.0.1", TEST_PORT_BASE + offset)) {
        listeners.push(listener);                // silently skips a taken port
    }
}
assert_eq!(listeners.len(), PORT_RANGE_WIDTH as usize);
```

Two properties of that code combine badly:

1. **The window is inside the ephemeral range.** `/proc/sys/net/ipv4/ip_local_port_range`
   is `32768 60999` here, and the kernel assigns outbound source ports from it
   without regard for who is already listening on that port inbound. The
   browser, the language server, the `cargo test` binary itself and every
   other process on the machine draw from it.
2. **A failed bind is skipped, not fatal.** That is what makes the failure
   surface as `left: 63, right: 64` on the *count* assertion, which reads
   like a test bug rather than an environment collision.

The test then asserts the allocator refuses — correct — and asserts the
listeners are alive, which it does not do:

```rust
assert_eq!(listeners.len(), PORT_RANGE_WIDTH as usize);
```

`listeners` is a local `Vec` that nothing between the two statements can
modify. The assertion is structurally incapable of failing.

## 2. Ownership

The test file. The product allocator is correct and is not touched: it walks
upwards, binds only what the kernel reports free, refuses after
`PORT_RANGE_WIDTH` busy candidates, and kills nothing. The defect is that its
*test* chose a window the kernel may hand out from under it.

## 3. The change

### 3.1 Read the host's ephemeral window

```rust
/// The window the kernel draws ephemeral source ports from.
fn ephemeral_range() -> Option<(u16, u16)>   // /proc/sys/net/ipv4/ip_local_port_range
```

On a host where it cannot be read the fallback is `(49152, u16::MAX)`, which
is the conservative reading: it is macOS's default and it keeps the candidates
below 49152, which is where the product's own default range lives.

### 3.2 Candidates outside that window

```rust
fn candidate_bases() -> Vec<u16>
```

Three sweeps, in order:

1. every 128 ports upward from `DEFAULT_PORT_RANGE_START` while the whole
   width-wide window stays strictly below the ephemeral low bound;
2. every 128 ports upward from the ephemeral high bound while the window still
   fits in the port space;
3. a last-resort sweep every 1024 ports from 1024 across anything that is not
   inside the ephemeral window, for a host whose ephemeral range swallows the
   usual space.

Sweep 1 is preferred so the test starts near the product's own default, which
keeps a printed port recognisable. Each sweep skips a candidate whose window
would touch the ephemeral range at all — not merely overlap it.

### 3.3 Occupy the chosen range and keep the listeners

```rust
fn reserve_range() -> (u16, Vec<TcpListener>)   // panics, naming every candidate tried
```

For each candidate the helper binds all 64 ports and, **if every bind
succeeded**, returns immediately with the listeners still held. If any bind
fails it drops what it bound and moves to the next candidate.

This is the detail that removes the race. A helper that probed for a free
range and then released the probes would leave a window between "free" and
"bind", and the test needing the range *busy* would still be taking a
guess. Here the ports the test occupies are the same sockets the search
verified, so between the search and the assertion there is no window at all.

This is not a retry loop papering over a flaky assertion. The search is
*setup*: it answers "where on this host is a width-wide window free?", which is
a question with no constant answer. The assertions below it are unchanged in
strength and are the thing that can fail.

### 3.4 "Without killing a listener" becomes a real proof

```rust
for (offset, _) in listeners.iter().enumerate() {
    let port = base + offset;
    TcpStream::connect(("127.0.0.1", port))            // a dead listener refuses
        .unwrap_or_else(|e| panic!("listener on {port} no longer accepts: {e}"));
    assert!(TcpListener::bind(("127.0.0.1", port)).is_err(),
            "port {port} was released: the allocator killed an unrelated listener");
}
```

Two observable facts per port:

- **something is still accepting** — a closed listening socket refuses
  connections, so a successful `connect` proves a listening socket is alive.
- **the port was not released** — a port some socket holds cannot be re-bound,
  so a successful re-bind would prove the port went back to the kernel.

Which half actually fires was measured rather than assumed. Removing one of the
test's own listeners **fails** the first assertion
(`listener on 4100 no longer accepts: Connection refused (os error 111)`). The
second cannot be made to fire by removing a listener, because in that case the
first one fires first; substituting a *different* listener on the same port
leaves both assertions satisfied. That is stated plainly rather than dressed
up: nothing observable from outside can tell our listener from another one
bound later, so the pair proves what is externally visible — a listener
survived and the port was not released — and not ownership. Ownership is not
something the allocator could lose anyway: `allocate_port` has no kill, no pid
and no descriptor to close.

`FORGE_STUDIO_STARTUP_TIMEOUT_SECS` is already 8 s for this target, and the
allocator refuses before any runner is spawned, so the loop is not the slow
part — 64 loopback connections to a backlog of `SOMAXCONN` complete
immediately.

### 3.5 One base for the whole binary

The five tests that need the range **free** share one base through a
`OnceLock`, so the binary does not print a different range per test and the
base cannot drift between `set_test_env()` and the assertion at
`tests/studio_preview_contract.rs:91`. The collision test uses its own
`reserve_range()` result, because it needs the listeners, not the number.

The `SERIAL` mutex the target already holds is why this is safe: the port range
and the startup-timeout env are process-global, so these tests were never
safe to run in parallel, and that is unchanged.

## 4. Behaviour before and after

| | Before | After |
|---|---|---|
| range chosen | constant `45800`, inside `32768 60999` | run-time, outside the host's ephemeral window |
| a port in the range already taken | skipped; the count assertion fails | search moves to the next candidate |
| collision test's listeners | real listeners, count asserted twice | real listeners, each proven alive **and** still owned |
| product allocator | unchanged | unchanged |
| journal rows, error code, persisted state | unchanged | unchanged |

## 5. Outstanding, deliberately not fixed here

1. **`tests/studio_api_contract.rs:193` (`47100`), `tests/studio_cli_contract.rs:505,549`
   (`47300`) and `tests/react_web_native_preview.rs:153` (`48200`)** hardcode
   bases inside this host's ephemeral range — the same mechanism as this fix,
   in different binaries, reached through a subprocess env instead of a
   process-global one. Fixing them means either a shared test support module or
   a repeated helper, and verifying them needs a Node toolchain and
   subprocess startup timing. Recorded rather than absorbed; the owner should
   schedule them.
2. **The allocator's `bind`/`drop` window.** `allocate_port`
   (`src/studio/preview.rs:695-709`) binds a candidate, drops the listener, and
   returns the number, so a runner that starts later can find it taken. Closing
   that window means handing the bound listener to the runner, which is a
   product change to the preview session, not a test fix.
3. **`SERIAL` is a mutex, not a real serialisation** against *other* test
   binaries; two binaries could still choose the same range. The preference
   for low candidates makes this unlikely, and a candidate whose ports are
   already bound is skipped, so the outcome is a different range, never a
   failure.

## 6. Verification strategy

| Check | Why it is honest |
|---|---|
| **30 consecutive `cargo test --test studio_preview_contract` runs**, every one recorded | the fix is "this assertion stops failing", so the only meaningful evidence is a long clean run |
| the same target at `--test-threads=8` | the range search and the shared base must not depend on thread timing |
| the collision test under a process holding ports from the chosen range | proves the search moves to another range instead of failing |
| the strengthened liveness loop with a listener removed after `start_preview` | **measured**: fails `listener on 4100 no longer accepts: Connection refused (os error 111)` |
| full workspace suite, twice | catches a range that collides with another test binary |
