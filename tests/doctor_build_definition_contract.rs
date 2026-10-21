//! Doctor build-definition truth contract (`doctor-build-definition-truth`).
//!
//! The doctor's table of expected build definitions must cover every
//! supported profile, and the table is pinned here in the house `MATRIX`
//! shape: a profile cannot be added to the catalogue without the doctor
//! learning it, and an expectation cannot change without this check
//! failing and naming the profile. The behaviour is driven through the
//! built binary, because a right table with a wrong report is still wrong:
//! a healthy React project passes, a React project without `package.json`
//! fails with the file named, and a profile the doctor cannot judge is
//! unavailable rather than failed.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use forge::profile::{is_supported, list_profiles};

/// `(profile, expected build definition)` for every supported profile.
const MATRIX: [(&str, &[&str]); 6] = [
    ("aspnet-web", &["*.csproj"]),
    ("flutter-app", &["pubspec.yaml"]),
    ("nextjs-web", &["package.json"]),
    (
        "python-service",
        &[
            "pyproject.toml",
            "requirements.txt",
            "setup.py",
            "setup.cfg",
        ],
    ),
    ("react-web", &["package.json"]),
    ("rust-web", &["Cargo.toml"]),
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

fn react_manifest(id: &str) -> String {
    format!(
        "schema: 1\nproject:\n  id: {id}\n  name: {id}\n  profile: react-web\n  maturity: L1\n  target_maturity: L1\nruntime:\n  language: typescript\n"
    )
}

/// The doctor report of one project, read from the JSON output.
fn doctor_report(dir: &Path) -> serde_json::Value {
    let db = dir.join("registry.db");
    let out = run_json(&db, &["doctor", &dir.join("project").display().to_string()]);
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

#[test]
fn every_supported_profile_has_a_build_definition() {
    for descriptor in list_profiles() {
        assert!(
            forge::doctor::expected_build_definition(&descriptor.id).is_some(),
            "the doctor has no expected build definition for the supported profile '{}'",
            descriptor.id
        );
    }
}

#[test]
fn the_doctors_table_matches_the_pinned_matrix_in_both_directions() {
    // The doctor holds exactly the pinned profiles, in table order.
    let doctor_profiles = forge::doctor::build_definition_profiles();
    let matrix_profiles: Vec<&str> = MATRIX.iter().map(|(id, _)| *id).collect();
    assert_eq!(
        doctor_profiles, matrix_profiles,
        "the doctor's table must hold exactly the pinned profiles, in table order"
    );

    // Each row carries the pinned expectation.
    for (profile, expected) in MATRIX {
        let definition = forge::doctor::expected_build_definition(profile)
            .unwrap_or_else(|| panic!("{profile} must have an expected build definition"));
        assert_eq!(
            definition.expected_names(),
            expected,
            "{profile} must carry the pinned expected build definition"
        );
    }

    // Every profile the doctor judges is a supported profile.
    for profile in forge::doctor::build_definition_profiles() {
        assert!(
            is_supported(profile),
            "the doctor judges '{profile}', which the catalogue does not support"
        );
    }
}

#[test]
fn a_healthy_react_project_passes_the_build_definition_checks() {
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("project");
    fs::create_dir_all(&project).unwrap();
    write_file(&project, "forge.yaml", &react_manifest("react-healthy"));
    write_file(&project, "package.json", "{\"name\": \"react-healthy\"}\n");

    let report = doctor_report(tmp.path());
    let build = finding(&report, "build-config");
    assert_eq!(build["status"], "pass", "{build}");
    assert!(
        evidence(build).contains("package.json"),
        "the evidence must name the file that was found: {build}"
    );

    let drift = finding(&report, "dependency-drift");
    assert_eq!(drift["status"], "pass", "{drift}");
    assert!(
        !evidence(drift).contains("unknown profile"),
        "a known profile is never reported as unknown: {drift}"
    );
}

#[test]
fn a_react_project_without_package_json_fails_the_build_definition() {
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("project");
    fs::create_dir_all(&project).unwrap();
    write_file(&project, "forge.yaml", &react_manifest("react-bare"));

    let report = doctor_report(tmp.path());
    let build = finding(&report, "build-config");
    assert_eq!(build["status"], "fail", "{build}");
    assert!(
        evidence(build).contains("package.json"),
        "the evidence must name the expected file: {build}"
    );
}

#[test]
fn a_profile_the_doctor_cannot_judge_is_unavailable_not_failed() {
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("project");
    fs::create_dir_all(&project).unwrap();
    write_file(
        &project,
        "forge.yaml",
        "schema: 1\nproject:\n  id: planned-app\n  name: planned-app\n  profile: flutter-client\n  maturity: L1\n  target_maturity: L1\nruntime:\n  language: dart\n",
    );

    let report = doctor_report(tmp.path());
    let build = finding(&report, "build-config");
    assert_eq!(
        build["status"], "unavailable",
        "the doctor's own gap is never the project's fault: {build}"
    );
    assert!(
        evidence(build).contains("flutter-client"),
        "the evidence must name the profile: {build}"
    );
    assert!(
        !evidence(build).contains("missing build definition"),
        "an unjudgeable profile is never reported as a missing build definition: {build}"
    );

    let drift = finding(&report, "dependency-drift");
    assert_eq!(
        drift["status"], "unavailable",
        "the drift cannot be evaluated without the build definition: {drift}"
    );
}
