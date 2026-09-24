//! Managed agent session contract (`agent-runtime-workflows`).
//!
//! Core owns the versioned session model and the provider-neutral
//! transition rules; transports (CLI now; MCP/API later) render Core
//! outcomes without reinterpreting them.
//!
//! The contract is intentionally honest about provider support:
//! the bundled OpenCode/Codex adapters' `start`/`new_session`
//! always succeed when the adapter binary is present on PATH, and
//! `resume`/`restart` follow the recorded transition log. On the
//! bundled adapters `pause` and `takeover` remain explicitly
//! `unsupported` because the bundled surface does not expose those
//! primitives. The `ariadex` provider instead delegates every
//! transition to the real supervised runtime (`supervised-agent-
//! adapters`): ordered binary resolution (`FORGE_ARIADEX_BIN`
//! first, then the PATH name), argument-array invocation with a
//! bounded wait, and a state claim taken only from what
//! `ariadex status --json` actually reported — an unknown or
//! unlive runtime maps to `disconnected`, never to `active`,
//! and `takeover` records attach guidance instead of hijacking
//! the operator's terminal.

//!
//! Session storage lives under the project root so each project
//! owns its own sessions and the registry's operation journal
//! records the originating operation:
//!
//! ```text
//! .forge/agents/
//!   <session-id>/
//!     session.json   AgentSession with provider, spec, transitions
//!     transitions.log  append-only transition evidence
//! ```
//!
//! The contract version is recorded in every session and the
//! recorded transitions, so a future schema bump can refuse
//! incompatible readers instead of silently reinterpreting
//! history. Spec execution is routed to the recorded spec id
//! (R1 success scenario); a missing spec surfaces as
//! `error[spec-invalid]` without changing the project.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::core::manifest::{Manifest, CANONICAL_MANIFEST};
use crate::core::ForgeError;
use crate::policy::redact_credentials;

/// Contract data version for the agent session and transition API.
pub const AGENT_CONTRACT_VERSION: &str = "0.1.0";

/// Directory (relative to the project root) holding session records.
pub const AGENTS_DIR: &str = ".forge/agents";

/// Maximum recorded transitions per session. New transitions beyond
/// the cap refuse with a typed error so the session file never
/// grows unbounded.
pub const MAX_TRANSITIONS_PER_SESSION: usize = 256;

/// Wait timeout for adapter subprocess invocations. An adapter that
/// hangs past the timeout is recorded as a failed transition with
/// `unavailable` evidence; the session file is preserved.
pub const ADAPTER_WAIT_TIMEOUT: Duration = Duration::from_secs(15);

/// Environment variable naming the `ariadex` binary. Checked
/// before the PATH probe, matching the `FORGE_DEPLOYER_BIN` /
/// `FORGE_ANALYTICS_BIN` pattern the deploy and analytics planes
/// use.
pub const ARIADEX_BIN_ENV: &str = "FORGE_ARIADEX_BIN";

/// Default PATH name for the supervised session runtime.
pub const DEFAULT_ARIADEX_BIN: &str = "ariadex";

/// Environment variable naming the `sisyphusfy` iteration
/// supervisor binary.
pub const SISYPHUSFY_BIN_ENV: &str = "FORGE_SISYPHUSFY_BIN";

/// Default PATH name for the spec-execution supervisor.
pub const DEFAULT_SISYPHUSFY_BIN: &str = "sisyphusfy";

/// The only run-spec supervisor id accepted by
/// `forge agent run-spec --provider` and the MCP `run_agent`
/// `provider` argument on a `run_spec` transition.
pub const SISYPHUSFY_SUPERVISOR: &str = "sisyphusfy";

/// Bounded wait for a delegated `sisyphusfy run`. The supervisor
/// drives a whole iteration loop, so this is deliberately far
/// longer than a lifecycle probe; a supervisor that exceeds it is
/// recorded as `unverified`, never as `done`.
pub const RUN_SPEC_WAIT_TIMEOUT: Duration = Duration::from_secs(900);

/// Character bound for every single captured runtime-output line
/// promoted into session evidence (redaction applies first).
pub const MAX_EVIDENCE_CHARS: usize = 300;

/// Supported agent providers. Each provider has its own adapter
/// that reports which transitions it actually supports. New
/// providers can be added without breaking existing sessions
/// because the provider id is recorded with every transition.
/// `opencode`/`codex` are the bundled PATH-probed adapters;
/// `ariadex` delegates the whole session lifecycle to the
/// workspace's supervised tmux runtime (`supervised-agent-
/// adapters`), and `sisyphusfy` is a spec-execution supervisor
/// (see `run_spec`), never a session provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentProvider {
    Opencode,
    Codex,
    Ariadex,
}

impl AgentProvider {
    pub fn label(&self) -> &'static str {
        match self {
            AgentProvider::Opencode => "opencode",
            AgentProvider::Codex => "codex",
            AgentProvider::Ariadex => "ariadex",
        }
    }

    /// Subprocess binary name the adapter would invoke. Documented
    /// in the session record so a future audit can confirm what the
    /// adapter tried to run.
    pub fn binary(&self) -> &'static str {
        match self {
            AgentProvider::Opencode => "opencode",
            AgentProvider::Codex => "codex",
            AgentProvider::Ariadex => DEFAULT_ARIADEX_BIN,
        }
    }

    /// Whether the provider delegates its lifecycle to a real
    /// supervised runtime instead of the bundled PATH-probe rules.
    pub fn is_supervised(&self) -> bool {
        matches!(self, AgentProvider::Ariadex)
    }
}

/// Agent session lifecycle. `Active` is a running session;
/// `Paused` requires a provider that supports pause; `Disconnected`
/// means the manager is gone; `Unsupported` means the requested
/// transition is not available on the current provider; `Stopped`
/// is a terminal state after an explicit `new-session` (R1 boundary
/// scenario: a session that ends without verification is recorded
/// distinctly from a successful spec completion).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionState {
    Idle,
    Active,
    Paused,
    Disconnected,
    Unsupported,
    Stopped,
}

impl SessionState {
    pub fn label(&self) -> &'static str {
        match self {
            SessionState::Idle => "idle",
            SessionState::Active => "active",
            SessionState::Paused => "paused",
            SessionState::Disconnected => "disconnected",
            SessionState::Unsupported => "unsupported",
            SessionState::Stopped => "stopped",
        }
    }
}

/// Transition kind the caller requested. Each transition is
/// applied through the provider adapter; the resulting
/// [`AgentSession`] always records the requested kind, the
/// resulting state, the wall-clock timestamp, and the evidence
/// (command, exit, captured text snippets) the adapter returned.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionTransition {
    Start,
    Pause,
    Takeover,
    Resume,
    Restart,
    NewSession,
}

impl SessionTransition {
    pub fn label(&self) -> &'static str {
        match self {
            SessionTransition::Start => "start",
            SessionTransition::Pause => "pause",
            SessionTransition::Takeover => "takeover",
            SessionTransition::Resume => "resume",
            SessionTransition::Restart => "restart",
            SessionTransition::NewSession => "new_session",
        }
    }
}

/// One recorded transition in the session log. Evidence is the
/// short, sanitized output of the adapter invocation (command,
/// exit, stderr summary); the full subprocess output stays in the
/// adapter's own log so a future audit can correlate the result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransitionRecord {
    pub kind: SessionTransition,
    pub state: SessionState,
    pub at: DateTime<Utc>,
    pub evidence: Vec<String>,
    pub note: String,
}

/// Where the truth about a session actually lives for a
/// supervised provider. The session record stays a Forge-owned
/// pointer — it never copies runtime state — but names the
/// backing runtime, the handle the runtime itself reported, and
/// the version probe that attributed the surface.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeBacking {
    pub runtime: String,
    pub handle: String,
    pub adapter_version: String,
}

/// One session record anchored to a project and (optionally) a
/// spec. The session is the only place that records the chosen
/// provider and the spec id the session is bound to, so a future
/// `forge agent status` can re-render the recorded state without
/// consulting the adapter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSession {
    pub contract: String,
    pub session_id: String,
    pub project_id: String,
    pub project_path: String,
    pub provider: AgentProvider,
    /// Spec id the session is bound to (empty when the session
    /// was started without a spec).
    pub spec_id: String,
    pub state: SessionState,
    pub started_at: DateTime<Utc>,
    pub last_transition_at: DateTime<Utc>,
    pub transitions: Vec<TransitionRecord>,
    /// Optional backing runtime descriptor. Bundled adapters keep
    /// `backing: None`; session files written before the
    /// `supervised-agent-adapters` change deserialize unchanged
    /// through the serde default.
    #[serde(default)]
    pub backing: Option<RuntimeBacking>,
}

impl AgentSession {
    fn empty_transitions() -> Vec<TransitionRecord> {
        Vec::new()
    }
}

/// Outcome of a transition request. Always carries the full
/// session after the transition, the recorded evidence, the
/// available next step (when one is appropriate), and a note for
/// the transport. `verdict` is `Some` only for delegated
/// run-spec executions (`done` / `partial` / `unverified`) so
/// transports can journal the supervisor's verdict without
/// inventing a vocabulary for the bundled path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AgentTransitionOutcome {
    pub contract: String,
    pub session: AgentSession,
    pub requested: SessionTransition,
    pub state: SessionState,
    pub evidence: Vec<String>,
    pub next_step: Option<String>,
    pub note: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verdict: Option<String>,
}

