//! The doctor's delivery-state rule (`lifecycle-observation-closure`).
//!
//! The delivery lane owns the phase vocabulary; the doctor reads it and
//! reports it. These tests cover the closed phase-to-status rule, the
//! not-applicable case for a project without a delivery, the unavailable
//! case for a state that cannot be read, and the two cases that produce
//! no delivery finding at all.

#[cfg(test)]
pub(super) mod tests {
    use super::super::model::{
        DeliveryObservation, Finding, FindingStatus, RegistryObservation, Remediation,
    };
    use super::super::runner::run_doctor;

    use crate::delivery::state::DeliveryPhase;
    use crate::delivery::DeliveryEnvironment;
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

    /// A registered project whose delivery state the caller observed.
    fn observation_with(delivery: Option<DeliveryObservation>) -> RegistryObservation {
        RegistryObservation {
            registered: true,
            observed_at: Some(chrono::Utc::now().to_rfc3339()),
            delivery,
        }
    }

    /// A recorded delivery for one phase, carrying every field the
    /// projection can report.
    fn recorded(phase: DeliveryPhase) -> DeliveryObservation {
        DeliveryObservation::Recorded {
            phase,
            environment: Some(DeliveryEnvironment::Stage),
            revision: Some("0123456789abcdef0123456789abcdef01234567".to_string()),
            updated_at: "2026-10-20T18:11:57Z".to_string(),
        }
    }

    /// The delivery finding of one report, if the doctor produced one.
    fn delivery_finding(report: &crate::doctor::DoctorReport) -> Option<&Finding> {
        report.findings.iter().find(|f| f.id == "delivery-state")
    }

    #[test]
    fn a_healthy_delivery_passes_and_names_the_delivery_evidence() {
        let tmp = TempDir::new().unwrap();
        write(tmp.path(), "forge.yaml", &rust_manifest("delivery-healthy"));
        write(tmp.path(), "Cargo.toml", "[package]\nname = \"demo\"\n");
        let obs = observation_with(Some(recorded(DeliveryPhase::Healthy)));
        let report = run_doctor(tmp.path(), None, Some(&obs), None).unwrap();
        let finding = delivery_finding(&report).expect("delivery-state finding");
        assert_eq!(finding.status, FindingStatus::Pass);
        assert!(finding.applicable);
        assert_eq!(finding.remediation, Remediation::Manual);
        let evidence = finding.evidence.join(" ");
        assert!(evidence.contains("phase=healthy"), "{evidence}");
        assert!(evidence.contains("environment=stage"), "{evidence}");
        assert!(
            evidence.contains("delivered revision=0123456789abcdef0123456789abcdef01234567"),
            "{evidence}"
        );
        assert!(evidence.contains("2026-10-20T18:11:57Z"), "{evidence}");
        assert!(
            finding.detail.contains("forge delivery status <project>"),
            "{}",
            finding.detail
        );
    }

    #[test]
    fn a_failed_delivery_fails_the_doctor() {
        for phase in [DeliveryPhase::StageFailed, DeliveryPhase::Degraded] {
            let tmp = TempDir::new().unwrap();
            write(tmp.path(), "forge.yaml", &rust_manifest("delivery-failed"));
            write(tmp.path(), "Cargo.toml", "[package]\nname = \"demo\"\n");
            let obs = observation_with(Some(recorded(phase)));
            let report = run_doctor(tmp.path(), None, Some(&obs), None).unwrap();
            let finding = delivery_finding(&report).expect("delivery-state finding");
            assert_eq!(finding.status, FindingStatus::Fail, "{phase}");
            assert!(finding.applicable, "{phase}");
            assert_eq!(finding.remediation, Remediation::Manual, "{phase}");
            assert!(
                finding.evidence.iter().any(|e| e.contains(phase.label())),
                "{:?}",
                finding.evidence
            );
            assert!(
                report
                    .blocking_findings()
                    .iter()
                    .any(|f| f.id == "delivery-state"),
                "a failed delivery denies health: {phase}"
            );
        }
    }

