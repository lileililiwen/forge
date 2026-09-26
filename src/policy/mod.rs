//! Delegated policy execution and quality result isolation
//! (`quality-policy-integration`).
//!
//! Core owns the typed policy adapter contract and the credential-redaction
//! pipeline. Transports (CLI now; MCP/API later) invoke [`run_driftwatch`]
//! through the same code path the registry uses for observations, so a
//! project-scoped execution produces an observation that names only its own
//! project, evidence is redacted before any caller sees it, and stale
//! observations are surfaced as such.
//!
//! The contract is versioned through [`POLICY_CONTRACT_VERSION`]; the
//! adapter speaks JSON on stdout and never through a shell, so a credential
//! in an evidence string cannot be expanded as a command. External output
//! and tool versions may drift; this module therefore reports
//! [`PolicyOutcome::Unavailable`] when the binary is missing, the
//! invocation times out or the output is not parseable JSON, and it never
//! maps an absent policy run to a `PASS`.

use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Contract data version for the DriftWatch adapter and the
/// `PolicyObservation` it produces. Independent of DriftWatch's own
/// tool version (reported per run inside [`PolicyReport::tool_version`]).
pub const POLICY_CONTRACT_VERSION: &str = "0.1.0";

/// Ordered binary candidates for the policy plane when no explicit
/// `FORGE_DRIFTWATCH_BIN` override is set: the cargo/installer name first,
/// then the npm launcher alias. First executable hit wins.
pub const DRIFTWATCH_BINARY_CANDIDATES: &[&str] = &["driftwatchdog", "driftwatch"];

/// Sibling machine-readable checker-report contract emitted by
/// `driftwatch check --format json` (driftwatchdog change
/// `checker-machine-output`). Same-major documents may add top-level
/// fields; consumers must ignore the unknown ones.
pub const CHECKER_REPORT_CONTRACT: &str = "driftwatch-checker/0.1.0";

/// Default per-run timeout. The adapter uses `Command::spawn` + bounded
/// `wait_timeout` so an unresponsive DriftWatch cannot hang the registry.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(60);

/// Categories that the brief §24 names as policy planes. Used as a
/// string on findings so a future taxonomy change does not require
/// schema regeneration.
pub const POLICY_CATEGORIES: &[&str] = &[
    "architecture",
    "security",
    "privacy",
    "dependency",
    "runtime",
    "spec",
    "documentation",
    "deployment",
    "accessibility",
];

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

fn default_applicable() -> bool {
    true
}

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

    fn from_str(raw: &str) -> Option<Self> {
        match raw.to_ascii_lowercase().as_str() {
            "pass" | "ok" | "info" => Some(PolicySeverity::Pass),
            "warn" | "warning" => Some(PolicySeverity::Warn),
            "fail" | "error" | "fatal" => Some(PolicySeverity::Fail),
            _ => None,
        }
    }

    #[cfg(test)]
    fn parse(raw: &str) -> Option<Self> {
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

/// First candidate name that exists as an executable file on `path_env`
/// (a `PATH`-shaped list of directories). Pure function of the inputs so
/// resolution order is testable without mutating the process environment.
pub fn first_binary_on_path(path_env: &OsStr, candidates: &[&str]) -> Option<PathBuf> {
    candidates.iter().find_map(|name| {
        for dir in std::env::split_paths(path_env) {
            if dir.as_os_str().is_empty() {
                continue;
            }
            let candidate = dir.join(format!("{name}{}", std::env::consts::EXE_SUFFIX));
            let Ok(meta) = fs::metadata(&candidate) else {
                continue;
            };
            if !meta.is_file() {
                continue;
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if meta.permissions().mode() & 0o111 == 0 {
                    continue;
                }
            }
            return Some(candidate);
        }
        None
    })
}

/// Where the adapter sends `dir`: gate-managed projects run the gate
/// surface, everything else the checker surface. The checker side uses
/// the sibling's documented no-persistence composition
/// (`check --dry-run --format json` emits the same versioned document
/// while writing nothing). The gate side has no such composition: the
/// sibling's `gate --dry-run` renders the human-readable plan and exits
/// before its JSON writer — verified live against
/// `driftwatchdog/commands/gate.rs` and captured in
/// `tests/fixtures/driftwatch/NOTES.md` — so Forge runs the real
/// `gate --format json`, whose only Forge-visible side effect is a
/// `gate_runs` row in the project's own `.driftwatch/` store (sibling
/// state, never Forge registry state). One source of truth for the
/// adapter and the provider probe.
pub fn policy_surface_args(dir: &Path) -> Vec<&'static str> {
    let gate_managed =
        dir.join("gate.toml").is_file() || dir.join(".ai-gate").join("gate.yaml").is_file();
    if gate_managed {
        vec!["gate", "--format", "json"]
    } else {
        vec!["check", "--dry-run", "--format", "json"]
    }
}

/// Invoke DriftWatch for `dir` and return the normalized outcome.
///
/// Binary resolution is an ordered probe: the explicit
/// `FORGE_DRIFTWATCH_BIN` override runs exactly that binary (no further
/// probing), otherwise the first executable hit of
/// [`DRIFTWATCH_BINARY_CANDIDATES`] wins. The project directory is the
/// working directory — Forge never sends the sibling's grammar a
/// fabricated `--project` flag — and the surface follows the project:
/// gate-managed directories run `gate --format json`, everything else
/// `check --dry-run --format json` (the sibling's `checker-machine-output`
/// contract; see [`policy_surface_args`] for why only the checker side
/// has a no-persistence composition).
///
/// Classification: a parseable document — including one reporting a
/// blocked gate or a failing checker, whatever the process exit code —
/// becomes normalized findings. Only a missing binary, a timeout, or an
/// unparseable stdout is `Unavailable`, and an unknown document contract
/// names the version it could not parse. The adapter never maps an absent
/// policy run to a `PASS`, invokes through a shell, or trusts the tool's
/// own words: captured text passes the redaction pipeline, and an
/// absolute project root reported by the document is replaced with
/// `<project>` before findings are built.
pub fn run_driftwatch(dir: &Path, config: &DriftWatchConfig) -> PolicyOutcome {
    if !dir.is_dir() {
        return PolicyOutcome::Unavailable {
            reason: format!("project path '{}' is not a directory", dir.display()),
        };
    }
    let target = match resolve_binary(config) {
        Ok(target) => target,
        Err(reason) => return PolicyOutcome::Unavailable { reason },
    };
    let tool_version = probe_version(&target, config.timeout);
    let args = policy_surface_args(dir);
    let mut command = Command::new(&target);
    command.args(args).current_dir(dir);
    let output = match run_with_timeout(&mut command, config.timeout) {
        Ok(out) => out,
        Err(reason) => {
            return PolicyOutcome::Unavailable {
                reason: format!("driftwatch invocation failed: {reason}"),
            };
        }
    };
    let raw = match String::from_utf8(output.stdout) {
        Ok(s) => s,
        Err(_) => {
            return PolicyOutcome::Unavailable {
                reason: "driftwatch stdout was not valid UTF-8".to_string(),
            };
        }
    };
    let status_note = || {
        format!(
            "driftwatch exited with status {}: {}",
            output.status,
            stderr_summary(&output.stderr)
        )
    };
    let value: serde_json::Value = match serde_json::from_str(&raw) {
        Ok(value) => value,
        Err(err) => {
            // A stale sibling (one that rejects the JSON flag outright)
            // exits non-zero with empty stdout; name the failure the
            // process actually reported, not just the bare parse error.
            let mut reason =
                format!("driftwatch output is not parseable as a policy document: {err}");
            if !output.status.success() {
                reason.push_str(&format!("; {}", status_note()));
            }
            return PolicyOutcome::Unavailable { reason };
        }
    };
    let document_contract = value.get("contract").and_then(|c| c.as_str());
    if let Some(contract) = document_contract {
        if let Some(rest) = contract.strip_prefix("driftwatch-checker/") {
            // Same-major tolerance: the sibling's forward-compat rule adds
            // top-level fields only, so any 0.x document parses.
            let minor = rest.split('.').next().unwrap_or_default();
            if !contract.starts_with("driftwatch-checker/0.") || minor.is_empty() {
                return PolicyOutcome::Unavailable {
                    reason: format!(
                        "driftwatch returned an unsupported document contract '{contract}'; \
                         Forge parses '{CHECKER_REPORT_CONTRACT}' (same-major only); never a PASS"
                    ),
                };
            }
            return map_checker_document(value, &target, tool_version, dir);
        }
        if contract != POLICY_CONTRACT_VERSION {
            return PolicyOutcome::Unavailable {
                reason: format!(
                    "driftwatch returned an unsupported document contract '{contract}'; \
                     Forge parses '{POLICY_CONTRACT_VERSION}' and '{CHECKER_REPORT_CONTRACT}'; \
                     never a PASS"
                ),
            };
        }
    }
    if value.get("findings").is_some() {
        // Legacy adapter-report shape (fixture scripts and external
        // adapters emit it): non-zero exit stays unavailable, matching
        // the prior contract.
        if !output.status.success() {
            return PolicyOutcome::Unavailable {
                reason: status_note(),
            };
        }
        return map_legacy_report(value, tool_version, dir);
    }
    if value.get("blocked").is_some() && value.get("results").is_some() {
        return map_gate_document(value, &target, tool_version, dir);
    }
    if !output.status.success() {
        return PolicyOutcome::Unavailable {
            reason: status_note(),
        };
    }
    PolicyOutcome::Unavailable {
        reason: "driftwatch output is valid JSON but not a recognized policy document \
                 (checker report, gate status or adapter report)"
            .to_string(),
    }
}

