//! Rendered-behavior verification for the portal responsive/accessibility
//! baseline (`portal-accessible-responsive-ui`).
//!
//! The portal stylesheet is inline, so the shipped page families can be
//! rendered through the real HTTP surface, written to a temp directory with
//! a manifest, and loaded from disk — no server is required. The pinned
//! Playwright harness in `tests/browser/portal-a11y-check.mjs` then measures
//! reflow, semantics, keyboard focus and contrast in a real Chromium engine
//! under light and dark colour schemes.
//!
//! If `node`, the pinned `tests/browser/node_modules/playwright` install, or a
//! Chromium engine is unavailable, the harness exits `2`, this test prints
//! `UNVERIFIED` and returns, and nothing is recorded as a pass. The
//! markup-level contract checks in `tests/portal_ui_contract.rs` are the
//! always-run layer; this test is the rendered-behavior oracle.

use std::collections::BTreeMap;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::process::Command;

use chrono::Utc;
use forge::api::{handle_buffered, ApiConfig, ApiRequest, ApiResponse};
use forge::registry::Registry;

const BEARER: &str = "deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef";
const LOOPBACK_ORIGIN: &str = "http://127.0.0.1:8765";

fn tmp_db_path() -> PathBuf {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut path = dir.path().to_path_buf();
    path.push("forge.db");
    path
}

fn seed_project(path: &PathBuf, id: &str, profile: &str, maturity: &str) {
    {
        let _registry = Registry::open(path).expect("open registry");
    }
    let conn = rusqlite::Connection::open(path).expect("open");
    conn.execute(
        "INSERT INTO projects \
         (id, name, path, profile, maturity, target_maturity, \
          schema_version, platform_version, features, observed_at) \
         VALUES (?1, ?1, ?2, ?3, ?4, NULL, 1, '0.1.0', '{}', datetime('now')) \
         ON CONFLICT(id) DO UPDATE SET \
           name = excluded.name, \
           path = excluded.path, \
           profile = excluded.profile, \
           maturity = excluded.maturity",
        rusqlite::params![id, format!("/tmp/{id}"), profile, maturity],
    )
    .unwrap_or_else(|err| panic!("insert {id}: {err}"));
}

fn drive(config: &ApiConfig, db_path: &PathBuf, request: &ApiRequest) -> ApiResponse {
    let mut full: Vec<u8> = Vec::new();
    full.extend_from_slice(request.method.as_bytes());
    full.extend_from_slice(b" ");
    full.extend_from_slice(request.path.as_bytes());
    if let Some(query) = &request.query {
        full.extend_from_slice(b"?");
        full.extend_from_slice(query.as_bytes());
    }
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

fn html_request(method: &str, path: &str, bearer: Option<&str>, body: Vec<u8>) -> ApiRequest {
    let mut headers = BTreeMap::new();
    headers.insert("accept".to_string(), "text/html".to_string());
    if !body.is_empty() {
        headers.insert(
            "content-type".to_string(),
            "application/x-www-form-urlencoded".to_string(),
        );
        headers.insert("origin".to_string(), LOOPBACK_ORIGIN.to_string());
    }
    if let Some(token) = bearer {
        headers.insert("authorization".to_string(), format!("Bearer {token}"));
    }
    ApiRequest {
        method: method.to_string(),
        path: path.to_string(),
        query: None,
        headers,
        body,
        idempotency_key: None,
        bearer_token: bearer.map(str::to_string),
        cookies: BTreeMap::new(),
        remote_addr: Some("127.0.0.1:9999".parse().unwrap()),
        started_at: Utc::now(),
    }
}

fn page(config: &ApiConfig, db_path: &PathBuf, request: &ApiRequest) -> (u16, String) {
    let resp = drive(config, db_path, request);
    (resp.status, String::from_utf8_lossy(&resp.body).to_string())
}

fn node_available() -> bool {
    Command::new("node")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

#[test]
fn portal_pages_pass_the_browser_a11y_baseline() {
    if !node_available() {
        eprintln!("UNVERIFIED: `node` is unavailable; browser a11y baseline not run");
        return;
    }

    let db_path = tmp_db_path();
    seed_project(&db_path, "alethefy", "rust-web", "L3");
    seed_project(&db_path, "forge", "rust-web", "L2");
    let config = ApiConfig::default();

    let form = format!("token={BEARER}&origin={LOOPBACK_ORIGIN}");
    let confirm = format!("{form}&confirm=yes");
    let portfolio = format!("{form}&tag=responsive");

    let mut pages: Vec<(&str, u16, String)> = Vec::new();
    let (status, body) = page(
        &config,
        &db_path,
        &html_request("GET", "/ui", Some(BEARER), Vec::new()),
    );
    pages.push(("fleet", status, body));
    let (status, body) = page(
        &config,
        &db_path,
        &html_request("GET", "/ui/projects/alethefy", Some(BEARER), Vec::new()),
    );
    pages.push(("detail", status, body));
    let (status, body) = page(
        &config,
        &db_path,
        &html_request(
            "POST",
            "/ui/projects/alethefy/publish",
            Some(BEARER),
            form.clone().into_bytes(),
        ),
    );
    pages.push(("publish-plan", status, body));
    let (status, body) = page(
        &config,
        &db_path,
        &html_request(
            "POST",
            "/ui/projects/alethefy/publish",
            Some(BEARER),
            confirm.into_bytes(),
        ),
    );
    pages.push(("operation-accepted", status, body));
    let (status, body) = page(
        &config,
        &db_path,
        &html_request(
            "POST",
            "/ui/projects/alethefy/portfolio",
            Some(BEARER),
            portfolio.into_bytes(),
        ),
    );
    pages.push(("portfolio-saved", status, body));
    let (status, body) = page(
        &config,
        &db_path,
        &html_request("GET", "/ui/studio/alethefy", Some(BEARER), Vec::new()),
    );
    pages.push(("studio", status, body));
    let (status, body) = page(
        &config,
        &db_path,
        &html_request("GET", "/ui", None, Vec::new()),
    );
    pages.push(("error", status, body));

    for (name, status, body) in &pages {
        assert!(
            *status == 200 || *status == 202 || *status == 401,
            "page {name} returned unexpected status {status}"
        );
        assert!(
            body.contains("<main id=\"main-content\">"),
            "page {name} did not render the shared shell"
        );
    }

    let out_dir = tempfile::tempdir().expect("html tempdir");
    let mut manifest = String::from("[");
    for (index, (name, _, body)) in pages.iter().enumerate() {
        let file = format!("{name}.html");
        std::fs::write(out_dir.path().join(&file), body).expect("write page");
        if index > 0 {
            manifest.push(',');
        }
        manifest.push_str(&format!("{{\"name\":\"{name}\",\"file\":\"{file}\"}}"));
    }
    manifest.push(']');
    std::fs::write(out_dir.path().join("manifest.json"), manifest).expect("write manifest");

    let script = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("browser")
        .join("portal-a11y-check.mjs");
    assert!(script.exists(), "browser harness missing at {script:?}");

    let output = Command::new("node")
        .arg(&script)
        .arg(out_dir.path())
        .output()
        .expect("spawn node");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    match output.status.code() {
        Some(0) => {
            eprintln!("{stdout}");
        }
        Some(2) => {
            eprintln!("UNVERIFIED: browser a11y baseline not run");
            eprintln!("{stdout}{stderr}");
        }
        code => panic!(
            "browser a11y baseline failed (exit {code:?})\n--- stdout ---\n{stdout}\n--- stderr ---\n{stderr}"
        ),
    }
}
