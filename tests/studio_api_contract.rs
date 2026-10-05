//! API contract for the Site Studio live preview
//! (`site-studio-preview-refinement`).
//!
//! The API server is the long-lived process that owns a project's
//! live preview session: a start binds the profile runner and keeps
//! the session in the server's configuration until an explicit stop
//! or server shutdown. These tests drive `handle_buffered` with a
//! real authorized session and a controlled runner stub, so the
//! `ready` → stop lifecycle and the same session remaining live
//! between calls are observable without a live `react-web`
//! toolchain.

use std::collections::BTreeMap;
use std::fs;
use std::io::Cursor;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use chrono::{Duration, Utc};
use forge::api::{handle_buffered, ApiConfig, ApiRequest, ApiResponse};
use forge::registry::Registry;
use forge::studio::{parse_spec_text, save_spec, PORT_RANGE_WIDTH};

#[path = "support/studio_ports.rs"]
mod studio_ports;
use studio_ports::shared_port_base;

/// See `support/studio_ports.rs`: each Studio target prefers a different
/// candidate window.
const SLOT: u16 = 1;

const PROJECT_ID: &str = "studio-api";
/// A hex bearer token; `load_session` resolves it under the
/// project's identity store and never contacts a provider.
const TOKEN: &str = "deadbeef";
const PREVIEW_CONTRACT: &str = "forge-studio-preview/0.1.0";

const APP_SPEC: &str = "schema_version: \"1\"\n\
project_id: studio-api\n\
name: Studio API\n\
profile: react-web\n\
pages:\n\
\x20\x20- route: /\n\
\x20\x20\x20\x20title: Home\n\
\x20\x20\x20\x20sections:\n\
\x20\x20\x20\x20\x20\x20- id: hero-block\n\
\x20\x20\x20\x20\x20\x20\x20\x20kind: hero\n";

fn lossy(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).to_string()
}

fn write_manifest(root: &Path) {
    fs::create_dir_all(root).unwrap();
    fs::write(
        root.join("forge.yaml"),
        "schema: 1\nproject:\n  id: studio-api\n  name: Studio API\n  profile: react-web\n  maturity: L0\n  target_maturity: L1\n",
    )
    .unwrap();
}

/// Write a valid, authorized admin session directly under the
/// project's identity store. Minting through the OIDC CLI is not
/// required to prove the transport ownership boundary.
fn write_identity_session(root: &Path) {
    let dir = root
        .join(".forge/identity")
        .join(PROJECT_ID)
        .join("sessions");
    fs::create_dir_all(&dir).unwrap();
    let now = Utc::now();
    let session = serde_json::json!({
        "session_id": TOKEN,
        "project_id": PROJECT_ID,
        "provider": "okta",
        "subject": "user-1",
        "issued_at": now,
        "expires_at": now + Duration::hours(1),
        "permissions": ["admin:access"],
        "state": "active",
    });
    fs::write(
        dir.join(format!("{TOKEN}.json")),
        serde_json::to_vec_pretty(&session).unwrap(),
    )
    .unwrap();
}

