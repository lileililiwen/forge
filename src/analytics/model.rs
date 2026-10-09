//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use crate::core::manifest::{AnalyticsMeta, Manifest};
use crate::core::ForgeError;
use serde::{Deserialize, Serialize};

use super::constants::{
    DEFAULT_WINDOW_DAYS, MAX_WINDOW_DAYS, MIN_WINDOW_DAYS, STATUS_AMBIGUOUS, STATUS_AVAILABLE,
    STATUS_DISABLED, STATUS_UNCONFIGURED,
};
use super::projections::parse_provider_list;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectMetricsReport {
    pub contract: String,
    pub generated_at: String,
    pub default_window_days: u32,
    pub aggregates: Vec<MetricAggregate>,
    /// `true` when every aggregate has a non-empty
    /// snapshot list. A run whose adapter is missing
    /// reports the metric as `unavailable`; this flag stays
    /// `false` until every metric has at least one
    /// observation.
    pub complete: bool,
}
impl ProjectMetricsReport {
    pub fn aggregate(&self, metric_id: &str) -> Option<&MetricAggregate> {
        self.aggregates.iter().find(|a| a.metric_id == metric_id)
    }
}
/// Options for [`inspect_external_planes`]. The dry-run
/// flag records the plan without invoking the adapter so
/// the operator can preview the round trip.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AnalyticsInspectOptions {
    pub dry_run: bool,
}
/// Persisted metrics summary. v0.1 keeps only the last
/// observation so the CLI can render the most recent
/// snapshot without re-running the aggregator.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetricsSummary {
    #[serde(default)]
    pub contract: String,
    #[serde(default)]
    pub project_id: String,
    #[serde(default)]
    pub generated_at: String,
    #[serde(default)]
    pub aggregates: Vec<MetricAggregate>,
}
/// Support status of a provider. Only `unified-content`
/// and `github-analytics` are supported in v0.1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ProviderSupportStatus {
    Supported,
    Planned,
}
/// Aggregate external-plane report. The CLI renders this
/// verbatim so a caller can audit every observation in one
/// pass.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExternalPlaneReport {
    pub contract: String,
    pub project_id: String,
    pub enabled: bool,
    pub default_window_days: u32,
    pub observations: Vec<HealthObservation>,
    /// `true` when at least one provider is `available`.
    /// Disabled entries are not failures; an all-disabled
    /// report is still `available: true` (the integration
    /// surface is healthy, it is just opted out).
    pub available: bool,
}
impl ExternalPlaneReport {
    /// `true` when every configured provider is either
    /// `available`, `disabled`, `unconfigured` (no entry
    /// declared) or `ambiguous-mapping` (refused, not a
    /// transport failure). A `failed`/`unavailable`
    /// adapter call flips the verdict to `false`.
    pub fn healthy(&self) -> bool {
        for obs in &self.observations {
            match obs.status.as_str() {
                STATUS_AVAILABLE | STATUS_DISABLED | STATUS_UNCONFIGURED | STATUS_AMBIGUOUS => {
                    continue
                }
                _ => return false,
            }
        }
        true
    }
}
/// Validated analytics config parsed from a manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AnalyticsConfig {
    pub enabled: bool,
    pub default_window_days: u32,
    pub content: Vec<AnalyticsProviderConfig>,
    pub repository: Vec<AnalyticsProviderConfig>,
}
impl AnalyticsConfig {
    /// Parse the raw [`AnalyticsMeta`] into a typed
    /// configuration. Refuses unknown providers, empty
    /// references on enabled entries, duplicate enabled
    /// providers, oversized lists and out-of-range window
    /// values with the typed `analytics-invalid` code.
    pub fn from_manifest_meta(meta: &AnalyticsMeta) -> Result<Self, ForgeError> {
        let enabled = meta.enabled.unwrap_or(false);
        let default_window_days = meta.default_window_days.unwrap_or(DEFAULT_WINDOW_DAYS);
        if !(MIN_WINDOW_DAYS..=MAX_WINDOW_DAYS).contains(&default_window_days) {
            return Err(ForgeError::AnalyticsInvalid {
                reason: format!(
                    "analytics.default_window_days {default_window_days} is outside the \
                     allowed range [{MIN_WINDOW_DAYS}, {MAX_WINDOW_DAYS}]"
                ),
            });
        }
        let content = parse_provider_list("content", &meta.content)?;
        let repository = parse_provider_list("repository", &meta.repository)?;
        Ok(AnalyticsConfig {
            enabled,
            default_window_days,
            content,
            repository,
        })
    }
    /// Build a config from a manifest's `analytics:` block,
    /// returning `Ok(None)` when the block is absent.
    pub fn from_manifest_opt(manifest: &Manifest) -> Result<Option<Self>, ForgeError> {
        match &manifest.analytics {
            Some(meta) => Ok(Some(Self::from_manifest_meta(meta)?)),
            None => Ok(None),
        }
    }
    /// Every configured provider, with content first and
    /// repository second, in declaration order.
    pub fn all_providers(&self) -> Vec<&AnalyticsProviderConfig> {
        let mut out: Vec<&AnalyticsProviderConfig> = Vec::new();
        out.extend(self.content.iter());
        out.extend(self.repository.iter());
        out
    }
}
/// Reduced doctor summary consumed by the metrics
/// aggregator. The CLI / MCP transport is responsible for
/// producing it; the aggregator stays read-only.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DoctorSummary {
    #[serde(default)]
    pub healthy: i64,
    #[serde(default)]
    pub warnings: i64,
    #[serde(default)]
    pub failures: i64,
    #[serde(default)]
    pub agents_running: i64,
    #[serde(default)]
    pub specs_queued: i64,
    #[serde(default)]
    pub deployments_running: i64,
    #[serde(default)]
    pub deployments_offline: i64,
    #[serde(default)]
    pub deployments_pending: i64,
}
/// Options for [`aggregate_project_metrics`].
#[derive(Debug, Clone)]
pub struct MetricsAggregateOptions {
    pub default_window_days: u32,
    pub external_observations: Vec<HealthObservation>,
}
/// Provider plane. The catalog keeps every supported and
/// planned provider so an upgrade never has to invent a
/// new enum variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AnalyticsProvider {
    UnifiedContent,
    GithubAnalytics,
    Notion,
    Confluence,
    GitlabAnalytics,
    CodebergAnalytics,
}
/// One validated provider entry from the manifest's
/// `analytics.content[]` or `analytics.repository[]` list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnalyticsProviderConfig {
    pub provider: AnalyticsProvider,
    pub enabled: bool,
    pub project_ref: String,
    pub adapter_command: Option<String>,
}
/// One health observation for a configured provider. The
/// observation is timestamped, names its source, and
/// records explicit evidence so the operator can audit
/// why a provider was reported available or unavailable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealthObservation {
    pub provider: String,
    pub plane: String,
    pub project_ref: String,
    pub status: String,
    pub observed_at: String,
    pub source: String,
    pub evidence: Vec<String>,
    pub note: String,
}
#[derive(Debug, Clone, Deserialize)]
pub(super) struct AdapterHealthPayload {
    #[serde(default)]
    pub(super) project_ref: Option<String>,
    #[serde(default)]
    pub(super) evidence: Vec<String>,
    #[serde(default)]
    pub(super) note: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetricAggregate {
    pub metric_id: String,
    pub state: String,
    pub snapshots: Vec<MetricSnapshot>,
    /// Current value when the snapshots agree on a single
    /// window. `None` when snapshots span different
    /// windows (R2 boundary) or all snapshots are
    /// `unavailable`.
    pub current: Option<i64>,
    pub windows: Vec<String>,
    pub note: String,
}
pub(super) struct AdapterOutput {
    pub(super) exit_code: Option<i32>,
    pub(super) stdout: Vec<u8>,
    pub(super) stderr: String,
}
/// Aggregate one observation. The `MetricAggregate` always
/// carries the `available` flag, the source list and the
/// observation windows so a partial report does not
/// silently sum across different windows (R2 boundary
/// scenario).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetricSnapshot {
    pub source: String,
    pub value: Option<i64>,
    pub observed_at: String,
    pub window: String,
    pub note: String,
}

impl Default for MetricsAggregateOptions {
    fn default() -> Self {
        MetricsAggregateOptions {
            default_window_days: DEFAULT_WINDOW_DAYS,
            external_observations: Vec::new(),
        }
    }
}
