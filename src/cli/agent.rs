//! Agent commands (`agent`).
//!
//! Typed CLI handlers for agent session management and transitions.
//! Bodies moved verbatim from the split of `src/main.rs`.
//!
//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use forge::agent::{
    apply_transition as apply_agent_transition, list_sessions, new_session, read_session,
    run_spec as run_session_spec, status_for, AgentProvider, SessionTransition,
};
use forge::core::ForgeError;
use std::path::{Path, PathBuf};

use super::commands::AgentCommands;
use super::projects::{as_output, open_registry};
use crate::{Format, Output};

pub(super) fn parse_provider(raw: &str) -> Result<AgentProvider, ForgeError> {
    match raw {
        "opencode" => Ok(AgentProvider::Opencode),
        "codex" => Ok(AgentProvider::Codex),
        "ariadex" => Ok(AgentProvider::Ariadex),
        "sisyphusfy" => Err(ForgeError::AgentUnavailable {
            reason: "sisyphusfy is a spec-execution supervisor, not a session provider; use `forge agent run-spec --provider sisyphusfy`".to_string(),
        }),
        other => Err(ForgeError::AgentUnavailable {
            reason: format!(
                "unknown agent provider `{other}`; expected one of: opencode, codex, ariadex"
            ),
        }),
    }
}

fn resolve_agent_project(target: &str) -> Result<(PathBuf, String), ForgeError> {
    let candidate = Path::new(target);
    if candidate.is_dir() {
        let canonical = candidate
            .canonicalize()
            .map_err(|_| ForgeError::PathUnavailable {
                path: target.to_string(),
            })?;
        let (manifest, _) = forge::core::manifest::Manifest::load_from_dir(&canonical, None)?;
        return Ok((canonical, manifest.project.id));
    }
    Err(ForgeError::PathUnavailable {
        path: target.to_string(),
    })
}

