//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use crate::core::manifest::{AnalyticsProviderEntry, Manifest};
use crate::core::ForgeError;
use crate::policy::redact_credentials;
use crate::registry::Registry;
use chrono::{DateTime, Utc};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use super::constants::{
    ANALYTICS_ADAPTER_TIMEOUT, ANALYTICS_BIN_ENV, ANALYTICS_CONTRACT_VERSION,
    DEFAULT_ANALYTICS_BIN, MAX_METRICS_PER_REPORT, MAX_PROVIDERS_PER_PLANE, MAX_WINDOW_DAYS,
    METRIC_AGENTS_RUNNING, METRIC_DEPLOYMENT_OFFLINE, METRIC_DEPLOYMENT_PENDING,
    METRIC_DEPLOYMENT_RUNNING, METRIC_PROJECTS, METRIC_QUALITY_FAILURES, METRIC_QUALITY_HEALTHY,
    METRIC_QUALITY_WARNINGS, METRIC_REPOSITORY_STARS, METRIC_REPOSITORY_STARS_GROWTH,
    METRIC_SPECS_QUEUED, METRIC_STATE_MIXED_WINDOWS, METRIC_STATE_PRESENT,
    METRIC_STATE_UNAVAILABLE, MIN_WINDOW_DAYS, PROVIDER_CODEBERG_ANALYTICS, PROVIDER_CONFLUENCE,
    PROVIDER_GITHUB_ANALYTICS, PROVIDER_GITLAB_ANALYTICS, PROVIDER_NOTION,
    PROVIDER_UNIFIED_CONTENT, STATUS_AMBIGUOUS, STATUS_AVAILABLE, STATUS_DISABLED,
    STATUS_UNAVAILABLE, WINDOW_INSTANT,
};
use super::model::{
    AdapterHealthPayload, AdapterOutput, AnalyticsConfig, AnalyticsInspectOptions,
    AnalyticsProvider, AnalyticsProviderConfig, DoctorSummary, ExternalPlaneReport,
    HealthObservation, MetricAggregate, MetricSnapshot, MetricsAggregateOptions, MetricsSummary,
    ProjectMetricsReport, ProviderSupportStatus,
};

/// Stable provider id parsing. Unknown providers are
/// refused with the typed `analytics-invalid` code so the
/// manifest cannot silently declare an unsupported
/// provider.
pub fn parse_provider(s: &str) -> Result<AnalyticsProvider, ForgeError> {
    match s.trim() {
        PROVIDER_UNIFIED_CONTENT => Ok(AnalyticsProvider::UnifiedContent),
        PROVIDER_GITHUB_ANALYTICS => Ok(AnalyticsProvider::GithubAnalytics),
        PROVIDER_NOTION => Ok(AnalyticsProvider::Notion),
        PROVIDER_CONFLUENCE => Ok(AnalyticsProvider::Confluence),
        PROVIDER_GITLAB_ANALYTICS => Ok(AnalyticsProvider::GitlabAnalytics),
        PROVIDER_CODEBERG_ANALYTICS => Ok(AnalyticsProvider::CodebergAnalytics),
        other => Err(ForgeError::AnalyticsInvalid {
            reason: format!(
                "unknown analytics provider `{other}`; supported: {PROVIDER_UNIFIED_CONTENT}, \
                 {PROVIDER_GITHUB_ANALYTICS}; planned: {PROVIDER_NOTION}, {PROVIDER_CONFLUENCE}, \
                 {PROVIDER_GITLAB_ANALYTICS}, {PROVIDER_CODEBERG_ANALYTICS}"
            ),
        }),
    }
}

