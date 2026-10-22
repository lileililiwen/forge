//! Doctor claim-evidence contract (`doctor-claim-evidence`).
//!
//! The doctor's statements must name only the checks it actually
//! performs. Three claims are pinned here:
//!
//! 1. The registration state is only claimed when the doctor holds an
//!    observation. A run without an observation says the state is unknown
//!    and stays not applicable; a run with an observation that says "not
//!    registered" still warns. The CLI and the MCP surface both consult
//!    the registry, so both report "not registered" for a project the
//!    registry does not hold.
//! 2. The `L3-registry` control follows the observation: it is applicable
//!    only when the doctor holds one, and it names the unknown.
//! 3. The `L1-structure` control and the `dependency-drift` finding name
//!    the facts they actually checked: the manifest and the profile's
//!    build definition, and the manifest's runtime language against the
//!    profile's language.
//!
//! The behaviour is driven through the public API and the built binary,
//! because a right rule with a wrong report is still wrong.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use forge::doctor::{run_doctor, FindingStatus, RegistryObservation};

fn forge_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_forge"))
}

fn clean_cmd() -> Command {
    let mut cmd = Command::new(forge_bin());
    cmd.env_remove("FORGE_REGISTRY");
    cmd.env_remove("HTTP_PROXY");
    cmd.env_remove("HTTPS_PROXY");
    cmd.env_remove("ALL_PROXY");
    cmd.env_remove("OPENAI_API_KEY");
    cmd.env_remove("ANTHROPIC_API_KEY");
    cmd
}

fn run_json(db: &Path, args: &[&str]) -> std::process::Output {
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(db);
    cmd.arg("--format").arg("json");
    for a in args {
        cmd.arg(a);
    }
    cmd.output().expect("run forge")
}

fn lossy(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).to_string()
}

fn write_file(dir: &Path, name: &str, text: &str) {
    let path = dir.join(name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, text).unwrap();
}

/// A Rust project manifest at one maturity level.
fn manifest(id: &str, maturity: &str) -> String {
    format!(
        "schema: 1\nproject:\n  id: {id}\n  name: {id}\n  profile: rust-web\n  maturity: {maturity}\n  target_maturity: {maturity}\nruntime:\n  language: rust\n"
    )
}

/// A project directory with a manifest and a build definition.
fn project(root: &Path, id: &str, maturity: &str) -> PathBuf {
    let dir = root.join("project");
    fs::create_dir_all(&dir).unwrap();
    write_file(&dir, "forge.yaml", &manifest(id, maturity));
    write_file(&dir, "Cargo.toml", "[package]\nname = \"demo\"\n");
    dir
}

/// One finding of a JSON doctor report.
fn json_finding<'a>(report: &'a serde_json::Value, id: &str) -> &'a serde_json::Value {
    report["findings"]
        .as_array()
        .expect("findings array")
        .iter()
        .find(|f| f["id"] == id)
        .unwrap_or_else(|| panic!("{id} finding must be present"))
}

/// The evidence of one finding, joined for readable assertions.
fn json_evidence(finding: &serde_json::Value) -> String {
    finding["evidence"]
        .as_array()
        .expect("evidence array")
        .iter()
        .map(|e| e.as_str().unwrap_or_default().to_string())
        .collect::<Vec<String>>()
        .join("; ")
}

#[test]
fn a_doctor_run_without_an_observation_does_not_claim_the_project_is_unregistered() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = project(tmp.path(), "unknown-registry", "L1");

    let report = run_doctor(&dir, None, None, None).expect("doctor report");
    let finding = report
        .findings
        .iter()
        .find(|f| f.id == "registry-observation")
        .expect("registry-observation finding");
    assert_eq!(
        finding.status,
        FindingStatus::Pass,
        "an unknown registration state is never a pass over a known one: {finding:?}"
    );
    assert!(
        !finding.applicable,
        "the doctor cannot judge a registration state it never read: {finding:?}"
    );
    assert!(
        finding
            .evidence
            .iter()
            .any(|e| e.contains("no registry observation was supplied")),
        "the evidence must name the unknown: {finding:?}"
    );
    assert!(
        !finding
            .evidence
            .iter()
            .any(|e| e.contains("not registered")),
        "the doctor must not claim the project is unregistered: {finding:?}"
    );
    assert!(
        !finding.detail.contains("not registered"),
        "the doctor must not claim the project is unregistered: {finding:?}"
    );
}

#[test]
fn an_observation_that_says_not_registered_still_warns() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = project(tmp.path(), "observed-unregistered", "L1");
    let observation = RegistryObservation {
        registered: false,
        observed_at: None,
        delivery: None,
    };

    let report = run_doctor(&dir, None, Some(&observation), None).expect("doctor report");
    let finding = report
        .findings
        .iter()
        .find(|f| f.id == "registry-observation")
        .expect("registry-observation finding");
    assert_eq!(
        finding.status,
        FindingStatus::Warn,
        "a read fact of 'not registered' still warns: {finding:?}"
    );
    assert!(
        finding.applicable,
        "a read fact of 'not registered' is applicable: {finding:?}"
    );
    assert!(
        finding
            .evidence
            .iter()
            .any(|e| e.contains("not registered")),
        "the evidence must name the state the caller read: {finding:?}"
    );
}

