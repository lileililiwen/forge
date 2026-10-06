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

use chrono::{DateTime, Utc};

use crate::api::{ApiConfig, ApiRequest, ApiResponse, API_CONTRACT_VERSION};
use crate::core::{validate_project_id, ForgeError};
use crate::identity::{
    self, build_challenge, complete_browser_auth, delete_challenge_file, load_challenge,
    load_session, save_challenge, terminate_session, AuthCallback, IdentityConfig, SessionState,
};
use crate::portfolio::parse_filter;
use crate::registry::Registry;

use super::auth::{check_origin, recheck_post_token, AuthDecision};
use super::data::{self, PortfolioFormInput};
use super::render::{
    error_page, fleet_list, operation_accepted, portfolio_saved, project_detail, publish_plan,
    sign_in_page, studio_page, ErrorKind,
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

/// The cookie name carrying the opaque project session id on
/// `/ui` routes. The cookie is `HttpOnly`, host-only, scoped
/// to `/ui`, and `SameSite=Lax`; see `cookie_value_for_session`
/// for the exact attribute string. The value is the existing
/// 64-character lowercase hex session id minted by the
/// `central-admin-identity` surface and persisted at
/// `.forge/identity/<project>/sessions/<id>.json`. The
/// constant lives here so every test and every cookie
/// site can refer to one name.
pub const SESSION_COOKIE: &str = "forge_session";

/// The transient cookie name that binds the callback to
/// the issuing browser. The value is the random hex
/// stored on the issue side, so the callback handler can
/// refuse a state value that was never bound to the
/// browser that started the round trip. The constant
/// lives here for the same reason `SESSION_COOKIE` does.
pub const STATE_COOKIE: &str = "forge_oidc_state";

fn extract_token(request: &ApiRequest) -> Option<String> {
    if let Some(token) = request.bearer_token.as_deref() {
        let trimmed = token.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }
    if let Some(cookie_value) = request.cookies.get(SESSION_COOKIE) {
        let trimmed = cookie_value.trim();
        if !trimmed.is_empty() && is_hex_token(trimmed) {
            return Some(trimmed.to_string());
        }
    }
    // The legacy `?token=` query parameter is deliberately
    // not read: browser routes never authenticate from a
    // URL credential. The value is ignored, not reflected.
    None
}

fn is_hex_token(value: &str) -> bool {
    !value.is_empty() && value.chars().all(|c| c.is_ascii_hexdigit())
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
            "missing session; supply Authorization: Bearer <session-id> or sign in \
             to the portal so the forge_session cookie is set",
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
    // A cookie-presented session is never trusted by
    // presence alone: resolve it against the project's
    // persisted state and refuse a cross-project, expired,
    // revoked, or unknown cookie before rendering.
    if let Some(resp) = refuse_invalid_cookie_session(db_path, project_id, request) {
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
                delivery: &view.delivery,
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

/// `GET /ui/studio/{project_id}` — read-only Studio page that
/// surfaces the saved AppSpec, the preview state, and the most
/// recent `studio.*` journal rows. Interactive controls land in a
/// follow-up cycle (`tasks.md` §5.2); this page renders the
/// current state honestly without claiming a preview is ready.
pub fn handle_studio_project(
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
    let token = match extract_token(request) {
        Some(t) => t,
        None => return require_token(None).unwrap(),
    };
    if let Some(resp) = require_token(Some(&token)) {
        return resp;
    }
    if let AuthDecision::Refuse { code, message } = check_origin(request.header("origin"), config) {
        return render_error(403, code, message, ErrorKind::Project);
    }
    match crate::identity::load_session(db_path, project_id, &token) {
        Ok(_) => {}
        Err(err) => return forge_error_to_html(err),
    }
    let registry = match Registry::open(db_path) {
        Ok(reg) => reg,
        Err(err) => return forge_error_to_html(err),
    };
    let record = match registry.inspect(project_id) {
        Ok(record) => record,
        Err(err) => return forge_error_to_html(err),
    };
    let project_root = std::path::PathBuf::from(&record.path);
    let session = crate::studio::load_session(&project_root).ok().flatten();
    let preview_envelope = session
        .as_ref()
        .map(crate::studio::envelope_from_session)
        .unwrap_or_else(|| {
            crate::studio::PreviewEnvelope::from_session(
                project_id,
                "r0",
                &crate::studio::state::SessionPreviewState::default(),
            )
        });
    let journal_rows = registry
        .operations_for_project(project_id, 16)
        .unwrap_or_default()
        .into_iter()
        .filter(|row| row.kind.starts_with("studio."))
        .collect::<Vec<_>>();
    let body = studio_page(
        project_id,
        session.as_ref(),
        &preview_envelope,
        &journal_rows,
        API_CONTRACT_VERSION,
    );
    html_response(200, body)
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

// --- browser sign-in / callback / sign-out ----------------------------

/// `GET /ui/sign-in?project=<id>&return=<local-path>`.
///
/// Anonymous sign-in start. The project must be registered
/// and have a valid identity configuration; the route
/// builds a fresh state/nonce/PKCE challenge, persists it,
/// and 303-redirects to the configured provider's
/// authorization endpoint. The transient `forge_oidc_state`
/// cookie binds the eventual callback to the issuing
/// browser. The `return` parameter must be a relative path
/// under `/ui`; anything else falls back to `/ui`.
pub fn handle_sign_in(db_path: &Path, config: &ApiConfig, request: &ApiRequest) -> ApiResponse {
    if !wants_html(request) {
        return json_only_response();
    }
    let project_id = match parse_query_project_id(request.query.as_deref()) {
        Some(id) => id,
        None => {
            return render_error(
                400,
                "ui-sign-in-missing-project",
                "sign-in requires a `project` query parameter naming a registered project",
                ErrorKind::Auth,
            );
        }
    };
    if let Err(err) = validate_project_id(&project_id) {
        return render_error(
            400,
            "manifest-invalid",
            &err.to_string(),
            ErrorKind::Project,
        );
    }
    let registry = match Registry::open(db_path) {
        Ok(reg) => reg,
        Err(err) => return forge_error_to_html(err),
    };
    let record = match registry.inspect(&project_id) {
        Ok(record) => record,
        Err(ForgeError::UnknownProject { .. }) => {
            return render_error(
                404,
                "unknown-project",
                &format!("project `{project_id}` is not in the registry"),
                ErrorKind::Auth,
            );
        }
        Err(err) => return forge_error_to_html(err),
    };
    let project_dir = std::path::PathBuf::from(&record.path);
    let manifest = match crate::core::manifest::Manifest::load_from_dir(&project_dir, None) {
        Ok((manifest, _)) => manifest,
        Err(err) => return forge_error_to_html(err),
    };
    let cfg = match IdentityConfig::from_manifest_opt(&project_id, &manifest) {
        Ok(Some(cfg)) => cfg,
        Ok(None) => {
            return render_error(
                400,
                "identity-invalid",
                &format!(
                    "project `{project_id}` has no `identity:` block; declare one before \
                     starting browser sign-in"
                ),
                ErrorKind::Auth,
            );
        }
        Err(err) => return forge_error_to_html(err),
    };
    let now = chrono::Utc::now();
    let challenge = match build_challenge(&project_id, &cfg, now) {
        Ok(c) => c,
        Err(err) => return forge_error_to_html(err),
    };
    if let Err(err) = save_challenge(&project_dir, &project_id, &challenge) {
        return forge_error_to_html(err);
    }
    let return_path = safe_return_path(request.query.as_deref(), "/ui");
    let auth_url = match build_provider_authorization_url(&cfg, &challenge, &return_path) {
        Ok(url) => url,
        Err(err) => return forge_error_to_html(err),
    };
    let mut response = html_response(
        303,
        sign_in_page(
            "Forge sign-in",
            &project_id,
            &auth_url,
            &return_path,
            API_CONTRACT_VERSION,
        ),
    );
    response
        .headers
        .insert("location".to_string(), auth_url.clone());
    let state_cookie = format!(
        "{}={}; Path=/ui/auth/callback; HttpOnly; SameSite=Lax{}; Expires={}",
        STATE_COOKIE,
        challenge.state,
        if is_loopback_origin(config) {
            ""
        } else {
            "; Secure"
        },
        rfc1123_expires(challenge.expires_at),
    );
    response
        .headers
        .insert("set-cookie".to_string(), state_cookie);
    response
}

/// `GET /ui/auth/callback?code=…&state=…`.
///
/// Anonymous OIDC callback. The state value must match a
/// persisted challenge; the `forge_oidc_state` cookie must
/// match the same state value so a replayed state is bound
/// to the issuing browser. The challenge is consumed before
/// the verifier runs so a duplicate callback cannot race to
/// a second session. A successful round trip mints a
/// project-scoped session, sets the `forge_session` cookie,
/// and 303-redirects to the sanitised return path.
pub fn handle_auth_callback(
    db_path: &Path,
    config: &ApiConfig,
    request: &ApiRequest,
    now: DateTime<Utc>,
) -> ApiResponse {
    let response = handle_auth_callback_inner(db_path, config, request, now);
    // The transient `forge_oidc_state` cookie has done its
    // job on every outcome, success or failure. Expire it
    // so a consumed or rejected challenge can never be
    // retried with a stale browser binding.
    with_expired_state_cookie(response)
}

fn handle_auth_callback_inner(
    db_path: &Path,
    config: &ApiConfig,
    request: &ApiRequest,
    now: DateTime<Utc>,
) -> ApiResponse {
    if !wants_html(request) {
        return json_only_response();
    }
    let (state, code, error, error_description) =
        match parse_callback_query(request.query.as_deref()) {
            Ok(parts) => parts,
            Err(reason) => {
                return render_error(400, "identity-invalid", &reason, ErrorKind::Auth);
            }
        };
    let state_cookie = request.cookies.get(STATE_COOKIE).cloned();
    if state_cookie.as_deref() != Some(state.as_str()) {
        return render_error(
            401,
            "identity-invalid",
            "auth callback state does not match the transient cookie; the callback \
             is not bound to the browser that started the round trip",
            ErrorKind::Auth,
        );
    }
    let project_id = match parse_state_project_id(db_path, &state) {
        Ok(id) => id,
        Err(ForgeError::IdentityInvalid { reason }) => {
            return render_error(401, "identity-invalid", &reason, ErrorKind::Auth);
        }
        Err(err) => return forge_error_to_html(err),
    };
    if let Err(err) = validate_project_id(&project_id) {
        return render_error(
            400,
            "manifest-invalid",
            &err.to_string(),
            ErrorKind::Project,
        );
    }
    let registry = match Registry::open(db_path) {
        Ok(reg) => reg,
        Err(err) => return forge_error_to_html(err),
    };
    let record = match registry.inspect(&project_id) {
        Ok(record) => record,
        Err(err) => return forge_error_to_html(err),
    };
    let project_dir = std::path::PathBuf::from(&record.path);
    let manifest = match crate::core::manifest::Manifest::load_from_dir(&project_dir, None) {
        Ok((manifest, _)) => manifest,
        Err(err) => return forge_error_to_html(err),
    };
    let cfg = match IdentityConfig::from_manifest_opt(&project_id, &manifest) {
        Ok(Some(cfg)) => cfg,
        Ok(None) => {
            return render_error(
                400,
                "identity-invalid",
                "project has no `identity:` block; the round trip cannot be validated",
                ErrorKind::Auth,
            );
        }
        Err(err) => return forge_error_to_html(err),
    };
    let challenge = match load_challenge(&project_dir, &project_id, &state) {
        Ok(Some(c)) => c,
        Ok(None) => {
            return render_error(
                401,
                "identity-invalid",
                "no pending OIDC challenge for the supplied state; either the \
                 challenge expired or it has already been consumed",
                ErrorKind::Auth,
            );
        }
        Err(err) => return forge_error_to_html(err),
    };
    // Consume the challenge up front so a duplicate callback
    // cannot race to a second session. The session is
    // minted by `complete_browser_auth` only after a
    // successful round trip; if the verifier fails the
    // challenge file is gone and the operator must restart
    // the flow.
    if let Err(err) = delete_challenge_file(&project_dir, &project_id, &state) {
        return forge_error_to_html(err);
    }
    let callback = AuthCallback {
        project_id: project_id.clone(),
        state: state.clone(),
        code,
        error,
        error_description,
    };
    let verifier: &dyn crate::identity::BrowserAuthVerifier = &*config.browser_auth_verifier;
    let session =
        match complete_browser_auth(&project_dir, &cfg, &callback, &challenge, verifier, now) {
            Ok(session) => session,
            Err(err) => return forge_error_to_html(err),
        };
    if let Ok(reg) = Registry::open(db_path) {
        let _ = reg.record_operation(
            "identity",
            &project_id,
            "done",
            &format!(
                "browser auth: session {} minted for subject {}",
                session.session_id, session.subject
            ),
        );
    }
    let return_path = safe_return_path(request.query.as_deref(), "/ui");
    let mut response = html_response(
        303,
        sign_in_page(
            "Forge sign-in",
            &project_id,
            "/ui",
            &return_path,
            API_CONTRACT_VERSION,
        ),
    );
    let cookie = session_cookie_value(&session, config);
    response.headers.insert("set-cookie".to_string(), cookie);
    response
        .headers
        .insert("location".to_string(), return_path.clone());
    response
}

/// Append an expiring `forge_oidc_state` cookie to the
/// response. If the response already carries a
/// `set-cookie` (the success path's `forge_session`), the
/// transients are combined into the single header the
/// transport can express.
fn with_expired_state_cookie(mut response: ApiResponse) -> ApiResponse {
    let expiry = format!(
        "{}=; Path=/ui/auth/callback; HttpOnly; SameSite=Lax; Expires=Thu, 01 Jan 1970 00:00:00 GMT",
        STATE_COOKIE
    );
    let combined = match response.headers.get("set-cookie") {
        Some(existing) => format!("{existing}, {expiry}"),
        None => expiry,
    };
    response.headers.insert("set-cookie".to_string(), combined);
    response
}

/// `POST /ui/sign-out`.
///
/// Revokes only the project that owns the cookie-presented
/// session, clears the cookie, and 303-redirects to
/// `/ui/sign-in`. The handler requires an exact
/// same-origin `Origin` header on every call; missing or
/// mismatched origin is refused before any session is
/// touched.
pub fn handle_sign_out(
    db_path: &Path,
    config: &ApiConfig,
    request: &ApiRequest,
    now: DateTime<Utc>,
) -> ApiResponse {
    if !wants_html(request) {
        return json_only_response();
    }
    let origin = request.header("origin");
    if origin.is_none() {
        return render_error(
            403,
            "ui-origin-mismatch",
            "sign-out requires a same-origin `Origin` header; refusing the request \
             before any session or cookie state is changed",
            ErrorKind::Auth,
        );
    }
    if let AuthDecision::Refuse { code, message } = check_origin(origin, config) {
        return render_error(403, code, message, ErrorKind::Auth);
    }
    if let Some(session_id) = request.cookies.get(SESSION_COOKIE).cloned() {
        if !session_id.is_empty() && is_hex_token(&session_id) {
            if let Ok(registry) = Registry::open(db_path) {
                let projects: Vec<(String, std::path::PathBuf)> = registry
                    .list()
                    .ok()
                    .map(|records| {
                        records
                            .into_iter()
                            .map(|r| (r.id, std::path::PathBuf::from(r.path)))
                            .collect()
                    })
                    .unwrap_or_default();
                if let Ok(Some((mut session, owner_id, owner_dir))) =
                    identity::lookup_session_across_projects(&session_id, projects)
                {
                    if let Err(err) = validate_session_for_signout(&session, &owner_id, now) {
                        return forge_error_to_html(err);
                    }
                    terminate_session(&mut session);
                    let _ = identity::delete_session_file(&owner_dir, &owner_id, &session_id);
                    if let Ok(reg) = Registry::open(db_path) {
                        let _ = reg.record_operation(
                            "identity",
                            &owner_id,
                            "done",
                            &format!("browser sign-out: session {} revoked", session.session_id),
                        );
                    }
                }
            }
        }
    }
    let mut response = html_response(
        303,
        sign_in_page(
            "Forge sign-in",
            "",
            "/ui/sign-in",
            "/ui/sign-in",
            API_CONTRACT_VERSION,
        ),
    );
    response
        .headers
        .insert("set-cookie".to_string(), clear_session_cookie(config));
    response
        .headers
        .insert("location".to_string(), "/ui/sign-in".to_string());
    response
}

/// Resolve the project id from the persisted challenge
/// without exposing any other field. The state value is
/// searched across the registered projects so a callback
/// does not have to carry `project` in the query string.
fn parse_state_project_id(db_path: &Path, state: &str) -> Result<String, ForgeError> {
    let registry = Registry::open(db_path)?;
    let projects: Vec<(String, std::path::PathBuf)> = registry
        .list()?
        .into_iter()
        .map(|r| (r.id, std::path::PathBuf::from(r.path)))
        .collect();
    for (project_id, dir) in projects {
        let path = identity::challenge_path_for(&dir, &project_id, state)?;
        if path.exists() {
            return Ok(project_id);
        }
    }
    Err(ForgeError::IdentityInvalid {
        reason: format!(
            "no pending OIDC challenge with state `{state}`; the round trip cannot \
             be matched to a project"
        ),
    })
}

/// Pull `state`, `code`, `error`, and `error_description`
/// out of the callback query string. Provider errors are
/// preserved so a failed round trip is reported with the
/// same field the provider used; the value flows through
/// the existing redaction on its way to a `ForgeError`.
fn parse_callback_query(
    query: Option<&str>,
) -> Result<(String, String, Option<String>, Option<String>), String> {
    let raw = query.ok_or_else(|| {
        "auth callback requires a query string carrying the OIDC state and code".to_string()
    })?;
    let mut state: Option<String> = None;
    let mut code: Option<String> = None;
    let mut error: Option<String> = None;
    let mut error_description: Option<String> = None;
    for pair in raw.split('&') {
        let (raw_key, raw_val) = match pair.split_once('=') {
            Some(parts) => parts,
            None => continue,
        };
        let key = percent_decode(raw_key);
        let value = percent_decode(raw_val);
        match key.as_str() {
            "state" => state = Some(value),
            "code" => code = Some(value),
            "error" => error = Some(value),
            "error_description" => error_description = Some(value),
            _ => {}
        }
    }
    let state =
        state.ok_or_else(|| "auth callback is missing the OIDC `state` parameter".to_string())?;
    let code = code.unwrap_or_default();
    Ok((state, code, error, error_description))
}

/// Build the provider's authorization URL from the
/// challenge and the project configuration. The
/// deterministic local OIDC issuer (or any real
/// production provider) sees the same parameters an
/// operator would type into a browser: response_type,
/// client_id, redirect_uri, scope, state, nonce,
/// code_challenge, code_challenge_method, and (for
/// providers that take it) the audience.
fn build_provider_authorization_url(
    cfg: &IdentityConfig,
    challenge: &crate::identity::AuthChallenge,
    return_path: &str,
) -> Result<String, ForgeError> {
    let _ = return_path; // sanitised at the route level; the provider does not see it
    let base = cfg.issuer.trim_end_matches('/');
    let url = format!(
        "{base}/protocol/openid-connect/auth?response_type=code&client_id={client_id}\
         &redirect_uri={redirect_uri}&scope={scope}&state={state}&nonce={nonce}\
         &code_challenge={code_challenge}&code_challenge_method=S256",
        base = base,
        client_id = percent_encode(&cfg.client_id),
        redirect_uri = percent_encode(&cfg.redirect_uri),
        scope = percent_encode(&challenge.scope),
        state = percent_encode(&challenge.state),
        nonce = percent_encode(&challenge.nonce),
        code_challenge = percent_encode(&challenge.code_challenge),
    );
    Ok(url)
}

/// Pull the `project` query value out of a sign-in query
/// string. The value is percent-decoded so a caller can
/// pass a normal project id; it is validated by the
/// handler's `validate_project_id` before it reaches the
/// registry.
fn parse_query_project_id(query: Option<&str>) -> Option<String> {
    let raw = query?;
    for pair in raw.split('&') {
        let (raw_key, raw_val) = match pair.split_once('=') {
            Some(parts) => parts,
            None => continue,
        };
        if raw_key == "project" && !raw_val.is_empty() {
            return Some(percent_decode(raw_val));
        }
    }
    None
}

/// Sanitise a `return` query value. The path must be a
/// relative path under `/ui`; anything else (absolute URL,
/// protocol-relative URL, empty, malformed) falls back to
/// `/ui`. The function never produces an external redirect.
fn safe_return_path(query: Option<&str>, fallback: &str) -> String {
    let raw = match query {
        Some(q) => q,
        None => return fallback.to_string(),
    };
    let mut return_path: Option<String> = None;
    for pair in raw.split('&') {
        let (raw_key, raw_val) = match pair.split_once('=') {
            Some(parts) => parts,
            None => continue,
        };
        if raw_key == "return" {
            return_path = Some(percent_decode(raw_val));
        }
    }
    let candidate = match return_path {
        Some(value) => value,
        None => return fallback.to_string(),
    };
    if candidate.is_empty() {
        return fallback.to_string();
    }
    if candidate.contains("://") || candidate.starts_with("//") {
        return fallback.to_string();
    }
    if !candidate.starts_with('/') {
        return fallback.to_string();
    }
    if !candidate.starts_with("/ui") {
        return fallback.to_string();
    }
    // Reject shell metacharacters and control characters
    // so a sanitised return value cannot smuggle a
    // header-split or a javascript URL fragment.
    for ch in candidate.chars() {
        if ch.is_control() || matches!(ch, ';' | '|' | '`' | '\n' | '\r') {
            return fallback.to_string();
        }
    }
    candidate
}

fn percent_encode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*byte as char);
            }
            _ => {
                out.push('%');
                out.push_str(&format!("{:02X}", byte));
            }
        }
    }
    out
}

