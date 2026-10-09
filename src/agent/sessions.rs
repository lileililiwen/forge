//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use crate::core::manifest::CANONICAL_MANIFEST;
use crate::core::ForgeError;
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::constants::{
    AGENTS_DIR, AGENT_CONTRACT_VERSION, ARIADEX_BIN_ENV, DEFAULT_SISYPHUSFY_BIN,
    MAX_EVIDENCE_CHARS, MAX_TRANSITIONS_PER_SESSION, RUN_SPEC_WAIT_TIMEOUT, SISYPHUSFY_BIN_ENV,
    SISYPHUSFY_SUPERVISOR,
};
use super::model::{
    AgentProvider, AgentSession, AgentTransitionOutcome, InvokeFailure, SessionListEntry,
    SessionState, SessionTransition, TransitionRecord,
};
use super::transitions::{
    apply_transition, bounded_evidence, first_json_document, invoke_runtime,
    resolve_runtime_binary, session_dir,
};

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
pub(super) fn classify_sisyphusfy_output(
    exit_ok: bool,
    stdout: &str,
) -> (&'static str, Vec<String>, String) {
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
pub(super) fn command_on_path(binary: &str) -> bool {
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
