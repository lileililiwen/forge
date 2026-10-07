//! JSON-only Forge-wide administrator endpoints used by `frontend/`.

use std::path::Path;

use chrono::Utc;
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use super::{ApiConfig, ApiRequest, ApiResponse, Route, API_CONTRACT_VERSION};
use crate::identity::global;
use crate::registry::Registry;

const COOKIE: &str = "forge_admin_session";

/// The `forge feature add` authoring command exposed as a session-gated,
/// preview + confirm/digest-bound admin route. Exported so the command catalog
/// can name the exact path the router registers, keeping the two in lockstep.
pub const ROUTE_ADMIN_FEATURE: &str = "POST /v1/admin/projects/{id}/feature";

/// The `forge spec generate` authoring command exposed as a session-gated,
/// preview + confirm/digest-bound admin route.
pub const ROUTE_ADMIN_SPEC: &str = "POST /v1/admin/projects/{id}/spec";

/// The `forge feature remove` lifecycle write, exposed as a session-gated,
/// preview + confirm/digest-bound admin route delegating to the same
/// `remove_feature` Core handler the CLI runs. Exported so the command
/// catalog names the exact path the router registers.
pub const ROUTE_ADMIN_FEATURE_REMOVE: &str = "POST /v1/admin/projects/{id}/feature/remove";

/// The `forge feature upgrade` lifecycle write, exposed as a session-gated,
/// preview + confirm/digest-bound admin route delegating to the same
/// `upgrade_feature` Core handler the CLI runs.
pub const ROUTE_ADMIN_FEATURE_UPGRADE: &str = "POST /v1/admin/projects/{id}/feature/upgrade";

/// The `forge spec apply` lifecycle write, exposed as a session-gated,
/// preview + confirm/digest-bound admin route delegating to the same
/// `apply_routing` Core handler the CLI runs.
pub const ROUTE_ADMIN_SPEC_APPLY: &str = "POST /v1/admin/projects/{id}/spec/apply";

#[derive(Deserialize)]
struct LoginBody {
    email: String,
    password: String,
}

pub(super) fn handle(
    config: &ApiConfig,
    db_path: &Path,
    request: &ApiRequest,
    route: &Route,
) -> ApiResponse {
    if !allowed_origin(config, request.header("origin")) {
        return cors(
            config,
            request,
            error(
                403,
                "admin-origin-rejected",
                "request origin is not allowed",
            ),
        );
    }

    let result = match route {
        Route::AdminSessionGet => session_state(db_path, request),
        Route::AdminSessionPost => sign_in(config, db_path, request),
        Route::AdminSessionDelete => sign_out(config, db_path, request),
        Route::AdminProjects => projects(db_path, request),
        Route::AdminCommands => commands(db_path, request),
        Route::AdminProjectDetail { id } => {
            guarded(db_path, request, |_| super::workbench::detail(db_path, id))
        }
        Route::AdminProjectPlan { id } => guarded(db_path, request, |req| {
            let feature = req.query.as_deref().and_then(parse_feature_query);
            super::workbench::plan(db_path, id, feature.as_deref())
        }),
        Route::AdminProjectApply { id } => {
            if !is_json(request) {
                return cors(
                    config,
                    request,
                    error(
                        415,
                        "admin-content-type-required",
                        "apply requires application/json",
                    ),
                );
            }
            guarded(db_path, request, |req| {
                let body = req.json_body();
                super::workbench::apply(db_path, id, &body, req)
            })
        }
        Route::AdminPortfolioList => {
            guarded(db_path, request, |req| super::portfolio::list(db_path, req))
        }
        Route::AdminPortfolioEvidence => {
            guarded(db_path, request, |_| super::portfolio::evidence(db_path))
        }
        Route::AdminPortfolioProject { id } => {
            guarded(db_path, request, |_| super::portfolio::detail(db_path, id))
        }
        Route::AdminPortfolioRead { id, kind } => guarded(db_path, request, |_| {
            super::portfolio::read_item(db_path, id, kind)
        }),
        Route::AdminPortfolioWrite { id, action } => {
            if !is_json(request) {
                return cors(
                    config,
                    request,
                    error(
                        415,
                        "admin-content-type-required",
                        "portfolio writes require application/json",
                    ),
                );
            }
            guarded(db_path, request, |req| {
                let body = req.json_body();
                super::portfolio::write_item(db_path, id, action, &body)
            })
        }
        Route::AdminDelivery => guarded(db_path, request, |_| super::delivery::overview(db_path)),
        Route::AdminDeliveryPreview => {
            guarded(db_path, request, |_| super::delivery::preview(db_path))
        }
        Route::AdminDeliveryOperation { key } => guarded(db_path, request, |_| {
            super::delivery::operation(db_path, key)
        }),
        Route::AdminDeliveryAllowlist { id } => {
            delivery_write(config, db_path, request, |req, body| {
                super::delivery::allowlist_set(db_path, id, &body, req)
            })
        }
        Route::AdminDeliveryAllowlistRemove { id } => {
            delivery_write(config, db_path, request, |req, body| {
                super::delivery::allowlist_remove(db_path, id, &body, req)
            })
        }
        Route::AdminDeliveryApprove => delivery_write(config, db_path, request, |req, body| {
            super::delivery::approve(db_path, &body, req)
        }),
        Route::AdminDeliveryPublish => delivery_write(config, db_path, request, |req, body| {
            super::delivery::publish(db_path, &body, req)
        }),
        Route::AdminDeliveryReconcile => delivery_write(config, db_path, request, |req, body| {
            super::delivery::reconcile(db_path, &body, req)
        }),
        Route::AdminProjectFeature { id } => {
            authoring_write(config, db_path, request, id, Authoring::FeatureAdd)
        }
        Route::AdminProjectFeatureRemove { id } => {
            authoring_write(config, db_path, request, id, Authoring::FeatureRemove)
        }
        Route::AdminProjectFeatureUpgrade { id } => {
            authoring_write(config, db_path, request, id, Authoring::FeatureUpgrade)
        }
        Route::AdminProjectSpec { id } => {
            authoring_write(config, db_path, request, id, Authoring::SpecGenerate)
        }
        Route::AdminProjectSpecApply { id } => {
            authoring_write(config, db_path, request, id, Authoring::SpecApply)
        }
        _ => error(404, "route-not-found", "no admin route matches the request"),
    };
    cors(config, request, result)
}