fn has_python3() -> bool {
    Command::new("/usr/bin/env")
        .args(["python3", "-c", "pass"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

/// A bounded "dev server" the profile runner can start: it binds
/// the reserved port from `FORGE_STUDIO_PORT` and answers 200.
fn stub_runner(dir: &Path) -> PathBuf {
    let path = dir.join("stub-runner.py");
    let script = "#!/usr/bin/env python3\n\
import http.server, socketserver, os\n\
port = int(os.environ[\"FORGE_STUDIO_PORT\"])\n\
class H(http.server.BaseHTTPRequestHandler):\n\
\x20\x20\x20\x20def do_GET(self):\n\
\x20\x20\x20\x20\x20\x20\x20\x20self.send_response(200)\n\
\x20\x20\x20\x20\x20\x20\x20\x20self.end_headers()\n\
\x20\x20\x20\x20\x20\x20\x20\x20self.wfile.write(b\"ok\")\n\
\x20\x20\x20\x20def log_message(self, *a):\n\
\x20\x20\x20\x20\x20\x20\x20\x20pass\n\
with socketserver.TCPServer((\"127.0.0.1\", port), H) as httpd:\n\
\x20\x20\x20\x20httpd.serve_forever()\n";
    fs::write(&path, script).unwrap();
    let mut perms = fs::metadata(&path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&path, perms).unwrap();
    path
}

fn make_request(method: &str, path: &str, body: Vec<u8>) -> ApiRequest {
    let mut headers = BTreeMap::new();
    headers.insert("authorization".to_string(), format!("Bearer {TOKEN}"));
    if !body.is_empty() {
        headers.insert("content-type".to_string(), "application/json".to_string());
    }
    ApiRequest {
        method: method.to_string(),
        path: path.to_string(),
        query: None,
        headers,
        body,
        idempotency_key: None,
        bearer_token: Some(TOKEN.to_string()),
        remote_addr: Some("127.0.0.1:9999".parse().unwrap()),
        started_at: Utc::now(),
    }
}

fn drive(config: &ApiConfig, db: &Path, request: &ApiRequest) -> ApiResponse {
    let mut full: Vec<u8> = Vec::new();
    full.extend_from_slice(request.method.as_bytes());
    full.push(b' ');
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
    handle_buffered(config, db, &mut reader, &mut writer, 1024 * 1024).expect("handle_buffered")
}

fn preview_route() -> String {
    format!("/v1/projects/{PROJECT_ID}/studio/preview")
}

fn preview_body(action: &str) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({"action": action, "confirm": "yes"})).unwrap()
}

fn preview_json(response: &ApiResponse) -> serde_json::Value {
    serde_json::from_slice(&response.body).unwrap_or_else(|err| {
        panic!(
            "invalid preview json: {err}; body={}",
            lossy(&response.body)
        )
    })
}

#[test]
fn api_owns_the_live_preview_between_start_and_stop() {
    if !has_python3() {
        eprintln!("skip: python3 is not available to host the runner stub");
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let root = tmp.path().join("project");
    write_manifest(&root);
    let mut registry = Registry::open(&db).unwrap();
    registry.register(&root, None).unwrap();
    write_identity_session(&root);
    fs::write(root.join("forge.app.yaml"), APP_SPEC).unwrap();
    let spec = parse_spec_text(APP_SPEC).unwrap();
    save_spec(&registry, PROJECT_ID, &root, spec, "r0").unwrap();

    // Controlled runner + a dedicated port range for this process.
    let runner = stub_runner(tmp.path());
    // The range is chosen at run time, outside this host's ephemeral window:
    // a hardcoded base inside `ip_local_port_range` loses a port to any
    // unrelated outbound connection on the machine.
    let port_base = shared_port_base(SLOT);
    std::env::set_var("FORGE_STUDIO_RUNNER_BIN", &runner);
    std::env::set_var("FORGE_STUDIO_PORT_RANGE_START", port_base.to_string());
    std::env::set_var("FORGE_STUDIO_STARTUP_TIMEOUT_SECS", "10");

    // One config spans both calls: the server owns the live session.
    let config = ApiConfig::default();

    let response = drive(
        &config,
        &db,
        &make_request("POST", &preview_route(), preview_body("start")),
    );
    assert_eq!(response.status, 200, "body={}", lossy(&response.body));
    let value = preview_json(&response);
    assert_eq!(value["preview"]["contract"], PREVIEW_CONTRACT);
    assert_eq!(value["preview"]["state"], "ready");
    assert_eq!(value["preview"]["project_id"], PROJECT_ID);
    let port = value["preview"]["port"].as_u64().expect("reserved port");
    assert!(
        (u64::from(port_base)..u64::from(port_base) + u64::from(PORT_RANGE_WIDTH)).contains(&port),
        "{port} outside {port_base}"
    );
    let url = value["preview"]["preview_url"]
        .as_str()
        .expect("preview url");
    assert!(url.contains(PROJECT_ID) || url.contains(&port.to_string()));

    // The session is still live on a status read through the same
    // server: the API keeps it until an explicit stop.
    let status = drive(
        &config,
        &db,
        &make_request("GET", &preview_route(), Vec::new()),
    );
    assert_eq!(status.status, 200, "body={}", lossy(&status.body));
    let status_value = preview_json(&status);
    assert_eq!(status_value["preview"]["state"], "ready");
    assert_eq!(status_value["preview"]["port"], port);

    // Explicit stop tears the live session down.
    let stop = drive(
        &config,
        &db,
        &make_request("POST", &preview_route(), preview_body("stop")),
    );
    assert_eq!(stop.status, 200, "body={}", lossy(&stop.body));
    let stop_value = preview_json(&stop);
    assert_eq!(stop_value["preview"]["state"], "stopped");

    let after = drive(
        &config,
        &db,
        &make_request("GET", &preview_route(), Vec::new()),
    );
    let after_value = preview_json(&after);
    assert_eq!(after_value["preview"]["state"], "stopped");
    assert!(after_value["preview"]["preview_url"].is_null());

    std::env::remove_var("FORGE_STUDIO_RUNNER_BIN");
    std::env::remove_var("FORGE_STUDIO_PORT_RANGE_START");
    std::env::remove_var("FORGE_STUDIO_STARTUP_TIMEOUT_SECS");
}

#[test]
fn api_start_requires_confirm_and_refuses_an_unknown_action() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let root = tmp.path().join("project");
    write_manifest(&root);
    let mut registry = Registry::open(&db).unwrap();
    registry.register(&root, None).unwrap();
    write_identity_session(&root);

    let config = ApiConfig::default();

    // Missing confirm is refused before any process work.
    let body = serde_json::to_vec(&serde_json::json!({"action": "start"})).unwrap();
    let response = drive(&config, &db, &make_request("POST", &preview_route(), body));
    assert_eq!(response.status, 400);
    assert!(lossy(&response.body).contains("studio-invalid-spec"));

    // An unknown action is refused with a typed error.
    let response = drive(
        &config,
        &db,
        &make_request("POST", &preview_route(), preview_body("dance")),
    );
    assert_eq!(response.status, 400);
    assert!(lossy(&response.body).contains("studio-invalid-spec"));
}
