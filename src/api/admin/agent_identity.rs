//! Read-only agent + identity session visibility (`web-agent-identity-readonly`).
//!
//! Five session-gated GET handlers over the persisted Core stores the
//! CLI already reads: `agent::{list_sessions, status_for}` for the
//! recorded agent sessions under `.forge/agents/`,
//! `identity::{list_sessions, load_session}` for the persisted admin
//! sessions under `.forge/identity/`, and
//! `IdentityConfig::from_manifest_opt` for the manifest's `identity:`
//! block. No write, no provider contact, no adapter subprocess, no
//! shell, no browser-supplied path on any path; every response
//! carries the admin contract version and is scrubbed of the absolute
//! project directory. No challenge is built (so no `code_verifier`
//! secret ever exists to leak), no password is read, and the live
//! runtime probe (`live_runtime_status`, which spawns
//! `ariadex status`) is never called — the browser renders the
//! recorded session file, with supervised sessions marked
//! `live: null` plus an honest CLI-only note.

use super::super::{ApiConfig, ApiRequest, ApiResponse, Route, API_CONTRACT_VERSION};
use crate::core::manifest::Manifest;
use serde_json::{json, Value};
use std::path::Path;

use super::deploy::deploy_id_gate;
use super::gateway::{cors, error, guarded, scrub_json, scrub_text};

/// Match the five read-only agent/identity routes before the main table runs.
/// The literals `agents`/`identity` never collide with the existing
/// `releases`/`deploys`/`delivery`/`catalog` arms, so trying these shapes
/// first shadows no existing route. Kept beside the handlers (not in
/// `router.rs`) so the route table stays under the source-file-size cap.
pub(in crate::api) fn route_agent_identity(method: &str, segments: &[&str]) -> Option<Route> {
    match (method, segments) {
        ("GET", ["v1", "admin", "projects", id, "agents"]) => Some(Route::AdminProjectAgents {
            id: (*id).to_string(),
            session: None,
        }),
        ("GET", ["v1", "admin", "projects", id, "agents", session_id]) => {
            Some(Route::AdminProjectAgents {
                id: (*id).to_string(),
                session: Some((*session_id).to_string()),
            })
        }
        ("GET", ["v1", "admin", "projects", id, "identity", "config"]) => {
            Some(Route::AdminProjectIdentityConfig {
                id: (*id).to_string(),
            })
        }
        ("GET", ["v1", "admin", "projects", id, "identity", "sessions"]) => {
            Some(Route::AdminProjectIdentitySessions {
                id: (*id).to_string(),
                session: None,
            })
        }
        ("GET", ["v1", "admin", "projects", id, "identity", "sessions", session_id]) => {
            Some(Route::AdminProjectIdentitySessions {
                id: (*id).to_string(),
                session: Some((*session_id).to_string()),
            })
        }
        _ => None,
    }
}

/// Validate one agent `{session_id}` path segment before any filesystem
/// read. A blank value, a path separator (raw or percent-encoded), a
/// backslash, a `..` traversal or any other percent-encoding is a static
/// typed `400` that never echoes the offending input. Recorded session
/// ids are kebab-case tokens that never contain those bytes.
fn agent_session_gate(raw: &str) -> Result<String, ApiResponse> {
    let trimmed = raw.trim();
    if trimmed.is_empty()
        || trimmed.contains('/')
        || trimmed.contains('\\')
        || trimmed.contains("..")
        || trimmed.contains('%')
    {
        return Err(error(
            400,
            "admin-invalid-agent-session-id",
            "that agent session id is not valid; use the id shown in the session list.",
        ));
    }
    Ok(trimmed.to_string())
}

/// Validate one identity `{session_id}` path segment before any
/// filesystem read. Minted session ids are non-empty hex tokens (the
/// `session_path_for` invariant); anything else is a static typed
/// `400` that never echoes the offending input.
fn identity_session_gate(raw: &str) -> Result<String, ApiResponse> {
    let trimmed = raw.trim();
    let hex = !trimmed.is_empty() && trimmed.chars().all(|c| c.is_ascii_hexdigit());
    if !hex {
        return Err(error(
            400,
            "admin-invalid-identity-session-id",
            "that identity session id is not valid; use the id shown in the session list.",
        ));
    }
    Ok(trimmed.to_string())
}

