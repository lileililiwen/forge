//! Session-gated project management (`forge-web-project-management`).
//!
//! Browser project creation (`new`), adoption (`import`) and manifest
//! registration (`register`) as preview + confirm/digest-bound typed routes.
//! The browser supplies validated names and typed fields only; every
//! destination is resolved server-side under the operator-configured root
//! (`FORGE_ADMIN_PROJECTS_ROOT`, runtime-only operator configuration).

use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use super::admin::{
    authoring_digest, cors, error, guarded, is_json, scrub_json, scrub_response, scrub_text,
};
use super::{ApiConfig, ApiRequest, ApiResponse, API_CONTRACT_VERSION};
use crate::core::manifest::Manifest;
use crate::registry::Registry;

/// The `forge new` creation command exposed as a session-gated, preview +
/// confirm/digest-bound admin route. The browser supplies only a validated
/// single-segment project name plus typed fields; the destination directory is
/// resolved server-side from [`ADMIN_PROJECTS_ROOT_ENV`]. Exported so the
/// command catalog can name the exact path the router registers.
pub const ROUTE_ADMIN_PROJECT_NEW: &str = "POST /v1/admin/projects/new";

/// The `forge import` adoption command exposed as a session-gated, preview +
/// confirm/digest-bound admin route. The destination is resolved server-side;
/// the browser never supplies a path.
pub const ROUTE_ADMIN_PROJECT_IMPORT: &str = "POST /v1/admin/projects/import";

/// The `forge register` command exposed as a session-gated, preview +
/// confirm/digest-bound admin route. The destination is resolved server-side;
/// the browser never supplies a path.
pub const ROUTE_ADMIN_PROJECT_REGISTER: &str = "POST /v1/admin/projects/register";

/// The server-side directory that browser-driven project creation, import and
/// registration are confined to. Left **unset by default**: an unset, blank or
/// non-directory value is a typed `409 admin-prerequisite`, mirroring the
/// delivery surface's `FORGE_SHARE_PUBLISH_TARGET`. The operator declares the
/// location; the browser never names one, so no web write can target a path the
/// operator did not explicitly opt into.
pub const ADMIN_PROJECTS_ROOT_ENV: &str = "FORGE_ADMIN_PROJECTS_ROOT";

/// Which handler-backed project-management command a
/// `/v1/admin/projects/{new,import,register}` write runs. A closed enum — the
/// browser picks one by the URL segment, and no free-form command, path or argv
/// ever reaches the handler.
#[derive(Clone, Copy)]
pub(super) enum ProjectManagement {
    New,
    Import,
    Register,
}

impl ProjectManagement {
    fn action(self) -> &'static str {
        match self {
            ProjectManagement::New => "project-new",
            ProjectManagement::Import => "project-import",
            ProjectManagement::Register => "project-register",
        }
    }

    fn operation_kind(self) -> &'static str {
        match self {
            ProjectManagement::New => "admin.project.new",
            ProjectManagement::Import => "admin.project.import",
            ProjectManagement::Register => "admin.project.register",
        }
    }
}

/// Build the canonical, **path-free** descriptor of a project-management action
/// from its structured fields. It refuses a missing field, a non-kebab
/// `project` name (so no path separator, traversal segment, percent-encoding or
/// uppercase reaches the filesystem) and an invalid `id` override, so a preview
/// never reports a digest for an action that cannot run. The absolute
/// destination is deliberately absent: the digest binds the reviewed fields, not
/// a machine location.
fn management_descriptor(
    kind: ProjectManagement,
    body: &Value,
) -> Result<Value, (&'static str, &'static str)> {
    let project = body
        .get("project")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or(("admin-field-required", "a `project` name is required"))?;
    if crate::core::validate_project_id(project).is_err() {
        return Err((
            "admin-invalid-project-name",
            "the `project` name must be a lowercase kebab-case identifier with no path separators.",
        ));
    }
    match kind {
        ProjectManagement::New => {
            let profile = body
                .get("profile")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .ok_or((
                    "admin-field-required",
                    "creating a project requires a `profile` field",
                ))?;
            let mut descriptor = json!({
                "action": kind.action(),
                "project": project,
                "profile": profile,
            });
            if let Some(name) = body
                .get("name")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                descriptor["name"] = json!(name);
            }
            let mut features: Vec<&str> = body
                .get("features")
                .and_then(Value::as_array)
                .map(|arr| {
                    arr.iter()
                        .filter_map(Value::as_str)
                        .filter(|value| !value.trim().is_empty())
                        .collect()
                })
                .unwrap_or_default();
            features.sort_unstable();
            features.dedup();
            if !features.is_empty() {
                descriptor["features"] = json!(features);
            }
            Ok(descriptor)
        }
        ProjectManagement::Import => {
            let mut descriptor = json!({ "action": kind.action(), "project": project });
            if let Some(profile) = body
                .get("profile")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                descriptor["profile"] = json!(profile);
            }
            if let Some(id) = body
                .get("id")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                if crate::core::validate_project_id(id).is_err() {
                    return Err((
                        "admin-invalid-project-name",
                        "the `id` override must be a lowercase kebab-case identifier with no path separators.",
                    ));
                }
                descriptor["id"] = json!(id);
            }
            Ok(descriptor)
        }
        ProjectManagement::Register => Ok(json!({ "action": kind.action(), "project": project })),
    }
}

