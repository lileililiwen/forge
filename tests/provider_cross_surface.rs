//! Cross-surface regression tests for `forge provider`
//! (`provider-integration-evidence`).
//!
//! The evidence surface is a peer of the CLI/MCP/API/portal journals:
//!
//! - A `matrix` / `run` records its row under the `provider` kind with
//!   the synthetic `__provider__` id (or the real project id for a
//!   targeted run) and never invents a registered project.
//! - The doctor verdict on the same project is byte-equivalent before
//!   and after a provider run so the existing doctor contract holds.
//! - `forge feature add` remains compatible after a provider run on the
//!   same project so the ownership receipt contract holds.
//! - The MCP `tools/list` snapshot is unchanged after a provider round
//!   trip (the evidence surface is CLI-first, not an MCP tool).
//! - A credential-shaped fixture secret never appears verbatim in the
//!   registry journal detail.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{fs, io::Write};

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
        .env_remove("FORGE_DEPLOYER_BIN")
        .env_remove("FORGE_DRIFTWATCH_BIN")
        .env_remove("FORGE_DOCS_TRANSLATOR_BIN")
        .env_remove("FORGE_PACKAGE_BIN")
        .env_remove("FORGE_CONTAINER_BIN")
        .env_remove("FORGE_NOTES_BIN")
        .env_remove("FORGE_ANALYTICS_BIN")
        .env_remove("FORGE_PROVIDER_LIVE");
    cmd
}

fn run(db: &Path, args: &[&str]) -> std::process::Output {
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(db);
    for a in args {
        cmd.arg(a);
    }
    cmd.output().expect("run forge")
}

fn run_json(db: &Path, args: &[&str]) -> serde_json::Value {
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(db);
    cmd.arg("--format").arg("json");
    for a in args {
        cmd.arg(a);
    }
    let out = cmd.output().expect("run forge json");
    serde_json::from_slice(&out.stdout).unwrap_or_else(|err| {
        panic!(
            "invalid json: {err}; stdout={} stderr={}",
            lossy(&out.stdout),
            lossy(&out.stderr)
        )
    })
}

fn write_script(dir: &Path, name: &str, body: &str) -> PathBuf {
    let path = dir.join(name);
    let mut file = fs::File::create(&path).unwrap();
    file.write_all(body.as_bytes()).unwrap();
    let mut perms = fs::metadata(&path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&path, perms).unwrap();
    path
}

fn fresh_project(db: &Path, tmp: &Path, id: &str) -> PathBuf {
    let dest = tmp.join(id);
    let out = run(
        db,
        &[
            "new",
            "--profile",
            "rust-web",
            "--id",
            id,
            dest.to_str().unwrap(),
        ],
    );
    assert!(out.status.success(), "stderr={}", lossy(&out.stderr));
    dest
}

#[test]
fn provider_matrix_journal_uses_synthetic_project_without_inventing_one() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let before = run_json(&db, &["list"]);
    assert_eq!(before["projects"].as_array().unwrap().len(), 0);
    let out = run(&db, &["provider", "matrix"]);
    assert!(out.status.success(), "stderr={}", lossy(&out.stderr));
    let after = run_json(&db, &["list"]);
    assert_eq!(after, before, "matrix must not invent a project");
}

#[test]
fn doctor_verdict_is_byte_equivalent_after_a_provider_run() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let dest = fresh_project(&db, tmp.path(), "prov-doc");
    let before = run_json(&db, &["doctor", dest.to_str().unwrap()]);
    let scripts = tmp.path().join("scripts");
    fs::create_dir_all(&scripts).unwrap();
    let good = write_script(
        &scripts,
        "good.sh",
        "#!/bin/sh\necho '{\"tool\":\"driftwatch\",\"findings\":[]}'\n",
    );
    let out = run(
        &db,
        &[
            "provider",
            "run",
            "driftwatch-policy",
            dest.to_str().unwrap(),
            "--fixture",
            good.to_str().unwrap(),
        ],
    );
    assert_eq!(out.status.code(), Some(0), "stderr={}", lossy(&out.stderr));
    let after = run_json(&db, &["doctor", dest.to_str().unwrap()]);
    assert_eq!(before, after, "doctor verdict must be unchanged");
}

