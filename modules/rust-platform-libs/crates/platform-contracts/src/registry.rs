//! Contract registry loader.
//!
//! The registry is a single JSON document published by the
//! `platform-contracts` repository at `schemas/registry.json`. It
//! lists every contract family, its per-family JSON Schema path, the
//! list of supported major versions, status fields, security-sensitive
//! fields, the runtime that owns the contract, and the set of secret
//! field-name substrings that the redaction policy rejects.
//!
//! This crate never invents a registry of its own; the foundation
//! loads the live document at runtime. If the file is missing or
//! malformed the loader returns a [`RegistryError`] and tests fail
//! loudly rather than silently passing on a stale snapshot.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// One entry in the registry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContractEntry {
    /// Path to the JSON Schema (relative to the repository root).
    pub schema: String,
    /// Major versions this consumer/runtime supports.
    pub supported_major_versions: Vec<u32>,
    /// Current published version of the contract.
    pub current_version: String,
    /// Status fields whose meaning is part of the contract.
    #[serde(default)]
    pub status_fields: Vec<String>,
    /// Security-sensitive field paths; new fields under these paths
    /// trigger a `compatibility.security_field_added` warning.
    #[serde(default)]
    pub security_sensitive_fields: Vec<String>,
    /// The runtime that owns this contract.
    pub owner_runtime: String,
    /// Short human-readable description.
    pub description: String,
}

impl ContractEntry {
    /// Returns true if `major` is in `supported_major_versions`.
    #[must_use]
    pub fn supports_major(&self, major: u32) -> bool {
        self.supported_major_versions.contains(&major)
    }
}