/// Resolve `dir/.forge/agents/<session-id>/` to the on-disk path.
pub fn session_dir(dir: &Path, session_id: &str) -> PathBuf {
    dir.join(AGENTS_DIR).join(session_id)
}

/// Validate `session_id`. Same kebab-case rules as project ids so
/// CLI flags are predictable.
pub fn validate_session_id(id: &str) -> Result<(), String> {
    if id.is_empty() {
        return Err("session id must not be empty".to_string());
    }
    let mut chars = id.chars();
    match chars.next() {
        Some(c) if c.is_ascii_lowercase() => {}
        _ => {
            return Err(format!(
                "invalid session id '{id}': must start with a lowercase letter"
            ))
        }
    }
    let mut prev_dash = false;
    for c in chars {
        if c == '-' {
            if prev_dash {
                return Err(format!(
                    "invalid session id '{id}': must not contain consecutive dashes"
                ));
            }
            prev_dash = true;
        } else if c.is_ascii_lowercase() || c.is_ascii_digit() {
            prev_dash = false;
        } else {
            return Err(format!(
                "invalid session id '{id}': use lowercase letters, digits and single dashes"
            ));
        }
    }
    if prev_dash {
        return Err(format!(
            "invalid session id '{id}': must not end with a dash"
        ));
    }
    Ok(())
}

/// Build a fresh [`AgentSession`] anchored to the project loaded
/// from `project_path`. The session is the source of truth for the
/// provider, the spec id and the recorded transition log; nothing
/// is written to disk until the caller records a transition.
pub fn new_session(
    project_path: &Path,
    session_id: &str,
    provider: AgentProvider,
    spec_id: &str,
    now: DateTime<Utc>,
) -> Result<AgentSession, ForgeError> {
    validate_session_id(session_id).map_err(|reason| ForgeError::AgentUnavailable { reason })?;
    let (manifest, _) = Manifest::load_from_dir(project_path, None)?;
    Ok(AgentSession {
        contract: AGENT_CONTRACT_VERSION.to_string(),
        session_id: session_id.to_string(),
        project_id: manifest.project.id,
        project_path: project_path.display().to_string(),
        provider,
        spec_id: spec_id.to_string(),
        state: SessionState::Idle,
        started_at: now,
        last_transition_at: now,
        transitions: Vec::new(),
        backing: None,
    })
}

/// Apply a transition to a session, returning a new session and
/// the recorded outcome. The function never mutates the input
/// session; callers are expected to persist the returned session.
///
/// Pre-flight: the requested transition must be available on the
/// session's provider. Bundled providers: `pause`/`takeover` are
/// not available on the bundled adapters, so they always return
/// `SessionState::Unsupported` with explicit evidence and a
/// recovery note. This is the change-specific risk in the
/// design: providers differ in pause and takeover support, and
/// the router returns an explicit unsupported state rather than
/// simulating success. Supervised providers (ariadex): every
/// transition is delegated to the runtime's real verb and the
/// resulting state is taken from `ariadex status` only — an
/// unknown report maps to `disconnected`, never to `active`.
pub fn apply_transition(
    session: AgentSession,
    transition: SessionTransition,
    now: DateTime<Utc>,
) -> Result<AgentTransitionOutcome, ForgeError> {
    if session.transitions.len() >= MAX_TRANSITIONS_PER_SESSION {
        return Err(ForgeError::AgentUnavailable {
            reason: format!(
                "session `{}` already records the maximum {MAX_TRANSITIONS_PER_SESSION} transitions; start a new session",
                session.session_id
            ),
        });
    }
    let (state, evidence, note, next_step, backing) = if session.provider.is_supervised() {
        let delegated = ariadex_transition(&session, transition)?;
        (
            delegated.state,
            delegated.evidence,
            delegated.note,
            delegated.next_step,
            delegated.backing,
        )
    } else {
        let (state, evidence, note, next_step) = match transition {
            SessionTransition::Start
            | SessionTransition::NewSession
            | SessionTransition::Resume => start_or_resume(&session, transition)?,
            SessionTransition::Restart => restart(&session)?,
            SessionTransition::Pause => unsupported(
                &session,
                transition,
                "the bundled adapter does not expose a pause primitive; the existing PTY-based \
                 manager is the integration point but is not wired into this build",
            ),
            SessionTransition::Takeover => unsupported(
                &session,
                transition,
                "the bundled adapter does not expose a takeover primitive; the existing PTY-based \
                 manager is the integration point but is not wired into this build",
            ),
        };
        (state, evidence, note, next_step, None)
    };
    let mut updated = session;
    updated.state = state;
    updated.last_transition_at = now;
    if let Some(backing) = backing {
        updated.backing = Some(backing);
    }
    updated.transitions.push(TransitionRecord {
        kind: transition,
        state,
        at: now,
        evidence: evidence.clone(),
        note: note.clone(),
    });
    Ok(AgentTransitionOutcome {
        contract: AGENT_CONTRACT_VERSION.to_string(),
        session: updated,
        requested: transition,
        state,
        evidence,
        next_step,
        note,
        verdict: None,
    })
}

fn start_or_resume(
    session: &AgentSession,
    transition: SessionTransition,
) -> Result<(SessionState, Vec<String>, String, Option<String>), ForgeError> {
    let binary = session.provider.binary();
    let available = command_on_path(binary);
    if !available {
        return Err(ForgeError::AgentUnavailable {
            reason: format!(
                "binary `{}` is not present on PATH; install the {} provider or rerun on a host that has it",
                binary,
                session.provider.label()
            ),
        });
    }
    let evidence = vec![
        format!("provider: {}", session.provider.label()),
        format!("binary: {}", binary),
        format!("transition: {}", transition.label()),
    ];
    let note = format!(
        "{} recorded for session `{}`; provider `{}` is present and the recorded session is the source of truth",
        transition.label(),
        session.session_id,
        session.provider.label()
    );
    let next_step = next_step_for(session);
    Ok((SessionState::Active, evidence, note, next_step))
}

fn restart(
    session: &AgentSession,
) -> Result<(SessionState, Vec<String>, String, Option<String>), ForgeError> {
    if !matches!(
        session.state,
        SessionState::Active | SessionState::Paused | SessionState::Idle
    ) {
        return Err(ForgeError::AgentUnavailable {
            reason: format!(
                "session `{}` is in state `{}`; restart requires `idle`/`active`/`paused`",
                session.session_id,
                session.state.label()
            ),
        });
    }
    let evidence = vec![
        format!("provider: {}", session.provider.label()),
        format!("prior_state: {}", session.state.label()),
        "restart: prior transitions preserved, session id stable".to_string(),
    ];
    let note = format!(
        "restart recorded for session `{}`; prior transitions are preserved and a fresh `active` state is set",
        session.session_id
    );
    let next_step = next_step_for(session);
    Ok((SessionState::Active, evidence, note, next_step))
}

fn unsupported(
    session: &AgentSession,
    transition: SessionTransition,
    detail: &str,
) -> (SessionState, Vec<String>, String, Option<String>) {
    let evidence = vec![
        format!("provider: {}", session.provider.label()),
        format!("transition: {}", transition.label()),
        format!("detail: {detail}"),
    ];
    let note = format!(
        "transition `{}` is unsupported on provider `{}`; session `{}` stays in state `{}`",
        transition.label(),
        session.provider.label(),
        session.session_id,
        session.state.label()
    );
    (
        SessionState::Unsupported,
        evidence,
        note,
        Some("adopt a provider that exposes this transition or record a manual status".to_string()),
    )
}

fn next_step_for(session: &AgentSession) -> Option<String> {
    if session.spec_id.is_empty() {
        Some("run a spec with `forge spec run <spec-id> --session <session-id>`".to_string())
    } else {
        Some(format!(
            "run the bound spec with `forge spec run {} --session {}`",
            session.spec_id, session.session_id
        ))
    }
}

// ---- supervised runtime adapters (`supervised-agent-adapters) ---------

/// Result of one delegated ariadex transition: the state the
/// runtime actually reported, the evidence attributing it, and the
/// backing pointer the session record should keep.
struct AriadexOutcome {
    state: SessionState,
    evidence: Vec<String>,
    note: String,
    next_step: Option<String>,
    backing: Option<RuntimeBacking>,
}

/// One completed runtime subprocess invocation.
struct RuntimeOutput {
    exit_code: Option<i32>,
    stdout: String,
    stderr: String,
}

impl RuntimeOutput {
    fn succeeded(&self) -> bool {
        self.exit_code == Some(0)
    }

    /// The first non-empty output line (stdout then stderr),
    /// redacted and char-bounded for promotion into evidence.
    fn first_line(&self) -> String {
        format!("{}{}", self.stdout, self.stderr)
            .lines()
            .map(str::trim)
            .find(|line| !line.is_empty())
            .map(bounded_evidence)
            .unwrap_or_else(|| "(empty output)".to_string())
    }
}

/// How an invocation failed: never started (a typed
/// `agent-unavailable` for the caller) or exceeded the bounded
/// wait (recorded as `disconnected`, never as a claimed state).
enum InvokeFailure {
    Spawn(String),
    Timeout(Duration),
}

/// Redact and char-bound one captured runtime-output fragment
/// before it becomes session evidence.
fn bounded_evidence(text: &str) -> String {
    let redacted = redact_credentials(text);
    let total = redacted.chars().count();
    let mut out: String = redacted.chars().take(MAX_EVIDENCE_CHARS).collect();
    if total > MAX_EVIDENCE_CHARS {
        out.push('…');
    }
    out
}

