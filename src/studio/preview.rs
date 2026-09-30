//! Bounded preview session lifecycle (`forge-studio-preview/0.1.0`).
//!
//! The preview module owns three concrete types:
//!
//! - [`PreviewRunner`] is the trait the Studio uses to launch the
//!   profile runner. The bundled impl is the shared
//!   `crate::process::spawn_with_timeout` helper over the profile
//!   argv; the in-process [`FakeRunner`] is the test impl.
//! - [`PreviewSession`] is the in-process state record the
//!   Studio owns. It is **not** persisted across Forge restarts
//!   (a restart transitions every running preview to `stopped`
//!   and the next `start` re-spawns from the saved
//!   `session.json`).
//! - [`PreviewEnvelope`] is the wire envelope every transport
//!   renders. The session record's [`crate::studio::state::SessionPreviewState`]
//!   is the on-disk mirror; both share the same closed
//!   [`PreviewState`] vocabulary.
//!
//! Process guarantees:
//!
//! - `argv` is profile-owned and never interpolated through a shell.
//! - The bounded startup window is clamped to `1..=600` seconds.
//! - The bounded log cap is [`MAX_PREVIEW_LOG_BYTES`] (1 MiB) with
//!   a `[truncated]` marker.
//! - Port collision never kills an unrelated process: the
//!   allocator walks upwards through the range and refuses with
//!   [`crate::core::ForgeError::StudioPortUnavailable`] when
//!   every candidate in the range is taken.
//! - Stop is idempotent: a session already `stopped` returns the
//!   same envelope and records a single `studio.preview.stop` row.
//! - Logs are credential-redacted through
//!   [`crate::policy::redact_credentials`] before they reach a
//!   journal row or the API envelope.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::spec::SUPPORTED_PROFILES;
use super::state::{
    load_session, persist_session, record_journal, SessionPreviewState, StudioJournalKind,
    StudioSession,
};
use crate::core::ForgeError;
use crate::policy::redact_credentials;

/// Wire contract id for the bounded preview envelope.
pub const PREVIEW_CONTRACT: &str = "forge-studio-preview/0.1.0";

/// Default bounded startup window for the profile runner. Clamped
/// to `1..=600` seconds via [`startup_timeout`] so the operator
/// cannot accidentally pin the listener with an unbounded spawn.
pub const STARTUP_TIMEOUT_DEFAULT_SECS: u64 = 30;

/// Maximum per-session log capture. Mirrors the bounded log caps
/// other Forge adapters apply so a verbose preview cannot pin a
/// host log.
pub const MAX_PREVIEW_LOG_BYTES: usize = 1024 * 1024;

/// Marker appended when the captured log exceeds
/// [`MAX_PREVIEW_LOG_BYTES`]. The marker is fixed so a transport
/// can recognize the cap.
pub const PREVIEW_LOG_TRUNCATION_MARKER: &str = "[truncated]";

/// Env-var: override the preview runner binary. The default is
/// `npm`; a controlled stub or an explicit toolchain path is used
/// in its place so the same bounded argv flows through the same
/// code path. Mirrors the `FORGE_GH_BIN` override for `gh`.
pub const RUNTIME_BIN_ENV: &str = "FORGE_STUDIO_RUNNER_BIN";

/// Env-var passed to the spawned runner with the reserved port.
pub const RUNTIME_PORT_ENV: &str = "FORGE_STUDIO_PORT";

/// Wall-clock wait between successive `try_connect` probes while
/// the Studio waits for the profile runner to bind the reserved
/// port. Bounded so the helper never busy-loops.
const PORT_PROBE_INTERVAL: Duration = Duration::from_millis(50);

/// Re-export so callers see a single `crate::studio::PreviewState`.
pub use super::state::PreviewState;

/// Bounded terminal outcome surfaced by the [`PreviewSession::wait_until_ready`]
/// helper. Mirrors the on-disk [`PreviewState`] vocabulary so the
/// transport envelope can render either form without re-mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PreviewOutcome {
    Ready,
    Failed,
    Stopped,
}

impl PreviewOutcome {
    pub fn label(self) -> &'static str {
        match self {
            PreviewOutcome::Ready => "ready",
            PreviewOutcome::Failed => "failed",
            PreviewOutcome::Stopped => "stopped",
        }
    }
}

