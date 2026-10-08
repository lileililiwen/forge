//! Plugin registry contract: `forge plugins list` (`forge-plugins/0.1.0`).
//!
//! This suite answers the questions the capability names:
//!
//! - A configured plugin with no descriptor is a **delivery** plugin,
//!   because that is the shape every existing `providers.yaml` has. No
//!   existing configuration needs editing to appear in the registry.
//! - A descriptor adds `kind`, `capabilities` and `description`, so the
//!   registry can answer "which plugin maintains metadata" without a
//!   hard-coded id.
//! - The capability vocabulary is **closed**. A descriptor naming
//!   anything outside it is reported `invalid` and advertises nothing,
//!   because an operator cannot tell which of a registry's claims are
//!   trustworthy if some of them are invented.
//! - One broken plugin does not hide the others: a missing command is
//!   `unavailable` with its reason, and the remaining plugins still list.
//! - An absent config is an honest empty registry, not a failure.
//!
//! No plugin is invoked. This is a read of the operator's own
//! configuration file.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

const CONTRACT: &str = "forge-plugins/0.1.0";

fn forge_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_forge"))
}

fn run(dir: &Path, args: &[&str]) -> std::process::Output {
    let mut cmd = Command::new(forge_bin());
    cmd.env_remove("FORGE_REGISTRY");
    cmd.arg("--registry").arg(dir.join("registry.db"));
    for arg in args {
        cmd.arg(arg);
    }
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
    serde_json::from_slice(&out.stdout).expect("plugins json")
}

/// Write a workspace whose `.forge/providers.yaml` carries `body`.
fn workspace(tmp: &Path, body: &str) -> PathBuf {
    let dir = tmp.to_path_buf();
    fs::create_dir_all(dir.join(".forge")).unwrap();
    fs::write(dir.join(".forge/providers.yaml"), body).unwrap();
    dir
}

fn plugin<'a>(page: &'a Value, id: &str) -> &'a Value {
    page["plugins"]
        .as_array()
        .expect("plugins array")
        .iter()
        .find(|p| p["id"] == id)
        .unwrap_or_else(|| panic!("no plugin `{id}` in {page}"))
}

/// The shape that exists in the wild: `{id, command, enabled}` and no
/// descriptor at all.
#[test]
fn a_configured_plugin_without_a_descriptor_is_a_delivery_plugin() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = workspace(
        tmp.path(),
        "providers:\n  - id: openpanel\n    command: /bin/true\n    enabled: true\n",
    );

    let page = run_json(&dir, &["plugins", "list", "--format", "json"]);
    assert_eq!(page["contract"], CONTRACT);
    let entry = plugin(&page, "openpanel");
    assert_eq!(entry["kind"], "delivery");
    assert_eq!(entry["state"], "ready");
    assert_eq!(entry["capabilities"], serde_json::json!(["delivery"]));
    assert_eq!(entry["reason"], Value::Null);
}

/// A descriptor is what turns a plugin into a metadata maintainer, and
/// it declares exactly which fields it can own.
#[test]
fn a_metadata_descriptor_declares_the_fields_the_plugin_maintains() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = workspace(
        tmp.path(),
        r#"providers:
  - id: github
    command: /bin/true
    enabled: true
plugins:
  - id: github
    kind: metadata
    capabilities: [topic, description]
    description: Maintains the repository description and topics.
"#,
    );

    let page = run_json(&dir, &["plugins", "list", "--format", "json"]);
    let entry = plugin(&page, "github");
    assert_eq!(entry["kind"], "metadata");
    assert_eq!(entry["state"], "ready");
    // Sorted, so the render is stable across runs.
    assert_eq!(
        entry["capabilities"],
        serde_json::json!(["description", "topic"])
    );
    assert_eq!(
        entry["description"],
        "Maintains the repository description and topics."
    );
}

/// The closed vocabulary is the point of the registry: an operator must
/// be able to trust that an advertised capability means something.
#[test]
fn a_capability_outside_the_closed_set_is_invalid_and_advertises_nothing() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = workspace(
        tmp.path(),
        r#"providers:
  - id: github
    command: /bin/true
    enabled: true
plugins:
  - id: github
    kind: metadata
    capabilities: [topic, teleport]
"#,
    );

    let page = run_json(&dir, &["plugins", "list", "--format", "json"]);
    let entry = plugin(&page, "github");
    assert_eq!(entry["state"], "invalid");
    assert_eq!(entry["capabilities"], serde_json::json!([]));
    let reason = entry["reason"].as_str().expect("reason");
    assert!(reason.contains("teleport"), "{reason}");
    assert!(reason.contains("closed set"), "{reason}");
}

