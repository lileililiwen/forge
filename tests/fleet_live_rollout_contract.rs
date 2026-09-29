//! Contract tests for `fleet-live-rollout`: concurrent fleet publish,
//! the sync timeout, and the `--fleet-registry` repair.
//!
//! Every test is dry-run or plan-only except the scheduler unit
//! tests (which live in `src/main.rs`): no SSH connection is
//! opened and no target state changes. Live 20/20 evidence is
//! recorded in HANDOFF, not here.

use std::fs;
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
        .env_remove("FORGE_WORKSPACE_ROOT")
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

/// A fresh registry file inside `dir` for one test.
fn registry_in(dir: &Path) -> PathBuf {
    dir.join("registry.db")
}

/// Parse the multi-document fleet dry-run output: one JSON report
/// per project plus the trailing fleet summary.
fn fleet_docs(stdout: &[u8]) -> Vec<Value> {
    let text = lossy(stdout);
    let mut docs = Vec::new();
    let mut rest = text.trim_start();
    while !rest.trim().is_empty() {
        rest = rest.trim_start();
        let mut depth = 0;
        let mut in_string = false;
        let mut escaped = false;
        let mut end = 0;
        for (i, ch) in rest.char_indices() {
            if in_string {
                if escaped {
                    escaped = false;
                } else if ch == '\\' {
                    escaped = true;
                } else if ch == '"' {
                    in_string = false;
                }
                continue;
            }
            match ch {
                '"' => in_string = true,
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        end = i + ch.len_utf8();
                        break;
                    }
                }
                _ => {}
            }
        }
        assert!(end > 0, "unbalanced JSON in fleet output");
        docs.push(serde_json::from_str(&rest[..end]).expect("fleet doc parses"));
        rest = &rest[end..];
    }
    docs
}

/// Stage a project directory with a real Compose file.
fn stage_project_dir(parent: &Path, project_id: &str) -> PathBuf {
    let dir = parent.join(project_id);
    fs::create_dir_all(&dir).expect("create project dir");
    fs::write(dir.join("docker-compose.yml"), "services: {}\n").expect("write compose");
    dir
}

/// Write a two-project inventory document pointing at staged dirs.
fn write_inventory(dir: &Path) -> PathBuf {
    let alpha = stage_project_dir(dir, "alpha");
    let beta = stage_project_dir(dir, "beta");
    let document = serde_json::json!({
        "contract": "forge-project-inventory/0.1.0",
        "provider": "local",
        "generated_at": "2026-09-29T00:00:00Z",
        "projects": [
            {
                "id": "alpha",
                "repository": "https://example.invalid/alpha.git",
                "revision": "0123456789abcdef0123456789abcdef01234567",
                "profile": "rust-product",
                "runtime": "web",
                "compose_file": "docker-compose.yml",
                "source_path": alpha.to_str().unwrap(),
            },
            {
                "id": "beta",
                "repository": "https://example.invalid/beta.git",
                "revision": "0123456789abcdef0123456789abcdef01234567",
                "profile": "rust-product",
                "runtime": "web",
                "compose_file": "docker-compose.yml",
                "source_path": beta.to_str().unwrap(),
            },
        ],
    });
    let path = dir.join("inventory.json");
    fs::write(&path, serde_json::to_string_pretty(&document).unwrap()).expect("write inventory");
    path
}

/// Write a legacy `projects.json` registry pointing at staged dirs.
/// Returns `(registry_path, workspace_root)`: entry paths resolve
/// against the workspace root, so the caller passes `--workspace-root`.
fn write_legacy_registry(dir: &Path) -> (PathBuf, PathBuf) {
    let ws = dir.join("ws");
    fs::create_dir_all(&ws).expect("create ws");
    stage_project_dir(&ws, "alpha");
    stage_project_dir(&ws, "beta");
    let document = serde_json::json!({
        "workspace_root": ws.to_str().unwrap(),
        "projects": [
            {"id": "alpha", "path": "alpha", "lifecycle": "active"},
            {"id": "beta", "path": "beta", "lifecycle": "active"},
        ],
    });
    let path = ws.join("projects.json");
    fs::write(&path, serde_json::to_string_pretty(&document).unwrap()).expect("write registry");
    (path, ws)
}

