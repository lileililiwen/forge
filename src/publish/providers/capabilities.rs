//! The publish provider's declared capabilities.
//!
//! The provider contract declares five operations. A provider answers
//! the `capabilities` operation with the subset it serves; Forge gates
//! the verbs that follow on that answer. Three readings are possible and
//! only two of them are a claim:
//!
//! - a valid list is the provider's own statement, enforced literally;
//! - no list, no answer, or no readable response is *unknown*, which
//!   permits the two operations every provider has served since `0.1.0`
//!   and permits nothing newer;
//! - a list Forge cannot read is refused, because a claim Forge cannot
//!   check is worse than no claim.

use std::collections::BTreeSet;

use super::model::{ProviderOperation, PublishProviderResponse};
use super::ProviderContractError;

/// The largest capability list a provider may declare.
pub const CAPABILITIES_MAX: usize = 16;

/// The longest capability name a provider may declare.
pub const CAPABILITY_NAME_MAX: usize = 32;

/// What a provider said it can do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CapabilityDeclaration {
    /// The provider answered and declared these operations.
    Declared(BTreeSet<ProviderOperation>),
    /// The provider made no readable claim, and why.
    Unknown { reason: String },
}

impl CapabilityDeclaration {
    /// Read a provider response as a declaration.
    ///
    /// The response has already passed `validate_response`, so a list
    /// that is present is well formed. Names this build does not know
    /// are ignored: an older Forge must not refuse a newer provider for
    /// adding an operation.
    pub fn from_response(response: &PublishProviderResponse) -> Self {
        match response.capabilities.as_deref() {
            Some(names) => Self::Declared(
                names
                    .iter()
                    .filter_map(|name| ProviderOperation::parse(name))
                    .collect(),
            ),
            None => Self::Unknown {
                reason: "the provider declared no capabilities".to_string(),
            },
        }
    }

    /// The provider made no readable claim.
    pub fn unknown(reason: impl Into<String>) -> Self {
        Self::Unknown {
            reason: reason.into(),
        }
    }

    /// Whether the provider may receive `operation`.
    ///
    /// An unknown provider keeps the contract's existing floor:
    /// `preflight` and `publish`. Every other operation requires the
    /// provider to have declared it.
    pub fn supports(&self, operation: ProviderOperation) -> bool {
        match self {
            Self::Declared(operations) => operations.contains(&operation),
            Self::Unknown { .. } => matches!(
                operation,
                ProviderOperation::Preflight | ProviderOperation::Publish
            ),
        }
    }