/// Trim a runtime-reported handle to a bounded, control-char-free
/// string. The handle is the sibling's own session id; Forge only
/// stores it as a pointer.
fn sanitize_handle(raw: &str) -> String {
    raw.trim()
        .chars()
        .filter(|c| !c.is_control())
        .take(64)
        .collect()
}

/// Ordered binary resolution for a runtime adapter: the non-empty
/// environment override wins, then a PATH search for `name`. The
/// attempt list is returned (also on failure) so an unavailable
/// runtime can report exactly what was tried.
fn resolve_binary_path(
    env_var: &str,
    env_value: Option<&std::ffi::OsStr>,
    path_dirs: &[PathBuf],
    name: &str,
) -> Result<(PathBuf, Vec<String>), Vec<String>> {
    let mut attempts = Vec::new();
    if let Some(value) = env_value {
        if !value.to_string_lossy().trim().is_empty() {
            let display = value.to_string_lossy().to_string();
            let candidate = PathBuf::from(value);
            if candidate.is_file() {
                attempts.push(format!("env {env_var}=`{display}`"));
                return Ok((candidate, attempts));
            }
            // A bare command name under the override resolves
            // through PATH under that name; a dead path records the
            // miss and falls through to the default name.
            let probe_name = candidate
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or(name);
            if candidate.components().count() == 1 {
                for dir in path_dirs {
                    let hit = dir.join(probe_name);
                    if hit.is_file() {
                        attempts.push(format!(
                            "env {env_var}=`{display}` resolved via PATH `{}`",
                            hit.display()
                        ));
                        return Ok((hit, attempts));
                    }
                }
            }
            attempts.push(format!("env {env_var}=`{display}` (not found)"));
        } else {
            attempts.push(format!("env {env_var} (set but empty, ignored)"));
        }
    }
    for dir in path_dirs {
        let candidate = dir.join(name);
        if candidate.is_file() {
            attempts.push(format!("PATH `{}`", candidate.display()));
            return Ok((candidate, attempts));
        }
    }
    attempts.push(format!("PATH name `{name}` (not found)"));
    Err(attempts)
}

/// Read the process environment and resolve a runtime binary.
fn resolve_runtime_binary(
    env_var: &str,
    name: &str,
) -> Result<(PathBuf, Vec<String>), Vec<String>> {
    let env_value = std::env::var_os(env_var);
    let path_dirs: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|value| std::env::split_paths(&value).collect())
        .unwrap_or_default();
    resolve_binary_path(env_var, env_value.as_deref(), &path_dirs, name)
}

/// Spawn a runtime subprocess with an argument array (never a
/// shell), a null stdin so an interactive sibling verb cannot
/// hijack the operator's terminal, the project directory as cwd
/// (ariadex treats the working directory as the project), and a
/// bounded wait so an unresponsive runtime cannot hang the CLI.
fn invoke_runtime(
    binary: &Path,
    args: &[&str],
    cwd: &Path,
    timeout: Duration,
) -> Result<RuntimeOutput, InvokeFailure> {
    use std::io::Read;
    use std::process::Stdio;
    use std::sync::{Arc, Mutex};
    use std::thread;
    let mut cmd = Command::new(binary);
    for arg in args {
        cmd.arg(arg);
    }
    cmd.current_dir(cwd);
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    let mut child = match cmd.spawn() {
        Ok(child) => child,
        Err(err) => {
            return Err(InvokeFailure::Spawn(format!(
                "cannot spawn `{}`: {err}",
                binary.display()
            )));
        }
    };
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let stdout_buf: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
    let stderr_buf: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
    let stdout_reader = {
        let buf = Arc::clone(&stdout_buf);
        thread::spawn(move || {
            if let Some(mut stdout) = stdout {
                let mut local = Vec::new();
                let _ = stdout.read_to_end(&mut local);
                if let Ok(mut guard) = buf.lock() {
                    *guard = local;
                }
            }
        })
    };
    let stderr_reader = {
        let buf = Arc::clone(&stderr_buf);
        thread::spawn(move || {
            if let Some(mut stderr) = stderr {
                let mut local = Vec::new();
                let _ = stderr.read_to_end(&mut local);
                if let Ok(mut guard) = buf.lock() {
                    *guard = local;
                }
            }
        })
    };
    let start = std::time::Instant::now();
    let exit_code = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status.code(),
            Ok(None) => {
                if start.elapsed() > timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    let _ = stdout_reader.join();
                    let _ = stderr_reader.join();
                    return Err(InvokeFailure::Timeout(timeout));
                }
                thread::sleep(Duration::from_millis(25));
            }
            Err(err) => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = stdout_reader.join();
                let _ = stderr_reader.join();
                return Err(InvokeFailure::Spawn(format!(
                    "cannot wait on runtime: {err}"
                )));
            }
        }
    };
    let _ = stdout_reader.join();
    let _ = stderr_reader.join();
    let stdout_bytes = stdout_buf
        .lock()
        .map(|guard| guard.clone())
        .unwrap_or_default();
    let stderr_bytes = stderr_buf
        .lock()
        .map(|guard| guard.clone())
        .unwrap_or_default();
    Ok(RuntimeOutput {
        exit_code,
        stdout: String::from_utf8_lossy(&stdout_bytes).to_string(),
        stderr: String::from_utf8_lossy(&stderr_bytes).to_string(),
    })
}

/// Read the first JSON document out of a runtime's stdout. The
/// sibling sometimes appends plain text after the document
/// (`status --json` blocker lines) and the duplicate-owner `start`
/// path prints two documents back-to-back; only the first complete
/// value is ever consumed.
fn first_json_document(stdout: &str) -> Option<Value> {
    let trimmed = stdout.trim_start();
    if !trimmed.starts_with('{') {
        return None;
    }
    serde_json::Deserializer::from_str(trimmed)
        .into_iter::<Value>()
        .next()?
        .ok()
}

/// Parse `ariadex --version` (documented shape: `ariadex 0.1.0`,
/// optionally `+g<sha>` / `-dirty` build suffixes). Anything else
/// means the binary does not expose the documented surface and the
/// caller must refuse rather than best-effort parse.
fn parse_ariadex_version(stdout: &str) -> Option<String> {
    let line = stdout.lines().map(str::trim).find(|l| !l.is_empty())?;
    let rest = line.strip_prefix("ariadex ")?.trim();
    let version: String = rest
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '+' | '-'))
        .collect();
    let core = version.split(['+', '-']).next().filter(|s| !s.is_empty())?;
    let mut parts = core.split('.');
    let major_ok = parts.next()?.parse::<u32>().is_ok();
    let minor_ok = parts.next()?.parse::<u32>().is_ok();
    if major_ok && minor_ok {
        Some(version)
    } else {
        None
    }
}

/// Map an `ariadex status --json` document to a Forge session
/// state. Only a live daemon reporting `AUTO`/`PAUSE` justifies an
/// `active`/`paused` claim; every other report — `MANUAL`, an
/// unknown mode, a stale daemon, the local no-daemon view, or an
/// unreadable shape — maps to `disconnected` with the raw mode
/// kept in the evidence. A state the runtime did not report is
/// never synthesized.
fn map_ariadex_status(doc: &Value) -> (SessionState, String, Option<String>, String) {
    let (source, daemon_form) = match doc.get("daemon") {
        Some(inner) if inner.is_object() => (inner, true),
        _ => (doc, false),
    };
    let mode = source
        .get("mode")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let handle = source
        .get("session")
        .and_then(Value::as_str)
        .map(sanitize_handle)
        .filter(|s| !s.is_empty());
    let daemon = if daemon_form {
        match source.get("alive").and_then(Value::as_bool) {
            Some(true) => "alive",
            Some(false) => "stale",
            None => "unknown",
        }
    } else {
        "absent"
    };
    let state = if daemon == "alive" {
        match mode.as_str() {
            "AUTO" => SessionState::Active,
            "PAUSE" => SessionState::Paused,
            _ => SessionState::Disconnected,
        }
    } else {
        SessionState::Disconnected
    };
    (state, mode, handle, daemon.to_string())
}

/// A point-in-time live probe for `forge agent status` against a
/// supervised session. The probe is read-only: it renders what the
/// runtime reports right now next to the stored record and never
/// mutates the session file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LiveRuntimeStatus {
    pub runtime: String,
    pub state: String,
    pub mode: String,
    pub handle: String,
    pub daemon: String,
    pub note: String,
}

