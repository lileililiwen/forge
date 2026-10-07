//! Browser oracle for workspace onboarding (`forge-web-workspace-onboarding`).
//!
//! This test starts throwaway API and web listeners with a fixture workspace
//! root, signs in through the shipped login page in real Chromium, and drives
//! the full discover → select → preview → confirm → results → fleet journey
//! for two fixture siblings (one manifest project, one importable Cargo
//! project). No concrete host folder is involved beyond the throwaway
//! tempdir.
//!
//! If `node`, the pinned Playwright install or Chromium is unavailable, the
//! Node harness exits `2`, this test prints `UNVERIFIED` and returns. That
//! outcome is never recorded as a pass.

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

const EMAIL: &str = "operator@example.test";
const PASSWORD: &str = "a-long-test-password";
const DIR_ONE: &str = "alpha-proj";
const DIR_TWO: &str = "beta-app";
const ID_ONE: &str = "alpha-proj";
const ID_TWO: &str = "beta-app";

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
fn dashboard_onboards_workspace_siblings_in_chromium() {
    if !node_available() {
        eprintln!("UNVERIFIED: `node` is unavailable; onboarding browser flow not run");
        return;
    }

    let dir = tempfile::tempdir().expect("tempdir");
    let db = dir.path().join("registry.db");
    forge::identity::global::setup(&db, EMAIL, PASSWORD).expect("admin setup");

    let root = dir.path().join("workspace");
    let alpha = root.join(DIR_ONE);
    std::fs::create_dir_all(&alpha).expect("alpha dir");
    std::fs::write(
        alpha.join("forge.yaml"),
        format!(
            "schema: 1\nproject:\n  id: {ID_ONE}\n  name: Browser {ID_ONE}\n  profile: rust-web\n  maturity: L1\nruntime:\n  language: rust\nfeatures: {{}}\n"
        ),
    )
    .expect("manifest");
    let beta = root.join(DIR_TWO);
    std::fs::create_dir_all(beta.join("src")).expect("beta dir");
    std::fs::write(
        beta.join("Cargo.toml"),
        "[package]\nname = \"beta-app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .expect("cargo manifest");
    std::fs::write(beta.join("src").join("main.rs"), "fn main() {}\n").expect("main");

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
        .join("workspace-onboarding-check.mjs");
    assert!(script.is_file(), "browser harness missing");
    let output = Command::new("node")
        .arg(script)
        .arg(&web_origin)
        .arg(EMAIL)
        .arg(PASSWORD)
        .arg(DIR_ONE)
        .arg(DIR_TWO)
        .arg(ID_ONE)
        .arg(ID_TWO)
        .arg(dir.path())
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
        _ => panic!("browser onboarding flow failed: {stdout} {stderr}"),
    }
}
