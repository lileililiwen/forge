//! CLI contract for the dry-run truth of `forge release apply`
//! (`release-dry-run-evidence-truth`).
//!
//! A dry run states that no side effect was attempted, so the
//! repository must carry no new tag and no new commit, and no
//! stage outcome in the same report may claim a delivery. The
//! cases drive the built binary through the built CLI:
//!
//! - a dry run writes nothing and records a `planned` journal row;
//! - a dry run with `--confirm` writes nothing either: the dry
//!   run is the stronger promise the report makes;
//! - a confirmed, non-dry release still delivers the tag.
//!
//! The journal row is read from the registry's own SQLite
//! database, the way the other release contract tests read it.

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

/// Run the CLI in JSON mode and return both the process outcome and
/// the parsed document, so a test can assert the exit code and read
/// the report from one run.
fn run_json_output(db: &Path, args: &[&str]) -> (std::process::Output, serde_json::Value) {
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(db);
    cmd.arg("--format").arg("json");
    for a in args {
        cmd.arg(a);
    }
    let out = cmd.output().expect("run forge json");
    let value = serde_json::from_slice(&out.stdout).unwrap_or_else(|err| {
        panic!(
            "invalid json: {err}; stdout={} stderr={}",
            lossy(&out.stdout),
            lossy(&out.stderr)
        )
    });
    (out, value)
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

/// The trimmed stdout of one git invocation.
fn git_output(dir: &Path, args: &[&str]) -> String {
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
    lossy(&out.stdout).trim().to_string()
}

/// Write the Gate evidence record the shared delivery rule accepts,
/// bound to the project's own HEAD revision. The record is written
/// after the fixture's commit, so the revision it names is the one a
/// release captures.
fn write_gate_evidence(dir: &Path, project_id: &str) {
    let revision = git_output(dir, &["rev-parse", "HEAD"]);
    let record = serde_json::json!({
        "contract": forge::gate::contract::GATE_CONTRACT_VERSION,
        "project_id": project_id,
        "runtime": "forge",
        "profile": "full",
        "revision": revision,
        "aggregate": "passed",
        "checks": [],
        "observed_at": "2026-10-21T00:00:00Z",
        "dry_run": false
    });
    let path = dir
        .join(".forge/gate")
        .join(project_id)
        .join("evidence.json");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, serde_json::to_vec(&record).unwrap()).unwrap();
}

/// A release project whose changelog carries an uncommitted entry,
/// so the commit stage would have something to commit if it were
/// allowed to write.
fn write_release_project(dir: &Path, id: &str) {
    write_release_project_with_distribution(dir, id, "");
}

/// The same project, with an optional `distribution` block that
/// gives the mirror stage a plan to collect.
fn write_release_project_with_distribution(dir: &Path, id: &str, distribution: &str) {
    fs::create_dir_all(dir).unwrap();
    let text = format!(
        "schema: 1\nproject:\n  id: {id}\n  name: {id}\n  profile: rust-web\n  maturity: L1\nruntime:\n  language: rust\n{distribution}release:\n  versioning: semver\n  checks:\n    - kind: doctor\n    - kind: test\n  changelog: CHANGELOG.md\n"
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
    // The Gate check always applies to a release, so the fixture
    // carries the evidence the shared delivery rule accepts.
    write_gate_evidence(dir, id);
    // An unreleased changelog entry the commit stage would commit
    // if the run were allowed to write.
    fs::write(
        dir.join("CHANGELOG.md"),
        "## 1.0.0\n- initial release\n- an unreleased entry\n",
    )
    .unwrap();
}

/// The journal row the command wrote for the `release` operation.
fn journal_row(db: &Path) -> (String, String) {
    let conn = rusqlite::Connection::open(db).expect("open db");
    conn.query_row(
        "SELECT state, detail FROM operations WHERE kind = 'release' ORDER BY op_id DESC LIMIT 1",
        [],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )
    .expect("a release journal row")
}

#[test]
fn dry_run_apply_writes_no_tag_and_records_a_planned_journal_row() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_release_project(&proj, "rel-dry-run");
    let head = git_output(&proj, &["rev-parse", "HEAD"]);
    let (out, value) = run_json_output(
        &db,
        &[
            "release",
            "apply",
            proj.to_str().unwrap(),
            "--version",
            "1.0.0",
            "--dry-run",
        ],
    );
    assert_eq!(out.status.code(), Some(0), "stderr={}", lossy(&out.stderr));
    let report = &value["release"];
    assert_eq!(report["dry_run"], true);
    let note = report["note"].as_str().unwrap();
    assert!(
        note.contains("no side effects were attempted"),
        "the note must state that nothing was written: {note}"
    );
    let outcomes = report["stage_outcomes"].as_array().unwrap();
    assert!(!outcomes.is_empty(), "the report must record stages");
    for outcome in outcomes {
        assert_ne!(
            outcome["status"].as_str().unwrap(),
            "delivered",
            "a dry run stage must not claim a delivery: {outcome:?}"
        );
    }
    assert_eq!(
        git_output(&proj, &["tag", "--list"]),
        "",
        "a dry run must create no tag"
    );
    assert_eq!(
        git_output(&proj, &["rev-parse", "HEAD"]),
        head,
        "a dry run must create no commit"
    );
    let (state, detail) = journal_row(&db);
    assert_eq!(state, "planned", "detail={detail}");
    assert!(detail.contains("dry_run=true"), "detail={detail}");
}

