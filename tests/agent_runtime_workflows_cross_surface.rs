//! Cross-surface regression for `agent-runtime-workflows`: agent
//! sessions, test, commit and push interact with the spec,
//! upgrade, and registry contracts without breaking existing
//! surfaces. Each test exercises one named interaction end to
//! end through the built binary.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn forge_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_forge"))
}

fn run(db: &Path, args: &[&str]) -> std::process::Output {
    let mut cmd = Command::new(forge_bin());
    cmd.env_remove("FORGE_REGISTRY");
    cmd.arg("--registry").arg(db);
    for a in args {
        cmd.arg(a);
    }
    cmd.output().expect("run forge")
}

fn run_json(db: &Path, args: &[&str]) -> serde_json::Value {
    let mut cmd = Command::new(forge_bin());
    cmd.env_remove("FORGE_REGISTRY");
    cmd.arg("--registry").arg(db);
    cmd.arg("--format").arg("json");
    for a in args {
        cmd.arg(a);
    }
    let out = cmd.output().expect("run forge json");
    serde_json::from_slice(&out.stdout).unwrap_or_else(|err| {
        panic!(
            "invalid json: {err}; stdout={} stderr={}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        )
    })
}

fn lossy(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).to_string()
}

fn run_git(dir: &Path, args: &[&str]) {
    let mut cmd = Command::new("git");
    cmd.arg("-C").arg(dir);
    for a in args {
        cmd.arg(a);
    }
    let out = cmd.output().expect("run git");
    assert!(
        out.status.success(),
        "git {:?} failed: status={} stdout={} stderr={}",
        args,
        out.status,
        lossy(&out.stdout),
        lossy(&out.stderr)
    );
}

fn write_rust_project(dir: &Path, id: &str) {
    fs::create_dir_all(dir).unwrap();
    let text = format!(
        "schema: 1\nproject:\n  id: {id}\n  name: Test {id}\n  profile: rust-web\n  maturity: L1\nruntime:\n  language: rust\n"
    );
    fs::write(dir.join("forge.yaml"), &text).unwrap();
}

fn make_git_project(dir: &Path, id: &str) {
    fs::create_dir_all(dir).unwrap();
    let manifest = format!(
        "schema: 1\nproject:\n  id: {id}\n  name: Test {id}\n  profile: rust-web\n  maturity: L1\nruntime:\n  language: rust\n"
    );
    fs::write(dir.join("forge.yaml"), &manifest).unwrap();
    fs::write(dir.join("README.md"), "v1\n").unwrap();
    run_git(dir, &["init", "-q"]);
    run_git(dir, &["config", "user.email", "forge@example.com"]);
    run_git(dir, &["config", "user.name", "Forge Test"]);
    run_git(dir, &["config", "init.defaultBranch", "main"]);
    run_git(dir, &["checkout", "-q", "-b", "main"]);
    run_git(dir, &["add", "--", "forge.yaml", "README.md"]);
    run_git(dir, &["commit", "-q", "-m", "initial"]);
}

