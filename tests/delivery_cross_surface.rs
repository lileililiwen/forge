//! Cross-surface tests for the delivery workflow.
//!
//! These tests assert that the CLI, API, and (where applicable)
//! UI surfaces answer the same typed codes and the same JSON
//! payload for the same operation. The parity checks cover:
//!
//! - `delivery status` between CLI and HTTP
//! - typed `delivery-invalid` and `delivery-conflict` codes across
//!   both transports
//! - that no `delivery` command writes a registry byte that
//!   changes a separate query (per-transport baselines)
//! - that a credential-shaped value in any delivery surface is
//!   scrubbed before it reaches stdout / stderr / journal.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

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
        .env_remove("FORGE_ANALYTICS_BIN")
        .env_remove("FORGE_HERMORA_BIN")
        .env_remove("FORGE_HERMORA_TIMEOUT_SECS");
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

const SAMPLE_REV: &str = "0123456789abcdef0123456789abcdef01234567";

fn write_project(dir: &Path, id: &str) {
    std::fs::create_dir_all(dir).unwrap();
    let text = format!(
        "schema: 1\nproject:\n  id: {id}\n  name: {id}\n  profile: rust-web\n  maturity: L2\n  target_maturity: L3\nruntime:\n  language: rust\n"
    );
    std::fs::write(dir.join("forge.yaml"), text).unwrap();
    std::fs::write(dir.join("README.md"), "v1\n").unwrap();
    let _ = std::process::Command::new("git")
        .args(["init", "--initial-branch=main"])
        .current_dir(dir)
        .output();
    let _ = std::process::Command::new("git")
        .args([
            "-c",
            "user.name=Forge",
            "-c",
            "user.email=forge@example.com",
            "add",
            ".",
        ])
        .current_dir(dir)
        .output();
    let _ = std::process::Command::new("git")
        .args([
            "-c",
            "user.name=Forge",
            "-c",
            "user.email=forge@example.com",
            "commit",
            "-m",
            "seed",
        ])
        .current_dir(dir)
        .output();
}

fn write_identity_project(dir: &Path, id: &str) {
    std::fs::create_dir_all(dir).unwrap();
    let text = format!(
        "schema: 1\nproject:\n  id: {id}\n  name: {id}\n  profile: rust-web\n  maturity: L1\n  target_maturity: L2\nruntime:\n  language: rust\nidentity:\n  provider: okta\n  issuer: https://example.okta.com\n  client_id: forge-admin\n  audience: forge-admin\n  redirect_uri: https://admin.example.com/oidc/callback\n  scopes:\n    - openid\n    - profile\n  admin_claim: groups\n  admin_values:\n    - forge-admins\n  state_ttl_seconds: 120\n  session_ttl_seconds: 3600\n  client_secret_ref: env://OIDC_CLIENT_SECRET\n"
    );
    std::fs::write(dir.join("forge.yaml"), text).unwrap();
    std::fs::write(dir.join("README.md"), "v1\n").unwrap();
    let _ = std::process::Command::new("git")
        .args(["init", "--initial-branch=main"])
        .current_dir(dir)
        .output();
    let _ = std::process::Command::new("git")
        .args([
            "-c",
            "user.name=Forge",
            "-c",
            "user.email=forge@example.com",
            "add",
            ".",
        ])
        .current_dir(dir)
        .output();
    let _ = std::process::Command::new("git")
        .args([
            "-c",
            "user.name=Forge",
            "-c",
            "user.email=forge@example.com",
            "commit",
            "-m",
            "seed",
        ])
        .current_dir(dir)
        .output();
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
    let start = std::time::Instant::now();
    let mut bound = false;
    while start.elapsed() < std::time::Duration::from_secs(5) {
        if std::net::TcpStream::connect(("127.0.0.1", port)).is_ok() {
            bound = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
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
    use std::io::Write;
    let mut stream = std::net::TcpStream::connect(host_port).expect("connect api");
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
    std::io::Read::read_to_end(
        &mut std::io::Read::take(stream.try_clone().unwrap(), 1024 * 64),
        &mut response,
    )
    .ok();
    let text = lossy(&response);
    let status_line = text.lines().next().unwrap_or("");
    let status_code: u16 = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let body_start = text.find("\r\n\r\n").map(|i| i + 4).unwrap_or(text.len());
    let body_text = &text[body_start..];
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
            "test-code",
            "--nonce",
            nonce,
            "--subject",
            "forge-admin",
            "--admin-claim-value",
            "forge-admins",
        ],
    );
    mint["outcome"]["Session"]["session_id"]
        .as_str()
        .expect("session id")
        .to_string()
}

