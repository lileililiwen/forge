//! Native-profile doctor assessment.

use crate::doctor::model::FindingStatus;
use crate::doctor::runner::run_doctor;
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

#[test]
fn native_profile_is_known_and_assessed() {
    let tmp = TempDir::new().unwrap();
    write(
        tmp.path(),
        "forge.yaml",
        "schema: 1\nproject:\n  id: native-proj\n  name: native-proj\n  profile: rust-product\n  maturity: L1\n  target_maturity: L1\nruntime:\n  language: rust\n",
    );
    write(
        tmp.path(),
        "Cargo.toml",
        "[package]\nname = \"native-proj\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    let report = run_doctor(tmp.path(), None, None, None).unwrap();
    let known = report
        .findings
        .iter()
        .find(|f| f.id == "profile-known")
        .unwrap();
    assert_eq!(known.status, FindingStatus::Pass);
    let features = report
        .findings
        .iter()
        .find(|f| f.id == "features-compatible")
        .unwrap();
    assert_eq!(features.status, FindingStatus::Pass);
    let build = report
        .findings
        .iter()
        .find(|f| f.id == "build-config")
        .unwrap();
    assert_eq!(build.status, FindingStatus::Pass);
}