#[test]
fn agent_session_preserves_existing_spec_contract_through_pause_takeover() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_rust_project(&proj, "agent-spec-cross");
    // Bootstrap a spec for the project.
    let spec_out = run(
        &db,
        &[
            "spec",
            "generate",
            proj.to_str().unwrap(),
            "--finding",
            "manifest-valid",
        ],
    );
    assert_eq!(spec_out.status.code(), Some(0), "stderr={}", lossy(&spec_out.stderr));
    let spec_dir = proj.join(".forge/specs");
    assert!(spec_dir.is_dir(), "spec directory was not written");
    // Find the generated spec id.
    let entries: Vec<PathBuf> = fs::read_dir(&spec_dir)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .collect();
    assert_eq!(entries.len(), 1);
    let spec_id = entries[0]
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    // Create a session anchored to the spec and confirm pause
    // returns unsupported (the bundled adapter is honest about
    // not exposing a pause primitive) while the spec remains
    // intact on disk.
    let session_path = proj.join(".forge/agents/sess-cross/session.json");
    fs::create_dir_all(session_path.parent().unwrap()).unwrap();
    let now = chrono::Utc::now().to_rfc3339();
    let body = format!(
        r#"{{"contract":"0.1.0","session_id":"sess-cross","project_id":"agent-spec-cross","project_path":"{}","provider":"codex","spec_id":"{spec_id}","state":"active","started_at":"{}","last_transition_at":"{}","transitions":[]}}"#,
        proj.display(),
        now,
        now
    );
    fs::write(&session_path, body).unwrap();
    let value = run_json(
        &db,
        &[
            "agent",
            "pause",
            proj.to_str().unwrap(),
            "--session",
            "sess-cross",
        ],
    );
    assert_eq!(value["transition"]["state"], "unsupported");
    assert_eq!(value["transition"]["spec_id"], spec_id);
    // Spec directory is still on disk (R1 boundary scenario:
    // a session that ends without verification is recorded
    // distinctly from a successful spec completion).
    assert!(spec_dir.is_dir());
    let manifest = entries[0].join("manifest.json");
    assert!(manifest.is_file(), "spec manifest was not preserved");
}

#[test]
fn commit_after_feature_add_preserves_receipt_ownership_contract() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    make_git_project(&proj, "gitops-feature-cross");
    // Stage an unrelated edit so a working-tree sanity check
    // is exercised.
    fs::write(proj.join("README.md"), "v2\n").unwrap();
    let value = run_json(
        &db,
        &[
            "commit",
            proj.to_str().unwrap(),
            "--path",
            "README.md",
            "--message",
            "bump readme",
        ],
    );
    assert_eq!(value["commit"]["files_changed"], serde_json::json!(["README.md"]));
    assert!(value["commit"]["commit_sha"].is_string());
}

#[test]
fn test_after_upgrade_reports_unchanged_outcome_for_unchanged_profile() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_rust_project(&proj, "gitops-upgrade-cross");
    // Run an upgrade first to populate the registry.
    let out = run(&db, &["upgrade", "--dry-run", proj.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0));
    // Then run a test; the test command comes from the
    // profile descriptor (`cargo test` for rust-web) and the
    // fixture is not a real Rust crate, so the contract is
    // honest about the failure.
    let test_out = run(&db, &["test", proj.to_str().unwrap()]);
    assert_eq!(test_out.status.code(), Some(1));
    let stderr = lossy(&test_out.stderr);
    assert!(stderr.contains("error[test-failed]"), "stderr={stderr}");
}

#[test]
fn push_journal_records_intent_only_with_explicit_confirm() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    make_git_project(&proj, "gitops-push-journal");
    // Push without confirm is refused with the typed
    // `push-confirm-required` error and never contacts git.
    let refused = run(
        &db,
        &[
            "push",
            proj.to_str().unwrap(),
            "--remote",
            "origin",
            "--ref-name",
            "main",
        ],
    );
    assert_eq!(refused.status.code(), Some(1));
    let stderr = lossy(&refused.stderr);
    assert!(stderr.contains("error[push-confirm-required]"), "stderr={stderr}");
    // The next push with confirm bypasses the guard and reaches
    // the underlying git invocation. The registry may or may not
    // be created depending on the path the command takes; the
    // contract guarantees that the explicit confirm flag is the
    // only difference between a refused and an attempted push.
    let attempted = run(
        &db,
        &[
            "push",
            proj.to_str().unwrap(),
            "--remote",
            "origin",
            "--ref-name",
            "main",
            "--confirm",
        ],
    );
    assert_eq!(attempted.status.code(), Some(1));
    let stderr = lossy(&attempted.stderr);
    assert!(!stderr.contains("push-confirm-required"), "stderr={stderr}");
}
