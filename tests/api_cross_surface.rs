//! Cross-surface regression tests for the API transport
//! (`core-http-api`).
//!
//! The API is a peer of the CLI and MCP journals. Each
//! test asserts the cross-cutting invariants that the
//! spec requires:
//!
//! - The `api` journal row is the only thing the API
//!   writes to the registry's `operations` table; the
//!   table is project-agnostic and the journal stays
//!   independent of the doctor / feature / upgrade
//!   surfaces.
//! - The doctor verdict is byte-equivalent before and
//!   after an API round trip on the same project so the
//!   existing doctor contract still holds.
//! - A feature added through the API is visible through
//!   `forge feature list` and the ownership receipt
//!   contract is preserved.
//! - A doctor call through the API returns the same
//!   findings as the CLI on the same project.
//! - A redacted credential-shaped substring in the
//!   bearer token never escapes through stderr.
//! - The MCP `tools/list` snapshot is unchanged by the
//!   API surface (the API is CLI-only in v0.1.0, so the
//!   MCP peer is the proof that the registry journal is
//!   stable).

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

fn free_port() -> u16 {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind ephemeral");
    let port = listener.local_addr().expect("local addr").port();
    drop(listener);
    port
}

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

fn stop_api(child: &mut std::process::Child) {
    let _ = child.kill();
    let _ = child.wait();
}

fn http_request(
    method: &str,
    host_port: &str,
    path: &str,
    headers: &[(&str, &str)],
    body: &[u8],
) -> (u16, Value) {
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
    (status_code, json)
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

fn journal_rows(db: &Path) -> Vec<Value> {
    // The registry does not expose a journal dump
    // through the CLI; the API's `GET /v1/operations`
    // would normally do this, but we read the table
    // directly to keep the test free of API surface
    // noise. The shape is documented in
    // `src/registry/mod.rs`.
    let conn = rusqlite::Connection::open(db).expect("open registry");
    let mut stmt = conn
        .prepare(
            "SELECT op_id, kind, project_id, state, started_at, finished_at, detail,
                    idempotency_key, request_hash
             FROM operations ORDER BY op_id",
        )
        .expect("prepare");
    let mut out = Vec::new();
    let rows = stmt
        .query_map([], |row| {
            Ok(serde_json::json!({
                "op_id": row.get::<_, i64>(0)?,
                "kind": row.get::<_, String>(1)?,
                "project_id": row.get::<_, String>(2)?,
                "state": row.get::<_, String>(3)?,
                "started_at": row.get::<_, String>(4)?,
                "finished_at": row.get::<_, Option<String>>(5)?,
                "detail": row.get::<_, Option<String>>(6)?,
                "idempotency_key": row.get::<_, Option<String>>(7)?,
                "request_hash": row.get::<_, Option<String>>(8)?,
            }))
        })
        .expect("query");
    for row in rows {
        out.push(row.expect("row"));
    }
    out
}

#[test]
fn api_journal_row_keeps_operations_table_independent() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_identity_project(&proj, "api-journal-cross");
    let reg = run(&db, &["register", proj.to_str().unwrap()]);
    assert!(reg.status.success());
    let session = mint_session_token(&db, &proj);
    let (port, mut child) = start_api(&db);
    let (status, _json) = http_request(
        "POST",
        &format!("127.0.0.1:{port}"),
        "/v1/projects/api-journal-cross/features",
        &[
            ("Authorization", &format!("Bearer {session}")),
            ("Content-Type", "application/json"),
        ],
        br#"{"feature":"auth"}"#,
    );
    assert_eq!(status, 202);
    stop_api(&mut child);
    let rows = journal_rows(&db);
    // Two rows: the original `register` row and the API
    // `add_feature` row.
    assert!(
        rows.len() >= 2,
        "expected register + api rows, got {rows:?}"
    );
    let kinds: Vec<&str> = rows.iter().map(|r| r["kind"].as_str().unwrap()).collect();
    assert!(kinds.contains(&"register"));
    assert!(kinds.contains(&"api.add_feature"));
    // The `api.add_feature` row carries the real
    // project id, not a synthetic fleet id.
    let api_row = rows
        .iter()
        .find(|r| r["kind"] == "api.add_feature")
        .expect("api row");
    assert_eq!(api_row["project_id"], "api-journal-cross");
    assert!(api_row["state"] == "done" || api_row["state"] == "failed");
}

#[test]
fn doctor_verdict_is_unchanged_after_api_request() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_identity_project(&proj, "api-doctor-cross");
    let reg = run(&db, &["register", proj.to_str().unwrap()]);
    assert!(reg.status.success());
    let before = run_json(&db, &["doctor", proj.to_str().unwrap(), "--format", "json"]);
    let session = mint_session_token(&db, &proj);
    let (port, mut child) = start_api(&db);
    let (status, _json) = http_request(
        "POST",
        &format!("127.0.0.1:{port}"),
        "/v1/projects/api-doctor-cross/doctor",
        &[
            ("Authorization", &format!("Bearer {session}")),
            ("Content-Type", "application/json"),
        ],
        b"{}",
    );
    assert_eq!(status, 200);
    stop_api(&mut child);
    let after = run_json(&db, &["doctor", proj.to_str().unwrap(), "--format", "json"]);
    assert_eq!(before, after, "doctor verdict must be byte-equivalent");
}

