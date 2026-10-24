//! CLI contract for `publish-dry-run-verdict-truth`.
//!
//! One rule: the verdict names what the run did. A plan is not a
//! delivery, a delivery is healthy, and a failure fails the command.
//!
//! - a dry run is `planned`, is never healthy, exits 0 and journals a
//!   `planned` row;
//! - a completed publish is `delivered`, is healthy, exits 0 and
//!   journals a `done` row;
//! - a failed publish is `failed`, is never healthy, exits 1 and
//!   journals a `failed` row.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;

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

/// Stage a project directory the publish lane can plan.
fn project_dir(parent: &Path, id: &str) -> PathBuf {
    let dir = parent.join(id);
    fs::create_dir_all(&dir).expect("project dir");
    fs::write(
        dir.join("docker-compose.yml"),
        "services:\n  web:\n    image: demo:latest\n    ports:\n      - \"8000:80\"\n",
    )
    .expect("compose file");
    dir
}

/// Stage a fake `ssh` and `rsync` pair, so the publish lane reaches a
/// target without one. Every staged command exits with `code`.
fn fake_target_bin(parent: &Path, code: i32) -> PathBuf {
    let bin = parent.join("fake-target-bin");
    fs::create_dir_all(&bin).expect("fake bin dir");
    for program in ["ssh", "rsync"] {
        let path = bin.join(program);
        fs::write(&path, format!("#!/bin/sh\nexit {code}\n")).expect("fake program");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).expect("chmod");
    }
    bin
}

/// The PATH the fake target programs are found under.
fn fake_target_path(bin: &Path) -> String {
    let inherited = std::env::var("PATH").unwrap_or_default();
    format!("{}:{inherited}", bin.display())
}

