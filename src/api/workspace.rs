//! Live workspace onboarding (`forge-web-workspace-onboarding`).
//!
//! Read-only candidate discovery over the operator-configured project root —
//! re-read live on every request so an expanding workspace needs no code,
//! list or configuration change — plus bulk preview + confirm/digest-bound
//! onboarding through the same Core import/register functions the CLI runs.
//! The browser supplies validated single-segment directory leaves plus typed
//! `id`/`profile` overrides; destinations resolve server-side under the
//! configured root. No concrete host folder appears here.

use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use super::admin::{authoring_digest, cors, error, guarded, is_json};
use super::project_management::projects_root;
use super::{ApiConfig, ApiRequest, ApiResponse, API_CONTRACT_VERSION};
use crate::core::manifest::Manifest;
use crate::registry::Registry;

/// Workspace candidate discovery, exposed as a session-gated read-only admin
/// route. It re-reads the operator-configured project root on every request
/// and returns a bounded, path-free candidate view per immediate child
/// directory. Exported so the command catalog names the exact path.
pub const ROUTE_ADMIN_WORKSPACE_CANDIDATES: &str = "GET /v1/admin/workspace/candidates";

/// Bulk workspace onboarding, exposed as a session-gated, preview +
/// confirm/digest-bound admin route. The browser supplies only validated
/// single-segment directory leaves plus typed `id`/`profile` overrides; every
/// destination is resolved server-side under the configured root.
/// Per-item outcomes are reported honestly, never as a blanket success.
pub const ROUTE_ADMIN_WORKSPACE_ONBOARD: &str = "POST /v1/admin/workspace/onboard";

/// Maximum candidates onboarded by one confirmed `workspace/onboard` request.
/// Bounds request time and journal width; the operator repeats the flow for
/// larger workspaces since discovery is live.
const WORKSPACE_ONBOARD_MAX_ITEMS: usize = 25;

/// Default/maximum page size for candidate discovery.
const WORKSPACE_CANDIDATES_DEFAULT_LIMIT: usize = 50;
const WORKSPACE_CANDIDATES_MAX_LIMIT: usize = 100;

/// Validate one workspace directory leaf supplied by the browser. Only a
/// single path segment is accepted — no separator, traversal, NUL byte,
/// control character, `.` or `..` — so the value can only ever address a
/// direct child of the configured root. Refusals are static and never echo
/// the offending input.
fn validate_workspace_leaf(raw: &str) -> Result<String, ApiResponse> {
    let leaf = raw.trim();
    if leaf.is_empty() || leaf.len() > 128 {
        return Err(error(
            400,
            "admin-invalid-directory",
            "each selected entry must be a single folder name of 1 to 128 characters.",
        ));
    }
    if leaf == "." || leaf == ".." {
        return Err(error(
            400,
            "admin-invalid-directory",
            "folder names may not be just dots; use the real folder name.",
        ));
    }
    if leaf
        .bytes()
        .any(|byte| byte == b'/' || byte == b'\\' || byte == 0 || byte.is_ascii_control())
    {
        return Err(error(
            400,
            "admin-invalid-directory",
            "folder names must be a single name with no slashes or special characters.",
        ));
    }
    Ok(leaf.to_string())
}

/// Resolve one validated leaf to a canonical descendant of the root. A
/// missing directory or a symlink escaping the root is refused before any
/// read; the leaf itself is never echoed.
fn workspace_destination(root: &Path, leaf: &str) -> Result<PathBuf, ApiResponse> {
    let canonical = root.join(leaf).canonicalize().map_err(|_| {
        error(
            409,
            "admin-prerequisite",
            "the named workspace folder does not exist under the configured project folder; refresh the folder list first.",
        )
    })?;
    if !canonical.starts_with(root) {
        return Err(error(
            409,
            "admin-prerequisite",
            "the named workspace folder is outside the configured project folder; refusing to touch it.",
        ));
    }
    if !canonical.is_dir() {
        return Err(error(
            409,
            "admin-prerequisite",
            "the named workspace path is not a folder under the configured project folder.",
        ));
    }
    Ok(canonical)
}

