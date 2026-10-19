//! End-to-end browser oracle for the assurance contract catalog and Gate profiles.
//!
//! Starts real Forge API and web binaries in a temporary workspace, then
//! drives the authenticated workbench in Chromium. If Playwright or Chromium
//! is unavailable, the harness reports UNVERIFIED instead of a pass.

#[path = "support/listener.rs"]
mod listener;

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const EMAIL: &str = "operator@example.test";
const PASSWORD: &str = "a-long-test-password";
const PROJECT: &str = "gate-profile-browser";

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

fn copy_dir(source: &Path, target: &Path) {
    std::fs::create_dir_all(target).expect("create web root");
    for entry in std::fs::read_dir(source).expect("read frontend") {
        let entry = entry.expect("frontend entry");
        let target = target.join(entry.file_name());
        if entry.file_type().expect("file type").is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).expect("copy frontend asset");
        }
    }
}

fn git(project: &Path, args: &[&str]) {
    let output = Command::new("git")
        .arg("-C")
        .arg(project)
        .args(args)
        .output()
        .expect("run git");
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn assurance_contracts_and_workbench_gate_profile_run_in_chromium() {
    if !node_available() {
        eprintln!("UNVERIFIED: node unavailable; Gate profile browser flow not run");
        return;
    }
    let dir = tempfile::tempdir().expect("tempdir");
    let db = dir.path().join("registry.db");
    forge::identity::global::setup(&db, EMAIL, PASSWORD).expect("admin setup");

    let project = dir.path().join("project");
    std::fs::create_dir_all(&project).expect("project directory");
    std::fs::write(
        project.join("forge.yaml"),
        format!(
            "schema: 1\nproject:\n  id: {PROJECT}\n  name: Browser Gate profile\n  profile: rust-web\n  maturity: L1\nruntime:\n  language: rust\n"
        ),
    )
    .expect("manifest");
    std::fs::write(project.join("README.md"), "Gate profile browser fixture\n").expect("readme");
    git(&project, &["init", "-q"]);
    git(&project, &["config", "user.email", "forge@example.test"]);
    git(&project, &["config", "user.name", "Forge Test"]);
    git(&project, &["config", "init.defaultBranch", "main"]);
    git(&project, &["checkout", "-q", "-b", "main"]);
    git(&project, &["add", "-A"]);
    git(&project, &["commit", "-q", "-m", "initial"]);
    let mut registry = forge::registry::Registry::open(&db).expect("registry");
    registry.register(&project, None).expect("register project");

    let api_claim = listener::claim(listener::suite::FORGE_WEB_GATE_PROFILE_BROWSER);
    let web_claim = listener::claim(listener::suite::FORGE_WEB_GATE_PROFILE_BROWSER);
    let web_origin = format!("http://127.0.0.1:{}", web_claim.port());
    let web_root = dir.path().join("web-root");
    copy_dir(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("frontend"),
        &web_root,
    );
    std::fs::write(
        web_root.join("config.js"),
        format!(
            "window.FORGE_API_BASE = \"http://127.0.0.1:{}\";\n",
            api_claim.port()
        ),
    )
    .expect("API origin");

    let api = api_claim.spawn("api", |port| {
        let mut command = Command::new(forge_bin());
        command
            .arg("--registry")
            .arg(&db)
            .env("FORGE_FRONTEND_ORIGIN", &web_origin)
            .arg("api")
            .arg("serve")
            .arg("--bind")
            .arg("127.0.0.1")
            .arg("--port")
            .arg(port.to_string())
            .stdin(Stdio::null())
            .stdout(Stdio::null());
        command
    });
    let web = web_claim.spawn("web", |port| {
        let mut command = Command::new(forge_bin());
        command
            .arg("web")
            .arg("serve")
            .arg("--bind")
            .arg("127.0.0.1")
            .arg("--port")
            .arg(port.to_string())
            .arg("--root")
            .arg(&web_root)
            .stdin(Stdio::null())
            .stdout(Stdio::null());
        command
    });

    let script = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/browser/gate-profile-workbench-check.mjs");
    let output = Command::new("node")
        .arg(script)
        .arg(&web_origin)
        .arg(EMAIL)
        .arg(PASSWORD)
        .arg(PROJECT)
        .output()
        .expect("run browser harness");
    drop(web);
    drop(api);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    match output.status.code() {
        Some(0) => {
            assert!(stdout.contains("VERIFIED"), "{stdout} {stderr}");
            eprintln!("{stdout}");
        }
        Some(2) => eprintln!("UNVERIFIED: browser harness unavailable: {stderr}"),
        _ => panic!("Gate profile browser flow failed: {stdout} {stderr}"),
    }
}
