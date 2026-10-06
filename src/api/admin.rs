//! JSON-only Forge-wide administrator endpoints used by `frontend/`.

use std::path::Path;

use serde::Deserialize;
use serde_json::json;

use super::{ApiConfig, ApiRequest, ApiResponse, Route, API_CONTRACT_VERSION};
use crate::identity::global;

const COOKIE: &str = "forge_admin_session";

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
        _ => error(404, "route-not-found", "no admin route matches the request"),
    };
    cors(config, request, result)
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
    let fleet = match super::ui::data::load_fleet_list(db_path) {
        Ok(value) => value,
        Err(_) => return unavailable(),
    };
    let projects: Vec<_> = fleet.rows.iter().map(|row| json!({
        "id": row.id,
        "profile": row.profile,
        "state": row.last_state,
        "updated_at": row.last_at,
        "lifecycle": row.lifecycle.map(|value| format!("{value:?}").to_ascii_lowercase()),
        "confidence": row.confidence.map(|value| format!("{value:?}").to_ascii_lowercase()),
        "tags": row.tags,
        "evidence": row.evidence.iter().map(|(source, status)| json!({"source": source, "status": status.label()})).collect::<Vec<_>>(),
    })).collect();
    let with_evidence = fleet
        .rows
        .iter()
        .filter(|row| !row.evidence.is_empty())
        .count();
    ApiResponse::json(
        200,
        json!({
            "projects": projects,
            "summary": { "registered": projects.len(), "with_evidence": with_evidence },
            "contract": API_CONTRACT_VERSION,
        }),
    )
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
            "Content-Type".to_string(),
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