/// One candidate's live registration state against the registry, computed
/// from canonical paths and ids only.
fn workspace_registration_state(
    projects: &[crate::registry::ProjectRecord],
    canonical: &str,
    manifest_id: Option<&str>,
    derived_id: Option<&str>,
) -> (&'static str, Option<String>) {
    if let Some(owner) = projects.iter().find(|record| record.path == canonical) {
        return ("registered", Some(owner.id.clone()));
    }
    for id in [manifest_id, derived_id].into_iter().flatten() {
        if let Some(owner) = projects.iter().find(|record| record.id == id) {
            if owner.path != canonical {
                return (
                    "id-collision",
                    Some(format!("{id} is already used by another folder")),
                );
            }
        }
    }
    ("unregistered", None)
}

/// Build the path-free candidate view for one live directory. Reads only:
/// manifest presence/validity, import detection and registry state. Never
/// serializes a path, a manifest body or a credential.
fn workspace_candidate(
    projects: &[crate::registry::ProjectRecord],
    root: &Path,
    leaf: &str,
) -> Value {
    let indexed = match workspace_destination(root, leaf) {
        Ok(dest) => dest,
        Err(_) => {
            return json!({
                "directory": leaf,
                "state": "unreadable",
                "selectable": false,
                "reason": "the folder cannot be read right now.",
            })
        }
    };
    let canonical = indexed.display().to_string();
    let manifest_path = indexed.join("forge.yaml");
    if manifest_path.is_file() {
        match Manifest::load_from_dir(&indexed, None) {
            Ok((manifest, _)) => {
                let (state, reason) = match workspace_registration_state(
                    projects,
                    &canonical,
                    Some(&manifest.project.id),
                    None,
                ) {
                    ("registered", _) => ("registered", None),
                    ("id-collision", reason) => ("id-collision", reason),
                    _ => ("unregistered", None),
                };
                return json!({
                    "directory": leaf,
                    "state": state,
                    "selectable": state == "unregistered",
                    "action": "register",
                    "id": manifest.project.id,
                    "profile": manifest.project.profile,
                    "manifest": true,
                    "reason": reason,
                });
            }
            Err(_) => {
                return json!({
                    "directory": leaf,
                    "state": "manifest-invalid",
                    "selectable": false,
                    "manifest": true,
                    "reason": "forge.yaml exists but does not validate; fix it in a terminal first.",
                })
            }
        }
    }
    let derived = crate::import::derive_project_id(&indexed, None)
        .ok()
        .filter(|id| {
            !projects
                .iter()
                .any(|record| record.id == *id && record.path != canonical)
        });
    match crate::import::inspect_import(&indexed, None) {
        Ok(proposal) => match proposal.suggested_profile.clone() {
            Some(suggested) => {
                let state = match derived {
                    Some(_) => "unregistered",
                    None => "undecidable",
                };
                let reason = if state == "undecidable" {
                    Some("no usable project id; enter one explicitly.".to_string())
                } else {
                    None
                };
                let (state, reason) = match workspace_registration_state(
                    projects,
                    &canonical,
                    None,
                    derived.as_deref(),
                ) {
                    ("registered", _) => ("registered", None),
                    ("id-collision", reason) => ("id-collision", reason),
                    _ => (state, reason),
                };
                json!({
                    "directory": leaf,
                    "state": state,
                    "selectable": state == "unregistered",
                    "action": "import",
                    "id": derived,
                    "profile": suggested,
                    "confidence": proposal.confidence,
                    "alternatives": proposal.alternatives,
                    "manifest": false,
                    "reason": reason,
                })
            }
            None => json!({
                "directory": leaf,
                "state": "undecidable",
                "selectable": false,
                "action": "import",
                "id": derived,
                "profile": Value::Null,
                "manifest": false,
                "reason": "no recognizable stack markers; choose an explicit profile to onboard.",
            }),
        },
        Err(err) => {
            let code = err.code();
            let blocked = code == "ambiguous-import";
            json!({
                "directory": leaf,
                "state": if blocked { "ambiguous" } else { "undecidable" },
                "selectable": false,
                "action": "import",
                "id": derived,
                "profile": Value::Null,
                "manifest": false,
                "reason": "several profiles match; choose an explicit profile to onboard.",
            })
        }
    }
}

