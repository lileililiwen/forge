//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use super::super::{ApiConfig, ApiRequest, ApiResponse, API_CONTRACT_VERSION};
use crate::delivery::handlers as delivery_handlers;
use crate::identity::global;
use crate::registry::Registry;
use chrono::Utc;
use serde_json::{json, Value};
use std::path::Path;

use super::deploy::authoring_digest;
use super::model::{DeliveryAction, DeliveryConfirmation, DeliveryResolution, LoginBody};
use super::publish::{delivery_confirmation, delivery_report, delivery_resolution};
use super::routes::COOKIE;

/// The path-free canonical descriptor a preview digest binds. Hermora URL and
/// secret reference participate in the digest but are intentionally omitted
/// from the visible preview; the digest is opaque and bound to them.
fn delivery_descriptor(
    action: DeliveryAction,
    id: &str,
    revision: &str,
    confirmation: &DeliveryConfirmation,
) -> Value {
    let mut descriptor = json!({
        "action": action.name(),
        "project_id": id,
        "revision": revision,
    });
    match confirmation {
        DeliveryConfirmation::None => {}
        DeliveryConfirmation::OperationId(operation_id) => {
            descriptor["confirm_operation_id"] = json!(operation_id);
        }
        DeliveryConfirmation::Revision(confirm_revision) => {
            descriptor["confirm_revision"] = json!(confirm_revision);
        }
        DeliveryConfirmation::Hermora {
            deployment_url,
            secret_ref,
        } => {
            descriptor["deployment_url"] = json!(deployment_url);
            descriptor["secret_ref"] = json!(secret_ref);
        }
    }
    descriptor
}

/// Flat preview fields the generic workbench renderer can print without
/// bespoke templates. Sensitive Hermora inputs are bound into the digest but
/// never echoed here.
fn delivery_preview(
    action: DeliveryAction,
    id: &str,
    resolution: &DeliveryResolution,
    confirmation: &DeliveryConfirmation,
    report: &Value,
) -> Value {
    let mut preview = json!({
        "action": action.name(),
        "project_id": id,
        "revision": resolution.revision,
        "phase": report.pointer("/phase").cloned().unwrap_or(Value::Null),
        "next": delivery_next(report, &resolution.revision),
    });
    match confirmation {
        DeliveryConfirmation::None => {}
        DeliveryConfirmation::OperationId(operation_id) => {
            preview["confirm_operation_id"] = json!(operation_id);
        }
        DeliveryConfirmation::Revision(confirm_revision) => {
            preview["confirm_revision"] = json!(confirm_revision);
        }
        DeliveryConfirmation::Hermora { .. } => {
            preview["deployment_url"] = json!("bound into the digest, not displayed");
            preview["secret_ref"] = json!("bound into the digest, not displayed");
        }
    }
    preview
}

/// Plain-language next staged confirmation derived from the scrubbed report.
/// Failed or stale prerequisites report `blocked` with recovery guidance
/// rather than suggesting the same mutation blindly.
fn delivery_next(report: &Value, revision: &str) -> Value {
    let verb = |name: &str| {
        report
            .pointer(&format!("/{name}"))
            .cloned()
            .unwrap_or(Value::Null)
    };
    let state = |value: &Value| {
        value
            .get("state")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string()
    };
    let report_revision = |value: &Value| {
        value
            .get("revision")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string()
    };
    let operation_id = |value: &Value| value.get("op_id").cloned().unwrap_or(Value::Null);

    let preflight = verb("preflight");
    let stage = verb("stage");
    let promote = verb("promote");
    let hermora = verb("hermora");
    let healthy = |value: &Value| state(value) == "done" && report_revision(value) == revision;

    if !healthy(&preflight) {
        return json!({
            "action": DeliveryAction::Preflight.name(),
            "requires": [],
            "blocked": state(&preflight) == "failed",
            "guidance": "Run delivery preflight for the registered revision. A failed preflight must be resolved before stage.",
        });
    }
    if !healthy(&stage) {
        return json!({
            "action": DeliveryAction::Stage.name(),
            "requires": ["confirm_operation_id"],
            "confirm_operation_id": operation_id(&preflight),
            "blocked": state(&stage) == "failed",
            "guidance": "Stage with the healthy preflight operation id. A failed stage blocks promotion.",
        });
    }
    if !healthy(&promote) {
        return json!({
            "action": DeliveryAction::Promote.name(),
            "requires": ["confirm_revision"],
            "confirm_revision": revision,
            "blocked": state(&promote) == "failed",
            "guidance": "Promote with the registered revision after a healthy stage. A failed promotion blocks Hermora enrollment.",
        });
    }
    json!({
        "action": DeliveryAction::HermoraRetry.name(),
        "requires": ["deployment_url", "secret_ref"],
        "blocked": state(&hermora) == "failed",
        "guidance": "Enroll the healthy deployment with Hermora using an HTTP(S) URL and environment-variable secret reference. This never republishes.",
    })
}

