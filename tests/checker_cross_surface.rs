//! Cross-surface regression for `external-checker-emission`: a
//! `forge check` run must leave every existing surface alone. The
//! registry journal stays independent of the checker surface (no row
//! is ever written by a check), the doctor verdict is byte-equivalent
//! before and after, the MCP `tools/list` snapshot never advertises
//! the checker, the portal dashboard still renders, and the feature
//! add workflow remains compatible after a check round trip.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use forge::registry::Registry;

fn forge_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_forge"))
}

fn lossy(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).to_string()
}

fn run(db: &Path, args: &[&str]) -> std::process::Output {
    let mut cmd = Command::new(forge_bin());
    cmd.env_remove("FORGE_REGISTRY")
        .env_remove("HTTP_PROXY")
        .env_remove("HTTPS_PROXY")
        .env_remove("ALL_PROXY")
        .env_remove("OPENAI_API_KEY")
        .env_remove("ANTHROPIC_API_KEY")
        .env_remove("FORGE_DRIFTWATCH_BIN");
    cmd.arg("--registry").arg(db);
    for a in args {
        cmd.arg(a);
    }
    cmd.output().expect("run forge")
}

fn write_file(dir: &Path, name: &str, text: &str) {
    let path = dir.join(name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, text).unwrap();
}

fn rust_manifest(id: &str) -> String {
    format!(
        "schema: 1\nproject:\n  id: {id}\n  name: {id}\n  profile: rust-web\n  maturity: L1\n  target_maturity: L1\nruntime:\n  language: rust\n"
    )
}

fn registered_project(parent: &Path, db: &Path, id: &str) -> PathBuf {
    let proj = parent.join(id);
    fs::create_dir_all(&proj).unwrap();
    write_file(&proj, "forge.yaml", &rust_manifest(id));
    write_file(&proj, "Cargo.toml", "[package]\nname = \"demo\"\n");
    write_file(&proj, "README.md", "# demo\n");
    write_file(&proj, "Dockerfile", "FROM scratch\n");
    let out = run(db, &["register", &proj.display().to_string()]);
    assert!(out.status.success(), "{}", lossy(&out.stderr));
    proj
}

fn journal_row_count(db: &Path) -> usize {
    Registry::open(db).unwrap().journal_entries().unwrap().len()
}

#[test]
fn doctor_verdict_is_byte_equivalent_across_a_check_run() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = registered_project(tmp.path(), &db, "cross-doctor");
    let args = proj.display().to_string();

    let before = run(&db, &["--format", "json", "doctor", &args]);
    let check = run(&db, &["check", &args]);
    let after = run(&db, &["--format", "json", "doctor", &args]);

    assert!(
        check.status.success(),
        "check failed: {}",
        lossy(&check.stderr)
    );
    assert_eq!(
        lossy(&before.stdout),
        lossy(&after.stdout),
        "the doctor verdict must be byte-equivalent across a check run"
    );
}

#[test]
fn checker_never_writes_a_journal_row() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = registered_project(tmp.path(), &db, "cross-journal");
    let before = journal_row_count(&db);

    for _ in 0..3 {
        let out = run(&db, &["check", &proj.display().to_string()]);
        assert!(out.status.success(), "{}", lossy(&out.stderr));
    }
    let after = journal_row_count(&db);
    assert_eq!(before, after, "a checker run must not journal operations");
}