/// Stable label for a provider. Used in journal rows and
/// in the rendered report.
pub fn provider_label(provider: AnalyticsProvider) -> &'static str {
    match provider {
        AnalyticsProvider::UnifiedContent => PROVIDER_UNIFIED_CONTENT,
        AnalyticsProvider::GithubAnalytics => PROVIDER_GITHUB_ANALYTICS,
        AnalyticsProvider::Notion => PROVIDER_NOTION,
        AnalyticsProvider::Confluence => PROVIDER_CONFLUENCE,
        AnalyticsProvider::GitlabAnalytics => PROVIDER_GITLAB_ANALYTICS,
        AnalyticsProvider::CodebergAnalytics => PROVIDER_CODEBERG_ANALYTICS,
    }
}

pub fn provider_support_status(provider: AnalyticsProvider) -> ProviderSupportStatus {
    match provider {
        AnalyticsProvider::UnifiedContent | AnalyticsProvider::GithubAnalytics => {
            ProviderSupportStatus::Supported
        }
        AnalyticsProvider::Notion
        | AnalyticsProvider::Confluence
        | AnalyticsProvider::GitlabAnalytics
        | AnalyticsProvider::CodebergAnalytics => ProviderSupportStatus::Planned,
    }
}

pub(super) fn parse_provider_list(
    plane: &str,
    entries: &[AnalyticsProviderEntry],
) -> Result<Vec<AnalyticsProviderConfig>, ForgeError> {
    if entries.len() > MAX_PROVIDERS_PER_PLANE {
        return Err(ForgeError::AnalyticsInvalid {
            reason: format!(
                "analytics.{plane} declares {} providers; the maximum is {MAX_PROVIDERS_PER_PLANE}",
                entries.len()
            ),
        });
    }
    let mut out: Vec<AnalyticsProviderConfig> = Vec::with_capacity(entries.len());
    for (idx, entry) in entries.iter().enumerate() {
        let provider_str = entry.provider.trim();
        if provider_str.is_empty() {
            return Err(ForgeError::AnalyticsInvalid {
                reason: format!("analytics.{plane}[{idx}].provider is empty"),
            });
        }
        let provider = parse_provider(provider_str)?;
        let enabled = entry.enabled.unwrap_or(true);
        let project_ref = match entry.project_ref.as_deref() {
            Some(value) => value.trim().to_string(),
            None => String::new(),
        };
        if project_ref.is_empty() {
            return Err(ForgeError::AnalyticsInvalid {
                reason: format!(
                    "analytics.{plane}[{idx}].project_ref is required; declare the external \
                     project id (e.g. `owner/repo` for github-analytics or a content slug for \
                     {PROVIDER_UNIFIED_CONTENT}) so Forge does not silently bind to another \
                     project's data"
                ),
            });
        }
        if project_ref.contains(char::is_whitespace)
            || project_ref.contains(';')
            || project_ref.contains('|')
            || project_ref.contains('\n')
            || project_ref.contains('\0')
        {
            return Err(ForgeError::AnalyticsInvalid {
                reason: format!(
                    "analytics.{plane}[{idx}].project_ref `{project_ref}` contains shell \
                     metacharacters or whitespace; references are data, not commands"
                ),
            });
        }
        if out
            .iter()
            .any(|p: &AnalyticsProviderConfig| p.provider == provider)
        {
            return Err(ForgeError::AnalyticsInvalid {
                reason: format!(
                    "analytics.{plane} declares provider `{provider_str}` more than once; \
                     configure each provider at most once"
                ),
            });
        }
        out.push(AnalyticsProviderConfig {
            provider,
            enabled,
            project_ref,
            adapter_command: entry.adapter_command.clone(),
        });
    }
    Ok(out)
}

