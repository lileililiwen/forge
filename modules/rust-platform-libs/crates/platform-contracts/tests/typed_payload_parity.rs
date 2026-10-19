//! Typed payload wire-compatibility tests.
//!
//! Each typed `platform-contracts` payload for the operational families
//! must serialize to a document the registry-backed validator accepts.
//! Forge's canonical contract tree is resolved the same way as in
//! `fixture_parity.rs`: `PLATFORM_CONTRACTS_DIR` first, then
//! `contracts/` relative to the Forge repository root.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use platform_contracts::{
    validate_document, Capability, ContractRegistry, EvidenceKind, GateResult, GateResultValue,
    JobOutcome, JobResult, ReleaseEvidence, Stability,
};
use serde_json::json;

fn platform_contracts_dir() -> PathBuf {
    if let Ok(env_path) = env::var("PLATFORM_CONTRACTS_DIR") {
        return PathBuf::from(env_path);
    }
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace = manifest
        .parent()
        .and_then(Path::parent)
        .expect("crate path is at least two levels deep");
    let forge_root = workspace
        .parent()
        .and_then(Path::parent)
        .expect("module path has a Forge repository parent");
    forge_root.join("contracts")
}

fn assert_envelope_valid(contract: &str, payload: serde_json::Value) {
    let dir = platform_contracts_dir();
    let result = if dir.join("registry.json").is_file() && dir.join("family-support.json").is_file()
    {
        fs::read_to_string(dir.join("registry.json"))
            .map_err(platform_contracts::RegistryError::Io)
            .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).map_err(Into::into))
            .and_then(|value| ContractRegistry::from_value(dir.clone(), value))
    } else {
        ContractRegistry::load(&dir)
    };
    let registry = result.unwrap_or_else(|err| {
        panic!(
            "BLOCKED: cannot load a supported contract registry from {}: {err}",
            dir.display()
        )
    });
    let document = json!({
        "contract": contract,
        "generated_at": "2026-01-01T00:00:00Z",
        "payload": payload,
    });
    let result = validate_document(&document, &registry).expect("validator runs");
    assert!(
        result.valid,
        "{contract} typed payload rejected: {:?}",
        result.errors
    );
}

#[test]
fn gate_result_typed_payload_is_wire_valid() {
    let value = GateResult {
        gate_id: "gate_basic".to_string(),
        pipeline: "deploy-prod".to_string(),
        evaluated_at: "2026-01-01T00:00:00Z".to_string(),
        result: GateResultValue::Passed,
        subject_ref: None,
        checks: None,
        evidence_refs: None,
        tenant_id: None,
    };
    assert_envelope_valid(
        "platform.gate-result/0.1.0",
        serde_json::to_value(&value).unwrap(),
    );
}

#[test]
fn job_outcome_typed_payload_is_wire_valid() {
    let dir = platform_contracts_dir();
    if dir.join("registry.json").is_file() && dir.join("family-support.json").is_file() {
        let support: serde_json::Value = serde_json::from_slice(
            &fs::read(dir.join("family-support.json")).expect("Forge family support exists"),
        )
        .expect("Forge family support is valid JSON");
        let entry = support["families"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["family"] == "platform.job-outcome")
            .expect("Forge family support names job-outcome");
        assert_eq!(entry["status"], "unsupported");
        assert!(entry["reason"]
            .as_str()
            .is_some_and(|reason| !reason.is_empty()));
        return;
    }
    let value = JobOutcome {
        job_id: "job_basic".to_string(),
        job_type: "data_import".to_string(),
        started_at: "2026-01-01T00:00:00Z".to_string(),
        finished_at: None,
        result: JobResult::Succeeded,
        items_processed: None,
        items_failed: None,
        tenant_id: None,
    };
    assert_envelope_valid(
        "platform.job-outcome/0.1.0",
        serde_json::to_value(&value).unwrap(),
    );
}

#[test]
fn release_evidence_typed_payload_is_wire_valid() {
    let value = ReleaseEvidence {
        release_id: "rel_basic".to_string(),
        service: "identity-admin".to_string(),
        version: "1.2.3".to_string(),
        released_at: "2026-01-01T00:00:00Z".to_string(),
        evidence_kind: EvidenceKind::Deploy,
        artifact_refs: None,
        gate_refs: None,
        tenant_id: None,
    };
    assert_envelope_valid(
        "platform.release-evidence/0.1.0",
        serde_json::to_value(&value).unwrap(),
    );
}

#[test]
fn capability_typed_payload_is_wire_valid() {
    let value = Capability {
        capability_id: "identity.read".to_string(),
        service: "identity-admin".to_string(),
        version: "1.0.0".to_string(),
        declared_at: "2026-01-01T00:00:00Z".to_string(),
        stability: Stability::Stable,
        contract_refs: None,
        description: None,
    };
    assert_envelope_valid(
        "platform.capability/0.1.0",
        serde_json::to_value(&value).unwrap(),
    );
}