fn session_cookie_value(session: &crate::identity::AdminSession, config: &ApiConfig) -> String {
    let secure = if is_loopback_origin(config) {
        ""
    } else {
        "; Secure"
    };
    let expires = rfc1123_expires(session.expires_at);
    format!(
        "{}={}; Path=/ui; HttpOnly; SameSite=Lax{}; Expires={}",
        SESSION_COOKIE, session.session_id, secure, expires
    )
}

fn clear_session_cookie(config: &ApiConfig) -> String {
    let secure = if is_loopback_origin(config) {
        ""
    } else {
        "; Secure"
    };
    format!(
        "{}=; Path=/ui; HttpOnly; SameSite=Lax{}; Expires=Thu, 01 Jan 1970 00:00:00 GMT",
        SESSION_COOKIE, secure
    )
}

fn rfc1123_expires(at: DateTime<Utc>) -> String {
    at.format("%a, %d %b %Y %H:%M:%S GMT").to_string()
}

fn is_loopback_origin(config: &ApiConfig) -> bool {
    use std::net::IpAddr;
    matches!(config.bind, IpAddr::V4(v4) if v4.is_loopback())
        || matches!(config.bind, IpAddr::V6(v6) if v6.is_loopback())
}

/// Resolve the on-disk root of a registered project so a
/// cookie session can be loaded from the project's own
/// identity directory.
fn project_root(db_path: &Path, project_id: &str) -> Result<std::path::PathBuf, ForgeError> {
    let registry = Registry::open(db_path)?;
    let record = registry.inspect(project_id)?;
    Ok(std::path::PathBuf::from(record.path))
}

