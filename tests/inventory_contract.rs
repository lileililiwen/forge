//! Contract tests for the portable project inventory surface
//! (`forge-independent-project-inventory-fleet`).
//!
//! Exercises the full `forge inventory show` and `forge publish
//! fleet --inventory` surface against local JSON fixtures and a
//! staged external adapter. Every scenario is driven through the
//! real CLI binary; the fixtures are versioned in
//! `tests/fixtures/inventory/` so the contract stays stable while
//! sibling workspace-governance adapters live in their own
//! repositories.

use std::fs;
use std::io::Write;
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
        .env_remove("FORGE_WORKSPACE_REGISTRY")
        .env_remove("FORGE_INVENTORY_SOURCE")
        .env_remove("HTTP_PROXY")
        .env_remove("HTTPS_PROXY")
        .env_remove("ALL_PROXY");
    cmd
}

fn run(db: &Path, args: &[&str]) -> std::process::Output {
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(db);
    for a in args {
        cmd.arg(a);
    }
    cmd.output().expect("forge CLI must run")
}

fn run_json(db: &Path, args: &[&str]) -> Value {
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(db);
    cmd.arg("--format").arg("json");
    for a in args {
        cmd.arg(a);
    }
    let out = cmd.output().expect("forge CLI json must run");
    serde_json::from_slice(&out.stdout).unwrap_or_else(|err| {
        panic!(
            "invalid json: {err}; stdout={} stderr={}",
            lossy(&out.stdout),
            lossy(&out.stderr),
        )
    })
}

/// Stage a project directory with a real Compose file so the
/// classifier marks it `compose_ready`.
fn stage_project_dir(parent: &Path, project_id: &str) -> PathBuf {
    let dir = parent.join(project_id);
    fs::create_dir_all(&dir).expect("create project dir");
    fs::write(dir.join("docker-compose.yml"), "services: {}\n").expect("write compose");
    dir
}

/// Write an inventory fixture into `target` substituting every
/// `__STAGE_<id>__` placeholder with the matching staged project
/// directory. Missing placeholders are left unchanged so the
/// classification test can intentionally point at non-existent
/// sources.
fn write_inventory(
    target: &Path,
    stage_root: &Path,
    fixture: &str,
    staged_ids: &[&str],
) -> PathBuf {
    let mut payload = fs::read_to_string(fixture)
        .expect("fixture exists")
        .to_string();
    for id in staged_ids {
        let project_dir = stage_project_dir(stage_root, id);
        let placeholder = format!("__STAGE_{id}__");
        payload = payload.replace(&placeholder, &project_dir.display().to_string());
    }
    let dest = target.join("inventory.json");
    let mut file = fs::File::create(&dest).expect("write fixture");
    file.write_all(payload.as_bytes()).expect("write");
    dest
}

#[test]
fn inventory_show_help_advertises_source_and_domain_flags() {
    let tmp = tempfile::tempdir().unwrap();
    let out = run(tmp.path(), &["inventory", "show", "--help"]);
    assert!(
        out.status.success(),
        "inventory show --help must succeed; stderr={}",
        lossy(&out.stderr)
    );
    let stdout = lossy(&out.stdout);
    assert!(stdout.contains("SOURCE"), "missing SOURCE placeholder");
    assert!(stdout.contains("--domain"), "missing --domain flag");
}

#[test]
fn inventory_show_refuses_missing_source() {
    let tmp = tempfile::tempdir().unwrap();
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(tmp.path());
    cmd.arg("inventory").arg("show");
    let out = cmd.output().expect("run");
    assert!(!out.status.success(), "missing source must refuse");
    let stderr = lossy(&out.stderr);
    assert!(
        stderr.contains("publish-invalid"),
        "expected publish-invalid; got {stderr}"
    );
    assert!(
        stderr.contains("inventory source is not configured"),
        "expected explicit refusal; got {stderr}"
    );
}

#[test]
fn inventory_show_reports_every_entry_explicitly() {
    let tmp = tempfile::tempdir().unwrap();
    // Stage alethefy (compose_ready, public). Leave forge + worker
    // un-staged so they classify as `source_unavailable`.
    let _ = stage_project_dir(tmp.path(), "alethefy");
    let inventory = write_inventory(
        tmp.path(),
        tmp.path(),
        "tests/fixtures/inventory/local-mixed.json",
        &["alethefy"],
    );
    let out = run_json(
        tmp.path(),
        &[
            "inventory",
            "show",
            inventory.to_str().unwrap(),
            "--domain",
            "example.uk",
        ],
    );
    let entries = out
        .get("entries")
        .and_then(Value::as_array)
        .expect("entries array");
    assert_eq!(entries.len(), 3, "every declared entry reported");
    let by_id = |id: &str| -> &Value {
        entries
            .iter()
            .find(|e| e.get("id").and_then(Value::as_str) == Some(id))
            .unwrap_or_else(|| panic!("missing entry {id}"))
    };
    assert_eq!(
        by_id("alethefy")
            .get("classification")
            .and_then(Value::as_str),
        Some("compose_ready")
    );
    assert_eq!(
        by_id("alethefy").get("subdomain").and_then(Value::as_str),
        Some("alethefy.example.uk")
    );
    assert_eq!(
        by_id("forge").get("classification").and_then(Value::as_str),
        Some("source_unavailable")
    );
    assert_eq!(
        by_id("worker")
            .get("classification")
            .and_then(Value::as_str),
        Some("source_unavailable")
    );
    for entry in entries {
        let classification = entry
            .get("classification")
            .and_then(Value::as_str)
            .unwrap_or("");
        assert!(
            matches!(
                classification,
                "compose_ready" | "compose_missing" | "source_unavailable" | "invalid"
            ),
            "unexpected classification: {classification}"
        );
    }
}