pub(crate) fn cmd_agent(
    db_path: &Path,
    command: &AgentCommands,
    format: Format,
) -> Result<Output, ForgeError> {
    match command {
        AgentCommands::Start {
            target,
            session,
            provider,
            spec,
        } => {
            let (path, project_id) = resolve_agent_project(target)?;
            let provider = parse_provider(provider)?;
            let spec_id = spec.clone().unwrap_or_default();
            let now = chrono::Utc::now();
            let session_rec = new_session(&path, session, provider, &spec_id, now)?;
            let outcome = apply_agent_transition(session_rec, SessionTransition::Start, now)?;
            let files = forge::agent::write_session(&path, &outcome.session)?;
            let registry = open_registry(db_path)?;
            let backing_note = outcome
                .session
                .backing
                .as_ref()
                .map(|b| {
                    format!(
                        " backing `{}`/`{}`",
                        b.runtime,
                        if b.handle.is_empty() {
                            "(none)"
                        } else {
                            &b.handle
                        }
                    )
                })
                .unwrap_or_default();
            let detail = format!(
                "agent `{}` session `{}` provider `{}` spec `{}` -> `{}`{backing_note}",
                project_id,
                session,
                provider.label(),
                spec_id,
                outcome.state.label()
            );
            let _ = registry.record_operation("agent", &project_id, "done", &detail);
            agent_outcome_output(&outcome, &files, &detail, format)
        }
        AgentCommands::Pause { target, session } => {
            let (path, project_id) = resolve_agent_project(target)?;
            run_agent_transition(
                db_path,
                &path,
                &project_id,
                session,
                SessionTransition::Pause,
                format,
            )
        }
        AgentCommands::Takeover { target, session } => {
            let (path, project_id) = resolve_agent_project(target)?;
            run_agent_transition(
                db_path,
                &path,
                &project_id,
                session,
                SessionTransition::Takeover,
                format,
            )
        }
        AgentCommands::Resume { target, session } => {
            let (path, project_id) = resolve_agent_project(target)?;
            run_agent_transition(
                db_path,
                &path,
                &project_id,
                session,
                SessionTransition::Resume,
                format,
            )
        }
        AgentCommands::Restart { target, session } => {
            let (path, project_id) = resolve_agent_project(target)?;
            run_agent_transition(
                db_path,
                &path,
                &project_id,
                session,
                SessionTransition::Restart,
                format,
            )
        }
        AgentCommands::NewSession {
            target,
            session,
            new_session: new_id,
        } => {
            let (path, project_id) = resolve_agent_project(target)?;
            let now = chrono::Utc::now();
            let prior =
                read_session(&path, session)?.ok_or_else(|| ForgeError::AgentUnavailable {
                    reason: format!("session `{session}` was not found under `.forge/agents/`"),
                })?;
            let new_rec = new_session(&path, new_id, prior.provider, &prior.spec_id, now)?;
            let outcome = apply_agent_transition(new_rec, SessionTransition::NewSession, now)?;
            let files = forge::agent::write_session(&path, &outcome.session)?;
            let registry = open_registry(db_path)?;
            let detail = format!(
                "agent `{}` new session `{}` from `{}` -> `{}`",
                project_id,
                new_id,
                session,
                outcome.state.label()
            );
            let _ = registry.record_operation("agent", &project_id, "done", &detail);
            agent_outcome_output(&outcome, &files, &detail, format)
        }
        AgentCommands::Status { target, session } => {
            let (path, _) = resolve_agent_project(target)?;
            let session_rec =
                status_for(&path, session)?.ok_or_else(|| ForgeError::AgentUnavailable {
                    reason: format!("session `{session}` was not found under `.forge/agents/`"),
                })?;
            let mut json =
                serde_json::to_value(&session_rec).map_err(|err| ForgeError::Registry {
                    reason: err.to_string(),
                })?;
            let mut human = render_agent_session_human(&session_rec);
            if let Some(live) = forge::agent::live_runtime_status(&session_rec) {
                json["live"] = serde_json::to_value(&live).map_err(|err| ForgeError::Registry {
                    reason: err.to_string(),
                })?;
                human.push_str(&format!(
                    "\nlive: {} (daemon={} mode={} handle={} note={})",
                    live.state,
                    if live.daemon.is_empty() {
                        "(none)"
                    } else {
                        &live.daemon
                    },
                    if live.mode.is_empty() {
                        "(none)"
                    } else {
                        &live.mode
                    },
                    if live.handle.is_empty() {
                        "(none)"
                    } else {
                        &live.handle
                    },
                    live.note
                ));
            }
            Ok(as_output(format, human, json))
        }
        AgentCommands::List { target } => {
            let (path, _) = resolve_agent_project(target)?;
            let entries = list_sessions(&path)?;
            agent_list_output(&entries, format)
        }
        AgentCommands::RunSpec {
            target,
            session,
            provider,
        } => {
            let (path, project_id) = resolve_agent_project(target)?;
            let supervisor = match provider.as_deref() {
                None => None,
                Some(value) if value == forge::agent::SISYPHUSFY_SUPERVISOR => Some(value),
                Some(other) => {
                    return Err(ForgeError::AgentUnavailable {
                        reason: format!(
                            "unknown run-spec provider `{other}`; spec execution supports the supervisor: {}",
                            forge::agent::SISYPHUSFY_SUPERVISOR
                        ),
                    })
                }
            };
            let now = chrono::Utc::now();
            let outcome = run_session_spec(&path, session, supervisor, now)?;
            let files = forge::agent::write_session(&path, &outcome.session)?;
            let registry = open_registry(db_path)?;
            let verdict = outcome
                .verdict
                .clone()
                .unwrap_or_else(|| "done".to_string());
            let detail = format!(
                "agent `{}` run_spec session `{}` -> `{}` (verdict `{verdict}`)",
                project_id,
                session,
                outcome.state.label()
            );
            let _ = registry.record_operation("agent", &project_id, &verdict, &detail);
            agent_outcome_output(&outcome, &files, &detail, format)
        }
    }
}

