//! Read-only release/deploy history (`web-release-deploy-history`).
//!
//! Five session-gated GET handlers over the persisted Core stores the
//! CLI already reads: `release::engine::{list_releases, read_release}`,
//! `deploy::engine::{list_deploys, read_deploy}` and the registry
//! `operations` journal filtered to the CLI's publish/deploy kind set.
//! No write, no provider, no adapter, no shell, no browser-supplied
//! path on any path; every response carries the admin contract
//! version and is scrubbed of the absolute project directory.

use super::super::{ApiConfig, ApiRequest, ApiResponse, Route, API_CONTRACT_VERSION};
use crate::registry::Registry;
use serde_json::{json, Value};
use std::path::Path;

use super::deploy::deploy_id_gate;
use super::gateway::{cors, error, guarded, scrub_json, scrub_text};

/// Match the five read-only history routes before the main table runs.
/// The literals `releases`/`deploys`/`deploy`+`status` never collide
/// with the singular `release`/`deploy` plan/apply arms, so trying
/// these shapes first shadows no existing route. Kept beside the
/// handlers (not in `router.rs`) so the route table stays under the
/// source-file-size cap.
pub(in crate::api) fn route_history(method: &str, segments: &[&str]) -> Option<Route> {
    match (method, segments) {
        ("GET", ["v1", "admin", "projects", id, "releases"]) => {
            Some(Route::AdminProjectReleaseHistory {
                id: (*id).to_string(),
            })
        }
        ("GET", ["v1", "admin", "projects", id, "releases", release_id]) => {
            Some(Route::AdminProjectReleaseInspect {
                id: (*id).to_string(),
                release_id: (*release_id).to_string(),
            })
        }
        ("GET", ["v1", "admin", "projects", id, "deploys"]) => {
            Some(Route::AdminProjectDeployHistory {
                id: (*id).to_string(),
            })
        }
        ("GET", ["v1", "admin", "projects", id, "deploys", deploy_id]) => {
            Some(Route::AdminProjectDeployInspect {
                id: (*id).to_string(),
                deploy_id: (*deploy_id).to_string(),
            })
        }
        ("GET", ["v1", "admin", "projects", id, "deploy", "status"]) => {
            Some(Route::AdminProjectDeployStatus {
                id: (*id).to_string(),
            })
        }
        _ => None,
    }
}

/// Validate one `{release_id}` / `{deploy_id}` path segment before any
/// filesystem read. A blank value, a path separator (raw or
/// percent-encoded), a backslash, a `..` traversal or any other
/// percent-encoding is a static typed `400` that never echoes the
/// offending input. Persisted ids are engine-derived tokens that never
/// contain those bytes.
fn history_id_gate(raw: &str, kind: &str) -> Result<String, ApiResponse> {
    let trimmed = raw.trim();
    if trimmed.is_empty()
        || trimmed.contains('/')
        || trimmed.contains('\\')
        || trimmed.contains("..")
        || trimmed.contains('%')
    {
        return Err(error(
            400,
            "admin-invalid-history-id",
            &format!("that {kind} id is not valid; use the id shown in the history list."),
        ));
    }
    Ok(trimmed.to_string())
}

/// Parse the `?limit=` query value for the status route. Absent means
/// 20; the value clamps to 1..=100; an unparseable value is a static
/// typed `400` that never echoes the offending input.
fn history_limit_gate(query: Option<&str>) -> Result<usize, ApiResponse> {
    let raw = query
        .unwrap_or("")
        .split('&')
        .filter(|part| !part.is_empty())
        .find_map(|pair| {
            let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
            (key == "limit").then(|| super::super::percent_decode(value))
        });
    let text = match raw {
        Some(value) if !value.trim().is_empty() => value,
        _ => return Ok(20),
    };
    match text.trim().parse::<usize>() {
        Ok(value) => Ok(value.clamp(1, 100)),
        Err(_) => Err(error(
            400,
            "admin-invalid-history-limit",
            "the history `limit` must be a number from 1 to 100.",
        )),
    }
}

/// Render a Core failure with the shared status mapping, scrubbing the
/// project directory from the message. The typed code and status are
/// preserved so a boundary scenario is reported honestly.
fn typed_history_error(project_dir: &Path, err: &crate::core::ForgeError) -> ApiResponse {
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

/// `GET /v1/admin/projects/{id}/releases`. Lists every persisted
/// release for the project via `release::engine::list_releases`.
/// `ReleaseListEntry` carries no path, so the entries serialize
/// unchanged.
pub(in crate::api) fn release_history(
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
            match crate::release::engine::list_releases(&project_dir, id) {
                Ok(entries) => ApiResponse::json(
                    200,
                    json!({
                        "project_id": id,
                        "releases": entries,
                        "contract": API_CONTRACT_VERSION,
                    }),
                ),
                Err(err) => typed_history_error(&project_dir, &err),
            }
        }),
    )
}

/// `GET /v1/admin/projects/{id}/releases/{release_id}`. Inspects one
/// persisted release via `release::engine::read_release`; an unknown
/// id is a typed `404`. `ReleaseState` carries no path.
pub(in crate::api) fn release_inspect(
    config: &ApiConfig,
    db_path: &Path,
    request: &ApiRequest,
    id: &str,
    release_id: &str,
) -> ApiResponse {
    cors(
        config,
        request,
        guarded(db_path, request, |_| {
            let release_id = match history_id_gate(release_id, "release") {
                Ok(value) => value,
                Err(response) => return response,
            };
            let project_dir = match deploy_id_gate(db_path, id) {
                Ok(dir) => dir,
                Err(response) => return response,
            };
            match crate::release::engine::read_release(&project_dir, id, &release_id) {
                Ok(Some(state)) => ApiResponse::json(
                    200,
                    json!({
                        "project_id": id,
                        "release": state,
                        "contract": API_CONTRACT_VERSION,
                    }),
                ),
                Ok(None) => error(
                    404,
                    "admin-history-not-found",
                    "that release was not found under this project's persisted releases.",
                ),
                Err(err) => typed_history_error(&project_dir, &err),
            }
        }),
    )
}

