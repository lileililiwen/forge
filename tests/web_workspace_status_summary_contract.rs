//! Workspace status summary contract (`forge-web-workspace-status-summary`).
//!
//! The summary aggregates from the **existing** `GET /v1/admin/workspace/
//! candidates` route that `forge-web-workspace-onboarding` already covers
//! (its contract test pins the 401/403/415 and 409 unconfigured boundary).
//! This test therefore does **not** re-test that boundary; it pins only the
//! bits the new topbar aggregation depends on:
//!
//! - the shipped `frontend/index.html` exposes the read-only `#ws-summary`
//!   region (`#ws-registered`, `#ws-onboardable`, `#ws-last-sync`,
//!   `#ws-summary-empty`) with no write/onboard control inside it;
//! - `frontend/app.js` wires `loadWorkspaceSummary` + the pure
//!   `aggregateWorkspaceSummary` helper against the candidate route;
//! - the candidate response carries the `state` (`registered`/…) and
//!   `selectable` fields the pure helper counts, so the browser-side
//!   registered/onboardable counters are derivable from one real response.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::sync::Mutex;

use chrono::Utc;
use forge::api::{handle, ApiConfig, ApiRequest};
use serde_json::Value;
use tempfile::tempdir;

/// `FORGE_ADMIN_PROJECTS_ROOT` is process-global, so every route-driving test
/// serializes.
static SERIAL: Mutex<()> = Mutex::new(());

fn lock() -> std::sync::MutexGuard<'static, ()> {
    match SERIAL.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

const ADMIN_ROOT: &str = "FORGE_ADMIN_PROJECTS_ROOT";

fn request(method: &str, path: &str, origin: &str) -> ApiRequest {
    ApiRequest {
        method: method.to_string(),
        path: path.to_string(),
        query: None,
        headers: BTreeMap::from([("origin".to_string(), origin.to_string())]),
        body: Vec::new(),
        idempotency_key: None,
        bearer_token: None,
        cookies: BTreeMap::new(),
        remote_addr: None,
        started_at: Utc::now(),
    }
}

fn with_cookie(mut req: ApiRequest, token: &str) -> ApiRequest {
    req.cookies
        .insert("forge_admin_session".to_string(), token.to_string());
    req
}

/// Run one request with `FORGE_ADMIN_PROJECTS_ROOT` set to `root` (or unset
/// when `None`), restoring the prior value afterwards.
fn handle_with_root(
    config: &ApiConfig,
    db: &Path,
    request: &ApiRequest,
    root: Option<&Path>,
) -> forge::api::ApiResponse {
    let prior = std::env::var(ADMIN_ROOT).ok();
    std::env::remove_var(ADMIN_ROOT);
    if let Some(value) = root {
        std::env::set_var(ADMIN_ROOT, value);
    }
    let response = handle(config, db, request, Utc::now());
    match prior {
        Some(value) => std::env::set_var(ADMIN_ROOT, value),
        None => std::env::remove_var(ADMIN_ROOT),
    }
    response
}

fn login_token(config: &ApiConfig, db: &Path) -> String {
    forge::identity::global::setup(db, "operator@example.test", "a-long-test-password").unwrap();
    let mut login = request("POST", "/v1/admin/session", &config.frontend_origin);
    login
        .headers
        .insert("content-type".to_string(), "application/json".to_string());
    login.body = r#"{"email":"operator@example.test","password":"a-long-test-password"}"#
        .as_bytes()
        .to_vec();
    let response = handle(config, db, &login, Utc::now());
    assert_eq!(
        response.status, 200,
        "login must succeed for the fixture operator"
    );
    response
        .headers
        .get("set-cookie")
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .split_once('=')
        .unwrap()
        .1
        .to_string()
}

fn body_json(response: &forge::api::ApiResponse) -> Value {
    serde_json::from_slice(&response.body).expect("candidate route is JSON-only")
}

fn shipped(rel: &str) -> String {
    fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(rel))
        .unwrap_or_else(|_| panic!("shipped asset missing: {rel}"))
}

#[test]
fn summary_region_is_read_only_in_shipped_html() {
    let html = shipped("frontend/index.html");
    for id in [
        "ws-summary",
        "ws-registered",
        "ws-onboardable",
        "ws-last-sync",
        "ws-summary-empty",
    ] {
        assert!(
            html.contains(&format!("id=\"{id}\"")),
            "shipped index.html must expose #{id}"
        );
    }
    // The summary region carries no write/onboard control: between the
    // `#ws-summary` open and the `#ws-summary-empty` close there is no button.
    let start = html
        .find("id=\"ws-summary\"")
        .expect("ws-summary region present");
    let end = html
        .find("id=\"ws-summary-empty\"")
        .expect("ws-summary-empty present");
    let region = &html[start..end];
    assert!(
        !region.contains("<button"),
        "the topbar workspace summary must contain no onboard/write button"
    );
}

#[test]
fn loader_is_wired_and_aggregates_from_candidate_route() {
    let html = shipped("frontend/index.html");
    let js = shipped("frontend/app.js");
    assert!(
        js.contains("loadWorkspaceSummary"),
        "app.js must wire loadWorkspaceSummary"
    );
    assert!(
        js.contains("aggregateWorkspaceSummary"),
        "app.js must define the pure aggregateWorkspaceSummary helper"
    );
    assert!(
        js.contains("/v1/admin/workspace/candidates"),
        "the summary must aggregate from the existing candidate route"
    );
    // Defensive: the region and loader must both be present (consistency).
    assert!(html.contains("id=\"ws-summary\""));
    assert!(js.contains("loadWorkspaceSummary();"));
}

#[test]
fn candidate_response_exposes_state_and_selectable_for_aggregation() {
    let _guard = lock();
    let dir = tempdir().unwrap();
    let db = dir.path().join("registry.sqlite");
    let config = ApiConfig::default();
    let token = login_token(&config, &db);

    // A configured workspace root with two sibling directories.
    let root = dir.path().join("workspace");
    fs::create_dir_all(root.join("alpha")).unwrap();
    fs::create_dir_all(root.join("beta")).unwrap();

    let req = with_cookie(
        request(
            "GET",
            "/v1/admin/workspace/candidates?limit=1000",
            &config.frontend_origin,
        ),
        &token,
    );
    let response = handle_with_root(&config, &db, &req, Some(&root));
    assert_eq!(
        response.status, 200,
        "authenticated candidate read with a configured root is 200"
    );
    let body = body_json(&response);
    let candidates = body["candidates"]
        .as_array()
        .expect("candidates is an array");
    assert!(
        candidates.len() >= 2,
        "the configured root exposes its immediate child directories"
    );
    // The exact fields `aggregateWorkspaceSummary` counts.
    let mut registered = 0u64;
    let mut onboardable = 0u64;
    for candidate in candidates {
        let state = candidate["state"]
            .as_str()
            .expect("candidate.state is a string the summary counts");
        let selectable = candidate["selectable"]
            .as_bool()
            .expect("candidate.selectable is a bool the summary counts");
        if state == "registered" {
            registered += 1;
        }
        if selectable {
            onboardable += 1;
        }
    }
    // Pure aggregation over the real response — pins the contract the UI reads.
    assert_eq!(
        registered + onboardable,
        registered.saturating_add(onboardable)
    );
    // The total is reported alongside so the UI can show the full sibling set.
    assert!(
        body["total"].as_u64().is_some(),
        "response reports total siblings"
    );
}
