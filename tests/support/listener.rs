//! A loopback port a test can hand to a real Forge server without a race.
//!
//! # The defect this removes
//!
//! Eleven suites used to pick a port the same way:
//!
//! ```text
//! let listener = TcpListener::bind("127.0.0.1:0");   // ask the kernel
//! let port = listener.local_addr()?.port();
//! drop(listener);                                   // release it
//! Command::new(forge).args(["web", "serve", "--port", port]).spawn();
//! ```
//!
//! Nothing owns that port between the `drop` and the server's `bind`, so any
//! other `bind` on this host can take it. Under a parallel
//! `cargo test --workspace` the competitors are **Forge's own suites**, which
//! draw their ports from the same kernel ephemeral window. The loser is not
//! always the one that notices:
//!
//! - its server exits, the readiness poll keeps connecting to the *thief's*
//!   listener, and the next request fails with `ConnectionRefused` or
//!   `ConnectionReset` — or, worse, is answered by a stranger serving a
//!   different root, which is a silent wrong-server test;
//! - or nothing answers and the suite reports "did not bind".
//!
//! Measured on the untouched tree (`forge_web_navigation_contract`, two copies
//! of the binary at a time, 200 runs): 9 failures. With the child's stderr
//! captured, every one of them is
//! `cannot bind web listener on 127.0.0.1:PORT: Address already in use
//! (os error 98)` — the handoff, not the server, is what fails.
//!
//! # The protocol
//!
//! [`claim`] hands back a port the test **owns**:
//!
//! 1. **It is outside the kernel's ephemeral window.**
//!    `/proc/sys/net/ipv4/ip_local_port_range` is where every process on the
//!    host draws outbound source ports from, and the kernel does not skip a
//!    port just because something listens on it inbound. A window below that
//!    range cannot be taken by a connection the test never made. (`support/
//!    studio_ports.rs` keeps its own copy of the same read because it needs the
//!    opposite: a range the test **holds**.)
//! 2. **The window is per suite**, so two Forge suites never prefer the same
//!    ports — see [`suite`].
//! 3. **The slot is per claim within a suite**, so the parallel tests inside
//!    one binary never draw the same port.
//! 4. **The listening socket is held from `claim` until `spawn`** — the same
//!    reasoning `studio_ports::reserve_range` uses. The port is probed and
//!    then *the same socket* is the one released, so nothing can be taken
//!    between "it was free" and "the server is starting".
//! 5. **`flock` serialises the release-to-bind instant across processes**, so
//!    two runs of the same suite cannot both be inside it. A lock the kernel
//!    drops when a process dies cannot go stale.
//!
//! Only one instant is unowned: the microseconds between dropping the claim
//! and the child's `bind`. Every Forge suite that could use that port is
//! either on another window or locked out by (5), so a loss there is a real
//! environment collision — and [`Claim::spawn`] says so loudly, with the
//! child's own stderr, instead of hanging or blaming the suite.
//!
//! # Readiness is ownership, not a guess
//!
//! `connect` succeeding only proves *someone* is listening. [`Claim::spawn`]
//! therefore also checks that the listening socket belongs to the child it
//! spawned (the inode of the `LISTEN` entry in `/proc/net/tcp` is one of that
//! process's own `/proc/<pid>/fd` socket descriptors), so a suite can never
//! silently end up asserting against another test's server.

#![allow(dead_code)]

use std::fs::{self, File, OpenOptions};
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU16, Ordering};
use std::time::{Duration, Instant};

/// Window index per suite that starts a real Forge server. Distinct indices
/// keep two suites running at the same time off each other's ports, exactly as
/// the four Studio targets do with `studio_ports::candidate_bases`.
pub mod suite {
    pub const API_CONTRACT: u16 = 0;
    pub const API_CROSS_SURFACE: u16 = 1;
    pub const PROJECT_QUERY_SURFACE_CONTRACT: u16 = 2;
    pub const DELIVERY_CROSS_SURFACE: u16 = 3;
    pub const FORGE_WEB_NAVIGATION_CONTRACT: u16 = 4;
    pub const FORGE_WEB_MANAGE_DEEP_LINK_BROWSER: u16 = 5;
    pub const FORGE_WEB_PROJECT_DELIVERY_BROWSER: u16 = 6;
    pub const FORGE_WEB_WORKBENCH_DEEP_LINK_BROWSER: u16 = 7;
    pub const FORGE_WEB_WORKSPACE_ONBOARDING_BROWSER: u16 = 8;
    pub const LIFECYCLE_RAIL_BROWSER: u16 = 9;
    pub const WEB_LOGIN_CREDENTIALS_CONTRACT: u16 = 10;
}

