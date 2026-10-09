//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use super::engine::default_enabled;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::PathBuf;

/// What the plugin reported back for a metadata propose request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MetadataProposeResponse {
    pub contract: String,
    #[serde(default)]
    pub operation_id: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub changes: Vec<Value>,
    /// The reviewed change set's PR reference (URL or id).
    #[serde(default)]
    pub pr: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
}
#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq, Eq)]
pub struct ProviderConfig {
    #[serde(default)]
    pub providers: Vec<ProviderEntry>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublishProviderResponse {
    pub contract: String,
    pub provider: String,
    pub operation_id: String,
    pub status: String,
    pub health: String,
    #[serde(default)]
    pub evidence: Vec<String>,
    #[serde(default)]
    pub recovery: Vec<String>,
    /// Optional echo of the request `queue_id`. Present only when the
    /// request carried one; used by Forge to match terminal responses
    /// back to the originating fleet run.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub queue_id: Option<String>,
    /// Optional echo of the committed Git revision the provider
    /// actually transferred. When the provider omits it the Forge
    /// status projection assumes the request's revision (so the
    /// additive fields never widen the contract for legacy providers).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision: Option<String>,
    /// Terminal status of the `build` phase (`succeeded`, `failed`,
    /// `not_started`, `unknown`). Missing fields render as `unknown`
    /// in the status projection — a response without phase evidence
    /// is not a verified success.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub build_status: Option<String>,
    /// Terminal status of the `run` phase (same vocabulary as
    /// `build_status`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_status: Option<String>,
    /// Bounded container / Compose project identity the provider
    /// actually deployed (canonical `forge-<project>-<sha12>` for
    /// sibling providers that follow the contract; absent when the
    /// provider cannot prove the identity).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub container_identity: Option<String>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderOperation {
    Capabilities,
    Preflight,
    Publish,
    Verify,
    Rollback,
}
/// Outcome of validating a single progress event line emitted by a
/// publish provider on its stderr stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProgressEventDecision {
    /// Event shape was valid and matched the active request envelope.
    /// Carries the bounded, redacted detail string (always bounded by
    /// [`PROGRESS_DETAIL_MAX`]).
    Accepted { detail: String },
    /// Event shape was wrong (not an object, wrong contract, missing
    /// field, malformed queue_id, …). The runtime's own reason is the
    /// payload so the operator can read what the provider claimed.
    Malformed { reason: String },
    /// Event was structurally fine but claimed a different
    /// operation_id / project_id / queue_id / contract than the
    /// active request — a provider protocol violation.
    Mismatched,
    /// Event type was something other than `publish.progress`. The
    /// line is ignored without surfacing a diagnostic so the transport
    /// stays quiet when a provider mixes informational lines.
    Ignored,
}
/// Request to propose a metadata change through a plugin. `mode`
/// is always `"pr"` from `classify apply`: direct mutation is not
/// reachable from that path.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MetadataProposeRequest {
    pub contract: String,
    pub operation_id: String,
    pub project_id: String,
    pub fields: BTreeMap<String, Value>,
    pub mode: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublishProviderRequest {
    pub contract: String,
    pub operation: ProviderOperation,
    pub provider: String,
    pub project_id: String,
    pub revision: String,
    pub operation_id: String,
    #[serde(default)]
    pub folder: Option<String>,
    #[serde(default)]
    pub dry_run: bool,
    /// Optional fleet run identifier. Carried through to the response
    /// and used by Forge to group per-project progress events under a
    /// single fleet invocation. Validated for shape (1..=128 chars,
    /// ASCII alphanumeric plus `-` and `_`) but never persisted by
    /// the provider; providers MUST echo it verbatim when present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub queue_id: Option<String>,
}
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct ProviderEntry {
    pub id: String,
    pub command: PathBuf,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ProviderContractError {
    #[error("provider request is not an object")]
    NotObject,
    #[error("provider contract must be {0}")]
    ContractMismatch(String),
    #[error("provider field `{0}` is required")]
    MissingField(&'static str),
    #[error("provider response contains a secret-like value")]
    SecretLeak,
    #[error("provider queue_id `{0}` is not 1..=128 ASCII alphanumeric/`-`/`_` characters")]
    QueueIdShape(String),
    #[error("provider revision `{0}` is not a 40-character hex SHA")]
    RevisionShape(String),
    #[error(
        "provider build_status `{0}` is not one of `succeeded`/`failed`/`not_started`/`unknown`"
    )]
    BuildStatusShape(String),
    #[error(
        "provider run_status `{0}` is not one of `succeeded`/`failed`/`not_started`/`unknown`"
    )]
    RunStatusShape(String),
    #[error("provider container_identity `{0}` must not be empty and must be <= 256 characters")]
    ContainerIdentityShape(String),
    #[error("provider progress event `{0}` is missing or invalid")]
    ProgressShape(&'static str),
    #[error("provider progress event claims a different operation_id than the active request")]
    ProgressOperationMismatch,
    #[error("provider progress event claims a different project_id than the active request")]
    ProgressProjectMismatch,
    #[error("provider progress event claims a different queue_id than the active request")]
    ProgressQueueMismatch,
    #[error("provider progress event claims a different contract than the active request")]
    ProgressContractMismatch,
}