    #[test]
    fn a_delivery_in_progress_warns_without_denying_health() {
        for phase in [
            DeliveryPhase::AwaitingProductionApproval,
            DeliveryPhase::HermoraFailed,
        ] {
            let tmp = TempDir::new().unwrap();
            write(
                tmp.path(),
                "forge.yaml",
                &rust_manifest("delivery-progress"),
            );
            write(tmp.path(), "Cargo.toml", "[package]\nname = \"demo\"\n");
            let obs = observation_with(Some(recorded(phase)));
            let report = run_doctor(tmp.path(), None, Some(&obs), None).unwrap();
            let finding = delivery_finding(&report).expect("delivery-state finding");
            assert_eq!(finding.status, FindingStatus::Warn, "{phase}");
            assert!(finding.applicable, "{phase}");
            assert!(
                !report
                    .blocking_findings()
                    .iter()
                    .any(|f| f.id == "delivery-state"),
                "a delivery in progress is a fact, not a fault: {phase}"
            );
        }
    }

    #[test]
    fn a_project_without_a_delivery_is_reported_as_not_applicable() {
        let tmp = TempDir::new().unwrap();
        write(tmp.path(), "forge.yaml", &rust_manifest("delivery-absent"));
        write(tmp.path(), "Cargo.toml", "[package]\nname = \"demo\"\n");
        let obs = observation_with(Some(DeliveryObservation::Absent));
        let report = run_doctor(tmp.path(), None, Some(&obs), None).unwrap();
        let finding = delivery_finding(&report).expect("delivery-state finding");
        assert_eq!(finding.status, FindingStatus::Pass);
        assert!(!finding.applicable);
        assert_eq!(finding.remediation, Remediation::Automatic);
        assert!(
            finding
                .evidence
                .iter()
                .any(|e| e.contains("no delivery has been recorded")),
            "{:?}",
            finding.evidence
        );
        assert!(
            !report
                .blocking_findings()
                .iter()
                .any(|f| f.id == "delivery-state"),
            "an absent delivery never denies health"
        );
    }

    #[test]
    fn an_unreadable_delivery_state_is_unavailable_and_names_the_reason() {
        let tmp = TempDir::new().unwrap();
        write(
            tmp.path(),
            "forge.yaml",
            &rust_manifest("delivery-unreadable"),
        );
        write(tmp.path(), "Cargo.toml", "[package]\nname = \"demo\"\n");
        let obs = observation_with(Some(DeliveryObservation::Unreadable {
            reason: "registry read failed: token=abcdef0123456789".to_string(),
        }));
        let report = run_doctor(tmp.path(), None, Some(&obs), None).unwrap();
        let finding = delivery_finding(&report).expect("delivery-state finding");
        assert_eq!(finding.status, FindingStatus::Unavailable);
        assert!(!finding.applicable);
        assert_eq!(finding.remediation, Remediation::Automatic);
        let evidence = finding.evidence.join(" ");
        assert!(evidence.contains("registry read failed"), "{evidence}");
        assert!(evidence.contains("[REDACTED]"), "{evidence}");
        assert!(!evidence.contains("abcdef0123456789"), "{evidence}");
        assert!(
            report
                .blocking_findings()
                .iter()
                .any(|f| f.id == "delivery-state"),
            "a state that cannot be read is never healthy"
        );
    }

    #[test]
    fn a_caller_that_did_not_look_produces_no_delivery_finding() {
        let tmp = TempDir::new().unwrap();
        write(
            tmp.path(),
            "forge.yaml",
            &rust_manifest("delivery-unlooked"),
        );
        write(tmp.path(), "Cargo.toml", "[package]\nname = \"demo\"\n");
        let obs = observation_with(None);
        let report = run_doctor(tmp.path(), None, Some(&obs), None).unwrap();
        assert!(
            delivery_finding(&report).is_none(),
            "the doctor never guesses what it did not read"
        );
    }

    #[test]
    fn an_unregistered_project_gets_no_delivery_finding() {
        let tmp = TempDir::new().unwrap();
        write(
            tmp.path(),
            "forge.yaml",
            &rust_manifest("delivery-unregistered"),
        );
        write(tmp.path(), "Cargo.toml", "[package]\nname = \"demo\"\n");
        let obs = RegistryObservation {
            registered: false,
            observed_at: None,
            delivery: Some(recorded(DeliveryPhase::StageFailed)),
        };
        let report = run_doctor(tmp.path(), None, Some(&obs), None).unwrap();
        assert!(
            delivery_finding(&report).is_none(),
            "an unregistered project is reported by registry-observation instead"
        );
    }
}