#[test]
fn fleet_help_advertises_jobs_flag() {
    let tmp = tempfile::tempdir().unwrap();
    let out = run(&registry_in(tmp.path()), &["publish", "fleet", "--help"]);
    assert!(out.status.success(), "stderr={}", lossy(&out.stderr));
    let stdout = lossy(&out.stdout);
    assert!(stdout.contains("--jobs"), "missing --jobs:\n{stdout}");
    assert!(
        stdout.contains("--fleet-registry"),
        "missing --fleet-registry:\n{stdout}"
    );
}

#[test]
fn jobs_flag_is_bounded() {
    let tmp = tempfile::tempdir().unwrap();
    let inventory = write_inventory(tmp.path());
    for bad in ["0", "33", "999"] {
        let db = registry_in(tmp.path());
        let out = run(
            &db,
            &[
                "publish",
                "fleet",
                "--inventory",
                inventory.to_str().unwrap(),
                "--dry-run",
                "--jobs",
                bad,
            ],
        );
        assert!(!out.status.success(), "--jobs {bad} must refuse");
        assert!(
            lossy(&out.stderr).contains("publish-invalid"),
            "expected publish-invalid; got {}",
            lossy(&out.stderr)
        );
        assert!(
            lossy(&out.stdout).is_empty(),
            "refusal must print nothing on stdout"
        );
    }
}

#[test]
fn jobs_one_dry_run_matches_default_dry_run() {
    // `--jobs 1` dry-run must be identical to the default
    // sequential dry-run: same per-project reports (modulo
    // elapsed_ms), same fleet counts. Dry-run never takes the
    // parallel path, so this pins the flag as behavior-preserving.
    let tmp = tempfile::tempdir().unwrap();
    let inventory = write_inventory(tmp.path());
    let db = registry_in(tmp.path());
    let one = run(
        &db,
        &[
            "--format",
            "json",
            "publish",
            "fleet",
            "--inventory",
            inventory.to_str().unwrap(),
            "--dry-run",
            "--jobs",
            "1",
        ],
    );
    assert!(one.status.success(), "stderr={}", lossy(&one.stderr));
    let seq = run(
        &db,
        &[
            "--format",
            "json",
            "publish",
            "fleet",
            "--inventory",
            inventory.to_str().unwrap(),
            "--dry-run",
        ],
    );
    assert!(seq.status.success(), "stderr={}", lossy(&seq.stderr));
    let one_docs = fleet_docs(&one.stdout);
    let seq_docs = fleet_docs(&seq.stdout);
    assert_eq!(one_docs.len(), seq_docs.len());
    assert_eq!(one_docs.len(), 3, "two projects plus summary");
    for (a, b) in one_docs.iter().zip(seq_docs.iter()) {
        let mut a = a.clone();
        let mut b = b.clone();
        for doc in [&mut a, &mut b] {
            if let Some(stages) = doc.get_mut("stages").and_then(Value::as_array_mut) {
                for stage in stages {
                    if let Some(obj) = stage.as_object_mut() {
                        obj.remove("elapsed_ms");
                    }
                }
            }
            // Volatile run identity: the queue id embeds a
            // timestamp, and `jobs` is the flag under test.
            if let Some(fleet) = doc.get_mut("fleet").and_then(Value::as_object_mut) {
                fleet.remove("queue_id");
                fleet.remove("jobs");
            }
        }
        assert_eq!(a, b);
    }
    let summary = one_docs.last().unwrap();
    assert_eq!(
        summary
            .get("fleet")
            .and_then(|f| f.get("compose_ready"))
            .and_then(Value::as_u64),
        Some(2)
    );
    assert_eq!(
        summary
            .get("fleet")
            .and_then(|f| f.get("jobs"))
            .and_then(Value::as_u64),
        Some(1)
    );
}