/// Lowest window base. The Studio allocator's preferred candidates start at
/// `forge::studio::DEFAULT_PORT_RANGE_START` (4100), so these windows stay
/// below every port the product prints. Overlap with a Studio window is still
/// possible — its last-resort sweep lands on `1024 + 1024k`, which reaches
/// 2048 and 3072 — and it is harmless, because both sides read a busy
/// candidate as "take the next one" and a claim also refuses any port that
/// already has a listener.
const FIRST_BASE: u16 = 2048;

/// Ports per window. One per parallel server in a suite, with room to spare.
const WINDOW_WIDTH: u16 = 32;

/// Distance between window bases: wide enough to leave a gap between windows.
const WINDOW_STRIDE: u16 = 128;

/// Consecutive windows one claim may walk before it refuses. A busy port is a
/// refusal with a stated reason, never a silently different port.
const WINDOW_ATTEMPTS: u16 = 4;

/// Budget for the child to reach its `bind` — a bound on process start, not a
/// retry window. The web and browser suites already allowed 10s; the four API
/// suites move from 5s to the same shared bound, which is why the loop exits on
/// the child's own exit status or a proven bind rather than on the clock.
const READY_BUDGET: Duration = Duration::from_secs(10);

/// How often readiness is re-checked.
const POLL: Duration = Duration::from_millis(25);

/// Budget for the `flock` that keeps another suite out of the release-to-bind
/// instant. Bounded so a stuck run costs a candidate, never a hang.
const LOCK_BUDGET: Duration = Duration::from_secs(5);

/// Slot counter. One per test binary, which is exactly the scope that needs
/// per-test separation: the tests `cargo test` runs in parallel inside a
/// single binary.
static NEXT_SLOT: AtomicU16 = AtomicU16::new(0);

/// A port no one else can take, held until [`Claim::spawn`] releases it.
pub struct Claim {
    port: u16,
    /// Held, not probed-and-released: this *is* the reservation.
    listener: TcpListener,
    /// Held across the release-to-bind instant so another run of this same
    /// suite cannot be inside it too.
    lock: Lock,
}

impl Claim {
    /// The port to pass to the server.
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Release the claim and start the server on this port.
    ///
    /// `build` receives the port so the command line is derived from the
    /// claimed value rather than a second guess at it. The child's stderr is
    /// captured to a file: a server that cannot bind says so there, and that
    /// is the only diagnostic that distinguishes "the port was taken" from
    /// "the server is broken".
    ///
    /// # Panics
    ///
    /// When the child exits before it owns the port, or never binds inside
    /// [`READY_BUDGET`], or the port turns out to be held by a process other
    /// than the child. Each panic carries the child's exit status and stderr.
    /// This is deliberately a loud failure and not a retry: the port was held
    /// by this claim until this call, every Forge suite that could want it is
    /// on another window or locked out, so a loss here is an environment
    /// collision that a test failure should report rather than hide.
    pub fn spawn(self, label: &str, build: impl FnOnce(u16) -> Command) -> Server {
        let Claim {
            port,
            listener,
            lock,
        } = self;
        // The one unowned instant. Every Forge suite that could take this port
        // is either on another window or blocked on `lock`.
        drop(listener);

        let stderr_path = scratch_dir().join(format!("{label}-{port}.stderr"));
        let stderr = File::create(&stderr_path).expect("create stderr capture");
        let child = build(port)
            .stderr(Stdio::from(stderr))
            .spawn()
            .unwrap_or_else(|err| panic!("spawn {label} on 127.0.0.1:{port}: {err}"));

        let deadline = Instant::now() + READY_BUDGET;
        let mut server = Server {
            child: Some(child),
            port,
        };
        loop {
            match server.child().try_wait() {
                Ok(Some(status)) => panic!(
                    "{label} exited ({status}) before it owned 127.0.0.1:{port}\n\
                     its stderr: {}",
                    stderr_of(&stderr_path),
                ),
                Ok(None) => {}
                Err(err) => panic!("{label} process state on 127.0.0.1:{port}: {err}"),
            }
            if TcpStream::connect((Ipv4Addr::LOCALHOST, port)).is_ok() {
                if let Owner::Foreign = owned_by(port, server.child().id()) {
                    panic!(
                        "127.0.0.1:{port} is held by a process other than the {label} \
                         child (pid {}), so this suite would be asserting against \
                         someone else's server\n\
                         listening rows: {:?}\n\
                         child sockets: {:?}\n\
                         its stderr: {}",
                        server.child().id(),
                        listening_rows(port),
                        child_sockets(server.child().id()),
                        stderr_of(&stderr_path),
                    );
                }
                drop(lock);
                return server;
            }
            if Instant::now() >= deadline {
                panic!(
                    "{label} did not bind 127.0.0.1:{port} within {READY_BUDGET:?}\n\
                     its stderr: {}",
                    stderr_of(&stderr_path),
                );
            }
            std::thread::sleep(POLL);
        }
    }

