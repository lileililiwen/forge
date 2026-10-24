//! CLI contract for `release-plan-verdict-truth`.
//!
//! One rule names what a release run did: a plan the checks allow, a
//! plan a check blocks, an applied release and an applied release that
//! did not complete every stage. A preparation releases nothing, so it
//! is never an applied release. Both writers read the same rule, and
//! every journal row names the run in its detail.
//!
//! - a ready preparation journals `planned` and names the run;
//! - a blocked preparation stays `blocked` and names the failing check;
//! - a rehearsal preparation still journals `planned`;
//! - an applied release still journals `done`.
//!
//! The journal row is read from the registry's own SQLite database,
//! the way the other release contract tests read it.

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
/// the plan from one run.
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
        "git {args:?} failed: status={} stderr={}",
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
        "git {args:?} failed: status={} stderr={}",
        out.status,
        lossy(&out.stderr)
    );
    lossy(&out.stdout).trim().to_string()
}

/// Write the Gate evidence record the shared delivery rule accepts,
/// bound to the project's own HEAD revision.
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
        "observed_at": "2026-10-24T00:00:00Z",
        "dry_run": false
    });
    let path = dir
        .join(".forge/gate")
        .join(project_id)
        .join("evidence.json");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, serde_json::to_vec(&record).unwrap()).unwrap();
}

/// A release project with a committed changelog. The project declares
/// no optional check, so the shared Gate rule is the only applicable
/// check and the plan's readiness follows the Gate evidence alone.
fn write_release_project(dir: &Path, id: &str) {
    fs::create_dir_all(dir).unwrap();
    let text = format!(
        "schema: 1\nproject:\n  id: {id}\n  name: {id}\n  profile: rust-web\n  maturity: L1\nruntime:\n  language: rust\nrelease:\n  versioning: semver\n  changelog: CHANGELOG.md\n"
    );
    fs::write(dir.join("forge.yaml"), text).unwrap();
    fs::write(dir.join("README.md"), "v1\n").unwrap();
    fs::write(dir.join("CHANGELOG.md"), "## 1.0.0\n- initial release\n").unwrap();
    run_git(dir, &["init", "-q"]);
    run_git(dir, &["config", "user.email", "forge@example.com"]);
    run_git(dir, &["config", "user.name", "Forge Test"]);
    run_git(dir, &["config", "init.defaultBranch", "main"]);
    run_git(
        dir,
        &["add", "--", "forge.yaml", "README.md", "CHANGELOG.md"],
    );
    run_git(dir, &["commit", "-q", "-m", "initial"]);
}

/// The same project with the Gate evidence that satisfies the shared
/// delivery rule, so the plan is ready.
fn write_gated_release_project(dir: &Path, id: &str) {
    write_release_project(dir, id);
    write_gate_evidence(dir, id);
}

/// The `release` journal rows, oldest first.
fn release_journal(db: &Path) -> Vec<(String, String, String)> {
    let conn = rusqlite::Connection::open(db).expect("open db");
    let mut stmt = conn
        .prepare(
            "SELECT project_id, state, detail FROM operations WHERE kind = 'release' ORDER BY op_id",
        )
        .expect("prepare");
    let mut rows = stmt.query([]).expect("query");
    let mut out = Vec::new();
    while let Some(row) = rows.next().expect("row") {
        out.push((
            row.get::<_, String>(0).expect("project_id"),
            row.get::<_, String>(1).expect("state"),
            row.get::<_, String>(2).expect("detail"),
        ));
    }
    out
}

#[test]
fn a_ready_preparation_is_planned_and_journals_a_planned_row() {
    // A preparation releases nothing: no tag, no new commit. The
    // journal row must not claim a completed release.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_gated_release_project(&proj, "rel-planned");
    let head = git_output(&proj, &["rev-parse", "HEAD"]);

    let (out, value) = run_json_output(
        &db,
        &[
            "release",
            "prepare",
            proj.to_str().unwrap(),
            "--version",
            "1.0.0",
        ],
    );
    assert_eq!(
        out.status.code(),
        Some(0),
        "a ready preparation exits 0; stderr={}",
        lossy(&out.stderr)
    );
    let plan = &value["plan"];
    assert_eq!(plan["ready"], true, "{plan}");

    assert_eq!(
        git_output(&proj, &["tag", "--list"]),
        "",
        "a preparation must create no tag"
    );
    assert_eq!(
        git_output(&proj, &["rev-parse", "HEAD"]),
        head,
        "a preparation must create no commit"
    );

    let rows = release_journal(&db);
    assert!(!rows.is_empty(), "the preparation wrote no journal row");
    let (project_id, state, detail) = rows.last().expect("journal row");
    assert_eq!(project_id, "rel-planned");
    assert_eq!(
        state, "planned",
        "a preparation is not an applied release; detail={detail}"
    );
    assert!(
        detail.contains("verdict=planned"),
        "the detail names the run: {detail}"
    );
    assert!(
        detail.contains("ready=true"),
        "the detail keeps the readiness: {detail}"
    );
}

