//! CLI contract for `publish-fleet-dry-run-verdict-truth`.
//!
//! The fleet publish's provider lane judges its run with the one publish
//! rule: a run that attempted no write is a plan, a run the provider
//! reported healthy is a delivery, and a run the provider did not report
//! healthy is a failure. The lane reads its health from the provider's
//! answer alone, records the run's verdict in the registry journal row
//! instead of a private copy of the rule, and records no container
//! identity for a run that published nothing.
//!
//! - a planned provider publish journals `planned` and is not healthy;
//! - a delivered provider publish still journals `done` and is healthy;
//! - a provider publish that is not healthy still journals `failed`.
//!
//! The journal row is read from the registry's own SQLite database, the
//! way the other publish contract tests read it.

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
        .env_remove("FORGE_PUBLISH_PROVIDER_TIMEOUT_SECS");
    cmd
}

fn run(db: &Path, args: &[&str]) -> std::process::Output {
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(db);
    cmd.arg("--format").arg("json");
    for a in args {
        cmd.arg(a);
    }
    cmd.output().expect("run forge")
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
        "git {args:?} failed: {}",
        lossy(&out.stderr)
    );
}

fn git_output(dir: &Path, args: &[&str]) -> String {
    let mut cmd = Command::new("git");
    cmd.arg("-C").arg(dir);
    for a in args {
        cmd.arg(a);
    }
    let out = cmd.output().expect("git");
    assert!(out.status.success(), "git {args:?} failed");
    lossy(&out.stdout).trim().to_string()
}

/// A project that is a git worktree at a real revision, with a Compose
/// contract and a configured provider.
fn write_project(dir: &Path, provider_command: &str) {
    fs::create_dir_all(dir.join(".forge")).unwrap();
    fs::write(dir.join("docker-compose.yml"), "services: {}\n").unwrap();
    fs::write(
        dir.join(".forge/providers.yaml"),
        format!("providers:\n  - id: probe\n    command: {provider_command}\n    enabled: true\n"),
    )
    .unwrap();
    run_git(dir, &["init", "-q"]);
    run_git(dir, &["config", "user.email", "forge@example.com"]);
    run_git(dir, &["config", "user.name", "Forge Test"]);
    run_git(dir, &["config", "init.defaultBranch", "main"]);
    run_git(
        dir,
        &["add", "--", "docker-compose.yml", ".forge/providers.yaml"],
    );
    run_git(dir, &["commit", "-q", "-m", "initial"]);
}

/// The inventory document the fleet lane consumes.
fn write_inventory(dir: &Path, project_dir: &Path, revision: &str) -> PathBuf {
    let document = serde_json::json!({
        "contract": "forge-project-inventory/0.1.0",
        "provider": "local",
        "generated_at": "2026-09-29T00:00:00Z",
        "projects": [
            {
                "id": "alpha",
                "repository": "https://example.invalid/alpha.git",
                "revision": revision,
                "profile": "rust-product",
                "runtime": "web",
                "compose_file": "docker-compose.yml",
                "source_path": project_dir.to_str().unwrap(),
            }
        ]
    });
    let path = dir.join("inventory.json");
    fs::write(&path, serde_json::to_vec(&document).unwrap()).unwrap();
    path
}

/// A stand-in provider that answers with a fixed health and status.
fn write_provider(dir: &Path, name: &str, health: &str, status: &str) -> PathBuf {
    let path = dir.join(name);
    fs::write(
        &path,
        format!(
            "#!/bin/sh\ncat >/dev/null\nprintf '%s' '{{\"contract\":\"forge-publish-provider/0.1.0\",\"provider\":\"probe\",\"operation_id\":\"op-1\",\"status\":\"{status}\",\"health\":\"{health}\",\"evidence\":[],\"recovery\":[],\"build_status\":\"succeeded\",\"run_status\":\"succeeded\",\"container_identity\":\"forge-alpha-0123456789ab\"}}'\n"
        ),
    )
    .unwrap();
    let mut perms = fs::metadata(&path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&path, perms).unwrap();
    path
}

/// One `publish` journal row with its phase evidence.
struct PublishRow {
    project_id: String,
    state: String,
    detail: String,
    container_identity: Option<String>,
    revision: Option<String>,
}

/// The `publish` journal rows with their phase evidence, oldest first.
fn publish_journal(db: &Path) -> Vec<PublishRow> {
    let conn = rusqlite::Connection::open(db).expect("open db");
    let mut stmt = conn
        .prepare(
            "SELECT project_id, state, detail, container_identity, revision \
             FROM operations WHERE kind = 'publish' ORDER BY op_id",
        )
        .expect("prepare");
    let mut rows = stmt.query([]).expect("query");
    let mut out = Vec::new();
    while let Some(row) = rows.next().expect("row") {
        out.push(PublishRow {
            project_id: row.get::<_, String>(0).expect("project_id"),
            state: row.get::<_, String>(1).expect("state"),
            detail: row.get::<_, String>(2).expect("detail"),
            container_identity: row.get::<_, Option<String>>(3).expect("container_identity"),
            revision: row.get::<_, Option<String>>(4).expect("revision"),
        });
    }
    out
}

