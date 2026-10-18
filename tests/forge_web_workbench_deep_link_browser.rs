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

#[path = "support/listener.rs"]
mod listener;

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use forge::ci_evidence::{HostedCiEvidenceEnvelope, HostedCiRun};
use forge::gate::{GateAggregate, GateCheck, GateCheckState, GateEvidence};
use forge::hosted_ci::{bind_repository_id, ingest, GitHubActionsClaims, IngestOutcome};

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

fn init_hosted_repo(project: &Path) -> String {
    let run = |args: &[&str]| {
        let output = Command::new("git")
            .arg("-C")
            .arg(project)
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    };
    run(&["init", "-q"]);
    run(&["config", "user.email", "browser@example.test"]);
    run(&["config", "user.name", "Forge Browser Test"]);
    run(&[
        "remote",
        "add",
        "origin",
        "https://github.com/owner/repository.git",
    ]);
    run(&["add", "forge.yaml"]);
    run(&["commit", "-qm", "fixture"]);
    let output = Command::new("git")
        .arg("-C")
        .arg(project)
        .args(["rev-parse", "HEAD"])
        .output()
        .unwrap();
    assert!(output.status.success());
    String::from_utf8(output.stdout).unwrap().trim().to_string()
}

fn seed_hosted_observation(db: &Path, project_id: &str) {
    bind_repository_id(db, project_id, "12345").expect("bind hosted GitHub repository");
    let sha = "a".repeat(40);
    let envelope = HostedCiEvidenceEnvelope {
        contract: "forge-hosted-ci-evidence/1.0".into(),
        project_id: project_id.into(),
        repository: "owner/repository".into(),
        run: HostedCiRun {
            provider: "github-actions".into(),
            workflow: "owner/repository/.github/workflows/forge-gate.yml@refs/heads/main".into(),
            run_id: "123456789".into(),
            run_attempt: 1,
            job: "upload-hosted-evidence".into(),
            url: "https://github.com/owner/repository/actions/runs/123456789".into(),
        },
        source_revision: sha.clone(),
        gate: GateEvidence {
            contract: forge::gate::GATE_CONTRACT_VERSION.into(),
            project_id: project_id.into(),
            runtime: "forge".into(),
            runtime_version: Some("0.1.0".into()),
            revision: Some(sha.clone()),
            manifest_digest: None,
            rule_pack_version: None,
            aggregate: GateAggregate::Passed,
            checks: vec![GateCheck {
                id: "baseline".into(),
                state: GateCheckState::Pass,
                note: None,
            }],
            observed_at: "2026-10-18T10:00:00Z".into(),
            dry_run: false,
            note: None,
        },
        observed_at: "2026-10-18T10:01:00Z".into(),
    };
    let claims = GitHubActionsClaims {
        repository: "owner/repository".into(),
        repository_id: "12345".into(),
        run_id: "123456789".into(),
        run_attempt: "1".into(),
        workflow_ref: envelope.run.workflow.clone(),
        sha,
        event_name: "push".into(),
        git_ref: "refs/heads/main".into(),
    };
    assert!(matches!(
        ingest(db, envelope, &claims).unwrap(),
        IngestOutcome::Inserted(_)
    ));
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
    init_hosted_repo(&root.join(BETA));
    let mut registry = forge::registry::Registry::open(&db).expect("open registry");
    registry
        .register(&root.join(ALPHA), None)
        .expect("register alpha");
    registry
        .register(&root.join(BETA), None)
        .expect("register beta");
    seed_hosted_observation(&db, BETA);

    // Both listeners are claimed before either one starts: the API child's
    // CORS origin names the web port and the web root's `config.js` names the
    // API port, and `support/listener.rs` holds each claim until that server
    // owns it — so neither port can be taken in the gap between them.
    let api_claim = listener::claim(listener::suite::FORGE_WEB_WORKBENCH_DEEP_LINK_BROWSER);
    let web_claim = listener::claim(listener::suite::FORGE_WEB_WORKBENCH_DEEP_LINK_BROWSER);
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
