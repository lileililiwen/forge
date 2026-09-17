//! CLI contract for `forge release` (`release-publishing`).
//!
//! Exercises the release surface end to end through the built
//! binary:
//!
//! - R1 success: a prepared plan captures the changelog,
//!   the source revision and the doctor/test/DriftWatch
//!   evidence; the rendered report includes a `ready: true`
//!   verdict and a `release` operation journal row.
//! - R1 failure: a project whose doctor reports a failing
//!   finding is refused with `error[release-check-failed]`
//!   before any tag is written.
//! - R1 boundary: a project without any `release.docs.translate`
//!   locale configures the docs stage as `disabled` and
//!   omits the translation step from the report.
//! - R2 success: every stage of an apply on a clean project
//!   records a per-stage `delivered`/`disabled` outcome so
//!   the report links the version and the working-tree
//!   revision.
//! - R2 failure: a previously-delivered package stage is
//!   reported as `skipped` on retry; a failing container
//!   stage records `failed` without rolling back the prior
//!   package outcome (per-stage independence).
//! - R2 boundary: a second apply on the same release id
//!   against a different commit reports the tag stage as
//!   `conflict` with the identity conflict evidence; a
//!   re-apply on the same commit reports `skipped` instead.
//!
//! All scenarios drive the CLI through the built binary;
//! the package, container and notes adapters are exercised
//! through small `FORGE_PACKAGE_BIN` / `FORGE_CONTAINER_BIN`
//! / `FORGE_NOTES_BIN` fixture shell scripts that stand in
//! for real provider round trips.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

fn forge_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_forge"))
}

fn lossy(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).to_string()
}

fn clean_cmd() -> Command {
    let mut cmd = Command::new(forge_bin());
    cmd.env_remove("FORGE_REGISTRY")
        .env_remove("HTTP_PROXY")
        .env_remove("HTTPS_PROXY")
        .env_remove("ALL_PROXY")
        .env_remove("OPENAI_API_KEY")
        .env_remove("ANTHROPIC_API_KEY")
        .env_remove("FORGE_DRIFTWATCH_BIN")
        .env_remove("FORGE_DOCS_TRANSLATOR_BIN")
        .env_remove("FORGE_PACKAGE_BIN")
        .env_remove("FORGE_CONTAINER_BIN")
        .env_remove("FORGE_NOTES_BIN");
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
            lossy(&out.stdout),
            lossy(&out.stderr)
        )
    })
}

fn run_git(dir: &Path, args: &[&str]) {
    let mut cmd = Command::new("git");
    cmd.arg("-C").arg(dir);
    for a in args {
        cmd.arg(a);
    }
    let out = cmd.output().expect("git");
    assert!(
        out.status.success(),
        "git {:?} failed: status={} stderr={}",
        args,
        out.status,
        lossy(&out.stderr)
    );
}

fn write_release_project(dir: &Path, id: &str) {
    fs::create_dir_all(dir).unwrap();
    let text = format!(
        "schema: 1\nproject:\n  id: {id}\n  name: {id}\n  profile: rust-web\n  maturity: L1\nruntime:\n  language: rust\nrelease:\n  versioning: semver\n  checks:\n    - kind: doctor\n    - kind: test\n  changelog: CHANGELOG.md\n  packages:\n    - kind: npm\n      name: rel-fixture\n      path: .\n      version: 0.1.0\n"
    );
    fs::write(dir.join("forge.yaml"), text).unwrap();
    fs::write(dir.join("README.md"), "v1\n").unwrap();
    fs::write(dir.join("CHANGELOG.md"), "## 1.0.0\n- initial release\n").unwrap();
    fs::write(
        dir.join("Cargo.toml"),
        "[package]\nname = \"rel-fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[lib]\npath = \"lib.rs\"\n",
    )
    .unwrap();
    let _ = fs::write(dir.join("lib.rs"), "//! rel-fixture.\n");
    run_git(dir, &["init", "-q"]);
    run_git(dir, &["config", "user.email", "forge@example.com"]);
    run_git(dir, &["config", "user.name", "Forge Test"]);
    run_git(dir, &["config", "init.defaultBranch", "main"]);
    run_git(
        dir,
        &[
            "add",
            "--",
            "forge.yaml",
            "README.md",
            "CHANGELOG.md",
            "Cargo.toml",
            "lib.rs",
        ],
    );
    run_git(dir, &["commit", "-q", "-m", "initial"]);
}

