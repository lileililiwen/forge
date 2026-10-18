use forge::ci_evidence::{HostedCiEvidenceEnvelope, HostedCiRun};
use forge::gate::{GateAggregate, GateCheckState, GateEvidence};

fn valid() -> HostedCiEvidenceEnvelope {
    HostedCiEvidenceEnvelope {
        contract: "forge-hosted-ci-evidence/1.0".into(),
        project_id: "sample-service".into(),
        repository: "owner/repository".into(),
        run: HostedCiRun {
            provider: "github-actions".into(),
            workflow: "ci.yml".into(),
            run_id: "123456789".into(),
            run_attempt: 1,
            job: "gate".into(),
            url: "https://github.com/owner/repository/actions/runs/123456789".into(),
        },
        source_revision: "a".repeat(40),
        gate: GateEvidence {
            contract: forge::gate::GATE_CONTRACT_VERSION.into(),
            project_id: "sample-service".into(),
            runtime: "forge".into(),
            runtime_version: Some("0.1.0".into()),
            revision: Some("a".repeat(40)),
            manifest_digest: None,
            rule_pack_version: Some("baseline-service@1.4.0".into()),
            aggregate: GateAggregate::Passed,
            checks: vec![forge::gate::GateCheck {
                id: "baseline".into(),
                state: GateCheckState::Pass,
                note: None,
            }],
            observed_at: "2026-10-18T10:00:00Z".into(),
            dry_run: false,
            note: None,
        },
        observed_at: "2026-10-18T10:01:00Z".into(),
    }
}

#[test]
fn valid_envelope_round_trips_and_validates() {
    let value = valid();
    value.validate().unwrap();
    let encoded = serde_json::to_string(&value).unwrap();
    let decoded: HostedCiEvidenceEnvelope = serde_json::from_str(&encoded).unwrap();
    decoded.validate().unwrap();
    assert_eq!(decoded, value);
}

#[test]
fn rejects_project_revision_and_rehearsal_contradictions() {
    let mut value = valid();
    value.gate.project_id = "other-project".into();
    assert!(value.validate().is_err());
    let mut value = valid();
    value.gate.revision = Some("b".repeat(40));
    assert!(value.validate().is_err());
    let mut value = valid();
    value.gate.dry_run = true;
    assert!(value.validate().is_err());
}

#[test]
fn rejects_unsafe_or_mismatched_run_urls() {
    for url in [
        "http://github.com/owner/repository/actions/runs/123456789",
        "https://attacker.example/owner/repository/actions/runs/123456789",
        "https://user:pass@github.com/owner/repository/actions/runs/123456789",
        "https://github.com/owner/repository/actions/runs/123456789?token=x",
        "https://github.com/owner/repository/actions/runs/123456788",
    ] {
        let mut value = valid();
        value.run.url = url.into();
        assert!(value.validate().is_err(), "accepted {url}");
    }
}

#[test]
fn rejects_unknown_envelope_fields() {
    let mut json = serde_json::to_value(valid()).unwrap();
    json["secret"] = serde_json::json!("must not be accepted");
    assert!(serde_json::from_value::<HostedCiEvidenceEnvelope>(json).is_err());
}

#[test]
fn rejects_unknown_aggregate_and_unbounded_gate_details() {
    let mut value = valid();
    value.gate.aggregate = GateAggregate::Unknown;
    assert!(value.validate().is_err());
    let mut value = valid();
    value.gate.checks[0].note = Some("x".repeat(300));
    assert!(value.validate().is_err());
}

#[test]
fn rejects_non_utc_or_malformed_observation_time() {
    let mut value = valid();
    value.observed_at = "2026-10-18T10:01:00+08:00".into();
    assert!(value.validate().is_err());
    let mut value = valid();
    value.gate.observed_at = "not-a-time".into();
    assert!(value.validate().is_err());
}

#[test]
fn refuses_credential_shaped_gate_diagnostics() {
    let mut value = valid();
    value.gate.checks[0].note =
        Some("leaked token ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghij".into());
    assert!(value.validate().is_err());
}