/// `GET /v1/admin/workspace/candidates`. Re-reads the configured root live
/// on every request so added, renamed or removed siblings change the next
/// response with no other update. Bounded, sorted and paginated; read-only.
pub(super) fn workspace_candidates(
    config: &ApiConfig,
    db_path: &Path,
    request: &ApiRequest,
) -> ApiResponse {
    cors(
        config,
        request,
        guarded(db_path, request, |req| {
            let root = match projects_root() {
                Ok(root) => root,
                Err(response) => return response,
            };
            let (limit, cursor) = workspace_page(req.query.as_deref());
            let mut leaves: Vec<String> = Vec::new();
            let entries = match std::fs::read_dir(&root) {
                Ok(entries) => entries,
                Err(_) => {
                    return error(
                        503,
                        "admin-api-unavailable",
                        "the configured project folder cannot be read right now.",
                    )
                }
            };
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with('.') {
                    continue;
                }
                let meta = match entry.path().symlink_metadata() {
                    Ok(meta) => meta,
                    Err(_) => continue,
                };
                if !meta.is_dir() || meta.is_symlink() {
                    continue;
                }
                if validate_workspace_leaf(&name).is_ok() {
                    leaves.push(name);
                }
            }
            leaves.sort();
            let total = leaves.len();
            let page: Vec<Value> = leaves
                .into_iter()
                .skip(cursor)
                .take(limit)
                .map(|leaf| {
                    let projects = Registry::open(db_path)
                        .ok()
                        .and_then(|registry| registry.list().ok())
                        .unwrap_or_default();
                    workspace_candidate(&projects, &root, &leaf)
                })
                .collect();
            ApiResponse::json(
                200,
                json!({
                    "candidates": page,
                    "total": total,
                    "cursor": cursor,
                    "limit": limit,
                    "contract": API_CONTRACT_VERSION,
                }),
            )
        }),
    )
}

/// Parse `limit`/`cursor` defensively: malformed values fall back to
/// defaults because they only shape a read.
fn workspace_page(query: Option<&str>) -> (usize, usize) {
    let mut limit = WORKSPACE_CANDIDATES_DEFAULT_LIMIT;
    let mut cursor = 0usize;
    if let Some(query) = query {
        for pair in query.split('&').filter(|part| !part.is_empty()) {
            let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
            match key {
                "limit" => {
                    if let Ok(parsed) = value.parse::<usize>() {
                        if parsed > 0 {
                            limit = parsed.min(WORKSPACE_CANDIDATES_MAX_LIMIT);
                        }
                    }
                }
                "cursor" => {
                    if let Ok(parsed) = value.parse::<usize>() {
                        cursor = parsed;
                    }
                }
                _ => {}
            }
        }
    }
    (limit, cursor)
}

/// One selected onboarding item: a validated leaf plus optional typed `id`
/// and `profile` overrides. Unknown keys are ignored.
struct OnboardItem {
    directory: String,
    id: Option<String>,
    profile: Option<String>,
}

fn parse_onboard_items(body: &Value) -> Result<Vec<OnboardItem>, ApiResponse> {
    let raw = body.get("items").and_then(Value::as_array).ok_or_else(|| {
        error(
            400,
            "admin-field-required",
            "onboarding needs 1 to 25 selected folders.",
        )
    })?;
    if raw.is_empty() || raw.len() > WORKSPACE_ONBOARD_MAX_ITEMS {
        return Err(error(
            400,
            "admin-field-required",
            "onboarding needs 1 to 25 selected folders.",
        ));
    }
    let mut items = Vec::with_capacity(raw.len());
    for entry in raw {
        let directory = entry
            .get("directory")
            .and_then(Value::as_str)
            .map(validate_workspace_leaf)
            .ok_or_else(|| {
                error(
                    400,
                    "admin-field-required",
                    "every entry needs a valid folder name.",
                )
            })??;
        let id = entry
            .get("id")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| {
                if crate::core::validate_project_id(value).is_err() {
                    Err(error(
                        400,
                        "admin-invalid-project-name",
                        "an id override must use lowercase letters, numbers and dashes.",
                    ))
                } else {
                    Ok(value.to_string())
                }
            })
            .transpose()?;
        let profile = entry
            .get("profile")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        items.push(OnboardItem {
            directory,
            id,
            profile,
        });
    }
    items.sort_by(|a, b| a.directory.cmp(&b.directory));
    Ok(items)
}