/// Inspect the manifest's `analytics:` block and produce
/// a typed [`ExternalPlaneReport`]. The function never
/// mutates the project: it reads the manifest, runs the
/// adapter for each enabled provider with argument arrays
/// plus a bounded timeout, and records the observation.
/// Disabled providers are reported as `disabled` without
/// invoking the adapter (R1 boundary).
pub fn inspect_external_planes(
    project_id: &str,
    config: &AnalyticsConfig,
    options: &AnalyticsInspectOptions,
) -> Result<ExternalPlaneReport, ForgeError> {
    validate_project_id(project_id)?;
    let now = Utc::now();
    let mut observations: Vec<HealthObservation> = Vec::new();
    if !config.enabled {
        for entry in config.all_providers() {
            observations.push(HealthObservation {
                provider: provider_label(entry.provider).to_string(),
                plane: plane_of(entry).to_string(),
                project_ref: entry.project_ref.clone(),
                status: STATUS_DISABLED.to_string(),
                observed_at: now.to_rfc3339(),
                source: "manifest".to_string(),
                evidence: Vec::new(),
                note: "analytics block is disabled; provider is not contacted".to_string(),
            });
        }
        return Ok(ExternalPlaneReport {
            contract: ANALYTICS_CONTRACT_VERSION.to_string(),
            project_id: project_id.to_string(),
            enabled: false,
            default_window_days: config.default_window_days,
            observations,
            available: true,
        });
    }
    for entry in config.content.iter() {
        observations.push(observe_one(project_id, "content", entry, options, now)?);
    }
    for entry in config.repository.iter() {
        observations.push(observe_one(project_id, "repository", entry, options, now)?);
    }
    let report = ExternalPlaneReport {
        contract: ANALYTICS_CONTRACT_VERSION.to_string(),
        project_id: project_id.to_string(),
        enabled: true,
        default_window_days: config.default_window_days,
        observations,
        available: false,
    };
    Ok(report)
}

fn plane_of(_entry: &AnalyticsProviderConfig) -> &'static str {
    // Plane identity is recorded at the report level; the
    // helper exists so the renderer does not duplicate
    // string literals.
    "provider"
}

fn observe_one(
    project_id: &str,
    plane: &str,
    entry: &AnalyticsProviderConfig,
    options: &AnalyticsInspectOptions,
    now: DateTime<Utc>,
) -> Result<HealthObservation, ForgeError> {
    if !entry.enabled {
        return Ok(HealthObservation {
            provider: provider_label(entry.provider).to_string(),
            plane: plane.to_string(),
            project_ref: entry.project_ref.clone(),
            status: STATUS_DISABLED.to_string(),
            observed_at: now.to_rfc3339(),
            source: "manifest".to_string(),
            evidence: Vec::new(),
            note: "provider is disabled in the manifest; not contacted".to_string(),
        });
    }
    if matches!(
        provider_support_status(entry.provider),
        ProviderSupportStatus::Planned
    ) {
        return Ok(HealthObservation {
            provider: provider_label(entry.provider).to_string(),
            plane: plane.to_string(),
            project_ref: entry.project_ref.clone(),
            status: STATUS_UNAVAILABLE.to_string(),
            observed_at: now.to_rfc3339(),
            source: "catalog".to_string(),
            evidence: Vec::new(),
            note: format!(
                "provider `{}` is declared but not in the supported analytics set for this \
                 release; planned for a later change",
                provider_label(entry.provider)
            ),
        });
    }
    if options.dry_run {
        return Ok(HealthObservation {
            provider: provider_label(entry.provider).to_string(),
            plane: plane.to_string(),
            project_ref: entry.project_ref.clone(),
            status: STATUS_AVAILABLE.to_string(),
            observed_at: now.to_rfc3339(),
            source: "dry-run".to_string(),
            evidence: vec![format!(
                "would invoke adapter for project_ref=`{}`",
                entry.project_ref
            )],
            note: "dry-run: adapter was not invoked".to_string(),
        });
    }
    run_adapter_health(project_id, plane, entry, now)
}

