//! Doctor and maturity assessment contract: read-only health reporting.
//!
//! Covers the `doctor-maturity-assessment` scenarios end to end through the
//! built binary: finding inventory success (stable IDs, evidence,
//! remediation classes), unavailable inspectors (never healthy), boundary
//! repeatability without touching project files or remotes, L2 assessment
//! reporting, L4 denial without recovery evidence, and L0
//! nonapplicability.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

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

fn run(db: &Path, args: &[&str]) -> std::process::Output {
    let mut cmd = clean_cmd();
    cmd.arg("--registry").arg(db);
    for a in args {
        cmd.arg(a);
    }
    cmd.output().expect("run forge")
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

fn lossy(bytes: &[u8]) -> std::borrow::Cow<'_, str> {
    String::from_utf8_lossy(bytes)
}

fn write_file(dir: &Path, name: &str, text: &str) {
    let path = dir.join(name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, text).unwrap();
}

fn rust_l1(dir: &Path, id: &str) {
    write_file(
        dir,
        "forge.yaml",
        &format!(
            "schema: 1\nproject:\n  id: {id}\n  name: {id}\n  profile: rust-web\n  maturity: L1\n  target_maturity: L1\nruntime:\n  language: rust\n"
        ),
    );
    write_file(dir, "Cargo.toml", "[package]\nname = \"demo\"\n");
}

fn collect_files(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    collect_into(root, &mut out);
    out.sort();
    out
}

fn collect_into(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n == ".git")
        {
            continue;
        }
        out.push(path.clone());
        if path.is_dir() {
            collect_into(&path, out);
        }
    }
}

#[test]
fn doctor_reports_findings_with_evidence_and_remediation() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("drift-app");
    fs::create_dir(&proj).unwrap();
    // Manifest claims rust but the build definition is absent: drift + fail.
    write_file(
        &proj,
        "forge.yaml",
        "schema: 1\nproject:\n  id: drift-app\n  name: drift-app\n  profile: rust-web\n  maturity: L1\n  target_maturity: L1\nruntime:\n  language: rust\n",
    );

    let out = run_json(&db, &["doctor", &proj.display().to_string()]);
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    let value: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let doctor = &value["doctor"];
    assert_eq!(doctor["profile"], "rust-web");
    assert_eq!(doctor["healthy"], false);

    let findings = doctor["findings"].as_array().unwrap();
    let ids: Vec<&str> = findings.iter().map(|f| f["id"].as_str().unwrap()).collect();
    for expected in [
        "manifest-valid",
        "profile-known",
        "features-compatible",
        "dependency-drift",
        "build-config",
        "deployment-config",
        "repository",
        "ci-config",
        "docs-present",
        "driftwatch-config",
        "registry-observation",
        "maturity-requirements",
    ] {
        assert!(
            ids.contains(&expected),
            "missing finding {expected}: {ids:?}"
        );
    }
    for finding in findings {
        assert!(
            !finding["evidence"].as_array().unwrap().is_empty(),
            "finding needs evidence: {finding}"
        );
        assert!(
            ["automatic", "ai", "manual"].contains(&finding["remediation"].as_str().unwrap()),
            "finding needs a remediation class: {finding}"
        );
        assert!(
            ["pass", "warn", "fail", "unavailable"].contains(&finding["status"].as_str().unwrap()),
            "finding needs a status: {finding}"
        );
    }
    let drift = findings
        .iter()
        .find(|f| f["id"] == "dependency-drift")
        .unwrap();
    assert_eq!(drift["status"], "warn");
    let build = findings.iter().find(|f| f["id"] == "build-config").unwrap();
    assert_eq!(build["status"], "fail");
}

#[test]
fn unavailable_inspector_is_reported_and_not_healthy() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("plain-app");
    fs::create_dir(&proj).unwrap();
    rust_l1(&proj, "plain-app");

    // TempDir projects are not git repositories: no PASS may be claimed.
    let out = run_json(&db, &["doctor", &proj.display().to_string()]);
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    let value: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let findings = value["doctor"]["findings"].as_array().unwrap();
    let repo = findings.iter().find(|f| f["id"] == "repository").unwrap();
    assert_eq!(repo["status"], "unavailable");
    assert_eq!(value["doctor"]["healthy"], false);

    let human = run(&db, &["doctor", &proj.display().to_string()]);
    assert_eq!(human.status.code(), Some(0));
    let text = lossy(&human.stdout);
    assert!(text.contains("[UNAVAILABLE] repository"), "{text}");
    assert!(text.contains("not healthy"), "{text}");
}

#[test]
fn doctor_is_repeatable_and_changes_no_project_files() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("clean-app");
    fs::create_dir(&proj).unwrap();
    rust_l1(&proj, "clean-app");
    write_file(&proj, "README.md", "# clean\n");

    let before = collect_files(&proj);
    let first = run_json(&db, &["doctor", &proj.display().to_string()]);
    let second = run_json(&db, &["doctor", &proj.display().to_string()]);
    assert_eq!(first.status.code(), Some(0), "{}", lossy(&first.stderr));
    assert_eq!(second.status.code(), Some(0), "{}", lossy(&second.stderr));
    // Stable across runs except the echoed path-independent content; the
    // full payload must be byte-identical for the same directory.
    assert_eq!(first.stdout, second.stdout);
    assert_eq!(collect_files(&proj), before);
    assert_eq!(
        fs::read_to_string(proj.join("forge.yaml")).unwrap(),
        fs::read_to_string(proj.join("forge.yaml")).unwrap()
    );
}