/// Which handler-backed authoring command a `/v1/admin/projects/{id}/…` write
/// runs. A closed enum — the browser picks one of these by the URL segment, and
/// no free-form command, path or argv ever reaches the handler.
#[derive(Clone, Copy)]
enum Authoring {
    FeatureAdd,
    FeatureRemove,
    FeatureUpgrade,
    SpecGenerate,
    SpecApply,
}

/// Build the canonical, path-free descriptor of an authoring action from its
/// structured fields, refusing (via `Err`) when a required field is missing or
/// malformed so a preview never reports a digest for an action that cannot run.
fn authoring_descriptor(
    kind: Authoring,
    id: &str,
    body: &Value,
) -> Result<Value, (&'static str, &'static str)> {
    match kind {
        Authoring::FeatureAdd => {
            let feature = body
                .get("feature")
                .and_then(Value::as_str)
                .filter(|value| !value.trim().is_empty())
                .ok_or((
                    "admin-feature-required",
                    "feature add requires a `feature` field",
                ))?;
            let mut descriptor = json!({
                "action": "feature-add",
                "project_id": id,
                "feature": feature,
            });
            if let Some(version) = body.get("version").and_then(Value::as_str) {
                descriptor["version"] = json!(version);
            }
            Ok(descriptor)
        }
        Authoring::SpecGenerate => {
            let findings: Vec<&str> = body
                .get("findings")
                .and_then(Value::as_array)
                .map(|arr| {
                    arr.iter()
                        .filter_map(Value::as_str)
                        .filter(|value| !value.trim().is_empty())
                        .collect()
                })
                .unwrap_or_default();
            if findings.is_empty() {
                return Err((
                    "admin-findings-required",
                    "spec generate requires at least one finding id",
                ));
            }
            let mut descriptor = json!({
                "action": "spec-generate",
                "project_id": id,
                "findings": findings,
            });
            if let Some(reason) = body.get("reason").and_then(Value::as_str) {
                descriptor["reason"] = json!(reason);
            }
            Ok(descriptor)
        }
        Authoring::FeatureRemove => {
            let feature = body
                .get("feature")
                .and_then(Value::as_str)
                .filter(|value| !value.trim().is_empty())
                .ok_or((
                    "admin-feature-required",
                    "feature remove requires a `feature` field",
                ))?;
            Ok(json!({
                "action": "feature-remove",
                "project_id": id,
                "feature": feature,
            }))
        }
        Authoring::FeatureUpgrade => {
            let feature = body
                .get("feature")
                .and_then(Value::as_str)
                .filter(|value| !value.trim().is_empty())
                .ok_or((
                    "admin-feature-required",
                    "feature upgrade requires a `feature` field",
                ))?;
            let mut descriptor = json!({
                "action": "feature-upgrade",
                "project_id": id,
                "feature": feature,
            });
            if let Some(version) = body.get("version").and_then(Value::as_str) {
                descriptor["version"] = json!(version);
            }
            Ok(descriptor)
        }
        Authoring::SpecApply => {
            let findings: Vec<&str> = body
                .get("findings")
                .and_then(Value::as_array)
                .map(|arr| {
                    arr.iter()
                        .filter_map(Value::as_str)
                        .filter(|value| !value.trim().is_empty())
                        .collect()
                })
                .unwrap_or_default();
            if findings.is_empty() {
                return Err((
                    "admin-findings-required",
                    "spec apply requires at least one finding id",
                ));
            }
            let mut descriptor = json!({
                "action": "spec-apply",
                "project_id": id,
                "findings": findings,
            });
            if let Some(reason) = body.get("reason").and_then(Value::as_str) {
                descriptor["reason"] = json!(reason);
            }
            Ok(descriptor)
        }
    }
}