/// Ordered binary resolution. An override runs exactly as given — it may
/// be a fixture path — and is never replaced by probing. Without one, the
/// first candidate present on `PATH` wins; absence of every candidate is
/// reported as unavailable naming the attempted names.
fn resolve_binary(config: &DriftWatchConfig) -> Result<OsString, String> {
    if let Some(binary) = &config.binary {
        return Ok(binary.clone());
    }
    let path_env = std::env::var_os("PATH").unwrap_or_default();
    first_binary_on_path(&path_env, DRIFTWATCH_BINARY_CANDIDATES)
        .map(|found| found.into_os_string())
        .ok_or_else(|| {
            format!(
                "no driftwatch binary found: tried '{}' (FORGE_DRIFTWATCH_BIN unset); local \
                 workflows continue without the policy plane",
                DRIFTWATCH_BINARY_CANDIDATES.join("', '")
            )
        })
}

fn binary_display_name(target: &OsStr) -> String {
    Path::new(target)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("driftwatch")
        .to_string()
}

/// Map the `driftwatch-checker/0.1.0` envelope: each alert becomes a
/// finding; failed/timeout/protocol-error rows become fail findings
/// carrying the row's own bounded error note; clean rows add nothing.
fn map_checker_document(
    value: serde_json::Value,
    target: &OsStr,
    tool_version: String,
    dir: &Path,
) -> PolicyOutcome {
    let project_root = value
        .get("project")
        .and_then(|p| p.as_str())
        .map(str::to_string);
    let mut findings: Vec<PolicyFinding> = Vec::new();
    let rows = match value.get("checkers").and_then(|c| c.as_array()) {
        Some(rows) => rows.clone(),
        None => {
            return PolicyOutcome::Unavailable {
                reason: "checker report document carries no 'checkers' array".to_string(),
            };
        }
    };
    for row in rows {
        let name = row
            .get("name")
            .and_then(|n| n.as_str())
            .unwrap_or("checker");
        let status = row
            .get("status")
            .and_then(|s| s.as_str())
            .unwrap_or("unknown");
        match status {
            "ok" => {}
            "alerting" => {
                let alerts = row
                    .get("alerts")
                    .and_then(|a| a.as_array())
                    .cloned()
                    .unwrap_or_default();
                if alerts.is_empty() {
                    findings.push(fail_finding(
                        name,
                        format!("checker '{name}' reported status 'alerting' with no alerts"),
                        vec![],
                    ));
                    continue;
                }
                for alert in alerts {
                    let severity = alert
                        .get("severity")
                        .and_then(|s| s.as_str())
                        .and_then(PolicySeverity::from_str)
                        // An alert with missing or unknown severity is
                        // still an alert: it must never read as PASS.
                        .unwrap_or(PolicySeverity::Warn);
                    let symbol = alert.get("symbol").and_then(|s| s.as_str());
                    let id = match symbol {
                        Some(symbol) if !symbol.is_empty() => format!("{name}/{symbol}"),
                        _ => name.to_string(),
                    };
                    let message = alert
                        .get("message")
                        .and_then(|m| m.as_str())
                        .unwrap_or("checker reported an alert");
                    let category = alert
                        .get("extra")
                        .and_then(|e| e.get("category"))
                        .and_then(|c| c.as_str())
                        .filter(|c| !c.trim().is_empty())
                        .unwrap_or("driftwatch")
                        .to_string();
                    let mut evidence = Vec::new();
                    if let Some(source) = alert.get("source").and_then(|s| s.as_str()) {
                        evidence.push(format!("source={source}"));
                    }
                    evidence.push(format!("checker={name} status={status}"));
                    findings.push(PolicyFinding {
                        id,
                        category,
                        severity,
                        applicable: true,
                        message: message.to_string(),
                        evidence,
                        reason: None,
                    });
                }
            }
            "failed" | "timeout" | "protocol-error" => {
                let note = row
                    .get("error")
                    .and_then(|e| e.as_str())
                    .unwrap_or("the checker produced no error note");
                findings.push(fail_finding(
                    name,
                    format!("checker '{name}' {status}: {note}"),
                    vec![format!("checker={name} status={status}")],
                ));
            }
            other => {
                findings.push(fail_finding(
                    name,
                    format!("checker '{name}' reported an unknown status '{other}'"),
                    vec![format!("checker={name} status={other}")],
                ));
            }
        }
    }
    let mut report = PolicyReport {
        tool: binary_display_name(target),
        tool_version: doc_version(&value).unwrap_or(tool_version),
        contract: POLICY_CONTRACT_VERSION.to_string(),
        source_revision: source_revision_for(dir),
        findings,
    };
    if let Some(root) = project_root {
        scrub_project_root(&mut report, Some(&root), dir);
    } else {
        scrub_project_root(&mut report, None, dir);
    }
    redact_report_in_place(&mut report);
    PolicyOutcome::Reported(report)
}

