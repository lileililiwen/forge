//! `forge classify apply` contract: approved metadata reaches the
//! configured metadata plugin as a `forge-metadata-propose/0.1.0`
//! request in PR mode, and nothing else.
//!
//! This suite answers the questions the capability names:
//!
//! - A `Suggested` proposal is refused by name; nothing is sent
//!   to any plugin.
//! - An approved set with no metadata plugin is refused, naming
//!   the missing capability.
//! - An approved value outside the four permitted fields is
//!   refused, naming the field.
//! - An approved set with a metadata plugin produces a
//!   `forge-metadata-propose/0.1.0` request in `mode: "pr"` and a
//!   journal row naming plugin, fields, PR reference.
//! - `apply` never mutates a remote directly: the request goes
//!   through the plugin command, not through any direct GitHub
//!   call.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

fn forge_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_forge"))
}

fn run(dir: &Path, args: &[&str]) -> std::process::Output {
    let mut cmd = Command::new(forge_bin());
    cmd.env_remove("FORGE_REGISTRY");
    cmd.env_remove("FORGE_WORKSPACE_REGISTRY");
    cmd.env_remove("FORGE_INVENTORY_SOURCE");
    cmd.env_remove("FORGE_PUBLISH_PROVIDER_CONFIG");
    cmd.env_remove("HTTP_PROXY");
    cmd.env_remove("HTTPS_PROXY");
    cmd.env_remove("ALL_PROXY");
    cmd.arg("--registry").arg(dir.join("registry.db"));
    for arg in args {
        cmd.arg(arg);
    }
    cmd.env("FORGE_STUB_REQUEST_OUT", dir.join("stub-request.json"));
    cmd.current_dir(dir);
    cmd.output().expect("run forge")
}

fn run_json(dir: &Path, args: &[&str]) -> Value {
    let out = run(dir, args);
    assert!(
        out.status.success(),
        "{:?} failed: {}",
        args,
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).expect("json")
}

fn write_project(dir: &Path) {
    fs::write(
        dir.join("forge.yaml"),
        "schema: 1\nproject:\n  id: demo\n  name: Demo Project\n  profile: rust-web\n  maturity: L1\n  target_maturity: L1\nruntime:\n  language: rust\n",
    )
    .unwrap();
    fs::write(dir.join("README.md"), "# Demo Project\n\nbody\n").unwrap();
}

/// Create a `description` proposal and approve it.
fn approve_description(dir: &Path, value: &str) {
    let out = run(
        dir,
        &[
            "describe",
            "suggest",
            "--suggested-value",
            value,
            "--evidence-path",
            "README.md",
            "--evidence-revision",
            "rev1",
        ],
    );
    assert!(
        out.status.success(),
        "suggest failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let page = run_json(dir, &["describe", "list", "--format", "json"]);
    let id = page["proposals"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["kind"] == "description")
        .expect("description proposal")["dir_name"]
        .as_str()
        .unwrap()
        .to_string();
    let out = run(dir, &["describe", "approve", &id, "--confirm"]);
    assert!(
        out.status.success(),
        "approve failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// Create a `domain` proposal and approve it.
fn approve_domain(dir: &Path, value: &str) {
    let out = run(
        dir,
        &[
            "classify",
            "suggest",
            "--suggested-value",
            value,
            "--evidence-path",
            "README.md",
            "--evidence-revision",
            "rev1",
        ],
    );
    assert!(
        out.status.success(),
        "suggest failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let page = run_json(dir, &["classify", "list", "--format", "json"]);
    let id = page["proposals"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["kind"] == "domain")
        .expect("domain proposal")["dir_name"]
        .as_str()
        .unwrap()
        .to_string();
    let out = run(dir, &["classify", "approve", &id, "--confirm"]);
    assert!(
        out.status.success(),
        "approve failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// Write a stub metadata plugin: a shell script that records the
/// request it received to `$FORGE_STUB_REQUEST_OUT` and answers
/// with a canned `forge-metadata-propose/0.1.0` response.
fn write_stub_plugin(dir: &Path) -> PathBuf {
    let script = dir.join("stub-plugin.sh");
    fs::write(
        &script,
        "#!/bin/sh\ncat > \"$FORGE_STUB_REQUEST_OUT\"\ncat <<'EOF'\n{\"contract\":\"forge-metadata-propose/0.1.0\",\"operation_id\":\"op-1\",\"status\":\"proposed\",\"changes\":[{\"field\":\"description\",\"new_value\":\"Demo Project\"}],\"pr\":\"https://example.invalid/pull/1\",\"note\":\"stub\"}\nEOF\n",
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
    }
    script
}

fn write_providers(dir: &Path, body: &str) {
    fs::create_dir_all(dir.join(".forge")).unwrap();
    fs::write(dir.join(".forge/providers.yaml"), body).unwrap();
}

#[test]
fn a_suggested_proposal_is_refused_by_name_and_nothing_is_sent() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    write_project(dir);
    let out = run(
        dir,
        &[
            "describe",
            "suggest",
            "--suggested-value",
            "demo",
            "--evidence-path",
            "README.md",
            "--evidence-revision",
            "rev1",
        ],
    );
    assert!(out.status.success());

    let out = run(dir, &["classify", "apply", "--confirm"]);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("suggested"), "{stderr}");
    assert!(stderr.contains("description"), "{stderr}");
    assert!(
        !dir.join("stub-request.json").exists(),
        "nothing may be sent to any plugin"
    );
}

#[test]
fn an_approved_set_with_no_metadata_plugin_is_refused_naming_the_capability() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    write_project(dir);
    approve_description(dir, "demo");
    write_providers(
        dir,
        "providers:\n  - id: openpanel\n    command: /bin/true\n    enabled: true\n",
    );

    let out = run(dir, &["classify", "apply", "--confirm"]);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("metadata"), "{stderr}");
    assert!(!dir.join("stub-request.json").exists());
}

#[test]
fn an_approved_value_outside_the_permitted_fields_is_refused_naming_the_field() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    write_project(dir);
    approve_domain(dir, "tools");

    let out = run(dir, &["classify", "apply", "--confirm"]);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("domain"), "{stderr}");
    assert!(!dir.join("stub-request.json").exists());
}