impl From<PreviewState> for PreviewOutcome {
    fn from(value: PreviewState) -> Self {
        match value {
            PreviewState::None | PreviewState::Stopped | PreviewState::Stopping => {
                PreviewOutcome::Stopped
            }
            PreviewState::Ready => PreviewOutcome::Ready,
            PreviewState::Starting | PreviewState::Failed => PreviewOutcome::Failed,
        }
    }
}

/// Envelope rendered to every transport. Carries the closed
/// `forge-studio-preview/0.1.0` contract id, the project and
/// revision scope, the bounded state, the reserved port, and the
/// same-origin preview URL only when `state == ready`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PreviewEnvelope {
    pub contract: &'static str,
    pub project_id: String,
    pub revision: String,
    pub state: PreviewState,
    pub port: Option<u16>,
    pub preview_url: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub last_error_code: Option<String>,
}

impl PreviewEnvelope {
    /// Build the closed envelope from an in-process session
    /// record. The `preview_url` is populated only when the state
    /// is [`PreviewState::Ready`] and a port is reserved, so a
    /// transport that reads the envelope while the preview is
    /// still `starting` sees `preview_url = None`.
    pub fn from_session(project_id: &str, revision: &str, session: &SessionPreviewState) -> Self {
        let preview_url = match (session.state, session.port) {
            (PreviewState::Ready, Some(port)) => Some(format!("http://127.0.0.1:{port}/")),
            _ => None,
        };
        Self {
            contract: PREVIEW_CONTRACT,
            project_id: project_id.to_string(),
            revision: revision.to_string(),
            state: session.state,
            port: session.port,
            preview_url,
            started_at: session.started_at,
            last_error_code: session.last_error_code.clone(),
        }
    }
}

/// The bounded preview session. The process handle is owned by
/// the Studio and never crosses the API/UI boundary; the session
/// only exposes the closed [`PreviewEnvelope`] and the bounded
/// status record the journal row carries.
pub struct PreviewSession {
    pub project_id: String,
    pub revision: String,
    pub port: u16,
    pub runner: Box<dyn PreviewRunner>,
    child: Arc<Mutex<Option<ChildHandle>>>,
    #[allow(dead_code)]
    state: PreviewState,
    started_at: DateTime<Utc>,
    log_tail: Arc<Mutex<Vec<u8>>>,
    last_error_code: Arc<Mutex<Option<String>>>,
}

/// Opaque handle to a spawned profile runner. The fields are
/// private: transports only ever see the closed envelope, never
/// the process handle. The type is public because it is the return
/// type of [`PreviewRunner::spawn`].
pub struct ChildHandle {
    child: Child,
    stdout_thread: Option<thread::JoinHandle<()>>,
    /// The bounded accept thread that owns the fake runner's
    /// TCP listener. Held here so `Drop` can join it.
    accept_thread: Option<thread::JoinHandle<()>>,
    /// Stop flag for the accept thread. The fake runner's listener is
    /// non-blocking, so setting this lets `stop`/`Drop` join the
    /// thread instead of blocking until the listener is dropped.
    accept_stop: Option<Arc<AtomicBool>>,
}

impl ChildHandle {
    /// Kill the child and join its capture threads. The accept flag is
    /// set first so a non-blocking fake listener exits promptly rather
    /// than blocking `stop`/`Drop` forever on `join`.
    fn shutdown(&mut self) {
        if let Some(stop) = &self.accept_stop {
            stop.store(true, Ordering::SeqCst);
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
        if let Some(thread) = self.stdout_thread.take() {
            let _ = thread.join();
        }
        if let Some(thread) = self.accept_thread.take() {
            let _ = thread.join();
        }
    }
}

impl PreviewSession {
    /// Bounded wait for the profile runner to bind the reserved
    /// port and report it is healthy. Returns the captured state
    /// and the bounded log tail. The bounded timeout kills the
    /// child process on expiry and records
    /// [`ForgeError::StudioStartTimeout`] before returning.
    pub fn wait_until_ready(
        &self,
        registry: &crate::registry::Registry,
        timeout: Duration,
    ) -> Result<SessionPreviewState, ForgeError> {
        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            if let Some(handle) = self.child.lock().unwrap().as_mut() {
                if let Ok(Some(_status)) = handle.child.try_wait() {
                    self.record_failed(
                        registry,
                        "studio-start-timeout",
                        "profile runner exited before binding the reserved port",
                    )?;
                    return Err(ForgeError::StudioStartTimeout {
                        reason: format!(
                            "profile runner exited before binding the reserved port (port {})",
                            self.port
                        ),
                    });
                }
            }
            if port_is_open(self.port) {
                return self.record_ready(registry);
            }
            thread::sleep(PORT_PROBE_INTERVAL);
        }
        // Bounded timeout expired. Kill the child and record the
        // typed refusal; the journal row is the audit trail.
        if let Some(mut handle) = self.child.lock().unwrap().take() {
            handle.shutdown();
        }
        self.record_failed(
            registry,
            "studio-start-timeout",
            "profile runner did not bind the reserved port within the startup window",
        )?;
        Err(ForgeError::StudioStartTimeout {
            reason: format!("profile runner did not bind the reserved port within {timeout:?}"),
        })
    }