#[test]
fn mcp_tools_list_never_advertises_the_checker_surface() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = registered_project(tmp.path(), &db, "cross-mcp");

    let mcp_list = |id: i64| {
        let mut cmd = Command::new(forge_bin());
        cmd.env_remove("FORGE_REGISTRY")
            .arg("--registry")
            .arg(&db)
            .arg("mcp")
            .arg("serve");
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        let mut child = cmd.spawn().expect("spawn forge mcp serve");
        {
            let stdin = child.stdin.as_mut().expect("stdin");
            let request = serde_json::json!({
                "jsonrpc": "2.0", "id": id, "method": "tools/list"
            });
            writeln!(stdin, "{request}").expect("write request");
        }
        let output = child.wait_with_output().expect("wait");
        lossy(&output.stdout)
    };

    let check = run(&db, &["check", &proj.display().to_string()]);
    assert!(check.status.success(), "{}", lossy(&check.stderr));
    let before = mcp_list(1);
    let after = mcp_list(2);

    // The transport may interleave diagnostics; select the response by id.
    let tools_of = |stdout: &str, id: i64| -> Vec<String> {
        for line in stdout.lines().filter(|line| !line.trim().is_empty()) {
            let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
                continue;
            };
            if value["id"] == serde_json::json!(id) {
                return value["result"]["tools"]
                    .as_array()
                    .expect("tools array")
                    .iter()
                    .map(|tool| tool["name"].as_str().expect("tool name").to_string())
                    .collect();
            }
        }
        panic!("no tools/list response for id {id} in: {stdout}");
    };
    let names = tools_of(&before, 1);
    let after_names = tools_of(&after, 2);
    assert!(!names.contains(&"check".to_string()), "{names:?}");
    assert_eq!(
        names, after_names,
        "the tools/list snapshot must not grow through the checker surface"
    );
}

#[test]
fn inspect_and_governance_surfaces_are_unchanged_by_a_check_run() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = registered_project(tmp.path(), &db, "cross-surfaces");
    let args = proj.display().to_string();

    let inspect_before = run(&db, &["--format", "json", "inspect", &args]);
    let gov_before = run(&db, &["--format", "json", "governance", "status", &args]);
    assert!(inspect_before.status.success() && gov_before.status.success());

    let check = run(&db, &["check", &args]);
    assert!(check.status.success(), "{}", lossy(&check.stderr));

    let inspect_after = run(&db, &["--format", "json", "inspect", &args]);
    let gov_after = run(&db, &["--format", "json", "governance", "status", &args]);
    assert_eq!(
        lossy(&inspect_before.stdout),
        lossy(&inspect_after.stdout),
        "the inspect record must be byte-equivalent across a check run"
    );
    // The observation carries a wall-clock `observed_at`; every other
    // field must be identical, proving the check run changed nothing.
    let strip_time = |bytes: &[u8]| -> serde_json::Value {
        let mut value: serde_json::Value = serde_json::from_slice(bytes).unwrap();
        if let Some(obj) = value.as_object_mut() {
            for key in ["observation", "governance"] {
                if let Some(o) = obj.get_mut(key).and_then(|v| v.as_object_mut()) {
                    o.remove("observed_at");
                }
            }
        }
        value
    };
    assert_eq!(
        strip_time(&gov_before.stdout),
        strip_time(&gov_after.stdout),
        "the governance observation must be unchanged across a check run"
    );
}

#[test]
fn portal_and_feature_workflows_remain_compatible_after_a_check() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = registered_project(tmp.path(), &db, "cross-portal");
    let args = proj.display().to_string();

    assert!(run(&db, &["check", &args]).status.success());
    let dashboard = run(&db, &["--format", "json", "portal", "dashboard", &args]);
    assert!(
        dashboard.status.success(),
        "portal dashboard after check: {}",
        lossy(&dashboard.stderr)
    );
    let doc: serde_json::Value = serde_json::from_slice(&dashboard.stdout).unwrap();
    assert_eq!(doc["contract"], "0.1.0");
    assert!(!doc["dashboard"]["sections"].as_array().unwrap().is_empty());

    // A check run must not consume the feature workflow's handoff:
    // adding a compatible feature still works and updates the manifest.
    let add = run(&db, &["feature", "add", "auth", &args]);
    assert!(
        add.status.success(),
        "feature add after check: {}",
        lossy(&add.stderr)
    );
    let manifest = fs::read_to_string(proj.join("forge.yaml")).unwrap();
    assert!(manifest.contains("auth"), "{manifest}");
}
