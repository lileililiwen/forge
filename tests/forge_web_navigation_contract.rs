//! Contract test for real, deep-linkable dashboard navigation routes
//! (`forge-web-navigation-routing`).
//!
//! Drives the real `forge web serve` binary on an ephemeral loopback port
//! against a throwaway copy of `frontend/`:
//!
//! - every dashboard path (`/projects`, `/workbench`, `/management`,
//!   `/portfolio`, `/delivery`) plus `/index.html` returns 200 with
//!   `text/html` and the exact application-shell bytes;
//! - `/` still serves `login.html`;
//! - the server stays an exact allowlist: an unknown path and traversal
//!   attempts still 404 and never leak a file body;
//! - the sidebar carries the real paths and no `#fragment` navigation.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use tempfile::TempDir;

const SHELL_ROUTES: [&str; 6] = [
    "/index.html",
    "/projects",
    "/workbench",
    "/management",
    "/portfolio",
    "/delivery",
];

const NAV_ROUTES: [&str; 5] = [
    "/projects",
    "/workbench",
    "/management",
    "/portfolio",
    "/delivery",
];

fn forge_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_forge"))
}

fn free_port() -> u16 {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind ephemeral");
    let port = listener.local_addr().expect("local addr").port();
    drop(listener);
    port
}

fn copy_dir(source: &Path, target: &Path) {
    std::fs::create_dir_all(target).expect("create web root");
    for entry in std::fs::read_dir(source).expect("read frontend") {
        let entry = entry.expect("frontend entry");
        let target = target.join(entry.file_name());
        let kind = entry.file_type().expect("file type");
        if kind.is_dir() {
            copy_dir(&entry.path(), &target);
        } else if kind.is_file() {
            std::fs::copy(entry.path(), target).expect("copy frontend file");
        }
    }
}

struct ChildGuard(Child);

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn wait_for_port(port: u16) {
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(10) {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    panic!("web server did not bind on 127.0.0.1:{port}");
}

/// One raw HTTP/1.1 request. The path is written literally so unknown-path
/// and traversal cases reach the server exactly as sent.
fn http_get(port: u16, path: &str) -> (u16, String, Vec<u8>) {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect web");
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(5)));
    stream
        .write_all(
            format!("GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n")
                .as_bytes(),
        )
        .expect("write request");
    let mut response = Vec::new();
    stream.read_to_end(&mut response).expect("read response");
    let split = response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .expect("response headers");
    let head = String::from_utf8_lossy(&response[..split]).to_string();
    let body = response[split + 4..].to_vec();
    let status = head
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|code| code.parse().ok())
        .unwrap_or(0);
    let content_type = head
        .lines()
        .find_map(|line| line.strip_prefix("Content-Type:"))
        .map(|value| value.trim().to_string())
        .unwrap_or_default();
    (status, content_type, body)
}

fn start_web(root: &Path, port: u16) -> ChildGuard {
    let child = Command::new(forge_bin())
        .env_remove("FORGE_REGISTRY")
        .arg("web")
        .arg("serve")
        .arg("--bind")
        .arg("127.0.0.1")
        .arg("--port")
        .arg(port.to_string())
        .arg("--root")
        .arg(root)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn web server");
    let guard = ChildGuard(child);
    wait_for_port(port);
    guard
}

fn web_root(tmp: &TempDir) -> PathBuf {
    let root = tmp.path().join("web-root");
    copy_dir(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("frontend"),
        &root,
    );
    root
}

#[test]
fn every_dashboard_route_serves_the_application_shell() {
    let tmp = TempDir::new().expect("tempdir");
    let root = web_root(&tmp);
    let shell = std::fs::read(root.join("index.html")).expect("read shell");
    let login = std::fs::read(root.join("login.html")).expect("read login");

    let port = free_port();
    let _server = start_web(&root, port);

    let (status, content_type, body) = http_get(port, "/");
    assert_eq!(status, 200, "GET / must serve the login page");
    assert_eq!(content_type, "text/html; charset=utf-8", "GET /");
    assert_eq!(body, login, "GET / must still return the login.html bytes");

    for route in SHELL_ROUTES {
        let (status, content_type, body) = http_get(port, route);
        assert_eq!(status, 200, "GET {route} must be served");
        assert_eq!(content_type, "text/html; charset=utf-8", "GET {route}");
        assert_eq!(body, shell, "GET {route} must return the shell bytes");
    }
}

#[test]
fn the_server_stays_an_exact_allowlist() {
    let tmp = TempDir::new().expect("tempdir");
    let root = web_root(&tmp);

    let port = free_port();
    let _server = start_web(&root, port);

    for path in [
        "/does-not-exist",
        "/v1/admin/session",
        "/../src/main.rs",
        "/../../etc/passwd",
        "/%2e%2e/src/main.rs",
    ] {
        let (status, _content_type, body) = http_get(port, path);
        assert_eq!(status, 404, "GET {path} must not be served");
        assert!(
            !String::from_utf8_lossy(&body).contains("fn main"),
            "GET {path} must never leak a file body"
        );
    }
}

#[test]
fn the_sidebar_uses_real_paths_and_no_fragment_navigation() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let html = std::fs::read_to_string(root.join("frontend/index.html")).expect("index.html");
    let app = std::fs::read_to_string(root.join("frontend/app.js")).expect("app.js");

    assert!(
        html.contains("<base href=\"/\">"),
        "the shell must declare a root base so assets resolve from sub-paths"
    );
    let nav = html
        .split_once("<nav")
        .and_then(|(_, rest)| rest.split_once("</nav>"))
        .map(|(nav, _)| nav)
        .expect("sidebar nav");
    assert!(
        !nav.contains("href=\"#"),
        "sidebar navigation must not use fragments: {nav}"
    );
    for route in NAV_ROUTES {
        assert!(
            nav.contains(route),
            "sidebar must link the real path {route}: {nav}"
        );
    }

    // The router owns view switching and history; no `href = "#..."` remains.
    for token in [
        "history.pushState",
        "popstate",
        "renderRoute",
        "VIEW_BY_PATH",
    ] {
        assert!(app.contains(token), "app.js must implement `{token}`");
    }
    assert!(
        !app.contains("href = \"#"),
        "no fragment navigation may remain in app.js"
    );
}