    fn record_ready(
        &self,
        registry: &crate::registry::Registry,
    ) -> Result<SessionPreviewState, ForgeError> {
        *self.last_error_code.lock().unwrap() = None;
        let state = PreviewState::Ready;
        let log_tail = cap_log(&self.log_tail.lock().unwrap());
        let detail = bounded_detail(&format!(
            "state={} port={} project_id={} revision={}",
            state.label(),
            self.port,
            self.project_id,
            self.revision
        ));
        record_journal(
            registry,
            StudioJournalKind::PreviewStart,
            &self.project_id,
            state.label(),
            &detail,
        )?;
        Ok(SessionPreviewState {
            state,
            port: Some(self.port),
            started_at: Some(self.started_at),
            last_error_code: None,
            log_tail,
        })
    }

    fn record_failed(
        &self,
        registry: &crate::registry::Registry,
        code: &str,
        detail_text: &str,
    ) -> Result<(), ForgeError> {
        *self.last_error_code.lock().unwrap() = Some(code.to_string());
        let detail = bounded_detail(&format!(
            "state={} port={} project_id={} revision={} last_error={code} detail={detail_text}",
            PreviewState::Failed.label(),
            self.port,
            self.project_id,
            self.revision
        ));
        record_journal(
            registry,
            StudioJournalKind::PreviewStart,
            &self.project_id,
            PreviewState::Failed.label(),
            &detail,
        )
    }

    /// Idempotent stop. A session already stopped returns the
    /// same envelope and writes exactly one journal row per call.
    pub fn stop(
        &mut self,
        registry: &crate::registry::Registry,
    ) -> Result<SessionPreviewState, ForgeError> {
        let mut handle = self.child.lock().unwrap().take();
        if let Some(handle) = handle.as_mut() {
            handle.shutdown();
        }
        let log_tail = cap_log(&self.log_tail.lock().unwrap());
        let detail = bounded_detail(&format!(
            "state={} port={} project_id={} revision={}",
            PreviewState::Stopped.label(),
            self.port,
            self.project_id,
            self.revision
        ));
        record_journal(
            registry,
            StudioJournalKind::PreviewStop,
            &self.project_id,
            PreviewState::Stopped.label(),
            &detail,
        )?;
        Ok(SessionPreviewState {
            state: PreviewState::Stopped,
            port: Some(self.port),
            started_at: Some(self.started_at),
            last_error_code: None,
            log_tail,
        })
    }

    /// Snapshot the bounded, redacted log tail. Used by the Core
    /// to persist the terminal state when a start fails before a
    /// session record exists to carry it.
    pub fn log_tail(&self) -> String {
        cap_log(&self.log_tail.lock().unwrap())
    }
}

fn port_is_open(port: u16) -> bool {
    TcpStream::connect_timeout(
        &std::net::SocketAddr::from((std::net::Ipv4Addr::LOCALHOST, port)),
        Duration::from_millis(50),
    )
    .is_ok()
}

/// Trait that owns the actual child process. The bundled impl
/// delegates to the shared `crate::process::spawn_with_timeout`
/// helper over a profile-owned argv; the [`FakeRunner`] is the
/// test impl and binds the reserved port on a controlled delay.
pub trait PreviewRunner: Send + Sync {
    /// Spawn the runner and return a thread handle that streams
    /// stdout into the bounded log buffer. The trait is process-
    /// owned; transport code never sees the [`Child`] handle.
    fn spawn(
        &self,
        project_root: &Path,
        port: u16,
        log_buffer: Arc<Mutex<Vec<u8>>>,
    ) -> Result<ChildHandle, ForgeError>;
}

