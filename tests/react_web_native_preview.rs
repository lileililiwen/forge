//! Native end-to-end verification for the `react-web` live preview
//! (`react-web-live-preview`).
//!
//! The default `cargo test` run does not execute this test because it
//! performs a real `npm install` and starts a real Vite dev server. Enable
//! it explicitly:
//!
//! ```sh
//! FORGE_NATIVE_REACT_WEB_PREVIEW=1 cargo test --test react_web_native_preview -- --nocapture
//! ```
//!
//! With the opt-in and `npm` available, the test generates the scaffold,
//! installs the pinned toolchain, starts the bounded preview through the
//! shared Core entry point, checks the served app shell, optionally drives
//! the committed Playwright smoke, stops the preview and asserts the
//! reserved port is released. Missing toolchain or browser is reported
//! `UNVERIFIED`; it is never recorded as a pass.

use std::io::{Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpStream};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use forge::generate::{normalize_explicit, render_files};
use forge::registry::Registry;
use forge::studio::{
    parse_spec_text, save_spec, start_preview, stop_preview, PreviewState, ProcessRunner,
};

#[path = "support/studio_ports.rs"]
mod studio_ports;
use studio_ports::shared_port_base;

/// See `support/studio_ports.rs`: each Studio target prefers a different
/// candidate window.
const SLOT: u16 = 3;

const ID: &str = "native-react-preview";

fn has_npm() -> bool {
    Command::new("npm")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn http_get(port: u16) -> Option<String> {
    let mut stream = TcpStream::connect_timeout(
        &SocketAddr::from((Ipv4Addr::LOCALHOST, port)),
        Duration::from_secs(5),
    )
    .ok()?;
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .ok()?;
    stream
        .write_all(b"GET / HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n")
        .ok()?;
    let mut body = String::new();
    stream.read_to_string(&mut body).ok()?;
    Some(body)
}

fn wait_port_free(port: u16, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if std::net::TcpListener::bind((Ipv4Addr::LOCALHOST, port)).is_ok() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    false
}

/// Drive the committed Playwright smoke. Exit 0 means the DOM rendered the
/// greeting, exit 2 means the browser tooling is unavailable (reported
/// `UNVERIFIED`), anything else fails the test.
fn run_browser_smoke(port: u16) {
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/browser/render-check.mjs");
    if !script.exists() {
        eprintln!("UNVERIFIED browser: {} is missing", script.display());
        return;
    }
    let url = format!("http://127.0.0.1:{port}/");
    let expected = format!("hello from {ID}");
    let output = Command::new("node")
        .arg(&script)
        .arg(&url)
        .arg(&expected)
        .output()
        .expect("run the Playwright smoke script");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    match output.status.code() {
        Some(0) => eprintln!("VERIFIED browser: {}", stdout.trim()),
        Some(2) => eprintln!("UNVERIFIED browser: {}", stderr.trim()),
        other => panic!("browser smoke failed ({other:?}):\nstdout: {stdout}\nstderr: {stderr}"),
    }
}

#[test]
fn native_react_web_preview_renders_in_a_browser() {
    if std::env::var("FORGE_NATIVE_REACT_WEB_PREVIEW").as_deref() != Ok("1") {
        eprintln!(
            "UNVERIFIED: set FORGE_NATIVE_REACT_WEB_PREVIEW=1 to run the native react-web preview check"
        );
        return;
    }
    if !has_npm() {
        eprintln!("UNVERIFIED: npm is not available on PATH");
        return;
    }

    let tmp = tempfile::tempdir().unwrap();
    let dest = tmp.path().join(ID);
    let request = normalize_explicit(
        Some("react-web"),
        Some(ID),
        Some("Native React Preview"),
        &[],
        &dest,
        None,
    )
    .expect("normalize react-web request");
    let files = render_files(&request).expect("render react-web files");
    std::fs::create_dir_all(&dest).unwrap();
    for (rel, contents) in &files {
        let path = dest.join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, contents).unwrap();
    }

    // Install the pinned React/Vite toolchain (network/cache required).
    let install = Command::new("npm")
        .args(["install", "--no-audit", "--no-fund"])
        .current_dir(&dest)
        .output()
        .expect("run npm install");
    assert!(
        install.status.success(),
        "npm install failed:\n{}",
        String::from_utf8_lossy(&install.stderr)
    );

    // Register the project and save a Studio spec (revision r1).
    let mut registry = Registry::open(&tmp.path().join("registry.db")).unwrap();
    registry.register(&dest, None).unwrap();
    let spec_text = format!(
        "schema_version: \"1\"\nproject_id: {ID}\nname: Native React Preview\nprofile: react-web\n\
         pages:\n  - route: /\n    title: Home\n    sections:\n      - id: hero\n        kind: hero\n"
    );
    let spec = parse_spec_text(&spec_text).expect("valid AppSpec");
    save_spec(&registry, ID, &dest, spec, "r0").expect("save spec");

    // Chosen at run time outside this host's ephemeral window; a hardcoded
    // base inside `ip_local_port_range` loses a port to any unrelated
    // outbound connection on the machine.
    std::env::set_var(
        "FORGE_STUDIO_PORT_RANGE_START",
        shared_port_base(SLOT).to_string(),
    );
    std::env::set_var("FORGE_STUDIO_STARTUP_TIMEOUT_SECS", "90");
    let runner = Box::new(ProcessRunner::react_web());
    let (session, live) = start_preview(&registry, &dest, runner).expect("preview reaches ready");
    assert_eq!(session.preview.state, PreviewState::Ready);
    let port = session.preview.port.expect("reserved port");

    // The dev server serves the app shell over HTTP.
    let response = http_get(port).expect("HTTP response from the preview");
    assert!(
        response.starts_with("HTTP/1.1 200"),
        "unexpected status line: {}",
        response.lines().next().unwrap_or("")
    );
    assert!(
        response.contains("id=\"root\""),
        "served body is missing the React mount point"
    );

    // A real browser engine renders the mounted React component.
    run_browser_smoke(port);

    // Stop and confirm the whole tree died and the port was released.
    stop_preview(&registry, &dest, Some(live)).expect("stop preview");
    assert!(
        wait_port_free(port, Duration::from_secs(5)),
        "reserved port {port} was not released after stop"
    );
}
