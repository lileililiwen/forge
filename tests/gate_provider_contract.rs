//! Gate-runtime provider row contract (`gate-runtime-evidence`).
//!
//! The `gate-runtime` matrix row honors the existing opt-in rules: it
//! reports `not-run` unless live, never claims a gate pass through a
//! probe, exercises only the side-effect-free `gate --dry-run` surface,
//! and classifies a responding plan, a parseable status document and a
//! refused invocation distinctly.

use std::borrow::Cow;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

fn forge_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_forge"))
}

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/gate")
        .join(name)
}

fn lossy(bytes: &[u8]) -> Cow<'_, str> {
    String::from_utf8_lossy(bytes)
}

fn write_script(dir: &Path, name: &str, body: &str) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    path
}

fn provider_run(db: &Path, args: &[&str]) -> std::process::Output {
    let mut cmd = Command::new(forge_bin());
    cmd.env_remove("FORGE_GATE_BIN")
        .env_remove("FORGE_DRIFTWATCH_BIN")
        .env_remove("FORGE_PROVIDER_LIVE")
        .env_remove("FORGE_REGISTRY");
    cmd.arg("--registry")
        .arg(db)
        .arg("--format")
        .arg("json")
        .arg("provider")
        .arg("run");
    for arg in args {
        cmd.arg(arg);
    }
    cmd.output().expect("run forge provider run")
}

fn provider_json(out: &std::process::Output) -> serde_json::Value {
    assert!(out.status.success(), "stderr: {}", lossy(&out.stderr));
    serde_json::from_slice(&out.stdout).expect("provider json")
}

#[test]
fn matrix_lists_gate_runtime_as_not_run_without_live() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let out = Command::new(forge_bin())
        .env_remove("FORGE_GATE_BIN")
        .env_remove("FORGE_PROVIDER_LIVE")
        .env_remove("FORGE_REGISTRY")
        .arg("--registry")
        .arg(&db)
        .arg("--format")
        .arg("json")
        .arg("provider")
        .arg("matrix")
        .output()
        .expect("matrix");
    let value: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let rows = value["matrix"]["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 6, "six providers in stable order");
    let ids: Vec<&str> = rows
        .iter()
        .map(|r| r["provider"].as_str().unwrap_or(""))
        .collect();
    assert_eq!(
        ids,
        vec![
            "driftwatch-policy",
            "gate-runtime",
            "oidc-identity",
            "analytics",
            "deploy",
            "release",
        ]
    );
    let row = rows
        .iter()
        .find(|r| r["provider"] == "gate-runtime")
        .unwrap();
    assert_eq!(row["status"], "not-run", "{row}");
    assert!(row["reason"]
        .as_str()
        .unwrap_or("")
        .contains("forge provider run gate-runtime"));
}

#[test]
fn inspect_gate_runtime_names_the_override_and_plan_surface() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let out = Command::new(forge_bin())
        .env_remove("FORGE_REGISTRY")
        .arg("--registry")
        .arg(&db)
        .arg("--format")
        .arg("json")
        .arg("provider")
        .arg("inspect")
        .arg("gate-runtime")
        .output()
        .expect("inspect");
    let value: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let descriptor = &value["descriptor"];
    assert_eq!(descriptor["provider"], "gate-runtime");
    assert_eq!(descriptor["binary_env"], "FORGE_GATE_BIN");
    assert_eq!(descriptor["default_binary"], "driftwatchdog|driftwatch");
    let boundary = descriptor["boundary"].as_str().unwrap_or("");
    assert!(boundary.contains("gate --dry-run"), "{boundary}");
    assert!(
        boundary.contains("never claims a gate pass") || boundary.contains("no gate pass"),
        "{boundary}"
    );
}

#[test]
fn fixture_plan_response_is_supported_and_claims_no_pass() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let plan = fixture("gate-dryrun-plan.txt").display().to_string();
    let stub = write_script(
        tmp.path(),
        "gate-plan.sh",
        &format!(
            "if [ \"$1\" = \"--version\" ]; then echo 'driftwatch 0.1.0'; exit 0; fi\ncat '{plan}'\nexit 0\n"
        ),
    );
    let out = provider_run(
        &db,
        &["gate-runtime", "--fixture", &stub.display().to_string()],
    );
    let value = provider_json(&out);
    let row = &value["run"];
    assert_eq!(row["status"], "supported", "{row}");
    assert_eq!(row["provenance"]["sandbox"], "fixture");
    let text = serde_json::to_string(&row).unwrap();
    // Attribution of the plan preview; never a gate pass claim.
    assert!(text.contains("gate-dry-run"), "{text}");
    assert!(!text.contains("\"aggregate\":\"passed\""), "{text}");
}

