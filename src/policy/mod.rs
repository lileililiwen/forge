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

use std::ffi::OsString;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Contract data version for the DriftWatch adapter and the
/// `PolicyObservation` it produces. Independent of DriftWatch's own
/// tool version (reported per run inside [`PolicyReport::tool_version`]).
pub const POLICY_CONTRACT_VERSION: &str = "0.1.0";

/// Default binary used to invoke DriftWatch. Overridable through the
/// `FORGE_DRIFTWATCH_BIN` environment variable so tests can substitute a
/// fixture binary; nothing in the source tree rewrites this value.
pub const DEFAULT_DRIFTWATCH_BIN: &str = "driftwatch";

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
/// `FORGE_DRIFTWATCH_BIN` environment variable (or a default) plus the
/// per-run timeout, then hands it to [`run_driftwatch`].
#[derive(Debug, Clone)]
pub struct DriftWatchConfig {
    pub binary: OsString,
    pub timeout: Duration,
}

impl DriftWatchConfig {
    /// Resolve the adapter binary from the environment, falling back to
    /// [`DEFAULT_DRIFTWATCH_BIN`]. Whitespace-only values are ignored so
    /// `FORGE_DRIFTWATCH_BIN=""` keeps the default.
    pub fn from_env() -> Self {
        let binary = std::env::var_os("FORGE_DRIFTWATCH_BIN")
            .filter(|v| !v.as_os_str().is_empty())
            .unwrap_or_else(|| OsString::from(DEFAULT_DRIFTWATCH_BIN));
        DriftWatchConfig {
            binary,
            timeout: DEFAULT_TIMEOUT,
        }
    }
}

/// Invoke DriftWatch for `dir` and return the normalized outcome.
///
/// The process is started with `Command::new(binary).args(...)` — never
/// through a shell — and its current directory is `dir` so DriftWatch can
/// only observe the project it was asked to evaluate. stdout is the only
/// trusted channel; stderr is captured into the unavailable reason for
/// diagnostics. A version probe (`<binary> --version`) is performed first
/// so the report names the tool version that produced the findings, and
/// failure of the probe is not a hard error: the adapter still runs the
/// `check` invocation and records `unknown` for the tool version when
/// probe output is missing or unparseable.
pub fn run_driftwatch(dir: &Path, config: &DriftWatchConfig) -> PolicyOutcome {
    if !dir.is_dir() {
        return PolicyOutcome::Unavailable {
            reason: format!("project path '{}' is not a directory", dir.display()),
        };
    }
    let tool_version = probe_version(&config.binary, config.timeout);
    let mut command = Command::new(&config.binary);
    command
        .arg("check")
        .arg("--project")
        .arg(dir)
        .arg("--format")
        .arg("json")
        .current_dir(dir);
    let output = match run_with_timeout(&mut command, config.timeout) {
        Ok(out) => out,
        Err(reason) => {
            return PolicyOutcome::Unavailable {
                reason: format!("driftwatch invocation failed: {reason}"),
            };
        }
    };
    if !output.status.success() {
        return PolicyOutcome::Unavailable {
            reason: format!(
                "driftwatch exited with status {}: {}",
                output.status,
                stderr_summary(&output.stderr)
            ),
        };
    }
    let raw = match String::from_utf8(output.stdout) {
        Ok(s) => s,
        Err(_) => {
            return PolicyOutcome::Unavailable {
                reason: "driftwatch stdout was not valid UTF-8".to_string(),
            };
        }
    };
    let mut report: PolicyReport = match serde_json::from_str(&raw) {
        Ok(r) => r,
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
        report.tool_version = tool_version.clone();
    }
    // Server-side scope guarantee: every evidence line is redacted
    // through the same pipeline before any caller sees it, so a
    // credential-like value cannot reach storage or display.
    redact_report_in_place(&mut report);
    if report.contract.is_empty() {
        report.contract = POLICY_CONTRACT_VERSION.to_string();
    }
    if report.source_revision.is_none() {
        report.source_revision = source_revision_for(dir);
    }
    PolicyOutcome::Reported(report)
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
    let keys = [
        "password",
        "passwd",
        "pwd",
        "secret",
        "token",
        "api_key",
        "apikey",
        "api-key",
        "access_key",
        "access-key",
    ];
    let mut best: Option<(usize, usize)> = None;
    for key in keys {
        let mut search_from = 0;
        while let Some(rel) = lower[search_from..].find(key) {
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
            binary: OsString::from("definitely-not-a-real-binary-xyz"),
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
            binary: OsString::from(script.as_os_str()),
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
            binary: OsString::from(script.as_os_str()),
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
            binary: OsString::from(script.as_os_str()),
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