/// Preview one selected item without writing: register plan for manifests,
/// import plan otherwise. A `blocked` reason excludes the item from apply.
fn onboard_preview(
    projects: &[crate::registry::ProjectRecord],
    root: &Path,
    item: &OnboardItem,
) -> Value {
    let dest = match workspace_destination(root, &item.directory) {
        Ok(dest) => dest,
        Err(_) => {
            return json!({
                "directory": item.directory,
                "blocked": "the folder cannot be read right now.",
            })
        }
    };
    let canonical = dest.display().to_string();
    if dest.join("forge.yaml").is_file() {
        return match Manifest::load_from_dir(&dest, None) {
            Ok((manifest, _)) => {
                match workspace_registration_state(
                    projects,
                    &canonical,
                    Some(&manifest.project.id),
                    None,
                ) {
                    ("registered", _) => json!({
                        "directory": item.directory,
                        "action": "register",
                        "id": manifest.project.id,
                        "profile": manifest.project.profile,
                        "already": true,
                    }),
                    ("id-collision", _) => json!({
                        "directory": item.directory,
                        "blocked": "its project id is already used by another folder.",
                    }),
                    _ => json!({
                        "directory": item.directory,
                        "action": "register",
                        "id": manifest.project.id,
                        "profile": manifest.project.profile,
                    }),
                }
            }
            Err(_) => json!({
                "directory": item.directory,
                "blocked": "forge.yaml exists but does not validate.",
            }),
        };
    }
    let derived = crate::import::derive_project_id(&dest, item.id.as_deref()).ok();
    match crate::import::inspect_import(&dest, item.profile.as_deref()) {
        Ok(proposal) => {
            let Some(profile) = proposal.suggested_profile.clone().or(item.profile.clone()) else {
                return json!({
                    "directory": item.directory,
                    "blocked": "no recognizable stack markers; choose an explicit profile.",
                });
            };
            let id = match derived {
                Some(id) => id,
                None => {
                    return json!({
                        "directory": item.directory,
                        "blocked": "no usable project id; enter one explicitly.",
                    })
                }
            };
            match workspace_registration_state(projects, &canonical, None, Some(&id)) {
                ("registered", _) => json!({
                    "directory": item.directory,
                    "action": "import",
                    "id": id,
                    "profile": profile,
                    "already": true,
                }),
                ("id-collision", _) => json!({
                    "directory": item.directory,
                    "blocked": "the resolved id is already used by another folder.",
                }),
                _ => json!({
                    "directory": item.directory,
                    "action": "import",
                    "id": id,
                    "profile": profile,
                    "confidence": proposal.confidence,
                }),
            }
        }
        Err(err) if err.code() == "ambiguous-import" => json!({
            "directory": item.directory,
            "blocked": "several profiles match; choose an explicit profile.",
        }),
        Err(_) => json!({
            "directory": item.directory,
            "blocked": "the folder cannot be onboarded in its current shape.",
        }),
    }
}

/// Apply one previewed item through the same Core functions the CLI runs.
/// Already-registered items are reported without rewriting.
fn onboard_apply(db_path: &Path, root: &Path, item: &OnboardItem, plan: &Value) -> Value {
    if plan
        .get("already")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return json!({
            "directory": item.directory,
            "ok": true,
            "id": plan.get("id").cloned().unwrap_or(Value::Null),
            "replayed": true,
        });
    }
    let dest = match workspace_destination(root, &item.directory) {
        Ok(dest) => dest,
        Err(response) => {
            return onboard_failure(item, &response);
        }
    };
    let action = plan.get("action").and_then(Value::as_str).unwrap_or("");
    let outcome = if action == "register" {
        Registry::open(db_path)
            .map_err(|err| ApiResponse::from_error(&err))
            .and_then(|mut registry| {
                registry
                    .register(&dest, None)
                    .map_err(|err| ApiResponse::from_error(&err))
            })
            .map(|record| json!({"id": record.id, "name": record.name, "profile": record.profile}))
    } else {
        Registry::open(db_path)
            .map_err(|err| ApiResponse::from_error(&err))
            .and_then(|mut registry| {
                crate::import::adopt_import(
                    &mut registry,
                    &dest,
                    item.profile.as_deref(),
                    item.id.as_deref(),
                )
                .map_err(|err| ApiResponse::from_error(&err))
            })
            .map(|record| json!({"id": record.id, "name": record.name, "profile": record.profile}))
    };
    match outcome {
        Ok(record) => json!({
            "directory": item.directory,
            "ok": true,
            "id": record.get("id").cloned().unwrap_or(Value::Null),
        }),
        Err(response) => onboard_failure(item, &response),
    }
}