fn run_adapter_health(
    project_id: &str,
    plane: &str,
    entry: &AnalyticsProviderConfig,
    now: DateTime<Utc>,
) -> Result<HealthObservation, ForgeError> {
    let bin = entry
        .adapter_command
        .clone()
        .or_else(|| std::env::var(ANALYTICS_BIN_ENV).ok())
        .unwrap_or_else(|| DEFAULT_ANALYTICS_BIN.to_string());
    let mut cmd = Command::new(&bin);
    cmd.arg("health")
        .arg("--provider")
        .arg(provider_label(entry.provider))
        .arg("--project")
        .arg(project_id)
        .arg("--project-ref")
        .arg(&entry.project_ref)
        .arg("--plane")
        .arg(plane);
    let output = match spawn_with_timeout(&mut cmd, ANALYTICS_ADAPTER_TIMEOUT) {
        Ok(out) => out,
        Err(reason) => {
            return Ok(HealthObservation {
                provider: provider_label(entry.provider).to_string(),
                plane: plane.to_string(),
                project_ref: entry.project_ref.clone(),
                status: STATUS_UNAVAILABLE.to_string(),
                observed_at: now.to_rfc3339(),
                source: "adapter".to_string(),
                evidence: vec![redact_credentials(&reason)],
                note: format!(
                    "adapter `{bin}` could not be executed; provider health is unavailable"
                ),
            });
        }
    };
    parse_health_output(&bin, output, entry, plane, now)
}

fn parse_health_output(
    bin: &str,
    output: AdapterOutput,
    entry: &AnalyticsProviderConfig,
    plane: &str,
    now: DateTime<Utc>,
) -> Result<HealthObservation, ForgeError> {
    let provider = provider_label(entry.provider).to_string();
    let project_ref = entry.project_ref.clone();
    let exit_code = output.exit_code;
    let stdout = output.stdout;
    let stderr = output.stderr;
    if exit_code != Some(0) {
        return Ok(HealthObservation {
            provider: provider.clone(),
            plane: plane.to_string(),
            project_ref: project_ref.clone(),
            status: STATUS_UNAVAILABLE.to_string(),
            observed_at: now.to_rfc3339(),
            source: "adapter".to_string(),
            evidence: vec![
                format!("exit_code={}", exit_code.unwrap_or(-1)),
                redact_credentials(&stderr),
            ],
            note: format!("adapter `{bin}` exited with non-zero status"),
        });
    }
    let payload: AdapterHealthPayload = match serde_json::from_slice(&stdout) {
        Ok(payload) => payload,
        Err(err) => {
            return Ok(HealthObservation {
                provider: provider.clone(),
                plane: plane.to_string(),
                project_ref: project_ref.clone(),
                status: STATUS_UNAVAILABLE.to_string(),
                observed_at: now.to_rfc3339(),
                source: "adapter".to_string(),
                evidence: vec![format!("parse-error: {err}")],
                note: format!("adapter `{bin}` returned a non-JSON health payload"),
            });
        }
    };
    if payload.project_ref.as_deref() != Some(project_ref.as_str()) {
        return Ok(HealthObservation {
            provider: provider.clone(),
            plane: plane.to_string(),
            project_ref: project_ref.clone(),
            status: STATUS_AMBIGUOUS.to_string(),
            observed_at: now.to_rfc3339(),
            source: "adapter".to_string(),
            evidence: vec![format!(
                "adapter reported project_ref=`{}`; expected `{}`",
                payload.project_ref.as_deref().unwrap_or(""),
                project_ref
            )],
            note: "adapter's reported project_ref does not match the manifest; refusing to \
                   attach another project's data"
                .to_string(),
        });
    }
    Ok(HealthObservation {
        provider: provider.clone(),
        plane: plane.to_string(),
        project_ref: project_ref.clone(),
        status: STATUS_AVAILABLE.to_string(),
        observed_at: now.to_rfc3339(),
        source: "adapter".to_string(),
        evidence: payload
            .evidence
            .into_iter()
            .map(|e| redact_credentials(&e))
            .collect(),
        note: redact_credentials(&payload.note),
    })
}