#[test]
fn a_blocked_preparation_stays_blocked() {
    // Without the Gate evidence the shared delivery rule refuses the
    // revision, so the plan is blocked and the row says so.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_release_project(&proj, "rel-blocked");

    let (out, value) = run_json_output(
        &db,
        &[
            "release",
            "prepare",
            proj.to_str().unwrap(),
            "--version",
            "1.0.0",
        ],
    );
    assert_eq!(
        out.status.code(),
        Some(0),
        "a blocked preparation still reports its plan; stderr={}",
        lossy(&out.stderr)
    );
    let plan = &value["plan"];
    assert_eq!(plan["ready"], false, "{plan}");
    let note = plan["note"].as_str().unwrap();
    assert!(
        note.contains("gate"),
        "the plan names the failing check: {note}"
    );

    let rows = release_journal(&db);
    let (_, state, detail) = rows.last().expect("journal row");
    assert_eq!(state, "blocked", "detail={detail}");
    assert!(
        detail.contains("verdict=blocked"),
        "the detail names the run: {detail}"
    );
    assert!(
        detail.contains("ready=false"),
        "the detail keeps the readiness: {detail}"
    );
}

#[test]
fn a_rehearsal_preparation_is_still_planned() {
    // A read-only preparation releases nothing either, so its row
    // names a planned run.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_gated_release_project(&proj, "rel-rehearsal");

    let (out, value) = run_json_output(
        &db,
        &[
            "release",
            "prepare",
            proj.to_str().unwrap(),
            "--version",
            "1.0.0",
            "--dry-run",
        ],
    );
    assert_eq!(
        out.status.code(),
        Some(0),
        "a rehearsal exits 0; stderr={}",
        lossy(&out.stderr)
    );
    assert_eq!(value["plan"]["ready"], true, "{:?}", value["plan"]);

    let rows = release_journal(&db);
    let (_, state, detail) = rows.last().expect("journal row");
    assert_eq!(state, "planned", "detail={detail}");
    assert!(
        detail.contains("verdict=planned"),
        "the detail names the run: {detail}"
    );
    assert!(
        detail.contains("dry_run=true"),
        "the detail keeps the rehearsal flag: {detail}"
    );
}

#[test]
fn an_applied_release_still_journals_a_done_row() {
    // The applied release is unchanged: it still journals `done`, and
    // the detail now names the run as well.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_gated_release_project(&proj, "rel-delivered");

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
            "tag",
        ],
    );
    assert_eq!(
        out.status.code(),
        Some(0),
        "an applied release exits 0; stderr={}",
        lossy(&out.stderr)
    );
    assert_eq!(value["release"]["healthy"], true, "{:?}", value["release"]);
    assert_eq!(git_output(&proj, &["tag", "--list"]), "v1.0.0");

    let rows = release_journal(&db);
    let (_, state, detail) = rows.last().expect("journal row");
    assert_eq!(state, "done", "detail={detail}");
    assert!(
        detail.contains("verdict=delivered"),
        "the detail names the run: {detail}"
    );
}

#[test]
fn an_applied_release_that_did_not_complete_journals_a_partial_row() {
    // The push stage cannot deliver because the project has no
    // `origin` remote, so the applied release is partial and the row
    // says so.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_gated_release_project(&proj, "rel-partial");

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
            "push",
        ],
    );
    assert_eq!(
        out.status.code(),
        Some(1),
        "an incomplete release exits 1; stderr={}",
        lossy(&out.stderr)
    );
    let report = &value["release"];
    assert_eq!(report["healthy"], false, "{report}");
    let outcomes = report["stage_outcomes"].as_array().unwrap();
    let push = outcomes
        .iter()
        .find(|o| o["stage"] == "push")
        .expect("push stage");
    assert_eq!(
        push["status"].as_str().unwrap(),
        "failed",
        "the push stage did not deliver: {push:?}"
    );

    let rows = release_journal(&db);
    let (_, state, detail) = rows.last().expect("journal row");
    assert_eq!(state, "partial", "detail={detail}");
    assert!(
        detail.contains("verdict=partial"),
        "the detail names the run: {detail}"
    );
}