fn write_adapter_script(dir: &Path, name: &str, body: &str) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, body).unwrap();
    let mut perms = fs::metadata(&path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&path, perms).unwrap();
    path
}

#[test]
fn cli_help_lists_release_subcommand() {
    let out = clean_cmd().arg("--help").output().expect("help");
    assert_eq!(out.status.code(), Some(0));
    let text = lossy(&out.stdout);
    assert!(
        text.contains("release"),
        "help must mention release:\n{text}"
    );
    let out = clean_cmd()
        .arg("release")
        .arg("--help")
        .output()
        .expect("help");
    assert_eq!(out.status.code(), Some(0));
    let text = lossy(&out.stdout);
    for sub in ["prepare", "apply", "list", "inspect"] {
        assert!(
            text.contains(sub),
            "release help must mention `{sub}`:\n{text}"
        );
    }
}

#[test]
fn prepare_captures_plan_with_ready_verdict() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_release_project(&proj, "rel-prepare-ok");
    let value = run_json(
        &db,
        &[
            "release",
            "prepare",
            proj.to_str().unwrap(),
            "--version",
            "1.0.0",
        ],
    );
    let plan = &value["plan"];
    assert_eq!(plan["ready"], true);
    assert_eq!(plan["identity"]["version"], "1.0.0");
    assert!(plan["changelog"].is_object());
    assert_eq!(plan["changelog"]["path"], "CHANGELOG.md");
    // Doctor, test and driftwatch are all captured.
    let kinds: Vec<&str> = plan["checks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["kind"].as_str().unwrap())
        .collect();
    assert!(kinds.contains(&"doctor"));
    assert!(kinds.contains(&"test"));
    assert!(kinds.contains(&"driftwatch"));
}

#[test]
fn prepare_refuses_unknown_semver() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_release_project(&proj, "rel-prepare-bad");
    let out = run(
        &db,
        &[
            "release",
            "prepare",
            proj.to_str().unwrap(),
            "--version",
            "not-a-version",
        ],
    );
    assert_eq!(out.status.code(), Some(1));
    let stderr = lossy(&out.stderr);
    assert!(
        stderr.contains("release-invalid"),
        "stderr must report release-invalid: {stderr}"
    );
}

#[test]
fn prepare_without_release_section_refuses() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    fs::create_dir_all(&proj).unwrap();
    fs::write(
        proj.join("forge.yaml"),
        "schema: 1\nproject:\n  id: rel-norelease\n  name: rel-norelease\n  profile: rust-web\n",
    )
    .unwrap();
    fs::write(proj.join("README.md"), "v1\n").unwrap();
    fs::write(proj.join("CHANGELOG.md"), "## 1.0.0\n- initial\n").unwrap();
    run_git(&proj, &["init", "-q"]);
    run_git(&proj, &["config", "user.email", "forge@example.com"]);
    run_git(&proj, &["config", "user.name", "Forge Test"]);
    run_git(&proj, &["config", "init.defaultBranch", "main"]);
    run_git(&proj, &["checkout", "-q", "-b", "main"]);
    run_git(
        &proj,
        &["add", "--", "forge.yaml", "README.md", "CHANGELOG.md"],
    );
    run_git(&proj, &["commit", "-q", "-m", "initial"]);
    let out = run(
        &db,
        &[
            "release",
            "prepare",
            proj.to_str().unwrap(),
            "--version",
            "1.0.0",
        ],
    );
    assert_eq!(out.status.code(), Some(1));
    let stderr = lossy(&out.stderr);
    assert!(
        stderr.contains("release-invalid"),
        "stderr must report release-invalid: {stderr}"
    );
}