/// Map the gate status document (`gate --format json`). A blocked
/// aggregate maps its failing checks to fail findings — evidence, not
/// adapter failure — review-required checks to warns, and not-applicable
/// checks to not-applicable pass findings.
fn map_gate_document(
    value: serde_json::Value,
    target: &OsStr,
    tool_version: String,
    dir: &Path,
) -> PolicyOutcome {
    let results = match value.get("results").and_then(|r| r.as_array()) {
        Some(results) => results.clone(),
        None => {
            return PolicyOutcome::Unavailable {
                reason: "gate status document carries no 'results' array".to_string(),
            };
        }
    };
    let mut findings: Vec<PolicyFinding> = Vec::new();
    for result in results {
        let gate_id = result
            .get("gate_id")
            .and_then(|g| g.as_str())
            .unwrap_or("gate");
        let status = result
            .get("status")
            .and_then(|s| s.as_str())
            .unwrap_or("UNKNOWN");
        let diagnostic = result
            .get("diagnostic")
            .and_then(|d| d.as_str())
            .map(str::to_string);
        let remediation = result
            .get("remediation")
            .and_then(|r| r.as_str())
            .map(str::to_string);
        let missing: Vec<String> = result
            .get("missing_evidence")
            .and_then(|m| m.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default();
        match status {
            "PASS" => {}
            "FAIL" => {
                let mut evidence: Vec<String> =
                    missing.iter().map(|m| format!("missing={m}")).collect();
                if let Some(remediation) = &remediation {
                    evidence.push(format!("remediation={remediation}"));
                }
                evidence.push(format!("gate={gate_id} status=FAIL"));
                findings.push(fail_finding(
                    gate_id,
                    diagnostic.unwrap_or_else(|| format!("gate '{gate_id}' failed")),
                    evidence,
                ));
            }
            "REVIEW_REQUIRED" => {
                let mut evidence: Vec<String> =
                    missing.iter().map(|m| format!("missing={m}")).collect();
                if let Some(remediation) = &remediation {
                    evidence.push(format!("remediation={remediation}"));
                }
                evidence.push(format!("gate={gate_id} status=REVIEW_REQUIRED"));
                findings.push(PolicyFinding {
                    id: gate_id.to_string(),
                    category: "driftwatch".to_string(),
                    severity: PolicySeverity::Warn,
                    applicable: true,
                    message: diagnostic
                        .unwrap_or_else(|| format!("gate '{gate_id}' requires human review")),
                    evidence,
                    reason: None,
                });
            }
            "NOT_APPLICABLE" => findings.push(PolicyFinding {
                id: gate_id.to_string(),
                category: "driftwatch".to_string(),
                severity: PolicySeverity::Pass,
                applicable: false,
                message: format!("gate '{gate_id}' is not applicable"),
                evidence: vec![format!("gate={gate_id} status=NOT_APPLICABLE")],
                reason: diagnostic.or(remediation),
            }),
            other => findings.push(fail_finding(
                gate_id,
                format!("gate '{gate_id}' reported an unknown status '{other}'"),
                vec![format!("gate={gate_id} status={other}")],
            )),
        }
    }
    let blocked = value
        .get("blocked")
        .and_then(|b| b.as_bool())
        .unwrap_or(false);
    let aggregate = value
        .get("status")
        .and_then(|s| s.as_str())
        .unwrap_or("UNKNOWN");
    if blocked && findings.is_empty() {
        findings.push(fail_finding(
            "gate",
            format!("gate aggregate is blocked (status {aggregate}) without per-gate detail"),
            vec!["blocked=true".to_string()],
        ));
    }
    let mut report = PolicyReport {
        tool: binary_display_name(target),
        tool_version: doc_version(&value).unwrap_or(tool_version),
        contract: POLICY_CONTRACT_VERSION.to_string(),
        source_revision: source_revision_for(dir),
        findings,
    };
    // The gate document reports no project field; scrub Forge's own
    // execution directory from any path the gates printed.
    scrub_project_root(&mut report, None, dir);
    redact_report_in_place(&mut report);
    PolicyOutcome::Reported(report)
}

/// Legacy adapter-report shape: unchanged behavior for fixture scripts
/// and external adapters that already emit a `PolicyReport`.
fn map_legacy_report(value: serde_json::Value, tool_version: String, dir: &Path) -> PolicyOutcome {
    let mut report: PolicyReport = match serde_json::from_value(value) {
        Ok(report) => report,
        Err(err) => {
            return PolicyOutcome::Unavailable {
                reason: format!("driftwatch output is not parseable as policy report: {err}"),
            };
        }
    };
    if report.tool.is_empty() {
        report.tool = "driftwatch".to_string();
    }
    if report.tool_version.is_empty() {
        report.tool_version = tool_version;
    }
    if report.contract.is_empty() {
        report.contract = POLICY_CONTRACT_VERSION.to_string();
    }
    if report.source_revision.is_none() {
        report.source_revision = source_revision_for(dir);
    }
    // Server-side scope guarantee: every evidence line is redacted
    // through the same pipeline before any caller sees it, so a
    // credential-like value cannot reach storage or display.
    redact_report_in_place(&mut report);
    PolicyOutcome::Reported(report)
}

fn fail_finding(id: &str, message: String, evidence: Vec<String>) -> PolicyFinding {
    PolicyFinding {
        id: id.to_string(),
        category: "driftwatch".to_string(),
        severity: PolicySeverity::Fail,
        applicable: true,
        message,
        evidence,
        reason: None,
    }
}

fn doc_version(value: &serde_json::Value) -> Option<String> {
    let raw = value.get("version").and_then(|v| v.as_str())?;
    if raw.trim().is_empty() {
        return None;
    }
    Some(
        crate::policy::redact_credentials(raw)
            .chars()
            .take(120)
            .collect(),
    )
}

/// Replace host paths with `<project>` in every finding string. The
/// needles are the directory Forge actually executed in and — when the
/// document reports one — the tool's own project root (the sibling may
/// omit the optional field). Whatever the tool prints about absolute
/// paths on this machine never persists into Forge evidence.
fn scrub_project_root(report: &mut PolicyReport, root: Option<&str>, dir: &Path) {
    let mut needles: Vec<String> = Vec::new();
    if let Some(current) = dir.canonicalize().ok().map(|p| p.display().to_string()) {
        needles.push(current);
    }
    if let Some(reported) = root {
        if !needles.iter().any(|n| n == reported) {
            needles.push(reported.to_string());
        }
    }
    for finding in &mut report.findings {
        for needle in &needles {
            finding.message = finding.message.replace(needle, "<project>");
            finding.evidence = finding
                .evidence
                .iter()
                .map(|line| line.replace(needle, "<project>"))
                .collect();
        }
    }
}

#[derive(Debug)]
struct CapturedOutput {
    status: std::process::ExitStatus,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

fn run_with_timeout(command: &mut Command, timeout: Duration) -> Result<CapturedOutput, String> {
    use std::io::Read;
    use std::process::Stdio;
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null())
        .spawn()
        .map_err(|err| {
            if err.kind() == std::io::ErrorKind::NotFound {
                "binary not found on PATH".to_string()
            } else {
                format!("spawn failed: {err}")
            }
        })?;
    let start = SystemTime::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let mut stdout = Vec::new();
                let mut stderr = Vec::new();
                if let Some(mut out) = child.stdout.take() {
                    let _ = out.read_to_end(&mut stdout);
                }
                if let Some(mut err) = child.stderr.take() {
                    let _ = err.read_to_end(&mut stderr);
                }
                return Ok(CapturedOutput {
                    status,
                    stdout,
                    stderr,
                });
            }
            Ok(None) => {
                if start.elapsed().unwrap_or_default() > timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!(
                        "driftwatch exceeded timeout of {} seconds",
                        timeout.as_secs()
                    ));
                }
                std::thread::sleep(Duration::from_millis(25));
            }
            Err(err) => return Err(format!("wait failed: {err}")),
        }
    }
}

fn probe_version(binary: &OsString, timeout: Duration) -> String {
    let mut command = Command::new(binary);
    command.arg("--version");
    let output = match run_with_timeout(&mut command, timeout) {
        Ok(out) => out,
        Err(_) => return "unknown".to_string(),
    };
    if !output.status.success() {
        return "unknown".to_string();
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return "unknown".to_string();
    }
    // DriftWatch conventionally prints `driftwatch <version>`; keep the
    // second whitespace-separated token so `driftwatch 0.1.0` becomes
    // `0.1.0` and bare output is reported verbatim.
    let tokens: Vec<&str> = trimmed.split_whitespace().collect();
    if tokens.len() >= 2 {
        tokens[1].to_string()
    } else {
        trimmed.to_string()
    }
}

fn stderr_summary(stderr: &[u8]) -> String {
    let text = String::from_utf8_lossy(stderr);
    let trimmed = text.trim();
    if trimmed.is_empty() {
        "no stderr output".to_string()
    } else {
        const MAX: usize = 200;
        if trimmed.len() > MAX {
            format!("{}…", &trimmed[..MAX])
        } else {
            trimmed.to_string()
        }
    }
}