#[test]
fn the_cli_reports_an_unregistered_project_as_not_registered() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = project(tmp.path(), "cli-unregistered", "L1");
    let db = tmp.path().join("registry.db");

    let out = run_json(&db, &["doctor", &dir.display().to_string()]);
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    let value: serde_json::Value = serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|err| panic!("doctor output must be JSON: {err}"));
    let report = &value["doctor"];

    let finding = json_finding(report, "registry-observation");
    assert_eq!(
        finding["status"], "warn",
        "the CLI consults the registry, so an unregistered project still warns: {finding}"
    );
    assert_eq!(finding["applicable"], true, "{finding}");
    assert!(
        json_evidence(finding).contains("not registered"),
        "the CLI must report the state it read: {finding}"
    );
}

#[test]
fn the_mcp_surface_reports_an_unregistered_project_as_not_registered() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = project(tmp.path(), "mcp-unregistered", "L1");
    let db = tmp.path().join("registry.db");

    let request = forge::mcp::McpRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(serde_json::Value::from(1)),
        method: "run_doctor".to_string(),
        params: serde_json::json!({ "path": dir.display().to_string() }),
    };
    let value = forge::mcp::dispatch(Some(&db), &request).expect("mcp run_doctor");
    let report = &value["doctor"];

    let finding = json_finding(report, "registry-observation");
    assert_eq!(
        finding["status"], "warn",
        "the MCP surface consults the registry, so an unregistered project still warns: {finding}"
    );
    assert_eq!(finding["applicable"], true, "{finding}");
    assert!(
        json_evidence(finding).contains("not registered"),
        "the MCP surface must report the state it read: {finding}"
    );
}

#[test]
fn the_l3_registry_control_is_applicable_only_with_an_observation() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = project(tmp.path(), "l3-registry", "L3");

    let report = run_doctor(&dir, None, None, None).expect("doctor report");
    let control = report
        .controls
        .iter()
        .find(|c| c.id == "L3-registry")
        .expect("L3-registry control");
    assert!(
        !control.applicable,
        "the control cannot be judged without an observation: {control:?}"
    );
    assert!(
        control
            .evidence
            .iter()
            .any(|e| e.contains("no registry observation was supplied")),
        "the control must name the unknown: {control:?}"
    );
    assert!(
        !report
            .unmet_controls()
            .iter()
            .any(|c| c.id == "L3-registry"),
        "an unknown state is never counted as an unmet control: {report:?}"
    );

    let observation = RegistryObservation {
        registered: false,
        observed_at: None,
        delivery: None,
    };
    let report = run_doctor(&dir, None, Some(&observation), None).expect("doctor report");
    let control = report
        .controls
        .iter()
        .find(|c| c.id == "L3-registry")
        .expect("L3-registry control");
    assert!(
        control.applicable,
        "a read fact of 'not registered' makes the control applicable: {control:?}"
    );
    assert!(
        !control.met,
        "a read fact of 'not registered' leaves the control unmet: {control:?}"
    );
    assert!(
        control
            .evidence
            .iter()
            .any(|e| e.contains("not registered")),
        "the control must name the state the caller read: {control:?}"
    );
}

#[test]
fn the_l1_structure_control_names_the_checks_it_performs() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = project(tmp.path(), "structure", "L1");

    let report = run_doctor(&dir, None, None, None).expect("doctor report");
    let control = report
        .controls
        .iter()
        .find(|c| c.id == "L1-structure")
        .expect("L1-structure control");
    assert!(
        control.description.contains("manifest"),
        "the control names the manifest it checks: {control:?}"
    );
    assert!(
        control.description.contains("build definition"),
        "the control names the build definition it checks: {control:?}"
    );
    assert!(
        !control.description.contains("layout"),
        "the control must not claim a layout comparison: {control:?}"
    );
}

#[test]
fn the_dependency_drift_evidence_names_both_languages() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = project(tmp.path(), "drift-evidence", "L1");

    let report = run_doctor(&dir, None, None, None).expect("doctor report");
    let finding = report
        .findings
        .iter()
        .find(|f| f.id == "dependency-drift")
        .expect("dependency-drift finding");
    assert_eq!(
        finding.status,
        FindingStatus::Pass,
        "a project with no drift passes the check: {finding:?}"
    );
    assert!(
        finding
            .evidence
            .iter()
            .any(|e| e
                .contains("manifest runtime language 'rust' agrees with profile language 'rust'")),
        "the evidence must name both languages: {finding:?}"
    );
    assert!(
        finding
            .evidence
            .iter()
            .any(|e| e.contains("the profile's build definition is present (Cargo.toml)")),
        "the evidence must name the build definition that was found: {finding:?}"
    );
    assert!(
        !finding
            .evidence
            .iter()
            .any(|e| e.contains("on-disk dependency evidence")),
        "the evidence must not credit evidence the check never read: {finding:?}"
    );
}

#[test]
fn a_manifest_without_a_runtime_language_says_no_language_was_compared() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = project(tmp.path(), "no-language", "L1");
    write_file(
        &dir,
        "forge.yaml",
        "schema: 1\nproject:\n  id: no-language\n  name: no-language\n  profile: rust-web\n  maturity: L1\n  target_maturity: L1\n",
    );

    let report = run_doctor(&dir, None, None, None).expect("doctor report");
    let finding = report
        .findings
        .iter()
        .find(|f| f.id == "dependency-drift")
        .expect("dependency-drift finding");
    assert_eq!(
        finding.status,
        FindingStatus::Pass,
        "a manifest with no runtime language is not a drift: {finding:?}"
    );
    assert!(
        finding.evidence.iter().any(|e| e.contains(
            "the manifest declares no runtime language; no manifest language was compared"
        )),
        "the evidence must say that no language was compared: {finding:?}"
    );
}
