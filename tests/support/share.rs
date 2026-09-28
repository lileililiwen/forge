//! Shared harness for the portfolio share contract tests.
//!
//! The CLI, HTTP and cross-surface suites all drive the same fixture
//! world: two registered identity-capable projects, one admin session,
//! and helpers that assert a *typed* refusal rather than a bare
//! non-zero exit. Keeping that world in one place means a change to
//! the harness cannot silently weaken one surface and not another.

#![allow(dead_code)]

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;
use forge::api::{handle, ApiConfig, ApiRequest, ApiResponse};
use serde_json::Value;

/// Path of the freshly built `forge` binary.
pub fn forge_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_forge"))
}

/// Decode bytes for a human-facing failure message.
pub fn lossy(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).to_string()
}

/// A `forge` invocation with `FORGE_REGISTRY` cleared, so the registry
/// under test is the only one the process can see.
pub fn clean_cmd() -> std::process::Command {
    let mut cmd = std::process::Command::new(forge_bin());
    cmd.env_remove("FORGE_REGISTRY");
    cmd
}

/// Run `forge --registry <db> <args>`.
pub fn run(db: &Path, args: &[&str]) -> std::process::Output {
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(db);
    for a in args {
        cmd.arg(a);
    }
    cmd.output().expect("run forge")
}

/// Run `forge --registry <db> --format json <args>` and parse stdout.
pub fn run_json(db: &Path, args: &[&str]) -> Value {
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(db);
    cmd.arg("--format").arg("json");
    for a in args {
        cmd.arg(a);
    }
    let out = cmd.output().expect("run forge json");
    assert!(
        out.status.success(),
        "forge {args:?} failed: {}",
        lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).unwrap_or_else(|err| {
        panic!(
            "invalid json: {err}; stdout={} stderr={}",
            lossy(&out.stdout),
            lossy(&out.stderr)
        )
    })
}

/// Assert a typed refusal: the exact error code on stderr, a non-zero
/// exit, and nothing printed to stdout. A bare non-zero exit would not
/// prove the refusal was *typed*.
pub fn expect_refusal(db: &Path, args: &[&str], code: &str) {
    let out = run(db, args);
    assert!(
        !out.status.success(),
        "forge {args:?} unexpectedly succeeded"
    );
    assert_eq!(
        out.stdout.len(),
        0,
        "a refusal must print nothing on stdout: {}",
        lossy(&out.stdout)
    );
    let stderr = lossy(&out.stderr);
    assert!(
        stderr.contains(&format!("error[{code}]")),
        "expected error[{code}] in {stderr}"
    );
}

const IDENTITY_YAML: &str = "  provider: okta\n  issuer: https://example.okta.com\n  client_id: forge-admin\n  audience: forge-admin\n  redirect_uri: https://admin.example.com/oidc/callback\n  scopes:\n    - openid\n    - profile\n  admin_claim: groups\n  admin_values:\n    - forge-admins\n  state_ttl_seconds: 120\n  session_ttl_seconds: 3600\n  client_secret_ref: env://OIDC_CLIENT_SECRET\n";

/// Write one identity-capable project so the API authorization
/// boundary has something to resolve a session against.
pub fn write_identity_project(dir: &Path, id: &str) {
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

/// Register one project into a fresh registry, returning `(db, dir)`.
pub fn registered(tmp: &Path, id: &str) -> (PathBuf, PathBuf) {
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

/// Mint an admin session for one project through the identity flow.
pub fn mint_session_token(db: &Path, project_dir: &Path, subject: &str) -> String {
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
            subject,
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

/// Register two projects and mint an admin session for `alethefy`.
/// Returns `(db, admin session token)`.
pub fn fleet(tmp: &Path) -> (PathBuf, String) {
    let (db, dir) = registered(tmp, "alethefy");
    registered(tmp, "forge");
    let token = mint_session_token(&db, &dir, "ops-admin");
    (db, token)
}

/// The `share set` argument tail every happy-path test reuses.
pub fn share_set_args(project: &str, title: &str) -> Vec<String> {
    vec![
        "portfolio".to_string(),
        "share".to_string(),
        "set".to_string(),
        project.to_string(),
        "--title".to_string(),
        title.to_string(),
        "--summary".to_string(),
        "Deterministic project evidence for public review.".to_string(),
        "--category".to_string(),
        "platform".to_string(),
        "--source-url".to_string(),
        format!("https://github.com/lileililiwen/{project}"),
        "--status".to_string(),
        "demo".to_string(),
        "--surface".to_string(),
        format!("Docs=https://example.com/{project}/docs"),
    ]
}

/// Write one share record through the CLI.
pub fn share_set(db: &Path, project: &str, title: &str) -> Value {
    run_json(
        db,
        &share_set_args(project, title)
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
    )
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

/// Parse a JSON API response body.
pub fn api_json(response: &ApiResponse) -> Value {
    serde_json::from_slice(&response.body).unwrap_or(Value::Null)
}

/// Drive one request through the JSON API in process.
pub fn api(db: &Path, request: &ApiRequest) -> ApiResponse {
    handle(&ApiConfig::default(), db, request, Utc::now())
}

/// Write an executable POSIX shell stub. The publication adapter tests
/// drive one of these; nothing here contacts a network host.
#[cfg(unix)]
pub fn shell_stub(path: &Path, body: &str) {
    use std::os::unix::fs::PermissionsExt;
    fs::write(path, body).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}
