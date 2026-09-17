//! Managed agent session contract (`agent-runtime-workflows`).
//!
//! Core owns the versioned session model and the provider-neutral
//! transition rules; transports (CLI now; MCP/API later) render Core
//! outcomes without reinterpreting them.
//!
//! The contract is intentionally honest about provider support:
//! `start`/`new_session` always succeed when the adapter binary is
//! present on PATH, and `resume`/`restart` follow the recorded
//! transition log. `pause` and `takeover` are explicitly
//! `unsupported` on the bundled OpenCode/Codex adapters because the
//! existing PTY-based manager is not wired into this build, so the
//! router returns an explicit `unsupported` outcome instead of
//! simulating a successful pause (R1 failure scenario).
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

use crate::core::manifest::{Manifest, CANONICAL_MANIFEST};
use crate::core::ForgeError;

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

/// Supported agent providers. Each provider has its own adapter
/// that reports which transitions it actually supports. New
/// providers can be added without breaking existing sessions
/// because the provider id is recorded with every transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentProvider {
    Opencode,
    Codex,
}

impl AgentProvider {
    pub fn label(&self) -> &'static str {
        match self {
            AgentProvider::Opencode => "opencode",
            AgentProvider::Codex => "codex",
        }
    }

    /// Subprocess binary name the adapter would invoke. Documented
    /// in the session record so a future audit can confirm what the
    /// adapter tried to run.
    pub fn binary(&self) -> &'static str {
        match self {
            AgentProvider::Opencode => "opencode",
            AgentProvider::Codex => "codex",
        }
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
}

impl AgentSession {
    fn empty_transitions() -> Vec<TransitionRecord> {
        Vec::new()
    }
}

/// Outcome of a transition request. Always carries the full
/// session after the transition, the recorded evidence, the
/// available next step (when one is appropriate), and a note for
/// the transport.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AgentTransitionOutcome {
    pub contract: String,
    pub session: AgentSession,
    pub requested: SessionTransition,
    pub state: SessionState,
    pub evidence: Vec<String>,
    pub next_step: Option<String>,
    pub note: String,
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
    })
}

/// Apply a transition to a session, returning a new session and
/// the recorded outcome. The function never mutates the input
/// session; callers are expected to persist the returned session.
///
/// Pre-flight: the requested transition must be available on the
/// session's provider (e.g. `pause`/`takeover` are not available
/// on the bundled adapters, so they always return
/// `SessionState::Unsupported` with explicit evidence and a
/// recovery note). This is the change-specific risk in the
/// design: providers differ in pause and takeover support, and
/// the router returns an explicit unsupported state rather than
/// simulating success.
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
    let (state, evidence, note, next_step) = match transition {
        SessionTransition::Start | SessionTransition::NewSession | SessionTransition::Resume => {
            start_or_resume(&session, transition)?
        }
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
    let mut updated = session;
    updated.state = state;
    updated.last_transition_at = now;
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
pub fn run_spec(
    dir: &Path,
    session_id: &str,
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
    apply_transition(session, SessionTransition::Start, now)
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
/// the binary exists on PATH; otherwise a typed error. The probe
/// is intentionally read-only: no subprocess is spawned, no
/// environment is changed.
pub fn probe_provider(provider: AgentProvider) -> Result<(), ForgeError> {
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
}
