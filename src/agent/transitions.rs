//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use crate::core::manifest::Manifest;
use crate::core::ForgeError;
use crate::policy::redact_credentials;
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use super::constants::{
    ADAPTER_WAIT_TIMEOUT, AGENTS_DIR, AGENT_CONTRACT_VERSION, ARIADEX_BIN_ENV, DEFAULT_ARIADEX_BIN,
    MAX_EVIDENCE_CHARS, MAX_TRANSITIONS_PER_SESSION,
};
use super::model::{
    AgentProvider, AgentSession, AgentTransitionOutcome, AriadexOutcome, InvokeFailure,
    LiveRuntimeStatus, RuntimeBacking, RuntimeOutput, SessionState, SessionTransition,
    TransitionRecord,
};
use super::sessions::command_on_path;

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

/// Redact and char-bound one captured runtime-output fragment
/// before it becomes session evidence.
pub(super) fn bounded_evidence(text: &str) -> String {
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
pub(super) fn sanitize_handle(raw: &str) -> String {
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
pub(super) fn resolve_binary_path(
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
pub(super) fn resolve_runtime_binary(
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
pub(super) fn invoke_runtime(
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
pub(super) fn first_json_document(stdout: &str) -> Option<Value> {
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
pub(super) fn parse_ariadex_version(stdout: &str) -> Option<String> {
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
pub(super) fn map_ariadex_status(doc: &Value) -> (SessionState, String, Option<String>, String) {
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