/// Build a fresh observation from a successful run. The caller (CLI or
/// registry) decides whether to persist it; the policy module never
/// writes registry rows itself.
pub fn observation_from_outcome(
    _dir: &Path,
    outcome: &PolicyOutcome,
    when: DateTime<Utc>,
) -> Option<PolicyObservation> {
    let report = outcome.report()?;
    Some(PolicyObservation {
        tool: report.tool.clone(),
        tool_version: report.tool_version.clone(),
        source_revision: report.source_revision,
        observed_at: when,
        finding_count: report.findings.len(),
        report: Some(report.clone()),
    })
}

/// Compute the source revision used for staleness comparison. The
/// revision is the latest modification time across the canonical manifest
/// and any configured driftwatch file inside `dir`; an empty directory
/// yields `None`. The value is always UTC to match the rest of the
/// registry timestamps.
pub fn source_revision_for(dir: &Path) -> Option<DateTime<Utc>> {
    let mut latest: Option<SystemTime> = None;
    for name in [
        "forge.yaml",
        "driftwatch.yaml",
        "driftwatch.yml",
        "driftwatch.json",
        ".driftwatch.yaml",
        ".driftwatch.yml",
    ] {
        let path = dir.join(name);
        if let Ok(meta) = fs::metadata(&path) {
            if let Ok(modified) = meta.modified() {
                latest = Some(latest.map_or(modified, |cur| cur.max(modified)));
            }
        }
    }
    if let Ok(meta) = fs::metadata(dir.join(".driftwatch")) {
        if meta.is_dir() {
            if let Ok(modified) = meta.modified() {
                latest = Some(latest.map_or(modified, |cur| cur.max(modified)));
            }
        }
    }
    let latest = latest?;
    let secs = latest.duration_since(UNIX_EPOCH).ok()?.as_secs() as i64;
    DateTime::<Utc>::from_timestamp(secs, 0)
}

/// Whether a previous observation is stale against the current project
/// state. Returns `false` when the observation carries no source
/// revision (older cache shape) or when the project is missing; in both
/// cases the doctor has other evidence to make the verdict.
pub fn observation_is_stale(observation: &PolicyObservation, current: Option<SystemTime>) -> bool {
    let Some(current) = current else {
        return false;
    };
    let Some(revision) = observation.source_revision else {
        return false;
    };
    let current_secs = current
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let current_dt = DateTime::<Utc>::from_timestamp(current_secs, 0);
    match current_dt {
        Some(now) => now > revision,
        None => false,
    }
}

/// Redact credential-like values from a single evidence string. The
/// rule set is intentionally conservative: only well-known token shapes
/// (AWS, GitHub, GitLab, Slack, JWT, private keys) and obvious
/// `key=value` secrets are replaced. Generic high-entropy values are
/// left alone so legitimate code is not over-scrubbed.
pub fn redact_credentials(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    loop {
        let (next_start, next_end) = match match_secret(rest) {
            Some(span) => span,
            None => {
                out.push_str(rest);
                return out;
            }
        };
        out.push_str(&rest[..next_start]);
        out.push_str("[REDACTED]");
        rest = &rest[next_end..];
    }
}

fn match_secret(input: &str) -> Option<(usize, usize)> {
    let mut best: Option<(usize, usize)> = None;
    for (start, end) in [
        span_aws_access_key(input),
        span_github_pat(input),
        span_github_fine_pat(input),
        span_gitlab_pat(input),
        span_slack_token(input),
        span_jwt(input),
        span_private_key(input),
        span_kv_secret(input),
    ]
    .into_iter()
    .flatten()
    {
        if best.is_none_or(|(s, _)| start < s) {
            best = Some((start, end));
        }
    }
    best
}

fn span_aws_access_key(input: &str) -> Option<(usize, usize)> {
    let bytes = input.as_bytes();
    let needle = b"AKIA";
    let start = find_subslice(bytes, needle)?;
    let mut end = start + needle.len();
    for _ in 0..16 {
        if end >= bytes.len() || !bytes[end].is_ascii_alphanumeric() {
            return None;
        }
        end += 1;
    }
    Some((start, end))
}

fn span_github_pat(input: &str) -> Option<(usize, usize)> {
    let bytes = input.as_bytes();
    let needle = b"ghp_";
    let start = find_subslice(bytes, needle)?;
    let mut end = start + needle.len();
    let mut matched = 0;
    while end < bytes.len() && bytes[end].is_ascii_alphanumeric() && matched < 64 {
        end += 1;
        matched += 1;
    }
    if matched < 20 {
        return None;
    }
    Some((start, end))
}

fn span_github_fine_pat(input: &str) -> Option<(usize, usize)> {
    let bytes = input.as_bytes();
    let needle = b"github_pat_";
    let start = find_subslice(bytes, needle)?;
    let mut end = start + needle.len();
    let mut matched = 0;
    while end < bytes.len()
        && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_')
        && matched < 128
    {
        end += 1;
        matched += 1;
    }
    if matched < 20 {
        return None;
    }
    Some((start, end))
}

fn span_gitlab_pat(input: &str) -> Option<(usize, usize)> {
    let bytes = input.as_bytes();
    let needle = b"glpat-";
    let start = find_subslice(bytes, needle)?;
    let mut end = start + needle.len();
    let mut matched = 0;
    while end < bytes.len()
        && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_' || bytes[end] == b'-')
        && matched < 128
    {
        end += 1;
        matched += 1;
    }
    if matched < 18 {
        return None;
    }
    Some((start, end))
}

fn span_slack_token(input: &str) -> Option<(usize, usize)> {
    let bytes = input.as_bytes();
    let needle = b"xox";
    let start = find_subslice(bytes, needle)?;
    if start + 4 >= bytes.len() || !matches!(bytes[start + 3], b'b' | b'a' | b'p' | b'r' | b's') {
        return None;
    }
    if bytes[start + 4] != b'-' {
        return None;
    }
    let mut end = start + 5;
    while end < bytes.len() && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'-') {
        end += 1;
    }
    Some((start, end))
}

fn span_jwt(input: &str) -> Option<(usize, usize)> {
    let bytes = input.as_bytes();
    let needle = b"eyJ";
    let start = find_subslice(bytes, needle)?;
    let parts = [start, 0, 0, 0];
    let _ = parts;
    // Token shape: header.payload.signature, each segment URL-safe
    // base64-ish. Reject if any segment contains whitespace or '='.
    let mut cursor = start;
    for seg in 0..3 {
        if seg > 0 {
            if cursor >= bytes.len() || bytes[cursor] != b'.' {
                return None;
            }
            cursor += 1;
        }
        let seg_start = cursor;
        while cursor < bytes.len() && bytes[cursor] != b'.' && !bytes[cursor].is_ascii_whitespace()
        {
            cursor += 1;
        }
        if cursor == seg_start {
            return None;
        }
        // Reject segments that are too short to be a JWT segment.
        if cursor - seg_start < 4 {
            return None;
        }
    }
    Some((start, cursor))
}

fn span_private_key(input: &str) -> Option<(usize, usize)> {
    let bytes = input.as_bytes();
    let begin = find_subslice(bytes, b"-----BEGIN ")?;
    let after = begin + b"-----BEGIN ".len();
    let line_end = bytes[after..]
        .iter()
        .position(|b| *b == b'\n')
        .unwrap_or(bytes.len() - after);
    let header = &bytes[after..after + line_end];
    if !header
        .iter()
        .all(|b| b.is_ascii_uppercase() || *b == b' ' || *b == b'-')
    {
        return None;
    }
    if !contains_ci(header, b"PRIVATE KEY") {
        return None;
    }
    let end_needle = b"-----END ";
    let rest = &bytes[after..];
    let end_off = find_subslice(rest, end_needle)?;
    let after_end = after + end_off + end_needle.len();
    let end_line = bytes[after_end..]
        .iter()
        .position(|b| *b == b'\n')
        .unwrap_or(bytes.len() - after_end);
    if !contains_ci(&bytes[after_end..after_end + end_line], b"PRIVATE KEY") {
        return None;
    }
    Some((begin, after_end + end_line))
}

