//! HTTP handlers for the in-process portal UI.
//!
//! Routes registered on the existing `forge api serve` listener:
//!
//! - `GET /ui` — fleet list, optionally filtered by portfolio
//!   metadata (`?tag=&lifecycle=&confidence=`)
//! - `GET /ui/projects/{id}` — project detail with the portfolio
//!   projection
//! - `POST /ui/projects/{id}/publish` — confirm-gated republish
//! - `POST /ui/projects/{id}/portfolio` — bearer- and
//!   origin-checked user-owned metadata write
//!
//! Content negotiation: `Accept: text/html` serves maud-
//! rendered HTML; otherwise the handler returns a JSON error
//! so the JSON clients route through the existing API handlers.

use std::collections::BTreeMap;
use std::path::Path;

use crate::api::{ApiConfig, ApiRequest, ApiResponse, API_CONTRACT_VERSION};
use crate::core::{validate_project_id, ForgeError};
use crate::portfolio::parse_filter;

use super::auth::{check_origin, recheck_post_token, AuthDecision};
use super::data::{self, PortfolioFormInput};
use super::render::{
    error_page, fleet_list, operation_accepted, portfolio_saved, project_detail, publish_plan,
    ErrorKind,
};

const FLEET_TITLE: &str = "Forge fleet";
const PROJECT_TITLE: &str = "Forge project";
const PLAN_TITLE: &str = "Forge publish plan";
const ENQUEUED_TITLE: &str = "Forge publish enqueued";
const PORTFOLIO_TITLE: &str = "Forge portfolio";

// --- response builders -------------------------------------------------

fn html_response(status: u16, body: String) -> ApiResponse {
    let mut headers = BTreeMap::new();
    headers.insert(
        "content-type".to_string(),
        "text/html; charset=utf-8".to_string(),
    );
    headers.insert(
        "x-forge-contract".to_string(),
        API_CONTRACT_VERSION.to_string(),
    );
    ApiResponse {
        status,
        headers,
        body: body.into_bytes(),
    }
}

fn render_error(status: u16, code: &'static str, message: &str, kind: ErrorKind) -> ApiResponse {
    html_response(
        status,
        error_page("Forge ui error", code, message, kind, API_CONTRACT_VERSION),
    )
}

fn forge_error_to_html(err: ForgeError) -> ApiResponse {
    let (status, kind, message) = match err.code() {
        "api-unauthorized" => (401, ErrorKind::Auth, "missing or invalid bearer token"),
        "unknown-project" => (404, ErrorKind::Project, "no such project"),
        "publish-confirm-required" => (409, ErrorKind::Conflict, "publish requires confirm=yes"),
        "portfolio-invalid" => (400, ErrorKind::Project, "portfolio metadata refused"),
        _ => (500, ErrorKind::Project, "internal error"),
    };
    render_error(status, err.code(), message, kind)
}

fn json_only_response() -> ApiResponse {
    ApiResponse::json(
        400,
        serde_json::json!({
            "error": {
                "code": "api-routing",
                "message": "this path serves the browser UI; pass Accept: text/html for HTML, otherwise use the v1 API surface"
            },
            "contract": API_CONTRACT_VERSION,
        }),
    )
}

// --- request helpers ---------------------------------------------------

fn wants_html(request: &ApiRequest) -> bool {
    match request.header("accept") {
        Some(value) => {
            let lower = value.to_ascii_lowercase();
            lower.contains("text/html") || lower.starts_with("*/*")
        }
        None => true,
    }
}

fn extract_token(request: &ApiRequest) -> Option<String> {
    if let Some(token) = request.bearer_token.as_deref() {
        let trimmed = token.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }
    if let Some(query) = request.query.as_deref() {
        for pair in query.split('&') {
            if let Some(rest) = pair.strip_prefix("token=") {
                let trimmed = rest.trim();
                if !trimmed.is_empty() {
                    return Some(percent_decode(trimmed));
                }
            }
        }
    }
    None
}

