//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::ffi::OsString;
use std::time::Duration;

use super::constants::DEFAULT_TIMEOUT;

/// DriftWatch-reported severity. The adapter normalizes external strings
/// to one of these values; anything unparseable becomes an
/// [`PolicyOutcome::Unavailable`] before any caller can mislabel it.
/// `info` and `ok` are accepted as aliases for `pass`: a not-applicable
/// policy or an informational detector result both surface as a passing
/// finding in the doctor.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PolicySeverity {
    Pass,
    Warn,
    Fail,
}
impl PolicySeverity {
    /// Stable lowercase label for display and JSON output. Used by
    /// doctor findings to name the severity without re-parsing the
    /// serialized form.
    pub fn severity_label(&self) -> &'static str {
        match self {
            PolicySeverity::Pass => "pass",
            PolicySeverity::Warn => "warn",
            PolicySeverity::Fail => "fail",
        }
    }
    pub(super) fn from_str(raw: &str) -> Option<Self> {
        match raw.to_ascii_lowercase().as_str() {
            "pass" | "ok" | "info" => Some(PolicySeverity::Pass),
            "warn" | "warning" => Some(PolicySeverity::Warn),
            "fail" | "error" | "fatal" => Some(PolicySeverity::Fail),
            _ => None,
        }
    }
    #[cfg(test)]
    pub(super) fn parse(raw: &str) -> Option<Self> {
        Self::from_str(raw)
    }
}
impl<'de> Deserialize<'de> for PolicySeverity {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        Self::from_str(&raw)
            .ok_or_else(|| serde::de::Error::custom(format!("unknown policy severity '{raw}'")))
    }
}
/// Configuration for the DriftWatch adapter. The CLI fills this from the
/// `FORGE_DRIFTWATCH_BIN` environment variable (an explicit operator
/// choice) plus the per-run timeout, then hands it to [`run_driftwatch`].
/// With no override, [`DRIFTWATCH_BINARY_CANDIDATES`] is probed in order.
#[derive(Debug, Clone)]
pub struct DriftWatchConfig {
    pub binary: Option<OsString>,
    pub timeout: Duration,
}
impl DriftWatchConfig {
    /// Resolve the adapter override from the environment. Whitespace-only
    /// values are ignored so `FORGE_DRIFTWATCH_BIN=""` keeps ordered
    /// candidate probing.
    pub fn from_env() -> Self {
        let binary = std::env::var_os("FORGE_DRIFTWATCH_BIN").filter(|v| !v.as_os_str().is_empty());
        DriftWatchConfig {
            binary,
            timeout: DEFAULT_TIMEOUT,
        }
    }
}
/// One normalized policy finding from a DriftWatch execution. Categories
/// follow [requirement.md] §24: architecture, security, privacy,
/// dependency, runtime, spec, documentation, deployment, accessibility.
/// `applicable == false` means DriftWatch reported the policy is not
/// meaningful for this profile, which the doctor surfaces explicitly
/// rather than manufacturing a detector result.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PolicyFinding {
    pub id: String,
    pub category: String,
    pub severity: PolicySeverity,
    #[serde(default = "default_applicable")]
    pub applicable: bool,
    pub message: String,
    #[serde(default)]
    pub evidence: Vec<String>,
    #[serde(default)]
    pub reason: Option<String>,
}
/// Result of one adapter invocation. A successful run with a missing
/// tool, a non-zero exit code, a timeout or a malformed payload all map
/// to [`PolicyOutcome::Unavailable`] so a doctor finding can be reported
/// honestly instead of being promoted to `PASS`.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub enum PolicyOutcome {
    /// DriftWatch ran and produced parseable output.
    Reported(PolicyReport),
    /// The adapter could not produce a reliable report. `reason` names
    /// the exact reason (missing binary, non-zero exit, timeout,
    /// unparseable JSON, etc.) and is safe to surface in the doctor
    /// finding detail.
    Unavailable { reason: String },
}
impl PolicyOutcome {
    /// Convenience: extract the report when one was produced.
    pub fn report(&self) -> Option<&PolicyReport> {
        match self {
            PolicyOutcome::Reported(report) => Some(report),
            PolicyOutcome::Unavailable { .. } => None,
        }
    }
    /// True when the adapter produced a usable report.
    pub fn is_reported(&self) -> bool {
        matches!(self, PolicyOutcome::Reported(_))
    }
}
/// Cached policy observation consumed by the doctor. `source_revision`
/// records what the project state was when the report was captured; the
/// doctor compares it with the current `forge.yaml` mtime to mark stale
/// observations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyObservation {
    pub tool: String,
    pub tool_version: String,
    pub source_revision: Option<DateTime<Utc>>,
    pub observed_at: DateTime<Utc>,
    pub finding_count: usize,
    pub report: Option<PolicyReport>,
}
/// Aggregated outcome of one DriftWatch execution, scoped to a single
/// project directory. `tool_version` is the version string DriftWatch
/// reported (or `unknown` when it could not be obtained); `source_revision`
/// is the latest modification time across `forge.yaml` and any configured
/// driftwatch file inside the project, used by the doctor to decide
/// whether a cached observation is stale.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PolicyReport {
    pub tool: String,
    pub tool_version: String,
    pub contract: String,
    pub source_revision: Option<DateTime<Utc>>,
    pub findings: Vec<PolicyFinding>,
}
#[derive(Debug)]
pub(super) struct CapturedOutput {
    pub(super) status: std::process::ExitStatus,
    pub(super) stdout: Vec<u8>,
    pub(super) stderr: Vec<u8>,
}

fn default_applicable() -> bool {
    true
}