/// The fleet summary document the command prints.
///
/// The command prints the provider's own answer first and the fleet
/// summary second, so the documents are read with a brace matcher and
/// the summary is the last one.
fn fleet_summary(stdout: &[u8]) -> serde_json::Value {
    let text = lossy(stdout);
    let mut last = serde_json::Value::Null;
    let mut rest = text.as_str();
    while let Some(start) = rest.find('{') {
        let mut depth = 0usize;
        let mut in_string = false;
        let mut escaped = false;
        let mut end = 0usize;
        for (index, ch) in rest[start..].char_indices() {
            if escaped {
                escaped = false;
                continue;
            }
            match ch {
                '\\' if in_string => escaped = true,
                '"' => in_string = !in_string,
                '{' if !in_string => depth += 1,
                '}' if !in_string => {
                    depth -= 1;
                    if depth == 0 {
                        end = start + index + ch.len_utf8();
                        break;
                    }
                }
                _ => {}
            }
        }
        if end == 0 {
            break;
        }
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&rest[start..end]) {
            last = value;
        }
        rest = &rest[end..];
    }
    assert!(
        last.get("fleet").is_some(),
        "no fleet summary in stdout: {text}"
    );
    last
}

#[test]
fn a_planned_fleet_provider_publish_is_not_a_completed_publish() {
    // A dry run sends the request and contacts nobody, so it is a plan:
    // not healthy, journalled as `planned`, and with no container.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    let provider = write_provider(tmp.path(), "probe.sh", "healthy", "delivered");
    write_project(&proj, provider.to_str().unwrap());
    let revision = git_output(&proj, &["rev-parse", "HEAD"]);
    let inventory = write_inventory(tmp.path(), &proj, &revision);

    let out = run(
        &db,
        &[
            "publish",
            "fleet",
            "--inventory",
            inventory.to_str().unwrap(),
            "--provider",
            "probe",
            "--dry-run",
        ],
    );
    assert_eq!(
        out.status.code(),
        Some(0),
        "a planned publish is not a failure; stderr={}",
        lossy(&out.stderr)
    );

    let summary = fleet_summary(&out.stdout);
    let project = &summary["fleet"]["projects"][0];
    assert_eq!(project["healthy"], false, "{project}");
    assert_eq!(project["verdict"], "planned", "{project}");

    let rows = publish_journal(&db);
    assert!(!rows.is_empty(), "the publish wrote no journal row");
    let row = rows.last().expect("journal row");
    assert_eq!(row.project_id, "alpha");
    assert_eq!(row.revision.as_deref(), Some(revision.as_str()));
    assert_eq!(row.state, "planned", "detail={}", row.detail);
    assert!(
        row.detail.contains("verdict=planned"),
        "the detail names the run: {}",
        row.detail
    );
    assert!(
        row.detail.contains("healthy=false"),
        "the detail names the health: {}",
        row.detail
    );
    assert!(
        row.container_identity.is_none(),
        "a plan created no container: {:?}",
        row.container_identity
    );
}

#[test]
fn a_delivered_fleet_provider_publish_is_unchanged() {
    // A provider that reports the target healthy is a delivery: `done`,
    // healthy, and the provider's container identity is recorded.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    let provider = write_provider(tmp.path(), "probe.sh", "healthy", "delivered");
    write_project(&proj, provider.to_str().unwrap());
    let revision = git_output(&proj, &["rev-parse", "HEAD"]);
    let inventory = write_inventory(tmp.path(), &proj, &revision);

    let out = run(
        &db,
        &[
            "publish",
            "fleet",
            "--inventory",
            inventory.to_str().unwrap(),
            "--provider",
            "probe",
        ],
    );
    assert_eq!(
        out.status.code(),
        Some(0),
        "a delivered publish exits 0; stderr={}",
        lossy(&out.stderr)
    );

    let summary = fleet_summary(&out.stdout);
    let project = &summary["fleet"]["projects"][0];
    assert_eq!(project["healthy"], true, "{project}");
    assert_eq!(project["verdict"], "delivered", "{project}");

    let rows = publish_journal(&db);
    let row = rows.last().expect("journal row");
    assert_eq!(row.project_id, "alpha");
    assert_eq!(row.state, "done", "detail={}", row.detail);
    assert!(
        row.detail.contains("verdict=delivered"),
        "the detail names the run: {}",
        row.detail
    );
    assert_eq!(
        row.container_identity.as_deref(),
        Some("forge-alpha-0123456789ab"),
        "the provider's container identity is recorded"
    );
}

#[test]
fn a_fleet_provider_publish_that_is_not_healthy_fails() {
    // A provider that reports no healthy target is a failure.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    let provider = write_provider(tmp.path(), "probe.sh", "degraded", "failed");
    write_project(&proj, provider.to_str().unwrap());
    let revision = git_output(&proj, &["rev-parse", "HEAD"]);
    let inventory = write_inventory(tmp.path(), &proj, &revision);

    let out = run(
        &db,
        &[
            "publish",
            "fleet",
            "--inventory",
            inventory.to_str().unwrap(),
            "--provider",
            "probe",
        ],
    );
    assert_eq!(
        out.status.code(),
        Some(1),
        "a failed publish exits 1; stderr={}",
        lossy(&out.stderr)
    );

    let summary = fleet_summary(&out.stdout);
    let project = &summary["fleet"]["projects"][0];
    assert_eq!(project["healthy"], false, "{project}");
    assert_eq!(project["verdict"], "failed", "{project}");
    assert_eq!(summary["fleet"]["failed"], 1, "{summary}");

    let rows = publish_journal(&db);
    let row = rows.last().expect("journal row");
    assert_eq!(row.project_id, "alpha");
    assert_eq!(row.state, "failed", "detail={}", row.detail);
    assert!(
        row.detail.contains("verdict=failed"),
        "the detail names the run: {}",
        row.detail
    );
}
