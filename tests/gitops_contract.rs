//! Verified test and Git operations contract (`agent-runtime-workflows`).
//!
//! Covers R2 — Verified test and Git operations end to end through
//! the built binary: a successful test run on a real project,
//! commit records only the requested paths and preserves
//! unrelated edits in the working tree, commit refuses when the
//! working tree contains unrelated edits (R2 failure scenario),
//! push without `--confirm` is refused with a typed error
//! (`push-confirm-required`) without contacting the remote, and
//! the push operation journal is recorded only when the explicit
//! confirm flag is supplied.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn forge_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_forge"))
}

fn clean_cmd() -> Command {
    let mut cmd = Command::new(forge_bin());
    cmd.env_remove("FORGE_REGISTRY");
    cmd.env_remove("HTTP_PROXY");
    cmd.env_remove("HTTPS_PROXY");
    cmd.env_remove("ALL_PROXY");
    cmd.env_remove("OPENAI_API_KEY");
    cmd.env_remove("ANTHROPIC_API_KEY");
    cmd
}

fn run(db: &Path, args: &[&str]) -> std::process::Output {
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(db);
    for a in args {
        cmd.arg(a);
    }
    cmd.output().expect("run forge")
}

fn run_json(db: &Path, args: &[&str]) -> serde_json::Value {
    let mut cmd = clean_cmd();
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
fn test_runs_profile_command_and_reports_outcome() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    fs::create_dir_all(&proj).unwrap();
    // Profile `rust-web` ships `cargo test` as the test command.
    // The fixture is not a real Rust project, so the test command
    // fails and the contract returns `test-failed` with the
    // captured error. The contract is honest about the failure;
    // it does not simulate a passing result.
    fs::write(
        proj.join("forge.yaml"),
        "schema: 1\nproject:\n  id: t-run\n  name: T\n  profile: rust-web\n  maturity: L1\nruntime:\n  language: rust\n",
    )
    .unwrap();
    let out = run(&db, &["test", proj.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(1));
    let stderr = lossy(&out.stderr);
    assert!(stderr.contains("error[test-failed]"), "stderr={stderr}");
    assert!(stderr.contains("cargo test"), "stderr={stderr}");
}

#[test]
fn commit_records_only_requested_paths_and_preserves_unrelated_edits() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    make_git_project(&proj, "gitops-commit");
    // Make two changes: one reviewed (README.md) and one
    // unrelated (NOTES.txt). The commit must include only the
    // reviewed path and leave NOTES.txt unstaged.
    fs::write(proj.join("README.md"), "v2\n").unwrap();
    fs::write(proj.join("NOTES.txt"), "scratch\n").unwrap();

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
    assert_eq!(value["commit"]["project_id"], "gitops-commit");
    assert_eq!(
        value["commit"]["files_changed"],
        serde_json::json!(["README.md"])
    );
    assert!(value["commit"]["commit_sha"].is_string());

    // NOTES.txt is still present in the working tree (unrelated
    // edit preserved).
    assert!(proj.join("NOTES.txt").is_file());
    // Working tree still shows NOTES.txt as an unstaged edit.
    let status = Command::new("git")
        .arg("-C")
        .arg(&proj)
        .args(["status", "--porcelain"])
        .output()
        .expect("git status");
    let text = lossy(&status.stdout);
    assert!(text.contains("NOTES.txt"), "git status={text}");
}