/// Render one failed item as data: typed code plus a scrubbed message, never
/// a path echo and never a raised error that aborts its siblings.
fn onboard_failure(item: &OnboardItem, response: &ApiResponse) -> Value {
    let body: Value = serde_json::from_slice(&response.body).unwrap_or(Value::Null);
    json!({
        "directory": item.directory,
        "ok": false,
        "code": body.pointer("/error/code").cloned().unwrap_or(json!("admin-onboard-failed")),
        "message": body.pointer("/error/message").cloned().unwrap_or(json!("the item could not be onboarded.")),
    })
}

/// One JSON-only, session-gated bulk onboarding mutation. Without `confirm`
/// it returns per-item previews plus the digest and writes nothing; a
/// mismatched digest is refused with a fresh preview and no write; only a
/// confirmed matching digest applies each item in order and reports honest
/// per-item results (`202` all ok, `207` partial).
pub(super) fn workspace_onboard_write(
    config: &ApiConfig,
    db_path: &Path,
    request: &ApiRequest,
) -> ApiResponse {
    if !is_json(request) {
        return cors(
            config,
            request,
            error(
                415,
                "admin-content-type-required",
                "workspace onboarding mutations require application/json",
            ),
        );
    }
    cors(
        config,
        request,
        guarded(db_path, request, |req| {
            let body = req.json_body();
            let items = match parse_onboard_items(&body) {
                Ok(items) => items,
                Err(response) => return response,
            };
            let root = match projects_root() {
                Ok(root) => root,
                Err(response) => return response,
            };
            let projects = Registry::open(db_path)
                .ok()
                .and_then(|registry| registry.list().ok())
                .unwrap_or_default();
            let plans: Vec<Value> = items
                .iter()
                .map(|item| onboard_preview(&projects, &root, item))
                .collect();
            // The digest binds the reviewed root plus the exact per-item
            // plans; the root travels as a hash input only, never serialized.
            let descriptor = json!({
                "action": "workspace-onboard",
                "root": root.display().to_string(),
                "items": plans,
            });
            let digest = authoring_digest(&descriptor);
            let confirm = body
                .get("confirm")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            if !confirm {
                return ApiResponse::json(
                    200,
                    json!({
                        "preview": plans,
                        "plan_digest": digest,
                        "confirmation": {
                            "requires": ["confirm", "plan_digest"],
                            "note": "This preview writes nothing. To onboard this exact selection, send `confirm: true` with this exact `plan_digest`; a changed set is refused.",
                        },
                        "contract": API_CONTRACT_VERSION,
                    }),
                );
            }
            let supplied = body
                .get("plan_digest")
                .and_then(Value::as_str)
                .unwrap_or("");
            if supplied != digest {
                return ApiResponse::json(
                    409,
                    json!({
                        "error": {
                            "code": "admin-digest-mismatch",
                            "message": "the confirmed digest does not match this onboarding preview; nothing was written. Review the refreshed preview and confirm its new digest.",
                        },
                        "preview": plans,
                        "plan_digest": digest,
                        "contract": API_CONTRACT_VERSION,
                    }),
                );
            }
            if plans.iter().any(|plan| plan.get("blocked").is_some()) {
                return ApiResponse::json(
                    409,
                    json!({
                        "error": {
                            "code": "admin-onboard-blocked",
                            "message": "one or more selected items cannot be onboarded as previewed; adjust the selection or overrides and preview again. Nothing was written.",
                        },
                        "preview": plans,
                        "plan_digest": digest,
                        "contract": API_CONTRACT_VERSION,
                    }),
                );
            }
            let results: Vec<Value> = items
                .iter()
                .zip(plans.iter())
                .map(|(item, plan)| onboard_apply(db_path, &root, item, plan))
                .collect();
            let succeeded = results
                .iter()
                .filter(|result| result.get("ok").and_then(Value::as_bool).unwrap_or(false))
                .count();
            let state = if succeeded == results.len() {
                "done"
            } else {
                "failed"
            };
            let status = if succeeded == results.len() { 202 } else { 207 };
            if let Ok(registry) = Registry::open(db_path) {
                let _ = registry.record_operation(
                    "admin.workspace.onboard",
                    super::API_SYNTHETIC_PROJECT,
                    state,
                    &format!("workspace onboard: {succeeded}/{} succeeded", results.len()),
                );
            }
            ApiResponse::json(
                status,
                json!({
                    "results": results,
                    "succeeded": succeeded,
                    "failed": results.len() - succeeded,
                    "contract": API_CONTRACT_VERSION,
                }),
            )
        }),
    )
}