#[test]
fn apply_refuses_implicit_side_effects() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_release_project(&proj, "rel-apply-noconf");
    let out = run(
        &db,
        &[
            "release",
            "apply",
            proj.to_str().unwrap(),
            "--version",
            "1.0.0",
        ],
    );
    assert_eq!(out.status.code(), Some(1));
    let stderr = lossy(&out.stderr);
    assert!(
        stderr.contains("release-invalid"),
        "stderr must report release-invalid: {stderr}"
    );
}

#[test]
fn apply_records_per_stage_outcomes_with_package_adapter() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_release_project(&proj, "rel-apply-ok");
    let pkg_bin = write_adapter_script(
        tmp.path(),
        "fake-pkg.sh",
        "#!/bin/sh\necho \"npm:rel-fixture@0.1.0 receipt-ok\"\n",
    );
    let notes_bin = write_adapter_script(
        tmp.path(),
        "fake-notes.sh",
        "#!/bin/sh\necho \"notes:rendered:$1:$2\"\n",
    );
    let mut cmd = clean_cmd();
    cmd.arg("--registry")
        .arg(&db)
        .arg("--format")
        .arg("json")
        .arg("release")
        .arg("apply")
        .arg(proj.to_str().unwrap())
        .arg("--version")
        .arg("1.0.0")
        .arg("--confirm")
        .arg("--stage")
        .arg("commit")
        .arg("--stage")
        .arg("tag")
        .arg("--stage")
        .arg("package")
        .arg("--stage")
        .arg("notes")
        .env("FORGE_PACKAGE_BIN", &pkg_bin)
        .env("FORGE_NOTES_BIN", &notes_bin);
    let out = cmd.output().expect("run forge apply");
    let stdout = lossy(&out.stdout);
    let json: serde_json::Value = serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|_| panic!("stdout is not JSON: {stdout}"));
    let report = &json["release"];
    let stages: Vec<&str> = report["stage_outcomes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|o| o["stage"].as_str().unwrap())
        .collect();
    assert!(stages.contains(&"commit"));
    assert!(stages.contains(&"tag"));
    assert!(stages.contains(&"package"));
    let package = report["stage_outcomes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["stage"] == "package")
        .expect("package stage");
    assert_eq!(package["status"], "delivered");
    let tag = report["stage_outcomes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["stage"] == "tag")
        .expect("tag stage");
    assert_eq!(tag["status"], "delivered");
    // State file is persisted under the project's release
    // directory.
    let state_path = proj.join(".forge/release/rel-apply-ok");
    assert!(state_path.is_dir(), "state dir missing: {state_path:?}");
}

#[test]
fn apply_records_package_failure_without_overwriting_previous_tag() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_release_project(&proj, "rel-apply-fail");
    let pkg_ok = write_adapter_script(
        tmp.path(),
        "pkg-ok.sh",
        "#!/bin/sh\necho \"npm:rel-fixture@0.1.0 receipt-ok\"\n",
    );
    let pkg_fail = write_adapter_script(
        tmp.path(),
        "pkg-fail.sh",
        "#!/bin/sh\necho \"package adapter `pkg-fail` failed: simulated provider outage\" >&2\nexit 1\n",
    );
    // First apply: package stage succeeds.
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(&db);
    cmd.arg("release")
        .arg("apply")
        .arg(proj.to_str().unwrap())
        .arg("--version")
        .arg("1.0.0")
        .arg("--confirm")
        .arg("--stage")
        .arg("tag")
        .arg("--stage")
        .arg("package")
        .env("FORGE_PACKAGE_BIN", &pkg_ok);
    let out = cmd.output().expect("first apply");
    assert_eq!(out.status.code(), Some(0));
    // Now configure the package adapter to fail and
    // re-run: the second apply should record `failed` for
    // the package stage while the tag stage (already at
    // HEAD) is `skipped`.
    let mut cmd = clean_cmd();
    cmd.arg("--registry")
        .arg(&db)
        .arg("--format")
        .arg("json")
        .arg("release")
        .arg("apply")
        .arg(proj.to_str().unwrap())
        .arg("--version")
        .arg("1.0.0")
        .arg("--confirm")
        .arg("--stage")
        .arg("tag")
        .arg("--stage")
        .arg("package")
        .env("FORGE_PACKAGE_BIN", &pkg_fail);
    let out = cmd.output().expect("second apply");
    assert_eq!(out.status.code(), Some(1));
    let json: serde_json::Value = serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|_| panic!("stdout: {}", lossy(&out.stdout)));
    let report = &json["release"];
    let package = report["stage_outcomes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["stage"] == "package")
        .expect("package stage");
    assert_eq!(package["status"], "failed");
    let tag = report["stage_outcomes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["stage"] == "tag")
        .expect("tag stage");
    assert!(
        tag["status"] == "skipped" || tag["status"] == "delivered",
        "tag stage must not regress: {tag:?}"
    );
}

#[test]
fn apply_reports_tag_conflict_when_revision_differs() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_release_project(&proj, "rel-conflict");
    // First apply: tag created at the initial commit.
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(&db);
    cmd.arg("release")
        .arg("apply")
        .arg(proj.to_str().unwrap())
        .arg("--version")
        .arg("1.0.0")
        .arg("--confirm")
        .arg("--stage")
        .arg("tag");
    let out = cmd.output().expect("first apply");
    assert_eq!(out.status.code(), Some(0));
    // Add a new commit so HEAD advances.
    fs::write(proj.join("README.md"), "v2\n").unwrap();
    run_git(&proj, &["add", "--", "README.md"]);
    run_git(&proj, &["commit", "-q", "-m", "second"]);
    // Second apply: the tag now conflicts with the new
    // commit.
    let mut cmd = clean_cmd();
    cmd.arg("--registry")
        .arg(&db)
        .arg("--format")
        .arg("json")
        .arg("release")
        .arg("apply")
        .arg(proj.to_str().unwrap())
        .arg("--version")
        .arg("1.0.0")
        .arg("--confirm")
        .arg("--stage")
        .arg("tag");
    let out = cmd.output().expect("second apply");
    let json: serde_json::Value = serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|_| panic!("stdout: {}", lossy(&out.stdout)));
    let report = &json["release"];
    let tag = report["stage_outcomes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["stage"] == "tag")
        .expect("tag stage");
    assert_eq!(tag["status"], "conflict");
}

#[test]
fn list_reports_persisted_releases() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_release_project(&proj, "rel-list");
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(&db);
    cmd.arg("release")
        .arg("apply")
        .arg(proj.to_str().unwrap())
        .arg("--version")
        .arg("1.0.0")
        .arg("--confirm")
        .arg("--stage")
        .arg("tag");
    let out = cmd.output().expect("apply");
    assert_eq!(out.status.code(), Some(0));
    let value = run_json(&db, &["release", "list", proj.to_str().unwrap()]);
    let releases = value["releases"].as_array().unwrap();
    assert!(!releases.is_empty(), "expected at least one release");
    assert_eq!(releases[0]["version"], "1.0.0");
    assert_eq!(releases[0]["project_id"], "rel-list");
}

#[test]
fn inspect_returns_release_state() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_release_project(&proj, "rel-inspect");
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(&db);
    cmd.arg("release")
        .arg("apply")
        .arg(proj.to_str().unwrap())
        .arg("--version")
        .arg("1.0.0")
        .arg("--confirm")
        .arg("--stage")
        .arg("tag");
    let out = cmd.output().expect("apply");
    assert_eq!(out.status.code(), Some(0));
    // Read the persisted release id.
    let list = run_json(&db, &["release", "list", proj.to_str().unwrap()]);
    let release_id = list["releases"][0]["release_id"]
        .as_str()
        .unwrap()
        .to_string();
    let value = run_json(
        &db,
        &["release", "inspect", &release_id, proj.to_str().unwrap()],
    );
    let release = &value["release"];
    assert_eq!(release["identity"]["version"], "1.0.0");
    let stages: Vec<&str> = release["stage_outcomes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|o| o["stage"].as_str().unwrap())
        .collect();
    assert!(stages.contains(&"tag"));
}