/// `GET /v1/admin/projects/{id}/delivery/status`. Session-gated read-only
/// delivery status; invokes no provider or adapter and writes nothing.
pub(super) fn delivery_status(
    config: &ApiConfig,
    db_path: &Path,
    request: &ApiRequest,
    id: &str,
) -> ApiResponse {
    cors(
        config,
        request,
        guarded(db_path, request, |_| {
            let resolution = match delivery_resolution(db_path, id) {
                Ok(resolution) => resolution,
                Err(response) => return response,
            };
            match delivery_report(db_path, id, &resolution) {
                Ok(report) => ApiResponse::json(
                    200,
                    json!({
                        "delivery_status": report,
                        "next": delivery_next(&report, &resolution.revision),
                        "contract": API_CONTRACT_VERSION,
                    }),
                ),
                Err(response) => response,
            }
        }),
    )
}

/// One JSON-only, session-gated staged delivery mutation. The request is
/// refused as non-JSON before the session gate; the managed-project,
/// typed-confirmation and digest checks precede any Core call. Confirmed
/// matching requests delegate to the unchanged delivery handler for the
/// verb. Core's typed errors preserve status and code; responses are
/// scrubbed of the project path and exact Hermora inputs.
pub(super) fn project_delivery_write(
    config: &ApiConfig,
    db_path: &Path,
    request: &ApiRequest,
    id: &str,
    action: DeliveryAction,
) -> ApiResponse {
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
        guarded(db_path, request, |req| {
            let resolution = match delivery_resolution(db_path, id) {
                Ok(resolution) => resolution,
                Err(response) => return response,
            };
            let body = req.json_body();
            let confirmation = match delivery_confirmation(action, &body) {
                Ok(confirmation) => confirmation,
                Err(response) => return response,
            };
            let digest = authoring_digest(&delivery_descriptor(
                action,
                id,
                &resolution.revision,
                &confirmation,
            ));
            let report = match delivery_report(db_path, id, &resolution) {
                Ok(report) => report,
                Err(response) => return response,
            };
            let preview = delivery_preview(action, id, &resolution, &confirmation, &report);
            let confirm = body
                .get("confirm")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            if !confirm {
                return ApiResponse::json(
                    200,
                    json!({
                        "preview": preview,
                        "plan_digest": digest,
                        "confirmation": {
                            "requires": ["confirm", "plan_digest"],
                            "note": "This preview dispatches nothing. To run this staged delivery verb, send `confirm: true` with this exact `plan_digest`; a changed or stale digest is refused.",
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
                            "message": "the confirmed digest does not match this delivery preview; nothing was dispatched. Review the refreshed preview and confirm its new digest.",
                        },
                        "preview": preview,
                        "plan_digest": digest,
                        "contract": API_CONTRACT_VERSION,
                    }),
                );
            }

            let mut secrets = vec![resolution.project_dir.display().to_string()];
            if let DeliveryConfirmation::Hermora {
                deployment_url,
                secret_ref,
            } = &confirmation
            {
                secrets.push(deployment_url.clone());
                secrets.push(secret_ref.clone());
            }
            let registry = match Registry::open(db_path) {
                Ok(registry) => registry,
                Err(_) => return unavailable(),
            };
            let now = Utc::now();
            let outcome = match (action, &confirmation) {
                (DeliveryAction::Preflight, DeliveryConfirmation::None) => {
                    delivery_handlers::run_preflight(&registry, id, now)
                }
                (DeliveryAction::Stage, DeliveryConfirmation::OperationId(operation_id)) => {
                    delivery_handlers::run_stage(&registry, id, *operation_id, now)
                }
                (DeliveryAction::Promote, DeliveryConfirmation::Revision(confirm_revision)) => {
                    delivery_handlers::run_promote(&registry, id, confirm_revision, now)
                }
                (
                    DeliveryAction::HermoraRetry,
                    DeliveryConfirmation::Hermora {
                        deployment_url,
                        secret_ref,
                    },
                ) => delivery_handlers::run_hermora_retry(
                    &registry,
                    id,
                    deployment_url,
                    secret_ref,
                    now,
                ),
                _ => {
                    return error(
                        400,
                        "delivery-invalid",
                        "the staged delivery confirmation does not match this route.",
                    )
                }
            };
            match outcome {
                Ok(outcome) => {
                    let report = match serde_json::to_value(&outcome.report) {
                        Ok(report) => scrub_json(report, &secrets),
                        Err(_) => {
                            return error(
                                500,
                                "admin-delivery-unavailable",
                                "delivery completed but its report could not be encoded; check the journal directly.",
                            )
                        }
                    };
                    ApiResponse::json(
                        202,
                        json!({
                            "project_id": id,
                            "operation_id": outcome.op_id,
                            "action": action.name(),
                            "delivery": report,
                            "contract": API_CONTRACT_VERSION,
                        }),
                    )
                }
                Err(err) => typed_delivery_error(&resolution.project_dir, &secrets, &err),
            }
        }),
    )
}