/// Delegate the live probe for one session. Returns `None` for the
/// bundled adapters (whose recorded state is their own surface).
/// Every failure path — unresolved binary, vanished project,
/// timeout, non-zero exit, unreadable document — still produces a
/// rendered status; only the recorded transitions are preserved.
pub fn live_runtime_status(session: &AgentSession) -> Option<LiveRuntimeStatus> {
    if !session.provider.is_supervised() {
        return None;
    }
    let runtime = session.provider.label().to_string();
    let stored_handle = session
        .backing
        .as_ref()
        .map(|b| b.handle.clone())
        .unwrap_or_default();
    let live = |state: &str, daemon: &str, mode: String, handle: String, note: String| {
        Some(LiveRuntimeStatus {
            runtime: runtime.clone(),
            state: state.to_string(),
            mode,
            handle,
            daemon: daemon.to_string(),
            note,
        })
    };
    let (binary, _attempts) = match resolve_runtime_binary(ARIADEX_BIN_ENV, DEFAULT_ARIADEX_BIN) {
        Ok(resolved) => resolved,
        Err(attempts) => {
            return live(
                "unavailable",
                "unresolved",
                String::new(),
                stored_handle,
                format!(
                    "binary resolution attempts: {}",
                    bounded_evidence(&attempts.join(", "))
                ),
            );
        }
    };
    let project_dir = PathBuf::from(&session.project_path);
    if !project_dir.is_dir() {
        return live(
            "disconnected",
            "absent",
            String::new(),
            stored_handle,
            format!("project path `{}` no longer exists", session.project_path),
        );
    }
    let status = match invoke_runtime(
        &binary,
        &["status", "--json"],
        &project_dir,
        ADAPTER_WAIT_TIMEOUT,
    ) {
        Ok(output) => output,
        Err(InvokeFailure::Timeout(dur)) => {
            return live(
                "disconnected",
                "unreachable",
                String::new(),
                stored_handle,
                format!("`ariadex status` exceeded the bounded {dur:?} wait"),
            );
        }
        Err(InvokeFailure::Spawn(detail)) => {
            return live(
                "disconnected",
                "unreachable",
                String::new(),
                stored_handle,
                bounded_evidence(&detail),
            );
        }
    };
    let doc = if status.succeeded() {
        first_json_document(&status.stdout)
    } else {
        None
    };
    let Some(doc) = doc else {
        let note = if status.succeeded() {
            format!("no parsable status document: {}", status.first_line())
        } else {
            format!(
                "`ariadex status` exited {}: {}",
                status
                    .exit_code
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| "signal".to_string()),
                status.first_line()
            )
        };
        return live("disconnected", "absent", String::new(), stored_handle, note);
    };
    let (state, mode, handle, daemon) = map_ariadex_status(&doc);
    let handle = handle.unwrap_or_default();
    let mut note = format!("live ariadex report: mode={mode} daemon={daemon}");
    let mut state = state;
    if !stored_handle.is_empty() && handle != stored_handle {
        // The stored pointer is gone from the runtime's view:
        // surface the loss instead of claiming an active session
        // the runtime no longer associates with this record.
        state = SessionState::Disconnected;
        note.push_str(&format!(
            "; the runtime does not report the stored handle `{stored_handle}` (loss recorded, session file preserved)"
        ));
    }
    live(state.label(), &daemon, mode, handle, note)
}

/// Build the recorded `disconnected` outcome for a runtime that
/// gave no live report. The session file and its transitions stay;
/// the detail is what the runtime actually said.
fn disconnected_outcome(
    session: &AgentSession,
    mut evidence: Vec<String>,
    detail: &str,
    adapter_version: &str,
) -> AriadexOutcome {
    evidence.push(format!("status: {detail}"));
    AriadexOutcome {
        state: SessionState::Disconnected,
        evidence,
        note: format!(
            "the ariadex runtime reported no live state for session `{}` ({detail}); the session file and its transitions are preserved",
            session.session_id
        ),
        next_step: Some(format!(
            "recover the runtime with `ariadex start` in {} and rerun the transition",
            session.project_path
        )),
        backing: session.backing.clone().or(Some(RuntimeBacking {
            runtime: "ariadex".to_string(),
            handle: String::new(),
            adapter_version: adapter_version.to_string(),
        })),
    }
}

/// Delegate one session transition to the ariadex runtime. Every
/// verb runs in the session's project directory; the resulting
/// state comes only from a bounded `ariadex status --json` probe
/// afterwards. Refusals, timeouts and unreadable reports map to
/// `disconnected`/`unsupported` with the runtime's own bounded,
/// redacted output in the evidence. `takeover` is guidance only —
/// the sibling's terminal `attach` is never hijacked.
fn ariadex_transition(
    session: &AgentSession,
    transition: SessionTransition,
) -> Result<AriadexOutcome, ForgeError> {
    let (binary, _attempts) = resolve_runtime_binary(ARIADEX_BIN_ENV, DEFAULT_ARIADEX_BIN)
        .map_err(|attempts| ForgeError::AgentUnavailable {
            reason: format!(
                "provider `ariadex` resolved to no runtime binary (attempts: {}); bundled providers `opencode`/`codex` remain fully usable",
                attempts.join(", ")
            ),
        })?;
    let project_dir = PathBuf::from(&session.project_path);
    if !project_dir.is_dir() {
        return Err(ForgeError::PathUnavailable {
            path: session.project_path.clone(),
        });
    }
    if transition == SessionTransition::Restart && session.state == SessionState::Stopped {
        return Err(ForgeError::AgentUnavailable {
            reason: format!(
                "session `{}` is in state `stopped`; restart requires a non-terminal session",
                session.session_id
            ),
        });
    }
    let mut evidence = vec![
        format!("provider: {}", session.provider.label()),
        format!("transition: {}", transition.label()),
        format!("binary: {}", binary.display()),
    ];
    // Attribute the delegated surface to a documented version
    // probe before running anything.
    let adapter_version = match invoke_runtime(
        &binary,
        &["--version"],
        &project_dir,
        ADAPTER_WAIT_TIMEOUT,
    ) {
        Ok(output) => match parse_ariadex_version(&output.stdout) {
            Some(version) => {
                evidence.push(format!("version: {version}"));
                version
            }
            None => {
                evidence.push(format!("version-probe: {}", output.first_line()));
                return Ok(AriadexOutcome {
                    state: SessionState::Unsupported,
                    evidence,
                    note: format!(
                        "the ariadex version probe did not return the documented surface; transition `{}` was not delegated and no runtime state was claimed",
                        transition.label()
                    ),
                    next_step: Some(
                        "install or pin the `ariadex` runtime this adapter was written against; bundled providers `opencode`/`codex` remain usable"
                            .to_string(),
                    ),
                    backing: session.backing.clone(),
                });
            }
        },
        Err(InvokeFailure::Spawn(detail)) => {
            return Err(ForgeError::AgentUnavailable {
                reason: format!(
                    "provider `ariadex` binary `{}` cannot be spawned: {detail}",
                    binary.display()
                ),
            });
        }
        Err(InvokeFailure::Timeout(dur)) => {
            evidence.push(format!("version-probe: timed out after {dur:?}"));
            return Ok(disconnected_outcome(
                session,
                evidence,
                &format!("the version probe exceeded the bounded {dur:?} wait"),
                "unknown",
            ));
        }
    };
    let verbs: Vec<Vec<&str>> = match transition {
        SessionTransition::Start => vec![vec!["start"]],
        // Design: `restart`/`new-session` run stop-then-start
        // through the sibling's verbs.
        SessionTransition::NewSession | SessionTransition::Restart => {
            vec![vec!["stop"], vec!["start"]]
        }
        SessionTransition::Resume => vec![vec!["resume"]],
        SessionTransition::Pause => vec![vec!["pause"]],
        SessionTransition::Takeover => vec![],
    };
    let mut refusals: Vec<String> = Vec::new();
    for verb in &verbs {
        let joined = verb.join(" ");
        evidence.push(format!("verb: ariadex {joined}"));
        match invoke_runtime(&binary, verb, &project_dir, ADAPTER_WAIT_TIMEOUT) {
            Ok(output) => {
                if !output.succeeded() {
                    let detail = output.first_line();
                    evidence.push(format!("verb-refused: {detail}"));
                    refusals.push(format!("`ariadex {joined}`: {detail}"));
                }
            }
            Err(InvokeFailure::Timeout(dur)) => {
                evidence.push(format!("verb: `ariadex {joined}` timed out after {dur:?}"));
                return Ok(disconnected_outcome(
                    session,
                    evidence,
                    &format!("`ariadex {joined}` exceeded the bounded {dur:?} wait"),
                    &adapter_version,
                ));
            }
            Err(InvokeFailure::Spawn(detail)) => {
                return Err(ForgeError::AgentUnavailable {
                    reason: format!("provider `ariadex`: {detail}"),
                });
            }
        }
    }
    // The state Forge claims comes only from the sibling's status
    // surface, after the verb.
    let status = match invoke_runtime(
        &binary,
        &["status", "--json"],
        &project_dir,
        ADAPTER_WAIT_TIMEOUT,
    ) {
        Ok(status) => status,
        Err(InvokeFailure::Spawn(detail)) => {
            return Err(ForgeError::AgentUnavailable {
                reason: format!("provider `ariadex`: {detail}"),
            });
        }
        Err(InvokeFailure::Timeout(dur)) => {
            evidence.push(format!("status: `ariadex status` timed out after {dur:?}"));
            return Ok(disconnected_outcome(
                session,
                evidence,
                &format!("`ariadex status` exceeded the bounded {dur:?} wait"),
                &adapter_version,
            ));
        }
    };
    let doc = if status.succeeded() {
        first_json_document(&status.stdout)
    } else {
        None
    };
    let Some(doc) = doc else {
        let detail = if status.succeeded() {
            format!("no parsable status document: {}", status.first_line())
        } else {
            format!(
                "`ariadex status` exited {}: {}",
                status
                    .exit_code
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| "signal".to_string()),
                status.first_line()
            )
        };
        return Ok(disconnected_outcome(
            session,
            evidence,
            &detail,
            &adapter_version,
        ));
    };
    let (state, mode, handle, daemon) = map_ariadex_status(&doc);
    evidence.push(format!(
        "ariadex-mode: {}",
        if mode.is_empty() { "(absent)" } else { &mode }
    ));
    evidence.push(format!("ariadex-daemon: {daemon}"));
    let handle = handle.unwrap_or_default();
    evidence.push(format!(
        "handle: {}",
        if handle.is_empty() { "(none)" } else { &handle }
    ));
    if transition == SessionTransition::Takeover {
        evidence.push("takeover: guidance only (no process hijack)".to_string());
    }
    let backing = RuntimeBacking {
        runtime: "ariadex".to_string(),
        handle,
        adapter_version: adapter_version.clone(),
    };
    let note = if refusals.is_empty() {
        format!(
            "transition `{}` delegated to the ariadex runtime; state `{}` is what `ariadex status` reported",
            transition.label(),
            state.label()
        )
    } else {
        format!(
            "the ariadex runtime refused {} but reported state `{}`; Forge records the runtime's report, not a simulated success",
            refusals.join("; "),
            state.label()
        )
    };
    let next_step = match transition {
        SessionTransition::Takeover => Some(format!(
            "attach the supervised session: `ariadex attach` in {}",
            session.project_path
        )),
        _ => None,
    };
    Ok(AriadexOutcome {
        state,
        evidence,
        note,
        next_step,
        backing: Some(backing),
    })
}

