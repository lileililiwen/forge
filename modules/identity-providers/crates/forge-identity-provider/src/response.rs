//! The response half of the contract.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::request::ProviderOperation;

/// The terminal status of a provider operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderStatus {
    /// The operation completed and the population is usable.
    Ready,
    /// The operation cannot proceed without an owner decision.
    Blocked,
    /// The operation failed.
    Failed,
    /// The provider or its runtime is not reachable.
    Unavailable,
}

impl ProviderStatus {
    /// The wire name of the status.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Blocked => "blocked",
            Self::Failed => "failed",
            Self::Unavailable => "unavailable",
        }
    }
}

/// One project or provider change the plan will make.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposedChange {
    /// The change kind, for example `manifest.auth` or `provider.realm`.
    pub kind: String,
    /// The file or provider object the change touches.
    pub target: String,
    /// A human readable, secret-free summary.
    pub summary: String,
}

/// The end-user population the provider created for one project.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Population {
    /// The provider-scoped population identifier.
    pub population_id: String,
    /// The OIDC issuer for the population.
    pub issuer: String,
    /// The OIDC client id for the population.
    pub client_id: String,
    /// The callback path the provider redirects to.
    pub callback_path: String,
    /// The scopes the client requests.
    #[serde(default)]
    pub scopes: Vec<String>,
    /// The registration page for the population, when the provider has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registration_url: Option<String>,
}

/// A recovery action the owner can take after a failure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryAction {
    /// A stable machine-readable reason code.
    pub reason: String,
    /// A human readable, secret-free instruction.
    pub instruction: String,
}

/// One response line read from a pack.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderResponse {
    /// The contract identifier, always `forge-identity-provider/1.0`.
    pub contract: String,
    /// The provider pack that answered.
    pub provider_id: String,
    /// The operation identifier from the request.
    pub operation_id: String,
    /// The project from the request.
    pub project_id: String,
    /// The project revision from the request.
    pub project_revision: String,
    /// The operation that was answered.
    pub operation: ProviderOperation,
    /// The terminal status.
    pub status: ProviderStatus,
    /// A secret-free summary of the outcome.
    #[serde(default)]
    pub summary: String,
    /// The digest of the plan, present for `plan` and `apply`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan_digest: Option<String>,
    /// The changes the plan will make.
    #[serde(default)]
    pub proposed_changes: Vec<ProposedChange>,
    /// The population the operation produced, when the operation produced one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub population: Option<Population>,
    /// References to the secrets the population uses. Never the values.
    #[serde(default)]
    pub secret_references: Vec<String>,
    /// The recovery actions for a blocked or failed operation.
    #[serde(default)]
    pub recovery: Vec<RecoveryAction>,
    /// Provider specific detail that is not part of the contract.
    #[serde(default)]
    pub details: Value,
}