fn run_agent_transition(
    db_path: &Path,
    path: &Path,
    project_id: &str,
    session_id: &str,
    transition: SessionTransition,
    format: Format,
) -> Result<Output, ForgeError> {
    let now = chrono::Utc::now();
    let session = read_session(path, session_id)?.ok_or_else(|| ForgeError::AgentUnavailable {
        reason: format!("session `{session_id}` was not found under `.forge/agents/`"),
    })?;
    let outcome = apply_agent_transition(session, transition, now)?;
    let files = forge::agent::write_session(path, &outcome.session)?;
    let registry = open_registry(db_path)?;
    let detail = format!(
        "agent `{}` session `{}` {} -> `{}`",
        project_id,
        session_id,
        transition.label(),
        outcome.state.label()
    );
    let _ = registry.record_operation("agent", project_id, "done", &detail);
    agent_outcome_output(&outcome, &files, &detail, format)
}

use forge::agent::AgentTransitionOutcome;

pub(super) fn agent_outcome_output(
    outcome: &AgentTransitionOutcome,
    files: &[String],
    detail: &str,
    format: Format,
) -> Result<Output, ForgeError> {
    let mut json = serde_json::json!({
        "transition": {
            "session_id": outcome.session.session_id,
            "project_id": outcome.session.project_id,
            "provider": outcome.session.provider.label(),
            "spec_id": outcome.session.spec_id,
            "state": outcome.state.label(),
            "requested": outcome.requested.label(),
            "evidence": outcome.evidence,
            "next_step": outcome.next_step,
            "note": outcome.note,
            "files_written": files,
            "registry_detail": detail,
        }
    });
    if let Some(verdict) = &outcome.verdict {
        json["transition"]["verdict"] = serde_json::Value::String(verdict.clone());
    }
    let mut human = format!(
        "session: {}\nproject: {}\nprovider: {}\nspec: {}\nrequested: {}\nstate: {}\nevidence: {}\nnext_step: {}\nnote: {}\nfiles: {}",
        outcome.session.session_id,
        outcome.session.project_id,
        outcome.session.provider.label(),
        outcome.session.spec_id,
        outcome.requested.label(),
        outcome.state.label(),
        if outcome.evidence.is_empty() { "(none)".to_string() } else { outcome.evidence.join("; ") },
        outcome.next_step.clone().unwrap_or_else(|| "(none)".to_string()),
        outcome.note,
        if files.is_empty() { "(none)".to_string() } else { files.join(", ") },
    );
    if let Some(verdict) = &outcome.verdict {
        human.push_str(&format!("\nverdict: {verdict}"));
    }
    Ok(as_output(format, human, json))
}

pub(super) fn agent_list_output(
    entries: &[forge::agent::SessionListEntry],
    format: Format,
) -> Result<Output, ForgeError> {
    let json = serde_json::json!({"sessions": entries});
    let mut human = format!(
        "{:<24} {:<12} {:<12} {:<8} {}",
        "Session", "Provider", "State", "Specs", "Transitions"
    );
    for entry in entries {
        human.push_str(&format!(
            "\n{:<24} {:<12} {:<12} {:<8} {}",
            entry.session_id,
            entry.provider.label(),
            entry.state.label(),
            entry.spec_id,
            entry.transition_count
        ));
    }
    Ok(as_output(format, human, json))
}

pub(super) fn render_agent_session_human(session: &forge::agent::AgentSession) -> String {
    let mut lines = vec![
        format!("session: {}", session.session_id),
        format!("project: {}", session.project_id),
        format!("provider: {}", session.provider.label()),
    ];
    if let Some(backing) = &session.backing {
        lines.push(format!(
            "backing: runtime={} handle={} adapter_version={}",
            backing.runtime,
            if backing.handle.is_empty() {
                "(none)"
            } else {
                &backing.handle
            },
            backing.adapter_version
        ));
    }
    lines.extend(vec![
        format!("spec: {}", session.spec_id),
        format!("state: {}", session.state.label()),
        format!("started_at: {}", session.started_at.to_rfc3339()),
        format!(
            "last_transition_at: {}",
            session.last_transition_at.to_rfc3339()
        ),
    ]);
    if session.transitions.is_empty() {
        lines.push("transitions: (none)".to_string());
    } else {
        lines.push("transitions:".to_string());
        for t in &session.transitions {
            lines.push(format!(
                "  - {} {} state={} evidence={} note={}",
                t.at.to_rfc3339(),
                t.kind.label(),
                t.state.label(),
                t.evidence.join(";"),
                t.note
            ));
        }
    }
    lines.join("\n")
}
