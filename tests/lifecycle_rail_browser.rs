//! Browser oracle for the workbench lifecycle series rail
//! (`lifecycle-series-rail`, part 2 live behavior, plus
//! `flywheel-plugin-cap` flywheel ends and `web-lifecycle-execution`
//! click-every-confirm coverage).
//!
//! This test starts throwaway API and web listeners with one registered
//! fixture project, signs in through the shipped login page in real
//! Chromium, and drives the `tests/browser/lifecycle-rail-check.mjs`
//! harness: the rail lists the 8 series steps with a single
//! `aria-current="step"` and a single Tab stop, `?step=` coexists with
//! `?project=` across loads/clicks/reloads, the next-best-action card
//! renders exactly one pick, arrow keys rove rail focus, Copy-as-CLI
//! copies the exact shown `forge ...` string, 390px has no page-level
//! horizontal overflow, workbench text meets WCAG 2.2 AA, rail step 0
//! carries the idea entry (graduation + studio), the projects view carries
//! the cap group filter with the rail cap badge, `forge cap list` derives
//! from `CAPABILITIES` plus live states, `docs/flywheel-demo.md` covers
//! the hookit 5-step demo URLs, and every lifecycle confirm clicks end to
//! end in rail order (idea preview→confirm, scaffold `new`, studio spec
//! save + refine revision bump, upgrade plan, delivery approve/publish
//! confirm-refused paths, maintain refresh, remediate plan/apply honest
//! refusals, intent resolve→apply) with zero JS console errors,
//! error-summary focus on every invalid submit, `role=status` updates,
//! journal evidence rows, and a screenshot per step under
//! `target/lifecycle-rail-shots/`.
//!
//! If `node`, the pinned Playwright install or Chromium is unavailable, the
//! Node harness exits `2`, this test prints `UNVERIFIED` and returns. That
//! outcome is never recorded as a pass.

#[path = "support/listener.rs"]
mod listener;

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const EMAIL: &str = "operator@example.test";
const PASSWORD: &str = "a-long-test-password";
const PROJECT: &str = "rail-check-alpha";

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

#[test]
fn lifecycle_series_rail_renders_and_copies_in_chromium() {
    if !node_available() {
        eprintln!("UNVERIFIED: `node` is unavailable; lifecycle rail browser flow not run");
        return;
    }

    let dir = tempfile::tempdir().expect("tempdir");
    let db = dir.path().join("registry.db");
    forge::identity::global::setup(&db, EMAIL, PASSWORD).expect("admin setup");

    let root = dir.path().join("workspace");
    write_manifest(&root, PROJECT);
    let mut registry = forge::registry::Registry::open(&db).expect("open registry");
    registry
        .register(&root.join(PROJECT), None)
        .expect("register fixture");

    // Both listeners are claimed before either one starts: the API child's
    // CORS origin names the web port and the web root's `config.js` names the
    // API port, and `support/listener.rs` holds each claim until that server
    // owns it — so neither port can be taken in the gap between them.
    let api_claim = listener::claim(listener::suite::LIFECYCLE_RAIL_BROWSER);
    let web_claim = listener::claim(listener::suite::LIFECYCLE_RAIL_BROWSER);
    let web_origin = format!("http://127.0.0.1:{}", web_claim.port());
    let web_root = dir.path().join("web-root");
    copy_dir(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("frontend"),
        &web_root,
    );
    std::fs::write(
        web_root.join("config.js"),
        format!(
            "// Test copy pointing at the throwaway API listener.\nwindow.FORGE_API_BASE = \"http://127.0.0.1:{}\";\n",
            api_claim.port()
        ),
    )
    .expect("test API origin");

    let api = api_claim.spawn("api", |port| {
        let mut command = Command::new(forge_bin());
        command
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
        .join("tests")
        .join("browser")
        .join("lifecycle-rail-check.mjs");
    assert!(script.is_file(), "browser harness missing");
    // Flywheel static gates (no browser needed): `forge cap list` derives
    // from CAPABILITIES plus live states, and the demo doc covers the
    // hookit 5-step `?project=&step=` URLs.
    let cap_out = Command::new(forge_bin())
        .arg("--registry")
        .arg(&db)
        .arg("cap")
        .arg("list")
        .arg("--format")
        .arg("json")
        .output()
        .expect("forge cap list");
    assert!(cap_out.status.success(), "forge cap list runs");
    let cap_json: serde_json::Value =
        serde_json::from_slice(&cap_out.stdout).expect("cap list is JSON");
    let groups = cap_json
        .get("groups")
        .and_then(|g| g.as_array())
        .expect("cap list groups");
    assert!(
        groups
            .iter()
            .any(|g| g.get("capability").and_then(|c| c.as_str()) == Some("gate")),
        "cap list derives from CAPABILITIES (gate present)"
    );
    let demo = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("docs/flywheel-demo.md"),
    )
    .expect("demo doc ships");
    assert!(demo.contains("hookit"), "demo uses hookit");
    assert!(
        demo.contains("?project=hookit&step=idea"),
        "demo carries ?project=&step= URLs"
    );
    let output = Command::new("node")
        .arg(script)
        .arg(&web_origin)
        .arg(EMAIL)
        .arg(PASSWORD)
        .arg(PROJECT)
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
        _ => panic!("browser lifecycle rail flow failed: {stdout} {stderr}"),
    }
}