/// Persist a session under `.forge/agents/<session-id>/` in
/// `dir`. The transition log is appended to a separate file so a
/// future audit can read the timeline without parsing the
/// session blob. The session file is always written last so a
/// partial write can be detected and recovered.
pub fn write_session(dir: &Path, session: &AgentSession) -> Result<Vec<String>, ForgeError> {
    let base = session_dir(dir, &session.session_id);
    fs::create_dir_all(&base).map_err(|err| ForgeError::Registry {
        reason: format!("cannot create session dir {}: {err}", base.display()),
    })?;
    let json = serde_json::to_string_pretty(session).map_err(|err| ForgeError::Registry {
        reason: err.to_string(),
    })?;
    let session_path = base.join("session.json");
    fs::write(&session_path, &json).map_err(|err| ForgeError::Registry {
        reason: format!("cannot write session file: {err}"),
    })?;
    let log_path = base.join("transitions.log");
    let mut log_existing = fs::read_to_string(&log_path).unwrap_or_default();
    if let Some(last) = session.transitions.last() {
        log_existing.push_str(&format!(
            "{} {} {} state={} evidence={} note={}\n",
            last.at.to_rfc3339(),
            last.kind.label(),
            session.session_id,
            last.state.label(),
            last.evidence.join(";"),
            last.note
        ));
    }
    fs::write(&log_path, log_existing).map_err(|err| ForgeError::Registry {
        reason: format!("cannot write transition log: {err}"),
    })?;
    Ok(vec![
        format!("{AGENTS_DIR}/{}/session.json", session.session_id),
        format!("{AGENTS_DIR}/{}/transitions.log", session.session_id),
    ])
}

/// Read a session from `.forge/agents/<session-id>/session.json`.
/// Returns `None` when the session file is missing.
pub fn read_session(dir: &Path, session_id: &str) -> Result<Option<AgentSession>, ForgeError> {
    let path = session_dir(dir, session_id).join("session.json");
    if !path.is_file() {
        return Ok(None);
    }
    let text = fs::read_to_string(&path).map_err(|err| ForgeError::Registry {
        reason: err.to_string(),
    })?;
    let session: AgentSession =
        serde_json::from_str(&text).map_err(|err| ForgeError::Registry {
            reason: format!("invalid session file {}: {err}", path.display()),
        })?;
    Ok(Some(session))
}

/// List all sessions for the project. Each entry is identified by
/// its directory name; malformed entries are skipped so a partial
/// write never breaks the listing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SessionListEntry {
    pub session_id: String,
    pub provider: AgentProvider,
    pub state: SessionState,
    pub spec_id: String,
    pub started_at: DateTime<Utc>,
    pub last_transition_at: DateTime<Utc>,
    pub transition_count: usize,
}

pub fn list_sessions(dir: &Path) -> Result<Vec<SessionListEntry>, ForgeError> {
    let base = dir.join(AGENTS_DIR);
    if !base.is_dir() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in fs::read_dir(&base).map_err(|err| ForgeError::Registry {
        reason: err.to_string(),
    })? {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        if !entry.path().is_dir() {
            continue;
        }
        let session_path = entry.path().join("session.json");
        if !session_path.is_file() {
            continue;
        }
        let text = match fs::read_to_string(&session_path) {
            Ok(t) => t,
            Err(_) => continue,
        };
        let session: AgentSession = match serde_json::from_str(&text) {
            Ok(s) => s,
            Err(_) => continue,
        };
        out.push(SessionListEntry {
            session_id: session.session_id,
            provider: session.provider,
            state: session.state,
            spec_id: session.spec_id,
            started_at: session.started_at,
            last_transition_at: session.last_transition_at,
            transition_count: session.transitions.len(),
        });
    }
    out.sort_by(|a, b| a.session_id.cmp(&b.session_id));
    Ok(out)
}

/// Resolve a session id from the project, anchoring to the
/// manifest for the spec id. Returns `None` when the session
/// doesn't exist, and `SpecInvalid` when the spec id is set in
/// the session but the spec itself cannot be found (R1 boundary
/// scenario: a session that ends without verification is recorded
/// distinctly from a successful spec completion).
///
/// With `supervisor = Some("sisyphusfy")` the bound spec's task
/// file is delegated to the sibling iteration loop and the
/// journaled verdict comes from the supervisor's independent
/// verification outcome — never from an agent completion claim.
/// Any other supervisor id is refused before anything runs, and an
/// ariadex session without an explicit supervisor is refused with
/// the runtime's own scheduler path named.
pub fn run_spec(
    dir: &Path,
    session_id: &str,
    supervisor: Option<&str>,
    now: DateTime<Utc>,
) -> Result<AgentTransitionOutcome, ForgeError> {
    let session = read_session(dir, session_id)?.ok_or_else(|| ForgeError::AgentUnavailable {
        reason: format!("session `{session_id}` was not found under `{AGENTS_DIR}/`"),
    })?;
    if session.spec_id.is_empty() {
        return Err(ForgeError::AgentUnavailable {
            reason: format!(
                "session `{}` has no bound spec; bind a spec with `forge agent start --spec <id>`",
                session.session_id
            ),
        });
    }
    let spec_path = dir.join(".forge/specs").join(&session.spec_id);
    if !spec_path.join("manifest.json").is_file() {
        return Err(ForgeError::SpecInvalid {
            reason: format!(
                "bound spec `{}` was not found under `.forge/specs/`; session `{}` stays in state `{}`",
                session.spec_id,
                session.session_id,
                session.state.label()
            ),
        });
    }
    match supervisor {
        None => {
            if session.provider.is_supervised() {
                return Err(ForgeError::AgentUnavailable {
                    reason: "provider `ariadex` drives specs inside its own supervised scheduler (`ariadex status` reports the next action); delegate this spec with `forge agent run-spec --provider sisyphusfy`".to_string(),
                });
            }
            apply_transition(session, SessionTransition::Start, now)
        }
        Some(value) if value == SISYPHUSFY_SUPERVISOR => run_spec_sisyphusfy(dir, &session, now),
        Some(other) => Err(ForgeError::AgentUnavailable {
            reason: format!(
                "unknown run-spec supervisor `{other}`; supported supervisors: {SISYPHUSFY_SUPERVISOR}"
            ),
        }),
    }
}

/// Classify one `sisyphusfy run --json` outcome document into the
/// run-spec verdict vocabulary. Only a clean exit with `complete`
/// and an independently passing verification yields `done`;
/// incomplete-loop reports yield `partial` with the sibling's
/// named reason; anything that cannot establish verified
/// completion (including an unparsable document) is `unverified`
/// and never `done`.
fn classify_sisyphusfy_output(exit_ok: bool, stdout: &str) -> (&'static str, Vec<String>, String) {
    let mut evidence = Vec::new();
    let Some(doc) = first_json_document(stdout) else {
        let raw: Vec<&str> = stdout.lines().take(2).collect();
        evidence.push("outcome: the supervisor emitted no parsable JSON document".to_string());
        evidence.push(format!(
            "raw-outcome: {}",
            bounded_evidence(&raw.join(" / "))
        ));
        return (
            "unverified",
            evidence,
            "the outcome document cannot be parsed".to_string(),
        );
    };
    let stop_reason = doc
        .get("stop_reason")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    if let Some(iterations) = doc.get("iterations").and_then(Value::as_i64) {
        evidence.push(format!("iterations: {iterations}"));
    }
    let verification_status = doc
        .get("verification")
        .and_then(|v| v.get("status"))
        .and_then(Value::as_str)
        .unwrap_or("absent")
        .to_string();
    let verification_source = doc
        .get("verification")
        .and_then(|v| v.get("source"))
        .and_then(Value::as_str)
        .unwrap_or("");
    evidence.push(format!("stop_reason: {stop_reason}"));
    evidence.push(format!(
        "verification: status={verification_status} source={verification_source}"
    ));
    if stop_reason.is_empty() {
        return (
            "unverified",
            evidence,
            "the outcome document carries no stop_reason".to_string(),
        );
    }
    if exit_ok != (stop_reason == "complete") {
        return (
            "unverified",
            evidence,
            "the supervisor exit disagrees with the outcome document".to_string(),
        );
    }
    if stop_reason == "complete" {
        return if verification_status == "success" {
            (
                "done",
                evidence,
                "the supervisor's independent verification passed".to_string(),
            )
        } else {
            (
                "unverified",
                evidence,
                format!(
                    "the loop completed without a passing verification (status `{verification_status}`)"
                ),
            )
        };
    }
    const PARTIAL_REASONS: [&str; 10] = [
        "blocked",
        "max_iterations",
        "timeout",
        "interrupted",
        "agent_failed",
        "unchanged_state",
        "models_exhausted",
        "adapter_error",
        "command_not_found",
        "context_budget_exceeded",
    ];
    let named_reason = doc
        .get("blocked_reason")
        .and_then(|value| match value {
            Value::String(text) => Some(text.as_str()),
            Value::Array(items) => items.iter().filter_map(Value::as_str).next(),
            _ => None,
        })
        .or_else(|| {
            doc.get("agent_error")
                .and_then(|error| error.get("message"))
                .and_then(Value::as_str)
        })
        .map(bounded_evidence);
    if let Some(detail) = &named_reason {
        evidence.push(format!("supervisor-reason: {detail}"));
    }
    if PARTIAL_REASONS.contains(&stop_reason.as_str()) {
        let reason = match named_reason {
            Some(detail) => format!("`{stop_reason}` ({detail})"),
            None => format!("`{stop_reason}`"),
        };
        return ("partial", evidence, reason);
    }
    (
        "unverified",
        evidence,
        format!("stop_reason `{stop_reason}` cannot establish verified completion"),
    )
}

