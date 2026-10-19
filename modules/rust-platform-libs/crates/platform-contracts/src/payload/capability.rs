//! `platform.capability` payload.

use serde::{Deserialize, Serialize};

/// Capability declaration document.
///
/// The wire shape mirrors `schemas/capability.schema.json` in the
/// `platform-contracts` repository. The Rust type is a permissive
/// builder; the JSON Schema is the source of truth for what is
/// accepted on the wire.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Capability {
    /// Stable capability identifier, e.g. `identity.read`.
    pub capability_id: String,
    /// Service declaring the capability.
    pub service: String,
    /// Declared version (semver).
    pub version: String,
    /// RFC3339 timestamp the capability was declared.
    pub declared_at: String,
    /// Stability level of the capability.
    pub stability: Stability,
    /// Contract references this capability produces or consumes, in
    /// `platform.<family>/<semver>` form.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contract_refs: Option<Vec<String>>,
    /// Optional human-readable description. Must not exceed 512
    /// characters.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Stability level of a capability. Mirrors the `stability` enum in
/// the platform schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stability {
    /// Stable and supported.
    Stable,
    /// Experimental and subject to change.
    Experimental,
    /// Deprecated and scheduled for removal.
    Deprecated,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimal_value_round_trips() {
        let value = Capability {
            capability_id: "identity.read".to_string(),
            service: "identity-admin".to_string(),
            version: "1.0.0".to_string(),
            declared_at: "2026-01-01T00:00:00Z".to_string(),
            stability: Stability::Stable,
            contract_refs: None,
            description: None,
        };
        let json = serde_json::to_value(&value).expect("serializes");
        for key in [
            "capability_id",
            "service",
            "version",
            "declared_at",
            "stability",
        ] {
            assert!(json.get(key).is_some(), "missing required key {key}");
        }
        let back: Capability = serde_json::from_value(json).expect("deserializes");
        assert_eq!(back, value);
    }
}
