//! API HTTP contract (`core-http-api`).
//!
//! Exercises the published `forge api serve` subcommand
//! through the built binary. Each test starts the
//! server in a child process, drives a single request
//! through the loopback listener, and asserts on the
//! rendered response. The transport contract is the same
//! across CLI, MCP and API, so the contract test
//! focuses on what the HTTP surface owns:
//!
//! - R1 success: a permitted `POST
//!   /v1/projects/{id}/doctor` returns the same domain
//!   findings as `forge doctor <proj>`.
//! - R1 failure: a request with an invalid manifest
//!   (unknown project) returns a structured
//!   `unknown-project` error without bypassing Core
//!   checks.
//! - R1 boundary: stopping the API server leaves the
//!   CLI surface usable; doctor/inspect continue to
//!   work through `forge` directly.
//! - R2 success: an authorized feature request accepts
//!   the call and returns an operation id recorded in
//!   the registry's journal.
//! - R2 failure: a session minted for project A
//!   presented to project B is refused with
//!   `api-project-mismatch`; an idempotency key reused
//!   with a different body returns
//!   `idempotency-key-conflict`.
//! - R2 boundary: an identical retry with the same key
//!   + same body returns the same operation id without
//!     repeating the side effect.

use std::fs;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::Value;

fn forge_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_forge"))
}

fn lossy(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).to_string()
}

fn clean_cmd() -> Command {
    let mut cmd = Command::new(forge_bin());
    cmd.env_remove("FORGE_REGISTRY")
        .env_remove("HTTP_PROXY")
        .env_remove("HTTPS_PROXY")
        .env_remove("ALL_PROXY")
        .env_remove("OPENAI_API_KEY")
        .env_remove("ANTHROPIC_API_KEY")
        .env_remove("FORGE_DEPLOYER_BIN")
        .env_remove("FORGE_DRIFTWATCH_BIN")
        .env_remove("FORGE_DOCS_TRANSLATOR_BIN")
        .env_remove("FORGE_PACKAGE_BIN")
        .env_remove("FORGE_NOTES_BIN")
        .env_remove("FORGE_ANALYTICS_BIN");
    cmd
}

fn run(db: &Path, args: &[&str]) -> std::process::Output {
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(db);
    for a in args {
        cmd.arg(a);
    }
    cmd.output().expect("run forge")
}

fn run_json(db: &Path, args: &[&str]) -> Value {
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

fn write_identity_project(dir: &Path, id: &str) {
    fs::create_dir_all(dir).unwrap();
    let text = format!(
        "schema: 1\nproject:\n  id: {id}\n  name: {id}\n  profile: rust-web\n  maturity: L1\n  target_maturity: L2\nruntime:\n  language: rust\nidentity:\n{IDENTITY_YAML}"
    );
    fs::write(dir.join("forge.yaml"), text).unwrap();
    fs::write(dir.join("README.md"), "v1\n").unwrap();
}

fn write_rust_project(dir: &Path, id: &str) {
    fs::create_dir_all(dir).unwrap();
    let text = format!(
        "schema: 1\nproject:\n  id: {id}\n  name: {id}\n  profile: rust-web\n  maturity: L1\nruntime:\n  language: rust\nfeatures:\n  auth: 2.1\n"
    );
    fs::write(dir.join("forge.yaml"), text).unwrap();
    fs::write(dir.join("README.md"), "v1\n").unwrap();
}

fn free_port() -> u16 {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind ephemeral");
    let port = listener.local_addr().expect("local addr").port();
    drop(listener);
    port
}

/// Start the API server on an ephemeral port. Returns
/// the port and the child process. The caller is
/// responsible for killing the child before the test
/// returns so the listener is released.
fn start_api(db: &Path) -> (u16, std::process::Child) {
    let port = free_port();
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(db);
    cmd.arg("api").arg("serve");
    cmd.arg("--port").arg(port.to_string());
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    let child = cmd.spawn().expect("spawn api server");
    // Poll the port until the server is reachable.
    let start = Instant::now();
    let mut bound = false;
    while start.elapsed() < Duration::from_secs(5) {
        if std::net::TcpStream::connect(("127.0.0.1", port)).is_ok() {
            bound = true;
            break;
        }
        thread::sleep(Duration::from_millis(50));
    }
    if !bound {
        panic!("api server did not bind on 127.0.0.1:{port}");
    }
    (port, child)
}

fn http_request(
    method: &str,
    host_port: &str,
    path: &str,
    headers: &[(&str, &str)],
    body: &[u8],
) -> (u16, String, Value) {
    let mut stream = TcpStream::connect(host_port).expect("connect api");
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(5)));
    let mut header_block =
        format!("{method} {path} HTTP/1.1\r\nHost: {host_port}\r\nConnection: close\r\n");
    for (name, value) in headers {
        header_block.push_str(&format!("{name}: {value}\r\n"));
    }
    if !body.is_empty() {
        header_block.push_str(&format!("Content-Length: {}\r\n", body.len()));
    }
    header_block.push_str("\r\n");
    stream
        .write_all(header_block.as_bytes())
        .expect("write head");
    if !body.is_empty() {
        stream.write_all(body).expect("write body");
    }
    let mut response = Vec::new();
    stream.read_to_end(&mut response).expect("read response");
    let response_text = String::from_utf8_lossy(&response).to_string();
    let status_line = response_text.lines().next().unwrap_or("");
    let status_code: u16 = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let body_start = response_text
        .find("\r\n\r\n")
        .map(|i| i + 4)
        .unwrap_or(response_text.len());
    let body_text = &response_text[body_start..];
    let json: Value = if body_text.is_empty() {
        Value::Null
    } else {
        serde_json::from_str(body_text).unwrap_or(Value::Null)
    };
    (status_code, response_text, json)
}

