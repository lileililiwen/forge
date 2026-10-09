//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use chrono::{DateTime, Utc};
use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use super::constants::{
    CHECKER_REPORT_CONTRACT, DRIFTWATCH_BINARY_CANDIDATES, POLICY_CONTRACT_VERSION,
};
use super::model::{
    CapturedOutput, DriftWatchConfig, PolicyFinding, PolicyObservation, PolicyOutcome,
    PolicyReport, PolicySeverity,
};
use super::redaction::redact_report_in_place;

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
pub(super) fn map_checker_document(
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
pub(super) fn map_gate_document(
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