fn spawn_with_timeout(cmd: &mut Command, timeout: Duration) -> Result<AdapterOutput, String> {
    use std::io::Read;
    use std::process::Stdio;
    use std::sync::{Arc, Mutex};
    use std::thread;
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = match cmd.spawn() {
        Ok(child) => child,
        Err(err) => return Err(format!("cannot spawn adapter: {err}")),
    };
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let stdout_buf: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
    let stderr_buf: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
    let stdout_reader = {
        let buf = Arc::clone(&stdout_buf);
        thread::spawn(move || {
            if let Some(mut stdout) = stdout {
                let mut local = Vec::new();
                let _ = stdout.read_to_end(&mut local);
                if let Ok(mut guard) = buf.lock() {
                    *guard = local;
                }
            }
        })
    };
    let stderr_reader = {
        let buf = Arc::clone(&stderr_buf);
        thread::spawn(move || {
            if let Some(mut stderr) = stderr {
                let mut local = Vec::new();
                let _ = stderr.read_to_end(&mut local);
                if let Ok(mut guard) = buf.lock() {
                    *guard = local;
                }
            }
        })
    };
    let start = std::time::Instant::now();
    let exit_code = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status.code(),
            Ok(None) => {
                if start.elapsed() > timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    let _ = stdout_reader.join();
                    let _ = stderr_reader.join();
                    return Err(format!("adapter timed out after {timeout:?}"));
                }
                thread::sleep(Duration::from_millis(50));
            }
            Err(err) => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = stdout_reader.join();
                let _ = stderr_reader.join();
                return Err(format!("cannot wait on adapter: {err}"));
            }
        }
    };
    let _ = stdout_reader.join();
    let _ = stderr_reader.join();
    let stdout_bytes = stdout_buf
        .lock()
        .map(|g| g.clone())
        .unwrap_or_else(|_| Vec::new());
    let stderr_bytes = stderr_buf
        .lock()
        .map(|g| g.clone())
        .unwrap_or_else(|_| Vec::new());
    Ok(AdapterOutput {
        exit_code,
        stdout: stdout_bytes,
        stderr: String::from_utf8_lossy(&stderr_bytes).to_string(),
    })
}