/// `GET /v1/admin/projects/{id}/deploys`. Lists every persisted deploy
/// for the project via `deploy::engine::list_deploys`. The absolute
/// `state_path` is projected out: the browser gets identity and
/// health only.
fn deploy_entry_view(entry: &crate::deploy::DeployListEntry) -> Value {
    json!({
        "deploy_id": entry.deploy_id,
        "project_id": entry.project_id,
        "target": entry.target,
        "source_revision": entry.source_revision,
        "current_state": entry.current_state,
        "last_run_at": entry.last_run_at,
    })
}

pub(in crate::api) fn deploy_history(
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
            match crate::deploy::engine::list_deploys(&project_dir, id) {
                Ok(entries) => {
                    let views: Vec<Value> = entries.iter().map(deploy_entry_view).collect();
                    let secrets = [project_dir.display().to_string()];
                    ApiResponse::json(
                        200,
                        scrub_json(
                            json!({
                                "project_id": id,
                                "deploys": views,
                                "contract": API_CONTRACT_VERSION,
                            }),
                            &secrets,
                        ),
                    )
                }
                Err(err) => typed_history_error(&project_dir, &err),
            }
        }),
    )
}

/// `GET /v1/admin/projects/{id}/deploys/{deploy_id}`. Inspects one
/// persisted deploy via `deploy::engine::read_deploy`; an unknown id
/// is a typed `404`. `DeployState` carries no path.
pub(in crate::api) fn deploy_inspect(
    config: &ApiConfig,
    db_path: &Path,
    request: &ApiRequest,
    id: &str,
    deploy_id: &str,
) -> ApiResponse {
    cors(
        config,
        request,
        guarded(db_path, request, |_| {
            let deploy_id = match history_id_gate(deploy_id, "deploy") {
                Ok(value) => value,
                Err(response) => return response,
            };
            let project_dir = match deploy_id_gate(db_path, id) {
                Ok(dir) => dir,
                Err(response) => return response,
            };
            match crate::deploy::engine::read_deploy(&project_dir, id, &deploy_id) {
                Ok(Some(state)) => {
                    let secrets = [project_dir.display().to_string()];
                    ApiResponse::json(
                        200,
                        scrub_json(
                            json!({
                                "project_id": id,
                                "deploy": state,
                                "contract": API_CONTRACT_VERSION,
                            }),
                            &secrets,
                        ),
                    )
                }
                Ok(None) => error(
                    404,
                    "admin-history-not-found",
                    "that deploy was not found under this project's persisted deploys.",
                ),
                Err(err) => typed_history_error(&project_dir, &err),
            }
        }),
    )
}

/// `GET /v1/admin/projects/{id}/deploy/status`. Answers from the
/// persisted registry journal only: `operations_for_project`
/// filtered to the CLI's `publish|deploy|publish.github` kind set.
/// No provider is contacted, no adapter runs, no queue scoping and
/// no watch loop exist on this route.
pub(in crate::api) fn deploy_status(
    config: &ApiConfig,
    db_path: &Path,
    request: &ApiRequest,
    id: &str,
) -> ApiResponse {
    cors(
        config,
        request,
        guarded(db_path, request, |req| {
            let limit = match history_limit_gate(req.query.as_deref()) {
                Ok(value) => value,
                Err(response) => return response,
            };
            // The id gate precedes the journal read so an unmanaged id
            // stays a typed 404 even when the journal holds no rows.
            if let Err(response) = deploy_id_gate(db_path, id) {
                return response;
            }
            let registry = match Registry::open(db_path) {
                Ok(registry) => registry,
                Err(err) => {
                    let status = super::super::err_status(&err);
                    return ApiResponse::json(
                        status,
                        json!({
                            "error": {
                                "code": err.code(),
                                "message": err.to_string(),
                            },
                            "contract": API_CONTRACT_VERSION,
                        }),
                    );
                }
            };
            match registry.operations_for_project(id, limit) {
                Ok(rows) => {
                    let entries: Vec<_> = rows
                        .into_iter()
                        .filter(|entry| {
                            entry.kind == "publish"
                                || entry.kind == "deploy"
                                || entry.kind == "publish.github"
                        })
                        .collect();
                    let state = entries
                        .first()
                        .map(|entry| entry.state.clone())
                        .unwrap_or_else(|| "empty".to_string());
                    ApiResponse::json(
                        200,
                        json!({
                            "project_id": id,
                            "scope": format!("project={id}"),
                            "state": state,
                            "entries": entries,
                            "read_only": true,
                            "contract": API_CONTRACT_VERSION,
                        }),
                    )
                }
                Err(err) => {
                    let status = super::super::err_status(&err);
                    ApiResponse::json(
                        status,
                        json!({
                            "error": {
                                "code": err.code(),
                                "message": err.to_string(),
                            },
                            "contract": API_CONTRACT_VERSION,
                        }),
                    )
                }
            }
        }),
    )
}