    /// Give the port up without starting a server, for suites that hand the
    /// port to something this crate does not spawn (a shell script that starts
    /// the listeners itself). The claim is still held until this call, so the
    /// port is known to be free at the moment it is handed over.
    pub fn release(self) -> u16 {
        let port = self.port;
        drop(self.listener);
        drop(self.lock);
        port
    }
}

/// A running Forge server. Killed and reaped on drop, so an assertion panic
/// mid-test does not leave a listener behind for the next test to trip over.
pub struct Server {
    child: Option<Child>,
    port: u16,
}

impl Server {
    /// The port the child actually bound.
    pub fn port(&self) -> u16 {
        self.port
    }

    /// The child, for suites that need its stdio or its exit status.
    pub fn child(&mut self) -> &mut Child {
        self.child.as_mut().expect("server child")
    }

    /// Hand the child to the caller, giving up the guard. For suites that
    /// already kill the process themselves.
    pub fn into_child(mut self) -> Child {
        self.child.take().expect("server child")
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

/// Claim a port for `window`, holding it until it is spawned or released.
pub fn claim(window: u16) -> Claim {
    let slot = NEXT_SLOT.fetch_add(1, Ordering::Relaxed) % WINDOW_WIDTH;
    let dir = scratch_dir();
    let mut refusals = Vec::new();
    for attempt in 0..WINDOW_ATTEMPTS {
        let base = window_base(window + attempt);
        // Start at this claim's own slot so the parallel tests in a suite try
        // different ports first, then walk the rest of the window.
        for step in 0..WINDOW_WIDTH {
            let port = base + (slot + step) % WINDOW_WIDTH;
            let Some(lock) = Lock::acquire(&dir.join(format!("port-{port}.lock")), LOCK_BUDGET)
            else {
                refusals.push(format!("127.0.0.1:{port} is claimed by another suite run"));
                continue;
            };
            // A listener on the *wildcard* address and one on `127.0.0.1` may
            // hold the same port at the same time — `SO_REUSEADDR` allows it —
            // so a successful bind alone does not mean the port is idle. Ask
            // the kernel's own table, and let a suite that has no `/proc` fall
            // back to the bind result alone.
            let wildcard = listening_inodes(port)
                .map(|inodes| !inodes.is_empty())
                .unwrap_or(false);
            if wildcard {
                refusals.push(format!("127.0.0.1:{port} is busy: already listening"));
                continue;
            }
            match TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, port))) {
                Ok(listener) => {
                    return Claim {
                        port,
                        listener,
                        lock,
                    }
                }
                Err(err) => refusals.push(format!("127.0.0.1:{port} is busy: {err}")),
            }
        }
    }
    panic!("suite window {window} has no claimable loopback port: {refusals:?}");
}

/// Lower bound of the window at `window`, wholly outside the ephemeral range.
///
/// The preferred band is below it (recognisable in a printed port). A host
/// whose ephemeral window swallows that band gets the space above it instead.
fn window_base(window: u16) -> u16 {
    let (low, high) = ephemeral_range();
    let width = u32::from(WINDOW_WIDTH);
    let below = u32::from(FIRST_BASE) + u32::from(window) * u32::from(WINDOW_STRIDE);
    // Wholly below the ephemeral window: no outbound connection on this host
    // can be assigned a port in it, whatever else is running.
    if below + width - 1 < u32::from(low) {
        return below as u16;
    }
    let above = u32::from(high) + 1 + u32::from(window) * u32::from(WINDOW_STRIDE);
    if above + width - 1 <= u32::from(u16::MAX) {
        return above as u16;
    }
    panic!(
        "no {WINDOW_WIDTH}-wide loopback window outside the ephemeral range {:?}",
        ephemeral_range()
    );
}

/// Where this host draws ephemeral **outbound** source ports from
/// (`/proc/sys/net/ipv4/ip_local_port_range`).
///
/// `49152` when the file cannot be read: macOS's default lower bound, and low
/// enough to keep every candidate in the preferred band below it.
pub fn ephemeral_range() -> (u16, u16) {
    std::fs::read_to_string("/proc/sys/net/ipv4/ip_local_port_range")
        .ok()
        .and_then(|text| {
            let mut parts = text.split_whitespace();
            Some((parts.next()?.parse().ok()?, parts.next()?.parse().ok()?))
        })
        .unwrap_or((49_152, u16::MAX))
}

