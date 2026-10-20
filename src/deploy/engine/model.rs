//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use super::super::{
    deploy_executor_contract, CapturedArtifact, DeployHealthSpec, DeployTargetSpec, STATUS_FAILED,
    STATUS_UNKNOWN,
};
use crate::core::ForgeError;
use crate::extension::contract::verify_extension;
use crate::extension::state::ExtensionDescriptor;
use crate::extension::version::ContractVersion;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Which executor verb Forge invokes. `apply` runs (or
/// rehearses, with `--dry-run`) the named deployment;
/// `observe` is read-only health status and must never
/// trigger a deploy side effect. The frozen argv for each
/// operation is documented in
/// `docs/adapter-contracts/deploy-executor.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AdapterOp {
    Apply,
    Observe,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct AdapterRequest {
    pub(super) contract: String,
    pub(super) project_id: String,
    pub(super) deploy_id: String,
    pub(super) target: DeployTargetSpec,
    pub(super) artifact: Option<CapturedArtifact>,
    pub(super) health: Option<DeployHealthSpec>,
    pub(super) revision: String,
    pub(super) dry_run: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct AdapterResponse {
    pub(super) contract: String,
    pub(super) apply_status: String,
    pub(super) apply_note: String,
    pub(super) apply_evidence: Vec<String>,
    pub(super) observation_status: String,
    pub(super) observation_detail: String,
    pub(super) observation_evidence: Vec<String>,
    pub(super) recovery: Vec<String>,
    /// Optional adapter self-identification (e.g.
    /// `forge-deployer-jenkins/0.1.0`), read into the family and
    /// the version it names. Forge attributes the stage evidence
    /// to it when present.
    pub(super) source: Option<ContractVersion>,
    /// Optional revision of the adapter's backing runtime
    /// scripts (e.g. the jenkins-local checkout's HEAD).
    /// Absent never reads as a claimed production version.
    pub(super) source_revision: Option<String>,
}
impl AdapterResponse {
    pub(super) fn parse(raw: &str) -> Result<Self, ForgeError> {
        let value: serde_json::Value =
            serde_json::from_str(raw).map_err(|err| ForgeError::DeployTargetUnavailable {
                reason: format!("deploy adapter returned non-JSON output: {err}"),
            })?;
        let obj = value
            .as_object()
            .ok_or_else(|| ForgeError::DeployTargetUnavailable {
                reason: "deploy adapter payload is not a JSON object".to_string(),
            })?;
        let contract = obj
            .get("contract")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        // The declared contract is read with the shared rule: the
        // executor family at the required major. A newer minor of
        // the same family satisfies it; a different family or a
        // different major does not.
        let declared = ContractVersion::parse(&contract).map_err(|err| {
            ForgeError::DeployTargetUnavailable {
                reason: format!(
                    "deploy adapter contract `{contract}` cannot be read: {}",
                    err.reason
                ),
            }
        })?;
        // The adapter's self-identification is a claim. A claim
        // that cannot be read is worse than no claim at all, so it
        // is refused; an absent claim stays absent and the
        // configured binary is named instead.
        let source = obj
            .get("source")
            .and_then(|v| v.as_str())
            .filter(|s| !s.trim().is_empty())
            .map(|raw_source| {
                ContractVersion::parse(raw_source).map_err(|err| {
                    ForgeError::DeployTargetUnavailable {
                        reason: format!(
                            "deploy adapter self-identification `{raw_source}` cannot be read: {}",
                            err.reason
                        ),
                    }
                })
            })
            .transpose()?;
        let id = source
            .as_ref()
            .map(|claimed| claimed.family.clone())
            .unwrap_or_else(|| "deploy adapter".to_string());
        let descriptor = ExtensionDescriptor {
            id,
            contract: declared,
            enabled: true,
        };
        let state = verify_extension(&descriptor, &deploy_executor_contract(), true);
        if !state.is_ready() {
            return Err(ForgeError::DeployTargetUnavailable {
                reason: state
                    .reason()
                    .unwrap_or("the deploy adapter does not satisfy the executor contract")
                    .to_string(),
            });
        }
        Ok(AdapterResponse {
            contract,
            apply_status: obj
                .get("apply_status")
                .and_then(|v| v.as_str())
                .unwrap_or(STATUS_FAILED)
                .to_string(),
            apply_note: obj
                .get("apply_note")
                .and_then(|v| v.as_str())
                .unwrap_or("deploy adapter did not provide a note")
                .to_string(),
            apply_evidence: obj
                .get("apply_evidence")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default(),
            observation_status: obj
                .get("observation_status")
                .and_then(|v| v.as_str())
                .unwrap_or(STATUS_UNKNOWN)
                .to_string(),
            observation_detail: obj
                .get("observation_detail")
                .and_then(|v| v.as_str())
                .unwrap_or("deploy adapter did not provide an observation detail")
                .to_string(),
            observation_evidence: obj
                .get("observation_evidence")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default(),
            recovery: obj
                .get("recovery")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default(),
            source,
            source_revision: obj
                .get("source_revision")
                .and_then(|v| v.as_str())
                .filter(|s| !s.trim().is_empty())
                .map(|s| s.to_string()),
        })
    }
    /// Attribution line naming the executor and its revision
    /// so every stage outcome is traceable to the adapter
    /// that produced it. An absent self-identification
    /// surfaces as the configured binary plus `unknown`
    /// revision — never a claimed version.
    pub(super) fn attribution(&self, fallback_bin: &str) -> String {
        let source = match &self.source {
            Some(claimed) => format!("{}/{}", claimed.family, claimed.version()),
            None => Path::new(fallback_bin)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| fallback_bin.to_string()),
        };
        let revision = self
            .source_revision
            .clone()
            .unwrap_or_else(|| "unknown".to_string());
        format!("executor={source}@{revision}")
    }
}