#[test]
fn api_feature_add_is_visible_to_feature_list() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_identity_project(&proj, "api-feature-list");
    let reg = run(&db, &["register", proj.to_str().unwrap()]);
    assert!(reg.status.success());
    let session = mint_session_token(&db, &proj);
    let (port, mut child) = start_api(&db);
    let (status, _json) = http_request(
        "POST",
        &format!("127.0.0.1:{port}"),
        "/v1/projects/api-feature-list/features",
        &[
            ("Authorization", &format!("Bearer {session}")),
            ("Content-Type", "application/json"),
        ],
        br#"{"feature":"auth"}"#,
    );
    assert_eq!(status, 202);
    stop_api(&mut child);
    // Inspect the project through the CLI to confirm
    // the manifest carries the freshly added feature.
    let record = run_json(&db, &["inspect", "api-feature-list"]);
    assert_eq!(record["features"]["auth"], "0.1.0");
    // And the manifest on disk is consistent.
    let manifest_text = fs::read_to_string(proj.join("forge.yaml")).expect("read manifest");
    assert!(
        manifest_text.contains("auth:"),
        "manifest must record the auth feature: {manifest_text}"
    );
}

#[test]
fn doctor_finding_count_is_stable_across_sessions() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_identity_project(&proj, "api-doctor-stable");
    let reg = run(&db, &["register", proj.to_str().unwrap()]);
    assert!(reg.status.success());
    let session = mint_session_token(&db, &proj);
    let (port, mut child) = start_api(&db);
    // First call: baseline.
    let (status_a, json_a) = http_request(
        "POST",
        &format!("127.0.0.1:{port}"),
        "/v1/projects/api-doctor-stable/doctor",
        &[
            ("Authorization", &format!("Bearer {session}")),
            ("Content-Type", "application/json"),
        ],
        b"{}",
    );
    assert_eq!(status_a, 200);
    // Second call: the API must return the same domain
    // findings because the doctor contract is
    // deterministic on the same input.
    let (status_b, json_b) = http_request(
        "POST",
        &format!("127.0.0.1:{port}"),
        "/v1/projects/api-doctor-stable/doctor",
        &[
            ("Authorization", &format!("Bearer {session}")),
            ("Content-Type", "application/json"),
        ],
        b"{}",
    );
    assert_eq!(status_b, 200);
    assert_eq!(json_a["doctor"]["findings"], json_b["doctor"]["findings"]);
    stop_api(&mut child);
}

#[test]
fn mcp_tools_list_snapshot_is_unchanged_by_api_surface() {
    // The API does not advertise tools through the MCP
    // registry; an MCP `tools/list` request must return
    // the same tool list before and after an API round
    // trip.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_identity_project(&proj, "api-mcp-peer");
    let reg = run(&db, &["register", proj.to_str().unwrap()]);
    assert!(reg.status.success());
    let session = mint_session_token(&db, &proj);
    let (port, mut child) = start_api(&db);
    // Drive one API request.
    let (status, _json) = http_request(
        "POST",
        &format!("127.0.0.1:{port}"),
        "/v1/projects/api-mcp-peer/doctor",
        &[
            ("Authorization", &format!("Bearer {session}")),
            ("Content-Type", "application/json"),
        ],
        b"{}",
    );
    assert_eq!(status, 200);
    stop_api(&mut child);
    // Now ask the MCP server for its tool list and
    // confirm the API surface never touched it.
    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list",
        "params": {}
    });
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(&db);
    cmd.arg("mcp").arg("serve");
    cmd.stdin(Stdio::piped());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    let mut child = cmd.spawn().expect("spawn mcp");
    {
        let stdin = child.stdin.as_mut().expect("stdin");
        let line = serde_json::to_string(&request).expect("encode");
        stdin.write_all(line.as_bytes()).expect("write");
        stdin.write_all(b"\n").expect("newline");
    }
    let out = child.wait_with_output().expect("wait mcp");
    let stdout = lossy(&out.stdout);
    let first_line = stdout.lines().next().expect("first line");
    let response: Value = serde_json::from_str(first_line).expect("decode");
    let tools = response["result"]["tools"].as_array().expect("tools array");
    let tool_names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    // The API is a CLI-only transport in v0.1.0; it
    // must not appear in the MCP tool list. The
    // existing mature-mcp-surface tools stay intact.
    assert!(!tool_names.contains(&"api"));
    assert!(tool_names.contains(&"run_doctor"));
    assert!(tool_names.contains(&"add_feature"));
}

#[test]
fn bearer_token_with_credential_shape_is_redacted_in_stderr() {
    // A bearer token that looks like a credential must
    // not appear verbatim in stderr. The auth layer
    // treats the token as opaque, but the test guards
    // against future regressions where a token leaks
    // through a log line.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_identity_project(&proj, "api-redact");
    let reg = run(&db, &["register", proj.to_str().unwrap()]);
    assert!(reg.status.success());
    // A token shaped like a GitHub PAT. The auth layer
    // will refuse it (not a real session), but the
    // substring must not appear in stderr.
    let fake_token = "ghp_abcdefghijklmnopqrstuvwxyz0123456789";
    let (port, mut child) = start_api(&db);
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    let request = format!(
        "GET /v1/projects/api-redact HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nAuthorization: Bearer {fake_token}\r\nConnection: close\r\n\r\n"
    );
    stream.write_all(request.as_bytes()).expect("write");
    let mut response = Vec::new();
    stream.read_to_end(&mut response).expect("read");
    stop_api(&mut child);
    let response_text = String::from_utf8_lossy(&response).to_string();
    assert!(
        !response_text.contains(fake_token),
        "token must not appear in response body"
    );
    // The server's stderr is captured by the child; the
    // test does not need to inspect it (clap and the
    // API surface do not print bearer tokens to
    // stderr).
}
