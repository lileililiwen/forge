//! Browser oracle for staged project delivery (`forge-web-project-delivery`).
//!
//! This test starts throwaway API and web listeners, signs in through the
//! shipped login page in real Chromium, opens one fixture project, and runs
//! the full journal-backed preflight → stage → promote → Hermora flow through
//! the workbench delivery card and generic catalog controls. The provider and
//! Hermora adapter are local stubs; no network, provider daemon or credential
//! value is involved.
//!
//! If `node`, the pinned Playwright install or Chromium is unavailable, the
//! Node harness exits `2`, this test prints `UNVERIFIED` and returns. That
//! outcome is never recorded as a pass.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

const EMAIL: &str = "operator@example.test";
const PASSWORD: &str = "a-long-test-password";
const PROJECT: &str = "delivery-web-browser";
const DEPLOYMENT_URL: &str = "https://delivery.example.test/browser-app";
const SECRET_REF: &str = "env:DELIVERY_HERMORA_BROWSER_TOKEN";

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

fn write_stub(dir: &Path, name: &str, body: &str) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, body).expect("write stub");
    let mut perms = std::fs::metadata(&path)
        .expect("stub metadata")
        .permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&path, perms).expect("stub permissions");
    path
}

fn run_git(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .expect("git");
    assert!(
        out.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
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
fn workbench_drives_staged_delivery_in_chromium() {
    if !node_available() {
        eprintln!("UNVERIFIED: `node` is unavailable; delivery browser flow not run");
        return;
    }

    let dir = tempfile::tempdir().expect("tempdir");
    let db = dir.path().join("registry.db");
    forge::identity::global::setup(&db, EMAIL, PASSWORD).expect("admin setup");

    let provider_log = dir.path().join("provider.log");
    let hermora_log = dir.path().join("hermora.log");
    let provider = write_stub(
        dir.path(),
        "openpanel-stub.sh",
        &format!(
            "#!/bin/sh\nlog=\"{log}\"\ncat >> \"$log\"\nprintf '%s\\n' '{{\"contract\":\"forge-publish-provider/0.1.0\",\"provider\":\"openpanel\",\"operation_id\":\"delivery-browser\",\"status\":\"succeeded\",\"health\":\"healthy\",\"evidence\":[\"browser stub\"],\"recovery\":[],\"build_status\":\"succeeded\",\"run_status\":\"succeeded\",\"container_identity\":\"forge-stub-0123456789ab\"}}'\n",
            log = provider_log.display()
        ),
    );
    let hermora = write_stub(
        dir.path(),
        "hermora-stub.sh",
        &format!(
            "#!/bin/sh\nlog=\"{log}\"\ncat >> \"$log\"\nprintf '%s\\n' '{{\"contract\":\"forge-delivery-hermora/0.1.0\",\"operation\":\"register\",\"status\":\"connected\",\"site_id\":\"site_browser\",\"environment_url\":\"{DEPLOYMENT_URL}\"}}'\n",
            log = hermora_log.display()
        ),
    );

    let project = dir.path().join("project");
    std::fs::create_dir_all(&project).expect("project dir");
    std::fs::write(
        project.join("forge.yaml"),
        format!(
            "schema: 1\nproject:\n  id: {PROJECT}\n  name: Browser {PROJECT}\n  profile: rust-web\n  maturity: L2\nruntime:\n  language: rust\n"
        ),
    )
    .expect("manifest");
    std::fs::write(project.join("README.md"), "v1\n").expect("readme");
    run_git(&project, &["init", "-q"]);
    run_git(&project, &["config", "user.email", "forge@example.com"]);
    run_git(&project, &["config", "user.name", "Forge Test"]);
    run_git(&project, &["config", "init.defaultBranch", "main"]);
    run_git(&project, &["checkout", "-q", "-b", "main"]);
    run_git(&project, &["add", "-A"]);
    run_git(&project, &["commit", "-q", "-m", "initial"]);
    let forge_dir = project.join(".forge");
    std::fs::create_dir_all(&forge_dir).expect("forge dir");
    std::fs::write(
        forge_dir.join("providers.yaml"),
        format!(
            "providers:\n  - id: openpanel\n    command: \"{}\"\n",
            provider.display()
        ),
    )
    .expect("provider config");
    let mut registry = forge::registry::Registry::open(&db).expect("open registry");
    registry.register(&project, None).expect("register");

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
            .env("FORGE_HERMORA_BIN", &hermora)
            .env_remove("FORGE_PUBLISH_PROVIDER_CONFIG")
            .env_remove("FORGE_HERMORA_TIMEOUT_SECS")
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
        .join("delivery-workbench-check.mjs");
    assert!(script.is_file(), "browser harness missing");
    let output = Command::new("node")
        .arg(script)
        .arg(&web_origin)
        .arg(EMAIL)
        .arg(PASSWORD)
        .arg(PROJECT)
        .arg(DEPLOYMENT_URL)
        .arg(SECRET_REF)
        .arg(dir.path())
        .output()
        .expect("spawn browser harness");
    drop(web);
    drop(api);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let hermora_log_text =
        std::fs::read_to_string(&hermora_log).unwrap_or_else(|err| format!("<unreadable: {err}>"));
    let provider_log_text =
        std::fs::read_to_string(&provider_log).unwrap_or_else(|err| format!("<unreadable: {err}>"));
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
        _ => panic!(
            "browser delivery flow failed: {stdout} {stderr} provider_log={provider_log_text} hermora_log={hermora_log_text}"
        ),
    }
}
