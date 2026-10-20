//! The one contract-version rule.
//!
//! Every extension boundary in Forge declares the contract it
//! speaks. This module owns the single way to read such a
//! declaration and the single rule that decides whether the
//! declaration satisfies a requirement: the families must be
//! equal and the majors must be equal. The minor and the patch
//! are reported, but they do not decide.

use std::fmt;

use serde::{Deserialize, Serialize};

/// A declared contract version: a family and a
/// `major.minor.patch` triple.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContractVersion {
    /// The contract family (e.g. `forge-deploy-executor`). A
    /// contract without a family cannot be compared with one
    /// that has, so the family is mandatory.
    pub family: String,
    /// The major version. A major bump is a deliberate break of
    /// the wire format.
    pub major: u32,
    /// The minor version. A minor bump is an addition the old
    /// reader can ignore.
    pub minor: u32,
    /// The patch version.
    pub patch: u32,
}

/// Why a declared contract version cannot be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractVersionError {
    /// The offending value, so a refusal can name it.
    pub value: String,
    /// Why the value cannot be read.
    pub reason: String,
}

impl fmt::Display for ContractVersionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.reason)
    }
}

impl std::error::Error for ContractVersionError {}

impl ContractVersion {
    /// Build a declared version directly.
    pub fn new(family: &str, major: u32, minor: u32, patch: u32) -> Self {
        Self {
            family: family.to_string(),
            major,
            minor,
            patch,
        }
    }

    /// Read `family/major.minor.patch`.
    ///
    /// The family is the part before the first `/`. A value
    /// without a `/` carries no family and is therefore
    /// unreadable as a contract, so the bare `0.1.0` form can
    /// never masquerade as conformant.
    pub fn parse(raw: &str) -> Result<Self, ContractVersionError> {
        let Some((family, version)) = raw.split_once('/') else {
            return Err(ContractVersionError {
                value: raw.to_string(),
                reason: format!(
                    "the version `{raw}` does not name a contract family; expected `family/major.minor.patch`"
                ),
            });
        };
        if family.is_empty() {
            return Err(ContractVersionError {
                value: raw.to_string(),
                reason: format!(
                    "the contract family in `{raw}` is empty; expected `family/major.minor.patch`"
                ),
            });
        }
        if family.chars().any(char::is_whitespace) {
            return Err(ContractVersionError {
                value: raw.to_string(),
                reason: format!(
                    "the contract family `{family}` in `{raw}` contains whitespace; expected `family/major.minor.patch`"
                ),
            });
        }
        let parts: Vec<&str> = version.split('.').collect();
        if parts.len() != 3 {
            return Err(ContractVersionError {
                value: raw.to_string(),
                reason: format!(
                    "the version `{version}` in `{raw}` does not have three parts; expected `family/major.minor.patch`"
                ),
            });
        }
        let mut numbers = [0u32; 3];
        for (slot, part) in numbers.iter_mut().zip(parts.iter()) {
            *slot = part.parse::<u32>().map_err(|_| ContractVersionError {
                value: raw.to_string(),
                reason: format!(
                    "the version part `{part}` in `{raw}` is not a number; expected `family/major.minor.patch`"
                ),
            })?;
        }
        Ok(Self {
            family: family.to_string(),
            major: numbers[0],
            minor: numbers[1],
            patch: numbers[2],
        })
    }

    /// The one rule: the families are equal and the majors are
    /// equal. The minor and the patch are reported, but they do
    /// not decide.
    pub fn is_compatible_with(&self, required: &ContractVersion) -> bool {
        self.family == required.family && self.major == required.major
    }

    /// Why the declaration does not satisfy the requirement, in
    /// the order the families are checked first. `None` when the
    /// declaration is compatible.
    pub fn incompatibility_detail(&self, required: &ContractVersion) -> Option<String> {
        if self.family != required.family {
            return Some(format!(
                "the families differ (`{}` and `{}`)",
                self.family, required.family
            ));
        }
        if self.major != required.major {
            return Some(format!(
                "the major versions differ ({} and {})",
                self.major, required.major
            ));
        }
        None
    }

    /// The version without the family.
    pub fn version(&self) -> String {
        format!("{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl fmt::Display for ContractVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.family, self.version())
    }
}