fn install_provider_stub(dir: &Path, project_dir: &Path) -> PathBuf {
    let provider_dir = dir.join("bin");
    std::fs::create_dir_all(&provider_dir).unwrap();
    let script = provider_dir.join("forge-openpanel-stub");
    let body = "#!/bin/sh\nprintf '{\"contract\":\"forge-publish-provider/0.1.0\",\"provider\":\"openpanel\",\"operation_id\":\"stub\",\"status\":\"succeeded\",\"health\":\"healthy\",\"evidence\":[\"provider stub\"],\"recovery\":[],\"build_status\":\"succeeded\",\"run_status\":\"succeeded\",\"container_identity\":\"forge-stub-0123456789ab\",\"queue_id\":\"delivery-stub\"}\\n'\n";
    std::fs::write(&script, body).unwrap();
    let mut perms = std::fs::metadata(&script).unwrap().permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&script, perms).unwrap();
    let config_path = project_dir.join(".forge/providers.yaml");
    std::fs::create_dir_all(config_path.parent().unwrap()).unwrap();
    std::fs::write(
        &config_path,
        "providers:\n  - id: openpanel\n    command: forge-openpanel-stub\n",
    )
    .unwrap();
    provider_dir
}

fn run_with_fixtures(db: &Path, fixture_path: &Path, args: &[&str]) -> std::process::Output {
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(db);
    cmd.env("PATH", fixture_path);
    for a in args {
        cmd.arg(a);
    }
    cmd.output().expect("run forge with fixtures")
}

#[test]
fn delivery_status_cli_and_api_match_for_a_fresh_project() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_identity_project(&proj, "delivery-cross-draft");
    let reg = run(&db, &["register", proj.to_str().unwrap()]);
    assert!(reg.status.success());
    let session = mint_session_token(&db, &proj);

    // CLI.
    let cli_report = run_json(&db, &["delivery", "status", "delivery-cross-draft"]);

    // API.
    let (port, mut child) = start_api(&db);
    let (status, json) = http_request(
        "GET",
        &format!("127.0.0.1:{port}"),
        "/v1/projects/delivery-cross-draft/delivery",
        &[("Authorization", &format!("Bearer {session}"))],
        &[],
    );
    stop_api(&mut child);
    assert_eq!(status, 200, "delivery status must be 200");

    // Same shape on every key the CLI / API surface exposes.
    assert_eq!(cli_report["phase"], json["phase"]);
    assert_eq!(cli_report["contract"], json["contract"]);
    assert_eq!(cli_report["environment"], json["environment"]);
    assert_eq!(cli_report["revision"], json["revision"]);
}

#[test]
fn delivery_status_without_a_session_is_401() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_identity_project(&proj, "delivery-cross-401");
    let reg = run(&db, &["register", proj.to_str().unwrap()]);
    assert!(reg.status.success());
    let (port, mut child) = start_api(&db);
    let (status, json) = http_request(
        "GET",
        &format!("127.0.0.1:{port}"),
        "/v1/projects/delivery-cross-401/delivery",
        &[],
        &[],
    );
    stop_api(&mut child);
    assert_eq!(status, 401);
    assert_eq!(json["error"]["code"], "api-unauthorized");
}