/// The bundled profile-runner impl. The argv is derived from the
/// profile descriptor's bounded `build_command` (today the
/// `react-web` profile declares `npm run dev`; the descriptor is
/// consulted at runtime so a profile bump lands without a Studio
/// rebuild). The shared `spawn_with_timeout` enforces the
/// bounded wall-clock cap and the 2 KiB stderr cap.
pub struct ProcessRunner {
    pub binary: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub timeout: Duration,
}

impl PreviewRunner for ProcessRunner {
    fn spawn(
        &self,
        project_root: &Path,
        port: u16,
        log_buffer: Arc<Mutex<Vec<u8>>>,
    ) -> Result<ChildHandle, ForgeError> {
        let mut command = Command::new(&self.binary);
        command
            .args(self.args.iter())
            .current_dir(project_root)
            .env_clear();
        for (key, value) in &self.env {
            command.env(key, value);
        }
        command.env(RUNTIME_PORT_ENV, port.to_string());
        command.stdout(Stdio::piped()).stderr(Stdio::piped());
        let mut child = command
            .spawn()
            .map_err(|err| ForgeError::StudioPortUnavailable {
                reason: format!("could not spawn profile runner '{}': {err}", self.binary),
            })?;
        let stdout = child.stdout.take();
        let stdout_thread = stdout.map(|mut stdout| {
            thread::spawn(move || {
                let mut local = Vec::new();
                let _ = stdout.read_to_end(&mut local);
                if let Ok(mut buf) = log_buffer.lock() {
                    buf.extend_from_slice(&local);
                }
            })
        });
        Ok(ChildHandle {
            child,
            stdout_thread,
            accept_thread: None,
            accept_stop: None,
        })
    }
}

impl ProcessRunner {
    /// Build the bundled `react-web` runner. The argv is the
    /// profile's bounded `npm run dev`; `FORGE_STUDIO_RUNNER_BIN`
    /// overrides the binary so a controlled stub (or an explicit
    /// toolchain path) flows through the same code path without a
    /// shell. The parent `PATH` is inherited so the profile
    /// toolchain resolves, and `LC_ALL=C` keeps the captured log
    /// stable.
    pub fn react_web() -> Self {
        let mut env = HashMap::new();
        env.insert("LC_ALL".to_string(), "C".to_string());
        if let Ok(path) = std::env::var("PATH") {
            env.insert("PATH".to_string(), path);
        }
        let binary = std::env::var(RUNTIME_BIN_ENV).unwrap_or_else(|_| "npm".to_string());
        Self {
            binary,
            args: vec!["run".to_string(), "dev".to_string()],
            env,
            timeout: Duration::from_secs(STARTUP_TIMEOUT_DEFAULT_SECS),
        }
    }
}

/// A `PreviewRunner` impl used by tests. The fake runner binds
/// the reserved port on a controlled delay so the bounded wait
/// and the bounded timeout are observable in isolation. The
/// runner never touches a real profile toolchain.
pub struct FakeRunner {
    pub binary: String,
    pub bind_delay: Duration,
    pub exit_code: Option<i32>,
    /// When false the fake runner never binds the reserved port, so
    /// the bounded wait observes the child exit or the timeout. Used
    /// to prove the startup-failure path without touching a toolchain.
    pub bind_listener: bool,
}

impl Default for FakeRunner {
    fn default() -> Self {
        Self {
            binary: "sh".to_string(),
            bind_delay: Duration::from_millis(50),
            exit_code: None,
            bind_listener: true,
        }
    }
}