/// Build a project-metrics report from the local registry,
/// the doctor finding inventory and the external
/// observations. The aggregator never sums snapshots from
/// different windows into one total: every snapshot
/// carries its `window` and the aggregate reports the
/// distinct windows it observed.
pub fn aggregate_project_metrics(
    registry: &Registry,
    doctor_summary: Option<DoctorSummary>,
    options: &MetricsAggregateOptions,
) -> Result<ProjectMetricsReport, ForgeError> {
    if options.default_window_days < MIN_WINDOW_DAYS
        || options.default_window_days > MAX_WINDOW_DAYS
    {
        return Err(ForgeError::AnalyticsInvalid {
            reason: format!(
                "metrics default_window_days {} is outside the allowed range \
                 [{MIN_WINDOW_DAYS}, {MAX_WINDOW_DAYS}]",
                options.default_window_days
            ),
        });
    }
    let now = Utc::now();
    let now_str = now.to_rfc3339();
    let mut aggregates: Vec<MetricAggregate> = Vec::new();
    let projects = registry.list()?;
    aggregates.push(count_metric(
        METRIC_PROJECTS,
        WINDOW_INSTANT,
        "registry",
        now_str.clone(),
        projects.len() as i64,
    ));
    let doctor = doctor_summary.unwrap_or_default();
    aggregates.push(count_metric(
        METRIC_QUALITY_HEALTHY,
        WINDOW_INSTANT,
        "doctor",
        now_str.clone(),
        doctor.healthy,
    ));
    aggregates.push(count_metric(
        METRIC_QUALITY_WARNINGS,
        WINDOW_INSTANT,
        "doctor",
        now_str.clone(),
        doctor.warnings,
    ));
    aggregates.push(count_metric(
        METRIC_QUALITY_FAILURES,
        WINDOW_INSTANT,
        "doctor",
        now_str.clone(),
        doctor.failures,
    ));
    aggregates.push(count_metric(
        METRIC_AGENTS_RUNNING,
        WINDOW_INSTANT,
        "agent-sessions",
        now_str.clone(),
        doctor.agents_running,
    ));
    aggregates.push(count_metric(
        METRIC_SPECS_QUEUED,
        WINDOW_INSTANT,
        "spec-registry",
        now_str.clone(),
        doctor.specs_queued,
    ));
    aggregates.push(count_metric(
        METRIC_DEPLOYMENT_RUNNING,
        WINDOW_INSTANT,
        "deploy-state",
        now_str.clone(),
        doctor.deployments_running,
    ));
    aggregates.push(count_metric(
        METRIC_DEPLOYMENT_OFFLINE,
        WINDOW_INSTANT,
        "deploy-state",
        now_str.clone(),
        doctor.deployments_offline,
    ));
    aggregates.push(count_metric(
        METRIC_DEPLOYMENT_PENDING,
        WINDOW_INSTANT,
        "deploy-state",
        now_str.clone(),
        doctor.deployments_pending,
    ));
    let stars_window = format!("{}-day", options.default_window_days);
    let mut repo_aggregate = MetricAggregate {
        metric_id: METRIC_REPOSITORY_STARS.to_string(),
        state: METRIC_STATE_UNAVAILABLE.to_string(),
        snapshots: Vec::new(),
        current: None,
        windows: Vec::new(),
        note: "no repository analytics observations".to_string(),
    };
    let mut growth_aggregate = MetricAggregate {
        metric_id: METRIC_REPOSITORY_STARS_GROWTH.to_string(),
        state: METRIC_STATE_UNAVAILABLE.to_string(),
        snapshots: Vec::new(),
        current: None,
        windows: Vec::new(),
        note: "no repository analytics observations".to_string(),
    };
    for obs in &options.external_observations {
        if !matches!(obs.plane.as_str(), "repository") {
            continue;
        }
        if obs.status != STATUS_AVAILABLE {
            continue;
        }
        for line in &obs.evidence {
            if let Some(value) = parse_kv_metric(line, "stars") {
                repo_aggregate.snapshots.push(MetricSnapshot {
                    source: obs.provider.clone(),
                    value: Some(value),
                    observed_at: obs.observed_at.clone(),
                    window: WINDOW_INSTANT.to_string(),
                    note: obs.note.clone(),
                });
            }
            if let Some(value) = parse_kv_metric(line, "growth") {
                growth_aggregate.snapshots.push(MetricSnapshot {
                    source: obs.provider.clone(),
                    value: Some(value),
                    observed_at: obs.observed_at.clone(),
                    window: stars_window.clone(),
                    note: obs.note.clone(),
                });
            }
        }
    }
    for agg in [&mut repo_aggregate, &mut growth_aggregate] {
        finalize_aggregate(agg);
    }
    aggregates.push(repo_aggregate);
    aggregates.push(growth_aggregate);
    if aggregates.len() > MAX_METRICS_PER_REPORT {
        return Err(ForgeError::AnalyticsInvalid {
            reason: format!(
                "metrics report carries {} aggregates; the maximum is {MAX_METRICS_PER_REPORT}",
                aggregates.len()
            ),
        });
    }
    let complete = aggregates
        .iter()
        .all(|a| a.state != METRIC_STATE_UNAVAILABLE);
    Ok(ProjectMetricsReport {
        contract: ANALYTICS_CONTRACT_VERSION.to_string(),
        generated_at: now_str,
        default_window_days: options.default_window_days,
        aggregates,
        complete,
    })
}