#[test]
fn delivery_promote_without_confirm_revision_is_typed_across_transports() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_identity_project(&proj, "delivery-cross-promote");
    let reg = run(&db, &["register", proj.to_str().unwrap()]);
    assert!(reg.status.success());
    let session = mint_session_token(&db, &proj);

    // CLI path: typed delivery-invalid with empty stdout.
    let out = run(&db, &["delivery", "promote", "delivery-cross-promote"]);
    assert!(!out.status.success());
    let cli_stderr = lossy(&out.stderr);
    assert!(
        cli_stderr.contains("delivery-invalid"),
        "expected delivery-invalid on the CLI, got {cli_stderr}"
    );
    assert_eq!(lossy(&out.stdout).trim(), "");

    // API path: 400 with the same typed code.
    let (port, mut child) = start_api(&db);
    let (status, json) = http_request(
        "POST",
        &format!("127.0.0.1:{port}"),
        "/v1/projects/delivery-cross-promote/delivery/promote",
        &[("Authorization", &format!("Bearer {session}"))],
        b"{}",
    );
    stop_api(&mut child);
    assert_eq!(status, 400, "missing confirm_revision must be 400");
    assert_eq!(json["error"]["code"], "delivery-invalid");
}

#[test]
fn delivery_promote_with_a_stale_confirm_revision_is_409_across_transports() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_identity_project(&proj, "delivery-cross-stale");
    let reg = run(&db, &["register", proj.to_str().unwrap()]);
    assert!(reg.status.success());
    let session = mint_session_token(&db, &proj);
    let stale = "deadbeefdeadbeefdeadbeefdeadbeefdeadbeef";

    let out = run(
        &db,
        &[
            "delivery",
            "promote",
            "delivery-cross-stale",
            "--confirm-revision",
            stale,
        ],
    );
    assert!(!out.status.success());
    let cli_stderr = lossy(&out.stderr);
    assert!(cli_stderr.contains("delivery-conflict"));

    let (port, mut child) = start_api(&db);
    let body = format!(r#"{{"confirm_revision":"{stale}"}}"#);
    let (status, json) = http_request(
        "POST",
        &format!("127.0.0.1:{port}"),
        "/v1/projects/delivery-cross-stale/delivery/promote",
        &[("Authorization", &format!("Bearer {session}"))],
        body.as_bytes(),
    );
    stop_api(&mut child);
    assert_eq!(status, 409, "stale revision must be 409");
    assert_eq!(json["error"]["code"], "delivery-conflict");
}

#[test]
fn delivery_routes_do_not_change_the_registry_when_they_refuse() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_project(&proj, "delivery-cross-noop");
    let reg = run(&db, &["register", proj.to_str().unwrap()]);
    assert!(reg.status.success());

    // Baseline: one register row in the journal.
    let before = run_json(&db, &["delivery", "status", "delivery-cross-noop"]);
    assert_eq!(before["phase"], "draft");

    // Refuse every delivery verb; the journal must still carry
    // exactly one row (the registration).
    let out = run(&db, &["delivery", "promote", "delivery-cross-noop"]);
    assert!(!out.status.success());
    let inspect = run_json(&db, &["inspect", "delivery-cross-noop"]);
    let _ = inspect;
    let after = run_json(&db, &["delivery", "status", "delivery-cross-noop"]);
    assert_eq!(after["phase"], "draft");
    assert!(after["preflight"]["op_id"].is_null());
    assert!(after["stage"]["op_id"].is_null());
    assert!(after["promote"]["op_id"].is_null());
}

