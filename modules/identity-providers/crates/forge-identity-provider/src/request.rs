//! The request half of the contract.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The contract identifier a pack and Forge Core agree on.
pub const CONTRACT_ID: &str = "forge-identity-provider/1.0";

/// The major version of the contract this crate implements.
pub const CONTRACT_MAJOR: u32 = 1;

/// The operation a pack is asked to perform.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderOperation {
    /// Report what the provider can do for the project. Never mutates.
    Probe,
    /// Produce a side-effect free plan. Never mutates.
    Plan,
    /// Perform the planned change, bound to the plan digest.
    Apply,
    /// Report the current state of the provider population.
    Status,
    /// Stop the provider runtime for the project.
    Stop,
}

impl ProviderOperation {
    /// The wire name of the operation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Probe => "probe",
            Self::Plan => "plan",
            Self::Apply => "apply",
            Self::Status => "status",
            Self::Stop => "stop",
        }
    }
}

/// One request line written to a pack.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderRequest {
    /// The contract identifier, always `forge-identity-provider/1.0`.
    pub contract: String,
    /// The provider pack that owns this request.
    pub provider_id: String,
    /// The caller-chosen operation identifier, echoed in the response.
    pub operation_id: String,
    /// The project the operation belongs to.
    pub project_id: String,
    /// The project revision the operation was planned against.
    pub project_revision: String,
    /// The operation to perform.
    pub operation: ProviderOperation,
    /// The operation input. Never carries a secret value.
    #[serde(default)]
    pub input: Value,
}

impl ProviderRequest {
    /// Builds a request for one operation.
    pub fn new(
        provider_id: &str,
        operation_id: &str,
        project_id: &str,
        project_revision: &str,
        operation: ProviderOperation,
        input: Value,
    ) -> Self {
        Self {
            contract: CONTRACT_ID.to_string(),
            provider_id: provider_id.to_string(),
            operation_id: operation_id.to_string(),
            project_id: project_id.to_string(),
            project_revision: project_revision.to_string(),
            operation,
            input,
        }
    }
}