#[test]
fn jobs_four_dry_run_reports_jobs_in_summary() {
    let tmp = tempfile::tempdir().unwrap();
    let inventory = write_inventory(tmp.path());
    let db = registry_in(tmp.path());
    let out = run(
        &db,
        &[
            "--format",
            "json",
            "publish",
            "fleet",
            "--inventory",
            inventory.to_str().unwrap(),
            "--dry-run",
            "--jobs",
            "4",
        ],
    );
    assert!(out.status.success(), "stderr={}", lossy(&out.stderr));
    let docs = fleet_docs(&out.stdout);
    let summary = docs.last().unwrap();
    assert_eq!(
        summary
            .get("fleet")
            .and_then(|f| f.get("jobs"))
            .and_then(Value::as_u64),
        Some(4)
    );
}

#[test]
fn fleet_registry_flag_loads_through_legacy_adapter() {
    // D3: an explicit `--fleet-registry <path>` routes through
    // `legacy_inventory_snapshot` — never the inventory branch —
    // so a valid `projects.json` classifies its entries.
    let tmp = tempfile::tempdir().unwrap();
    let (registry, workspace_root) = write_legacy_registry(tmp.path());
    let db = registry_in(tmp.path());
    let out = run(
        &db,
        &[
            "--format",
            "json",
            "publish",
            "fleet",
            "--fleet-registry",
            registry.to_str().unwrap(),
            "--workspace-root",
            workspace_root.to_str().unwrap(),
            "--dry-run",
        ],
    );
    assert!(out.status.success(), "stderr={}", lossy(&out.stderr));
    let docs = fleet_docs(&out.stdout);
    let summary = docs.last().unwrap();
    assert!(
        summary
            .get("inventory_source")
            .and_then(Value::as_str)
            .map(|s| s.starts_with("registry:"))
            .unwrap_or(false),
        "source must be the legacy registry; got {summary}"
    );
    assert_eq!(
        summary
            .get("fleet")
            .and_then(|f| f.get("compose_ready"))
            .and_then(Value::as_u64),
        Some(2)
    );
}

#[test]
fn fleet_registry_flag_beats_inventory_flag() {
    // When both flags are present, the explicit `--fleet-registry`
    // wins (documented precedence change of D3).
    let tmp = tempfile::tempdir().unwrap();
    let inventory = write_inventory(tmp.path());
    let legacy_dir = tmp.path().join("legacy");
    fs::create_dir_all(&legacy_dir).unwrap();
    let (registry, workspace_root) = write_legacy_registry(&legacy_dir);
    let db = registry_in(tmp.path());
    let out = run(
        &db,
        &[
            "--format",
            "json",
            "publish",
            "fleet",
            "--inventory",
            inventory.to_str().unwrap(),
            "--fleet-registry",
            registry.to_str().unwrap(),
            "--workspace-root",
            workspace_root.to_str().unwrap(),
            "--dry-run",
        ],
    );
    assert!(out.status.success(), "stderr={}", lossy(&out.stderr));
    let docs = fleet_docs(&out.stdout);
    let summary = docs.last().unwrap();
    assert!(
        summary
            .get("inventory_source")
            .and_then(Value::as_str)
            .map(|s| s.starts_with("registry:"))
            .unwrap_or(false),
        "fleet-registry must win; got {summary}"
    );
}

#[test]
fn fleet_registry_missing_file_refuses_typed() {
    let tmp = tempfile::tempdir().unwrap();
    let missing = tmp.path().join("no-such-registry.json");
    let db = registry_in(tmp.path());
    let out = run(
        &db,
        &[
            "publish",
            "fleet",
            "--fleet-registry",
            missing.to_str().unwrap(),
            "--dry-run",
        ],
    );
    assert!(!out.status.success(), "missing registry must refuse");
    assert!(
        lossy(&out.stderr).contains("publish-invalid"),
        "expected publish-invalid; got {}",
        lossy(&out.stderr)
    );
}