#[test]
fn dry_run_apply_with_confirm_writes_no_tag() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_release_project(&proj, "rel-dry-run-confirm");
    let head = git_output(&proj, &["rev-parse", "HEAD"]);
    let (out, value) = run_json_output(
        &db,
        &[
            "release",
            "apply",
            proj.to_str().unwrap(),
            "--version",
            "1.0.0",
            "--confirm",
            "--dry-run",
        ],
    );
    assert_eq!(out.status.code(), Some(0), "stderr={}", lossy(&out.stderr));
    assert_eq!(value["release"]["dry_run"], true);
    assert_eq!(
        git_output(&proj, &["tag", "--list"]),
        "",
        "a dry run must create no tag"
    );
    assert_eq!(
        git_output(&proj, &["rev-parse", "HEAD"]),
        head,
        "a dry run must create no commit"
    );
    let (state, _) = journal_row(&db);
    assert_eq!(state, "planned");
}

#[test]
fn confirmed_apply_still_delivers_the_tag() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_release_project(&proj, "rel-confirmed");
    let (out, value) = run_json_output(
        &db,
        &[
            "release",
            "apply",
            proj.to_str().unwrap(),
            "--version",
            "1.0.0",
            "--confirm",
            "--stage",
            "commit",
            "--stage",
            "tag",
        ],
    );
    assert_eq!(out.status.code(), Some(0), "stderr={}", lossy(&out.stderr));
    assert_eq!(git_output(&proj, &["tag", "--list"]), "v1.0.0");
    let report = &value["release"];
    let tag = report["stage_outcomes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["stage"] == "tag")
        .expect("tag stage");
    assert_eq!(tag["status"], "delivered");
}

#[test]
fn dry_run_reports_the_mirror_plan_without_claiming_a_delivery() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_release_project_with_distribution(
        &proj,
        "rel-mirror-plan",
        "distribution:\n  primary: github\n  mirrors:\n    - gitee\n",
    );
    let (out, value) = run_json_output(
        &db,
        &[
            "release",
            "apply",
            proj.to_str().unwrap(),
            "--version",
            "1.0.0",
            "--dry-run",
        ],
    );
    assert_eq!(out.status.code(), Some(0), "stderr={}", lossy(&out.stderr));
    let report = &value["release"];
    let mirror = report["stage_outcomes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["stage"] == "mirror")
        .expect("mirror stage");
    // The stage reports the plan it collected, and it does not
    // claim that it pushed anything.
    assert_eq!(mirror["status"], "disabled");
    let note = mirror["note"].as_str().unwrap();
    assert!(
        note.contains("mirror plan only"),
        "the mirror stage must name the plan: {note}"
    );
    assert!(
        note.contains("no side effect was attempted"),
        "the mirror stage must name why it did not write: {note}"
    );
    // The plan evidence is still there.
    let evidence: Vec<&str> = mirror["evidence"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e.as_str().unwrap())
        .collect();
    assert!(
        evidence.iter().any(|line| line.contains("github")),
        "the plan must name the primary: {evidence:?}"
    );
    assert!(
        evidence.iter().any(|line| line.contains("gitee")),
        "the plan must name the mirror: {evidence:?}"
    );
}