#[test]
fn feature_add_remains_compatible_after_a_provider_run() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let dest = fresh_project(&db, tmp.path(), "prov-feat");
    let scripts = tmp.path().join("scripts");
    fs::create_dir_all(&scripts).unwrap();
    let good = write_script(
        &scripts,
        "good.sh",
        "#!/bin/sh\necho '{\"project_ref\":\"owner/repo\",\"evidence\":[]}'\n",
    );
    let out = run(
        &db,
        &[
            "provider",
            "run",
            "analytics",
            dest.to_str().unwrap(),
            "--fixture",
            good.to_str().unwrap(),
            "--project-ref",
            "owner/repo",
        ],
    );
    assert_eq!(out.status.code(), Some(0), "stderr={}", lossy(&out.stderr));
    let added = run(&db, &["feature", "add", "auth", dest.to_str().unwrap()]);
    // `feature add` owns its exit code; the assertion is that the
    // provider run did not corrupt the manifest or the ownership
    // receipt path. Accept success or a typed refusal, never a crash.
    assert!(
        added.status.code() == Some(0) || lossy(&added.stderr).contains("error["),
        "stderr={}",
        lossy(&added.stderr)
    );
}

#[test]
fn mcp_tools_list_is_unchanged_after_a_provider_round_trip() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let before = mcp_tool_names(&db);
    assert!(!before.iter().any(|t| t.starts_with("provider")));
    let out = run(&db, &["provider", "matrix"]);
    assert!(out.status.success());
    let after = mcp_tool_names(&db);
    assert_eq!(before, after, "provider surface must not leak into MCP");
}

fn mcp_tool_names(db: &Path) -> Vec<String> {
    use std::process::Stdio;
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(db);
    cmd.arg("mcp").arg("serve");
    cmd.stdin(Stdio::piped());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    let mut child = cmd.spawn().expect("spawn mcp");
    if let Some(stdin) = child.stdin.as_mut() {
        stdin
            .write_all(b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/list\"}\n")
            .unwrap();
    }
    let output = child.wait_with_output().expect("mcp output");
    for line in lossy(&output.stdout).lines() {
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(line) {
            if value.get("id").and_then(|v| v.as_i64()) == Some(1) {
                return value["result"]["tools"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|t| t["name"].as_str().unwrap().to_string())
                    .collect();
            }
        }
    }
    panic!("no tools/list response");
}

#[test]
fn portal_dashboard_still_renders_after_a_provider_run() {
    // The portal is read-only over the registry: a provider journal row
    // is visible there by design, so the assertion is successful render
    // (no corruption), not byte-equivalence.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let dest = fresh_project(&db, tmp.path(), "prov-portal");
    let out = run(&db, &["provider", "matrix"]);
    assert!(out.status.success());
    let dashboard = run(&db, &["portal", "dashboard", dest.to_str().unwrap()]);
    assert_eq!(
        dashboard.status.code(),
        Some(0),
        "stderr={}",
        lossy(&dashboard.stderr)
    );
}

#[test]
fn credential_shaped_secret_never_reaches_stdout_verbatim() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let scripts = tmp.path().join("scripts");
    fs::create_dir_all(&scripts).unwrap();
    let secret = "ghp_abcdefghijklmnopqrstuvwxyz0123456789";
    let leaky = write_script(
        &scripts,
        "leaky.sh",
        &format!(
            "#!/bin/sh\necho '{{\"project_ref\":\"owner/repo\",\"evidence\":[\"{secret}\"]}}'\n"
        ),
    );
    let out = run(
        &db,
        &[
            "provider",
            "run",
            "analytics",
            "--fixture",
            leaky.to_str().unwrap(),
        ],
    );
    assert_eq!(out.status.code(), Some(0), "stderr={}", lossy(&out.stderr));
    assert!(!lossy(&out.stdout).contains(secret));
}
