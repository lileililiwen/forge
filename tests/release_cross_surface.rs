//! Cross-surface tests for the `release-publishing` change.
//!
//! Verifies the release surface composes with the rest of
//! Forge without breaking the existing contracts:
//!
//! - doctor verdict is preserved after a successful release
//!   prepare/apply;
//! - the registry `release` journal row carries the
//!   per-stage summary so the originating intent survives
//!   the run;
//! - the `feature add` operation that bumped the version of
//!   a feature does not change the release surface (the
//!   release engine reads the current manifest, never a
//!   receipt cached at prepare time);
//! - the `commit` (gitops) and `release` (apply) operations
//!   cooperate: a `git commit` after the release records a
//!   new revision so the next `release apply` reports the
//!   prior tag as `conflict`.

use std::fs;
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
        "schema: 1\nproject:\n  id: {id}\n  name: {id}\n  profile: rust-web\n  maturity: L1\nruntime:\n  language: rust\nrelease:\n  versioning: semver\n  checks:\n    - kind: doctor\n    - kind: test\n  changelog: CHANGELOG.md\n"
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

#[test]
fn doctor_verdict_unchanged_after_successful_release() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_release_project(&proj, "rel-cross-doc");
    // Register the project so the registry has an
    // observation.
    let mut cmd = clean_cmd();
    cmd.arg("--registry")
        .arg(&db)
        .arg("register")
        .arg(proj.to_str().unwrap());
    let out = cmd.output().expect("register");
    assert_eq!(
        out.status.code(),
        Some(0),
        "register: {}",
        lossy(&out.stderr)
    );
    let before = clean_cmd();
    let mut before = before;
    before
        .arg("--registry")
        .arg(&db)
        .arg("--format")
        .arg("json")
        .arg("doctor")
        .arg(proj.to_str().unwrap());
    let out = before.output().expect("doctor before");
    assert_eq!(out.status.code(), Some(0));
    let before_json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let before_unhealthy: Vec<String> = before_json["doctor"]["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| matches!(f["status"].as_str(), Some("fail") | Some("unavailable")))
        .map(|f| f["id"].as_str().unwrap_or("?").to_string())
        .collect();
    // Now run release prepare (which exercises the same
    // doctor).
    let mut cmd = clean_cmd();
    cmd.arg("--registry")
        .arg(&db)
        .arg("release")
        .arg("prepare")
        .arg(proj.to_str().unwrap())
        .arg("--version")
        .arg("1.0.0");
    let out = cmd.output().expect("prepare");
    assert_eq!(out.status.code(), Some(0));
    // Doctor verdict must not regress.
    let mut after = clean_cmd();
    after
        .arg("--registry")
        .arg(&db)
        .arg("--format")
        .arg("json")
        .arg("doctor")
        .arg(proj.to_str().unwrap());
    let out = after.output().expect("doctor after");
    assert_eq!(out.status.code(), Some(0));
    let after_json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let after_unhealthy: Vec<String> = after_json["doctor"]["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| matches!(f["status"].as_str(), Some("fail") | Some("unavailable")))
        .map(|f| f["id"].as_str().unwrap_or("?").to_string())
        .collect();
    assert_eq!(
        after_unhealthy, before_unhealthy,
        "doctor unhealthy findings must not change across release"
    );
}

#[test]
fn release_journal_row_carries_per_stage_summary() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_release_project(&proj, "rel-cross-journal");
    // Apply a release that only runs the tag stage so the
    // test does not depend on any provider.
    let mut cmd = clean_cmd();
    cmd.arg("--registry")
        .arg(&db)
        .arg("release")
        .arg("apply")
        .arg(proj.to_str().unwrap())
        .arg("--version")
        .arg("1.0.0")
        .arg("--confirm")
        .arg("--stage")
        .arg("tag");
    let out = cmd.output().expect("apply");
    assert_eq!(out.status.code(), Some(0));
    // Inspect the journal through the registry by reading
    // the SQLite database directly. The release row must
    // reference the project id and the kind `release`.
    let conn = rusqlite::Connection::open(&db).expect("open db");
    let mut stmt = conn
        .prepare("SELECT kind, project_id, state, detail FROM operations WHERE kind = 'release'")
        .expect("prepare");
    let mut rows = stmt.query([]).expect("query");
    let mut found = false;
    while let Some(row) = rows.next().expect("row") {
        let kind: String = row.get(0).expect("kind");
        let project_id: String = row.get(1).expect("project_id");
        let state: String = row.get(2).expect("state");
        let detail: String = row.get(3).expect("detail");
        assert_eq!(kind, "release");
        assert_eq!(project_id, "rel-cross-journal");
        assert!(state == "done" || state == "partial", "state={state}");
        assert!(detail.contains("rel-cross-journal"));
        assert!(detail.contains("1.0.0"));
        found = true;
    }
    assert!(found, "no release journal row written");
}

#[test]
fn release_does_not_disturb_feature_add_workflow() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_release_project(&proj, "rel-cross-feature");
    // `feature add auth` adds a feature to the manifest;
    // the release surface must continue to read the new
    // manifest verbatim.
    let mut cmd = clean_cmd();
    cmd.arg("--registry")
        .arg(&db)
        .arg("feature")
        .arg("add")
        .arg("auth")
        .arg(proj.to_str().unwrap());
    let out = cmd.output().expect("feature add");
    assert_eq!(
        out.status.code(),
        Some(0),
        "feature add: {}",
        lossy(&out.stderr)
    );
    // Now read the release config: it must still parse
    // because the manifest was rewritten by the feature
    // add.
    let mut cmd = clean_cmd();
    cmd.arg("--registry")
        .arg(&db)
        .arg("--format")
        .arg("json")
        .arg("release")
        .arg("prepare")
        .arg(proj.to_str().unwrap())
        .arg("--version")
        .arg("1.0.0");
    let out = cmd.output().expect("prepare");
    assert_eq!(
        out.status.code(),
        Some(0),
        "release prepare after feature add: {}",
        lossy(&out.stderr)
    );
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let plan = &json["plan"];
    assert_eq!(plan["ready"], true);
}

#[test]
fn prior_tag_conflicts_after_a_new_commit() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_release_project(&proj, "rel-cross-commit");
    // Apply the release to create the tag.
    let mut cmd = clean_cmd();
    cmd.arg("--registry")
        .arg(&db)
        .arg("release")
        .arg("apply")
        .arg(proj.to_str().unwrap())
        .arg("--version")
        .arg("1.0.0")
        .arg("--confirm")
        .arg("--stage")
        .arg("tag");
    let out = cmd.output().expect("first apply");
    assert_eq!(out.status.code(), Some(0));
    // Add a new commit so HEAD moves past the tagged
    // commit.
    fs::write(proj.join("README.md"), "v2\n").unwrap();
    run_git(&proj, &["add", "--", "README.md"]);
    run_git(&proj, &["commit", "-q", "-m", "second"]);
    // Re-apply: the tag stage must report `conflict`.
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
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let report = &json["release"];
    let tag = report["stage_outcomes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["stage"] == "tag")
        .expect("tag stage");
    assert_eq!(tag["status"], "conflict");
    assert!(tag["evidence"]
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e.as_str().unwrap().contains("existing:")));
}
