//! Contract test for real, deep-linkable dashboard navigation routes
//! (`forge-web-navigation-routing`).
//!
//! Drives the real `forge web serve` binary on an ephemeral loopback port
//! against a throwaway copy of `frontend/`:
//!
//! - every dashboard path (`/projects`, `/workbench`, `/management`,
//!   `/portfolio`, `/delivery`) plus `/index.html` returns 200 with
//!   `text/html` and the exact application-shell bytes, including with a
//!   `?project=` query string (the fleet "Manage" deep link survives reload);
//! - `/` still serves `login.html`;
//! - the server stays an exact allowlist: an unknown path and traversal
//!   attempts still 404 and never leak a file body;
//! - the sidebar carries the real paths and no `#fragment` navigation;
//! - every fleet Manage link carries its project identity in the URL and the
//!   project-scoped management view boots that project (unknown/empty ids
//!   select nothing, plain /management stays the global bulk view).

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
fn dashboard_routes_keep_serving_the_shell_with_a_query_string() {
    // The fleet "Manage" deep link (`/management?project=<id>`) must survive
    // a reload: the web listener strips the query before its exact-path
    // allowlist match and returns the same application shell. An unknown
    // path with a query string must still 404.
    let tmp = TempDir::new().expect("tempdir");
    let root = web_root(&tmp);
    let shell = std::fs::read(root.join("index.html")).expect("read shell");

    let port = free_port();
    let _server = start_web(&root, port);

    let (status, content_type, body) = http_get(port, "/management?project=deep-link-alpha");
    assert_eq!(status, 200, "GET /management?project= must be served");
    assert_eq!(content_type, "text/html; charset=utf-8");
    assert_eq!(
        body, shell,
        "GET /management?project= must return the shell bytes"
    );

    let (status, _, _) = http_get(port, "/does-not-exist?project=deep-link-alpha");
    assert_eq!(
        status, 404,
        "GET /does-not-exist?project= must not be served"
    );
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

#[test]
fn manage_links_carry_the_project_and_the_destination_boots_it() {
    // Regression test: clicking a fleet row's "Manage" action must navigate
    // to the management view with that project's identity in the URL, and
    // the destination must lead with the project-scoped card for that id
    // (bulk hidden) — never the global bulk table as the primary content.
    // Plain /management keeps the bulk view; unknown or empty ids select
    // nothing. Real path routing only — no `#fragment` navigation.
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let app = std::fs::read_to_string(root.join("frontend/app.js")).expect("app.js");
    let html = std::fs::read_to_string(root.join("frontend/index.html")).expect("index.html");

    // Every per-row Manage entry point carries the project identity.
    assert!(
        app.contains("/management?project="),
        "fleet Manage links must carry `?project=`"
    );
    assert!(
        app.contains("encodeURIComponent(project.identity)"),
        "the carried identity must be the row's project identity"
    );

    // SPA navigation preserves the query string instead of dropping it.
    assert!(
        app.contains("url.pathname + url.search"),
        "the click interceptor must preserve the query string"
    );
    assert!(
        app.contains("window.location.search"),
        "route changes must account for the query string"
    );

    // The destination reads the parameter on every route render (boot,
    // reload, back/forward) and reconciles the scoped card with it.
    for token in [
        "applyManagementProjectParam",
        "managementProjectParam",
        "mgmtHasProjectParam",
        "mgmtScoped",
        "mgmt-project-preview",
        "mgmtScopedPreview",
        "mgmtScopedRun",
        "URLSearchParams",
        "ws.discovered",
    ] {
        assert!(app.contains(token), "app.js must implement `{token}`");
    }

    // With `?project=` present the scoped card leads and the bulk table
    // hides; without it the bulk view is exact.
    for token in ["id=\"mgmt-project\"", "id=\"ws-bulk\""] {
        assert!(html.contains(token), "index.html must declare `{token}`");
    }
    assert!(
        app.contains("mgmt-project") && app.contains("ws-bulk"),
        "app.js must toggle the scoped card against the bulk table"
    );

    // Unknown and empty ids are handled safely: nothing is ticked and the
    // operator is told so — the wrong project is never loaded.
    assert!(
        app.contains("nothing was selected"),
        "an unmatched project id must select nothing and say so"
    );
    assert!(
        !app.contains("href = \"#"),
        "no fragment navigation may remain in app.js"
    );
}

#[test]
fn workbench_and_login_links_carry_the_project() {
    // Regression test: a managed fleet row's "Open" action must navigate to
    // the workbench with that project's identity in the URL; the workbench
    // must read it on every route render (boot, reload, back/forward) once
    // the fleet has populated its selector; a registered id on the
    // management view must redirect to the workbench; and the login
    // round-trip must carry the deep link back after sign-in. Real path
    // routing only — no `#fragment` navigation, no off-origin `next`.
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let app = std::fs::read_to_string(root.join("frontend/app.js")).expect("app.js");

    // The managed row action carries the project identity to the workbench.
    assert!(
        app.contains("/workbench?project="),
        "fleet Open actions must carry `?project=`"
    );

    // The workbench destination reconciles the parameter with its selector.
    for token in [
        "applyWorkbenchProjectParam",
        "workbenchProjectParam",
        "wbAutoParam",
        "wbFleetReady",
    ] {
        assert!(app.contains(token), "app.js must implement `{token}`");
    }

    // Unknown workbench ids load nothing and say so; a bare workbench URL
    // never auto-loads.
    assert!(
        app.contains("nothing was loaded"),
        "an unmatched workbench id must load nothing and say so"
    );

    // A registered workspace candidate redirects to the managing view.
    assert!(
        app.contains("match.state === \"registered\""),
        "a registered management param must redirect to the workbench"
    );

    // The login handoff carries the deep link and validates it as a
    // same-origin route before returning to it.
    for token in [
        "loginNextTarget",
        "login.html?next=",
        "isRoutePath(url.pathname)",
    ] {
        assert!(app.contains(token), "app.js must implement `{token}`");
    }
}

#[test]
fn slice_four_layout_navigation_tokens() {
    // Static contract for `portal-layout-navigation` (slice 4): unknown
    // paths render an honest empty state, filter/search state survives
    // navigation with `?project=` staying URL-owned, breakpoints cover
    // 375/768/1024/1440 plus short landscape, the small-phone sidebar
    // keeps every destination reachable without page scroll, and a named
    // z-index scale orders the layers.
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let app = std::fs::read_to_string(root.join("frontend/app.js")).expect("app.js");
    let html = std::fs::read_to_string(root.join("frontend/index.html")).expect("index.html");
    let css = std::fs::read_to_string(root.join("frontend/styles.css")).expect("styles.css");

    // Unknown-route fallback: an honest section, cleared nav state, and a
    // named crumb — never a silent fleet render.
    assert!(
        html.contains("id=\"view-unknown\""),
        "index.html must declare the unknown-route section"
    );
    let unknown = html
        .split_once("id=\"view-unknown\"")
        .and_then(|(_, rest)| rest.split_once("</nav>"))
        .map(|(nav, _)| nav)
        .expect("unknown nav");
    for route in NAV_ROUTES {
        assert!(
            unknown.contains(route),
            "the unknown state must link the real destination {route}"
        );
    }
    for token in [
        "view-unknown",
        "Page not found",
        "VIEW_CRUMBS.unknown",
        "unknown.hidden = known",
    ] {
        assert!(app.contains(token), "app.js must implement `{token}`");
    }
    assert!(
        !app.contains("VIEW_CRUMBS.projects;") || app.contains("VIEW_CRUMBS.unknown"),
        "the crumb fallback must name the miss, not the fleet"
    );

    // Filter/search preservation with the URL owning `?project=`.
    for token in [
        "forge.filter-state.v1",
        "FILTER_STATE_IDS",
        "persistFilterState",
        "restoreFilterState",
        "renderProjectsFromInputs",
        "initFilterStatePersistence",
        "project-search",
        "source-filter",
        "filter-language",
        "filter-tag",
        "sessionStorage",
    ] {
        assert!(app.contains(token), "app.js must implement `{token}`");
    }

    // Breakpoint scale: 1024 + 375 + short landscape join the existing
    // 850/560 rules; desktop above 1024px keeps its values.
    for token in [
        "max-width:1024px",
        "max-width:375px",
        "orientation:landscape",
        "max-height:500px",
    ] {
        assert!(css.contains(token), "styles.css must declare `{token}`");
    }

    // Small-phone sidebar: the nav scrolls in-row (no page-level forced
    // scroll) and every destination stays on one reachable line.
    for token in [
        ".sidebar nav{display:flex",
        "overflow-x:auto",
        "white-space:nowrap",
        "unknown-nav",
    ] {
        assert!(css.contains(token), "styles.css must implement `{token}`");
    }

    // Named z-index scale with the skip link topmost.
    for token in [
        "--z-sticky:10",
        "--z-nav:30",
        "--z-dropdown:40",
        "--z-banner:45",
        "--z-overlay:50",
        "--z-skip:60",
        "z-index:var(--z-skip)",
        "z-index:var(--z-sticky)",
        "z-index:var(--z-nav)",
    ] {
        assert!(css.contains(token), "styles.css must implement `{token}`");
    }
}
