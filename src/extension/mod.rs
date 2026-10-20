//! The shared extension contract (`extension-point-contract`).
//!
//! Forge has nine extension boundaries and they used to share
//! nothing: each one invented its own descriptor, its own version
//! rule and its own state vocabulary, and every version check was
//! exact string equality. A newer extension was therefore refused
//! exactly like an unrelated one.
//!
//! This module owns the shared contract:
//!
//! - [`version::ContractVersion`] reads a declared contract
//!   version and judges it against a required one. The families
//!   must be equal and the majors must be equal; the minor and the
//!   patch are reported, but they do not decide.
//! - [`state::ExtensionState`] is the one vocabulary: `ready`,
//!   `disabled`, `unavailable` and `invalid`, and every refusal
//!   carries a reason.
//! - [`kinds`] owns the deploy target kinds and the one wording
//!   that refuses a kind this build does not implement.
//! - [`contract::verify_extension`] is the one verification every
//!   kind runs.
//!
//! The deploy executor boundary is the first reader. The other
//! extension surfaces converge onto this contract one at a time.

pub mod contract;
pub mod extension_tests;
pub mod kinds;
pub mod state;
pub mod version;

pub use contract::verify_extension;
pub use kinds::{DeployTargetKind, DEPLOY_TARGET_KINDS};
pub use state::{ExtensionDescriptor, ExtensionState};
pub use version::{ContractVersion, ContractVersionError};
