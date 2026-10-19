//! `platform.gate-result` payload.

use serde::{Deserialize, Serialize};

/// Gate result document.
///
/// The wire shape mirrors `schemas/gate-result.schema.json` in the
/// `platform-contracts` repository. The Rust type is a permissive
/// builder; the JSON Schema is the source of truth for what is
/// accepted on the wire.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateResult {
    /// Stable gate identifier.
    pub gate_id: String,
    /// Pipeline the gate guards, e.g. `deploy-prod`.
    pub pipeline: String,
    /// RFC3339 timestamp the gate was evaluated.
    pub evaluated_at: String,
    /// Overall gate outcome.
    pub result: GateResultValue,
    /// Identifier of the run, build, or change set being gated.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject_ref: Option<String>,
    /// Per-check outcomes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checks: Option<Vec<GateCheck>>,
    /// References to supporting evidence documents.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence_refs: Option<Vec<String>>,
    /// Optional tenant identifier for multi-tenant pipelines.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<String>,
}

/// A single gate check entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateCheck {
    /// Check identifier.
    pub check_id: String,
    /// Check outcome.
    pub status: GateResultValue,
    /// Optional human-readable detail. Must not exceed 512 characters.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// Gate outcome value. Mirrors the `result` / `checks.status` enums in
/// the platform schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GateResultValue {
    /// The gate/check passed.
    Passed,
    /// The gate/check failed.
    Failed,
    /// The gate/check errored.
    Errored,
    /// The gate/check was skipped.
    Skipped,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimal_value_round_trips() {
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
        let json = serde_json::to_value(&value).expect("serializes");
        for key in ["gate_id", "pipeline", "evaluated_at", "result"] {
            assert!(json.get(key).is_some(), "missing required key {key}");
        }
        let back: GateResult = serde_json::from_value(json).expect("deserializes");
        assert_eq!(back, value);
    }
}
