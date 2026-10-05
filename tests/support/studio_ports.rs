//! Shared choice of a Studio test port range.
//!
//! Four targets drive the Studio allocator: `studio_preview_contract`,
//! `studio_api_contract`, `studio_cli_contract` and
//! `react_web_native_preview`. Each of them needs a width-wide window of
//! ports that is free, and each of them used to name its own base —
//! `45800`, `47100`, `47300`, `48200` — all of which lie **inside** this
//! host's ephemeral window (`/proc/sys/net/ipv4/ip_local_port_range` =
//! `32768 60999`).
//!
//! Two properties of a hardcoded base combine badly:
//!
//! 1. **The window is inside the ephemeral range.** The kernel assigns
//!    outbound *source* ports from there for every connection by every
//!    process on the machine, and it does not skip a port because
//!    something is already listening on it inbound. The browser, the
//!    language server, the `cargo test` binary itself and every other
//!    process draw from it.
//! 2. **A taken port is a failed assertion, not a skip.** The allocator
//!    binds only what the kernel reports free and refuses after
//!    [`PORT_RANGE_WIDTH`] consecutive busy candidates, so one lost port
//!    either walks the assertion to the next candidate or refuses the
//!    whole range — an environment collision reported as a test failure.
//!
//! The fix is to read the window and consider only candidates lying
//! wholly outside it, which is what these helpers do.
//!
//! Each target passes a distinct `start_index`, so the four of them prefer
//! different windows and two test binaries running at the same time do not
//! fight over the same ports:
//!
//! | target | `start_index` |
//! |---|---|
//! | `studio_preview_contract` | 0 |
//! | `studio_api_contract` | 1 |
//! | `studio_cli_contract` | 2 |
//! | `react_web_native_preview` | 3 |

#![allow(dead_code)]

use std::collections::HashMap;
use std::net::TcpListener;
use std::sync::{LazyLock, Mutex};

use forge::studio::{DEFAULT_PORT_RANGE_START, PORT_RANGE_WIDTH};

/// Distance between candidate range lower bounds. The window is
/// [`PORT_RANGE_WIDTH`] wide, so 128 leaves a gap between candidates and
/// never two candidates that overlap.
const CANDIDATE_STEP: u16 = 128;

/// The window this host draws ephemeral **outbound** source ports from
/// (`/proc/sys/net/ipv4/ip_local_port_range`).
///
/// When the file cannot be read, `49152` is used: it is macOS's default
/// lower bound and it keeps the candidates well below, where the product's
/// own default range lives.
pub fn ephemeral_range() -> (u16, u16) {
    std::fs::read_to_string("/proc/sys/net/ipv4/ip_local_port_range")
        .ok()
        .and_then(|text| {
            let mut parts = text.split_whitespace();
            Some((parts.next()?.parse().ok()?, parts.next()?.parse().ok()?))
        })
        .unwrap_or((49_152, u16::MAX))
}

/// Lower bounds to try for a width-wide range, in order, beginning at
/// candidate `start_index`.
///
/// Every candidate is kept **wholly** outside the ephemeral window — not
/// merely non-overlapping — so no outbound connection on this host can take
/// any port in it. The first sweep starts near the product's own default
/// range so a printed port stays recognisable.
///
/// `start_index` rotates that preference list rather than filtering it, so a
/// caller that starts at index 3 still sees every earlier candidate if those
/// are taken. It exists because the four Studio test targets would otherwise
/// all prefer the *same* window: `cargo test` runs test binaries one at a
/// time, but two `cargo test` invocations at once, or a runner that executes
/// test binaries in parallel, do not. Measured on this change: with all four
/// preferring `4100`, a concurrent run failed `studio_api_contract` and
/// `studio_cli_contract` with `studio-start-timeout: profile runner exited
/// before binding the reserved port (port 4100)`, because the other target's
/// runner held that port.
pub fn candidate_bases(start_index: u16) -> Vec<u16> {
    let width = u32::from(PORT_RANGE_WIDTH);
    let first = u32::from(DEFAULT_PORT_RANGE_START).max(1024);
    let last = u32::from(u16::MAX) - width + 1;
    let (low, high) = ephemeral_range();
    let (low, high) = (u32::from(low), u32::from(high));
    let mut bases = Vec::new();

    // Below the ephemeral window, starting at the product's default range.
    let mut base = first;
    while base + width <= low {
        bases.push(base as u16);
        base += u32::from(CANDIDATE_STEP);
    }
    // Above the ephemeral window.
    let mut base = high + 1;
    while base <= last {
        bases.push(base as u16);
        base += u32::from(CANDIDATE_STEP);
    }
    // Last resort for a host whose ephemeral window swallows the space
    // above: anything at all that is not inside the window.
    let mut base = 1024;
    while base <= last {
        if base + width <= low || base > high {
            bases.push(base as u16);
        }
        base += 1024;
    }
    let len = bases.len();
    if len > 0 {
        bases.rotate_left(usize::from(start_index) % len);
    }
    bases
}

/// Occupy a width-wide range that lies outside the ephemeral window, and
/// **keep the listeners**.
///
/// The listeners are the point. A helper that probed for a free range and
/// then released its probes would leave a window between "free" and "bind",
/// and the test that needs the range *busy* would still be taking a guess.
/// Here the ports the caller occupies are the same sockets the search
/// verified, so nothing can change between the search and the assertions.
///
/// This is setup, not a retry over a flaky assertion: it answers "where on
/// this host is a width-wide window free?", a question that has no constant
/// answer. The assertions built on top of it are unchanged in strength.
pub fn reserve_range(start_index: u16) -> (u16, Vec<TcpListener>) {
    let mut tried = Vec::new();
    for base in candidate_bases(start_index) {
        let mut listeners = Vec::new();
        let mut complete = true;
        for offset in 0..PORT_RANGE_WIDTH {
            match TcpListener::bind(("127.0.0.1", base + offset)) {
                Ok(listener) => listeners.push(listener),
                Err(_) => {
                    complete = false;
                    break;
                }
            }
        }
        if complete {
            return (base, listeners);
        }
        tried.push(base);
    }
    panic!(
        "no {PORT_RANGE_WIDTH}-wide port range could be bound outside the ephemeral window {:?}; \
         tried lower bounds {tried:?}",
        ephemeral_range()
    );
}

/// The one range this test binary uses when a test needs the window
/// **free**. Chosen once per slot, so the base cannot drift between the
/// `FORGE_STUDIO_PORT_RANGE_START` a test sets and the assertion that checks
/// the allocated port falls inside it.
///
/// Each target passes its own `start_index`, so two test binaries running at
/// the same time prefer different windows.
pub fn shared_port_base(start_index: u16) -> u16 {
    static BASES: LazyLock<Mutex<HashMap<u16, u16>>> = LazyLock::new(|| Mutex::new(HashMap::new()));
    let mut bases = BASES
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    *bases
        .entry(start_index)
        .or_insert_with(|| reserve_range(start_index).0)
}