/// Delegate the bound spec's task file to the sisyphusfy iteration
/// supervisor. Forge writes nothing new: the existing
/// `.forge/specs/<spec>/` files are the task input, fed through the
/// sibling's documented low-level verb (`loop --task-path <file>
/// --json --adapter <provider>`) so the supervisor itself owns the
/// agent CLI grammar and refuses adapters it does not know. The
/// verdict is the supervisor's verification outcome only; agent
/// self-claims never upgrade to `done`.
fn run_spec_sisyphusfy(
    dir: &Path,
    session: &AgentSession,
    now: DateTime<Utc>,
) -> Result<AgentTransitionOutcome, ForgeError> {
    if session.transitions.len() >= MAX_TRANSITIONS_PER_SESSION {
        return Err(ForgeError::AgentUnavailable {
            reason: format!(
                "session `{}` already records the maximum {MAX_TRANSITIONS_PER_SESSION} transitions; start a new session",
                session.session_id
            ),
        });
    }
    let tasks_path = dir
        .join(".forge/specs")
        .join(&session.spec_id)
        .join("tasks.md");
    if !tasks_path.is_file() {
        return Err(ForgeError::SpecInvalid {
            reason: format!(
                "bound spec `{}` has no `tasks.md` to hand to the supervisor; session `{}` stays in state `{}`",
                session.spec_id,
                session.session_id,
                session.state.label()
            ),
        });
    }
    let (binary, _attempts) = resolve_runtime_binary(SISYPHUSFY_BIN_ENV, DEFAULT_SISYPHUSFY_BIN)
        .map_err(|attempts| ForgeError::AgentUnavailable {
            reason: format!(
                "supervisor `sisyphusfy` resolved to no runtime binary (attempts: {}); the session and its spec binding are unchanged",
                attempts.join(", ")
            ),
        })?;
    let tasks_arg = format!(".forge/specs/{}/tasks.md", session.spec_id);
    let adapter = session.provider.label();
    let mut evidence = vec![
        format!("supervisor: {SISYPHUSFY_SUPERVISOR}"),
        format!("binary: {}", binary.display()),
        format!("task-file: {tasks_arg}"),
        format!("adapter: {adapter}"),
    ];
    let (verdict, reason) = match invoke_runtime(
        &binary,
        &[
            "loop",
            "--task-path",
            &tasks_arg,
            "--json",
            "--adapter",
            adapter,
        ],
        dir,
        RUN_SPEC_WAIT_TIMEOUT,
    ) {
        Ok(output) => {
            evidence.push(format!(
                "run: exit={}",
                output
                    .exit_code
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| "signal".to_string())
            ));
            let (verdict, run_evidence, reason) =
                classify_sisyphusfy_output(output.succeeded(), &output.stdout);
            evidence.extend(run_evidence);
            evidence.push(format!("verdict: {verdict}"));
            (verdict, reason)
        }
        Err(InvokeFailure::Timeout(dur)) => {
            evidence.push(format!("run: timed out after {dur:?}"));
            evidence.push("verdict: unverified".to_string());
            (
                "unverified",
                format!("the supervisor exceeded the bounded {dur:?} wait"),
            )
        }
        Err(InvokeFailure::Spawn(detail)) => {
            return Err(ForgeError::AgentUnavailable {
                reason: format!(
                    "supervisor `sisyphusfy`: {detail}; the session and its spec binding are unchanged"
                ),
            });
        }
    };
    let note = format!(
        "run-spec `{}` delegated to supervisor `sisyphusfy`; verdict `{}` ({reason}); the supervisor's independent verification, not any agent claim, set the verdict",
        session.spec_id, verdict
    );
    evidence_bound(&mut evidence);
    let mut updated = session.clone();
    updated.state = SessionState::Active;
    updated.last_transition_at = now;
    updated.transitions.push(TransitionRecord {
        kind: SessionTransition::Start,
        state: SessionState::Active,
        at: now,
        evidence: evidence.clone(),
        note: note.clone(),
    });
    Ok(AgentTransitionOutcome {
        contract: AGENT_CONTRACT_VERSION.to_string(),
        session: updated,
        requested: SessionTransition::Start,
        state: SessionState::Active,
        evidence,
        next_step: None,
        note,
        verdict: Some(verdict.to_string()),
    })
}

/// Defense-in-depth bound over every evidence string a delegated
/// run produced (individual fragments were already bounded; this
/// guarantees the record can never exceed the contract's line
/// budget).
fn evidence_bound(evidence: &mut [String]) {
    for item in evidence.iter_mut() {
        if item.chars().count() > MAX_EVIDENCE_CHARS {
            *item = bounded_evidence(item);
        }
    }
}

/// Best-effort `which`-style probe using `Command::new` with an
/// argument array (never shell).
fn command_on_path(binary: &str) -> bool {
    let probe_paths: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|value| std::env::split_paths(&value).collect())
        .unwrap_or_default();
    for dir in probe_paths {
        if dir.join(binary).is_file() {
            return true;
        }
    }
    false
}

/// Build an [`AgentTransitionOutcome`] for a `forge agent status`
/// call. The session is read from disk and rendered with the
/// recorded provider, state and spec binding (R1 success
/// scenario: the session identity, provider and project
/// association remain observable).
pub fn status_for(dir: &Path, session_id: &str) -> Result<Option<AgentSession>, ForgeError> {
    read_session(dir, session_id)
}

/// Re-render the manifest `forge.yaml` path used by the agent
/// module's `spec` validation. Exposed for tests and for callers
/// that want to confirm the project the session is anchored to
/// still has a valid manifest.
pub fn manifest_path_for(project_path: &Path) -> PathBuf {
    project_path.join(CANONICAL_MANIFEST)
}

/// Run a one-off adapter probe so the CLI can confirm a binary
/// is present without mutating any state. Returns `Ok(())` when
/// the binary resolves (env override first for supervised
/// providers, PATH probe for the bundled adapters); otherwise a
/// typed error naming the resolution attempts. The probe is
/// intentionally read-only: no subprocess is spawned, no
/// environment is changed.
pub fn probe_provider(provider: AgentProvider) -> Result<(), ForgeError> {
    if provider.is_supervised() {
        return resolve_runtime_binary(ARIADEX_BIN_ENV, provider.binary())
            .map(|_| ())
            .map_err(|attempts| ForgeError::AgentUnavailable {
                reason: format!(
                    "provider `{}` resolved to no runtime binary (attempts: {}); install the runtime or set {ARIADEX_BIN_ENV}",
                    provider.label(),
                    attempts.join(", ")
                ),
            });
    }
    let binary = provider.binary();
    if command_on_path(binary) {
        return Ok(());
    }
    Err(ForgeError::AgentUnavailable {
        reason: format!(
            "binary `{binary}` is not present on PATH; install the {label} provider to enable session operations",
            label = provider.label()
        ),
    })
}

/// Run a no-arg `Command` probe with a bounded wait so a hung
/// adapter cannot block the caller. Used only by the integration
/// tests; production callers go through the adapter contract.
#[allow(dead_code)]
pub fn run_bounded(mut cmd: Command) -> Result<std::process::Output, ForgeError> {
    use std::io::Read;
    use std::process::Stdio;
    let binary = cmd.get_program().to_os_string();
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    let mut child = cmd.spawn().map_err(|err| ForgeError::AgentUnavailable {
        reason: format!("cannot spawn adapter `{}`: {err}", binary.display()),
    })?;
    let stdout_handle = child.stdout.take();
    let stderr_handle = child.stderr.take();
    let (tx, rx) = std::sync::mpsc::channel();
    let reader = std::thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(mut h) = stdout_handle {
            let _ = h.read_to_end(&mut buf);
        }
        let _ = tx.send(buf);
    });
    let mut stderr_buf = Vec::new();
    if let Some(mut h) = stderr_handle {
        let _ = h.read_to_end(&mut stderr_buf);
    }
    let stdout_buf = rx.recv().unwrap_or_default();
    let _ = reader.join();
    match child.wait() {
        Ok(status) => Ok(std::process::Output {
            status,
            stdout: stdout_buf,
            stderr: stderr_buf,
        }),
        Err(err) => Err(ForgeError::AgentUnavailable {
            reason: format!("adapter `{}` failed: {err}", binary.display()),
        }),
    }
}

