//! `platform.release-evidence` payload.

use serde::{Deserialize, Serialize};

/// Release evidence document.
///
/// The wire shape mirrors `schemas/release-evidence.schema.json` in
/// the `platform-contracts` repository. The Rust type is a permissive
/// builder; the JSON Schema is the source of truth for what is
/// accepted on the wire.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseEvidence {
    /// Stable release identifier.
    pub release_id: String,
    /// Service the release belongs to.
    pub service: String,
    /// Released version (semver).
    pub version: String,
    /// RFC3339 timestamp the release was published.
    pub released_at: String,
    /// Kind of evidence this document carries.
    pub evidence_kind: EvidenceKind,
    /// References to release artifacts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact_refs: Option<Vec<String>>,
    /// References to the gates evaluated for this release.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gate_refs: Option<Vec<String>>,
    /// Optional tenant identifier for multi-tenant releases.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<String>,
}

/// Kind of release evidence. Mirrors the `evidence_kind` enum in the
/// platform schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    /// A production deploy.
    Deploy,
    /// A rollback.
    Rollback,
    /// A dry run (no mutation performed).
    DryRun,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimal_value_round_trips() {
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
        let json = serde_json::to_value(&value).expect("serializes");
        for key in [
            "release_id",
            "service",
            "version",
            "released_at",
            "evidence_kind",
        ] {
            assert!(json.get(key).is_some(), "missing required key {key}");
        }
        let back: ReleaseEvidence = serde_json::from_value(json).expect("deserializes");
        assert_eq!(back, value);
    }
}