pub(super) fn count_metric(
    metric_id: &str,
    window: &str,
    source: &str,
    observed_at: String,
    value: i64,
) -> MetricAggregate {
    MetricAggregate {
        metric_id: metric_id.to_string(),
        state: METRIC_STATE_PRESENT.to_string(),
        snapshots: vec![MetricSnapshot {
            source: source.to_string(),
            value: Some(value),
            observed_at,
            window: window.to_string(),
            note: String::new(),
        }],
        current: Some(value),
        windows: vec![window.to_string()],
        note: String::new(),
    }
}

fn parse_kv_metric(line: &str, key: &str) -> Option<i64> {
    let needle = format!("{key}=");
    let idx = line.find(&needle)?;
    let after = &line[idx + needle.len()..];
    let mut digits = String::new();
    for c in after.chars() {
        let accept = c.is_ascii_digit() || (c == '-' && digits.is_empty());
        if accept {
            digits.push(c);
        } else {
            break;
        }
    }
    if digits.is_empty() {
        None
    } else {
        digits.parse::<i64>().ok()
    }
}

fn finalize_aggregate(aggregate: &mut MetricAggregate) {
    let mut windows: Vec<String> = aggregate
        .snapshots
        .iter()
        .map(|s| s.window.clone())
        .collect();
    windows.sort();
    windows.dedup();
    aggregate.windows = windows.clone();
    if aggregate.snapshots.is_empty() {
        return;
    }
    if windows.len() > 1 {
        aggregate.state = METRIC_STATE_MIXED_WINDOWS.to_string();
        aggregate.current = None;
        aggregate.note = format!(
            "snapshots span multiple windows: {}; refusing to sum across windows",
            windows.join(", ")
        );
        return;
    }
    let distinct: Vec<i64> = aggregate
        .snapshots
        .iter()
        .filter_map(|s| s.value)
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    if distinct.len() > 1 {
        aggregate.state = METRIC_STATE_MIXED_WINDOWS.to_string();
        aggregate.current = None;
        aggregate.note =
            "snapshots disagree on the same window; refusing to pick a winner".to_string();
        return;
    }
    aggregate.current = distinct.first().copied();
    aggregate.state = METRIC_STATE_PRESENT.to_string();
    aggregate.note = format!("observed at {}", aggregate.snapshots[0].observed_at);
}

/// Redact credential-shaped substrings in analytics
/// evidence. Delegates to
/// [`crate::policy::redact_credentials`] so the analytics
/// contract shares one definition of "secret" with the
/// policy, release, distribution, docs, deploy and
/// identity adapters.
pub fn redact_analytics_evidence(text: &str) -> String {
    redact_credentials(text)
}

/// Render an [`ExternalPlaneReport`] in human-readable form.
pub fn render_report_human(report: &ExternalPlaneReport) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "analytics contract={} project={} enabled={} default_window={}d\n",
        report.contract, report.project_id, report.enabled, report.default_window_days
    ));
    for obs in &report.observations {
        out.push_str(&format!(
            "  plane={} provider={} project_ref={} status={} source={} observed_at={} \
             note={}\n",
            obs.plane,
            obs.provider,
            obs.project_ref,
            obs.status,
            obs.source,
            obs.observed_at,
            redact_analytics_evidence(&obs.note)
        ));
        for line in &obs.evidence {
            out.push_str(&format!(
                "    evidence: {}\n",
                redact_analytics_evidence(line)
            ));
        }
    }
    out.trim_end().to_string()
}