#[test]
fn an_approved_set_with_a_metadata_plugin_produces_a_pr_mode_request_and_a_journal_row() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    write_project(dir);
    approve_description(dir, "Demo Project");
    let stub = write_stub_plugin(dir);
    write_providers(
        dir,
        &format!(
            "providers:\n  - id: github\n    command: {}\n    enabled: true\nplugins:\n  - id: github\n    kind: metadata\n    capabilities: [topic, description, homepage, language]\n",
            stub.display()
        ),
    );

    let page = run_json(dir, &["classify", "apply", "--confirm", "--format", "json"]);
    assert_eq!(page["contract"], "forge-metadata-propose/0.1.0");
    assert_eq!(page["plugin"], "github");
    assert_eq!(page["fields"], serde_json::json!(["description"]));
    assert_eq!(page["pr"], "https://example.invalid/pull/1");

    // The request the plugin received is the PR-mode metadata
    // propose request.
    let request: Value =
        serde_json::from_slice(&fs::read(dir.join("stub-request.json")).unwrap()).unwrap();
    assert_eq!(request["contract"], "forge-metadata-propose/0.1.0");
    assert_eq!(request["mode"], "pr");
    assert_eq!(request["project_id"], "demo");
    assert_eq!(request["fields"]["description"], "Demo Project");

    // The journal row names the plugin, the fields, and the PR
    // reference — no path, no credential.
    let conn = rusqlite::Connection::open(dir.join("registry.db")).unwrap();
    let (kind, project_id, state, detail): (String, String, String, String) = conn
        .query_row(
            "SELECT kind, project_id, state, detail FROM operations WHERE kind = 'classify.apply'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .unwrap();
    assert_eq!(kind, "classify.apply");
    assert_eq!(project_id, "demo");
    assert_eq!(state, "succeeded");
    assert!(detail.contains("plugin=github"), "{detail}");
    assert!(detail.contains("fields=description"), "{detail}");
    assert!(
        detail.contains("pr=https://example.invalid/pull/1"),
        "{detail}"
    );
}

#[test]
fn apply_never_mutates_a_remote_directly() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    write_project(dir);
    approve_description(dir, "Demo Project");
    let stub = write_stub_plugin(dir);
    write_providers(
        dir,
        &format!(
            "providers:\n  - id: github\n    command: {}\n    enabled: true\nplugins:\n  - id: github\n    kind: metadata\n    capabilities: [topic, description, homepage, language]\n",
            stub.display()
        ),
    );

    let page = run_json(dir, &["classify", "apply", "--confirm", "--format", "json"]);
    assert_eq!(page["plugin"], "github");
    // The only evidence of the publish is the request file the
    // stub plugin wrote: the request went through the plugin
    // command, not through any direct GitHub call.
    let request: Value =
        serde_json::from_slice(&fs::read(dir.join("stub-request.json")).unwrap()).unwrap();
    assert_eq!(request["contract"], "forge-metadata-propose/0.1.0");
    assert_eq!(request["mode"], "pr");
}
