//! Platform contract envelope.
//!
//! The envelope wraps every contract document with a `contract`
//! identifier, a `generated_at` timestamp, and the contract-specific
//! `payload`. See `schemas/envelope.schema.json` in the
//! `platform-contracts` repository for the authoritative shape.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

/// Error returned by [`parse_contract_identifier`] when the input does
/// not match the expected `platform.<family>/<semver>` form.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ContractIdError {
    /// The identifier does not contain a `/` separating the family
    /// from the version.
    #[error("contract identifier {0:?} is missing a '/' between family and version")]
    MissingSlash(String),
    /// The version segment is not `<major>.<minor>.<patch>`.
    #[error("contract identifier {0:?} has a non-semver version segment {1:?}")]
    NonSemver(String, String),
    /// A version component is not a valid non-negative integer.
    #[error("contract identifier {0:?} has a non-integer version component {1:?}")]
    NonInteger(String, String),
}

impl From<ContractIdError> for crate::registry::RegistryError {
    fn from(value: ContractIdError) -> Self {
        Self::Malformed(value.to_string())
    }
}

/// Semantic version: `(major, minor, patch)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ContractVersion {
    /// Major version.
    pub major: u32,
    /// Minor version.
    pub minor: u32,
    /// Patch version.
    pub patch: u32,
}

impl ContractVersion {
    /// Construct a version from its three numeric components.
    #[must_use]
    pub const fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }
}

impl fmt::Display for ContractVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl FromStr for ContractVersion {
    type Err = ContractIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.split('.');
        let major = parts
            .next()
            .and_then(|p| p.parse::<u32>().ok())
            .ok_or_else(|| ContractIdError::NonInteger(s.to_string(), "major".to_string()))?;
        let minor = parts
            .next()
            .and_then(|p| p.parse::<u32>().ok())
            .ok_or_else(|| ContractIdError::NonInteger(s.to_string(), "minor".to_string()))?;
        let patch = parts
            .next()
            .and_then(|p| p.parse::<u32>().ok())
            .ok_or_else(|| ContractIdError::NonInteger(s.to_string(), "patch".to_string()))?;
        if parts.next().is_some() {
            return Err(ContractIdError::NonSemver(
                s.to_string(),
                "too many components".to_string(),
            ));
        }
        Ok(Self {
            major,
            minor,
            patch,
        })
    }
}

/// Parse a `platform.<family>/<semver>` identifier.
///
/// # Errors
///
/// Returns [`ContractIdError::MissingSlash`] if the input does not
/// contain `/`, [`ContractIdError::NonSemver`] if the right-hand side
/// is not three dot-separated components, or
/// [`ContractIdError::NonInteger`] if a version component is not a
/// valid non-negative integer.
pub fn parse_contract_identifier(
    contract: &str,
) -> Result<(String, ContractVersion), ContractIdError> {
    let (family, version) = contract
        .split_once('/')
        .ok_or_else(|| ContractIdError::MissingSlash(contract.to_string()))?;
    let version: ContractVersion = version.parse()?;
    Ok((family.to_string(), version))
}

/// The shared envelope wrapping every contract document.
///
/// The envelope is generic over the payload type. The payload's Rust
/// type is opaque to the envelope; the wire shape is enforced by the
/// JSON Schema in `platform-contracts`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Envelope<P> {
    /// Contract identifier in the form `platform.<family>/<semver>`.
    pub contract: String,
    /// RFC3339 UTC timestamp at which the producer emitted the
    /// document.
    pub generated_at: OffsetDateTime,
    /// Contract-specific payload. Validated against the per-family
    /// JSON Schema at the receiving end.
    pub payload: P,
}

impl<P> Envelope<P> {
    /// Construct a new envelope for the given contract identifier
    /// and payload. The caller is responsible for supplying an
    /// `OffsetDateTime` (typically from a `Clock`).
    #[must_use]
    pub fn new(contract: impl Into<String>, generated_at: OffsetDateTime, payload: P) -> Self {
        Self {
            contract: contract.into(),
            generated_at,
            payload,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_supported_identifier() {
        let (family, version) =
            parse_contract_identifier("platform.identity-subject/0.1.0").unwrap();
        assert_eq!(family, "platform.identity-subject");
        assert_eq!(version, ContractVersion::new(0, 1, 0));
    }

    #[test]
    fn rejects_missing_slash() {
        assert!(matches!(
            parse_contract_identifier("platform.identity-subject"),
            Err(ContractIdError::MissingSlash(_))
        ));
    }

    #[test]
    fn rejects_non_semver() {
        let err = parse_contract_identifier("platform.x/1.0").unwrap_err();
        assert!(matches!(err, ContractIdError::NonInteger(_, _)));
    }

    #[test]
    fn rejects_letters_in_version() {
        let err = parse_contract_identifier("platform.x/1.0.x").unwrap_err();
        assert!(matches!(err, ContractIdError::NonInteger(_, _)));
    }

    #[test]
    fn envelope_roundtrip() {
        let env: Envelope<serde_json::Value> = Envelope::new(
            "platform.identity-subject/0.1.0",
            OffsetDateTime::UNIX_EPOCH,
            serde_json::json!({"subject_id": "usr_1"}),
        );
        let json = serde_json::to_string(&env).unwrap();
        let back: Envelope<serde_json::Value> = serde_json::from_str(&json).unwrap();
        assert_eq!(back.contract, env.contract);
        assert_eq!(back.generated_at, env.generated_at);
        assert_eq!(back.payload, env.payload);
    }
}
