//! Forge-administrator password rotation
//! (`forge-portal-identity-rotation`).
//!
//! One session-gated POST that mirrors `forge identity change-password`
//! into the browser: `POST /v1/admin/identity/change-password`.
//! The handler is the only new admin route introduced for the topbar
//! `Change password` control; the route shape does not collide with
//! the existing `v1/admin/projects/{id}/identity/{config,sessions,...}`
//! reads (handled in `agent_identity.rs`) because the path under
//! `/v1/admin/identity/` is a four-segment global route matched beside
//! the table, not a five-segment project-bound read. Kept beside the
//! handlers (not in `router.rs`) so the route table stays under the
//! source-file-size cap — the `agent_identity.rs` and `history.rs`
//! precedents exactly.
//!
//! All Core work happens inside `forge::identity::global::change_password`
//! (Argon2id hash update + `revoke_all_sessions_in` cascade). The
//! handler is a thin envelope: it re-validates the request shape, maps
//! the typed Core `String` errors to the wire envelope, and never
//! echoes the password fields. The `current` field is verified against
//! the registered administrator by re-using
//! `forge::identity::global::authenticate`, so a borrowed session
//! cannot rotate the password without re-authenticating.

use super::super::Route;
use super::super::{ApiConfig, ApiRequest, ApiResponse, API_CONTRACT_VERSION};
use serde_json::{json, Value};
use std::path::Path;

use super::gateway::{cors, error, guarded, is_json};

/// Match `POST /v1/admin/identity/change-password` before the project
/// table arms. The literal `identity` segment never collides with the
/// project-bound `["v1","admin","projects",id,"identity",...]` reads
/// (different parent); trying this shape first shadows no existing arm.
pub(in crate::api) fn route_identity(method: &str, segments: &[&str]) -> Option<Route> {
    match (method, segments) {
        ("POST", ["v1", "admin", "identity", "change-password"]) => {
            Some(Route::AdminIdentityChangePassword)
        }
        _ => None,
    }
}

/// JSON request body for the change-password route.
struct ChangePasswordBody {
    current: String,
    password: String,
    confirm: String,
}

fn parse_change_password_body(body: &[u8]) -> Result<ChangePasswordBody, ApiResponse> {
    let parsed: Value = match serde_json::from_slice(body) {
        Ok(value) => value,
        Err(_) => {
            return Err(error(
                400,
                "admin-invalid-request",
                "current, password and confirm are required",
            ))
        }
    };
    let obj = match parsed.as_object() {
        Some(obj) => obj,
        None => {
            return Err(error(
                400,
                "admin-invalid-request",
                "current, password and confirm are required",
            ))
        }
    };
    let current = match obj.get("current").and_then(Value::as_str) {
        Some(value) if !value.is_empty() => value.to_string(),
        _ => {
            return Err(error(
                400,
                "admin-invalid-request",
                "current, password and confirm are required",
            ))
        }
    };
    let password = match obj.get("password").and_then(Value::as_str) {
        Some(value) if !value.is_empty() => value.to_string(),
        _ => {
            return Err(error(
                400,
                "admin-invalid-request",
                "current, password and confirm are required",
            ))
        }
    };
    let confirm = match obj.get("confirm").and_then(Value::as_str) {
        Some(value) if !value.is_empty() => value.to_string(),
        _ => {
            return Err(error(
                400,
                "admin-invalid-request",
                "current, password and confirm are required",
            ))
        }
    };
    Ok(ChangePasswordBody {
        current,
        password,
        confirm,
    })
}

/// Map the typed Core `String` error into the wire envelope. Core
/// returns one of two strings from `change_password` (the storage
/// contract in `src/identity/global.rs:172-204`); the wire codes are
/// stable for the browser and the CLI to agree on, while the Core
/// string stays the source of truth.
fn map_change_password_error(err: &str) -> ApiResponse {
    let trimmed = err.trim();
    if trimmed.starts_with("password must contain 12 to 1024 characters") {
        return error(
            400,
            "admin-password-too-weak",
            "password must contain 12 to 1024 characters",
        );
    }
    if trimmed.starts_with("no Forge administrator is configured") {
        return error(
            403,
            "admin-identity-not-configured",
            "no Forge administrator is configured; run `forge identity setup` first",
        );
    }
    error(500, "admin-password-rotation-failed", trimmed)
}

