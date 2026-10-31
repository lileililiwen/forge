//! Doctor handling of a policy plane with no configured checkers.

use crate::doctor::model::{FindingStatus, RegistryObservation};
use crate::doctor::runner::run_doctor;
use crate::policy::{PolicyFinding, PolicyOutcome, PolicyReport};
use std::fs;
use std::path::Path;
use tempfile::TempDir;

fn write(dir: &Path, name: &str, text: &str) {
    let path = dir.join(name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, text).unwrap();
}

fn rust_manifest(id: &str) -> String {
    format!(
        "schema: 1\nproject:\n  id: {id}\n  name: {id}\n  profile: rust-web\n  maturity: L1\n  target_maturity: L1\nruntime:\n  language: rust\n"
    )
}

fn sample_report(findings: Vec<PolicyFinding>) -> PolicyReport {
    PolicyReport {
        tool: "driftwatch".to_string(),
        tool_version: "0.1.0".to_string(),
        contract: crate::policy::POLICY_CONTRACT_VERSION.to_string(),
        source_revision: None,
        checkers: 1,
        findings,
    }
}

fn no_registry() -> Option<RegistryObservation> {
    None
}

#[test]
fn no_configured_checkers_is_a_warning_not_unavailable() {
    let tmp = TempDir::new().unwrap();
    write(tmp.path(), "forge.yaml", &rust_manifest("policy-none"));
    let mut report = sample_report(Vec::new());
    report.checkers = 0;
    let outcome = PolicyOutcome::Reported(report);
    let doctor_report =
        run_doctor(tmp.path(), None, no_registry().as_ref(), Some(&outcome)).unwrap();
    let finding = doctor_report
        .findings
        .iter()
        .find(|f| f.id == "driftwatch-policy")
        .expect("driftwatch-policy finding");
    assert_eq!(finding.status, FindingStatus::Warn);
}
