//! Project-to-production delivery workflow.
//!
//! This module coordinates the staged publication of a registered
//! project through the existing `forge-publish-provider/0.1.0`
//! contract, plus the optional post-deployment Hermora site
//! enrollment. The package is a thin orchestration layer: Forge owns
//! the state machine, the journal projection, the CLI/API/UI
//! surfaces, and the typed refusal vocabulary; OpenPanel owns the
//! deploy/verify provider and Hermora owns site enrollment.
//!
//! Persistence uses the existing `operations` table — no new
//! schema. Every verb is encoded as a row with a closed `kind`
//! (`delivery.preflight`, `delivery.stage`, `delivery.promote`,
//! `delivery.hermora`) and a deterministic idempotency key built
//! from `(project_id, revision, environment)`.
//!
//! See `openspec/specs/project-to-production-workflow/spec.md` for
//! the contract requirements and `openspec/changes/project-to-
//! production-workflow/design.md` for the implementation decisions.

pub mod cli;
pub mod handlers;
pub mod hermora;
pub mod invoke;
pub mod projection;
pub mod state;

use std::fmt;

/// Machine contract for the read-only delivery projection and the
/// CLI/API response shape. Pinned to the spec the OpenSpec change
/// owns (`forge-delivery-status/0.1.0`).
pub const DELIVERY_CONTRACT_VERSION: &str = "forge-delivery-status/0.1.0";

/// The four closed environment labels the workflow recognises.
/// Stage and production are the only two runtime environments; the
/// draft phase carries no environment label.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DeliveryEnvironment {
    Stage,
    Production,
}

impl DeliveryEnvironment {
    pub fn label(&self) -> &'static str {
        match self {
            DeliveryEnvironment::Stage => "stage",
            DeliveryEnvironment::Production => "production",
        }
    }
}

impl fmt::Display for DeliveryEnvironment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// Error raised when an input (CLI flag or API body field) is
/// missing or out of range. Surfaced as `delivery-invalid` (HTTP
/// 400) on every transport.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeliveryInputError {
    pub reason: String,
}

impl DeliveryInputError {
    pub fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
        }
    }
}

impl std::fmt::Display for DeliveryInputError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.reason)
    }
}

impl std::error::Error for DeliveryInputError {}

impl From<DeliveryInputError> for crate::core::ForgeError {
    fn from(err: DeliveryInputError) -> Self {
        crate::core::ForgeError::DeliveryInvalid { reason: err.reason }
    }
}
