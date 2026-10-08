//! Browser oracle for the workbench deep link
//! (`/workbench?project=<identity>`) and the login round-trip that
//! preserves per-project links.
//!
//! This test starts throwaway API and web listeners with two registered
//! fixture projects (so the fleet renders per-row Open actions and the
//! workbench selector offers them) plus one unregistered workspace sibling
//! (so the management panel can tick it), signs in through the shipped login
//! page in real Chromium, and drives the
//! `tests/browser/workbench-deep-link-check.mjs` harness: the Open action
//! URL carries the project, direct loads and reloads boot it, an unknown id
//! loads nothing, back/forward follow the URL, a registered id on the
//! management view redirects to the workbench, signed-out deep links return
//! through `login.html?next=` and boot, and a hostile `next` falls back to
//! the dashboard. No concrete host folder is involved beyond the throwaway
//! tempdir.
//!
//! If `node`, the pinned Playwright install or Chromium is unavailable, the
//! Node harness exits `2`, this test prints `UNVERIFIED` and returns. That
//! outcome is never recorded as a pass.

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

const EMAIL: &str = "operator@example.test";
const PASSWORD: &str = "a-long-test-password";
const ALPHA: &str = "wb-link-alpha";
const BETA: &str = "wb-link-beta";
const GAMMA: &str = "ws-link-gamma";

fn forge_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_forge"))
}

fn node_available() -> bool {
    Command::new("node")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
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

fn write_manifest(dir: &Path, id: &str) {
    let proj = dir.join(id);
    std::fs::create_dir_all(&proj).expect("sibling dir");
    std::fs::write(
        proj.join("forge.yaml"),
        format!(
            "schema: 1\nproject:\n  id: {id}\n  name: Browser {id}\n  profile: rust-web\n  maturity: L1\nruntime:\n  language: rust\nfeatures: {{}}\n"
        ),
    )
    .expect("manifest");
}

struct ChildGuard(Child);

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn wait_for_port(port: u16) {
    let start = std::time::Instant::now();
    while start.elapsed() < std::time::Duration::from_secs(10) {
        if std::net::TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    panic!("server did not bind on 127.0.0.1:{port}");
}

#[test]
fn fleet_open_action_boots_that_project_in_chromium() {
    if !node_available() {
        eprintln!("UNVERIFIED: `node` is unavailable; workbench deep-link browser flow not run");
        return;
    }

    let dir = tempfile::tempdir().expect("tempdir");
    let db = dir.path().join("registry.db");
    forge::identity::global::setup(&db, EMAIL, PASSWORD).expect("admin setup");

    // Two managed fixtures (fleet Open actions + workbench options) and one
    // unregistered sibling (management tick + login round-trip target), all
    // under the same throwaway workspace root.
    let root = dir.path().join("workspace");
    write_manifest(&root, ALPHA);
    write_manifest(&root, BETA);
    write_manifest(&root, GAMMA);
    let mut registry = forge::registry::Registry::open(&db).expect("open registry");
    registry
        .register(&root.join(ALPHA), None)
        .expect("register alpha");
    registry
        .register(&root.join(BETA), None)
        .expect("register beta");

    let api_port = free_port();
    let web_port = free_port();
    let web_origin = format!("http://127.0.0.1:{web_port}");
    let web_root = dir.path().join("web-root");
    copy_dir(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("frontend"),
        &web_root,
    );
    std::fs::write(
        web_root.join("config.js"),
        format!(
            "// Test copy pointing at the throwaway API listener.\nwindow.FORGE_API_BASE = \"http://127.0.0.1:{api_port}\";\n"
        ),
    )
    .expect("test API origin");

    let api = ChildGuard(
        Command::new(forge_bin())
            .arg("--registry")
            .arg(&db)
            .env("FORGE_FRONTEND_ORIGIN", &web_origin)
            .env("FORGE_ADMIN_PROJECTS_ROOT", &root)
            .env_remove("HTTP_PROXY")
            .env_remove("HTTPS_PROXY")
            .env_remove("ALL_PROXY")
            .arg("api")
            .arg("serve")
            .arg("--bind")
            .arg("127.0.0.1")
            .arg("--port")
            .arg(api_port.to_string())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn API"),
    );
    wait_for_port(api_port);
    let web = ChildGuard(
        Command::new(forge_bin())
            .arg("web")
            .arg("serve")
            .arg("--bind")
            .arg("127.0.0.1")
            .arg("--port")
            .arg(web_port.to_string())
            .arg("--root")
            .arg(&web_root)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn web server"),
    );
    wait_for_port(web_port);

    let script = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("browser")
        .join("workbench-deep-link-check.mjs");
    assert!(script.is_file(), "browser harness missing");
    let output = Command::new("node")
        .arg(script)
        .arg(&web_origin)
        .arg(EMAIL)
        .arg(PASSWORD)
        .arg(ALPHA)
        .arg(BETA)
        .arg(GAMMA)
        .output()
        .expect("spawn browser harness");
    drop(web);
    drop(api);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    match output.status.code() {
        Some(0) => {
            assert!(
                stdout.contains("VERIFIED"),
                "browser harness did not report VERIFIED: {stdout} {stderr}"
            );
            eprintln!("{stdout}");
        }
        Some(2) => {
            eprintln!("UNVERIFIED: browser harness unavailable: {stderr}");
        }
        _ => panic!("browser workbench deep-link flow failed: {stdout} {stderr}"),
    }
}