/// Hex SHA-256 of the canonical descriptor bytes, matching the workbench plan
/// digest. The descriptor carries no absolute path, so the digest binds
/// confirmation to the exact project, action and structured field set.
fn authoring_digest(descriptor: &Value) -> String {
    let bytes = serde_json::to_vec(descriptor).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let out = hasher.finalize();
    let mut hex = String::with_capacity(out.len() * 2);
    for byte in out {
        use std::fmt::Write as _;
        let _ = write!(hex, "{byte:02x}");
    }
    hex
}

/// One JSON-only, session-gated, preview-then-confirm authoring mutation. It
/// never trusts a browser to run the action implicitly: without `confirm: true`
/// it returns only the canonical descriptor plus its `plan_digest` and runs no
/// Core write; with `confirm: true` it recomputes the digest and, only if the
/// supplied digest still matches, delegates to the same in-process Core handler
/// the CLI and `/v1` bearer route use. A stale or forged digest is refused with
/// a fresh digest and no write. Structured fields only — never a shell, argv or
/// browser-supplied path.
fn authoring_write(
    config: &ApiConfig,
    db_path: &Path,
    request: &ApiRequest,
    id: &str,
    kind: Authoring,
) -> ApiResponse {
    if !is_json(request) {
        return cors(
            config,
            request,
            error(
                415,
                "admin-content-type-required",
                "authoring mutations require application/json",
            ),
        );
    }
    cors(
        config,
        request,
        guarded(db_path, request, |req| {
            // Id gate (design §4): the route `{id}` must be a valid kebab-case
            // identifier that resolves to a project managed by this registry,
            // checked before any descriptor, digest or Core call. A hostile,
            // path-bearing, unknown or observed-only id is refused with a
            // static typed error — the offending input is never echoed and no
            // absolute path is ever serialized.
            if crate::core::validate_project_id(id).is_err() {
                return error(
                    400,
                    "admin-invalid-project-id",
                    "the project id is not a valid identifier; it may not contain a path.",
                );
            }
            let managed = Registry::open(db_path)
                .ok()
                .and_then(|registry| registry.inspect(id).ok())
                .is_some();
            if !managed {
                return error(
                    404,
                    "admin-project-unmanaged",
                    "this project is not managed by this Forge registry; register it with `forge register <path>` in a terminal first.",
                );
            }
            let body = req.json_body();
            let descriptor = match authoring_descriptor(kind, id, &body) {
                Ok(value) => value,
                Err((code, message)) => return error(400, code, message),
            };
            let digest = authoring_digest(&descriptor);
            let confirm = body
                .get("confirm")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            if !confirm {
                return ApiResponse::json(
                    200,
                    json!({
                        "preview": descriptor,
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
                return ApiResponse::json(
                    409,
                    json!({
                        "error": {
                            "code": "admin-digest-mismatch",
                            "message": "the confirmed digest does not match this action's current preview; nothing was written. Review the refreshed preview and confirm its new digest.",
                        },
                        "preview": descriptor,
                        "plan_digest": digest,
                        "contract": API_CONTRACT_VERSION,
                    }),
                );
            }
            let now = Utc::now();
            match kind {
                Authoring::FeatureAdd => super::handle_add_feature(db_path, id, req, now),
                Authoring::FeatureRemove => super::handle_remove_feature(db_path, id, req, now),
                Authoring::FeatureUpgrade => super::handle_upgrade_feature(db_path, id, req, now),
                Authoring::SpecGenerate => super::handle_generate_spec(db_path, id, req, now),
                Authoring::SpecApply => super::handle_apply_spec(db_path, id, req, now),
            }
        }),
    )
}

/// One JSON-only, session-gated delivery mutation: a non-JSON content type
/// is refused before the session gate ever runs the handler, and the
/// handler itself enforces the confirm- and digest-binding. Delivery
/// actions are typed in-process Core calls — never a shell, never a
/// browser-supplied path.
fn delivery_write<F>(config: &ApiConfig, db_path: &Path, request: &ApiRequest, f: F) -> ApiResponse
where
    F: FnOnce(&ApiRequest, serde_json::Value) -> ApiResponse,
{
    if !is_json(request) {
        return cors(
            config,
            request,
            error(
                415,
                "admin-content-type-required",
                "delivery mutations require application/json",
            ),
        );
    }
    cors(
        config,
        request,
        guarded(db_path, request, |req| f(req, req.json_body())),
    )
}

/// Run `f` only after the same global-admin session gate every other admin
/// read/write uses. A missing or expired session is a `401`, never a partial
/// workbench projection; a registry failure is an honest `503`. The workbench
/// `f` closures perform only typed in-process Core calls — no shell.
fn guarded<F>(db_path: &Path, request: &ApiRequest, f: F) -> ApiResponse
where
    F: FnOnce(&ApiRequest) -> ApiResponse,
{
    let token = request
        .cookies
        .get(COOKIE)
        .map(String::as_str)
        .unwrap_or("");
    match global::session_valid(db_path, token) {
        Ok(true) => f(request),
        Ok(false) => error(
            401,
            "api-unauthorized",
            "Forge administrator session is required",
        ),
        Err(_) => unavailable(),
    }
}

/// True when the request declares a JSON content type, so a mutating apply
/// never parses a body of an unexpected media type.
fn is_json(request: &ApiRequest) -> bool {
    request
        .header("content-type")
        .is_some_and(|value| value.to_ascii_lowercase().starts_with("application/json"))
}

/// Extract the optional `feature` query parameter (the one catalog feature id
/// to upgrade). It is percent-decoded like every other browser query value and
/// used only as a catalog feature key by Core — never as a path or argument.
fn parse_feature_query(query: &str) -> Option<String> {
    for pair in query.split('&').filter(|part| !part.is_empty()) {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        if key == "feature" && !value.is_empty() {
            return Some(super::percent_decode(value));
        }
    }
    None
}

pub(super) fn handle_preflight(config: &ApiConfig, request: &ApiRequest) -> ApiResponse {
    if !allowed_origin(config, request.header("origin")) {
        return error(
            403,
            "admin-origin-rejected",
            "request origin is not allowed",
        );
    }
    ApiResponse {
        status: 204,
        headers: cors_headers(config, request),
        body: Vec::new(),
    }
}

fn session_state(db_path: &Path, request: &ApiRequest) -> ApiResponse {
    match global::is_configured(db_path) {
        Ok(configured) => {
            let authenticated = request
                .cookies
                .get(COOKIE)
                .filter(|token| !token.is_empty())
                .and_then(|token| global::session_valid(db_path, token).ok())
                .unwrap_or(false);
            let mut response = ApiResponse::json(
                200,
                json!({
                    "configured": configured,
                    "authenticated": authenticated,
                    "setup_command": if configured { serde_json::Value::Null } else { json!("forge identity setup --email you@example.com") },
                    "contract": API_CONTRACT_VERSION,
                }),
            );
            if request.cookies.contains_key(COOKIE) && !authenticated {
                response.headers.insert(
                    "set-cookie".to_string(),
                    format!(
                        "{COOKIE}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0{}",
                        if request
                            .header("origin")
                            .unwrap_or("")
                            .starts_with("https://")
                        {
                            "; Secure"
                        } else {
                            ""
                        }
                    ),
                );
            }
            response
        }
        Err(_) => unavailable(),
    }
}

fn sign_in(config: &ApiConfig, db_path: &Path, request: &ApiRequest) -> ApiResponse {
    if request.body.len() > 8192 {
        return error(413, "admin-request-too-large", "login request is too large");
    }
    if !request
        .header("content-type")
        .is_some_and(|value| value.to_ascii_lowercase().starts_with("application/json"))
    {
        return error(
            415,
            "admin-content-type-required",
            "login requires application/json",
        );
    }
    let input: LoginBody = match serde_json::from_slice(&request.body) {
        Ok(value) => value,
        Err(_) => {
            return error(
                400,
                "admin-invalid-request",
                "email and password are required",
            )
        }
    };
    let token = match global::authenticate(db_path, &input.email, &input.password) {
        Ok(Some(token)) => token,
        Ok(None) => return error(401, "api-unauthorized", "email or password is incorrect"),
        Err(_) => return unavailable(),
    };
    let mut response = ApiResponse::json(
        200,
        json!({ "authenticated": true, "contract": API_CONTRACT_VERSION }),
    );
    response.headers.insert(
        "set-cookie".to_string(),
        format!(
            "{COOKIE}={token}; Path=/; HttpOnly; SameSite=Lax; Max-Age=43200{}",
            if config.frontend_origin.starts_with("https://") {
                "; Secure"
            } else {
                ""
            }
        ),
    );
    response
}

fn sign_out(config: &ApiConfig, db_path: &Path, request: &ApiRequest) -> ApiResponse {
    let Some(token) = request
        .cookies
        .get(COOKIE)
        .filter(|value| !value.is_empty())
    else {
        return error(
            401,
            "api-unauthorized",
            "Forge administrator session is required",
        );
    };
    match global::session_valid(db_path, token) {
        Ok(true) => {}
        Ok(false) => {
            return error(
                401,
                "api-unauthorized",
                "Forge administrator session is required",
            )
        }
        Err(_) => return unavailable(),
    }
    match global::revoke(db_path, token) {
        Ok(()) => {
            let mut response = ApiResponse::json(
                200,
                json!({ "authenticated": false, "contract": API_CONTRACT_VERSION }),
            );
            response.headers.insert(
                "set-cookie".to_string(),
                format!(
                    "{COOKIE}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0{}",
                    if config.frontend_origin.starts_with("https://") {
                        "; Secure"
                    } else {
                        ""
                    }
                ),
            );
            response
        }
        Err(_) => unavailable(),
    }
}

fn projects(db_path: &Path, request: &ApiRequest) -> ApiResponse {
    let token = request
        .cookies
        .get(COOKIE)
        .map(String::as_str)
        .unwrap_or("");
    match global::session_valid(db_path, token) {
        Ok(true) => {}
        Ok(false) => {
            return error(
                401,
                "api-unauthorized",
                "Forge administrator session is required",
            )
        }
        Err(_) => return unavailable(),
    }
    match super::fleet::load(db_path) {
        Ok(envelope) => ApiResponse::json(200, envelope),
        Err(_) => unavailable(),
    }
}

/// `GET /v1/admin/commands`: the typed CLI command catalog. Same global
/// session gate as [`projects`]; the body is static metadata only — the
/// catalog describes commands and never executes anything, and the API
/// exposes no shell/eval route anywhere.
fn commands(db_path: &Path, request: &ApiRequest) -> ApiResponse {
    let token = request
        .cookies
        .get(COOKIE)
        .map(String::as_str)
        .unwrap_or("");
    match global::session_valid(db_path, token) {
        Ok(true) => {}
        Ok(false) => {
            return error(
                401,
                "api-unauthorized",
                "Forge administrator session is required",
            )
        }
        Err(_) => return unavailable(),
    }
    ApiResponse::json(200, super::command_catalog::envelope())
}

fn allowed_origin(config: &ApiConfig, origin: Option<&str>) -> bool {
    origin == Some(config.frontend_origin.as_str())
}

fn cors(config: &ApiConfig, request: &ApiRequest, mut response: ApiResponse) -> ApiResponse {
    if allowed_origin(config, request.header("origin")) {
        response.headers.extend(cors_headers(config, request));
    }
    response
}

fn cors_headers(
    config: &ApiConfig,
    _request: &ApiRequest,
) -> std::collections::BTreeMap<String, String> {
    std::collections::BTreeMap::from([
        (
            "access-control-allow-origin".to_string(),
            config.frontend_origin.clone(),
        ),
        (
            "access-control-allow-credentials".to_string(),
            "true".to_string(),
        ),
        (
            "access-control-allow-methods".to_string(),
            "GET, POST, DELETE, OPTIONS".to_string(),
        ),
        (
            "access-control-allow-headers".to_string(),
            "Content-Type, Idempotency-Key".to_string(),
        ),
        ("vary".to_string(), "Origin".to_string()),
    ])
}

fn error(status: u16, code: &str, message: &str) -> ApiResponse {
    ApiResponse::json(
        status,
        json!({ "error": { "code": code, "message": message }, "contract": API_CONTRACT_VERSION }),
    )
}

fn unavailable() -> ApiResponse {
    error(
        503,
        "admin-api-unavailable",
        "Forge administrator service is unavailable",
    )
}
