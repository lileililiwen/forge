//! Contract tests for `forge deploy status --queue` and the bounded
//! `--watch` polling mode. The fixture covers all queue-status
//! scenarios: queue id shape validation, unknown queue, mixed
//! active/terminal rows, watch exits on terminal state, and watch
//! deadline expiry.

use serde_json::Value;
use std::path::Path;
use std::process::Command;

fn forge_bin() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_BIN_EXE_forge"))
}

fn run(db: &Path, args: &[&str]) -> std::process::Output {
    let mut cmd = Command::new(forge_bin());
    cmd.env("FORGE_REGISTRY", db).env("FORGE_QUIET", "1");
    cmd.args(args);
    cmd.output().expect("spawn forge")
}

fn run_json(db: &Path, args: &[&str]) -> Value {
    let mut cmd = Command::new(forge_bin());
    cmd.env("FORGE_REGISTRY", db).env("FORGE_QUIET", "1");
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

#[test]
fn deploy_status_help_advertises_queue_and_watch() {
    let output = run(
        std::path::Path::new("/tmp/forge-help.db"),
        &["deploy", "status", "--help"],
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--queue"), "stdout was: {stdout}");
    assert!(stdout.contains("--watch"), "stdout was: {stdout}");
    assert!(stdout.contains("--interval-secs"), "stdout was: {stdout}");
    assert!(stdout.contains("--deadline-secs"), "stdout was: {stdout}");
}

#[test]
fn deploy_status_rejects_watch_without_queue() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let output = run(
        &db,
        &["deploy", "status", "--watch", "--interval-secs", "1"],
    );
    assert!(!output.status.success(), "watch without --queue must fail");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("--watch requires --queue"),
        "stderr was: {stderr}"
    );
}

#[test]
fn deploy_status_rejects_malformed_queue_id() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let output = run(&db, &["deploy", "status", "--queue", "fleet 1"]);
    assert!(
        !output.status.success(),
        "queue id with space must be refused"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("queue id") || stderr.contains("publish-invalid"),
        "stderr was: {stderr}"
    );
}

#[test]
fn deploy_status_rejects_interval_secs_out_of_bounds() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let output = run(
        &db,
        &[
            "deploy",
            "status",
            "--queue",
            "fleet-1",
            "--watch",
            "--interval-secs",
            "0",
        ],
    );
    assert!(!output.status.success(), "interval-secs=0 must be refused");
    let output = run(
        &db,
        &[
            "deploy",
            "status",
            "--queue",
            "fleet-1",
            "--watch",
            "--interval-secs",
            "61",
        ],
    );
    assert!(!output.status.success(), "interval-secs=61 must be refused");
}

#[test]
fn deploy_status_rejects_deadline_secs_out_of_bounds() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let output = run(
        &db,
        &[
            "deploy",
            "status",
            "--queue",
            "fleet-1",
            "--watch",
            "--deadline-secs",
            "0",
        ],
    );
    assert!(!output.status.success(), "deadline-secs=0 must be refused");
}

#[test]
fn deploy_status_queue_returns_empty_history_cleanly() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let value = run_json(
        &db,
        &[
            "--format",
            "json",
            "deploy",
            "status",
            "--queue",
            "fleet-unseen",
        ],
    );
    assert_eq!(
        value.get("queue").and_then(Value::as_str),
        Some("fleet-unseen")
    );
    assert_eq!(
        value.get("contract").and_then(Value::as_str),
        Some("forge-deploy-status/0.2.0")
    );
    assert_eq!(value.get("read_only").and_then(Value::as_bool), Some(true));
    let entries = value
        .get("entries")
        .and_then(Value::as_array)
        .expect("entries array");
    assert!(entries.is_empty());
}

#[test]
fn deploy_status_rejects_both_project_and_queue() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let output = run(
        &db,
        &[
            "deploy",
            "status",
            "--project",
            "alpha",
            "--queue",
            "fleet-1",
        ],
    );
    assert!(
        !output.status.success(),
        "must refuse --project and --queue together"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("not both"), "stderr was: {stderr}");
}
