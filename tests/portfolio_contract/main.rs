//! Portfolio metadata and evidence-snapshot contract
//! (`portfolio-metadata-and-review`).
//!
//! Covers the verification oracle in the change design:
//!
//! - Migration round trip: a registry written before the
//!   portfolio package migrates forward with its project and
//!   journal rows intact, and every new registry command works
//!   against the migrated file.
//! - Domain boundaries: unknown project, duplicate tag,
//!   invalid relation type, self relation, malformed evidence
//!   payload and expired freshness bound are all typed
//!   refusals that leave no portfolio state behind.
//! - Stale/unavailable projection: an expired freshness bound
//!   reads `stale` and an unavailable provider reads
//!   `unavailable`; neither is ever reported as a pass.
//! - API contract: the portfolio read and mutation routes answer
//!   through the existing authorization boundary, so an
//!   unauthorized mutation persists no change.
//!
//! The evidence import path is exercised with local fixtures
//! only; no external provider is contacted and no remote or
//! multi-user readiness is claimed here.
//! The target was a single 1,261-line file; it is now a directory of focused
//! submodules, splitting the CLI surface from the HTTP contract. The shared helpers
//! and seed fixtures below stay reachable to every submodule through `super::`.

use std::collections::BTreeMap;
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};

use chrono::Utc;
use forge::api::{handle_buffered, ApiConfig, ApiRequest, ApiResponse};
use forge::registry::Registry;
use serde_json::Value;

mod cli;
mod http;

pub(crate) fn forge_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_forge"))
}

pub(crate) fn lossy(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).to_string()
}

pub(crate) fn clean_cmd() -> std::process::Command {
    let mut cmd = std::process::Command::new(forge_bin());
    cmd.env_remove("FORGE_REGISTRY");
    cmd
}

pub(crate) fn run(db: &Path, args: &[&str]) -> std::process::Output {
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(db);
    for a in args {
        cmd.arg(a);
    }
    cmd.output().expect("run forge")
}

pub(crate) fn run_json(db: &Path, args: &[&str]) -> Value {
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(db);
    cmd.arg("--format").arg("json");
    for a in args {
        cmd.arg(a);
    }
    let out = cmd.output().expect("run forge json");
    serde_json::from_slice(&out.stdout).unwrap_or_else(|err| {
        panic!(
            "invalid json: {err}; stdout={} stderr={}",
            lossy(&out.stdout),
            lossy(&out.stderr)
        )
    })
}

const IDENTITY_YAML: &str = "  provider: okta\n  issuer: https://example.okta.com\n  client_id: forge-admin\n  audience: forge-admin\n  redirect_uri: https://admin.example.com/oidc/callback\n  scopes:\n    - openid\n    - profile\n  admin_claim: groups\n  admin_values:\n    - forge-admins\n  state_ttl_seconds: 120\n  session_ttl_seconds: 3600\n  client_secret_ref: env://OIDC_CLIENT_SECRET\n";

pub(crate) fn write_identity_project(dir: &Path, id: &str) {
    fs::create_dir_all(dir).unwrap();
    let text = format!(
        "schema: 1\nproject:\n  id: {id}\n  name: {id}\n  profile: rust-web\n  maturity: L1\n  target_maturity: L2\nruntime:\n  language: rust\nidentity:\n{IDENTITY_YAML}"
    );
    fs::write(dir.join("forge.yaml"), text).unwrap();
    fs::create_dir_all(dir.join(".forge/identity")).unwrap();
    fs::write(
        dir.join(".forge/identity/config.yaml"),
        format!("schema: 1\nproject: {id}\nidentity:\n{IDENTITY_YAML}"),
    )
    .unwrap();
}

/// Register one identity-capable project and return `(db, dir)`.
pub(crate) fn registered(tmp: &Path, id: &str) -> (PathBuf, PathBuf) {
    let dir = tmp.join(id);
    write_identity_project(&dir, id);
    let db = tmp.join("registry.db");
    let out = run(&db, &["register", dir.to_str().unwrap()]);
    assert!(
        out.status.success(),
        "register {id} failed: {}",
        lossy(&out.stderr)
    );
    (db, dir)
}

pub(crate) fn mint_session_token(db: &Path, project_dir: &Path) -> String {
    let challenge = run_json(
        db,
        &["identity", "build-challenge", project_dir.to_str().unwrap()],
    );
    let state = challenge["challenge"]["state"].as_str().unwrap();
    let nonce = challenge["challenge"]["nonce"].as_str().unwrap();
    let mint = run_json(
        db,
        &[
            "identity",
            "complete-auth",
            project_dir.to_str().unwrap(),
            "--state",
            state,
            "--code",
            "abcd1234",
            "--subject",
            "user-1",
            "--nonce",
            nonce,
            "--admin-claim-value",
            "forge-admins",
        ],
    );
    mint["outcome"]["Session"]["session_id"]
        .as_str()
        .expect("session id")
        .to_string()
}

// --- API contract -------------------------------------------------------

pub(crate) fn drive(config: &ApiConfig, db_path: &Path, request: &ApiRequest) -> ApiResponse {
    let mut full: Vec<u8> = Vec::new();
    full.extend_from_slice(request.method.as_bytes());
    full.extend_from_slice(b" ");
    full.extend_from_slice(request.path.as_bytes());
    full.extend_from_slice(b" HTTP/1.1\r\n");
    for (k, v) in &request.headers {
        full.extend_from_slice(k.as_bytes());
        full.extend_from_slice(b": ");
        full.extend_from_slice(v.as_bytes());
        full.extend_from_slice(b"\r\n");
    }
    if !request.body.is_empty() {
        full.extend_from_slice(format!("content-length: {}\r\n", request.body.len()).as_bytes());
    }
    full.extend_from_slice(b"\r\n");
    full.extend_from_slice(&request.body);
    let mut reader = Cursor::new(full);
    let mut writer = Vec::new();
    handle_buffered(config, db_path, &mut reader, &mut writer, 1024 * 1024)
        .expect("handle_buffered")
}

pub(crate) fn api_request(
    method: &str,
    path: &str,
    bearer: Option<&str>,
    body: &str,
) -> ApiRequest {
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
        cookies: BTreeMap::new(),
        remote_addr: Some("127.0.0.1:9999".parse().unwrap()),
        started_at: Utc::now(),
    }
}

pub(crate) fn api_json(response: &ApiResponse) -> Value {
    serde_json::from_slice(&response.body).unwrap_or(Value::Null)
}

pub(crate) fn seed_two_projects(tmp: &Path) -> (PathBuf, String) {
    let (db, dir) = registered(tmp, "alethefy");
    registered(tmp, "forge");
    let session = mint_session_token(&db, &dir);
    (db, session)
}
