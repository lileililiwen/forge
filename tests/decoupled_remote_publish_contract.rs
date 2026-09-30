//! Contract tests for `decoupled-remote-publish`: the Forge-owned
//! remote-compose adapter is the default publish lane and the legacy
//! Mac-script lane remains selectable for one-cycle rollback.
//!
//! Every test is dry-run only: no SSH connection is opened and no
//! target state changes. The suite pins tasks 2.6 (adapter selection
//! without new CLI flags), 3.1 (no script strings on the decoupled
//! plan), 3.4 (env-only cloud portability) and 3.5 (rollback is a
//! flag).

use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;

fn forge_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_forge"))
}

fn project_dir(parent: &tempfile::TempDir, id: &str) -> PathBuf {
    let dir = parent.path().join(id);
    std::fs::create_dir_all(&dir).expect("fixture dir");
    std::fs::write(
        dir.join("docker-compose.yml"),
        "services:\n  web:\n    image: demo:latest\n    ports:\n      - \"8000:80\"\n",
    )
    .expect("fixture compose");
    dir
}

fn run_json(envs: &[(&str, &str)], args: &[&str]) -> Value {
    let tmp = tempfile::tempdir().expect("registry tempdir");
    let db = tmp.path().join("registry.db");
    let mut cmd = Command::new(forge_bin());
    cmd.env("FORGE_REGISTRY", &db).env("FORGE_QUIET", "1");
    for (key, value) in envs {
        cmd.env(key, value);
    }
    cmd.args(args);
    let output = cmd.output().expect("spawn forge");
    assert!(
        output.status.success(),
        "forge {args:?} failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("parse forge JSON")
}

fn commands_of(report: &Value) -> Vec<String> {
    report
        .get("stages")
        .and_then(Value::as_array)
        .expect("stages array")
        .iter()
        .flat_map(|stage| {
            stage
                .get("command")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
        })
        .filter_map(|cmd| cmd.as_str().map(str::to_string))
        .collect()
}

#[test]
fn decoupled_adapter_is_the_default_and_invokes_no_target_script() {
    let parent = tempfile::tempdir().unwrap();
    let dir = project_dir(&parent, "canaryproj");
    let report = run_json(
        &[],
        &[
            "publish",
            "prepare",
            dir.to_str().unwrap(),
            "--dry-run",
            "--format",
            "json",
        ],
    );
    assert_eq!(
        report.get("adapter").and_then(Value::as_str),
        Some("remote-compose")
    );
    let commands = commands_of(&report).join("\n");
    for forbidden in [
        "project-ports.sh",
        "project-action.sh",
        "shared-postgres.sh",
    ] {
        assert!(
            !commands.contains(forbidden),
            "decoupled plan must not reference `{forbidden}`: {commands}"
        );
    }
    assert!(commands.contains("ssh "), "plan was: {commands}");
    assert!(
        commands.contains("scp ") || commands.contains("rsync "),
        "plan was: {commands}"
    );
}

#[test]
fn decoupled_all_stages_render_ssh_rsync_and_compose() {
    let parent = tempfile::tempdir().unwrap();
    let dir = project_dir(&parent, "canaryproj");
    let report = run_json(
        &[],
        &[
            "publish",
            "all",
            dir.to_str().unwrap(),
            "--dry-run",
            "--format",
            "json",
        ],
    );
    assert_eq!(
        report.get("adapter").and_then(Value::as_str),
        Some("remote-compose")
    );
    let commands = commands_of(&report).join("\n");
    assert!(commands.contains("rsync -az"), "plan was: {commands}");
    assert!(
        commands.contains("--exclude appendonlydir/") && commands.contains("*.rdb"),
        "runtime volume state must never sync: {commands}"
    );
    assert!(commands.contains("docker compose"), "plan was: {commands}");
    assert!(
        commands.contains("-p forge-canaryproj"),
        "plan was: {commands}"
    );
    assert!(
        !commands.contains("project-action.sh"),
        "plan was: {commands}"
    );
}

#[test]
fn legacy_adapter_returns_with_an_env_flag() {
    let parent = tempfile::tempdir().unwrap();
    let dir = project_dir(&parent, "canaryproj");
    let report = run_json(
        &[("FORGE_PUBLISH_ADAPTER", "jenkins")],
        &[
            "publish",
            "prepare",
            dir.to_str().unwrap(),
            "--dry-run",
            "--format",
            "json",
        ],
    );
    assert_eq!(
        report.get("adapter").and_then(Value::as_str),
        Some("jenkins")
    );
    let commands = commands_of(&report).join("\n");
    assert!(
        commands.contains("project-ports.sh"),
        "legacy plan was: {commands}"
    );
}

#[test]
fn cloud_target_is_env_only_with_no_script_path() {
    let parent = tempfile::tempdir().unwrap();
    let dir = project_dir(&parent, "canaryproj");
    let report = run_json(
        &[
            ("FORGE_PUBLISH_SSH_TARGET", "cloud"),
            ("FORGE_PUBLISH_REMOTE_ROOT", "/srv/projects"),
            ("FORGE_PUBLISH_RUNTIME_ROOT", "/srv/runtime"),
            ("FORGE_PUBLISH_PLATFORM_ROOT", "/srv/platform"),
            ("FORGE_PUBLISH_SECRETS_ROOT", "/srv/secrets"),
            ("FORGE_PUBLISH_SHARED_INFRA_ROOT", "/srv/shared-infra"),
        ],
        &[
            "publish",
            "sync",
            dir.to_str().unwrap(),
            "--dry-run",
            "--format",
            "json",
        ],
    );
    assert_eq!(
        report.get("adapter").and_then(Value::as_str),
        Some("remote-compose")
    );
    let commands = commands_of(&report).join("\n");
    assert!(
        commands.contains("cloud:/srv/projects/canaryproj/"),
        "plan was: {commands}"
    );
    assert!(
        !commands.contains(".sh"),
        "cloud plan must not reference a shell script: {commands}"
    );
}

#[test]
fn adapter_flag_is_case_insensitive_and_legacy_aliases_work() {
    let parent = tempfile::tempdir().unwrap();
    let dir = project_dir(&parent, "canaryproj");
    for flag in ["JENKINS", "legacy"] {
        let report = run_json(
            &[("FORGE_PUBLISH_ADAPTER", flag)],
            &[
                "publish",
                "prepare",
                dir.to_str().unwrap(),
                "--dry-run",
                "--format",
                "json",
            ],
        );
        assert_eq!(
            report.get("adapter").and_then(Value::as_str),
            Some("jenkins"),
            "flag was: {flag}"
        );
    }
    let _ = Path::new(".");
}
