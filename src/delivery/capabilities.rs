//! The delivery lane's capability probe and verb gate.
//!
//! Every delivery verb asks the provider what it can do, and refuses a
//! verb the provider did not declare. The probe runs after the verb's
//! local gates and before its operation is reserved, so a refusal writes
//! no journal row, moves no phase and runs no requested operation.
//!
//! See `openspec/specs/delivery-provider-capabilities/spec.md`.

use std::path::Path;

use crate::core::ForgeError;
use crate::publish::providers::{self, CapabilityDeclaration, ProviderEntry};

use super::invoke::operation_for;
use super::state::DeliveryVerb;

/// The bare verb name used in a refusal sentence.
fn verb_name(verb: DeliveryVerb) -> &'static str {
    match verb {
        DeliveryVerb::Preflight => "preflight",
        DeliveryVerb::Stage => "stage",
        DeliveryVerb::Promote => "promote",
        DeliveryVerb::Rollback => "rollback",
        DeliveryVerb::Hermora => "hermora",
    }
}

/// Probe the provider and gate the verb on its declaration.
///
/// `preflight` is never gated: it is the entry verb, and a provider that
/// cannot preflight fails on its own. Every other verb requires the
/// provider's operation for it to be supported. An unknown declaration
/// keeps the contract's existing floor, which the declaration itself
/// decides.
pub fn gate(
    provider: &ProviderEntry,
    project_id: &str,
    revision: &str,
    project_dir: &Path,
    verb: DeliveryVerb,
) -> Result<CapabilityDeclaration, ForgeError> {
    let operation_id = format!(
        "delivery-capabilities-{project_id}-{}",
        revision.chars().take(12).collect::<String>()
    );
    let declaration =
        providers::probe_capabilities(provider, project_id, revision, project_dir, &operation_id)
            .map_err(|err| ForgeError::DeliveryInvalid {
            reason: err.to_string(),
        })?;
    let required = operation_for(verb);
    if verb != DeliveryVerb::Preflight && !declaration.supports(required) {
        return Err(ForgeError::DeliveryInvalid {
            reason: format!(
                "delivery {}: publish provider `{}` does not declare the `{}` capability ({}); \
                 refused before the requested verb ran",
                verb_name(verb),
                provider.id,
                required.as_str(),
                declaration.summary()
            ),
        });
    }
    Ok(declaration)
}
