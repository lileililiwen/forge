//! Versioned feature lifecycle (`feature-lifecycle`).
//!
//! Core owns the feature catalog, the dependency resolver and the
//! add/remove/upgrade operations; transports render Core outcomes without
//! reinterpreting them.
//!
//! The catalog pins one tested version per feature ([`TESTED_VERSION`],
//! aligned with the MVP profiles and the deterministic generator). Only
//! tested profile mappings are installable: compatibility is derived from
//! the profile descriptors, and an unsupported mapping fails as
//! `incompatible-feature` instead of inventing an implementation.
//!
//! Lifecycle operations mutate only the `features:` map of the canonical
//! `forge.yaml` (all other manifest sections are preserved byte-for-byte at
//! the data level) plus one deterministic ownership receipt per installed
//! feature under `.forge/features/`. A receipt is a pure function of its
//! feature identity, so a receipt that no longer matches means user-owned
//! edits are present and the mutating operation blocks instead of
//! overwriting them. Removal additionally blocks on reverse dependencies.
//! Every mutation is validated after the write (manifest re-parse plus full
//! graph re-resolution) and then refreshed into the registry; a validation
//! failure restores the prior manifest bytes and receipts.
//!
//! Package installation itself stays per-service native resolution (the
//! offline-portable scaffolds ship dependency-free by design); declared
//! policy validators are reported with each plan while DriftWatch execution
//! evidence stays deferred to `quality-policy-integration`.

pub mod contract;
pub mod engine;
pub mod engine_tests;
pub mod model;
pub mod render;

// Re-export all types
pub use contract::*;
pub use engine::*;
pub use model::*;
pub use render::*;