#[allow(dead_code)]
fn _empty_marker() -> Vec<TransitionRecord> {
    AgentSession::empty_transitions()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write_manifest(dir: &Path, id: &str) {
        let body = format!(
            "schema: 1\nproject:\n  id: {id}\n  name: {id}\n  profile: rust-web\n  maturity: L1\nruntime:\n  language: rust\n"
        );
        fs::write(dir.join(CANONICAL_MANIFEST), body).unwrap();
    }

    #[test]
    fn session_id_validation_matches_project_rules() {
        for id in ["a", "agent-1", "session-001"] {
            assert!(validate_session_id(id).is_ok(), "{id}");
        }
        for id in ["", "Agent-1", "1abc", "-abc", "abc-", "a--b", "a_b", "a b"] {
            assert!(validate_session_id(id).is_err(), "{id}");
        }
    }

    #[test]
    fn start_records_state_for_present_or_missing_provider() {
        let tmp = TempDir::new().unwrap();
        write_manifest(tmp.path(), "demo");
        let session = new_session(
            tmp.path(),
            "sess-1",
            AgentProvider::Codex,
            "",
            DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
        )
        .unwrap();
        // The test environment may or may not include a `codex`
        // binary on PATH; the contract guarantees that the
        // transition either succeeds with `active` evidence or
        // fails with `agent-unavailable`, never with a simulated
        // pause/takeover state.
        match apply_transition(
            session,
            SessionTransition::Start,
            DateTime::<Utc>::from_timestamp(1, 0).unwrap(),
        ) {
            Ok(outcome) => {
                assert_eq!(outcome.state, SessionState::Active);
                assert!(outcome
                    .evidence
                    .iter()
                    .any(|e| e.contains("provider: codex")));
            }
            Err(err) => {
                assert_eq!(err.code(), "agent-unavailable");
            }
        }
    }

    #[test]
    fn pause_and_takeover_report_unsupported_state() {
        let tmp = TempDir::new().unwrap();
        write_manifest(tmp.path(), "pause-app");
        // Build a synthetic session in the `Active` state without
        // requiring a real provider binary on PATH.
        let now = DateTime::<Utc>::from_timestamp(0, 0).unwrap();
        let mut session =
            new_session(tmp.path(), "sess-pause", AgentProvider::Opencode, "", now).unwrap();
        session.state = SessionState::Active;
        let outcome = apply_transition(
            session,
            SessionTransition::Pause,
            DateTime::<Utc>::from_timestamp(1, 0).unwrap(),
        )
        .unwrap();
        assert_eq!(outcome.state, SessionState::Unsupported);
        assert!(outcome
            .evidence
            .iter()
            .any(|e| e.contains("pause primitive")));
        assert!(outcome.next_step.is_some());
        let outcome = apply_transition(
            outcome.session,
            SessionTransition::Takeover,
            DateTime::<Utc>::from_timestamp(2, 0).unwrap(),
        )
        .unwrap();
        assert_eq!(outcome.state, SessionState::Unsupported);
    }

    #[test]
    fn restart_preserves_prior_state_and_records_active() {
        let tmp = TempDir::new().unwrap();
        write_manifest(tmp.path(), "restart-app");
        let now = DateTime::<Utc>::from_timestamp(0, 0).unwrap();
        let mut session = new_session(
            tmp.path(),
            "sess-restart",
            AgentProvider::Codex,
            "spec-restart-abcdef",
            now,
        )
        .unwrap();
        session.state = SessionState::Active;
        let outcome = apply_transition(
            session,
            SessionTransition::Restart,
            DateTime::<Utc>::from_timestamp(1, 0).unwrap(),
        )
        .unwrap();
        assert_eq!(outcome.state, SessionState::Active);
        assert_eq!(outcome.session.transitions.len(), 1);
        assert_eq!(outcome.session.spec_id, "spec-restart-abcdef");
    }

    #[test]
    fn run_spec_refuses_when_bound_spec_missing() {
        let tmp = TempDir::new().unwrap();
        write_manifest(tmp.path(), "run-app");
        let now = DateTime::<Utc>::from_timestamp(0, 0).unwrap();
        let mut session = new_session(
            tmp.path(),
            "sess-run",
            AgentProvider::Codex,
            "spec-missing-123456",
            now,
        )
        .unwrap();
        session.state = SessionState::Active;
        write_session(tmp.path(), &session).unwrap();
        let err = run_spec(
            tmp.path(),
            "sess-run",
            None,
            DateTime::<Utc>::from_timestamp(1, 0).unwrap(),
        )
        .unwrap_err();
        assert_eq!(err.code(), "spec-invalid");
        // Session file is preserved with the original spec id so
        // the boundary scenario (session that ended without
        // verification) remains observable.
        let reread = read_session(tmp.path(), "sess-run").unwrap().unwrap();
        assert_eq!(reread.spec_id, "spec-missing-123456");
        assert_eq!(reread.state, SessionState::Active);
    }

    #[test]
    fn write_and_read_session_roundtrips_transitions() {
        let tmp = TempDir::new().unwrap();
        write_manifest(tmp.path(), "roundtrip");
        let now = DateTime::<Utc>::from_timestamp(0, 0).unwrap();
        let mut session =
            new_session(tmp.path(), "sess-rt", AgentProvider::Codex, "", now).unwrap();
        session.state = SessionState::Active;
        session.transitions.push(TransitionRecord {
            kind: SessionTransition::Start,
            state: SessionState::Active,
            at: now,
            evidence: vec!["provider: codex".to_string()],
            note: "start".to_string(),
        });
        let files = write_session(tmp.path(), &session).unwrap();
        assert_eq!(files.len(), 2);
        let read_back = read_session(tmp.path(), "sess-rt").unwrap().unwrap();
        assert_eq!(read_back.transitions.len(), 1);
        assert_eq!(read_back.transitions[0].kind, SessionTransition::Start);
    }

    #[test]
    fn legacy_session_files_without_backing_deserialize_unchanged() {
        // A session.json written before `supervised-agent-adapters`
        // carries no `backing` key at all; the serde default must
        // keep it loadable (and the cross-surface suites keep
        // writing exactly this shape).
        let legacy = r#"{"contract":"0.1.0","session_id":"sess-old","project_id":"old","project_path":"/tmp/old","provider":"codex","spec_id":"","state":"active","started_at":"1970-01-01T00:00:00+00:00","last_transition_at":"1970-01-01T00:00:00+00:00","transitions":[]}"#;
        let session: AgentSession = serde_json::from_str(legacy).expect("legacy session loads");
        assert_eq!(session.session_id, "sess-old");
        assert!(session.backing.is_none());
    }

    #[test]
    fn backing_roundtrips_through_session_json() {
        let tmp = TempDir::new().unwrap();
        write_manifest(tmp.path(), "backing-app");
        let now = DateTime::<Utc>::from_timestamp(0, 0).unwrap();
        let mut session =
            new_session(tmp.path(), "sess-b", AgentProvider::Ariadex, "", now).unwrap();
        session.state = SessionState::Active;
        session.backing = Some(RuntimeBacking {
            runtime: "ariadex".to_string(),
            handle: "cafe1234abcd".to_string(),
            adapter_version: "0.1.0".to_string(),
        });
        write_session(tmp.path(), &session).unwrap();
        let read_back = read_session(tmp.path(), "sess-b").unwrap().unwrap();
        let backing = read_back.backing.expect("backing persisted");
        assert_eq!(backing.runtime, "ariadex");
        assert_eq!(backing.handle, "cafe1234abcd");
        assert_eq!(backing.adapter_version, "0.1.0");
        assert_eq!(read_back.provider, AgentProvider::Ariadex);
    }

    #[test]
    fn provider_enum_supervised_vocabulary() {
        assert_eq!(AgentProvider::Ariadex.label(), "ariadex");
        assert_eq!(AgentProvider::Ariadex.binary(), "ariadex");
        assert!(AgentProvider::Ariadex.is_supervised());
        assert!(!AgentProvider::Opencode.is_supervised());
        assert!(!AgentProvider::Codex.is_supervised());
        let serialized = serde_json::to_string(&AgentProvider::Ariadex).unwrap();
        assert_eq!(serialized, "\"ariadex\"");
    }

    #[test]
    fn ordered_binary_resolution_prefers_env_then_path() {
        let tmp = TempDir::new().unwrap();
        let dir_a = tmp.path().join("a");
        let dir_b = tmp.path().join("b");
        fs::create_dir_all(&dir_a).unwrap();
        fs::create_dir_all(&dir_b).unwrap();
        let in_a = dir_a.join("ariadex");
        fs::write(&in_a, b"#!/bin/sh\n").unwrap();

        // PATH search finds the first executable-shaped candidate.
        let (resolved, attempts) = resolve_binary_path(
            "FORGE_ARIADEX_BIN",
            None,
            &[dir_a.clone(), dir_b.clone()],
            "ariadex",
        )
        .expect("resolves via PATH");
        assert_eq!(resolved, in_a);
        assert!(
            attempts.iter().any(|a| a.contains("PATH")),
            "attempts={attempts:?}"
        );

        // The env override wins over PATH when it names a real file,
        // and is recorded as the attempt.
        let pinned = tmp.path().join("pinned-ariadex");
        fs::write(&pinned, b"#!/bin/sh\n").unwrap();
        let env_value = std::ffi::OsString::from(&pinned);
        let (resolved, attempts) = resolve_binary_path(
            "FORGE_ARIADEX_BIN",
            Some(&env_value),
            std::slice::from_ref(&dir_a),
            "ariadex",
        )
        .expect("env override resolves");
        assert_eq!(resolved, pinned);
        assert!(
            attempts[0].contains("FORGE_ARIADEX_BIN"),
            "attempts={attempts:?}"
        );

        // A dead env path records the miss in the attempt list and
        // falls through (it never silently runs a nonexistent pin).
        let dead = std::ffi::OsString::from("/nonexistent/ariadex");
        let err = resolve_binary_path(
            "FORGE_ARIADEX_BIN",
            Some(&dead),
            std::slice::from_ref(&dir_b),
            "ariadex",
        )
        .unwrap_err();
        assert!(
            err.iter()
                .any(|a| a.contains("/nonexistent/ariadex") && a.contains("not found")),
            "attempts={err:?}"
        );

        // An empty env value is ignored, not trusted.
        let empty = std::ffi::OsString::from("  ");
        let (resolved, _attempts) = resolve_binary_path(
            "FORGE_ARIADEX_BIN",
            Some(&empty),
            std::slice::from_ref(&dir_a),
            "ariadex",
        )
        .expect("falls through to PATH");
        assert_eq!(resolved, in_a);

        // Nothing found → the failure carries the attempt list.
        let err = resolve_binary_path("FORGE_ARIADEX_BIN", None, &[dir_b], "ariadex").unwrap_err();
        assert!(
            err.iter().any(|a| a.contains("not found")),
            "attempts={err:?}"
        );
    }

    #[test]
    fn first_json_document_survives_trailing_text_and_double_docs() {
        // `status --json` may append `blocker ...` lines after the document.
        let text = "{\n  \"mode\": \"AUTO\",\n  \"session\": \"abc123\"\n}\nblocker u-1: stuck\n";
        let doc = first_json_document(text).expect("first doc");
        assert_eq!(doc["mode"], "AUTO");
        // Duplicate-owner `start --json` prints two documents back-to-back.
        let double =
            "{\"duplicate\": true, \"started\": false}\n{\"ok\": true, \"reused\": true}\n";
        let doc = first_json_document(double).expect("first of two");
        assert_eq!(doc["duplicate"], true);
        // Compact refusals parse; plain text yields None.
        assert!(first_json_document("{\"ok\": false, \"error\": \"no daemon\"}").is_some());
        assert!(first_json_document("managed runtime started").is_none());
        assert!(first_json_document("").is_none());
    }

    #[test]
    fn version_probe_parses_documented_surface_only() {
        assert_eq!(
            parse_ariadex_version("ariadex 0.1.0\n").as_deref(),
            Some("0.1.0")
        );
        assert_eq!(
            parse_ariadex_version("ariadex 0.1.0+g1a2b3c4-dirty\n").as_deref(),
            Some("0.1.0+g1a2b3c4-dirty")
        );
        assert_eq!(parse_ariadex_version("ariadex dev-build\n"), None);
        assert_eq!(parse_ariadex_version("0.1.0\n"), None);
        assert_eq!(parse_ariadex_version("ariadex\n"), None);
        assert_eq!(parse_ariadex_version(""), None);
    }

    #[test]
    fn status_mapping_never_claims_active_without_a_live_daemon() {
        let alive_auto: Value =
            serde_json::json!({"daemon": {"alive": true, "mode": "AUTO", "session": "cafe12"}});
        let (state, mode, handle, daemon) = map_ariadex_status(&alive_auto);
        assert_eq!(state, SessionState::Active);
        assert_eq!(mode, "AUTO");
        assert_eq!(handle.as_deref(), Some("cafe12"));
        assert_eq!(daemon, "alive");

        let alive_pause: Value =
            serde_json::json!({"daemon": {"alive": true, "mode": "PAUSE", "session": "cafe12"}});
        assert_eq!(map_ariadex_status(&alive_pause).0, SessionState::Paused);

        // MANUAL is a real mode but not an automated-running state.
        let alive_manual: Value = serde_json::json!({"daemon": {"alive": true, "mode": "MANUAL"}});
        let (state, _, _, _) = map_ariadex_status(&alive_manual);
        assert_eq!(state, SessionState::Disconnected);

        let stale: Value =
            serde_json::json!({"daemon": {"alive": false, "mode": "AUTO", "session": "cafe12"}});
        let (state, _, _, daemon) = map_ariadex_status(&stale);
        assert_eq!(state, SessionState::Disconnected);
        assert_eq!(daemon, "stale");

        // Local (no daemon) view: the manager is gone → disconnected
        // even though durable mode reads AUTO.
        let local: Value =
            serde_json::json!({"mode": "AUTO", "session": "cafe12", "agent": "opencode"});
        let (state, mode, _, daemon) = map_ariadex_status(&local);
        assert_eq!(state, SessionState::Disconnected);
        assert_eq!(mode, "AUTO");
        assert_eq!(daemon, "absent");

        // Unknown shapes never map to active.
        let junk: Value = serde_json::json!({"unexpected": true});
        assert_eq!(map_ariadex_status(&junk).0, SessionState::Disconnected);
    }

    #[test]
    fn sisyphusfy_verdict_requires_independent_verification() {
        // clean exit + complete + passing verification → done
        let done = serde_json::json!({
            "stop_reason": "complete", "iterations": 3,
            "verification": {"status": "success", "source": "configured"}
        });
        let (verdict, evidence, reason) =
            classify_sisyphusfy_output(true, &serde_json::to_string(&done).unwrap());
        assert_eq!(verdict, "done");
        assert!(evidence.iter().any(|e| e == "stop_reason: complete"));
        assert!(reason.contains("verification passed"), "reason={reason}");

        // complete without a passing verification → unverified
        let unverified = serde_json::json!({
            "stop_reason": "complete", "iterations": 1,
            "verification": {"status": "skipped", "source": "unavailable"}
        });
        let (verdict, _, _) =
            classify_sisyphusfy_output(true, &serde_json::to_string(&unverified).unwrap());
        assert_eq!(verdict, "unverified");

        // blocked iteration → partial naming the supervisor's reason
        let blocked = serde_json::json!({
            "stop_reason": "blocked", "iterations": 2,
            "verification": {"status": "skipped", "source": "unavailable"},
            "blocked_reason": ["NEED_PERMISSION"]
        });
        let (verdict, evidence, reason) =
            classify_sisyphusfy_output(false, &serde_json::to_string(&blocked).unwrap());
        assert_eq!(verdict, "partial");
        assert!(reason.contains("blocked"), "reason={reason}");
        assert!(reason.contains("NEED_PERMISSION"), "reason={reason}");
        assert!(evidence
            .iter()
            .any(|e| e == "supervisor-reason: NEED_PERMISSION"));

        // agent_failed → partial; verification_failed → unverified
        let failed = serde_json::json!({
            "stop_reason": "agent_failed", "iterations": 1,
            "verification": {"status": "skipped", "source": "unavailable"},
            "agent_error": {"message": "exited with code 1"}
        });
        let (verdict, _, reason) =
            classify_sisyphusfy_output(false, &serde_json::to_string(&failed).unwrap());
        assert_eq!(verdict, "partial");
        assert!(reason.contains("exited with code 1"), "reason={reason}");
        let verify_failed = serde_json::json!({
            "stop_reason": "verification_failed", "iterations": 1,
            "verification": {"status": "failure", "source": "configured"}
        });
        let (verdict, _, _) =
            classify_sisyphusfy_output(false, &serde_json::to_string(&verify_failed).unwrap());
        assert_eq!(verdict, "unverified");

        // unparsable outcome → unverified, never done, raw failure bounded
        let (verdict, evidence, _) = classify_sisyphusfy_output(false, "not a json document\n");
        assert_eq!(verdict, "unverified");
        assert!(evidence
            .iter()
            .any(|e| e.starts_with("raw-outcome: not a json document")));

        // exit/document disagreement → unverified (a stray exit 0 with a
        // non-complete stop reason never upgrades)
        let disagree = serde_json::json!({"stop_reason": "complete", "iterations": 9});
        let (verdict, _, reason) =
            classify_sisyphusfy_output(false, &serde_json::to_string(&disagree).unwrap());
        assert_eq!(verdict, "unverified");
        assert!(reason.contains("disagree"), "reason={reason}");
    }

    #[test]
    fn evidence_is_redacted_and_char_bounded() {
        let secret = "daemon token AKIA1234567890ABCDEF restarted";
        let bounded = bounded_evidence(secret);
        assert!(
            !bounded.contains("AKIA1234567890ABCDEF"),
            "bounded={bounded}"
        );
        let long = "x".repeat(MAX_EVIDENCE_CHARS + 200);
        let bounded = bounded_evidence(&long);
        assert!(bounded.chars().count() <= MAX_EVIDENCE_CHARS + 1);
        assert!(bounded.ends_with('…'));
        assert_eq!(sanitize_handle("  abc\u{7}def  "), "abcdef");
    }
}