/// `POST /v1/admin/identity/change-password`. Wraps the typed
/// envelope in the same session + CORS gate every other admin
/// route uses (`guarded` + `cors` from `gateway.rs`): an
/// anonymous request is refused as `401 api-unauthorized` before
/// any Core call, and the response is wrapped in the
/// exact-origin CORS envelope. Inside the gate the handler
/// verifies the `current` password against the registered
/// administrator by re-using
/// `forge::identity::global::authenticate`; on a hit, the probe's
/// minted session is revoked before the rotation runs so the only
/// surviving session is the one Core mints during the rotation
/// (which is then revoked by the rotation itself). On a miss the
/// handler answers `403 admin-current-password-incorrect` with
/// no leak of which side was wrong. Then the handler delegates
/// the Argon2id hash update + session cascade to
/// `forge::identity::global::change_password`. Every response
/// carries the `forge-admin-login/1.0.0` contract id the CLI emits
/// (`src/cli/identity.rs:54`, `:76`) plus the global
/// `API_CONTRACT_VERSION`.
pub(in crate::api) fn change_password(
    config: &ApiConfig,
    db_path: &Path,
    request: &ApiRequest,
) -> ApiResponse {
    cors(
        config,
        request,
        guarded(db_path, request, |_| {
            change_password_inner(config, db_path, request)
        }),
    )
}

fn change_password_inner(_config: &ApiConfig, db_path: &Path, request: &ApiRequest) -> ApiResponse {
    if request.body.len() > 8192 {
        return error(
            413,
            "admin-request-too-large",
            "change-password request is too large",
        );
    }
    if !is_json(request) {
        return error(
            415,
            "admin-content-type-required",
            "change-password requires application/json",
        );
    }
    let body = match parse_change_password_body(&request.body) {
        Ok(value) => value,
        Err(response) => return response,
    };
    if body.password != body.confirm {
        return error(
            400,
            "admin-password-confirmation-mismatch",
            "new password and confirmation do not match",
        );
    }
    let configured = match crate::identity::global::is_configured(db_path) {
        Ok(value) => value,
        Err(_) => {
            return error(
                500,
                "admin-password-rotation-failed",
                "registry is unavailable",
            )
        }
    };
    if !configured {
        return error(
            403,
            "admin-identity-not-configured",
            "no Forge administrator is configured; run `forge identity setup` first",
        );
    }
    let email = match crate::identity::global::email(db_path) {
        Ok(Some(value)) => value,
        _ => {
            return error(
                500,
                "admin-password-rotation-failed",
                "registered administrator email is unreadable",
            )
        }
    };
    // Verify the supplied `current` password by re-running the
    // sign-in credential check against the registered email. The
    // probe mints a session when the password is right; revoke it
    // immediately so the only session left is the one the rotation
    // cascade invalidates next.
    // Verify the supplied `current` password by re-running the
    // sign-in credential check against the registered email. The
    // probe mints a session when the password is right; capture its
    // token so the only session we revoke on a failed rotation is the
    // probe itself — the caller's session must survive a rejected
    // (e.g. too-weak) new password. `change_password` revokes all
    // sessions on success.
    match crate::identity::global::authenticate(db_path, &email, &body.current) {
        Ok(Some(token)) => {
            if crate::identity::global::revoke(db_path, &token).is_err() {
                return error(
                    500,
                    "admin-password-rotation-failed",
                    "could not revoke probe session",
                );
            }
        }
        Ok(None) => {
            return error(
                403,
                "admin-current-password-incorrect",
                "current password is incorrect",
            );
        }
        Err(_) => {
            return error(
                500,
                "admin-password-rotation-failed",
                "current password probe failed",
            )
        }
    }
    // Delegate the Argon2id hash update + the
    // `revoke_all_sessions_in` cascade to the same Core function
    // the CLI calls. The Core function's typed `String` errors
    // are mapped to the wire envelope above.
    if let Err(reason) = crate::identity::global::change_password(db_path, &body.password) {
        return map_change_password_error(&reason);
    }
    ApiResponse::json(
        200,
        json!({
            "changed": true,
            "contract": "forge-admin-login/1.0.0",
            "version": API_CONTRACT_VERSION,
        }),
    )
}