#[test]
fn fixture_parseable_gate_document_classifies_its_own_aggregate() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let doc = fixture("gate-status-blocked.json").display().to_string();
    let stub = write_script(
        tmp.path(),
        "gate-doc.sh",
        &format!(
            "if [ \"$1\" = \"--version\" ]; then echo 'driftwatch 0.1.0'; exit 0; fi\ncat '{doc}'\nexit 1\n"
        ),
    );
    let out = provider_run(
        &db,
        &["gate-runtime", "--fixture", &stub.display().to_string()],
    );
    let value = provider_json(&out);
    let row = &value["run"];
    // A parseable status document is a successful round trip whatever
    // the exit code, and it carries its own aggregate label.
    assert_eq!(row["status"], "supported", "{row}");
    let text = serde_json::to_string(&row).unwrap();
    assert!(text.contains("aggregate=blocked"), "{text}");
}

#[test]
fn fixture_refusal_is_unavailable_never_supported() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let stub = write_script(
        tmp.path(),
        "gate-refuse.sh",
        "if [ \"$1\" = \"--version\" ]; then echo 'driftwatch 0.1.0'; exit 0; fi\necho 'gate: uninitialized project' >&2\nexit 1\n",
    );
    let out = provider_run(
        &db,
        &["gate-runtime", "--fixture", &stub.display().to_string()],
    );
    let value = provider_json(&out);
    assert_eq!(value["run"]["status"], "unavailable", "{}", value["run"]);
}

#[test]
fn live_without_the_opt_in_flag_stays_not_run() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let out = provider_run(&db, &["gate-runtime", "--live"]);
    let value = provider_json(&out);
    assert_eq!(value["run"]["status"], "not-run", "{}", value["run"]);
    assert!(value["run"]["reason"]
        .as_str()
        .unwrap_or("")
        .contains("FORGE_PROVIDER_LIVE"));
}

#[test]
fn live_env_override_runs_exactly_the_named_binary() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let plan = fixture("gate-dryrun-plan.txt").display().to_string();
    let stub = write_script(
        tmp.path(),
        "driftwatchdog",
        &format!(
            "if [ \"$1\" = \"--version\" ]; then echo 'driftwatch 0.1.0'; exit 0; fi\ncat '{plan}'\nexit 0\n"
        ),
    );
    let mut cmd = Command::new(forge_bin());
    cmd.env("FORGE_GATE_BIN", &stub)
        .env("FORGE_PROVIDER_LIVE", "1")
        .env_remove("FORGE_DRIFTWATCH_BIN")
        .env_remove("FORGE_REGISTRY");
    let out = cmd
        .arg("--registry")
        .arg(&db)
        .arg("--format")
        .arg("json")
        .arg("provider")
        .arg("run")
        .arg("gate-runtime")
        .arg("--live")
        .output()
        .expect("run");
    let value: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let row = &value["run"];
    assert_eq!(row["status"], "supported", "{row}");
    assert_eq!(row["provenance"]["sandbox"], "live");
    assert_eq!(row["provenance"]["tool_version"], "driftwatch 0.1.0");
}

#[test]
fn live_probe_resolves_the_ordered_sibling_names() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let bin_dir = tmp.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    // Only the alias name exists on this PATH: the ordered probe must
    // find `driftwatch` and the row must attribute it by name.
    let plan = fixture("gate-dryrun-plan.txt").display().to_string();
    let _alias = write_script(
        &bin_dir,
        "driftwatch",
        &format!(
            "if [ \"$1\" = \"--version\" ]; then echo 'driftwatch 0.1.0'; exit 0; fi\ncat '{plan}'\nexit 0\n"
        ),
    );
    let mut cmd = Command::new(forge_bin());
    cmd.env_remove("FORGE_GATE_BIN")
        .env_remove("FORGE_DRIFTWATCH_BIN")
        .env("FORGE_PROVIDER_LIVE", "1")
        .env_remove("FORGE_REGISTRY")
        .env(
            "PATH",
            format!(
                "{}:{}",
                bin_dir.display(),
                std::env::var("PATH").unwrap_or_default()
            ),
        );
    // Ensure the real sibling name is NOT resolvable ahead of it by
    // pointing PATH solely at the scratch dir.
    cmd.env("PATH", &bin_dir);
    let out = cmd
        .arg("--registry")
        .arg(&db)
        .arg("--format")
        .arg("json")
        .arg("provider")
        .arg("run")
        .arg("gate-runtime")
        .arg("--live")
        .output()
        .expect("run");
    let value: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let row = &value["run"];
    assert_eq!(row["status"], "supported", "{row}");
    assert!(
        row["provenance"]["source"]
            .as_str()
            .unwrap_or("")
            .contains("driftwatch"),
        "{}",
        row["provenance"]["source"]
    );
}