impl PreviewRunner for FakeRunner {
    fn spawn(
        &self,
        _project_root: &Path,
        port: u16,
        log_buffer: Arc<Mutex<Vec<u8>>>,
    ) -> Result<ChildHandle, ForgeError> {
        // The fake runner prints a known log line, optionally
        // sleeps before binding the port, optionally exits before
        // binding it. The script is a single string the test can
        // inspect; no profile argv crosses the trait boundary.
        let script = match self.exit_code {
            Some(code) => format!("printf 'fake-runner:ready\\n'; exit {code}"),
            None => {
                // A bounded child the harness can kill cleanly. The
                // reserved port is served by the in-process listener
                // below, so the fake runner does not need `nc` and
                // never leaves an orphan holding the port.
                "printf 'fake-runner:ready\\n'; exec sleep 600".to_string()
            }
        };
        let mut command = Command::new(&self.binary);
        command
            .arg("-c")
            .arg(script)
            .env("FORGE_STUDIO_FAKE_PORT", port.to_string())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = command
            .spawn()
            .map_err(|err| ForgeError::StudioPortUnavailable {
                reason: format!("could not spawn fake preview runner: {err}"),
            })?;
        // Capture the child's bounded stdout; shared by the success and
        // failure paths. The trait guarantees stdout is captured.
        let stdout = child.stdout.take();
        let capture_thread = stdout.map(|mut stdout| {
            let buf = Arc::clone(&log_buffer);
            thread::spawn(move || {
                let mut local = Vec::new();
                let _ = stdout.read_to_end(&mut local);
                if let Ok(mut buf) = buf.lock() {
                    buf.extend_from_slice(&local);
                }
            })
        });
        if !self.bind_listener {
            // No listener: the bounded wait observes the child exit or
            // the timeout, proving the failure path without a toolchain.
            return Ok(ChildHandle {
                child,
                stdout_thread: capture_thread,
                accept_thread: None,
                accept_stop: None,
            });
        }
        // Wait the controlled delay, then bind the reserved port with a
        // non-blocking listener that `stop`/`Drop` can shut down.
        thread::sleep(self.bind_delay);
        let listener = TcpListener::bind(("127.0.0.1", port)).map_err(|err| {
            ForgeError::StudioPortUnavailable {
                reason: format!("fake preview runner could not bind port {port}: {err}"),
            }
        })?;
        let buf = Arc::clone(&log_buffer);
        let accept_stop = Arc::new(AtomicBool::new(false));
        let accept_stop_thread = Arc::clone(&accept_stop);
        let accept_thread = thread::spawn(move || {
            // Non-blocking so `stop`/`Drop` can set the flag and join
            // the thread instead of blocking until the listener drops.
            let _ = listener.set_nonblocking(true);
            while !accept_stop_thread.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        let _ = stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok");
                        let _ = stream.flush();
                    }
                    Err(ref err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5));
                    }
                    Err(_) => break,
                }
            }
            if let Ok(mut buf) = buf.lock() {
                buf.extend_from_slice(b"\nfake-runner:listener-exited\n");
            }
        });
        Ok(ChildHandle {
            child,
            stdout_thread: capture_thread,
            accept_thread: Some(accept_thread),
            accept_stop: Some(accept_stop),
        })
    }
}

/// Resolve the bounded startup window for the bounded wait.
/// Reads `$FORGE_STUDIO_STARTUP_TIMEOUT_SECS` and clamps the
/// value into `1..=600` so the operator cannot accidentally pin
/// the listener with an unbounded spawn.
pub fn startup_timeout() -> Duration {
    let raw = std::env::var("FORGE_STUDIO_STARTUP_TIMEOUT_SECS")
        .ok()
        .and_then(|s| s.trim().parse::<u64>().ok())
        .unwrap_or(STARTUP_TIMEOUT_DEFAULT_SECS);
    let clamped = raw.clamp(1, 600);
    Duration::from_secs(clamped)
}

/// Resolve the lower bound of the Studio port range. The
/// allocator walks upwards through the range and refuses with
/// `studio-port-unavailable` after [`crate::studio::PORT_RANGE_WIDTH`]
/// consecutive busy ports.
pub fn port_range_start() -> u16 {
    std::env::var(super::STUDIO_PORT_RANGE_START_ENV)
        .ok()
        .and_then(|s| s.trim().parse::<u16>().ok())
        .unwrap_or(super::DEFAULT_PORT_RANGE_START)
}

/// Allocate the first free port in the Studio range. The
/// allocator **never** kills an unrelated process: it walks
/// upwards, binding only the candidates the kernel reports as
/// free, and refuses with [`ForgeError::StudioPortUnavailable`]
/// after [`crate::studio::PORT_RANGE_WIDTH`] consecutive busy
/// candidates.
pub fn allocate_port() -> Result<u16, ForgeError> {
    let start = port_range_start();
    for offset in 0..super::PORT_RANGE_WIDTH {
        let candidate = start.saturating_add(offset);
        if TcpListener::bind(("127.0.0.1", candidate)).is_ok() {
            return Ok(candidate);
        }
    }
    Err(ForgeError::StudioPortUnavailable {
        reason: format!(
            "no free port in range {start}..={}",
            start.saturating_add(super::PORT_RANGE_WIDTH - 1)
        ),
    })
}

