//! Doctor finding applicability contract (`doctor-finding-applicability`).
//!
//! A configuration finding and the maturity control it mirrors must answer
//! the same question the same way: a project that does not reach the level
//! that declares the control is not warned about the configuration, and the
//! absence stays in the evidence. The DriftWatch requirement is the
//! project's own: a manifest that declares the DriftWatch release check
//! requires the configuration at any maturity, exactly as the release lane
//! reads it. The behaviour is driven through the built binary, because a
//! right rule with a wrong report is still wrong.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The three configuration findings and the controls they mirror.
const CONFIGURATION_FINDINGS: [(&str, &str); 3] = [
    ("ci-config", "L2-ci"),
    ("deployment-config", "L2-deployment"),
    ("driftwatch-config", "L2-driftwatch"),
];

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

/// A Rust project manifest at one maturity level, with an optional
/// `release.checks` block appended.
fn manifest(id: &str, maturity: &str, release_checks: &str) -> String {
    let mut text = format!(
        "schema: 1\nproject:\n  id: {id}\n  name: {id}\n  profile: rust-web\n  maturity: {maturity}\n  target_maturity: {maturity}\nruntime:\n  language: rust\n"
    );
    if !release_checks.is_empty() {
        text.push_str(release_checks);
    }
    text
}

/// The doctor report of one project, read from the JSON output.
fn doctor_report(root: &Path) -> serde_json::Value {
    let db = root.join("registry.db");
    let out = run_json(
        &db,
        &["doctor", &root.join("project").display().to_string()],
    );
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    let value: serde_json::Value = serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|err| panic!("doctor output must be JSON: {err}"));
    value["doctor"].clone()
}

/// One finding of the doctor report.
fn finding<'a>(report: &'a serde_json::Value, id: &str) -> &'a serde_json::Value {
    report["findings"]
        .as_array()
        .expect("findings array")
        .iter()
        .find(|f| f["id"] == id)
        .unwrap_or_else(|| panic!("{id} finding must be present"))
}

/// One maturity control of the doctor report.
fn control<'a>(report: &'a serde_json::Value, id: &str) -> &'a serde_json::Value {
    report["controls"]
        .as_array()
        .expect("controls array")
        .iter()
        .find(|c| c["id"] == id)
        .unwrap_or_else(|| panic!("{id} control must be present"))
}

/// The evidence of one finding, joined for readable assertions.
fn evidence(finding: &serde_json::Value) -> String {
    finding["evidence"]
        .as_array()
        .expect("evidence array")
        .iter()
        .map(|e| e.as_str().unwrap_or_default().to_string())
        .collect::<Vec<String>>()
        .join("; ")
}

/// A project directory with a manifest and a build definition.
fn project(root: &Path, id: &str, maturity: &str, release_checks: &str) -> PathBuf {
    let dir = root.join("project");
    fs::create_dir_all(&dir).unwrap();
    write_file(&dir, "forge.yaml", &manifest(id, maturity, release_checks));
    write_file(&dir, "Cargo.toml", "[package]\nname = \"demo\"\n");
    dir
}

#[test]
fn an_l1_project_does_not_require_the_l2_configurations() {
    let tmp = tempfile::tempdir().unwrap();
    project(tmp.path(), "l1-app", "L1", "");

    let report = doctor_report(tmp.path());
    for (id, control_id) in CONFIGURATION_FINDINGS {
        let config = finding(&report, id);
        assert_eq!(
            config["applicable"], false,
            "{id} must not be applicable below the level that declares its control: {config}"
        );
        assert_eq!(
            config["status"], "pass",
            "{id} must not warn about a configuration the project does not require: {config}"
        );
        assert!(
            evidence(config).contains("no "),
            "{id} must keep the absence in its evidence: {config}"
        );
        let mirrored = control(&report, control_id);
        assert_eq!(
            mirrored["applicable"], false,
            "{id} and {control_id} must agree about the requirement: {mirrored}"
        );
    }
}

#[test]
fn an_l2_project_is_warned_about_the_missing_configurations() {
    let tmp = tempfile::tempdir().unwrap();
    project(tmp.path(), "l2-app", "L2", "");

    let report = doctor_report(tmp.path());
    for (id, control_id) in CONFIGURATION_FINDINGS {
        let config = finding(&report, id);
        assert_eq!(
            config["applicable"], true,
            "{id} must be applicable at the level that declares its control: {config}"
        );
        assert_eq!(
            config["status"], "warn",
            "{id} must warn when the required configuration is missing: {config}"
        );
        assert!(
            config["detail"]
                .as_str()
                .unwrap()
                .contains("required for L2"),
            "{id} must name the requirement it warns about: {config}"
        );
        let mirrored = control(&report, control_id);
        assert_eq!(
            mirrored["applicable"], true,
            "{id} and {control_id} must agree about the requirement: {mirrored}"
        );
    }
}

#[test]
fn a_declared_driftwatch_check_is_required_at_any_maturity() {
    let tmp = tempfile::tempdir().unwrap();
    project(
        tmp.path(),
        "checked-app",
        "L1",
        "release:\n  versioning: semver\n  checks:\n    - kind: driftwatch\n",
    );

    let report = doctor_report(tmp.path());
    let driftwatch = finding(&report, "driftwatch-config");
    assert_eq!(
        driftwatch["applicable"], true,
        "a declared driftwatch check requires the configuration at any maturity: {driftwatch}"
    );
    assert_eq!(
        driftwatch["status"], "warn",
        "a declared driftwatch check warns when the configuration is missing: {driftwatch}"
    );
    assert!(
        evidence(driftwatch).contains("no driftwatch configuration"),
        "the absence stays in the evidence: {driftwatch}"
    );
    for id in ["ci-config", "deployment-config"] {
        let config = finding(&report, id);
        assert_eq!(
            config["applicable"], false,
            "{id} is not declared by the manifest and must stay not applicable: {config}"
        );
    }
}

#[test]
fn a_present_configuration_passes_at_l2() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = project(tmp.path(), "configured-app", "L2", "");
    write_file(&dir, "Dockerfile", "FROM scratch\n");
    write_file(&dir, ".gitlab-ci.yml", "stages: [build]\n");
    write_file(&dir, "driftwatch.yaml", "version: 1\n");

    let report = doctor_report(tmp.path());
    for (id, _) in CONFIGURATION_FINDINGS {
        let config = finding(&report, id);
        assert_eq!(
            config["applicable"], true,
            "{id} must be applicable at the level that declares its control: {config}"
        );
        assert_eq!(
            config["status"], "pass",
            "{id} must pass when the configuration is present: {config}"
        );
    }
}

#[test]
fn the_checker_plane_reports_no_alert_for_a_configuration_the_project_does_not_require() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = project(tmp.path(), "alert-free-app", "L1", "");

    let report = forge::doctor::run_doctor(&dir, None, None, None).expect("doctor report");
    let alerts = forge::checker::doctor_alerts(&dir, &report);
    for (id, _) in CONFIGURATION_FINDINGS {
        assert!(
            !alerts
                .iter()
                .any(|alert| alert.symbol == format!("doctor/{id}")),
            "{id} must not raise a checker alert when the project does not require it: {alerts:?}"
        );
    }
}