/// Run the CLI in JSON mode and return the parsed document together
/// with the raw process output, so the test can assert the exit code
/// and the report from one run.
fn run_json_out(db: &Path, envs: &[(&str, &str)], args: &[&str]) -> (Value, Output) {
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(db);
    cmd.arg("--format").arg("json");
    for (key, value) in envs {
        cmd.env(key, value);
    }
    for arg in args {
        cmd.arg(arg);
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

/// The `publish` journal rows, oldest first.
fn publish_journal(db: &Path) -> Vec<(String, String, String)> {
    let conn = rusqlite::Connection::open(db).expect("open db");
    let mut stmt = conn
        .prepare(
            "SELECT project_id, state, detail FROM operations WHERE kind = 'publish' ORDER BY op_id",
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

/// The per-project fleet rows: the queued terminal row each project
/// writes, with its state and its detail.
fn publish_queue_rows(db: &Path) -> Vec<(String, String, String)> {
    let conn = rusqlite::Connection::open(db).expect("open db");
    let mut stmt = conn
        .prepare(
            "SELECT project_id, state, detail FROM operations \
             WHERE kind = 'publish' AND queue_id IS NOT NULL ORDER BY op_id",
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

/// The summary row: the one journal row that names the run's verdict.
fn publish_summary_row(db: &Path) -> (String, String, String) {
    let rows = publish_journal(db);
    let summary = rows
        .into_iter()
        .filter(|(_, _, detail)| detail.contains("summary"))
        .collect::<Vec<_>>();
    assert_eq!(summary.len(), 1, "one summary row per run: {summary:?}");
    summary.into_iter().next().expect("summary row")
}

/// Every JSON document the command printed, in order. A fleet run
/// prints one document per project and one fleet summary.
fn json_documents(stdout: &[u8]) -> Vec<Value> {
    let text = lossy(stdout);
    let stream = serde_json::Deserializer::from_str(&text).into_iter::<Value>();
    let mut docs = Vec::new();
    for doc in stream {
        docs.push(doc.unwrap_or_else(|err| panic!("invalid json: {err}; stdout={text}")));
    }
    docs
}

/// Write an inventory that declares one project at `source`.
fn write_inventory(path: &Path, project_id: &str, source: &Path) {
    let payload = serde_json::json!({
        "contract": "forge-project-inventory/0.1.0",
        "provider": "local",
        "generated_at": "2026-09-28T00:00:00Z",
        "projects": [{
            "id": project_id,
            "repository": format!("https://github.com/example/{project_id}.git"),
            "revision": "0123456789abcdef0123456789abcdef01234567",
            "profile": "rust-product",
            "runtime": "web",
            "compose_file": "docker-compose.yml",
            "source_path": source.display().to_string(),
            "public_http": true,
            "public_port": 8080,
        }],
    });
    fs::write(
        path,
        serde_json::to_string_pretty(&payload).expect("inventory json"),
    )
    .expect("write");
}

#[test]
fn a_dry_run_is_planned_and_journals_a_planned_row() {
    // A plan contacts no target and writes nothing. The report must
    // say so, the command must exit 0, and the journal must not record
    // a completed publish.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = project_dir(tmp.path(), "verdictplan");

    let (value, out) = run_json_out(
        &db,
        &[],
        &["publish", "all", proj.to_str().unwrap(), "--dry-run"],
    );
    assert_eq!(
        out.status.code(),
        Some(0),
        "a plan must exit 0; stderr={}",
        lossy(&out.stderr)
    );
    assert_eq!(value["dry_run"], true);
    assert_eq!(value["verdict"], "planned");
    assert_eq!(value["healthy"], false);
    let note = value["note"].as_str().expect("note");
    assert!(note.contains("planned, no side effects"), "note={note}");

    let (project_id, state, detail) = publish_summary_row(&db);
    assert_eq!(project_id, "verdictplan");
    assert_eq!(state, "planned");
    assert!(detail.contains("verdict=planned"), "detail={detail}");
    assert!(detail.contains("healthy=false"), "detail={detail}");
}

#[test]
fn a_completed_publish_is_delivered_and_journals_a_done_row() {
    // Every stage completes, so the run delivered and the journal
    // records a completed publish.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = project_dir(tmp.path(), "verdictshipped");
    let bin = fake_target_bin(tmp.path(), 0);

    let (value, out) = run_json_out(
        &db,
        &[
            ("FORGE_PUBLISH_ADAPTER", "jenkins"),
            ("PATH", &fake_target_path(&bin)),
        ],
        &["publish", "all", proj.to_str().unwrap()],
    );
    assert_eq!(
        out.status.code(),
        Some(0),
        "a delivery must exit 0; stderr={}",
        lossy(&out.stderr)
    );
    assert_eq!(value["dry_run"], false);
    assert_eq!(value["verdict"], "delivered");
    assert_eq!(value["healthy"], true);

    let (project_id, state, detail) = publish_summary_row(&db);
    assert_eq!(project_id, "verdictshipped");
    assert_eq!(state, "done");
    assert!(detail.contains("verdict=delivered"), "detail={detail}");
    assert!(detail.contains("healthy=true"), "detail={detail}");
}

#[test]
fn a_failed_publish_is_failed_and_journals_a_failed_row() {
    // The target refuses the first stage, so the run failed, the
    // command exits 1 and the journal records a failed publish.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = project_dir(tmp.path(), "verdictfailed");
    let bin = fake_target_bin(tmp.path(), 1);

    let (value, out) = run_json_out(
        &db,
        &[
            ("FORGE_PUBLISH_ADAPTER", "jenkins"),
            ("PATH", &fake_target_path(&bin)),
        ],
        &["publish", "all", proj.to_str().unwrap()],
    );
    assert_eq!(
        out.status.code(),
        Some(1),
        "a failed publish must exit 1; stderr={}",
        lossy(&out.stderr)
    );
    assert_eq!(value["verdict"], "failed");
    assert_eq!(value["healthy"], false);

    let (project_id, state, detail) = publish_summary_row(&db);
    assert_eq!(project_id, "verdictfailed");
    assert_eq!(state, "failed");
    assert!(detail.contains("verdict=failed"), "detail={detail}");
    assert!(detail.contains("healthy=false"), "detail={detail}");
}

#[test]
fn a_planned_fleet_publish_is_planned_and_not_a_failure() {
    // The fleet lane follows the same rule: a planned project records
    // the state `planned`, its summary names the verdict, and the
    // fleet is not a failure.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = project_dir(tmp.path(), "fleetplan");
    let inventory = tmp.path().join("inventory.json");
    write_inventory(&inventory, "fleetplan", &proj);
    let runtime = tmp.path().join("runtime");
    fs::create_dir_all(&runtime).expect("runtime root");

    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(&db);
    cmd.arg("--format").arg("json");
    cmd.env("FORGE_PUBLISH_RUNTIME_ROOT", &runtime);
    cmd.args([
        "publish",
        "fleet",
        "--inventory",
        inventory.to_str().unwrap(),
        "--dry-run",
    ]);
    let out = cmd.output().expect("run forge fleet");
    assert_eq!(
        out.status.code(),
        Some(0),
        "a planned fleet is not a failure; stderr={}",
        lossy(&out.stderr)
    );

    let docs = json_documents(&out.stdout);
    let summary = docs.last().expect("fleet summary document");
    assert_eq!(summary["fleet"]["failed"], 0);
    let project = &summary["fleet"]["projects"][0];
    assert_eq!(project["project"], "fleetplan");
    assert_eq!(project["verdict"], "planned");
    assert_eq!(project["healthy"], false);

    let queue_rows = publish_queue_rows(&db);
    assert_eq!(queue_rows.len(), 1, "one per-project row: {queue_rows:?}");
    assert_eq!(queue_rows[0].0, "fleetplan");
    assert_eq!(queue_rows[0].1, "planned");
    assert!(
        queue_rows[0].2.contains("verdict=planned"),
        "detail={}",
        queue_rows[0].2
    );
}