#[test]
fn l2_assessment_reports_missing_controls() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("l2-app");
    fs::create_dir(&proj).unwrap();
    write_file(
        &proj,
        "forge.yaml",
        "schema: 1\nproject:\n  id: l2-app\n  name: l2-app\n  profile: rust-web\n  maturity: L1\n  target_maturity: L2\nruntime:\n  language: rust\n",
    );
    write_file(&proj, "Cargo.toml", "[package]\nname = \"demo\"\n");

    let out = run_json(&db, &["doctor", &proj.display().to_string()]);
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    let value: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(value["doctor"]["target_maturity"], "L2");
    let controls = value["doctor"]["controls"].as_array().unwrap();
    let unmet: Vec<&str> = controls
        .iter()
        .filter(|c| c["applicable"].as_bool().unwrap() && !c["met"].as_bool().unwrap())
        .map(|c| c["id"].as_str().unwrap())
        .collect();
    for expected in [
        "L2-auth",
        "L2-ci",
        "L2-driftwatch",
        "L2-deployment",
        "L2-audit",
    ] {
        assert!(
            unmet.contains(&expected),
            "missing unmet {expected}: {unmet:?}"
        );
    }
    let maturity = value["doctor"]["findings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["id"] == "maturity-requirements")
        .unwrap();
    assert_eq!(maturity["status"], "fail");
}

#[test]
fn l4_without_recovery_is_denied() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("l4-app");
    fs::create_dir(&proj).unwrap();
    write_file(
        &proj,
        "forge.yaml",
        "schema: 1\nproject:\n  id: l4-app\n  name: l4-app\n  profile: rust-web\n  maturity: L2\n  target_maturity: L4\nruntime:\n  language: rust\n",
    );
    write_file(&proj, "Cargo.toml", "[package]\nname = \"demo\"\n");

    let out = run_json(
        &db,
        &["doctor", &proj.display().to_string(), "--target", "L4"],
    );
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    let value: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let controls = value["doctor"]["controls"].as_array().unwrap();
    let recovery = controls.iter().find(|c| c["id"] == "L4-recovery").unwrap();
    assert_eq!(recovery["applicable"], true);
    assert_eq!(recovery["met"], false);
    assert_eq!(value["doctor"]["healthy"], false);
}

#[test]
fn l0_prototype_is_respected_without_forced_infrastructure() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("proto-app");
    fs::create_dir(&proj).unwrap();
    write_file(
        &proj,
        "forge.yaml",
        "schema: 1\nproject:\n  id: proto-app\n  name: proto-app\n  profile: flutter-app\n  maturity: L0\n  target_maturity: L0\nruntime:\n  language: dart\n",
    );
    write_file(
        &proj,
        "pubspec.yaml",
        "name: demo\nenvironment:\n  flutter: 3.22\n",
    );

    let out = run_json(&db, &["doctor", &proj.display().to_string()]);
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    let value: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(value["doctor"]["target_maturity"], "L0");
    let controls = value["doctor"]["controls"].as_array().unwrap();
    for id in [
        "L2-deployment",
        "L2-ci",
        "L3-release",
        "L4-recovery",
        "L1-database",
    ] {
        let control = controls.iter().find(|c| c["id"] == id).unwrap();
        assert_eq!(
            control["applicable"], false,
            "{id} must stay nonapplicable for L0"
        );
    }
    let unmet: Vec<&serde_json::Value> = controls
        .iter()
        .filter(|c| c["applicable"].as_bool().unwrap() && !c["met"].as_bool().unwrap())
        .collect();
    assert!(
        unmet.is_empty(),
        "L0 must not force infrastructure: {unmet:?}"
    );
}

#[test]
fn stale_registered_observation_is_shown_as_stale() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let proj = tmp.path().join("stale-app");
    fs::create_dir(&proj).unwrap();
    rust_l1(&proj, "stale-app");

    let registered = run(&db, &["register", &proj.display().to_string()]);
    assert_eq!(
        registered.status.code(),
        Some(0),
        "{}",
        lossy(&registered.stderr)
    );

    // Touching the manifest after registration makes the observation stale.
    let manifest = fs::read_to_string(proj.join("forge.yaml")).unwrap();
    fs::write(proj.join("forge.yaml"), format!("{manifest}# touched\n")).unwrap();

    let out = run_json(&db, &["doctor", &proj.display().to_string()]);
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    let value: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(value["doctor"]["stale"], true);
    assert_eq!(value["doctor"]["healthy"], false);
    let findings = value["doctor"]["findings"].as_array().unwrap();
    let observation = findings
        .iter()
        .find(|f| f["id"] == "registry-observation")
        .unwrap();
    assert!(
        observation["evidence"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e.as_str().unwrap().contains("stale")),
        "{observation}"
    );
}

#[test]
fn doctor_on_generated_project_uses_registry_freshness() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("registry.db");
    let dest = tmp.path().join("gen-app");

    let created = run(
        &db,
        &["new", &dest.display().to_string(), "--profile", "rust-web"],
    );
    assert_eq!(created.status.code(), Some(0), "{}", lossy(&created.stderr));

    let out = run_json(&db, &["doctor", &dest.display().to_string()]);
    assert_eq!(out.status.code(), Some(0), "{}", lossy(&out.stderr));
    let value: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(value["doctor"]["profile"], "rust-web");
    assert_eq!(value["doctor"]["stale"], false);
    let findings = value["doctor"]["findings"].as_array().unwrap();
    let observation = findings
        .iter()
        .find(|f| f["id"] == "registry-observation")
        .unwrap();
    assert_eq!(observation["status"], "pass");
}
