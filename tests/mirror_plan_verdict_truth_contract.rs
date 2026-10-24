//! CLI contract for `mirror-plan-verdict-truth`.
//!
//! One rule: the verdict names what the run did. A plan is not a
//! delivery, and a delivery that did not reach every enabled remote
//! is not a delivery either.
//!
//! - a dry run is `planned`, is never healthy, exits 0 and journals
//!   a `planned` row;
//! - a provider that is declared but not supported is `unavailable`
//!   in the plan and in the apply, and neither run claims a
//!   delivery;
//! - a full delivery is `delivered`, is healthy and journals a `done`
//!   row;
//! - a partial delivery is `partial`, is not healthy, exits 1 and
//!   journals a `partial` row.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

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

fn lossy(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).to_string()
}

/// Run the CLI in JSON mode and return the parsed document together
/// with the raw process output, so the test can assert the exit code
/// and the report from one run.
fn run_json_out(db: &Path, args: &[&str]) -> (serde_json::Value, Output) {
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
    (value, out)
}

/// Run the CLI in human mode and return the raw process output.
fn run_human(db: &Path, args: &[&str]) -> Output {
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(db);
    for a in args {
        cmd.arg(a);
    }
    cmd.output().expect("run forge human")
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
        "git {:?} failed: status={} stdout={} stderr={}",
        args,
        out.status,
        lossy(&out.stdout),
        lossy(&out.stderr)
    );
}

fn bare_repo(tmp: &Path, label: &str) -> PathBuf {
    let path = tmp.join(format!("{label}.git"));
    fs::create_dir_all(&path).unwrap();
    let mut cmd = Command::new("git");
    cmd.arg("init").arg("--bare").arg("-q").arg(&path);
    let out = cmd.output().expect("git init --bare");
    assert!(
        out.status.success(),
        "git init --bare failed: {}",
        lossy(&out.stderr)
    );
    let mut head_cmd = Command::new("git");
    head_cmd
        .arg("symbolic-ref")
        .arg("HEAD")
        .arg("refs/heads/main")
        .current_dir(&path);
    let head_out = head_cmd.output().expect("git symbolic-ref HEAD");
    assert!(
        head_out.status.success(),
        "git symbolic-ref HEAD failed: {}",
        lossy(&head_out.stderr)
    );
    path
}

/// Write a project whose manifest declares a github primary and the
/// given mirrors.
fn write_distribution_project(dir: &Path, id: &str, mirrors: &[&str]) {
    fs::create_dir_all(dir).unwrap();
    let mut text = format!(
        "schema: 1\nproject:\n  id: {id}\n  name: Test {id}\n  profile: rust-web\n  maturity: L1\nruntime:\n  language: rust\ndistribution:\n  primary: github\n  mirrors:\n"
    );
    for mirror in mirrors {
        text.push_str(&format!("    - {mirror}\n"));
    }
    fs::write(dir.join("forge.yaml"), &text).unwrap();
    fs::write(dir.join("README.md"), "v1\n").unwrap();
    run_git(dir, &["init", "-q"]);
    run_git(dir, &["config", "user.email", "forge@example.com"]);
    run_git(dir, &["config", "user.name", "Forge Test"]);
    run_git(dir, &["config", "init.defaultBranch", "main"]);
    run_git(dir, &["checkout", "-q", "-b", "main"]);
    run_git(dir, &["add", "--", "forge.yaml", "README.md"]);
    run_git(dir, &["commit", "-q", "-m", "initial"]);
}

