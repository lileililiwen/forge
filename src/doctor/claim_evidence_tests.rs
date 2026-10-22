//! The doctor's claim-evidence rules (`doctor-claim-evidence`).
//!
//! The doctor's statements must name only the checks it actually
//! performs. These tests cover the three registry states (a fresh
//! observation, an observation that says the project is not registered,
//! and the absence of an observation), the `L3-registry` control that
//! follows the observation, the `L1-structure` control that names the
//! two facts it checks, and the `dependency-drift` evidence that names
//! the languages it compared.

#[cfg(test)]
pub(super) mod tests {
    use super::super::model::{Finding, FindingStatus, RegistryObservation};
    use super::super::runner::run_doctor;

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

    /// A Rust project manifest at one maturity level.
    fn rust_manifest_at(id: &str, maturity: &str) -> String {
        format!(
            "schema: 1\nproject:\n  id: {id}\n  name: {id}\n  profile: rust-web\n  maturity: {maturity}\n  target_maturity: {maturity}\nruntime:\n  language: rust\n"
        )
    }

    /// One finding of a doctor report.
    fn finding_of<'a>(report: &'a crate::doctor::DoctorReport, id: &str) -> &'a Finding {
        report
            .findings
            .iter()
            .find(|f| f.id == id)
            .unwrap_or_else(|| panic!("{id} finding must be present"))
    }

    /// The caller supplied no observation at all.
    fn no_registry() -> Option<RegistryObservation> {
        None
    }

    /// The caller read the registry and found no record for the project.
    fn not_registered() -> RegistryObservation {
        RegistryObservation {
            registered: false,
            observed_at: None,
            delivery: None,
        }
    }

    #[test]
    fn no_registry_observation_leaves_the_registration_state_unknown() {
        let tmp = TempDir::new().unwrap();
        write(
            tmp.path(),
            "forge.yaml",
            &rust_manifest("unknown-registry-proj"),
        );
        write(tmp.path(), "Cargo.toml", "[package]\nname = \"demo\"\n");
        let report = run_doctor(tmp.path(), None, no_registry().as_ref(), None).unwrap();
        let finding = finding_of(&report, "registry-observation");
        assert_eq!(finding.status, FindingStatus::Pass);
        assert!(
            !finding.applicable,
            "the doctor cannot evaluate a registration state it never read"
        );
        assert!(
            finding
                .evidence
                .iter()
                .any(|e| e.contains("no registry observation was supplied")),
            "{:?}",
            finding.evidence
        );
        assert!(
            !finding
                .evidence
                .iter()
                .any(|e| e.contains("not registered")),
            "the doctor must not claim the project is unregistered: {:?}",
            finding.evidence
        );
        assert!(
            !finding.detail.contains("not registered"),
            "the doctor must not claim the project is unregistered: {}",
            finding.detail
        );
    }

    #[test]
    fn an_observation_that_says_not_registered_still_warns() {
        let tmp = TempDir::new().unwrap();
        write(
            tmp.path(),
            "forge.yaml",
            &rust_manifest("unregistered-proj"),
        );
        write(tmp.path(), "Cargo.toml", "[package]\nname = \"demo\"\n");
        let obs = not_registered();
        let report = run_doctor(tmp.path(), None, Some(&obs), None).unwrap();
        let finding = finding_of(&report, "registry-observation");
        assert_eq!(finding.status, FindingStatus::Warn);
        assert!(finding.applicable);
        assert!(
            finding
                .evidence
                .iter()
                .any(|e| e.contains("not registered")),
            "{:?}",
            finding.evidence
        );
        assert_eq!(finding.detail, "project is not registered");
    }

    #[test]
    fn the_l3_registry_control_follows_the_observation() {
        let tmp = TempDir::new().unwrap();
        write(
            tmp.path(),
            "forge.yaml",
            &rust_manifest_at("l3-registry-proj", "L3"),
        );
        write(tmp.path(), "Cargo.toml", "[package]\nname = \"demo\"\n");

        // No observation: the registration state is unknown, so the control
        // is not applicable and the project is not failed for it.
        let report = run_doctor(tmp.path(), None, no_registry().as_ref(), None).unwrap();
        let control = report
            .controls
            .iter()
            .find(|c| c.id == "L3-registry")
            .unwrap();
        assert!(!control.applicable);
        assert!(!control.met);
        assert!(
            control
                .evidence
                .iter()
                .any(|e| e.contains("no registry observation was supplied")),
            "{:?}",
            control.evidence
        );
        assert!(
            !report
                .unmet_controls()
                .iter()
                .any(|c| c.id == "L3-registry"),
            "an unknown state is never counted as an unmet control"
        );

        // An observation that says the project is not registered: the
        // control is applicable and unmet.
        let obs = not_registered();
        let report = run_doctor(tmp.path(), None, Some(&obs), None).unwrap();
        let control = report
            .controls
            .iter()
            .find(|c| c.id == "L3-registry")
            .unwrap();
        assert!(control.applicable);
        assert!(!control.met);
        assert!(
            control
                .evidence
                .iter()
                .any(|e| e.contains("not registered")),
            "{:?}",
            control.evidence
        );
    }

    #[test]
    fn the_l1_structure_control_names_the_checks_it_performs() {
        let tmp = TempDir::new().unwrap();
        write(tmp.path(), "forge.yaml", &rust_manifest("structure-proj"));
        write(tmp.path(), "Cargo.toml", "[package]\nname = \"demo\"\n");
        let report = run_doctor(tmp.path(), None, no_registry().as_ref(), None).unwrap();
        let control = report
            .controls
            .iter()
            .find(|c| c.id == "L1-structure")
            .unwrap();
        assert!(
            control.description.contains("manifest"),
            "the control names the manifest it checks: {}",
            control.description
        );
        assert!(
            control.description.contains("build definition"),
            "the control names the build definition it checks: {}",
            control.description
        );
        assert!(
            !control.description.contains("layout"),
            "the control must not claim a layout comparison: {}",
            control.description
        );
    }

    #[test]
    fn the_dependency_drift_evidence_names_both_languages() {
        let tmp = TempDir::new().unwrap();
        write(
            tmp.path(),
            "forge.yaml",
            &rust_manifest("drift-evidence-proj"),
        );
        write(tmp.path(), "Cargo.toml", "[package]\nname = \"demo\"\n");
        let report = run_doctor(tmp.path(), None, no_registry().as_ref(), None).unwrap();
        let drift = finding_of(&report, "dependency-drift");
        assert_eq!(drift.status, FindingStatus::Pass);
        assert!(
            drift.evidence.iter().any(|e| e
                .contains("manifest runtime language 'rust' agrees with profile language 'rust'")),
            "{:?}",
            drift.evidence
        );
        assert!(
            drift
                .evidence
                .iter()
                .any(|e| e.contains("the profile's build definition is present (Cargo.toml)")),
            "{:?}",
            drift.evidence
        );
        assert!(
            !drift
                .evidence
                .iter()
                .any(|e| e.contains("on-disk dependency evidence")),
            "the evidence must not credit evidence the check never read: {:?}",
            drift.evidence
        );
    }

    #[test]
    fn a_manifest_without_a_runtime_language_says_no_language_was_compared() {
        let tmp = TempDir::new().unwrap();
        write(
            tmp.path(),
            "forge.yaml",
            "schema: 1\nproject:\n  id: no-lang-proj\n  name: no-lang-proj\n  profile: rust-web\n  maturity: L1\n  target_maturity: L1\n",
        );
        write(tmp.path(), "Cargo.toml", "[package]\nname = \"demo\"\n");
        let report = run_doctor(tmp.path(), None, no_registry().as_ref(), None).unwrap();
        let drift = finding_of(&report, "dependency-drift");
        assert_eq!(drift.status, FindingStatus::Pass);
        assert!(
            drift.evidence.iter().any(|e| e.contains(
                "the manifest declares no runtime language; no manifest language was compared"
            )),
            "{:?}",
            drift.evidence
        );
    }
}