/// Refuse a request whose credential came from the
/// `forge_session` cookie but does not resolve to a live,
/// unexpired, unrevoked session owned by the requested
/// project. Bearer-sourced UI automation keeps the
/// existing presence-only contract during migration, and
/// an absent cookie falls through to the existing
/// unauthorized response.
fn refuse_invalid_cookie_session(
    db_path: &Path,
    project_id: &str,
    request: &ApiRequest,
) -> Option<ApiResponse> {
    if bearer_is_present(request) {
        return None;
    }
    let session_id = request.cookies.get(SESSION_COOKIE)?.trim().to_string();
    if session_id.is_empty() {
        return None;
    }
    let root = match project_root(db_path, project_id) {
        Ok(root) => root,
        Err(err) => return Some(forge_error_to_html(err)),
    };
    match load_session(&root, project_id, &session_id) {
        Ok(Some(session)) => {
            if session.state == SessionState::Revoked || session.is_expired(Utc::now()) {
                Some(render_error(
                    401,
                    "api-unauthorized",
                    "the browser session is expired or revoked; sign in again",
                    ErrorKind::Auth,
                ))
            } else {
                None
            }
        }
        Ok(None) => Some(render_error(
            401,
            "api-unauthorized",
            "the browser session is not valid for this project",
            ErrorKind::Auth,
        )),
        Err(err) => Some(forge_error_to_html(err)),
    }
}