#[test]
fn inventory_show_never_routes_non_web_runtime() {
    let tmp = tempfile::tempdir().unwrap();
    let _ = stage_project_dir(tmp.path(), "worker");
    let inventory = write_inventory(
        tmp.path(),
        tmp.path(),
        "tests/fixtures/inventory/local-mixed.json",
        &["worker"],
    );
    let out = run_json(
        tmp.path(),
        &["inventory", "show", inventory.to_str().unwrap()],
    );
    let entries = out.get("entries").and_then(Value::as_array).unwrap();
    let worker = entries
        .iter()
        .find(|e| e.get("id").and_then(Value::as_str) == Some("worker"))
        .expect("worker entry present");
    assert_eq!(
        worker.get("runtime").and_then(Value::as_str),
        Some("worker")
    );
    assert_eq!(
        worker.get("classification").and_then(Value::as_str),
        Some("compose_ready"),
        "worker has a real Compose file at the staged source path"
    );
    assert!(
        worker.get("subdomain").is_none() || worker.get("subdomain") == Some(&Value::Null),
        "worker must never receive a public subdomain"
    );
}

#[test]
fn inventory_show_refuses_wrong_contract() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("wrong.json");
    fs::copy("tests/fixtures/inventory/wrong-contract.json", &path).unwrap();
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(tmp.path());
    cmd.arg("inventory").arg("show").arg(&path);
    let out = cmd.output().expect("run");
    assert!(!out.status.success(), "wrong contract must refuse");
    assert!(
        lossy(&out.stderr).contains("publish-invalid"),
        "expected publish-invalid; got {}",
        lossy(&out.stderr)
    );
}

#[test]
fn inventory_show_reports_malformed_entries_with_reason() {
    let tmp = tempfile::tempdir().unwrap();
    let _ = stage_project_dir(tmp.path(), "ok");
    let inventory = write_inventory(
        tmp.path(),
        tmp.path(),
        "tests/fixtures/inventory/mixed-validity.json",
        &["ok"],
    );
    let out = run_json(
        tmp.path(),
        &["inventory", "show", inventory.to_str().unwrap()],
    );
    let entries = out.get("entries").and_then(Value::as_array).unwrap();
    let bad = entries
        .iter()
        .find(|e| e.get("id").and_then(Value::as_str) == Some("bad-rev"))
        .expect("bad-rev entry present");
    assert_eq!(
        bad.get("classification").and_then(Value::as_str),
        Some("invalid")
    );
    assert!(
        bad.get("reason").is_some(),
        "invalid entry must carry a reason"
    );
}

#[test]
fn inventory_show_consumes_external_adapter_executable() {
    let tmp = tempfile::tempdir().unwrap();
    let adapter_src = PathBuf::from("tests/fixtures/inventory/fixture-adapter.sh");
    let adapter = tmp.path().join("inventory-adapter.sh");
    fs::copy(&adapter_src, &adapter).unwrap();
    let mut perms = fs::metadata(&adapter).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&adapter, perms).unwrap();
    let out = run_json(
        tmp.path(),
        &["inventory", "show", adapter.to_str().unwrap()],
    );
    let entries = out.get("entries").and_then(Value::as_array).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(
        entries[0].get("id").and_then(Value::as_str),
        Some("adapter-web")
    );
    assert_eq!(
        out.get("provider").and_then(Value::as_str),
        Some("fixture-adapter")
    );
    assert_eq!(
        entries[0].get("classification").and_then(Value::as_str),
        Some("compose_ready")
    );
}

#[test]
fn publish_fleet_inventory_advertises_new_flag() {
    let tmp = tempfile::tempdir().unwrap();
    let out = run(tmp.path(), &["publish", "fleet", "--help"]);
    assert!(out.status.success());
    let stdout = lossy(&out.stdout);
    assert!(
        stdout.contains("--inventory"),
        "publish fleet help must advertise --inventory; got {stdout}"
    );
    assert!(
        stdout.contains("forge-project-inventory"),
        "publish fleet help must reference the inventory contract"
    );
}

#[test]
fn publish_fleet_inventory_reports_zero_eligible_when_all_missing() {
    let tmp = tempfile::tempdir().unwrap();
    let inventory = tmp.path().join("inventory.json");
    fs::write(
        &inventory,
        r#"{"contract":"forge-project-inventory/0.1.0","provider":"local","generated_at":"2026-09-28T00:00:00Z","projects":[]}"#,
    )
    .unwrap();
    let out = run_json(
        tmp.path(),
        &[
            "publish",
            "fleet",
            "--inventory",
            inventory.to_str().unwrap(),
        ],
    );
    assert_eq!(out.get("compose_ready").and_then(Value::as_u64), Some(0));
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(tmp.path());
    cmd.arg("publish")
        .arg("fleet")
        .arg("--inventory")
        .arg(&inventory);
    let raw = cmd.output().expect("run");
    assert!(!raw.status.success(), "must refuse with zero eligible");
    assert!(
        lossy(&raw.stderr).contains("publish-invalid"),
        "expected publish-invalid; got {}",
        lossy(&raw.stderr)
    );
}