/// Resolve the server-side project root. Unset, blank or non-directory is a
/// typed `409 admin-prerequisite`; the configured value is never echoed. The
/// root is canonicalized once so every destination is compared against the same
/// resolved prefix.
pub(super) fn projects_root() -> Result<PathBuf, ApiResponse> {
    let value = std::env::var(ADMIN_PROJECTS_ROOT_ENV).unwrap_or_default();
    if value.trim().is_empty() {
        return Err(error(
            409,
            "admin-prerequisite",
            "the server has no project root configured; set FORGE_ADMIN_PROJECTS_ROOT to a workspace directory in the server environment before managing projects from the browser.",
        ));
    }
    match PathBuf::from(value.trim()).canonicalize() {
        Ok(canonical) if canonical.is_dir() => Ok(canonical),
        _ => Err(error(
            409,
            "admin-prerequisite",
            "the configured server-side project root does not resolve to an existing directory; fix the server configuration and retry.",
        )),
    }
}

/// Join a validated single-segment project name to the resolved root. For
/// `new` the destination may not exist yet; for `register`/`import` the
/// directory must exist, canonicalize (resolving symlinks) and stay a descendant
/// of the root, so a symlink out of the root is refused before any read.
fn management_destination(
    kind: ProjectManagement,
    root: &Path,
    name: &str,
) -> Result<PathBuf, ApiResponse> {
    let candidate = root.join(name);
    if matches!(kind, ProjectManagement::New) {
        return Ok(candidate);
    }
    let canonical = candidate.canonicalize().map_err(|_| {
        error(
            409,
            "admin-prerequisite",
            "the named project directory does not exist under the configured root; create or place it there first.",
        )
    })?;
    if !canonical.starts_with(root) {
        return Err(error(
            409,
            "admin-prerequisite",
            "the named project directory resolves outside the configured root; refusing to touch it.",
        ));
    }
    if !canonical.is_dir() {
        return Err(error(
            409,
            "admin-prerequisite",
            "the named project path is not a directory under the configured root.",
        ));
    }
    Ok(canonical)
}

/// Build the read-only, path-free preview for one management action. It runs
/// only the Core function's read side (`normalize_explicit` validation,
/// `inspect_import` proposal, manifest load) and writes nothing. A Core failure
/// is returned as an already-typed response (scrubbed by the caller).
fn management_preview(
    kind: ProjectManagement,
    descriptor: &Value,
    dest: &Path,
) -> Result<Value, ApiResponse> {
    match kind {
        ProjectManagement::New => {
            let profile = descriptor["profile"].as_str().unwrap_or_default();
            let project = descriptor["project"].as_str().unwrap_or_default();
            let name = descriptor.get("name").and_then(Value::as_str);
            let features: Vec<String> = descriptor
                .get("features")
                .and_then(Value::as_array)
                .map(|arr| {
                    arr.iter()
                        .filter_map(Value::as_str)
                        .map(str::to_string)
                        .collect()
                })
                .unwrap_or_default();
            let request = crate::generate::normalize_explicit(
                Some(profile),
                Some(project),
                name,
                &features,
                dest,
                None,
            )
            .map_err(|err| management_error(&err, &[dest]))?;
            Ok(json!({
                "action": "project-new",
                "project": request.id,
                "profile": request.profile,
                "name": request.name,
                "features": request.features,
            }))
        }
        ProjectManagement::Import => {
            let profile = descriptor.get("profile").and_then(Value::as_str);
            let proposal = crate::import::inspect_import(dest, profile)
                .map_err(|err| management_error(&err, &[dest]))?;
            Ok(json!({
                "action": "project-import",
                "suggested_profile": proposal.suggested_profile,
                "suggested_maturity": proposal.suggested_maturity,
                "confidence": proposal.confidence,
                "language": proposal.language.value,
                "manifest_exists": proposal.manifest_exists,
                "explicit_profile": proposal.explicit_profile,
                "alternatives": proposal.alternatives,
            }))
        }
        ProjectManagement::Register => {
            let (manifest, _) = Manifest::load_from_dir(dest, None)
                .map_err(|err| management_error(&err, &[dest]))?;
            Ok(json!({
                "action": "project-register",
                "manifest": {
                    "id": manifest.project.id,
                    "name": manifest.project.name,
                    "profile": manifest.project.profile,
                },
                "ready": true,
            }))
        }
    }
}