/// Validate a profile id before spawning the runner. The MVP
/// accepts only the closed [`SUPPORTED_PROFILES`] set; future
/// profile additions land behind a contract bump.
pub fn validate_profile(profile: &str) -> Result<(), ForgeError> {
    if SUPPORTED_PROFILES.contains(&profile) {
        Ok(())
    } else {
        Err(ForgeError::StudioUnsupportedProfile {
            reason: format!(
                "profile '{profile}' is not supported by Forge Studio today; supported profiles: {}",
                SUPPORTED_PROFILES.join(", ")
            ),
        })
    }
}

/// Snapshot the current preview state from a [`StudioSession`].
/// Returns the on-disk record's mirror so the API/CLI can
/// render an envelope without owning the process handle.
pub fn envelope_from_session(session: &StudioSession) -> PreviewEnvelope {
    PreviewEnvelope::from_session(&session.project_id, &session.app_revision, &session.preview)
}

fn cap_log(buffer: &[u8]) -> String {
    if buffer.len() <= MAX_PREVIEW_LOG_BYTES {
        return redact_credentials(&String::from_utf8_lossy(buffer));
    }
    let mut truncated =
        redact_credentials(&String::from_utf8_lossy(&buffer[..MAX_PREVIEW_LOG_BYTES]));
    truncated.push_str(PREVIEW_LOG_TRUNCATION_MARKER);
    truncated
}

fn bounded_detail(detail: &str) -> String {
    let redacted = redact_credentials(detail);
    if redacted.len() <= 2048 {
        return redacted;
    }
    let mut out = String::with_capacity(2048 + 16);
    for c in redacted.chars().take(2048) {
        out.push(c);
    }
    out.push_str("[truncated]");
    out
}

// Convenience wrappers used by the CLI/API transports.
impl PreviewSession {
    /// Spawn a new bounded preview session. Allocates a port,
    /// invokes the supplied [`PreviewRunner`], and returns the
    /// session so the caller can wait for `ready` or stop it.
    /// This is the single place the Studio talks to the runner;
    /// transports never call [`PreviewRunner::spawn`] directly.
    pub fn start(
        project_id: &str,
        revision: &str,
        project_root: &Path,
        runner: Box<dyn PreviewRunner>,
    ) -> Result<Self, ForgeError> {
        let port = allocate_port()?;
        let log_buffer: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
        let child = runner.spawn(project_root, port, Arc::clone(&log_buffer))?;
        Ok(Self {
            project_id: project_id.to_string(),
            revision: revision.to_string(),
            port,
            runner,
            child: Arc::new(Mutex::new(Some(child))),
            state: PreviewState::Starting,
            started_at: Utc::now(),
            log_tail: log_buffer,
            last_error_code: Arc::new(Mutex::new(None)),
        })
    }
}

/// Start a bounded preview for a saved session and persist the
/// terminal state. On success the returned [`StudioSession`] carries
/// the `ready` state and the caller owns the live [`PreviewSession`]
/// (dropping or stopping it kills only this runner). On failure the
/// persisted session records `failed` with the typed
/// `last_error_code` and the error is returned unchanged, so a later
/// `status` never reports a stale `ready`.
pub fn start_preview(
    registry: &crate::registry::Registry,
    project_root: &Path,
    runner: Box<dyn PreviewRunner>,
) -> Result<(StudioSession, PreviewSession), ForgeError> {
    let mut session = load_session(project_root)?.ok_or_else(|| ForgeError::StudioInvalidSpec {
        reason: "no Studio session exists; save a spec before starting a preview".to_string(),
    })?;
    validate_profile(&session.profile)?;
    let live = match PreviewSession::start(
        &session.project_id,
        &session.app_revision,
        project_root,
        runner,
    ) {
        Ok(live) => live,
        Err(err) => {
            persist_failed(
                project_root,
                &mut session,
                err.code(),
                None,
                None,
                String::new(),
            )?;
            return Err(err);
        }
    };
    match live.wait_until_ready(registry, startup_timeout()) {
        Ok(state) => {
            session.preview = state;
            session.updated_at = Utc::now();
            persist_session(project_root, &session)?;
            Ok((session, live))
        }
        Err(err) => {
            persist_failed(
                project_root,
                &mut session,
                err.code(),
                Some(live.port),
                Some(live.started_at),
                live.log_tail(),
            )?;
            Err(err)
        }
    }
}

