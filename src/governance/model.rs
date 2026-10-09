//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use serde::{Deserialize, Serialize};

use super::contract::LOCAL_PROVIDER_ID;
use super::engine::{default_enabled, default_protocol_version, default_timeout_ms};

#[derive(Debug, Deserialize)]
pub(super) struct AdapterObservation {
    #[serde(default)]
    pub(super) provider: Option<String>,
    pub(super) protocol_version: String,
    pub(super) project_id: String,
    pub(super) status: String,
    #[serde(default)]
    pub(super) source_revision: Option<String>,
    #[serde(default)]
    pub(super) evidence: Vec<String>,
    #[serde(default)]
    pub(super) detail: Option<String>,
    #[serde(default)]
    pub(super) metadata: serde_json::Value,
}
/// The outcome of one bounded run: the child's exit status, both drained pipes,
/// and whether Forge stopped waiting first.
#[derive(Debug)]
pub(super) struct BoundedRun {
    pub(super) status: std::process::ExitStatus,
    pub(super) stdout: PipeDrain,
    pub(super) stderr: PipeDrain,
    pub(super) timed_out: bool,
    /// A descendant inherited a pipe and still held it open past the deadline,
    /// so its bytes are incomplete. Recorded rather than assumed away.
    pub(super) truncated: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProviderStatus {
    Pass,
    Fail,
    Blocked,
    Unknown,
    Unavailable,
    Stale,
    Disabled,
    Incompatible,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GovernanceStatus(pub ProviderStatus);
impl GovernanceStatus {
    pub fn is_healthy(self) -> bool {
        matches!(self.0, ProviderStatus::Pass)
    }
}
/// One drained pipe.
///
/// `bytes` holds the first `cap` bytes and `total` counts **every** byte the
/// child wrote, so the caller can enforce its cap without the reader having to
/// stop. That distinction is the whole point: a reader that stopped at `cap`
/// would block the child on its next write and re-create the deadlock this
/// drain exists to remove.
#[derive(Clone, Debug)]
pub(super) struct PipeDrain {
    pub(super) bytes: Vec<u8>,
    pub(super) total: usize,
    pub(super) error: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GovernanceObservation {
    pub provider: String,
    pub protocol_version: String,
    pub project_id: String,
    pub project_path: String,
    pub status: ProviderStatus,
    pub observed_at: String,
    #[serde(default)]
    pub source_revision: Option<String>,
    #[serde(default)]
    pub evidence: Vec<String>,
    #[serde(default)]
    pub detail: Option<String>,
    #[serde(default)]
    pub metadata: serde_json::Value,
}
#[derive(Debug, Serialize)]
pub(super) struct AdapterRequest<'a> {
    pub(super) contract: &'static str,
    pub(super) action: &'static str,
    pub(super) project_id: &'a str,
    pub(super) project_path: &'a str,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GovernanceProviderDescriptor {
    pub provider: String,
    pub configured: bool,
    pub enabled: bool,
    pub adapter: Option<String>,
    pub protocol_version: String,
}
/// Which pipe a drain result belongs to.
#[derive(Clone, Copy)]
pub(super) enum Pipe {
    Stdout,
    Stderr,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct GovernanceConfig {
    #[serde(default)]
    pub provider: Option<GovernanceProviderConfig>,
}
impl GovernanceConfig {
    pub fn selected_provider(&self) -> &str {
        self.provider
            .as_ref()
            .map(|provider| provider.provider.as_str())
            .unwrap_or(LOCAL_PROVIDER_ID)
    }
}
pub(super) struct AdapterOutput {
    pub(super) status: std::process::ExitStatus,
    pub(super) stdout: Vec<u8>,
    pub(super) stderr: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GovernanceProviderConfig {
    pub provider: String,
    #[serde(default)]
    pub adapter: Option<String>,
    /// Explicit workspace root recorded at selection time. When set it is
    /// passed to the adapter as `WORKSPACE_ROOT` at run time, so later
    /// checks never depend on the environment staying set. Absent-when-
    /// unset keeps every pre-change selection file byte-identical.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_root: Option<String>,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default = "default_protocol_version")]
    pub protocol_version: String,
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
}
