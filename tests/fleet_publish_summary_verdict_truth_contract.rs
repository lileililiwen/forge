//! CLI contract for `fleet-publish-summary-verdict-truth`.
//!
//! The fleet summary counts the verdict: a delivery is a success, a
//! plan is a plan and a failure is a failure. A plan is never counted
//! as a success, and a delivery is never counted as nothing — the
//! summary never contradicts the rows it summarises.
//!
//! - a planned fleet publish is counted as a plan, not as a success;
//! - a delivered fleet publish is counted as a success;
//! - a failed fleet publish is counted as a failure;
//! - the human summary names the planned runs.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

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

fn run_git(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap_or_else(|_| panic!("git {args:?} failed"));
    assert!(
        out.status.success(),
        "git {args:?} failed: {}",
        lossy(&out.stderr)
    );
}

fn git_output(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .expect("git");
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

/// The fleet summary document the command prints.
///
/// The command prints the provider's own answer first and the fleet
/// summary second, so the documents are read with a brace matcher and
/// the summary is the last one.
fn fleet_summary(stdout: &[u8]) -> Value {
    let text = lossy(stdout);
    let mut last = Value::Null;
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
        if let Ok(value) = serde_json::from_str::<Value>(&rest[start..end]) {
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

/// Run the fleet publish and return its exit code, its summary document
/// and its human line.
///
/// The command prints one document per format, so it runs twice: once
/// in JSON for the summary counters and once in the default format for
/// the human line.
fn publish_fleet(db: &Path, inventory: &Path, dry_run: bool) -> (i32, Value, String) {
    let run = |json: bool| {
        let mut cmd = clean_cmd();
        cmd.arg("--registry").arg(db);
        if json {
            cmd.arg("--format").arg("json");
        }
        cmd.arg("publish")
            .arg("fleet")
            .arg("--inventory")
            .arg(inventory)
            .arg("--provider")
            .arg("probe");
        if dry_run {
            cmd.arg("--dry-run");
        }
        let out = cmd.output().expect("run forge publish fleet");
        (out.status.code().unwrap_or(-1), out.stdout)
    };
    let (code, stdout) = run(true);
    let summary = fleet_summary(&stdout);
    let (_, human) = run(false);
    (code, summary, lossy(&human))
}

#[test]
fn a_planned_fleet_publish_is_not_counted_as_a_success() {
    // A dry run contacts nobody: it is a plan, and the summary must
    // not claim it succeeded.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    let provider = write_provider(tmp.path(), "probe.sh", "healthy", "delivered");
    write_project(&proj, provider.to_str().unwrap());
    let revision = git_output(&proj, &["rev-parse", "HEAD"]);
    let inventory = write_inventory(tmp.path(), &proj, &revision);

    let (code, summary, stdout) = publish_fleet(&db, &inventory, true);
    assert_eq!(
        code, 0,
        "a planned publish is not a failure; stdout={stdout}"
    );

    let fleet = &summary["fleet"];
    assert_eq!(fleet["success"], 0, "a plan is not a success: {fleet}");
    assert_eq!(
        fleet["planned"], 1,
        "the plan is counted as a plan: {fleet}"
    );
    assert_eq!(fleet["failed"], 0, "{fleet}");
    assert_eq!(
        fleet["projects"][0]["verdict"], "planned",
        "the summary agrees with the row it summarises: {fleet}"
    );
    assert!(
        stdout.contains("0/1 publishable succeeded"),
        "the human line names the delivery count: {stdout}"
    );
    assert!(
        stdout.contains("1 planned"),
        "the human line names the planned runs: {stdout}"
    );
}

#[test]
fn a_delivered_fleet_publish_is_counted_as_a_success() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    let provider = write_provider(tmp.path(), "probe.sh", "healthy", "delivered");
    write_project(&proj, provider.to_str().unwrap());
    let revision = git_output(&proj, &["rev-parse", "HEAD"]);
    let inventory = write_inventory(tmp.path(), &proj, &revision);

    let (code, summary, stdout) = publish_fleet(&db, &inventory, false);
    assert_eq!(code, 0, "a delivered publish succeeds; stdout={stdout}");

    let fleet = &summary["fleet"];
    assert_eq!(fleet["success"], 1, "a delivery is a success: {fleet}");
    assert_eq!(fleet["planned"], 0, "{fleet}");
    assert_eq!(fleet["failed"], 0, "{fleet}");
    assert!(
        stdout.contains("1/1 publishable succeeded"),
        "the human line names the delivery count: {stdout}"
    );
}

#[test]
fn a_failed_fleet_publish_is_counted_as_a_failure() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    let provider = write_provider(tmp.path(), "probe.sh", "unhealthy", "failed");
    write_project(&proj, provider.to_str().unwrap());
    let revision = git_output(&proj, &["rev-parse", "HEAD"]);
    let inventory = write_inventory(tmp.path(), &proj, &revision);

    let (code, summary, stdout) = publish_fleet(&db, &inventory, false);
    assert_ne!(code, 0, "a failed publish fails the command: {stdout}");

    let fleet = &summary["fleet"];
    assert_eq!(fleet["success"], 0, "a failure is not a success: {fleet}");
    assert_eq!(fleet["planned"], 0, "{fleet}");
    assert_eq!(fleet["failed"], 1, "the failure is counted: {fleet}");
}