/// The `mirror` journal rows, oldest first.
fn mirror_journal(db: &Path) -> Vec<(String, String, String)> {
    let conn = rusqlite::Connection::open(db).expect("open db");
    let mut stmt = conn
        .prepare(
            "SELECT project_id, state, detail FROM operations WHERE kind = 'mirror' ORDER BY op_id",
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

fn outcome_for(report: &serde_json::Value, provider: &str) -> serde_json::Value {
    report["outcomes"]
        .as_array()
        .expect("outcomes")
        .iter()
        .find(|o| o["provider"] == provider)
        .cloned()
        .unwrap_or_else(|| panic!("no outcome for provider {provider}"))
}

#[test]
fn dry_run_mirror_is_planned_and_journals_a_planned_row() {
    // A plan contacts no remote and pushes nothing. The report must
    // say so, and the journal must not record a completed mirror.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_distribution_project(&proj, "plan-verdict", &["gitee"]);

    let (value, out) = run_json_out(
        &db,
        &[
            "mirror",
            proj.to_str().unwrap(),
            "--ref",
            "main",
            "--dry-run",
        ],
    );
    assert_eq!(
        out.status.code(),
        Some(0),
        "a plan must exit 0; stderr={}",
        lossy(&out.stderr)
    );
    let report = &value["mirror"];
    assert_eq!(report["verdict"], "planned");
    assert_eq!(report["dry_run"], true);
    assert_eq!(report["healthy"], false);
    let note = report["note"].as_str().expect("note");
    assert!(note.contains("plan only"), "note={note}");

    let rows = mirror_journal(&db);
    assert_eq!(rows.len(), 1, "one journal row per run: {rows:?}");
    let (project_id, state, detail) = &rows[0];
    assert_eq!(project_id, "plan-verdict");
    assert_eq!(state, "planned");
    assert!(detail.contains("verdict=planned"), "detail={detail}");
    assert!(detail.contains("healthy=false"), "detail={detail}");

    // The human renderer names the same verdict the JSON document
    // and the journal row name.
    let human_db = tmp.path().join("human-registry.db");
    let human = run_human(
        &human_db,
        &[
            "mirror",
            proj.to_str().unwrap(),
            "--ref",
            "main",
            "--dry-run",
        ],
    );
    assert_eq!(human.status.code(), Some(0));
    let text = lossy(&human.stdout);
    assert!(text.contains("verdict: planned"), "human output:\n{text}");
    assert!(text.contains("summary: plan only"), "human output:\n{text}");
}

#[test]
fn an_unsupported_provider_is_unavailable_in_both_the_plan_and_the_apply() {
    // GitLab is declared but not supported. The plan and the apply
    // must name the same provider outcome, and neither run may claim
    // a delivery.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_distribution_project(&proj, "unsupported-provider", &["gitlab"]);
    let primary = bare_repo(tmp.path(), "origin");
    run_git(
        &proj,
        &["remote", "add", "origin", primary.to_str().unwrap()],
    );

    let (plan, plan_out) = run_json_out(
        &db,
        &[
            "mirror",
            proj.to_str().unwrap(),
            "--ref",
            "main",
            "--dry-run",
        ],
    );
    assert_eq!(
        plan_out.status.code(),
        Some(0),
        "a plan must exit 0; stderr={}",
        lossy(&plan_out.stderr)
    );
    let plan_report = &plan["mirror"];
    assert_eq!(plan_report["verdict"], "planned");
    assert_eq!(plan_report["healthy"], false);
    assert_eq!(outcome_for(plan_report, "gitlab")["status"], "unavailable");

    let (apply, apply_out) = run_json_out(
        &db,
        &[
            "mirror",
            proj.to_str().unwrap(),
            "--ref",
            "main",
            "--confirm",
        ],
    );
    assert_eq!(
        apply_out.status.code(),
        Some(1),
        "a run that did not reach every remote must fail; stderr={}",
        lossy(&apply_out.stderr)
    );
    let apply_report = &apply["mirror"];
    assert_eq!(apply_report["verdict"], "partial");
    assert_eq!(apply_report["healthy"], false);
    assert_eq!(outcome_for(apply_report, "gitlab")["status"], "unavailable");
    // The primary was written; the planned provider was not.
    assert_eq!(outcome_for(apply_report, "github")["status"], "delivered");
}

#[test]
fn a_delivered_mirror_is_delivered() {
    // Both remotes are real bare repositories, so every enabled
    // remote reaches the ref.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_distribution_project(&proj, "delivered-verdict", &["gitee"]);
    let primary = bare_repo(tmp.path(), "origin");
    let mirror = bare_repo(tmp.path(), "gitee");
    run_git(
        &proj,
        &["remote", "add", "origin", primary.to_str().unwrap()],
    );
    run_git(
        &proj,
        &["remote", "add", "mirror-gitee", mirror.to_str().unwrap()],
    );

    let (value, out) = run_json_out(
        &db,
        &[
            "mirror",
            proj.to_str().unwrap(),
            "--ref",
            "main",
            "--confirm",
        ],
    );
    assert_eq!(
        out.status.code(),
        Some(0),
        "a delivery must exit 0; stderr={}",
        lossy(&out.stderr)
    );
    let report = &value["mirror"];
    assert_eq!(report["verdict"], "delivered");
    assert_eq!(report["healthy"], true);

    let rows = mirror_journal(&db);
    assert_eq!(rows.len(), 1, "one journal row per run: {rows:?}");
    assert_eq!(rows[0].0, "delivered-verdict");
    assert_eq!(rows[0].1, "done");
    assert!(
        rows[0].2.contains("verdict=delivered"),
        "detail={}",
        rows[0].2
    );
}

#[test]
fn a_partial_mirror_is_partial() {
    // The primary is a real bare repository; the mirror remote is
    // intentionally absent, so the mirror push fails.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_distribution_project(&proj, "partial-verdict", &["gitee"]);
    let primary = bare_repo(tmp.path(), "origin");
    run_git(
        &proj,
        &["remote", "add", "origin", primary.to_str().unwrap()],
    );

    let (value, out) = run_json_out(
        &db,
        &[
            "mirror",
            proj.to_str().unwrap(),
            "--ref",
            "main",
            "--confirm",
        ],
    );
    assert_eq!(
        out.status.code(),
        Some(1),
        "a partial run must fail; stderr={}",
        lossy(&out.stderr)
    );
    let report = &value["mirror"];
    assert_eq!(report["verdict"], "partial");
    assert_eq!(report["healthy"], false);
    assert_eq!(outcome_for(report, "github")["status"], "delivered");
    assert_eq!(outcome_for(report, "gitee")["status"], "failed");

    let rows = mirror_journal(&db);
    assert_eq!(rows.len(), 1, "one journal row per run: {rows:?}");
    assert_eq!(rows[0].0, "partial-verdict");
    assert_eq!(rows[0].1, "partial");
    assert!(
        rows[0].2.contains("verdict=partial"),
        "detail={}",
        rows[0].2
    );
}