/// Render a Core failure with the shared status mapping, scrubbing the
/// project directory from the message. The typed code and status are
/// preserved so a boundary scenario is reported honestly.
fn typed_agent_identity_error(project_dir: &Path, err: &crate::core::ForgeError) -> ApiResponse {
    let secrets = [project_dir.display().to_string()];
    let status = super::super::err_status(err);
    ApiResponse::json(
        status,
        json!({
            "error": {
                "code": err.code(),
                "message": scrub_text(&err.to_string(), &secrets),
            },
            "contract": API_CONTRACT_VERSION,
        }),
    )
}

/// `GET /v1/admin/projects/{id}/agents`. Lists every recorded agent
/// session for the project via `agent::list_sessions`.
/// `SessionListEntry` carries no path, so the entries serialize
/// unchanged.
pub(in crate::api) fn agent_sessions(
    config: &ApiConfig,
    db_path: &Path,
    request: &ApiRequest,
    id: &str,
    session: &Option<String>,
) -> ApiResponse {
    match session {
        None => agent_session_list(config, db_path, request, id),
        Some(session_id) => agent_session_status(config, db_path, request, id, session_id),
    }
}

fn agent_session_list(
    config: &ApiConfig,
    db_path: &Path,
    request: &ApiRequest,
    id: &str,
) -> ApiResponse {
    cors(
        config,
        request,
        guarded(db_path, request, |_| {
            let project_dir = match deploy_id_gate(db_path, id) {
                Ok(dir) => dir,
                Err(response) => return response,
            };
            match crate::agent::list_sessions(&project_dir) {
                Ok(entries) => ApiResponse::json(
                    200,
                    json!({
                        "project_id": id,
                        "agent_sessions": entries,
                        "contract": API_CONTRACT_VERSION,
                    }),
                ),
                Err(err) => typed_agent_identity_error(&project_dir, &err),
            }
        }),
    )
}

/// One recorded agent session via `agent::status_for` (`read_session`).
/// An unknown id is a typed `404`. The absolute `project_path` is
/// projected out: the browser gets identity and recorded state only.
/// The live runtime probe is never invoked (it spawns an adapter
/// subprocess); supervised sessions report `live: null` with an
/// honest CLI-only note, and adapter availability comes from the
/// spawn-free `probe_provider` PATH check only.
fn agent_session_status(
    config: &ApiConfig,
    db_path: &Path,
    request: &ApiRequest,
    id: &str,
    session_id: &str,
) -> ApiResponse {
    cors(
        config,
        request,
        guarded(db_path, request, |_| {
            let session_id = match agent_session_gate(session_id) {
                Ok(value) => value,
                Err(response) => return response,
            };
            let project_dir = match deploy_id_gate(db_path, id) {
                Ok(dir) => dir,
                Err(response) => return response,
            };
            match crate::agent::status_for(&project_dir, &session_id) {
                Ok(Some(session)) => {
                    let (adapter_available, adapter_reason) =
                        match crate::agent::probe_provider(session.provider) {
                            Ok(()) => (
                                true,
                                format!(
                                    "provider `{}` resolved on this host",
                                    session.provider.label()
                                ),
                            ),
                            Err(err) => (false, err.to_string()),
                        };
                    let live_note = if session.provider.is_supervised() {
                        "live runtime state is a CLI observation (`forge agent status`); the browser renders the recorded session file."
                    } else {
                        "recorded session state is the live surface for this provider; no runtime probe exists."
                    };
                    let view = json!({
                        "session_id": session.session_id,
                        "project_id": session.project_id,
                        "provider": session.provider,
                        "spec_id": session.spec_id,
                        "state": session.state,
                        "started_at": session.started_at,
                        "last_transition_at": session.last_transition_at,
                        "transitions": session.transitions,
                        "backing": session.backing,
                    });
                    let secrets = [project_dir.display().to_string()];
                    ApiResponse::json(
                        200,
                        scrub_json(
                            json!({
                                "project_id": id,
                                "session": view,
                                "live": Value::Null,
                                "live_note": live_note,
                                "adapter": {
                                    "available": adapter_available,
                                    "reason": adapter_reason,
                                },
                                "contract": API_CONTRACT_VERSION,
                            }),
                            &secrets,
                        ),
                    )
                }
                Ok(None) => error(
                    404,
                    "admin-agent-session-not-found",
                    "that agent session was not found under this project's recorded sessions.",
                ),
                Err(err) => typed_agent_identity_error(&project_dir, &err),
            }
        }),
    )
}

