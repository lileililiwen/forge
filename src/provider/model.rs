//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use super::contract::PROVIDER_CONTRACT_VERSION;
use super::matrix::{display_for, redact_provider_evidence};
use super::probes::teardown_probe;

/// Static descriptor for one evidence provider (no probe performed).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderDescriptor {
    pub contract: String,
    pub provider: String,
    pub display: String,
    pub boundary: String,
    pub binary_env: Option<String>,
    pub default_binary: Option<String>,
    pub secret_rule: String,
    pub teardown_rule: String,
}
/// Shared attribution for one probe outcome. Bundles the parameters
/// every row builder needs so each row carries identical provenance
/// shape.
pub(super) struct RowParams<'a> {
    pub(super) provider: &'a str,
    pub(super) sandbox: SandboxKind,
    pub(super) source: &'a str,
    pub(super) project_id: &'a str,
    pub(super) revision: Option<String>,
    pub(super) temp_dir: Option<&'a Path>,
}
impl RowParams<'_> {
    pub(super) fn supported(
        self,
        tool_version: Option<String>,
        receipt: Vec<String>,
        evidence: Vec<String>,
        note: &str,
    ) -> ProviderRow {
        let teardown = self.temp_dir.map(teardown_probe).unwrap_or(true);
        ProviderRow {
            contract: PROVIDER_CONTRACT_VERSION.to_string(),
            provider: self.provider.to_string(),
            display: display_for(self.provider).to_string(),
            status: ProviderStatus::Supported.id().to_string(),
            reason: format!(
                "controlled {sandbox} round trip succeeded for project `{project}`; same outcome is exposed through every transport",
                sandbox = self.sandbox.id(), project = self.project_id,
            ),
            provenance: Some(EvidenceProvenance {
                provider: self.provider.to_string(),
                sandbox: self.sandbox.id().to_string(),
                source: self.source.to_string(),
                project_id: Some(self.project_id.to_string()),
                revision: self.revision,
                observed_at: Utc::now().to_rfc3339(),
                tool_version,
                receipt,
                teardown,
                note: redact_provider_evidence(note),
            }),
            evidence,
        }
    }
}
/// How the caller wants one controlled round trip to run.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RunOptions {
    /// Attempt the real sandbox binary (requires `FORGE_PROVIDER_LIVE=1`
    /// unless `allow_unflagged_live` is set by tests).
    pub live: bool,
    /// Explicit fixture binary standing in for the sandbox. Labels the
    /// row `sandbox: fixture` so fixture proof never reads as provider
    /// support.
    pub fixture: Option<PathBuf>,
    /// Bypass the `FORGE_PROVIDER_LIVE` gate (tests only).
    pub allow_unflagged_live: bool,
}
/// Which sandbox produced the evidence. `fixture` rows are supplemental
/// harness proof; only `live` rows can verify real provider support.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SandboxKind {
    Live,
    Fixture,
}
impl SandboxKind {
    pub fn id(&self) -> &'static str {
        match self {
            SandboxKind::Live => "live",
            SandboxKind::Fixture => "fixture",
        }
    }
}
pub(super) struct CapturedRun {
    pub(super) exit_code: Option<i32>,
    pub(super) stdout: String,
    pub(super) stderr: String,
    pub(super) timed_out: bool,
}
/// Attributable provenance for one controlled round trip.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EvidenceProvenance {
    pub provider: String,
    pub sandbox: String,
    pub source: String,
    pub project_id: Option<String>,
    pub revision: Option<String>,
    pub observed_at: String,
    pub tool_version: Option<String>,
    pub receipt: Vec<String>,
    pub teardown: bool,
    pub note: String,
}
/// The provider matrix over all six evidence providers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderMatrix {
    pub contract: String,
    pub generated_at: String,
    pub live: bool,
    pub rows: Vec<ProviderRow>,
    pub supported: usize,
    pub unavailable: usize,
    pub not_run: usize,
    pub disabled: usize,
}
/// Native evidence classification for one provider row. `not-run` is the
/// default: the sandbox was never attempted. `supported` requires a
/// controlled round trip with provenance; `unavailable` records a live
/// attempt that failed without fabricating health; `disabled` preserves
/// the manifest-disabled semantics of the underlying adapter.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ProviderStatus {
    Supported,
    Unavailable,
    NotRun,
    Disabled,
}
impl ProviderStatus {
    pub fn id(&self) -> &'static str {
        match self {
            ProviderStatus::Supported => "supported",
            ProviderStatus::Unavailable => "unavailable",
            ProviderStatus::NotRun => "not-run",
            ProviderStatus::Disabled => "disabled",
        }
    }
}
/// One provider evidence row.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderRow {
    pub contract: String,
    pub provider: String,
    pub display: String,
    pub status: String,
    pub reason: String,
    pub provenance: Option<EvidenceProvenance>,
    pub evidence: Vec<String>,
}