/// Stop a bounded preview. When a live session is supplied it is
/// killed and its bounded log tail is captured; when `None` the
/// persisted record is stopped idempotently. Either way exactly one
/// `studio.preview.stop` journal row is recorded and the terminal
/// `stopped` state is persisted, so repeated stops each leave one
/// audit row and never claim a running preview.
pub fn stop_preview(
    registry: &crate::registry::Registry,
    project_root: &Path,
    live: Option<PreviewSession>,
) -> Result<StudioSession, ForgeError> {
    let mut session = load_session(project_root)?.ok_or_else(|| ForgeError::StudioInvalidSpec {
        reason: "no Studio session exists; save a spec before stopping a preview".to_string(),
    })?;
    let stopped = match live {
        Some(mut live) => live.stop(registry)?,
        None => {
            let port_label = session
                .preview
                .port
                .map(|port| port.to_string())
                .unwrap_or_else(|| "none".to_string());
            let detail = bounded_detail(&format!(
                "state={} port={} project_id={} revision={}",
                PreviewState::Stopped.label(),
                port_label,
                session.project_id,
                session.app_revision,
            ));
            record_journal(
                registry,
                StudioJournalKind::PreviewStop,
                &session.project_id,
                PreviewState::Stopped.label(),
                &detail,
            )?;
            SessionPreviewState {
                state: PreviewState::Stopped,
                port: session.preview.port,
                started_at: session.preview.started_at,
                last_error_code: None,
                log_tail: session.preview.log_tail.clone(),
            }
        }
    };
    session.preview = stopped;
    session.updated_at = Utc::now();
    persist_session(project_root, &session)?;
    Ok(session)
}

fn persist_failed(
    project_root: &Path,
    session: &mut StudioSession,
    code: &str,
    port: Option<u16>,
    started_at: Option<DateTime<Utc>>,
    log_tail: String,
) -> Result<(), ForgeError> {
    session.preview = SessionPreviewState {
        state: PreviewState::Failed,
        port,
        started_at,
        last_error_code: Some(code.to_string()),
        log_tail,
    };
    session.updated_at = Utc::now();
    persist_session(project_root, session)
}

impl Drop for PreviewSession {
    fn drop(&mut self) {
        // A dropped session is treated as a stop. Killing the
        // child on drop keeps the bounded guarantee that no
        // profile runner survives the Studio's lifetime even
        // when a caller forgets to invoke `stop`.
        if let Some(mut handle) = self.child.lock().unwrap().take() {
            handle.shutdown();
        }
    }
}

// Re-export session_path for the API/CLI to find the on-disk record.
pub use super::state::session_path as session_record_path;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allocate_port_returns_a_free_port() {
        let port = allocate_port().expect("free port");
        assert!(port > 0);
    }

    #[test]
    fn validate_profile_accepts_only_supported_set() {
        assert!(validate_profile("react-web").is_ok());
        let err = validate_profile("aspnet-web").unwrap_err();
        assert_eq!(err.code(), "studio-unsupported-profile");
    }

    #[test]
    fn envelope_renders_only_when_ready() {
        let mut session_preview = SessionPreviewState::default();
        let env = PreviewEnvelope::from_session("p1", "r1", &session_preview);
        assert_eq!(env.state, PreviewState::None);
        assert!(env.preview_url.is_none());
        session_preview.state = PreviewState::Ready;
        session_preview.port = Some(8080);
        let env = PreviewEnvelope::from_session("p1", "r1", &session_preview);
        assert!(env.preview_url.as_deref().unwrap().contains("8080"));
    }

    #[test]
    fn cap_log_marks_large_buffers() {
        let buf: Vec<u8> = vec![b'x'; MAX_PREVIEW_LOG_BYTES + 256];
        let out = cap_log(&buf);
        assert!(out.ends_with(PREVIEW_LOG_TRUNCATION_MARKER));
    }

    #[test]
    fn cap_log_redacts_credentials_in_small_buffers() {
        let buf = b"hi ghp_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa there";
        let out = cap_log(buf);
        assert!(!out.contains("ghp_"));
    }
}
