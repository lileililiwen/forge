//! CLI contract for `portal-policy-verdict-truth`.
//!
//! The control-plane portal states a policy verdict only when it
//! observed one. A project whose policy plane was never evaluated is
//! `unknown`, never `ok`; a project whose directory cannot be read is
//! `unavailable`. The Policies entry names what the portal looked at —
//! the DriftWatch policy artifacts in the project directory — and the
//! doctor and the portal read the same artifact list.
//!
//! - a project with a DriftWatch configuration reports `unknown` and
//!   names the configuration it found;
//! - a project with no DriftWatch configuration reports `unknown` and
//!   says so;
//! - a project whose directory cannot be read reports `unavailable`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

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
        .env_remove("ANTHROPIC_API_KEY");
    cmd
}

fn run(db: &Path, args: &[&str]) -> std::process::Output {
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(db);
    cmd.arg("--format").arg("json");
    for a in args {
        cmd.arg(a);
    }
    cmd.output().expect("run forge")
}

/// A registered project with a DriftWatch configuration.
fn write_project(dir: &Path, id: &str, policy: Option<&str>) {
    fs::create_dir_all(dir).unwrap();
    let text = format!(
        "schema: 1\nproject:\n  id: {id}\n  name: {id}\n  profile: rust-web\n  maturity: L1\nruntime:\n  language: rust\n"
    );
    fs::write(dir.join("forge.yaml"), text).unwrap();
    fs::write(dir.join("README.md"), "v1\n").unwrap();
    if let Some(policy) = policy {
        fs::write(dir.join(policy), "rules: []\n").unwrap();
    }
}

/// The Policies entry of the portal's policies view.
fn policies_entry(db: &Path, project_id: &str) -> serde_json::Value {
    let out = run(db, &["portal", "view", "policies", project_id]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "the policies view exits 0; stderr={}",
        lossy(&out.stderr)
    );
    let value: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("the policies view is a JSON document");
    let entries = value["view"]["entries"]
        .as_array()
        .expect("the view carries entries")
        .clone();
    assert_eq!(entries.len(), 1, "{entries:?}");
    entries[0].clone()
}

fn evidence(entry: &serde_json::Value) -> Vec<String> {
    entry["evidence"]
        .as_array()
        .map(|lines| {
            lines
                .iter()
                .map(|line| line.as_str().unwrap_or_default().to_string())
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn a_policy_that_was_never_evaluated_is_unknown() {
    // The project has a DriftWatch configuration, so the portal can say
    // which one it found — but nothing evaluated it, so the section is
    // `unknown`, not `ok`.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_project(&proj, "portal-policy", Some("driftwatch.yaml"));

    let out = run(&db, &["register", proj.to_str().unwrap()]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "the project registers; stderr={}",
        lossy(&out.stderr)
    );

    let entry = policies_entry(&db, "portal-policy");
    assert_eq!(entry["status"], "unknown", "{entry}");
    let lines = evidence(&entry);
    assert!(
        lines
            .iter()
            .any(|line| line.contains("unverified is not a pass")),
        "the entry says an unevaluated policy is not a pass: {lines:?}"
    );
    assert!(
        lines.iter().any(|line| line.contains("forge doctor")),
        "the entry names the command that produces a verdict: {lines:?}"
    );
    assert!(
        lines.iter().any(|line| line.contains("driftwatch.yaml")),
        "the entry names the artifact the portal found: {lines:?}"
    );
}

#[test]
fn a_project_with_no_policy_configuration_is_unknown() {
    // No configuration at all: the section says so and stays `unknown`.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_project(&proj, "portal-no-policy", None);

    let out = run(&db, &["register", proj.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));

    let entry = policies_entry(&db, "portal-no-policy");
    assert_eq!(entry["status"], "unknown", "{entry}");
    let lines = evidence(&entry);
    assert!(
        lines
            .iter()
            .any(|line| line.contains("no driftwatch configuration")),
        "the entry names what the portal looked at: {lines:?}"
    );
}

#[test]
fn a_project_whose_directory_cannot_be_read_is_unavailable() {
    // The portal could not observe the project, so it says so instead
    // of reporting a verdict it never read.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proj");
    write_project(&proj, "portal-gone", Some("driftwatch.yaml"));

    let out = run(&db, &["register", proj.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    fs::remove_dir_all(&proj).expect("remove the project directory");

    let entry = policies_entry(&db, "portal-gone");
    assert_eq!(entry["status"], "unavailable", "{entry}");
    let lines = evidence(&entry);
    assert!(
        lines.iter().any(|line| line.contains("could not be read")),
        "the entry says the policy plane was not observed: {lines:?}"
    );
}
