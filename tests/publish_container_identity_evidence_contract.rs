//! Contract for `publish-container-identity-evidence`.
//!
//! One rule names the container identity a publish response proves: the
//! provider's own identity is the truth, the canonical
//! `forge-<project>-<sha12>` identity is composed only for a response
//! that published, and a response that published nothing carries no
//! identity at all.
//!
//! - a failed response without an identity carries no container;
//! - a successful response without an identity keeps the canonical one;
//! - a provider's own identity is kept, whatever the status says;
//! - the status projection renders a missing identity as `unknown`.
//!
//! Every case is driven through the real CLI binary against a staged
//! provider, the way the publish observability contract does.

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
        .env_remove("HTTP_PROXY")
        .env_remove("HTTPS_PROXY")
        .env_remove("ALL_PROXY")
        .env_remove("OPENAI_API_KEY")
        .env_remove("ANTHROPIC_API_KEY")
        .env_remove("FORGE_PUBLISH_PROVIDER_CONFIG")
        .env_remove("FORGE_PUBLISH_PROVIDER")
        .env_remove("FORGE_PUBLISH_PROVIDER_TIMEOUT_SECS");
    cmd
}

fn run_json(db: &Path, args: &[&str]) -> Value {
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(db);
    cmd.arg("--format").arg("json");
    for a in args {
        cmd.arg(a);
    }
    let out = cmd.output().expect("publish CLI json must run");
    serde_json::from_slice(&out.stdout).unwrap_or_else(|err| {
        panic!(
            "invalid json: {err}; stdout={} stderr={}",
            lossy(&out.stdout),
            lossy(&out.stderr)
        )
    })
}

fn write_minimal_project(dir: &Path, id: &str) {
    fs::create_dir_all(dir).unwrap();
    fs::write(
        dir.join("forge.yaml"),
        format!(
            "schema: 1\nproject:\n  id: {id}\n  name: {id}\n  profile: rust-web\n  maturity: L1\nruntime:\n  language: rust\n"
        ),
    )
    .unwrap();
    fs::write(dir.join("README.md"), "v1\n").unwrap();
}

fn init_git_repo(dir: &Path) {
    let _ = Command::new("git")
        .args(["init", "--initial-branch", "main"])
        .current_dir(dir)
        .output();
    let _ = Command::new("git")
        .args(["config", "user.email", "forge@example.com"])
        .current_dir(dir)
        .output();
    let _ = Command::new("git")
        .args(["config", "user.name", "Forge"])
        .current_dir(dir)
        .output();
    let _ = Command::new("git")
        .args(["add", "."])
        .current_dir(dir)
        .output();
    let _ = Command::new("git")
        .args(["commit", "-m", "init"])
        .current_dir(dir)
        .output();
}

/// A provider that reports the given status and health. It echoes the
/// revision it was asked for and reports a container identity only when
/// one was asked for.
fn provider(status: &str, health: &str, identity: bool) -> String {
    let identity_line = if identity {
        ",\n  \"container_identity\": \"forge-fixture-0123456789ab\""
    } else {
        ""
    };
    format!(
        r#"#!/bin/sh
set -e
cat > /dev/null
revision=$(cat /dev/null; echo "")
cat <<JSON
{{
  "contract": "forge-publish-provider/0.1.0",
  "provider": "fixture",
  "operation_id": "op-1",
  "status": "{status}",
  "health": "{health}",
  "evidence": ["fixture provider round-tripped the request"],
  "recovery": []
{identity_line}
}}
JSON
"#
    )
}

fn write_provider_config(project_dir: &Path, provider: &Path) {
    let forge_dir = project_dir.join(".forge");
    fs::create_dir_all(&forge_dir).unwrap();
    fs::write(
        forge_dir.join("providers.yaml"),
        format!(
            "providers:\n  - id: fixture\n    command: \"{}\"\n    enabled: true\n",
            provider.display()
        ),
    )
    .unwrap();
}

/// The publish row the CLI wrote.
fn publish_row(db: &Path) -> forge::registry::OperationEntry {
    let registry = forge::registry::Registry::open(db).unwrap();
    registry
        .journal_entries()
        .unwrap()
        .into_iter()
        .find(|entry| entry.kind == "publish")
        .expect("a publish row")
}