/// Map a delivery Core error to its typed API status/code while scrubbing
/// the project path and any exact caller-supplied Hermora inputs. The typed
/// code and status are preserved so boundary scenarios stay honest.
pub(super) fn typed_delivery_error(
    project_dir: &Path,
    secrets: &[String],
    err: &crate::core::ForgeError,
) -> ApiResponse {
    let mut all = vec![project_dir.display().to_string()];
    all.extend(secrets.iter().cloned());
    scrub_response(ApiResponse::from_error(err), &all)
}

/// Replace the raw body of an already-built error response with a scrubbed copy,
/// preserving status and shape. Used on the confirmed-apply failure path where
/// the engine's message can name the adapter binary.
pub(in crate::api) fn scrub_response(response: ApiResponse, secrets: &[String]) -> ApiResponse {
    let status = response.status;
    let parsed: Value = serde_json::from_slice(&response.body).unwrap_or(Value::Null);
    ApiResponse::json(status, scrub_json(parsed, secrets))
}

/// Recursively scrub every string in a JSON value: replace each known absolute
/// secret verbatim, then redact any remaining absolute-path token.
pub(in crate::api) fn scrub_json(value: Value, secrets: &[String]) -> Value {
    match value {
        Value::String(text) => Value::String(scrub_text(&text, secrets)),
        Value::Array(items) => {
            Value::Array(items.into_iter().map(|v| scrub_json(v, secrets)).collect())
        }
        Value::Object(map) => {
            let mut out = serde_json::Map::new();
            for (key, item) in map {
                out.insert(key, scrub_json(item, secrets));
            }
            Value::Object(out)
        }
        other => other,
    }
}

/// Replace each secret substring (the project directory, the adapter binary)
/// with a fixed marker, then apply the whitespace-token path redactor so no bare
/// absolute path survives either.
pub(in crate::api) fn scrub_text(text: &str, secrets: &[String]) -> String {
    let mut out = text.to_string();
    for secret in secrets {
        if !secret.is_empty() {
            out = out.replace(secret.as_str(), "[local path]");
        }
    }
    redact_local_paths(&out)
}