/// Error returned by the registry loader.
#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
    /// The registry file does not exist.
    #[error("registry file not found: {0}")]
    NotFound(PathBuf),
    /// The registry file is not valid JSON.
    #[error("registry file is not valid JSON: {0}")]
    NotJson(#[from] serde_json::Error),
    /// The registry file is missing a required key.
    #[error("registry is missing required key {0:?}")]
    MissingKey(String),
    /// The registry has no `contracts` object.
    #[error("registry is missing a non-empty 'contracts' object")]
    NoContracts,
    /// The registry has no secret substrings.
    #[error("registry must declare at least one secret_field_substrings entry")]
    NoSecretSubstrings,
    /// The contract identifier was malformed.
    #[error("malformed contract identifier: {0}")]
    Malformed(String),
    /// I/O error.
    #[error("I/O error reading registry: {0}")]
    Io(#[from] std::io::Error),
}

/// Loaded registry. Cheap to clone.
#[derive(Debug, Clone)]
pub struct ContractRegistry {
    /// Path to the directory containing the registry (used to
    /// resolve relative schema paths).
    pub root: PathBuf,
    contracts: BTreeMap<String, ContractEntry>,
    secret_substrings: Vec<String>,
}

impl ContractRegistry {
    /// Load the registry from the given `path` (a directory
    /// containing `schemas/registry.json`).
    ///
    /// # Errors
    ///
    /// Returns [`RegistryError`] if the file does not exist, the JSON
    /// is malformed, or the registry is missing required keys.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, RegistryError> {
        let root = path.as_ref().to_path_buf();
        let registry_path = root.join("schemas").join("registry.json");
        if !registry_path.is_file() {
            return Err(RegistryError::NotFound(registry_path));
        }
        let raw = fs::read_to_string(&registry_path)?;
        let value: serde_json::Value = serde_json::from_str(&raw)?;
        Self::from_value(root, value)
    }

    /// Build a registry from an already-parsed JSON value.
    ///
    /// # Errors
    ///
    /// Returns [`RegistryError`] if the value is missing required keys.
    pub fn from_value(root: PathBuf, value: serde_json::Value) -> Result<Self, RegistryError> {
        let contracts_raw = value
            .get("contracts")
            .and_then(|v| v.as_object())
            .ok_or(RegistryError::NoContracts)?;
        if contracts_raw.is_empty() {
            return Err(RegistryError::NoContracts);
        }
        let mut contracts = BTreeMap::new();
        for (contract_id, entry_value) in contracts_raw {
            let entry: ContractEntry = serde_json::from_value(entry_value.clone())?;
            contracts.insert(contract_id.clone(), entry);
        }
        let secret_substrings = value
            .get("secret_field_substrings")
            .and_then(|v| v.as_array())
            .ok_or(RegistryError::NoSecretSubstrings)?
            .iter()
            .filter_map(|v| v.as_str().map(str::to_lowercase))
            .collect::<Vec<_>>();
        if secret_substrings.is_empty() {
            return Err(RegistryError::NoSecretSubstrings);
        }
        Ok(Self {
            root,
            contracts,
            secret_substrings,
        })
    }

    /// Resolve a contract entry by family identifier
    /// (e.g. `platform.identity-subject`).
    #[must_use]
    pub fn get(&self, family: &str) -> Option<&ContractEntry> {
        self.contracts.get(family)
    }

    /// Returns true if `family` is a known contract family.
    #[must_use]
    pub fn is_known(&self, family: &str) -> bool {
        self.contracts.contains_key(family)
    }

    /// Returns true if the family exists and supports `major`.
    #[must_use]
    pub fn is_major_supported(&self, family: &str, major: u32) -> bool {
        self.contracts
            .get(family)
            .is_some_and(|e| e.supports_major(major))
    }

    /// Iterates over every known contract family identifier.
    pub fn contract_ids(&self) -> impl Iterator<Item = &str> {
        self.contracts.keys().map(String::as_str)
    }

    /// Secret-name substrings that the redaction policy rejects.
    #[must_use]
    pub fn secret_substrings(&self) -> &[String] {
        &self.secret_substrings
    }

    /// Absolute path to the per-family JSON Schema for `family`.
    #[must_use]
    pub fn schema_path(&self, family: &str) -> Option<PathBuf> {
        self.contracts
            .get(family)
            .map(|e| self.root.join(&e.schema))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
        "contracts": {
            "platform.identity-subject": {
                "schema": "schemas/identity-subject.schema.json",
                "supported_major_versions": [0],
                "current_version": "0.1.0",
                "status_fields": ["subject_type"],
                "security_sensitive_fields": ["subject_id", "tenant_id"],
                "owner_runtime": "identity-rs",
                "description": "Test."
            }
        },
        "secret_field_substrings": ["password", "token"]
    }"#;

    #[test]
    fn loads_from_value() {
        let value: serde_json::Value = serde_json::from_str(SAMPLE).unwrap();
        let reg = ContractRegistry::from_value(PathBuf::from("/tmp"), value).unwrap();
        assert!(reg.is_known("platform.identity-subject"));
        assert!(reg.is_major_supported("platform.identity-subject", 0));
        assert!(!reg.is_major_supported("platform.identity-subject", 1));
        let entry = reg.get("platform.identity-subject").unwrap();
        assert_eq!(entry.current_version, "0.1.0");
        assert_eq!(
            entry.security_sensitive_fields,
            vec!["subject_id", "tenant_id"]
        );
    }

    #[test]
    fn rejects_missing_secret_substrings() {
        let sample = r#"{"contracts": {"x": {"schema": "x", "supported_major_versions": [0], "current_version": "0.0.0", "owner_runtime": "x", "description": "x"}}}"#;
        let value: serde_json::Value = serde_json::from_str(sample).unwrap();
        let err = ContractRegistry::from_value(PathBuf::from("/tmp"), value).unwrap_err();
        assert!(matches!(err, RegistryError::NoSecretSubstrings));
    }

    #[test]
    fn rejects_empty_contracts() {
        let sample = r#"{"contracts": {}, "secret_field_substrings": ["x"]}"#;
        let value: serde_json::Value = serde_json::from_str(sample).unwrap();
        let err = ContractRegistry::from_value(PathBuf::from("/tmp"), value).unwrap_err();
        assert!(matches!(err, RegistryError::NoContracts));
    }
}