fn percent_decode(raw: &str) -> String {
    let bytes = raw.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' if i + 2 < bytes.len() => {
                let hi = hex_digit(bytes[i + 1]);
                let lo = hex_digit(bytes[i + 2]);
                match (hi, lo) {
                    (Some(h), Some(l)) => {
                        out.push((h << 4) | l);
                        i += 3;
                    }
                    _ => {
                        out.push(bytes[i]);
                        i += 1;
                    }
                }
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex_digit(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

fn parse_form(body: &[u8]) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    if body.is_empty() {
        return out;
    }
    let text = match std::str::from_utf8(body) {
        Ok(value) => value,
        Err(_) => return out,
    };
    for pair in text.split('&') {
        if pair.is_empty() {
            continue;
        }
        let (raw_key, raw_val) = match pair.split_once('=') {
            Some((k, v)) => (k, v),
            None => (pair, ""),
        };
        out.insert(percent_decode(raw_key), percent_decode(raw_val));
    }
    out
}

fn require_token(token: Option<&str>) -> Option<ApiResponse> {
    if token.is_none() {
        Some(render_error(
            401,
            "api-unauthorized",
            "missing bearer token; supply Authorization: Bearer <session-id> or ?token=<id>",
            ErrorKind::Auth,
        ))
    } else {
        None
    }
}

// --- handlers ----------------------------------------------------------

/// `GET /ui`
pub fn handle_fleet(db_path: &Path, _config: &ApiConfig, request: &ApiRequest) -> ApiResponse {
    if !wants_html(request) {
        return json_only_response();
    }
    let token = match extract_token(request) {
        Some(t) => t,
        None => return require_token(None).unwrap(),
    };
    if let Some(resp) = require_token(Some(&token)) {
        return resp;
    }
    // A malformed filter is a typed refusal, not a silently
    // widened result set: an operator who typed `lifecycle=shipped`
    // must learn the vocabulary rather than see every project.
    let filter = match parse_filter(request.query.as_deref().unwrap_or("")) {
        Ok(filter) => filter,
        Err(reason) => {
            return render_error(
                400,
                "portfolio-invalid",
                &format!("invalid portfolio filter: {reason}"),
                ErrorKind::Project,
            )
        }
    };
    match data::load_fleet_list_filtered(db_path, &filter) {
        Ok(view) => html_response(
            200,
            fleet_list(
                FLEET_TITLE,
                &view.rows,
                &[],
                &view.filter,
                API_CONTRACT_VERSION,
            ),
        ),
        Err(err) => forge_error_to_html(err),
    }
}

/// `GET /ui/projects/{id}`
pub fn handle_project_detail(
    db_path: &Path,
    _config: &ApiConfig,
    request: &ApiRequest,
    project_id: &str,
) -> ApiResponse {
    if !wants_html(request) {
        return json_only_response();
    }
    if let Err(err) = validate_project_id(project_id) {
        return render_error(
            400,
            "manifest-invalid",
            &err.to_string(),
            ErrorKind::Project,
        );
    }
    let token = extract_token(request);
    if let Some(resp) = require_token(token.as_deref()) {
        return resp;
    }
    // The detail page is project-scoped: the portfolio filter
    // belongs to the fleet list, so a query string here is
    // ignored rather than silently narrowing the view.
    match data::load_project_detail(db_path, project_id) {
        Ok(view) => {
            let args = super::render::ProjectDetailArgs {
                title: PROJECT_TITLE,
                identity: &view.identity,
                doctor: &view.doctor,
                inventory_subdomain: &view.inventory_subdomain,
                journal: &view.journal,
                portfolio: &view.portfolio,
                token: &token.unwrap_or_default(),
                origin: request.header("origin").unwrap_or(""),
                contract: API_CONTRACT_VERSION,
            };
            html_response(200, project_detail(args))
        }
        Err(ForgeError::UnknownProject { .. }) => render_error(
            404,
            "unknown-project",
            &format!("project `{project_id}` is not in the registry"),
            ErrorKind::Project,
        ),
        Err(err) => forge_error_to_html(err),
    }
}

/// `POST /ui/projects/{id}/publish`
pub fn handle_project_publish(
    db_path: &Path,
    config: &ApiConfig,
    request: &ApiRequest,
    project_id: &str,
) -> ApiResponse {
    if !wants_html(request) {
        return json_only_response();
    }
    if let Err(err) = validate_project_id(project_id) {
        return render_error(
            400,
            "manifest-invalid",
            &err.to_string(),
            ErrorKind::Project,
        );
    }

    // 1. Bearer token
    let token = match extract_token(request) {
        Some(t) => t,
        None => return require_token(None).unwrap(),
    };
    if let Some(resp) = require_token(Some(&token)) {
        return resp;
    }

    // 2. Cross-origin check
    if let AuthDecision::Refuse { code, message } = check_origin(request.header("origin"), config) {
        return render_error(403, code, message, ErrorKind::Project);
    }

    // 3. Form
    let form = parse_form(&request.body);
    let confirm = form.get("confirm").map(String::as_str) == Some("yes");

    // 4. Re-check token
    if let AuthDecision::Refuse { code, message } =
        recheck_post_token(form.get("token").map(String::as_str), &token)
    {
        return render_error(403, code, message, ErrorKind::Project);
    }

    if !confirm {
        return render_plan_preview(db_path, project_id, &token, request);
    }
    enqueue_confirmed_publish(db_path, project_id)
}

fn render_plan_preview(
    db_path: &Path,
    project_id: &str,
    token: &str,
    request: &ApiRequest,
) -> ApiResponse {
    let origin = request.header("origin").unwrap_or("");
    let steps = match data::load_publish_plan_steps(db_path, project_id) {
        Ok(steps) => steps,
        Err(err) => return forge_error_to_html(err),
    };
    html_response(
        200,
        publish_plan(
            PLAN_TITLE,
            project_id,
            &steps,
            None,
            token,
            origin,
            API_CONTRACT_VERSION,
        ),
    )
}

fn enqueue_confirmed_publish(db_path: &Path, project_id: &str) -> ApiResponse {
    let identity = match data::record_ui_publish(db_path, project_id) {
        Ok(value) => value,
        Err(err) => return forge_error_to_html(err),
    };
    let mut response = html_response(
        202,
        operation_accepted(ENQUEUED_TITLE, project_id, &identity, API_CONTRACT_VERSION),
    );
    response
        .headers
        .insert("location".to_string(), format!("/ui/projects/{project_id}"));
    response
}

/// `POST /ui/projects/{id}/portfolio` — user-owned metadata write.
///
/// The same three checks the republish POST runs apply here:
/// a bearer token must be present, the `Origin` header must match
/// the loopback bind, and the hidden `token` field must match the
/// bearer token. Only then is any field validated or written, so an
/// unauthorized or cross-origin submission persists no change.
/// Imported evidence has no form on this page: source-owned
/// snapshots are append-only and arrive through the import
/// surface.
pub fn handle_project_portfolio(
    db_path: &Path,
    config: &ApiConfig,
    request: &ApiRequest,
    project_id: &str,
) -> ApiResponse {
    if !wants_html(request) {
        return json_only_response();
    }
    if let Err(err) = validate_project_id(project_id) {
        return render_error(
            400,
            "manifest-invalid",
            &err.to_string(),
            ErrorKind::Project,
        );
    }

    // 1. Bearer token
    let token = match extract_token(request) {
        Some(t) => t,
        None => return require_token(None).unwrap(),
    };
    if let Some(resp) = require_token(Some(&token)) {
        return resp;
    }

    // 2. Cross-origin check
    if let AuthDecision::Refuse { code, message } = check_origin(request.header("origin"), config) {
        return render_error(403, code, message, ErrorKind::Project);
    }

    // 3. Form token re-check
    let form = parse_form(&request.body);
    if let AuthDecision::Refuse { code, message } =
        recheck_post_token(form.get("token").map(String::as_str), &token)
    {
        return render_error(403, code, message, ErrorKind::Project);
    }

    let input = PortfolioFormInput {
        lifecycle: form_value(&form, "lifecycle"),
        confidence: form_value(&form, "confidence"),
        next_action: form_value(&form, "next_action"),
        blocker: form_value(&form, "blocker"),
        tag: form_value(&form, "tag"),
        remove_tag: form_value(&form, "remove_tag"),
    };
    match data::apply_portfolio_form(db_path, project_id, &input) {
        Ok(edits) => {
            let mut response = html_response(
                200,
                portfolio_saved(PORTFOLIO_TITLE, project_id, &edits, API_CONTRACT_VERSION),
            );
            response
                .headers
                .insert("location".to_string(), format!("/ui/projects/{project_id}"));
            response
        }
        Err(ForgeError::UnknownProject { .. }) => render_error(
            404,
            "unknown-project",
            &format!("project `{project_id}` is not in the registry"),
            ErrorKind::Project,
        ),
        Err(err) => forge_error_to_html(err),
    }
}

/// One trimmed, non-empty form field. An empty box means "leave
/// this field alone" and is never treated as a value to store.
fn form_value(form: &BTreeMap<String, String>, key: &str) -> Option<String> {
    form.get(key)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

// --- tests -------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_form_decodes_url_encoded() {
        let body = b"token=deadbeef&confirm=yes".to_vec();
        let out = parse_form(&body);
        assert_eq!(out.get("token").map(String::as_str), Some("deadbeef"));
        assert_eq!(out.get("confirm").map(String::as_str), Some("yes"));
    }

    #[test]
    fn parse_form_handles_plus_and_percent() {
        let body = b"a=hello+world&b=x%2Fy".to_vec();
        let out = parse_form(&body);
        assert_eq!(out.get("a").map(String::as_str), Some("hello world"));
        assert_eq!(out.get("b").map(String::as_str), Some("x/y"));
    }

    #[test]
    fn wants_html_defaults_true_when_no_accept() {
        let req = ApiRequest {
            method: "GET".to_string(),
            path: "/ui".to_string(),
            query: None,
            headers: BTreeMap::new(),
            body: Vec::new(),
            idempotency_key: None,
            bearer_token: None,
            remote_addr: None,
            started_at: chrono::Utc::now(),
        };
        assert!(wants_html(&req));
    }

    #[test]
    fn wants_html_prefers_html_over_json() {
        let mut req = ApiRequest {
            method: "GET".to_string(),
            path: "/ui".to_string(),
            query: None,
            headers: BTreeMap::new(),
            body: Vec::new(),
            idempotency_key: None,
            bearer_token: None,
            remote_addr: None,
            started_at: chrono::Utc::now(),
        };
        req.headers
            .insert("accept".to_string(), "text/html".to_string());
        assert!(wants_html(&req));
        req.headers
            .insert("accept".to_string(), "application/json".to_string());
        assert!(!wants_html(&req));
    }

    #[test]
    fn extract_token_finds_bearer() {
        let mut req = ApiRequest {
            method: "GET".to_string(),
            path: "/ui".to_string(),
            query: None,
            headers: BTreeMap::new(),
            body: Vec::new(),
            idempotency_key: None,
            bearer_token: Some("deadbeef".to_string()),
            remote_addr: None,
            started_at: chrono::Utc::now(),
        };
        assert_eq!(extract_token(&req).as_deref(), Some("deadbeef"));

        req.bearer_token = None;
        req.query = Some("token=cafebabe".to_string());
        assert_eq!(extract_token(&req).as_deref(), Some("cafebabe"));

        req.query = None;
        assert!(extract_token(&req).is_none());
    }

    #[test]
    fn require_token_returns_401_when_missing() {
        let resp = require_token(None).unwrap();
        assert_eq!(resp.status, 401);
    }
}