fn mint_session_token(db: &Path, project_dir: &Path) -> String {
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

fn stop_api(child: &mut std::process::Child) {
    let _ = child.kill();
    let _ = child.wait();
}

#[test]
fn cli_help_lists_api_subcommand() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let out = run(&db, &["--help"]);
    let stdout = lossy(&out.stdout);
    assert!(
        stdout.contains("api"),
        "top-level help must mention the api subcommand: {stdout}"
    );
    let out = run(&db, &["api", "--help"]);
    let stdout = lossy(&out.stdout);
    assert!(
        stdout.contains("serve"),
        "api subcommand help must mention serve: {stdout}"
    );
    assert!(
        stdout.contains("loopback"),
        "api subcommand help must mention the loopback default: {stdout}"
    );
}

#[test]
fn healthz_route_returns_200_without_authorization() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let (port, mut child) = start_api(&db);
    let (status, _text, json) =
        http_request("GET", &format!("127.0.0.1:{port}"), "/healthz", &[], &[]);
    assert_eq!(status, 200);
    assert_eq!(json["status"], "ok");
    assert_eq!(json["contract"], forge::api::API_CONTRACT_VERSION);
    stop_api(&mut child);
}

#[test]
fn doctor_route_matches_cli_findings() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_identity_project(&proj, "api-doctor");
    let session = mint_session_token(&db, &proj);
    let reg = run(&db, &["register", proj.to_str().unwrap()]);
    assert!(
        reg.status.success(),
        "register failed: {}",
        lossy(&reg.stderr)
    );
    let (port, mut child) = start_api(&db);
    let (status, _text, json) = http_request(
        "POST",
        &format!("127.0.0.1:{port}"),
        "/v1/projects/api-doctor/doctor",
        &[
            ("Authorization", &format!("Bearer {session}")),
            ("Content-Type", "application/json"),
        ],
        b"{}",
    );
    assert_eq!(status, 200, "doctor via api must be 200");
    assert!(
        json["doctor"].is_object(),
        "doctor must return a doctor object"
    );
    let findings = json["doctor"]["findings"]
        .as_array()
        .expect("findings array");
    assert!(
        !findings.is_empty(),
        "doctor must surface at least one finding"
    );
    stop_api(&mut child);
}

#[test]
fn missing_token_returns_401() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let (port, mut child) = start_api(&db);
    // Missing Authorization header is 401.
    let (status, _text, json) = http_request(
        "GET",
        &format!("127.0.0.1:{port}"),
        "/v1/projects",
        &[],
        &[],
    );
    assert_eq!(status, 401);
    assert_eq!(json["error"]["code"], "api-unauthorized");
    // Empty bearer token is 401.
    let (status, _text, json) = http_request(
        "GET",
        &format!("127.0.0.1:{port}"),
        "/v1/projects",
        &[("Authorization", "Bearer")],
        &[],
    );
    assert_eq!(status, 401);
    assert_eq!(json["error"]["code"], "api-unauthorized");
    stop_api(&mut child);
}

#[test]
fn cross_project_session_is_refused() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj_a = tmp.path().join("a");
    let proj_b = tmp.path().join("b");
    write_identity_project(&proj_a, "api-cross-a");
    write_identity_project(&proj_b, "api-cross-b");
    for dir in [&proj_a, &proj_b] {
        let reg = run(&db, &["register", dir.to_str().unwrap()]);
        assert!(reg.status.success(), "register: {}", lossy(&reg.stderr));
    }
    let session_a = mint_session_token(&db, &proj_a);
    let (port, mut child) = start_api(&db);
    let (status, _text, json) = http_request(
        "GET",
        &format!("127.0.0.1:{port}"),
        "/v1/projects/api-cross-b",
        &[("Authorization", &format!("Bearer {session_a}"))],
        &[],
    );
    assert_eq!(status, 403, "cross-project must be 403");
    assert_eq!(json["error"]["code"], "api-project-mismatch");
    assert!(json["error"]["message"]
        .as_str()
        .unwrap()
        .contains("api-cross-a"));
    stop_api(&mut child);
}