/// The publish entry the status projection shows.
fn projected_entry(db: &Path, project: &str) -> Value {
    let status = run_json(db, &["deploy", "status", "--project", project]);
    status["entries"]
        .as_array()
        .expect("entries array")
        .iter()
        .find(|entry| entry["kind"] == "publish")
        .cloned()
        .expect("publish entry")
}

const REVISION: &str = "0123456789abcdef0123456789abcdef01234567";

#[test]
fn a_response_that_published_nothing_carries_no_container() {
    // The provider failed and reported no identity: nothing was
    // deployed, so the row must not name a container.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("demo");
    write_minimal_project(&proj, "demo");
    init_git_repo(&proj);
    let scripts = tmp.path().join("scripts");
    fs::create_dir_all(&scripts).unwrap();
    let provider_path = scripts.join("fixture.sh");
    fs::write(&provider_path, provider("failed", "unhealthy", false)).unwrap();
    let mut perms = fs::metadata(&provider_path).unwrap().permissions();
    {
        use std::os::unix::fs::PermissionsExt;
        perms.set_mode(0o755);
    }
    fs::set_permissions(&provider_path, perms).unwrap();
    write_provider_config(&proj, &provider_path);

    let json = run_json(
        &db,
        &[
            "publish",
            "--folder",
            proj.to_str().unwrap(),
            "--provider",
            "fixture",
            "--revision",
            REVISION,
        ],
    );
    assert_eq!(json["status"], "failed");

    let row = publish_row(&db);
    assert_eq!(row.state, "failed", "the row keeps the provider's status");
    assert_eq!(
        row.container_identity, None,
        "a response that published nothing names no container"
    );

    let entry = projected_entry(&db, "demo");
    assert_eq!(
        entry["container_identity"], "unknown",
        "a missing identity renders as unknown, never as a verified success"
    );
}

#[test]
fn a_successful_response_keeps_the_canonical_identity() {
    // The legacy provider published and reported no identity: the
    // canonical identity is composed from the recorded revision, so the
    // provider is not silently absent from observability.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("demo");
    write_minimal_project(&proj, "demo");
    init_git_repo(&proj);
    let scripts = tmp.path().join("scripts");
    fs::create_dir_all(&scripts).unwrap();
    let provider_path = scripts.join("fixture.sh");
    fs::write(&provider_path, provider("done", "healthy", false)).unwrap();
    let mut perms = fs::metadata(&provider_path).unwrap().permissions();
    {
        use std::os::unix::fs::PermissionsExt;
        perms.set_mode(0o755);
    }
    fs::set_permissions(&provider_path, perms).unwrap();
    write_provider_config(&proj, &provider_path);

    let json = run_json(
        &db,
        &[
            "publish",
            "--folder",
            proj.to_str().unwrap(),
            "--provider",
            "fixture",
            "--revision",
            REVISION,
        ],
    );
    assert_eq!(json["status"], "done");

    let row = publish_row(&db);
    assert_eq!(
        row.container_identity.as_deref(),
        Some("forge-demo-0123456789ab"),
        "a published run keeps the canonical identity"
    );

    let entry = projected_entry(&db, "demo");
    assert_eq!(entry["container_identity"], "forge-demo-0123456789ab");
}

#[test]
fn the_providers_own_identity_is_always_kept() {
    // The provider's own report is the truth: a provider that names a
    // container keeps that name, whatever its status says.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("demo");
    write_minimal_project(&proj, "demo");
    init_git_repo(&proj);
    let scripts = tmp.path().join("scripts");
    fs::create_dir_all(&scripts).unwrap();
    let provider_path = scripts.join("fixture.sh");
    fs::write(&provider_path, provider("failed", "unhealthy", true)).unwrap();
    let mut perms = fs::metadata(&provider_path).unwrap().permissions();
    {
        use std::os::unix::fs::PermissionsExt;
        perms.set_mode(0o755);
    }
    fs::set_permissions(&provider_path, perms).unwrap();
    write_provider_config(&proj, &provider_path);

    let _json = run_json(
        &db,
        &[
            "publish",
            "--folder",
            proj.to_str().unwrap(),
            "--provider",
            "fixture",
            "--revision",
            REVISION,
        ],
    );

    let row = publish_row(&db);
    assert_eq!(
        row.container_identity.as_deref(),
        Some("forge-fixture-0123456789ab"),
        "the provider's own identity is the truth"
    );
}