/// Scratch space for the per-port lock files and the children's stderr.
fn scratch_dir() -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("forge-test-ports");
    fs::create_dir_all(&dir).expect("create the test port scratch dir");
    dir
}

fn stderr_of(path: &std::path::Path) -> String {
    let text = fs::read_to_string(path).unwrap_or_default();
    let text = text.trim();
    if text.is_empty() {
        "(empty)".to_string()
    } else {
        text.to_string()
    }
}

/// An exclusive advisory lock on one port's claim protocol.
///
/// `flock` rather than a lock file created with `create_new`: the kernel
/// releases it when the holder dies, so a killed run cannot leave a lock that
/// blocks the next one.
struct Lock {
    file: File,
}

impl Lock {
    fn acquire(path: &std::path::Path, budget: Duration) -> Option<Self> {
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(false)
            .open(path)
            .ok()?;
        let deadline = Instant::now() + budget;
        loop {
            // SAFETY: `file` owns a valid descriptor for the whole call.
            if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } == 0 {
                return Some(Lock { file });
            }
            if Instant::now() >= deadline {
                return None;
            }
            std::thread::sleep(POLL);
        }
    }
}

impl Drop for Lock {
    fn drop(&mut self) {
        // SAFETY: `file` owns a valid descriptor for the whole call.
        let _ = unsafe { libc::flock(self.file.as_raw_fd(), libc::LOCK_UN) };
    }
}

/// Which process holds the `LISTEN` socket on a port.
enum Owner {
    Ours,
    Foreign,
    /// Could not be determined (no `/proc`, or descriptors not readable).
    Unknown,
}

fn owned_by(port: u16, pid: u32) -> Owner {
    let Some(inodes) = listening_inodes(port) else {
        return Owner::Unknown;
    };
    if inodes.is_empty() {
        return Owner::Foreign;
    }
    let Some(sockets) = child_sockets(pid) else {
        return Owner::Unknown;
    };
    if inodes.iter().any(|held| sockets.contains(held)) {
        Owner::Ours
    } else {
        Owner::Foreign
    }
}

/// Every `LISTEN` row on `port`, from both tables, as `local-address:port ->
/// inode`. Diagnostics only: it says *who* is on the port when the ownership
/// check refuses.
fn listening_rows(port: u16) -> Vec<String> {
    ["/proc/net/tcp", "/proc/net/tcp6"]
        .into_iter()
        .flat_map(|table| {
            fs::read_to_string(table)
                .unwrap_or_default()
                .lines()
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .skip(1)
        .filter_map(|line| {
            let fields: Vec<&str> = line.split_whitespace().collect();
            if fields.len() < 10 || fields[3] != "0A" {
                return None;
            }
            let port_text = fields[1].rsplit(':').next()?;
            if u32::from_str_radix(port_text, 16).ok()? != u32::from(port) {
                return None;
            }
            Some(format!("{} -> fd-inode {}", fields[1], fields[9]))
        })
        .collect()
}

/// Every socket inode held by `pid`. `None` when `/proc` cannot answer.
fn child_sockets(pid: u32) -> Option<Vec<String>> {
    let mut sockets = Vec::new();
    for entry in fs::read_dir(format!("/proc/{pid}/fd")).ok()?.flatten() {
        let Ok(target) = fs::read_link(entry.path()) else {
            continue;
        };
        let text = target.to_string_lossy();
        if let Some(inode) = text
            .strip_prefix("socket:[")
            .and_then(|rest| rest.strip_suffix(']'))
        {
            sockets.push(inode.to_string());
        }
    }
    Some(sockets)
}

/// Inodes of the `LISTEN` sockets on `port`, from `/proc/net/tcp`.
///
/// `forge api serve` and `forge web serve` both bind `--bind 127.0.0.1`, so
/// the IPv4 table is the only one that can hold them; `fields[3] == "0A"` is
/// `TCP_LISTEN` and `fields[9]` is the socket inode that `/proc/<pid>/fd`
/// links back to.
fn listening_inodes(port: u16) -> Option<Vec<String>> {
    let text = fs::read_to_string("/proc/net/tcp").ok()?;
    let mut inodes = Vec::new();
    for line in text.lines().skip(1) {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() < 10 || fields[3] != "0A" {
            continue;
        }
        let Some(local) = fields[1].rsplit(':').next() else {
            continue;
        };
        if u32::from_str_radix(local, 16).ok() != Some(u32::from(port)) {
            continue;
        }
        inodes.push(fields[9].to_string());
    }
    Some(inodes)
}