/// Replace whitespace-separated tokens that look like absolute local paths
/// (`/home/…`, `C:\…`, `key=/value`) with a fixed marker. API route strings
/// (always under `/v1/…`) are left intact: they are self-authored endpoints,
/// never filesystem locations. Mirrors the delivery projection's discipline so a
/// deploy response can never carry a real filesystem path.
fn redact_local_paths(text: &str) -> String {
    text.split_whitespace()
        .map(|token| {
            let is_route = token == "/v1" || token.starts_with("/v1/");
            let is_abs = !is_route
                && (token.starts_with('/')
                    || (token.len() > 2
                        && token.as_bytes()[1] == b':'
                        && (token.as_bytes()[2] == b'\\' || token.as_bytes()[2] == b'/'))
                    || token.contains("=/")
                    || token.contains(":\\"));
            if is_abs {
                "[local path]"
            } else {
                token
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// One JSON-only, session-gated delivery mutation: a non-JSON content type
/// is refused before the session gate ever runs the handler, and the
/// handler itself enforces the confirm- and digest-binding. Delivery
/// actions are typed in-process Core calls — never a shell, never a
/// browser-supplied path.
pub(super) fn delivery_write<F>(
    config: &ApiConfig,
    db_path: &Path,
    request: &ApiRequest,
    f: F,
) -> ApiResponse
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
pub(in crate::api) fn guarded<F>(db_path: &Path, request: &ApiRequest, f: F) -> ApiResponse
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
pub(in crate::api) fn is_json(request: &ApiRequest) -> bool {
    request
        .header("content-type")
        .is_some_and(|value| value.to_ascii_lowercase().starts_with("application/json"))
}

/// Extract the optional `feature` query parameter (the one catalog feature id
/// to upgrade). It is percent-decoded like every other browser query value and
/// used only as a catalog feature key by Core — never as a path or argument.
pub(super) fn parse_feature_query(query: &str) -> Option<String> {
    for pair in query.split('&').filter(|part| !part.is_empty()) {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        if key == "feature" && !value.is_empty() {
            return Some(super::super::percent_decode(value));
        }
    }
    None
}

pub(in crate::api) fn handle_preflight(config: &ApiConfig, request: &ApiRequest) -> ApiResponse {
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

pub(super) fn session_state(db_path: &Path, request: &ApiRequest) -> ApiResponse {
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

pub(super) fn sign_in(config: &ApiConfig, db_path: &Path, request: &ApiRequest) -> ApiResponse {
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

pub(super) fn sign_out(config: &ApiConfig, db_path: &Path, request: &ApiRequest) -> ApiResponse {
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

pub(super) fn projects(db_path: &Path, request: &ApiRequest) -> ApiResponse {
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
    match super::super::fleet::load(db_path) {
        Ok(envelope) => ApiResponse::json(200, envelope),
        Err(_) => unavailable(),
    }
}

/// `GET /v1/admin/commands`: the typed CLI command catalog. Same global
/// session gate as [`projects`]; the body is static metadata only — the
/// catalog describes commands and never executes anything, and the API
/// exposes no shell/eval route anywhere.
pub(super) fn commands(db_path: &Path, request: &ApiRequest) -> ApiResponse {
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
    ApiResponse::json(200, super::super::command_catalog::envelope())
}

pub(in crate::api) fn allowed_origin(config: &ApiConfig, origin: Option<&str>) -> bool {
    origin == Some(config.frontend_origin.as_str())
}

pub(in crate::api) fn cors(
    config: &ApiConfig,
    request: &ApiRequest,
    mut response: ApiResponse,
) -> ApiResponse {
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

pub(in crate::api) fn error(status: u16, code: &str, message: &str) -> ApiResponse {
    ApiResponse::json(
        status,
        json!({ "error": { "code": code, "message": message }, "contract": API_CONTRACT_VERSION }),
    )
}

pub(super) fn unavailable() -> ApiResponse {
    error(
        503,
        "admin-api-unavailable",
        "Forge administrator service is unavailable",
    )
}