fn bearer_is_present(request: &ApiRequest) -> bool {
    request
        .bearer_token
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_some()
}

fn validate_session_for_signout(
    session: &identity::AdminSession,
    owner_id: &str,
    now: DateTime<Utc>,
) -> Result<(), ForgeError> {
    if session.project_id != owner_id {
        return Err(ForgeError::IdentitySessionCrossProject {
            reason: format!(
                "session `{}` was minted for project `{}`; sign-out cannot be performed \
                 against project `{owner_id}`",
                session.session_id, session.project_id
            ),
        });
    }
    if session.state == SessionState::Revoked {
        return Ok(());
    }
    if session.is_expired(now) {
        return Ok(());
    }
    Ok(())
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
            cookies: BTreeMap::new(),
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
            cookies: BTreeMap::new(),
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
            cookies: BTreeMap::new(),
            remote_addr: None,
            started_at: chrono::Utc::now(),
        };
        assert_eq!(extract_token(&req).as_deref(), Some("deadbeef"));

        req.bearer_token = None;
        req.query = Some("token=cafebabe".to_string());
        // A query-string token is never authenticated; the
        // browser credential must arrive via the session
        // cookie or an Authorization bearer header.
        assert!(extract_token(&req).is_none());

        req.query = None;
        assert!(extract_token(&req).is_none());
    }

    #[test]
    fn require_token_returns_401_when_missing() {
        let resp = require_token(None).unwrap();
        assert_eq!(resp.status, 401);
    }
}