fn contains_ci(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.len() > haystack.len() {
        return false;
    }
    haystack.windows(needle.len()).any(|w| {
        w.iter()
            .zip(needle.iter())
            .all(|(a, b)| a.eq_ignore_ascii_case(b))
    })
}

fn span_kv_secret(input: &str) -> Option<(usize, usize)> {
    let lower = input.to_ascii_lowercase();
    let mut keys: Vec<String> = vec![
        "password".to_string(),
        "passwd".to_string(),
        "pwd".to_string(),
        "secret".to_string(),
        "token".to_string(),
        "api_key".to_string(),
        "apikey".to_string(),
        "api-key".to_string(),
        "access_key".to_string(),
        "access-key".to_string(),
    ];
    for extra in crate::contract::secret_field_substrings() {
        let e = extra.to_ascii_lowercase();
        if !keys.iter().any(|k| k == &e) {
            keys.push(e);
        }
    }
    let keys = keys;
    let mut best: Option<(usize, usize)> = None;
    for key in &keys {
        let mut search_from = 0;
        while let Some(rel) = lower[search_from..].find(key.as_str()) {
            let key_start = search_from + rel;
            let key_end = key_start + key.len();
            // Require a non-letter boundary so `tokenized` does not match.
            let prev_ok = key_start == 0 || !input.as_bytes()[key_start - 1].is_ascii_alphabetic();
            let after = &input[key_end..];
            let after_trim = after.trim_start();
            let trim_len = after.len() - after_trim.len();
            if !prev_ok || after_trim.is_empty() || !matches!(after_trim.as_bytes()[0], b':' | b'=')
            {
                search_from = key_end;
                continue;
            }
            // Skip the separator.
            let sep_off = trim_len + 1;
            let bytes = input.as_bytes();
            let mut value_start = key_end + sep_off;
            // Skip one optional space after the separator.
            if value_start < bytes.len() && bytes[value_start] == b' ' {
                value_start += 1;
            }
            let mut end = value_start;
            while end < bytes.len() {
                let b = bytes[end];
                if b.is_ascii_whitespace() || b == b',' || b == b';' || b == b'}' || b == b']' {
                    break;
                }
                end += 1;
            }
            let value_len = end - value_start;
            if value_len >= 6 {
                let span = (key_start, end);
                if best.is_none_or(|(s, _)| key_start < s) {
                    best = Some(span);
                }
                // Continue scanning for the same key in case a later
                // occurrence of the same key produces an earlier span
                // in a different key iteration (the outer `for` will
                // still pick the earliest across all keys).
            }
            search_from = key_end;
        }
    }
    best
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

/// Redact credential-like values from a [`PolicyReport`] in place.
/// Public so callers (doctor, registry, future transports) can apply
/// the same pipeline when they consume a report from a source that did
/// not go through [`run_driftwatch`].
pub fn redact_report_in_place(report: &mut PolicyReport) {
    for finding in &mut report.findings {
        finding.message = redact_credentials(&finding.message);
        finding.evidence = std::mem::take(&mut finding.evidence)
            .into_iter()
            .map(|line| redact_credentials(&line))
            .collect();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn empty_report() -> PolicyReport {
        PolicyReport {
            tool: "driftwatch".to_string(),
            tool_version: "0.1.0".to_string(),
            contract: POLICY_CONTRACT_VERSION.to_string(),
            source_revision: None,
            findings: Vec::new(),
        }
    }

    fn write_script(dir: &Path, name: &str, body: &str) -> std::path::PathBuf {
        let path = dir.join(name);
        fs::write(&path, body).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        }
        path
    }

    fn script_config(path: &Path) -> DriftWatchConfig {
        DriftWatchConfig {
            binary: Some(path.as_os_str().to_os_string()),
            timeout: Duration::from_secs(5),
        }
    }

    fn fixture_doc(name: &str) -> serde_json::Value {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/driftwatch")
            .join(name);
        let text = fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("fixture {}: {err}", path.display()));
        serde_json::from_str(&text).expect("fixture is valid JSON")
    }

    #[test]
    fn verbatim_sibling_fixtures_project_honestly() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("forge.yaml"), "schema: 1\n").unwrap();
        let target = OsStr::new("/usr/bin/driftwatchdog");

        // The passing document maps to an honest empty report.
        match map_checker_document(
            fixture_doc("checker-report-passing.json"),
            target,
            "probe".to_string(),
            tmp.path(),
        ) {
            PolicyOutcome::Reported(report) => {
                assert!(report.findings.is_empty(), "{:?}", report.findings);
                assert_eq!(report.tool, "driftwatchdog");
                assert_eq!(report.tool_version, "0.1.0", "document version wins");
                assert_eq!(report.contract, POLICY_CONTRACT_VERSION);
            }
            other => panic!("passing fixture must report: {other:?}"),
        }

        // The alerting document: error severity fails, declared category
        // survives, and the fixture's embedded GitHub PAT is redacted.
        match map_checker_document(
            fixture_doc("checker-report-alerting.json"),
            target,
            "probe".to_string(),
            tmp.path(),
        ) {
            PolicyOutcome::Reported(report) => {
                // Mixed outcomes in one document: alerting maps to its
                // alert findings and the broken checker stays visible as
                // its own fail finding — no crash, no masking.
                assert_eq!(report.findings.len(), 2, "{:?}", report.findings);
                let finding = report
                    .findings
                    .iter()
                    .find(|f| f.id == "auth/AUTH-001")
                    .expect("alert finding");
                assert_eq!(finding.severity, PolicySeverity::Fail);
                assert_eq!(finding.category, "security");
                assert!(!finding.message.contains("ghp_"), "{}", finding.message);
                assert!(
                    finding.message.contains("[REDACTED]"),
                    "{}",
                    finding.message
                );
                let broken = report
                    .findings
                    .iter()
                    .find(|f| f.id == "broken")
                    .expect("isolated failure stays visible");
                assert_eq!(broken.severity, PolicySeverity::Fail);
                assert!(broken.message.contains("protocol-error"), "{broken:?}");
            }
            other => panic!("alerting fixture must report: {other:?}"),
        }

        // The blocked gate document produces the failing gate's findings.
        match map_gate_document(
            fixture_doc("gate-status-blocked.json"),
            target,
            "probe".to_string(),
            tmp.path(),
        ) {
            PolicyOutcome::Reported(report) => {
                let finding = report
                    .findings
                    .iter()
                    .find(|f| f.id == "docs")
                    .expect("blocked gate names its failing check");
                assert_eq!(finding.severity, PolicySeverity::Fail);
                assert!(
                    report
                        .findings
                        .iter()
                        .any(|f| f.evidence.iter().any(|e| e.starts_with("remediation="))),
                    "{:?}",
                    report.findings
                );
            }
            other => panic!("blocked gate must report: {other:?}"),
        }
        // A passing gate document stays reported with nothing to show.
        match map_gate_document(
            fixture_doc("gate-status-pass.json"),
            target,
            "probe".to_string(),
            tmp.path(),
        ) {
            PolicyOutcome::Reported(report) => assert!(report.findings.is_empty()),
            other => panic!("pass gate must report: {other:?}"),
        }

        // The unknown-contract fixture is refused through the real entry
        // point, naming the version and never reporting.
        let script = write_script(
            tmp.path(),
            "dw-9.sh",
            &format!(
                "#!/bin/sh\ncat '{}'\n",
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("tests/fixtures/driftwatch/checker-report-unknown-contract.json")
                    .display()
            ),
        );
        let outcome = run_driftwatch(tmp.path(), &script_config(&script));
        match outcome {
            PolicyOutcome::Unavailable { reason } => {
                assert!(reason.contains("driftwatch-checker/9.0.0"), "{reason}");
            }
            other => panic!("unknown contract must refuse: {other:?}"),
        }
    }

    #[test]
    fn first_binary_on_path_prefers_driftwatchdog_then_alias() {
        let tmp = TempDir::new().unwrap();
        let bin = tmp.path().join("bin");
        fs::create_dir_all(&bin).unwrap();
        let both = write_script(&bin, "driftwatchdog", "#!/bin/sh\necho wd\n");
        let alias = write_script(&bin, "driftwatch", "#!/bin/sh\necho dw\n");
        let path = std::env::join_paths([&bin]).unwrap();
        assert_eq!(
            first_binary_on_path(&path, DRIFTWATCH_BINARY_CANDIDATES),
            Some(both)
        );
        // Alias-only hosts resolve the npm launcher name.
        fs::remove_file(tmp.path().join("bin").join("driftwatchdog")).unwrap();
        assert_eq!(
            first_binary_on_path(&path, DRIFTWATCH_BINARY_CANDIDATES),
            Some(alias)
        );
        // Empty PATH and non-executable files find nothing.
        assert_eq!(
            first_binary_on_path(OsStr::new(""), DRIFTWATCH_BINARY_CANDIDATES),
            None
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let plain = bin.join("driftwatchdog");
            fs::write(&plain, "not executable").unwrap();
            fs::set_permissions(&plain, fs::Permissions::from_mode(0o644)).unwrap();
            // The non-executable first candidate is skipped; the probe
            // continues to the alias rather than selecting a dead file.
            let found = first_binary_on_path(&path, DRIFTWATCH_BINARY_CANDIDATES);
            assert_eq!(found, Some(bin.join("driftwatch")));
            let plain_alias = bin.join("driftwatch");
            fs::set_permissions(&plain_alias, fs::Permissions::from_mode(0o644)).unwrap();
            assert_eq!(
                first_binary_on_path(&path, DRIFTWATCH_BINARY_CANDIDATES),
                None,
                "a non-executable candidate must not be selected"
            );
        }
    }

    #[test]
    fn surface_follows_gate_manifests() {
        let tmp = TempDir::new().unwrap();
        assert_eq!(
            policy_surface_args(tmp.path()),
            vec!["check", "--dry-run", "--format", "json"]
        );
        fs::write(tmp.path().join("gate.toml"), "[gate]\n").unwrap();
        assert_eq!(
            policy_surface_args(tmp.path()),
            vec!["gate", "--format", "json"]
        );
        let tmp2 = TempDir::new().unwrap();
        fs::create_dir_all(tmp2.path().join(".ai-gate")).unwrap();
        fs::write(tmp2.path().join(".ai-gate/gate.yaml"), "version: 1\n").unwrap();
        assert_eq!(
            policy_surface_args(tmp2.path()),
            vec!["gate", "--format", "json"]
        );
    }

    #[test]
    fn checker_envelope_maps_alerts_rows_and_scrubs_roots() {
        let tmp = TempDir::new().unwrap();
        let proj = tmp.path().join("proj");
        fs::create_dir_all(&proj).unwrap();
        fs::write(proj.join("forge.yaml"), "schema: 1\n").unwrap();
        let doc = format!(
            "{{\"contract\":\"driftwatch-checker/0.1.0\",\"tool\":\"driftwatch\",\"version\":\"0.4.2\",\"project\":\"{}\",\"generated_at\":\"2026-09-24T00:00:00Z\",\"checkers\":[{{\"name\":\"sec-scan\",\"status\":\"alerting\",\"alerts\":[{{\"severity\":\"warning\",\"message\":\"auth gap in {}\",\"source\":\"src/main.rs\",\"symbol\":\"SEC-001\",\"extra\":{{\"category\":\"security\"}}}},{{\"message\":\"unclassified\",\"source\":\"x\",\"symbol\":\"X-1\"}}]}},{{\"name\":\"gate-probe\",\"status\":\"failed\",\"alerts\":[],\"error\":\"checker exited with status 3\"}},{{\"name\":\"quiet\",\"status\":\"ok\",\"alerts\":[]}}],\"summary\":{{\"total\":3,\"ok\":1,\"alerting\":1,\"failed\":1,\"timeout\":0,\"protocol_error\":0,\"alerts\":2}}}}",
            tmp.path().display(),
            tmp.path().display()
        );
        let script = write_script(
            &proj,
            "dw-env.sh",
            &format!(
                "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo 'driftwatch 0.4.2'; exit 0; fi\ncat <<'DWEOF'\n{doc}\nDWEOF\n"
            ),
        );
        let outcome = run_driftwatch(&proj, &script_config(&script));
        let report = outcome.report().expect("envelope maps to a report");
        // The observation names the resolved binary and the document's version.
        assert_eq!(report.tool, "dw-env");
        assert_eq!(report.tool_version, "0.4.2");
        assert_eq!(report.contract, POLICY_CONTRACT_VERSION);
        assert_eq!(report.findings.len(), 3, "{:?}", report.findings);
        let alert = &report.findings[0];
        assert_eq!(alert.id, "sec-scan/SEC-001");
        assert_eq!(alert.category, "security", "declared category preserved");
        assert_eq!(alert.severity, PolicySeverity::Warn);
        // Host paths from the document are scrubbed to <project>.
        assert!(
            !alert.message.contains(&tmp.path().display().to_string()),
            "leaked root: {}",
            alert.message
        );
        assert!(alert.message.contains("<project>"), "{}", alert.message);
        // A severity-less alert stays warn, never pass.
        assert_eq!(report.findings[1].id, "sec-scan/X-1");
        assert_eq!(report.findings[1].severity, PolicySeverity::Warn);
        // The failed row becomes a fail finding carrying the runtime note.
        let failed = &report.findings[2];
        assert_eq!(failed.id, "gate-probe");
        assert_eq!(failed.severity, PolicySeverity::Fail);
        assert!(failed.message.contains("exited with status 3"));
        // The clean row adds no findings; report source_revision is stamped.
        assert_eq!(
            report.findings.iter().filter(|f| f.id == "quiet").count(),
            0
        );
        assert!(report.source_revision.is_some());
    }

    #[test]
    fn checker_envelope_nonzero_exit_is_still_evidence() {
        let tmp = TempDir::new().unwrap();
        let proj = tmp.path().join("proj");
        fs::create_dir_all(&proj).unwrap();
        fs::write(proj.join("forge.yaml"), "schema: 1\n").unwrap();
        let script = write_script(
            &proj,
            "dw-part.sh",
            "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then exit 1; fi\ncat <<'DWEOF'\n{\"contract\":\"driftwatch-checker/0.1.0\",\"tool\":\"driftwatch\",\"version\":\"0.4.2\",\"checkers\":[{\"name\":\"boom\",\"status\":\"timeout\",\"alerts\":[],\"error\":\"checker exceeded timeout\"}],\"summary\":{\"total\":1,\"ok\":0,\"alerting\":0,\"failed\":0,\"timeout\":1,\"protocol_error\":0,\"alerts\":0}}\nDWEOF\nexit 2\n",
        );
        let outcome = run_driftwatch(&proj, &script_config(&script));
        let report = outcome
            .report()
            .expect("parseable failing document reports");
        assert_eq!(report.findings.len(), 1);
        assert_eq!(report.findings[0].severity, PolicySeverity::Fail);
        assert_eq!(
            report.findings[0].message,
            "checker 'boom' timeout: checker exceeded timeout"
        );
        assert_eq!(
            report.tool_version, "0.4.2",
            "document version wins over probe"
        );
    }

    #[test]
    fn blocked_gate_document_maps_findings_not_unavailable() {
        let tmp = TempDir::new().unwrap();
        let proj = tmp.path().join("proj");
        fs::create_dir_all(&proj).unwrap();
        fs::write(proj.join("forge.yaml"), "schema: 1\n").unwrap();
        // gate-managed surface selection proves itself: the stub only
        // answers the gate argv with a document.
        fs::write(proj.join("gate.toml"), "[gate]\nrules = []\n").unwrap();
        let script = write_script(
            &proj,
            "dw-gate.sh",
            "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo 'driftwatchdog 0.4.2'; exit 0; fi\ncat <<'DWEOF'\n{\"status\":\"FAIL\",\"blocked\":true,\"failures\":[\"product-quality\"],\"pending_reviews\":[\"ux-review\"],\"not_applicable\":[],\"manifest_digest\":\"abc\",\"rule_pack_version\":1,\"results\":[{\"gate_id\":\"build\",\"source\":\"ci\",\"status\":\"PASS\",\"severity\":\"info\",\"findings\":[],\"evidence\":[],\"missing_evidence\":[]},{\"gate_id\":\"product-quality\",\"source\":\"quality\",\"status\":\"FAIL\",\"severity\":\"error\",\"findings\":[],\"evidence\":[],\"missing_evidence\":[\"tests:run\"],\"diagnostic\":\"tests failing\",\"remediation\":\"run cargo test\"},{\"gate_id\":\"ux-review\",\"source\":\"ai\",\"status\":\"REVIEW_REQUIRED\",\"severity\":\"warning\",\"findings\":[],\"evidence\":[],\"missing_evidence\":[\"checker:run\"]},{\"gate_id\":\"deploy\",\"source\":\"gate\",\"status\":\"NOT_APPLICABLE\",\"severity\":\"info\",\"findings\":[],\"evidence\":[],\"missing_evidence\":[]}]}\nDWEOF\nexit 1\n",
        );
        let outcome = run_driftwatch(&proj, &script_config(&script));
        let report = outcome.report().expect("blocked gate is evidence");
        let by_id: std::collections::BTreeMap<&str, &PolicyFinding> =
            report.findings.iter().map(|f| (f.id.as_str(), f)).collect();
        assert_eq!(by_id["product-quality"].severity, PolicySeverity::Fail);
        assert!(by_id["product-quality"].message.contains("tests failing"));
        assert!(by_id["product-quality"]
            .evidence
            .iter()
            .any(|e| e == "missing=tests:run"));
        assert_eq!(by_id["ux-review"].severity, PolicySeverity::Warn);
        assert!(!by_id.contains_key("build"), "PASS adds nothing");
        assert!(!by_id["deploy"].applicable);
        assert_eq!(by_id["deploy"].severity, PolicySeverity::Pass);
    }

    #[test]
    fn unknown_document_contract_refuses_naming_version() {
        let tmp = TempDir::new().unwrap();
        let proj = tmp.path().join("proj");
        fs::create_dir_all(&proj).unwrap();
        fs::write(proj.join("forge.yaml"), "schema: 1\n").unwrap();
        let script = write_script(
            &proj,
            "dw-future.sh",
            "#!/bin/sh\ncat <<'DWEOF'\n{\"contract\":\"driftwatch-checker/2.0.0\",\"tool\":\"driftwatch\",\"version\":\"9.9\",\"checkers\":[],\"summary\":{}}\nDWEOF\n",
        );
        let outcome = run_driftwatch(&proj, &script_config(&script));
        let reason = match outcome {
            PolicyOutcome::Unavailable { reason } => reason,
            other => panic!("unknown contract must refuse: {other:?}"),
        };
        assert!(reason.contains("driftwatch-checker/2.0.0"), "{reason}");
        assert!(reason.contains("never a PASS"), "{reason}");
        // Same-major additions parse (forward-compat rule).
        let script = write_script(
            &proj,
            "dw-minor.sh",
            "#!/bin/sh\ncat <<'DWEOF'\n{\"contract\":\"driftwatch-checker/0.2.0\",\"tool\":\"driftwatch\",\"version\":\"9.9\",\"checkers\":[],\"summary\":{},\"future_field\":true}\nDWEOF\n",
        );
        let outcome = run_driftwatch(&proj, &script_config(&script));
        assert!(outcome.report().is_some(), "same-major must parse");
    }

    #[test]
    fn unrecognized_json_documents_never_report() {
        let tmp = TempDir::new().unwrap();
        let proj = tmp.path().join("proj");
        fs::create_dir_all(&proj).unwrap();
        fs::write(proj.join("forge.yaml"), "schema: 1\n").unwrap();
        let script = write_script(
            &proj,
            "dw-junk.sh",
            "#!/bin/sh\necho '{\"surprise\":true}'\n",
        );
        let outcome = run_driftwatch(&proj, &script_config(&script));
        assert!(!outcome.is_reported());
        let reason = match outcome {
            PolicyOutcome::Unavailable { reason } => reason,
            _ => unreachable!(),
        };
        assert!(
            reason.contains("not a recognized policy document"),
            "{reason}"
        );
    }

    #[test]
    fn missing_override_does_not_fall_through_to_candidates() {
        // Operator override runs exactly that binary: a dead override is
        // reported unavailable, never silently replaced by PATH probing.
        let tmp = TempDir::new().unwrap();
        let config = DriftWatchConfig {
            binary: Some(OsString::from(
                "/definitely/not/a/real/driftwatch-override-xyz",
            )),
            timeout: Duration::from_secs(2),
        };
        let outcome = run_driftwatch(tmp.path(), &config);
        let reason = match outcome {
            PolicyOutcome::Unavailable { reason } => reason,
            other => panic!("dead override must not report: {other:?}"),
        };
        assert!(reason.contains("not found"), "{reason}");
    }

    #[test]
    fn redacts_aws_github_gitlab_jwt_and_kv_secrets() {
        let cases = [
            ("aws key AKIAABCDEFGHIJKLMNOP", "AKIAABCDEFGHIJKLMNOP"),
            (
                "github token ghp_abcdefghijklmnopqrstuvwxyz0123456789",
                "ghp_abcdefghijklmnopqrstuvwxyz0123456789",
            ),
            (
                "gitlab token glpat-abcdefghijklmnopqrstuv",
                "glpat-abcdefghijklmnopqrstuv",
            ),
            (
                "jwt eyJhbGciOi.eyJzdWIiOi.signaturehere",
                "eyJhbGciOi.eyJzdWIiOi.signaturehere",
            ),
            ("password=hunter2hunter2", "hunter2hunter2"),
            (
                "api_key: bearer-token-1234567890",
                "bearer-token-1234567890",
            ),
        ];
        for (input, secret) in cases {
            let out = redact_credentials(input);
            assert!(out.contains("[REDACTED]"), "input: {input}\noutput: {out}");
            assert!(
                !out.contains(secret),
                "secret leaked: input: {input}\noutput: {out}"
            );
        }
    }

    #[test]
    fn redacts_private_key_blocks() {
        let input = "BEGIN MARKER\n-----BEGIN RSA PRIVATE KEY-----\nABCDEF\n-----END RSA PRIVATE KEY-----\nAFTER";
        let out = redact_credentials(input);
        assert!(!out.contains("ABCDEF"), "{out}");
        assert!(out.contains("BEGIN MARKER"), "{out}");
        assert!(out.contains("AFTER"), "{out}");
        assert!(out.contains("[REDACTED]"), "{out}");
    }

    #[test]
    fn redacts_slack_token() {
        let out = redact_credentials("token: xoxb-1234567890-12345-abcdefghijkl");
        assert!(out.contains("[REDACTED]"), "{out}");
        assert!(!out.contains("xoxb-"), "{out}");
    }

    #[test]
    fn leaves_benign_evidence_unchanged() {
        let inputs = [
            "no credentials in here",
            "package.json references react and typescript",
            "license: MIT",
            "auth: required for endpoints",
        ];
        for input in inputs {
            assert_eq!(redact_credentials(input), input, "input: {input}");
        }
    }

    #[test]
    fn does_not_match_short_token_shapes() {
        // No high-entropy content, no real credential — must stay verbatim.
        let inputs = [
            "AKIA",       // 4 chars, no payload
            "ghp_short",  // too short to be a real PAT
            "glpat-x",    // too short
            "password=x", // too short to be a real secret
        ];
        for input in inputs {
            assert_eq!(redact_credentials(input), input, "input: {input}");
        }
    }

    #[test]
    fn redact_report_runs_through_every_finding() {
        let mut report = empty_report();
        report.findings.push(PolicyFinding {
            id: "AUTH-001".to_string(),
            category: "security".to_string(),
            severity: PolicySeverity::Fail,
            applicable: true,
            message: "secret leaked: token=abcdef0123456789".to_string(),
            evidence: vec!["github token ghp_abcdefghijklmnopqrstuvwxyz0123456789".to_string()],
            reason: None,
        });
        redact_report_in_place(&mut report);
        assert!(report.findings[0].message.contains("[REDACTED]"));
        assert!(report.findings[0].evidence[0].contains("[REDACTED]"));
        assert!(!report.findings[0].evidence[0].contains("ghp_"));
    }

    #[test]
    fn source_revision_uses_latest_known_mtime() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("forge.yaml");
        fs::write(&path, "schema: 1\n").unwrap();
        let rev = source_revision_for(tmp.path());
        assert!(rev.is_some());
    }

    #[test]
    fn source_revision_is_none_for_empty_directory() {
        let tmp = TempDir::new().unwrap();
        assert!(source_revision_for(tmp.path()).is_none());
    }

    #[test]
    fn parse_severity_accepts_common_spellings() {
        for (raw, expected) in [
            ("pass", Some(PolicySeverity::Pass)),
            ("WARN", Some(PolicySeverity::Warn)),
            ("error", Some(PolicySeverity::Fail)),
            ("fatal", Some(PolicySeverity::Fail)),
            ("info", Some(PolicySeverity::Pass)),
            ("unknown", None),
        ] {
            assert_eq!(PolicySeverity::parse(raw), expected, "input: {raw}");
        }
    }

    #[test]
    fn missing_binary_reports_unavailable_with_reason() {
        let tmp = TempDir::new().unwrap();
        let config = DriftWatchConfig {
            binary: Some(OsString::from("definitely-not-a-real-binary-xyz")),
            timeout: Duration::from_secs(2),
        };
        let outcome = run_driftwatch(tmp.path(), &config);
        assert!(!outcome.is_reported());
        let reason = match outcome {
            PolicyOutcome::Unavailable { reason } => reason,
            _ => unreachable!(),
        };
        assert!(
            reason.contains("not found") || reason.contains("binary not found"),
            "reason: {reason}"
        );
    }

    #[test]
    fn missing_project_path_reports_unavailable() {
        let config = DriftWatchConfig::from_env();
        let outcome = run_driftwatch(Path::new("/definitely/not/here/zzz"), &config);
        assert!(!outcome.is_reported());
    }

    #[test]
    fn parses_and_normalizes_well_formed_report() {
        let tmp = TempDir::new().unwrap();
        let script = tmp.path().join("fake-driftwatch.sh");
        fs::write(
            &script,
            "#!/bin/sh\n\
             if [ \"$1\" = \"--version\" ]; then\n\
             \techo 'driftwatch 0.1.0'\n\
             \texit 0\n\
             fi\n\
             echo '{\"tool\":\"driftwatch\",\"tool_version\":\"0.1.0\",\"contract\":\"0.1.0\",\"source_revision\":null,\"findings\":[{\"id\":\"AUTH-001\",\"category\":\"security\",\"severity\":\"fail\",\"applicable\":true,\"message\":\"missing auth markers\",\"evidence\":[\"Cargo.toml has no auth dep\"]}]}'\n",
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&script).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&script, perms).unwrap();
        }
        fs::write(tmp.path().join("forge.yaml"), "schema: 1\n").unwrap();
        let config = DriftWatchConfig {
            binary: Some(OsString::from(script.as_os_str())),
            timeout: Duration::from_secs(2),
        };
        let outcome = run_driftwatch(tmp.path(), &config);
        let report = outcome.report().expect("report should parse");
        assert_eq!(report.tool, "driftwatch");
        assert_eq!(report.tool_version, "0.1.0");
        assert_eq!(report.findings.len(), 1);
        assert_eq!(report.findings[0].id, "AUTH-001");
        assert_eq!(report.findings[0].severity, PolicySeverity::Fail);
    }

    #[test]
    fn invalid_output_reports_unavailable() {
        let tmp = TempDir::new().unwrap();
        let script = tmp.path().join("fake-driftwatch.sh");
        fs::write(
            &script,
            "#!/bin/sh\n\
             if [ \"$1\" = \"--version\" ]; then\n\
             \techo 'driftwatch 0.1.0'\n\
             \texit 0\n\
             fi\n\
             echo 'this is not json'\n",
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&script).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&script, perms).unwrap();
        }
        fs::write(tmp.path().join("forge.yaml"), "schema: 1\n").unwrap();
        let config = DriftWatchConfig {
            binary: Some(OsString::from(script.as_os_str())),
            timeout: Duration::from_secs(2),
        };
        let outcome = run_driftwatch(tmp.path(), &config);
        assert!(!outcome.is_reported());
    }

    #[test]
    fn non_zero_exit_reports_unavailable() {
        let tmp = TempDir::new().unwrap();
        let script = tmp.path().join("fake-driftwatch.sh");
        fs::write(
            &script,
            "#!/bin/sh\n\
             if [ \"$1\" = \"--version\" ]; then\n\
             \techo 'driftwatch 0.1.0'\n\
             \texit 0\n\
             fi\n\
             echo 'fatal: profile mismatch' >&2\n\
             exit 2\n",
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&script).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&script, perms).unwrap();
        }
        fs::write(tmp.path().join("forge.yaml"), "schema: 1\n").unwrap();
        let config = DriftWatchConfig {
            binary: Some(OsString::from(script.as_os_str())),
            timeout: Duration::from_secs(2),
        };
        let outcome = run_driftwatch(tmp.path(), &config);
        assert!(!outcome.is_reported());
    }

    #[test]
    fn observation_stale_when_source_advanced() {
        let past: DateTime<Utc> = DateTime::parse_from_rfc3339("2000-01-01T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let obs = PolicyObservation {
            tool: "driftwatch".to_string(),
            tool_version: "0.1.0".to_string(),
            source_revision: Some(past),
            observed_at: past,
            finding_count: 0,
            report: None,
        };
        let now = SystemTime::now();
        assert!(observation_is_stale(&obs, Some(now)));
        let old = UNIX_EPOCH + Duration::from_secs(60);
        assert!(!observation_is_stale(&obs, Some(old)));
    }

    #[test]
    fn observation_without_revision_is_never_stale() {
        let obs = PolicyObservation {
            tool: "driftwatch".to_string(),
            tool_version: "0.1.0".to_string(),
            source_revision: None,
            observed_at: Utc::now(),
            finding_count: 0,
            report: None,
        };
        assert!(!observation_is_stale(&obs, Some(SystemTime::now())));
    }

    #[test]
    fn redacts_multiple_credentials_in_one_evidence() {
        let input = "see AKIAABCDEFGHIJKLMNOP and ghp_abcdefghijklmnopqrstuvwxyz0123456789";
        let out = redact_credentials(input);
        assert_eq!(out.matches("[REDACTED]").count(), 2, "output: {out}");
    }

    #[test]
    fn redacts_token_and_password_in_one_evidence() {
        let input =
            "config/secret.yaml contains token=abcdef0123456789 and password=hunter2hunter2";
        let out = redact_credentials(input);
        assert_eq!(out.matches("[REDACTED]").count(), 2, "output: {out}");
        assert!(!out.contains("abcdef0123456789"), "leaked: {out}");
        assert!(!out.contains("hunter2hunter2"), "leaked: {out}");
    }
}