/// `GET /v1/admin/projects/{id}/identity/config`. Validates the
/// manifest's `identity:` block via `IdentityConfig::from_manifest_opt`
/// without contacting any provider, building any challenge, or
/// persisting anything. A project with no `identity:` block is a
/// typed `404`; an invalid block reports its typed
/// `identity-invalid` reason honestly.
pub(in crate::api) fn identity_config(
    config: &ApiConfig,
    db_path: &Path,
    request: &ApiRequest,
    id: &str,
) -> ApiResponse {
    cors(
        config,
        request,
        guarded(db_path, request, |_| {
            let project_dir = match deploy_id_gate(db_path, id) {
                Ok(dir) => dir,
                Err(response) => return response,
            };
            let (manifest, _) = match Manifest::load_from_dir(&project_dir, None) {
                Ok(value) => value,
                Err(err) => return typed_agent_identity_error(&project_dir, &err),
            };
            match crate::identity::IdentityConfig::from_manifest_opt(id, &manifest) {
                Ok(Some(validated)) => {
                    let secrets = [project_dir.display().to_string()];
                    ApiResponse::json(
                        200,
                        scrub_json(
                            json!({
                                "project_id": id,
                                "identity_config": validated,
                                "contract": API_CONTRACT_VERSION,
                            }),
                            &secrets,
                        ),
                    )
                }
                Ok(None) => error(
                    404,
                    "admin-identity-unconfigured",
                    "this project declares no `identity:` block; declare one in forge.yaml to enable OIDC admin federation.",
                ),
                Err(err) => typed_agent_identity_error(&project_dir, &err),
            }
        }),
    )
}

/// `GET /v1/admin/projects/{id}/identity/sessions` and
/// `GET .../sessions/{session_id}`. Lists every persisted admin
/// session via `identity::list_sessions`, or inspects one via
/// `identity::load_session`. An unknown id is a typed `404`; a hex
/// id owned by a sibling project is a typed `403` cross-project
/// refusal (the CLI `session-inspect` boundary). No challenge is
/// built, no session is minted or revoked, and no credential value
/// is returned.
pub(in crate::api) fn identity_sessions(
    config: &ApiConfig,
    db_path: &Path,
    request: &ApiRequest,
    id: &str,
    session: &Option<String>,
) -> ApiResponse {
    match session {
        None => identity_session_list(config, db_path, request, id),
        Some(session_id) => identity_session_inspect(config, db_path, request, id, session_id),
    }
}

fn identity_session_list(
    config: &ApiConfig,
    db_path: &Path,
    request: &ApiRequest,
    id: &str,
) -> ApiResponse {
    cors(
        config,
        request,
        guarded(db_path, request, |_| {
            let project_dir = match deploy_id_gate(db_path, id) {
                Ok(dir) => dir,
                Err(response) => return response,
            };
            match crate::identity::list_sessions(&project_dir, id) {
                Ok(sessions) => ApiResponse::json(
                    200,
                    json!({
                        "project_id": id,
                        "sessions": sessions,
                        "contract": API_CONTRACT_VERSION,
                    }),
                ),
                Err(err) => typed_agent_identity_error(&project_dir, &err),
            }
        }),
    )
}

fn identity_session_inspect(
    config: &ApiConfig,
    db_path: &Path,
    request: &ApiRequest,
    id: &str,
    session_id: &str,
) -> ApiResponse {
    cors(
        config,
        request,
        guarded(db_path, request, |_| {
            let session_id = match identity_session_gate(session_id) {
                Ok(value) => value,
                Err(response) => return response,
            };
            let project_dir = match deploy_id_gate(db_path, id) {
                Ok(dir) => dir,
                Err(response) => return response,
            };
            match crate::identity::load_session(&project_dir, id, &session_id) {
                Ok(Some(session)) => ApiResponse::json(
                    200,
                    json!({
                        "project_id": id,
                        "session": session,
                        "contract": API_CONTRACT_VERSION,
                    }),
                ),
                Ok(None) => {
                    match crate::identity::lookup_session_in_sibling_projects(&project_dir, &session_id) {
                        Ok(Some(_)) => error(
                            403,
                            "identity-session-cross-project",
                            "that session was minted for another project; sessions are project-scoped and cannot be inspected through this project.",
                        ),
                        _ => error(
                            404,
                            "identity-session-not-found",
                            "that session was not found under this project's persisted identity sessions.",
                        ),
                    }
                }
                Err(err) => typed_agent_identity_error(&project_dir, &err),
            }
        }),
    )
}
