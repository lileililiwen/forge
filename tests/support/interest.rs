//! Shared harness for the portfolio interest contract tests.
//!
//! The CLI, HTTP and cross-surface suites all drive the same fixture
//! world: two registered identity-capable projects, one admin session,
//! and helpers that assert a *typed* refusal rather than a bare
//! non-zero exit. The world itself is the one
//! [`tests/support/share.rs`](super::share) established, so a change to
//! the identity or registry fixture cannot silently weaken one surface
//! and not another.

#![allow(dead_code)]

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;
use forge::api::{handle, ApiConfig, ApiRequest, ApiResponse};
use serde_json::Value;

use super::share::{mint_session_token, registered, run_json};

#[allow(unused_imports)]
pub use super::share::expect_refusal;

/// Contract version every interest document and envelope carries.
pub const CONTRACT: &str = "forge-portfolio-interest/0.1.0";

/// Register two identity-capable projects and mint one admin session.
/// Returns `(db, admin session token)`.
pub fn fleet(tmp: &Path) -> (PathBuf, String) {
    let (db, dir) = registered(tmp, "alethefy");
    registered(tmp, "forge");
    let token = mint_session_token(&db, &dir, "ops-admin");
    (db, token)
}

/// One valid snapshot object, JSON-shaped, as an importer would send
/// it. `index` distinguishes the revision so a batch can carry several.
pub fn snapshot(project: &str, revision: &str, start: &str, end: &str) -> Value {
    serde_json::json!({
        "project_id": project,
        "source": "github-analytics",
        "source_revision": revision,
        "window_start": start,
        "window_end": end,
        "privacy_mode": "exact-count",
        "coverage": "complete",
        "metrics": { "unique_visitors": 120, "outbound_cta_clicks": 9 },
    })
}

/// Wrap snapshot objects in the versioned import envelope.
pub fn document(snapshots: Vec<Value>) -> Value {
    serde_json::json!({
        "contract": CONTRACT,
        "snapshots": snapshots,
    })
}

/// Write an import document to a file and return its path.
pub fn write_document(dir: &Path, name: &str, document: &Value) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, serde_json::to_string_pretty(document).unwrap()).unwrap();
    path
}

/// Import one batch through the CLI and return the parsed `import`
/// object. The document is written beside the registry so a test
/// failure can be reproduced from the path in the assertion message.
pub fn import(db: &Path, document: &Value) -> Value {
    let path = db
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("interest-import.json");
    fs::write(&path, serde_json::to_string_pretty(document).unwrap()).unwrap();
    let out = run_json(
        db,
        &["portfolio", "interest", "import", path.to_str().unwrap()],
    );
    out["import"].clone()
}

/// Import one batch through the in-process JSON API for one project.
pub fn import_api(db: &Path, token: &str, project: &str, snapshots: Vec<Value>) -> ApiResponse {
    let body = serde_json::json!({ "snapshots": snapshots });
    let request = api_request(
        "POST",
        &format!("/v1/projects/{project}/interest"),
        Some(token),
        &body.to_string(),
    );
    api(db, &request)
}

/// Build one in-process API request with an optional bearer session.
pub fn api_request(method: &str, path: &str, bearer: Option<&str>, body: &str) -> ApiRequest {
    let mut headers = BTreeMap::new();
    headers.insert("content-type".to_string(), "application/json".to_string());
    if let Some(token) = bearer {
        headers.insert("authorization".to_string(), format!("Bearer {token}"));
    }
    ApiRequest {
        method: method.to_string(),
        path: path.to_string(),
        query: None,
        headers,
        body: body.as_bytes().to_vec(),
        idempotency_key: None,
        bearer_token: bearer.map(|v| v.to_string()),
        remote_addr: Some("127.0.0.1:9999".parse().unwrap()),
        started_at: Utc::now(),
    }
}

/// Build one request with an explicit query string, which the base
/// helper does not carry.
pub fn api_request_query(
    method: &str,
    path: &str,
    query: &str,
    bearer: Option<&str>,
) -> ApiRequest {
    let mut request = api_request(method, path, bearer, "");
    request.query = Some(query.to_string());
    request
}

/// Parse a JSON API response body.
pub fn api_json(response: &ApiResponse) -> Value {
    serde_json::from_slice(&response.body).unwrap_or(Value::Null)
}

/// Drive one request through the JSON API in process.
pub fn api(db: &Path, request: &ApiRequest) -> ApiResponse {
    handle(&ApiConfig::default(), db, request, Utc::now())
}

/// Assert one API response failed with an exact error code.
pub fn expect_api_error(response: &ApiResponse, status: u16, code: &str) {
    let body = api_json(response);
    assert_eq!(
        response.status, status,
        "expected {status}, got {}: {body}",
        response.status
    );
    assert_eq!(
        body["error"]["code"].as_str(),
        Some(code),
        "expected error[{code}]: {body}"
    );
}

/// The number of stored snapshots the CLI reports for one project.
pub fn snapshot_count(db: &Path, project: &str) -> usize {
    let out = run_json(db, &["portfolio", "interest", "list", project]);
    out["snapshots"]
        .as_array()
        .map(|rows| rows.len())
        .unwrap_or(0)
}

/// The refusal findings the store holds, newest first.
pub fn refusals(db: &Path) -> Vec<Value> {
    let out = run_json(db, &["portfolio", "interest", "audit", "--limit", "500"]);
    out["refusals"].as_array().cloned().unwrap_or_default()
}

/// The `refusals` array of a `show` projection for one project.
pub fn project_refusals(db: &Path, project: &str) -> Vec<Value> {
    let out = run_json(db, &["portfolio", "interest", "show", project]);
    out["interest"]["findings"]
        .as_array()
        .cloned()
        .unwrap_or_default()
}