    /// A stable, operator-readable rendering of the declaration.
    pub fn summary(&self) -> String {
        match self {
            Self::Declared(operations) if operations.is_empty() => "declared: none".to_string(),
            Self::Declared(operations) => format!(
                "declared: {}",
                operations
                    .iter()
                    .map(|operation| operation.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Self::Unknown { reason } => format!("unknown: {reason}"),
        }
    }

    /// The bounded journal value recorded beside a delivery verb.
    pub fn detail(&self) -> serde_json::Value {
        match self {
            Self::Declared(operations) => serde_json::json!({
                "state": "declared",
                "operations": operations
                    .iter()
                    .map(|operation| operation.as_str())
                    .collect::<Vec<_>>(),
            }),
            Self::Unknown { reason } => serde_json::json!({
                "state": "unknown",
                "reason": reason,
            }),
        }
    }
}

/// Validate a declared capability list.
///
/// The rules are the ones the contract document states: a bounded count,
/// a bounded lower-case name shape, and no duplicates. A name this build
/// does not recognise is not a validation failure; see
/// [`CapabilityDeclaration::from_response`].
pub fn validate_capabilities(names: &[String]) -> Result<(), ProviderContractError> {
    if names.len() > CAPABILITIES_MAX {
        return Err(ProviderContractError::CapabilityCount(
            names.len(),
            CAPABILITIES_MAX,
        ));
    }
    let mut seen = BTreeSet::new();
    for name in names {
        let shaped = !name.is_empty()
            && name.len() <= CAPABILITY_NAME_MAX
            && name
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-');
        if !shaped {
            return Err(ProviderContractError::CapabilityShape(name.clone()));
        }
        if !seen.insert(name.as_str()) {
            return Err(ProviderContractError::CapabilityDuplicate(name.clone()));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::publish::providers::PUBLISH_PROVIDER_CONTRACT;

    fn response(capabilities: Option<Vec<&str>>) -> PublishProviderResponse {
        PublishProviderResponse {
            contract: PUBLISH_PROVIDER_CONTRACT.to_string(),
            provider: "openpanel".to_string(),
            operation_id: "probe-1".to_string(),
            status: "done".to_string(),
            health: "unknown".to_string(),
            evidence: Vec::new(),
            recovery: Vec::new(),
            queue_id: None,
            revision: None,
            build_status: None,
            run_status: None,
            container_identity: None,
            capabilities: capabilities.map(|names| names.into_iter().map(str::to_string).collect()),
        }
    }

    #[test]
    fn an_absent_list_is_unknown() {
        let declaration = CapabilityDeclaration::from_response(&response(None));
        assert!(matches!(declaration, CapabilityDeclaration::Unknown { .. }));
    }

    #[test]
    fn a_declared_list_is_read_as_a_set() {
        let declaration =
            CapabilityDeclaration::from_response(&response(Some(vec!["preflight", "publish"])));
        assert!(declaration.supports(ProviderOperation::Preflight));
        assert!(declaration.supports(ProviderOperation::Publish));
        assert!(!declaration.supports(ProviderOperation::Rollback));
        assert_eq!(declaration.summary(), "declared: preflight, publish");
    }

    #[test]
    fn an_unknown_name_is_ignored_not_refused() {
        let declaration =
            CapabilityDeclaration::from_response(&response(Some(vec!["publish", "preview"])));
        assert!(declaration.supports(ProviderOperation::Publish));
        assert!(!declaration.supports(ProviderOperation::Rollback));
    }

    #[test]
    fn an_empty_list_declares_nothing() {
        let declaration = CapabilityDeclaration::from_response(&response(Some(Vec::new())));
        assert!(!declaration.supports(ProviderOperation::Preflight));
        assert!(!declaration.supports(ProviderOperation::Publish));
        assert_eq!(declaration.summary(), "declared: none");
    }

    #[test]
    fn unknown_permits_the_existing_floor_only() {
        let declaration = CapabilityDeclaration::unknown("no answer");
        assert!(declaration.supports(ProviderOperation::Preflight));
        assert!(declaration.supports(ProviderOperation::Publish));
        assert!(!declaration.supports(ProviderOperation::Verify));
        assert!(!declaration.supports(ProviderOperation::Rollback));
        assert_eq!(declaration.summary(), "unknown: no answer");
    }

    #[test]
    fn a_duplicate_name_is_refused() {
        let names = vec!["publish".to_string(), "publish".to_string()];
        assert_eq!(
            validate_capabilities(&names),
            Err(ProviderContractError::CapabilityDuplicate(
                "publish".to_string()
            ))
        );
    }

    #[test]
    fn an_out_of_shape_name_is_refused() {
        for name in ["", "Publish", "pub lish", "pub-lish!", &"x".repeat(33)] {
            let names = vec![name.to_string()];
            assert_eq!(
                validate_capabilities(&names),
                Err(ProviderContractError::CapabilityShape(name.to_string())),
                "name {name:?} must be refused"
            );
        }
    }

    #[test]
    fn too_many_names_are_refused() {
        let names: Vec<String> = (0..=CAPABILITIES_MAX).map(|n| format!("op{n}")).collect();
        assert_eq!(
            validate_capabilities(&names),
            Err(ProviderContractError::CapabilityCount(
                CAPABILITIES_MAX + 1,
                CAPABILITIES_MAX
            ))
        );
    }

    #[test]
    fn every_contract_operation_round_trips() {
        for operation in ProviderOperation::ALL {
            assert_eq!(
                ProviderOperation::parse(operation.as_str()),
                Some(operation)
            );
        }
        assert_eq!(ProviderOperation::parse("preview"), None);
    }
}