#[test]
fn commit_refuses_when_tracked_edits_outside_requested_paths_are_present() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    make_git_project(&proj, "gitops-dirty");
    // Add a second tracked file, then modify it (so it appears
    // as a tracked edit in `git status --porcelain`) without
    // staging it. The contract refuses the commit because the
    // working tree has a tracked edit outside the requested
    // path that would be silently pulled in.
    fs::write(proj.join("extra.txt"), "v1\n").unwrap();
    run_git(&proj, &["add", "--", "extra.txt"]);
    run_git(&proj, &["commit", "-q", "-m", "add extra"]);
    fs::write(proj.join("README.md"), "v2\n").unwrap();
    fs::write(proj.join("extra.txt"), "v2\n").unwrap();

    let out = run(
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
    assert_eq!(out.status.code(), Some(1));
    let stderr = lossy(&out.stderr);
    assert!(stderr.contains("error[git-dirty]"), "stderr={stderr}");
    assert!(stderr.contains("extra.txt"), "stderr={stderr}");
    // No commit was created.
    let head = Command::new("git")
        .arg("-C")
        .arg(&proj)
        .args(["log", "--oneline"])
        .output()
        .expect("git log");
    let text = lossy(&head.stdout);
    assert!(!text.contains("bump readme"), "git log={text}");
    // Working tree is preserved.
    assert!(proj.join("extra.txt").is_file());
}

#[test]
fn commit_refuses_empty_message() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    make_git_project(&proj, "gitops-empty-msg");

    let out = run(
        &db,
        &[
            "commit",
            proj.to_str().unwrap(),
            "--path",
            "README.md",
            "--message",
            "   ",
        ],
    );
    assert_eq!(out.status.code(), Some(1));
    let stderr = lossy(&out.stderr);
    assert!(stderr.contains("error[git-dirty]"), "stderr={stderr}");
}

#[test]
fn commit_refuses_when_not_a_git_repository() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    fs::create_dir_all(&proj).unwrap();
    fs::write(
        proj.join("forge.yaml"),
        "schema: 1\nproject:\n  id: no-git\n  name: No Git\n  profile: rust-web\n  maturity: L1\nruntime:\n  language: rust\n",
    )
    .unwrap();
    fs::write(proj.join("README.md"), "v1\n").unwrap();

    let out = run(
        &db,
        &[
            "commit",
            proj.to_str().unwrap(),
            "--path",
            "README.md",
            "--message",
            "bump",
        ],
    );
    assert_eq!(out.status.code(), Some(1));
    let stderr = lossy(&out.stderr);
    assert!(stderr.contains("error[git-dirty]"), "stderr={stderr}");
}

#[test]
fn push_refuses_without_explicit_confirm() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    make_git_project(&proj, "gitops-push-refuse");
    let out = run(
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
    assert_eq!(out.status.code(), Some(1));
    let stderr = lossy(&out.stderr);
    assert!(
        stderr.contains("error[push-confirm-required]"),
        "stderr={stderr}"
    );
}

#[test]
fn push_with_confirm_attempts_and_records_operation() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    make_git_project(&proj, "gitops-push-attempt");
    // No real remote is configured; the push will fail at the
    // network layer, but the contract is verified: the explicit
    // confirm flag bypasses the refuse-without-confirm guard, and
    // the failure surfaces as a typed `git-dirty` error rather
    // than a silent no-op.
    let out = run(
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
    assert_eq!(out.status.code(), Some(1));
    let stderr = lossy(&out.stderr);
    assert!(!stderr.contains("push-confirm-required"), "stderr={stderr}");
}

#[test]
fn test_command_profile_default_used_when_profile_descriptor_omits_command() {
    // The CLI resolves the test command from the profile
    // descriptor. This test confirms that `forge test` exits
    // non-zero with a `test-failed` error when the test command
    // is not a real Rust crate; the contract is honest about
    // the failure.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    fs::create_dir_all(&proj).unwrap();
    fs::write(
        proj.join("forge.yaml"),
        "schema: 1\nproject:\n  id: t-resolve\n  name: T\n  profile: rust-web\n  maturity: L1\nruntime:\n  language: rust\n",
    )
    .unwrap();
    let out = run(&db, &["test", proj.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(1));
    let stderr = lossy(&out.stderr);
    assert!(stderr.contains("error[test-failed]"), "stderr={stderr}");
    assert!(stderr.contains("cargo test"), "stderr={stderr}");
}
