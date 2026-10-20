//! The shared contract verification.
//!
//! Every extension kind runs this one verification, so a new kind
//! inherits the rules instead of restating them.

use super::state::{ExtensionDescriptor, ExtensionState};
use super::version::ContractVersion;

/// Judge an extension's declaration against the contract it must
/// satisfy.
pub fn verify_extension(
    descriptor: &ExtensionDescriptor,
    required: &ContractVersion,
    executable: bool,
) -> ExtensionState {
    descriptor.state(required, executable)
}