/// An unknown kind is refused for the same reason as an unknown
/// capability.
#[test]
fn an_unknown_kind_is_invalid() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = workspace(
        tmp.path(),
        r#"providers:
  - id: weird
    command: /bin/true
    enabled: true
plugins:
  - id: weird
    kind: teleport
"#,
    );

    let page = run_json(&dir, &["plugins", "list", "--format", "json"]);
    let entry = plugin(&page, "weird");
    assert_eq!(entry["state"], "invalid");
    assert!(entry["reason"]
        .as_str()
        .unwrap()
        .contains("unknown plugin kind"));
}

/// One broken entry must not hide the rest of the workspace's reach.
#[test]
fn a_missing_command_does_not_hide_the_other_plugins() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = workspace(
        tmp.path(),
        r#"providers:
  - id: broken
    command: /nonexistent/plugin
    enabled: true
  - id: working
    command: /bin/true
    enabled: true
"#,
    );

    let page = run_json(&dir, &["plugins", "list", "--format", "json"]);
    assert_eq!(page["plugins"].as_array().unwrap().len(), 2);
    let broken = plugin(&page, "broken");
    assert_eq!(broken["state"], "unavailable");
    assert!(broken["reason"]
        .as_str()
        .unwrap()
        .contains("/nonexistent/plugin"));
    // The healthy plugin is still listed and still ready.
    assert_eq!(plugin(&page, "working")["state"], "ready");
}

/// A disabled plugin is the operator's choice, not a fault, and is
/// reported as such.
#[test]
fn a_disabled_plugin_is_reported_disabled() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = workspace(
        tmp.path(),
        "providers:\n  - id: off\n    command: /bin/true\n    enabled: false\n",
    );

    let page = run_json(&dir, &["plugins", "list", "--format", "json"]);
    let entry = plugin(&page, "off");
    assert_eq!(entry["state"], "disabled");
    assert_eq!(entry["enabled"], false);
    assert_eq!(entry["reason"], Value::Null);
}

/// No config is a real answer: "this workspace reaches nothing yet" is
/// exactly what an operator with no plugins needs to be told.
#[test]
fn an_absent_config_is_an_honest_empty_registry() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().to_path_buf();
    fs::create_dir_all(&dir).unwrap();

    let page = run_json(&dir, &["plugins", "list", "--format", "json"]);
    assert_eq!(page["contract"], CONTRACT);
    assert_eq!(page["plugins"], serde_json::json!([]));

    // And the human form says what to do rather than failing.
    let out = run(&dir, &["plugins", "list"]);
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("No plugins configured"), "{text}");
    assert!(text.contains("delivery plugin"), "{text}");
}

/// Both plugin kinds coexist in one workspace — the point of the
/// registry is that a GitHub maintainer and an OpenPanel deployer are
/// the same kind of thing.
#[test]
fn metadata_and_delivery_plugins_coexist_and_are_listed_in_id_order() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = workspace(
        tmp.path(),
        r#"providers:
  - id: openpanel
    command: /bin/true
    enabled: true
  - id: github
    command: /bin/true
    enabled: true
plugins:
  - id: github
    kind: metadata
    capabilities: [topic, description, homepage, language]
"#,
    );

    let page = run_json(&dir, &["plugins", "list", "--format", "json"]);
    let ids: Vec<&str> = page["plugins"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, vec!["github", "openpanel"]);
    assert_eq!(plugin(&page, "github")["kind"], "metadata");
    assert_eq!(plugin(&page, "openpanel")["kind"], "delivery");

    // The human render names both kinds and both states.
    let out = run(&dir, &["plugins", "list"]);
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("github  metadata  ready"), "{text}");
    assert!(text.contains("openpanel  delivery  ready"), "{text}");
}

/// The registry must never claim a plugin can maintain a field it did
/// not declare — this is what `classify apply` routes on.
#[test]
fn capabilities_are_reported_exactly_as_declared() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = workspace(
        tmp.path(),
        r#"providers:
  - id: partial
    command: /bin/true
    enabled: true
plugins:
  - id: partial
    kind: metadata
    capabilities: [description]
"#,
    );

    let page = run_json(&dir, &["plugins", "list", "--format", "json"]);
    let entry = plugin(&page, "partial");
    // Only `description`. A plugin that cannot maintain topics must not be
    // asked to.
    assert_eq!(entry["capabilities"], serde_json::json!(["description"]));
}
