//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use super::super::{
    CapturedArtifact, DeployHealthSpec, DeployTargetSpec, DEPLOY_EXECUTOR_CONTRACT, STATUS_FAILED,
    STATUS_UNKNOWN,
};
use crate::core::ForgeError;
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
    /// `forge-deployer-jenkins/0.1.0`). Forge attributes the
    /// stage evidence to it when present.
    pub(super) source: Option<String>,
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
        let contract = value
            .get("contract")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if contract != DEPLOY_EXECUTOR_CONTRACT {
            return Err(ForgeError::DeployTargetUnavailable {
                reason: format!(
                    "deploy adapter contract `{contract}` does not match expected `{DEPLOY_EXECUTOR_CONTRACT}`"
                ),
            });
        }
        let obj = value
            .as_object()
            .ok_or_else(|| ForgeError::DeployTargetUnavailable {
                reason: "deploy adapter payload is not a JSON object".to_string(),
            })?;
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
            source: obj
                .get("source")
                .and_then(|v| v.as_str())
                .filter(|s| !s.trim().is_empty())
                .map(|s| s.to_string()),
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
        let source = self.source.clone().unwrap_or_else(|| {
            Path::new(fallback_bin)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| fallback_bin.to_string())
        });
        let revision = self
            .source_revision
            .clone()
            .unwrap_or_else(|| "unknown".to_string());
        format!("executor={source}@{revision}")
    }
}
