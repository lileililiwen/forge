//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::Duration;

use super::constants::DEFAULT_ARIADEX_BIN;
use super::transitions::bounded_evidence;

/// How an invocation failed: never started (a typed
/// `agent-unavailable` for the caller) or exceeded the bounded
/// wait (recorded as `disconnected`, never as a claimed state).
pub(super) enum InvokeFailure {
    Spawn(String),
    Timeout(Duration),
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
/// One completed runtime subprocess invocation.
pub(super) struct RuntimeOutput {
    pub(super) exit_code: Option<i32>,
    pub(super) stdout: String,
    pub(super) stderr: String,
}
impl RuntimeOutput {
    pub(super) fn succeeded(&self) -> bool {
        self.exit_code == Some(0)
    }
    /// The first non-empty output line (stdout then stderr),
    /// redacted and char-bounded for promotion into evidence.
    pub(super) fn first_line(&self) -> String {
        format!("{}{}", self.stdout, self.stderr)
            .lines()
            .map(str::trim)
            .find(|line| !line.is_empty())
            .map(bounded_evidence)
            .unwrap_or_else(|| "(empty output)".to_string())
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
    pub(super) fn empty_transitions() -> Vec<TransitionRecord> {
        Vec::new()
    }
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
/// Result of one delegated ariadex transition: the state the
/// runtime actually reported, the evidence attributing it, and the
/// backing pointer the session record should keep.
pub(super) struct AriadexOutcome {
    pub(super) state: SessionState,
    pub(super) evidence: Vec<String>,
    pub(super) note: String,
    pub(super) next_step: Option<String>,
    pub(super) backing: Option<RuntimeBacking>,
}