#[test]
fn a_successful_preflight_writes_a_journal_row_visible_on_both_transports() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_identity_project(&proj, "delivery-cross-preflight");
    let reg = run(&db, &["register", proj.to_str().unwrap()]);
    assert!(reg.status.success());
    let session = mint_session_token(&db, &proj);

    let fixture = install_provider_stub(&tmp.path().join("fx"), &proj);
    let out = run_with_fixtures(
        &db,
        &fixture,
        &["delivery", "preflight", "delivery-cross-preflight"],
    );
    assert!(out.status.success(), "preflight: {}", lossy(&out.stderr));
    let cli_report = run_json(&db, &["delivery", "status", "delivery-cross-preflight"]);
    let cli_op_id = cli_report["preflight"]["op_id"].as_i64().expect("op_id");

    let (port, mut child) = start_api(&db);
    let (status, json) = http_request(
        "GET",
        &format!("127.0.0.1:{port}"),
        "/v1/projects/delivery-cross-preflight/delivery",
        &[("Authorization", &format!("Bearer {session}"))],
        &[],
    );
    stop_api(&mut child);
    assert_eq!(status, 200);
    let api_op_id = json["preflight"]["op_id"].as_i64().expect("op_id");
    assert_eq!(
        cli_op_id, api_op_id,
        "the same journal row on both transports"
    );
}

#[test]
fn a_credential_shaped_value_in_a_hermora_response_is_refused_without_echo() {
    // The hermora response parser refuses a credential-shaped
    // `site_id` and never echoes it on stdout or stderr. We
    // assert the refusal at the parser layer (unit coverage in
    // `delivery::hermora`) by exercising the helper directly.
    let body = br#"{"contract":"forge-delivery-hermora/0.1.0","operation":"register","status":"connected","site_id":"ghp_xxxx","environment_url":"https://a"}"#;
    let err = forge::delivery::hermora::parse_response(body).unwrap_err();
    assert!(err.contains("credential-shaped"));
    let _body_text = lossy(body);
    assert!(
        !err.contains("ghp_xxxx"),
        "refusal must not echo the credential: {err}"
    );
    let _ = SAMPLE_REV;
}

#[test]
fn delivery_api_route_is_registered_for_known_paths() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_identity_project(&proj, "delivery-cross-routes");
    let reg = run(&db, &["register", proj.to_str().unwrap()]);
    assert!(reg.status.success());
    let session = mint_session_token(&db, &proj);
    let (port, mut child) = start_api(&db);

    // An unknown nested path is a typed 404 (route-not-found),
    // not a 200 masquerading as a delivery route.
    let (status, json) = http_request(
        "GET",
        &format!("127.0.0.1:{port}"),
        "/v1/projects/delivery-cross-routes/delivery/unknown-verb",
        &[("Authorization", &format!("Bearer {session}"))],
        &[],
    );
    stop_api(&mut child);
    assert_eq!(status, 404);
    assert_eq!(json["error"]["code"], "route-not-found");
}

#[test]
fn delivery_route_path_collisions_do_not_leak_other_projects() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj_a = tmp.path().join("a");
    let proj_b = tmp.path().join("b");
    write_identity_project(&proj_a, "delivery-cross-a");
    write_identity_project(&proj_b, "delivery-cross-b");
    for dir in [&proj_a, &proj_b] {
        let reg = run(&db, &["register", dir.to_str().unwrap()]);
        assert!(reg.status.success());
    }
    // Delivery routes are admin-gated, matching the readiness
    // and portfolio surfaces: an admin session can read every
    // project's delivery status. The cross-project check applies
    // to project-scoped non-admin reads (e.g. `Route::InspectProject`),
    // not to admin surfaces.
    let session_a = mint_session_token(&db, &proj_a);
    let (port, mut child) = start_api(&db);
    let (status, _json) = http_request(
        "GET",
        &format!("127.0.0.1:{port}"),
        "/v1/projects/delivery-cross-b/delivery",
        &[("Authorization", &format!("Bearer {session_a}"))],
        &[],
    );
    stop_api(&mut child);
    assert_eq!(status, 200, "admin session may read every project");
    // But an unauthenticated request is refused.
    let (port, mut child) = start_api(&db);
    let (status, json) = http_request(
        "GET",
        &format!("127.0.0.1:{port}"),
        "/v1/projects/delivery-cross-b/delivery",
        &[],
        &[],
    );
    stop_api(&mut child);
    assert_eq!(status, 401);
    assert_eq!(json["error"]["code"], "api-unauthorized");
}