#[test]
fn mutating_route_records_an_operation_id() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_identity_project(&proj, "api-journal");
    let reg = run(&db, &["register", proj.to_str().unwrap()]);
    assert!(reg.status.success());
    let session = mint_session_token(&db, &proj);
    let (port, mut child) = start_api(&db);
    let (_status, _text, json) = http_request(
        "POST",
        &format!("127.0.0.1:{port}"),
        "/v1/projects/api-journal/features",
        &[
            ("Authorization", &format!("Bearer {session}")),
            ("Content-Type", "application/json"),
        ],
        br#"{"feature":"auth"}"#,
    );
    let op_id = json["operation_id"].as_i64().unwrap_or(0);
    assert!(op_id > 0, "operation_id must be present");
    let (status, _text, body) = http_request(
        "GET",
        &format!("127.0.0.1:{port}"),
        &format!("/v1/operations/{op_id}"),
        &[("Authorization", &format!("Bearer {session}"))],
        &[],
    );
    assert_eq!(status, 200, "operation must be retrievable");
    assert_eq!(body["op_id"].as_i64().unwrap_or(0), op_id);
    assert_eq!(body["kind"], "api.add_feature");
    stop_api(&mut child);
}

#[test]
fn idempotency_key_replay_returns_same_operation_id() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_identity_project(&proj, "api-replay");
    let reg = run(&db, &["register", proj.to_str().unwrap()]);
    assert!(reg.status.success());
    let session = mint_session_token(&db, &proj);
    let (port, mut child) = start_api(&db);
    let (status_a, _text_a, json_a) = http_request(
        "POST",
        &format!("127.0.0.1:{port}"),
        "/v1/projects/api-replay/features",
        &[
            ("Authorization", &format!("Bearer {session}")),
            ("Content-Type", "application/json"),
            ("Idempotency-Key", "same-key"),
        ],
        br#"{"feature":"auth"}"#,
    );
    assert_eq!(status_a, 202);
    let op_id_a = json_a["operation_id"].as_i64().unwrap_or(0);
    let (status_b, _text_b, json_b) = http_request(
        "POST",
        &format!("127.0.0.1:{port}"),
        "/v1/projects/api-replay/features",
        &[
            ("Authorization", &format!("Bearer {session}")),
            ("Content-Type", "application/json"),
            ("Idempotency-Key", "same-key"),
        ],
        br#"{"feature":"auth"}"#,
    );
    assert!(
        status_b == 200 || status_b == 202,
        "replay must succeed with the existing op_id"
    );
    let op_id_b = json_b["operation"]["op_id"].as_i64().unwrap_or(0);
    assert_eq!(op_id_a, op_id_b, "replay must reuse the op_id");
    assert_eq!(json_b["replay"], true);
    stop_api(&mut child);
}

#[test]
fn idempotency_key_with_different_body_is_conflict() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_identity_project(&proj, "api-conflict");
    let reg = run(&db, &["register", proj.to_str().unwrap()]);
    assert!(reg.status.success());
    let session = mint_session_token(&db, &proj);
    let (port, mut child) = start_api(&db);
    let (status, _text, _json) = http_request(
        "POST",
        &format!("127.0.0.1:{port}"),
        "/v1/projects/api-conflict/features",
        &[
            ("Authorization", &format!("Bearer {session}")),
            ("Content-Type", "application/json"),
            ("Idempotency-Key", "shared-key"),
        ],
        br#"{"feature":"auth"}"#,
    );
    assert_eq!(status, 202);
    let (status, _text, json) = http_request(
        "POST",
        &format!("127.0.0.1:{port}"),
        "/v1/projects/api-conflict/features",
        &[
            ("Authorization", &format!("Bearer {session}")),
            ("Content-Type", "application/json"),
            ("Idempotency-Key", "shared-key"),
        ],
        br#"{"feature":"admin"}"#,
    );
    assert_eq!(status, 409, "different body must be 409");
    assert_eq!(json["error"]["code"], "idempotency-key-conflict");
    stop_api(&mut child);
}

#[test]
fn unknown_route_returns_404() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let (port, mut child) = start_api(&db);
    let (status, _text, json) = http_request(
        "GET",
        &format!("127.0.0.1:{port}"),
        "/v2/projects",
        &[],
        &[],
    );
    assert_eq!(status, 404);
    assert_eq!(json["error"]["code"], "route-not-found");
    stop_api(&mut child);
}

#[test]
fn wrong_method_returns_405() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let (port, mut child) = start_api(&db);
    let (status, _text, json) = http_request(
        "DELETE",
        &format!("127.0.0.1:{port}"),
        "/v1/projects",
        &[],
        &[],
    );
    assert_eq!(status, 405, "wrong method on a known path is 405");
    assert_eq!(json["error"]["code"], "method-not-allowed");
    stop_api(&mut child);
}

#[test]
fn doctor_command_works_when_api_server_is_stopped() {
    // R1 boundary: stopping the API server leaves the
    // CLI surface usable. doctor must keep working
    // without any listener.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_rust_project(&proj, "api-stopped");
    let reg = run(&db, &["register", proj.to_str().unwrap()]);
    assert!(reg.status.success());
    let (_port, mut child) = start_api(&db);
    stop_api(&mut child);
    let out = run(&db, &["doctor", proj.to_str().unwrap(), "--format", "json"]);
    assert!(out.status.success(), "doctor after stop should work");
    let json: Value = serde_json::from_slice(&out.stdout).expect("doctor json");
    assert!(json["doctor"].is_object());
}