/// One JSON-only, session-gated, preview-then-confirm project-management
/// mutation (`forge new` / `forge import` / `forge register`). The browser
/// supplies only structured typed fields; the destination is resolved from the
/// server-side root. Without `confirm: true` it returns a path-free preview and
/// the digest and writes nothing; a mismatched digest is a `409` with a fresh
/// preview and no write; only a confirmed matching digest delegates to the same
/// in-process Core function the CLI runs, journaled through the shared operation
/// boundary. Every response is scrubbed of the root and destination.
pub(super) fn management_write(
    config: &ApiConfig,
    db_path: &Path,
    request: &ApiRequest,
    kind: ProjectManagement,
) -> ApiResponse {
    if !is_json(request) {
        return cors(
            config,
            request,
            error(
                415,
                "admin-content-type-required",
                "project-management mutations require application/json",
            ),
        );
    }
    cors(
        config,
        request,
        guarded(db_path, request, |req| {
            let body = req.json_body();
            let descriptor = match management_descriptor(kind, &body) {
                Ok(value) => value,
                Err((code, message)) => return error(400, code, message),
            };
            let digest = authoring_digest(&descriptor);
            let root = match projects_root() {
                Ok(root) => root,
                Err(response) => return response,
            };
            let project = descriptor["project"].as_str().unwrap_or_default();
            let dest = match management_destination(kind, &root, project) {
                Ok(dest) => dest,
                Err(response) => return response,
            };
            let secrets = [root.display().to_string(), dest.display().to_string()];

            let confirm = body
                .get("confirm")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            if !confirm {
                let preview = match management_preview(kind, &descriptor, &dest) {
                    Ok(view) => scrub_json(view, &secrets),
                    Err(response) => return scrub_response(response, &secrets),
                };
                return ApiResponse::json(
                    200,
                    json!({
                        "preview": preview,
                        "plan_digest": digest,
                        "confirmation": {
                            "requires": ["confirm", "plan_digest"],
                            "note": "This preview writes nothing. To run the action, send `confirm: true` with this exact `plan_digest`; a changed or stale digest is refused.",
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
                let preview = match management_preview(kind, &descriptor, &dest) {
                    Ok(view) => scrub_json(view, &secrets),
                    Err(response) => return scrub_response(response, &secrets),
                };
                return ApiResponse::json(
                    409,
                    json!({
                        "error": {
                            "code": "admin-digest-mismatch",
                            "message": "the confirmed digest does not match this action's current preview; nothing was written. Review the refreshed preview and confirm its new digest.",
                        },
                        "preview": preview,
                        "plan_digest": digest,
                        "contract": API_CONTRACT_VERSION,
                    }),
                );
            }
            // Confirmed and matching: delegate to the same in-process Core
            // function the CLI runs, journaled as `admin.project.*`. No shell,
            // argv or browser path ever reaches the handler.
            let kind_name = kind.operation_kind();
            let result = match kind {
                ProjectManagement::New => {
                    run_management_new(db_path, req, kind_name, &descriptor, &dest)
                }
                ProjectManagement::Import => {
                    run_management_import(db_path, req, kind_name, &descriptor, &dest)
                }
                ProjectManagement::Register => {
                    run_management_register(db_path, req, kind_name, &dest)
                }
            };
            match result {
                Ok(response) => response,
                Err(response) => scrub_response(response, &secrets),
            }
        }),
    )
}

/// Confirmed `forge new`: normalize the typed fields against the server-resolved
/// destination and run the deterministic `generate` Core function under the
/// shared operation journal. Returns a path-free `created` view.
fn run_management_new(
    db_path: &Path,
    req: &ApiRequest,
    kind: &str,
    descriptor: &Value,
    dest: &Path,
) -> Result<ApiResponse, ApiResponse> {
    let profile = descriptor["profile"].as_str().unwrap_or_default();
    let project = descriptor["project"].as_str().unwrap_or_default();
    let name = descriptor.get("name").and_then(Value::as_str);
    let features: Vec<String> = descriptor
        .get("features")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    let normalized = crate::generate::normalize_explicit(
        Some(profile),
        Some(project),
        name,
        &features,
        dest,
        None,
    )
    .map_err(|err| ApiResponse::from_error(&err))?;
    let (view, op_id, project_id) = super::run_with_operation(
        db_path,
        kind,
        super::API_SYNTHETIC_PROJECT,
        req,
        |op_id, _registry| {
            let mut registry = Registry::open(db_path)?;
            let generated = crate::generate::generate(&mut registry, &normalized)?;
            Ok((
                json!({
                    "created": {
                        "id": generated.record.id,
                        "profile": generated.record.profile,
                        "name": generated.record.name,
                        "files": generated.files,
                    }
                }),
                op_id,
                generated.record.id,
            ))
        },
    )?;
    Ok(managed_response(view, op_id, project_id))
}

/// Confirmed `forge import`: adopt an existing directory through the same
/// `adopt_import` Core function the CLI runs, under the shared operation
/// journal. Returns a path-free `imported` view.
fn run_management_import(
    db_path: &Path,
    req: &ApiRequest,
    kind: &str,
    descriptor: &Value,
    dest: &Path,
) -> Result<ApiResponse, ApiResponse> {
    let profile = descriptor.get("profile").and_then(Value::as_str);
    let id = descriptor.get("id").and_then(Value::as_str);
    let (view, op_id, project_id) = super::run_with_operation(
        db_path,
        kind,
        super::API_SYNTHETIC_PROJECT,
        req,
        |op_id, _registry| {
            let mut registry = Registry::open(db_path)?;
            let record = crate::import::adopt_import(&mut registry, dest, profile, id)?;
            Ok((
                json!({
                    "imported": {
                        "id": record.id,
                        "name": record.name,
                        "profile": record.profile,
                    }
                }),
                op_id,
                record.id,
            ))
        },
    )?;
    Ok(managed_response(view, op_id, project_id))
}

/// Confirmed `forge register`: persist an existing manifest directory through
/// the same `Registry::register` Core function the CLI runs, under the shared
/// operation journal. Returns a path-free `registered` view.
fn run_management_register(
    db_path: &Path,
    req: &ApiRequest,
    kind: &str,
    dest: &Path,
) -> Result<ApiResponse, ApiResponse> {
    let (view, op_id, project_id) = super::run_with_operation(
        db_path,
        kind,
        super::API_SYNTHETIC_PROJECT,
        req,
        |op_id, _registry| {
            let mut registry = Registry::open(db_path)?;
            let record = registry.register(dest, None)?;
            Ok((
                json!({
                    "registered": {
                        "id": record.id,
                        "name": record.name,
                        "profile": record.profile,
                    }
                }),
                op_id,
                record.id,
            ))
        },
    )?;
    Ok(managed_response(view, op_id, project_id))
}

/// Attach the operation id, resolved project id and contract to a
/// project-management success view.
fn managed_response(mut view: Value, op_id: i64, project_id: String) -> ApiResponse {
    view["operation_id"] = json!(op_id);
    view["project_id"] = json!(project_id);
    view["contract"] = json!(API_CONTRACT_VERSION);
    ApiResponse::json(202, view)
}

/// Render a project-management Core failure with the shared status mapping,
/// scrubbing the resolved destination (the only absolute path the preview can
/// leak) from the message. The typed code and status are preserved.
fn management_error(err: &crate::core::ForgeError, secrets: &[&Path]) -> ApiResponse {
    let owned: Vec<String> = secrets.iter().map(|p| p.display().to_string()).collect();
    let status = super::err_status(err);
    ApiResponse::json(
        status,
        json!({
            "error": {
                "code": err.code(),
                "message": scrub_text(&err.to_string(), &owned),
            },
            "contract": API_CONTRACT_VERSION,
        }),
    )
}