/// Render a [`ProjectMetricsReport`] in human-readable form.
/// Each aggregate lists the observed snapshots, the
/// distinct windows and the explicit state so a caller
/// can audit why a metric was reported `present`,
/// `unavailable`, `stale` or `mixed-windows` without
/// re-reading the JSON.
pub fn render_metrics_human(report: &ProjectMetricsReport) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "metrics contract={} generated_at={} default_window={}d complete={}\n",
        report.contract, report.generated_at, report.default_window_days, report.complete
    ));
    for agg in &report.aggregates {
        let current = agg
            .current
            .map(|v| v.to_string())
            .unwrap_or_else(|| "n/a".to_string());
        out.push_str(&format!(
            "  metric={} state={} current={} windows={}\n",
            agg.metric_id,
            agg.state,
            current,
            if agg.windows.is_empty() {
                "(none)".to_string()
            } else {
                agg.windows.join(",")
            }
        ));
        for snap in &agg.snapshots {
            let value = snap
                .value
                .map(|v| v.to_string())
                .unwrap_or_else(|| "n/a".to_string());
            out.push_str(&format!(
                "    source={} value={} window={} observed_at={}\n",
                snap.source, value, snap.window, snap.observed_at
            ));
        }
        if !agg.note.is_empty() {
            out.push_str(&format!(
                "    note: {}\n",
                redact_analytics_evidence(&agg.note)
            ));
        }
    }
    out.trim_end().to_string()
}

fn validate_project_id(id: &str) -> Result<(), ForgeError> {
    crate::core::validate_project_id(id).map_err(|reason| ForgeError::AnalyticsInvalid { reason })
}

/// Resolve the manifest's analytics block from disk, read
/// without contacting any provider. Used by the CLI to
/// render the `analytics inspect` plan; the adapter runs
/// only when [`inspect_external_planes`] is called.
pub fn load_config(project_dir: &Path) -> Result<Option<AnalyticsConfig>, ForgeError> {
    let manifest_path = project_dir.join(crate::core::manifest::CANONICAL_MANIFEST);
    if !manifest_path.exists() {
        return Err(ForgeError::ManifestNotFound {
            path: manifest_path.display().to_string(),
        });
    }
    let (manifest, _) = Manifest::load_from_dir(project_dir, None)?;
    AnalyticsConfig::from_manifest_opt(&manifest)
}

/// Path to the persisted metrics summary, scoped to the
/// project id so a future transport cannot confuse two
/// project's metrics. The file is optional: when the
/// aggregator never ran, the file does not exist and the
/// reader returns [`MetricsSummary::default`].
pub fn metrics_summary_path(project_dir: &Path, project_id: &str) -> Result<PathBuf, ForgeError> {
    if project_id.trim().is_empty() {
        return Err(ForgeError::AnalyticsInvalid {
            reason: "project id is required to resolve the metrics summary path".to_string(),
        });
    }
    Ok(project_dir
        .join(".forge")
        .join("analytics")
        .join(project_id)
        .join("metrics.json"))
}

pub fn load_metrics_summary(path: &Path) -> Result<MetricsSummary, ForgeError> {
    if !path.exists() {
        return Ok(MetricsSummary::default());
    }
    let bytes = fs::read(path).map_err(|err| ForgeError::AnalyticsInvalid {
        reason: format!("cannot read metrics summary {}: {err}", path.display()),
    })?;
    if bytes.is_empty() {
        return Ok(MetricsSummary::default());
    }
    let summary: MetricsSummary =
        serde_json::from_slice(&bytes).map_err(|err| ForgeError::AnalyticsInvalid {
            reason: format!(
                "metrics summary at {} is not valid JSON: {err}",
                path.display()
            ),
        })?;
    Ok(summary)
}

pub fn save_metrics_summary(path: &Path, summary: &MetricsSummary) -> Result<(), ForgeError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| ForgeError::AnalyticsInvalid {
            reason: format!(
                "cannot create metrics summary directory {}: {err}",
                parent.display()
            ),
        })?;
    }
    let bytes = serde_json::to_vec_pretty(summary).map_err(|err| ForgeError::AnalyticsInvalid {
        reason: format!("cannot serialize metrics summary: {err}"),
    })?;
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, &bytes).map_err(|err| ForgeError::AnalyticsInvalid {
        reason: format!("cannot write metrics summary tmp {}: {err}", tmp.display()),
    })?;
    fs::rename(&tmp, path).map_err(|err| ForgeError::AnalyticsInvalid {
        reason: format!("cannot rename metrics summary {}: {err}", path.display()),
    })?;
    Ok(())
}
