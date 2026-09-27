//! Shared gate runtime evidence (`gate-runtime-evidence`).
//!
//! Driftwatchdog's Gate owns plan resolution (`gate.toml`,
//! `.ai-gate/gate.yaml`, rule-pack identity, blocking policy), child
//! execution, its own `gate_runs` history and exit semantics. This module
//! owns only the Forge side of that boundary: resolving which runtime a
//! project declares, one bounded argument-array invocation, normalization
//! of the returned status document into a versioned [`GateEvidence`]
//! record, and revision-bound freshness. Forge never re-decides blocking
//! policy: the sibling says passed/blocked, Forge reports that
//! attribution.
//!
//! Live sibling evidence (`tests/fixtures/gate/NOTES.md`, captured at
//! driftwatchdog `25811ed`) disproved the change design's assumption that
//! `gate --dry-run` has a JSON composition: the dry-run surface prints a
//! human-readable plan and exits before the JSON writer, so a rehearsal
//! is reported as a plan preview that is never persisted and never
//! journaled, while the evidence surface is the real
//! `gate --format json` run (whose only side effect is one `gate_runs`
//! row in the project's own `.driftwatch/` store — sibling state, never
//! Forge registry state).
//!
//! Classification discipline (shared with the policy plane): a parseable
//! gate status document is evidence whatever the exit code — the sibling
//! exits non-zero exactly when blocked, and a blocked document is a
//! recorded verdict, not an adapter failure. Unparseable output, a
//! timeout or a spawn failure is `Unavailable` and leaves any previously
//! persisted evidence byte-identical. Every captured string passes
//! [`crate::policy::redact_credentials`] and a character bound before it
//! reaches the evidence file, the journal or a display surface.

use std::ffi::{OsStr, OsString};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread::sleep;
use std::time::{Duration, Instant};

use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};

use crate::core::ForgeError;
use crate::policy::{first_binary_on_path, redact_credentials, DRIFTWATCH_BINARY_CANDIDATES};

/// Versioned contract for the persisted [`GateEvidence`] record. The
/// sibling's own gate document carries no contract field and is
/// discriminated by shape, exactly as the policy plane does.
pub const GATE_CONTRACT_VERSION: &str = "0.1.0";

/// Environment override naming the gate binary explicitly. Like every
/// other adapter override it runs exactly the named binary with no
/// fall-through to PATH probing.
pub const GATE_BIN_ENV: &str = "FORGE_GATE_BIN";

/// Gate evidence lives at `<project>/.forge/gate/<project-id>/evidence.json`
/// — one latest record per project; run history belongs to the operations
/// table, matching how the release and deploy planes split state.
pub const GATE_EVIDENCE_DIR: &str = ".forge/gate";
pub const GATE_EVIDENCE_FILE: &str = "evidence.json";

/// Gates can be long: the default bounded wait is deliberately far larger
/// than the policy-plane default, and `--timeout-secs` may raise it to
/// this hard maximum. Values outside `MIN..=MAX` refuse before any spawn.
pub const DEFAULT_GATE_TIMEOUT: Duration = Duration::from_secs(600);
pub const MIN_GATE_TIMEOUT_SECS: u64 = 1;
pub const MAX_GATE_TIMEOUT_SECS: u64 = 86_400;

/// Bounded stdout capture (mirrors the governance adapter bound) so a
/// chatty runtime cannot flood the evidence file or the journal.
pub const MAX_GATE_OUTPUT_BYTES: usize = 256 * 1024;

/// Character bound on any single captured note, check annotation or plan
/// line (mirrors the supervised-agent evidence bound).
pub const MAX_NOTE_CHARS: usize = 300;

/// Bound on plan-preview lines kept from a `gate --dry-run` rehearsal.
pub const MAX_PLAN_LINES: usize = 40;

/// The only runtime names with a packaged resolution path. A declaration
/// naming anything else is refused rather than silently probed.
pub const SUPPORTED_GATE_RUNTIMES: &[&str] = &["driftwatchdog"];

/// Aggregate verdict mapped from the sibling's gate status document.
/// `failed` and `unknown` keep a non-blocking failure and an
/// unclassifiable document honestly separated from `blocked`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GateAggregate {
    Passed,
    Blocked,
    Failed,
    Unknown,
}

impl GateAggregate {
    pub fn label(self) -> &'static str {
        match self {
            GateAggregate::Passed => "passed",
            GateAggregate::Blocked => "blocked",
            GateAggregate::Failed => "failed",
            GateAggregate::Unknown => "unknown",
        }
    }
}

/// One per-check row. Unknown or unexpected state strings become
/// `unresolved` — never `pass`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GateCheckState {
    Pass,
    Fail,
    Skip,
    NotApplicable,
    Unresolved,
}

impl GateCheckState {
    pub fn label(self) -> &'static str {
        match self {
            GateCheckState::Pass => "pass",
            GateCheckState::Fail => "fail",
            GateCheckState::Skip => "skip",
            GateCheckState::NotApplicable => "not_applicable",
            GateCheckState::Unresolved => "unresolved",
        }
    }

    fn from_raw(raw: &str) -> Self {
        match raw {
            "PASS" => GateCheckState::Pass,
            "FAIL" => GateCheckState::Fail,
            "SKIP" => GateCheckState::Skip,
            "NOT_APPLICABLE" => GateCheckState::NotApplicable,
            _ => GateCheckState::Unresolved,
        }
    }
}

/// One normalized check row from the sibling's `results[]`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateCheck {
    pub id: String,
    pub state: GateCheckState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// The versioned, revision-bound evidence record persisted per project.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateEvidence {
    pub contract: String,
    pub project_id: String,
    pub runtime: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_version: Option<String>,
    /// Git HEAD captured at invocation. Evidence without a binding can
    /// never be `fresh` — an unbound revision is never proven current.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manifest_digest: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rule_pack_version: Option<String>,
    pub aggregate: GateAggregate,
    #[serde(default)]
    pub checks: Vec<GateCheck>,
    pub observed_at: String,
    /// Rehearsal marker. A persisted record is always the product of a
    /// real run (`dry_run: false`); a `--dry-run` preview is reported in
    /// the plan-preview shape and never persisted.
    #[serde(default)]
    pub dry_run: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

impl GateEvidence {
    /// Short form of the bound revision for display and journal lines.
    pub fn short_revision(&self) -> Option<String> {
        self.revision
            .as_ref()
            .map(|sha| sha.chars().take(12).collect())
    }
}

/// Resolved gate runtime plus the human-readable attempt list that made
/// the resolution (or its failure) auditable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedRuntime {
    pub binary: PathBuf,
    pub name: String,
    pub attempts: Vec<String>,
}

/// How the persisted evidence compares against the current revision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GateFreshness {
    /// Evidence exists and its bound revision is the current HEAD.
    Fresh,
    /// Evidence exists but the revision moved (or the record was never
    /// bound / the current revision is unreadable): stale evidence can
    /// never satisfy a verification claim.
    Stale,
    /// No persisted evidence exists.
    Absent,
}

impl GateFreshness {
    pub fn label(self) -> &'static str {
        match self {
            GateFreshness::Fresh => "fresh",
            GateFreshness::Stale => "stale",
            GateFreshness::Absent => "absent",
        }
    }
}

/// Outcome of one bounded gate invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GateOutcome {
    /// A parseable status document mapped into evidence (real run), or
    /// a forward-compatible parseable document from a rehearsal
    /// (`dry_run: true`, never persisted).
    Evidence(GateEvidence),
    /// A `--dry-run` rehearsal whose runtime answered with the real
    /// sibling's human plan text. Nothing was executed, persisted or
    /// journaled; the plan lines are already redacted and bounded.
    PlanPreview {
        runtime: String,
        runtime_version: Option<String>,
        plan: Vec<String>,
    },
    /// The runtime could not be resolved, could not be spawned, timed
    /// out, or produced no parseable document. Prior evidence stays
    /// byte-identical.
    Unavailable { reason: String },
}

/// Bounded invocation configuration, resolved from the environment and
/// CLI input by the caller.
#[derive(Debug, Clone)]
pub struct GateConfig {
    /// `FORGE_GATE_BIN` (or an explicit test path). Runs exactly as
    /// named with no fall-through, like every other adapter override.
    pub binary: Option<OsString>,
    pub timeout: Duration,
}

impl Default for GateConfig {
    fn default() -> Self {
        Self::from_env()
    }
}

impl GateConfig {
    /// Resolve the override from the environment. Whitespace-only values
    /// are ignored so `FORGE_GATE_BIN=""` keeps declared/fallback
    /// resolution (same rule as the policy plane's override).
    pub fn from_env() -> Self {
        let binary =
            std::env::var_os(GATE_BIN_ENV).filter(|v| !v.to_string_lossy().trim().is_empty());
        GateConfig {
            binary,
            timeout: DEFAULT_GATE_TIMEOUT,
        }
    }
}

/// Validate a `--timeout-secs` override before anything runs.
pub fn parse_timeout_secs(raw: u64) -> Result<Duration, ForgeError> {
    if !(MIN_GATE_TIMEOUT_SECS..=MAX_GATE_TIMEOUT_SECS).contains(&raw) {
        return Err(ForgeError::GateInvalid {
            reason: format!(
                "--timeout-secs {raw} is outside the bounded range {MIN_GATE_TIMEOUT_SECS}..={MAX_GATE_TIMEOUT_SECS}"
            ),
        });
    }
    Ok(Duration::from_secs(raw))
}

/// Current RFC 3339 timestamp (second precision, UTC), shared with the
/// checker plane's document convention.
pub fn now_rfc3339() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}

/// Redact and char-bound one captured string before it is stored or
/// displayed (no project context available). Truncation is marked so a
/// bound never hides content silently.
pub fn bound_note(text: &str) -> String {
    let redacted = redact_credentials(text);
    if redacted.len() <= MAX_NOTE_CHARS {
        return redacted;
    }
    let mut end = MAX_NOTE_CHARS;
    while !redacted.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…[truncated]", &redacted[..end])
}

/// Redact, bound and replace host paths of the assessed project with
/// `<project>` (same discipline as the policy plane) so a captured
/// document never exports the machine layout.
fn capture_note(text: &str, dir: &Path) -> String {
    let mut scrubbed = redact_credentials(text);
    let display = dir.display().to_string();
    if !display.is_empty() {
        scrubbed = scrubbed.replace(&display, "<project>");
    }
    if let Ok(canonical) = dir.canonicalize() {
        let c = canonical.display().to_string();
        if c != display {
            scrubbed = scrubbed.replace(&c, "<project>");
        }
    }
    if scrubbed.len() <= MAX_NOTE_CHARS {
        return scrubbed;
    }
    let mut end = MAX_NOTE_CHARS;
    while !scrubbed.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…[truncated]", &scrubbed[..end])
}

/// Read `verification.gate_runtime` from the project's `.project.json`
/// declaration. The declaration is untrusted input: a missing file, a
/// missing key or a non-string value is simply "undeclared"; a file that
/// exists but cannot be parsed is reported so the resolution list stays
/// honest about why the declaration was not taken.
pub fn declared_gate_runtime(dir: &Path) -> DeclaredRuntime {
    let path = dir.join(".project.json");
    if !path.is_file() {
        return DeclaredRuntime::Undeclared;
    }
    let Ok(text) = fs::read_to_string(&path) else {
        return DeclaredRuntime::Unreadable(path.display().to_string());
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
        return DeclaredRuntime::Unreadable(path.display().to_string());
    };
    match value
        .get("verification")
        .and_then(|v| v.get("gate_runtime"))
        .and_then(|v| v.as_str())
    {
        Some(name) if !name.trim().is_empty() => DeclaredRuntime::Declared(name.trim().to_string()),
        _ => DeclaredRuntime::Undeclared,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeclaredRuntime {
    /// No `.project.json`, no key, or an empty value.
    Undeclared,
    /// A declaration exists but the document could not be read as JSON.
    Unreadable(String),
    /// A non-empty declaration string.
    Declared(String),
}

/// Read the Workspace Governance declaration's project `id` (sibling
/// schema_version 1). Used only to name a gate journal row and evidence
/// directory for a project that is not in the local registry; `None`
/// when there is no readable declaration.
pub fn declared_project_id(dir: &Path) -> Option<String> {
    let text = fs::read_to_string(dir.join(".project.json")).ok()?;
    let value = serde_json::from_str::<serde_json::Value>(&text).ok()?;
    value
        .get("id")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

/// Ordered runtime resolution (design chain):
///
/// 1. `FORGE_GATE_BIN` names an explicit binary, run exactly as given.
/// 2. `.project.json` `verification.gate_runtime` maps a declared name to
///    its probe vocabulary; an unknown declared name is refused rather
///    than silently probed.
/// 3. With no declaration, the ordered policy-binary probe
///    (`driftwatchdog` → `driftwatch`) is the fallback.
///
/// Every branch records its attempts so an unavailable result names what
/// was tried. A resolved-but-nonexistent explicit override is reported
/// like the policy plane does (the spawn failure or the missing file is
/// named, never replaced by probing).
pub fn resolve_runtime(dir: &Path, config: &GateConfig) -> Result<ResolvedRuntime, String> {
    let mut attempts: Vec<String> = Vec::new();
    if let Some(binary) = &config.binary {
        attempts.push(format!("{GATE_BIN_ENV}={}", Path::new(binary).display()));
        return Ok(ResolvedRuntime {
            binary: PathBuf::from(binary),
            name: binary_name(binary),
            attempts,
        });
    }
    match declared_gate_runtime(dir) {
        DeclaredRuntime::Declared(name) => {
            if !SUPPORTED_GATE_RUNTIMES.contains(&name.as_str()) {
                attempts.push(format!(
                    "declared gate_runtime `{name}` (no resolution path)"
                ));
                return Err(format!(
                    "project declares verification.gate_runtime `{name}`, which Forge cannot \
                     resolve; supported runtimes: {}; attempts: {}",
                    SUPPORTED_GATE_RUNTIMES.join(", "),
                    attempts.join("; ")
                ));
            }
            attempts.push(format!("declared gate_runtime `{name}`"));
            resolve_by_probe(&mut attempts, &format!("PATH (declared `{name}`)"))
        }
        DeclaredRuntime::Unreadable(path) => {
            attempts.push(format!("declared runtime unreadable at `{path}`"));
            resolve_by_probe(&mut attempts, "PATH (declaration unreadable)")
        }
        DeclaredRuntime::Undeclared => {
            attempts.push(format!("{GATE_BIN_ENV} unset"));
            attempts.push("no verification.gate_runtime declared".to_string());
            resolve_by_probe(&mut attempts, "PATH (fallback probe)")
        }
    }
}

fn resolve_by_probe(attempts: &mut Vec<String>, label: &str) -> Result<ResolvedRuntime, String> {
    let path_env = std::env::var_os("PATH").unwrap_or_default();
    match first_binary_on_path(&path_env, DRIFTWATCH_BINARY_CANDIDATES) {
        Some(found) => {
            attempts.push(format!("{label}: {}", found.display()));
            let name = found
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("driftwatch")
                .to_string();
            Ok(ResolvedRuntime {
                binary: found,
                name,
                attempts: attempts.clone(),
            })
        }
        None => {
            attempts.push(format!(
                "{label}: none of '{}'",
                DRIFTWATCH_BINARY_CANDIDATES.join("', '")
            ));
            Err(format!(
                "no gate runtime resolvable: tried {}; the gate plane stays unverified \
                 (never a pass) and prior evidence is untouched",
                attempts.join("; ")
            ))
        }
    }
}

fn binary_name(binary: &OsStr) -> String {
    Path::new(binary)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("gate-runtime")
        .to_string()
}

/// Best-effort capture of the current git HEAD. `None` when the project
/// is not a git working tree: evidence without a binding (or read without
/// one) is never reported as fresh.
pub fn capture_revision(dir: &Path) -> Option<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .arg("rev-parse")
        .arg("HEAD")
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let sha = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if sha.is_empty() {
        None
    } else {
        Some(sha)
    }
}

/// True when the project is gate-managed by the sibling (a resolved
/// `gate.toml` or `.ai-gate/gate.yaml` manifest). One source of truth
/// with the policy plane's surface selection.
pub fn is_gate_managed(dir: &Path) -> bool {
    dir.join("gate.toml").is_file() || dir.join(".ai-gate").join("gate.yaml").is_file()
}

/// Run the declared gate runtime for `dir`. `dry_run` rehearses with the
/// real `gate --dry-run` surface (the side-effect-free plan preview).
/// This function never writes anything; persistence and journaling are
/// the caller's decision based on the returned outcome.
pub fn run_gate(dir: &Path, project_id: &str, config: &GateConfig, dry_run: bool) -> GateOutcome {
    if !dir.is_dir() {
        return GateOutcome::Unavailable {
            reason: format!("project path '{}' is not a directory", dir.display()),
        };
    }
    let resolved = match resolve_runtime(dir, config) {
        Ok(resolved) => resolved,
        Err(reason) => return GateOutcome::Unavailable { reason },
    };
    let runtime_version = probe_version(&resolved.binary, config.timeout);
    let mut args: Vec<&OsStr> = vec![OsStr::new("gate")];
    if dry_run {
        args.push(OsStr::new("--dry-run"));
    }
    args.push(OsStr::new("--format"));
    args.push(OsStr::new("json"));
    let mut command = Command::new(&resolved.binary);
    command.args(&args).current_dir(dir).stdin(Stdio::null());
    let output = match run_bounded(&mut command, config.timeout) {
        Ok(output) => output,
        Err(reason) => {
            return GateOutcome::Unavailable {
                reason: format!(
                    "gate runtime `{}` invocation failed: {reason}",
                    resolved.name
                ),
            };
        }
    };
    let raw = match String::from_utf8(output.stdout) {
        Ok(text) => text,
        Err(_) => {
            return GateOutcome::Unavailable {
                reason: "gate runtime stdout was not valid UTF-8".to_string(),
            };
        }
    };
    let status_note = || {
        format!(
            "gate runtime exited with status {}: {}",
            output.status,
            capture_note(&stderr_summary(&output.stderr), dir)
        )
    };
    let document: Option<serde_json::Value> = serde_json::from_str(&raw).ok();
    if let Some(value) = document {
        if let Some(mapped) = parse_gate_document(&value, dir) {
            let revision = capture_revision(dir);
            let aggregate =
                classify_aggregate(&mapped.status, mapped.blocked, output.status.success());
            let evidence = GateEvidence {
                contract: GATE_CONTRACT_VERSION.to_string(),
                project_id: project_id.to_string(),
                runtime: resolved.name.clone(),
                runtime_version: runtime_version.clone(),
                revision,
                manifest_digest: mapped.manifest_digest.map(|d| capture_note(&d, dir)),
                rule_pack_version: mapped.rule_pack_version.map(|r| capture_note(&r, dir)),
                aggregate,
                checks: mapped.checks,
                observed_at: now_rfc3339(),
                dry_run,
                note: mapped.note,
            };
            return GateOutcome::Evidence(evidence);
        }
    }
    if dry_run {
        // The real sibling's dry-run surface: a human plan with exit 0
        // and no JSON composition (verified at driftwatchdog `25811ed`).
        // It is a genuine side-effect-free preview, not evidence.
        if output.status.success() {
            let plan = bounded_plan_lines(&raw, dir);
            if !plan.is_empty() {
                return GateOutcome::PlanPreview {
                    runtime: resolved.name,
                    runtime_version,
                    plan,
                };
            }
        }
        return GateOutcome::Unavailable {
            reason: if status_success(&output.status) {
                format!(
                    "gate runtime `{}` produced no parseable dry-run document and no plan \
                     preview; never a pass",
                    resolved.name
                )
            } else {
                status_note()
            },
        };
    }
    // A real run without a parseable status document is an unavailable
    // runtime answer (including the sibling's "nothing to gate" text in
    // projects without a gate manifest), never evidence and never a pass.
    // The runtime's own words stay in the reason, bounded and redacted.
    let own_words = raw
        .lines()
        .find(|line| !line.trim().is_empty())
        .map(|line| capture_note(line, dir))
        .unwrap_or_else(|| "no stdout output".to_string());
    if output.status.success() {
        return GateOutcome::Unavailable {
            reason: format!(
                "gate runtime `{}` exited successfully without a parseable gate status \
                 document: {own_words}",
                resolved.name
            ),
        };
    }
    GateOutcome::Unavailable {
        reason: format!("{}; output: {own_words}", status_note()),
    }
}

fn status_success(status: &std::process::ExitStatus) -> bool {
    status.success()
}

/// The sibling's gate document has no `contract` field; it is
/// discriminated by shape (`blocked` + `results`), the same rule the
/// policy plane applies.
struct MappedDocument {
    status: String,
    blocked: bool,
    checks: Vec<GateCheck>,
    manifest_digest: Option<String>,
    rule_pack_version: Option<String>,
    note: Option<String>,
}

fn parse_gate_document(value: &serde_json::Value, dir: &Path) -> Option<MappedDocument> {
    let blocked = value.get("blocked")?.as_bool()?;
    let results = value.get("results")?.as_array()?;
    let status = value
        .get("status")
        .and_then(|s| s.as_str())
        .unwrap_or("")
        .to_string();
    let checks = results
        .iter()
        .enumerate()
        .map(|(index, row)| {
            let id = row
                .get("gate_id")
                .and_then(|g| g.as_str())
                .filter(|s| !s.trim().is_empty())
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("unnamed-{index}"));
            let state =
                GateCheckState::from_raw(row.get("status").and_then(|s| s.as_str()).unwrap_or(""));
            let mut parts: Vec<String> = Vec::new();
            if let Some(diagnostic) = row.get("diagnostic").and_then(|d| d.as_str()) {
                if !diagnostic.trim().is_empty() {
                    parts.push(format!("diagnostic: {}", diagnostic.trim()));
                }
            }
            if let Some(remediation) = row.get("remediation").and_then(|d| d.as_str()) {
                if !remediation.trim().is_empty() {
                    parts.push(format!("remediation: {}", remediation.trim()));
                }
            }
            let missing: Vec<&str> = row
                .get("missing_evidence")
                .and_then(|m| m.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str())
                        .filter(|s| !s.trim().is_empty())
                        .collect()
                })
                .unwrap_or_default();
            if !missing.is_empty() {
                parts.push(format!("missing={}", missing.join(",")));
            }
            if state == GateCheckState::Unresolved {
                parts.insert(
                    0,
                    format!(
                        "state `{}` is not in the known vocabulary and never counts as pass",
                        row.get("status")
                            .and_then(|s| s.as_str())
                            .unwrap_or("missing")
                    ),
                );
            }
            GateCheck {
                id,
                state,
                note: if parts.is_empty() {
                    None
                } else {
                    Some(capture_note(&parts.join("; "), dir))
                },
            }
        })
        .collect();
    Some(MappedDocument {
        status,
        blocked,
        checks,
        manifest_digest: value
            .get("manifest_digest")
            .and_then(|d| d.as_str())
            .map(str::to_string),
        rule_pack_version: value.get("rule_pack_version").map(|r| match r {
            serde_json::Value::String(s) => s.clone(),
            other => other.to_string(),
        }),
        note: None,
    })
}

/// Map the document's top-level outcome into the aggregate vocabulary.
/// A parseable document wins over the exit code — the sibling exits
/// non-zero exactly when blocked. The real runtime (captured at
/// `25811ed`) reports three top-level statuses: `PASS`, `FAIL` and
/// `REVIEW_REQUIRED` (pending review with `review_required_blocks`),
/// and `blocked` stays the authoritative blocking bit. The one honest
/// exception to "document wins": a `PASS` claim riding a failure exit
/// is a contradiction and downgrades to `unknown`, never a fabricated
/// pass (same rule the deploy executor applies to contradictory
/// envelopes).
fn classify_aggregate(status: &str, blocked: bool, exit_success: bool) -> GateAggregate {
    if blocked {
        return GateAggregate::Blocked;
    }
    match status {
        "PASS" if exit_success => GateAggregate::Passed,
        "PASS" => GateAggregate::Unknown,
        "FAIL" => GateAggregate::Failed,
        // Pending reviews with a non-blocking policy are not a passed
        // gate: never a pass, never a hard failure either.
        "REVIEW_REQUIRED" => GateAggregate::Unknown,
        _ => GateAggregate::Unknown,
    }
}

fn stderr_summary(stderr: &[u8]) -> String {
    let text = String::from_utf8_lossy(stderr);
    let first = text
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("no stderr output")
        .trim();
    if first.is_empty() {
        "no stderr output".to_string()
    } else {
        first.to_string()
    }
}

fn bounded_plan_lines(raw: &str, dir: &Path) -> Vec<String> {
    raw.lines()
        .take(MAX_PLAN_LINES)
        .filter(|line| !line.trim().is_empty())
        .map(|line| capture_note(line, dir))
        .collect()
}

/// Bounded child process execution with full capture. Spawns with an
/// argument array (never a shell) and null stdin so an interactive
/// runtime can never hijack the operator terminal.
fn run_bounded(command: &mut Command, timeout: Duration) -> Result<std::process::Output, String> {
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = command
        .spawn()
        .map_err(|err| format!("binary not found or not executable: {err}"))?;
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => {
                let mut stdout = Vec::new();
                let mut stderr = Vec::new();
                if let Some(out) = child.stdout.as_mut() {
                    out.take((MAX_GATE_OUTPUT_BYTES + 1) as u64)
                        .read_to_end(&mut stdout)
                        .map_err(|err| format!("cannot read stdout: {err}"))?;
                }
                if let Some(err) = child.stderr.as_mut() {
                    err.take(64 * 1024 + 1)
                        .read_to_end(&mut stderr)
                        .map_err(|err| format!("cannot read stderr: {err}"))?;
                }
                if stdout.len() > MAX_GATE_OUTPUT_BYTES {
                    let _ = child.wait();
                    return Err(format!(
                        "gate runtime exceeded the {MAX_GATE_OUTPUT_BYTES}-byte output bound"
                    ));
                }
                let status = child
                    .wait()
                    .map_err(|err| format!("cannot collect exit status: {err}"))?;
                return Ok(std::process::Output {
                    status,
                    stdout,
                    stderr,
                });
            }
            Ok(None) => {
                if start.elapsed() > timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!(
                        "timeout after {}s (bounded wait; the gate never ran to completion)",
                        timeout.as_secs()
                    ));
                }
                sleep(Duration::from_millis(25));
            }
            Err(err) => return Err(format!("child wait failed: {err}")),
        }
    }
}

fn probe_version(binary: &Path, timeout: Duration) -> Option<String> {
    // The version probe is a lifecycle check, not the gate run: bound it
    // far below the gate timeout so an unresponsive binary cannot stall
    // the preview path.
    let probe_timeout = timeout.min(Duration::from_secs(15));
    let mut command = Command::new(binary);
    command.arg("--version").stdin(Stdio::null());
    let out = run_bounded(&mut command, probe_timeout).ok()?;
    if !out.status.success() {
        return None;
    }
    let line = String::from_utf8_lossy(&out.stdout)
        .lines()
        .next()
        .unwrap_or("")
        .trim()
        .to_string();
    if line.is_empty() {
        None
    } else {
        Some(bound_note(&line))
    }
}

/// Path of a project's evidence file, relative to the project directory:
/// `.forge/gate/<project-id>/evidence.json`.
pub fn evidence_path(project_id: &str) -> PathBuf {
    PathBuf::from(GATE_EVIDENCE_DIR)
        .join(project_id)
        .join(GATE_EVIDENCE_FILE)
}

/// Persist the latest evidence record atomically (tmp file + rename), so
/// an interrupted write can never corrupt the previous record. Only real
/// (non-dry-run) evidence is ever persisted.
pub fn save_evidence(dir: &Path, evidence: &GateEvidence) -> Result<PathBuf, ForgeError> {
    if evidence.dry_run {
        return Err(ForgeError::GateInvalid {
            reason: "a dry-run rehearsal is never persisted".to_string(),
        });
    }
    // The project id names a directory inside `.forge/gate/`; keep the
    // path confined even though callers pass already-validated ids.
    crate::core::validate_project_id(&evidence.project_id).map_err(|reason| {
        ForgeError::GateInvalid {
            reason: format!("invalid project id for gate evidence: {reason}"),
        }
    })?;
    let relative = evidence_path(&evidence.project_id);
    let path = dir.join(&relative);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| ForgeError::GateInvalid {
            reason: format!(
                "cannot create gate evidence directory {}: {err}",
                parent.display()
            ),
        })?;
    }
    let bytes = serde_json::to_vec_pretty(evidence).map_err(|err| ForgeError::GateInvalid {
        reason: format!("cannot serialize gate evidence: {err}"),
    })?;
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, &bytes).map_err(|err| ForgeError::GateInvalid {
        reason: format!("cannot write gate evidence tmp {}: {err}", tmp.display()),
    })?;
    fs::rename(&tmp, &path).map_err(|err| ForgeError::GateInvalid {
        reason: format!("cannot persist gate evidence {}: {err}", path.display()),
    })?;
    Ok(relative)
}

/// Load the newest persisted evidence record for `dir`, scanning
/// `.forge/gate/*/evidence.json` (the project directory owns exactly one
/// identity; newest observed_at wins, id breaks ties deterministically).
/// A file that exists but cannot be parsed is an error rather than an
/// invented absence: absent-shaped readings of corrupt evidence would
/// silently erase the recorded history.
pub fn load_latest_evidence(dir: &Path) -> Result<Option<GateEvidence>, ForgeError> {
    let base = dir.join(GATE_EVIDENCE_DIR);
    if !base.is_dir() {
        return Ok(None);
    }
    let mut candidates: Vec<PathBuf> = Vec::new();
    let entries = fs::read_dir(&base).map_err(|err| ForgeError::GateInvalid {
        reason: format!("cannot read {}: {err}", base.display()),
    })?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let file = path.join(GATE_EVIDENCE_FILE);
            if file.is_file() {
                candidates.push(file);
            }
        }
    }
    let mut best: Option<GateEvidence> = None;
    for path in candidates {
        let evidence = read_evidence_file(&path)?;
        best = Some(match best {
            None => evidence,
            Some(current) => {
                if (evidence.observed_at.as_str(), evidence.project_id.as_str())
                    > (current.observed_at.as_str(), current.project_id.as_str())
                {
                    evidence
                } else {
                    current
                }
            }
        });
    }
    Ok(best)
}

fn read_evidence_file(path: &Path) -> Result<GateEvidence, ForgeError> {
    let bytes = fs::read(path).map_err(|err| ForgeError::GateInvalid {
        reason: format!("cannot read gate evidence {}: {err}", path.display()),
    })?;
    serde_json::from_slice(&bytes).map_err(|err| ForgeError::GateInvalid {
        reason: format!(
            "gate evidence at {} is not a valid GateEvidence record: {err}",
            path.display()
        ),
    })
}

/// Classify persisted evidence against the current working-tree revision.
/// Evidence without a revision binding, or read against an unreadable
/// HEAD, is stale: neither can be proven current.
pub fn evidence_freshness(
    evidence: &GateEvidence,
    current_revision: Option<&str>,
) -> GateFreshness {
    match (&evidence.revision, current_revision) {
        (Some(bound), Some(current)) if bound == current => match evidence.aggregate {
            GateAggregate::Passed | GateAggregate::Blocked | GateAggregate::Failed => {
                GateFreshness::Fresh
            }
            // An unclassifiable aggregate is never "current verification".
            GateAggregate::Unknown => GateFreshness::Stale,
        },
        _ => GateFreshness::Stale,
    }
}

/// One-line provenance summary used in journal rows and human output.
pub fn evidence_summary(evidence: &GateEvidence) -> String {
    let version = evidence
        .runtime_version
        .as_deref()
        .unwrap_or("unknown-version");
    let revision = evidence
        .short_revision()
        .unwrap_or_else(|| "unbound".to_string());
    bound_note(&format!(
        "aggregate={} runtime={}@{} revision={revision} observed_at={}",
        evidence.aggregate.label(),
        evidence.runtime,
        version,
        evidence.observed_at
    ))
}

/// Journal verdict for one outcome: `done` for a passed aggregate,
/// `blocked` for a blocked one, `failed` for every other attempted run
/// (failed, unknown, unavailable, timeout). `partial` stays reserved.
pub fn journal_verdict(outcome: &GateOutcome) -> &'static str {
    match outcome {
        GateOutcome::Evidence(evidence) => match evidence.aggregate {
            GateAggregate::Passed => "done",
            GateAggregate::Blocked => "blocked",
            _ => "failed",
        },
        GateOutcome::PlanPreview { .. } => "done",
        GateOutcome::Unavailable { .. } => "failed",
    }
}

/// Human rendering of a persisted evidence record. The JSON envelope
/// carries the identical fields, so no surface can report a different
/// aggregate, revision, runtime or timestamp than another.
pub fn render_evidence_human(
    evidence: &GateEvidence,
    freshness: Option<GateFreshness>,
    persisted: Option<&str>,
) -> String {
    let mut lines = Vec::new();
    lines.push(format!("project: {}", evidence.project_id));
    if let Some(freshness) = freshness {
        lines.push(format!("freshness: {}", freshness.label()));
    }
    lines.push(format!("aggregate: {}", evidence.aggregate.label()));
    lines.push(format!(
        "runtime: {} ({})",
        evidence.runtime,
        evidence
            .runtime_version
            .as_deref()
            .unwrap_or("unknown-version")
    ));
    lines.push(format!(
        "revision: {}",
        evidence
            .short_revision()
            .unwrap_or_else(|| "unbound".to_string())
    ));
    lines.push(format!("observed_at: {}", evidence.observed_at));
    if let Some(digest) = &evidence.manifest_digest {
        lines.push(format!("manifest_digest: {digest}"));
    }
    if let Some(pack) = &evidence.rule_pack_version {
        lines.push(format!("rule_pack_version: {pack}"));
    }
    if let Some(note) = &evidence.note {
        lines.push(format!("note: {note}"));
    }
    if !evidence.checks.is_empty() {
        lines.push("checks:".to_string());
        for check in &evidence.checks {
            let mut row = format!("  - {} {}", check.id, check.state.label());
            if let Some(note) = &check.note {
                row.push_str(&format!(": {note}"));
            }
            lines.push(row);
        }
    }
    if let Some(path) = persisted {
        lines.push(format!("persisted: {path}"));
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    use tempfile::TempDir;

    fn project_dir() -> TempDir {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("forge.yaml"), "schema: 1\n").unwrap();
        tmp
    }

    fn executable(path: &Path, script: &str) {
        fs::write(path, script).unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
    }

    fn write_fixture_script(tmp: &TempDir, name: &str, body: &str) -> PathBuf {
        let path = tmp.path().join(name);
        executable(
            &path,
            &format!("#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo 'driftwatch 0.1.0'; exit 0; fi\n{body}\n"),
        );
        path
    }

    fn fixture_value(name: &str) -> serde_json::Value {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/gate")
            .join(name);
        serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
    }

    #[test]
    fn explicit_override_wins_and_never_falls_through() {
        let tmp = project_dir();
        let config = GateConfig {
            binary: Some(OsString::from("/definitely/not/a/gate-runtime-xyz")),
            timeout: Duration::from_secs(2),
        };
        let resolved = resolve_runtime(tmp.path(), &config).expect("override resolves as named");
        assert_eq!(resolved.name, "gate-runtime-xyz");
        assert!(resolved.attempts[0].contains(GATE_BIN_ENV));
        // A dead override surfaces the spawn failure honestly.
        let outcome = run_gate(tmp.path(), "demo", &config, false);
        assert!(matches!(outcome, GateOutcome::Unavailable { .. }));
    }

    #[test]
    fn declared_unknown_runtime_is_refused_by_name() {
        let tmp = project_dir();
        fs::write(
            tmp.path().join(".project.json"),
            "{\"schema_version\":1,\"id\":\"x\",\"verification\":{\"gate_runtime\":\"jenkins-gate\"}}",
        )
        .unwrap();
        let config = GateConfig {
            binary: None,
            timeout: Duration::from_secs(2),
        };
        let err = resolve_runtime(tmp.path(), &config).unwrap_err();
        assert!(err.contains("jenkins-gate"), "{err}");
        assert!(err.contains("driftwatchdog"), "{err}");
    }

    #[test]
    fn declared_known_runtime_and_fallback_use_the_ordered_probe() {
        let tmp = project_dir();
        fs::write(
            tmp.path().join(".project.json"),
            "{\"schema_version\":1,\"id\":\"x\",\"verification\":{\"gate_runtime\":\"driftwatchdog\"}}",
        )
        .unwrap();
        // With an empty PATH nothing resolves, and the attempts say so.
        let config = GateConfig {
            binary: None,
            timeout: Duration::from_secs(2),
        };
        // (Runs with the ambient PATH; a resolution is environment
        // dependent, so assert only the refusal text shape.)
        match resolve_runtime(tmp.path(), &config) {
            Ok(resolved) => assert!(resolved
                .attempts
                .iter()
                .any(|a| a.contains("declared gate_runtime `driftwatchdog`"))),
            Err(reason) => assert!(reason.contains("no gate runtime resolvable"), "{reason}"),
        }
    }

    #[test]
    fn unreadable_declaration_still_probes_with_honest_attempts() {
        let tmp = project_dir();
        fs::write(tmp.path().join(".project.json"), "not json at all").unwrap();
        assert!(matches!(
            declared_gate_runtime(tmp.path()),
            DeclaredRuntime::Unreadable(_)
        ));
    }

    #[test]
    fn passing_document_maps_to_fresh_able_evidence() {
        let value = fixture_value("gate-status-pass.json");
        let doc = parse_gate_document(&value, Path::new(".")).expect("fixture shape");
        let aggregate = classify_aggregate(&doc.status, doc.blocked, true);
        assert_eq!(aggregate, GateAggregate::Passed);
        assert_eq!(doc.checks.len(), 1);
        assert_eq!(doc.checks[0].id, "docs");
        assert_eq!(doc.checks[0].state, GateCheckState::Pass);
        assert_eq!(
            doc.manifest_digest.as_deref(),
            Some("sha256:dc1fb6a3db59efd5")
        );
        assert_eq!(doc.rule_pack_version.as_deref(), Some("local"));
    }

    #[test]
    fn blocked_document_maps_with_real_evidence_codes() {
        let value = fixture_value("gate-status-blocked.json");
        let doc = parse_gate_document(&value, Path::new(".")).expect("fixture shape");
        let aggregate = classify_aggregate(&doc.status, doc.blocked, false);
        assert_eq!(aggregate, GateAggregate::Blocked);
        assert_eq!(doc.checks[0].state, GateCheckState::Fail);
        let note = doc.checks[0].note.as_deref().unwrap_or("");
        assert!(note.contains("diagnostic: exit 1"), "{note}");
        assert!(
            note.contains("remediation: Fix the failing command"),
            "{note}"
        );
    }

    #[test]
    fn review_required_never_counts_as_pass() {
        let value = serde_json::json!({
            "blocked": true, "status": "FAIL",
            "results": [{"gate_id": "ux-review", "status": "REVIEW_REQUIRED", "severity": "warning"}]
        });
        let doc = parse_gate_document(&value, Path::new(".")).expect("shape");
        assert_eq!(doc.checks[0].state, GateCheckState::Unresolved);
        assert_eq!(
            classify_aggregate("FAIL", true, false),
            GateAggregate::Blocked
        );
    }

    #[test]
    fn real_review_required_fixture_blocks_without_inventing_pass() {
        // Verbatim sibling capture: top-level status REVIEW_REQUIRED,
        // blocked=true, and a NOT_APPLICABLE row for the pending review.
        let value = fixture_value("gate-status-review-required.json");
        let doc = parse_gate_document(&value, Path::new(".")).expect("fixture shape");
        assert_eq!(
            classify_aggregate(&doc.status, doc.blocked, false),
            GateAggregate::Blocked
        );
        let by_id: std::collections::BTreeMap<&str, &GateCheck> =
            doc.checks.iter().map(|c| (c.id.as_str(), c)).collect();
        assert_eq!(by_id["docs"].state, GateCheckState::Pass);
        assert_eq!(by_id["ux-review"].state, GateCheckState::NotApplicable);
    }

    #[test]
    fn non_blocking_review_required_is_unknown_never_passed() {
        // review_required_blocks=false keeps exit 0; a pending review is
        // still not a passed gate, and neither is any future status.
        assert_eq!(
            classify_aggregate("REVIEW_REQUIRED", false, true),
            GateAggregate::Unknown
        );
        assert_eq!(
            classify_aggregate("SOMETHING_NEW", false, true),
            GateAggregate::Unknown
        );
    }

    #[test]
    fn real_not_applicable_fixture_passes_with_explicit_na_row() {
        // Verbatim sibling capture: status PASS, blocked=false, one PASS
        // row and one NOT_APPLICABLE row for an opted-out concern.
        let value = fixture_value("gate-status-not-applicable.json");
        let doc = parse_gate_document(&value, Path::new(".")).expect("fixture shape");
        assert_eq!(
            classify_aggregate(&doc.status, doc.blocked, true),
            GateAggregate::Passed
        );
        let by_id: std::collections::BTreeMap<&str, &GateCheck> =
            doc.checks.iter().map(|c| (c.id.as_str(), c)).collect();
        assert_eq!(by_id["build"].state, GateCheckState::Pass);
        assert_eq!(by_id["deploy"].state, GateCheckState::NotApplicable);
        assert!(by_id["deploy"]
            .note
            .as_deref()
            .unwrap_or("")
            .contains("missing=deploy:command"));
    }

    #[test]
    fn not_applicable_and_unknown_states_are_distinct_and_never_pass() {
        let value = serde_json::json!({
            "blocked": false, "status": "PASS",
            "results": [
                {"gate_id": "deploy", "status": "NOT_APPLICABLE"},
                {"gate_id": "future", "status": "WEIRD_FUTURE_STATE"}
            ]
        });
        let doc = parse_gate_document(&value, Path::new(".")).expect("shape");
        assert_eq!(doc.checks[0].state, GateCheckState::NotApplicable);
        assert_eq!(doc.checks[1].state, GateCheckState::Unresolved);
        assert!(doc.checks[1]
            .note
            .as_deref()
            .unwrap_or("")
            .contains("never counts as pass"));
    }

    #[test]
    fn passing_document_with_nonzero_exit_downgrades_to_unknown() {
        // A parseable document wins over the exit code for *classification
        // as evidence*, but a PASS claim riding a failure exit is a
        // contradiction and never produces a pass verdict.
        assert_eq!(
            classify_aggregate("PASS", false, false),
            GateAggregate::Unknown
        );
        assert_eq!(
            classify_aggregate("PASS", false, true),
            GateAggregate::Passed
        );
        assert_eq!(
            classify_aggregate("FAIL", false, true),
            GateAggregate::Failed
        );
    }

    #[test]
    fn real_run_executes_gate_surface_and_records_evidence() {
        let tmp = project_dir();
        let script = write_fixture_script(
            &tmp,
            "gate-ok.sh",
            "cat <<'EOF'\n{\"status\":\"PASS\",\"blocked\":false,\"failures\":[],\"pending_reviews\":[],\"not_applicable\":[],\"manifest_digest\":\"sha256:abc\",\"rule_pack_version\":\"local\",\"results\":[{\"gate_id\":\"build\",\"status\":\"PASS\"}]}\nEOF\nexit 0\n",
        );
        let config = GateConfig {
            binary: Some(script.into_os_string()),
            timeout: Duration::from_secs(10),
        };
        let outcome = run_gate(tmp.path(), "demo", &config, false);
        let GateOutcome::Evidence(evidence) = outcome else {
            panic!("parseable document must map to evidence: {outcome:?}");
        };
        assert_eq!(evidence.aggregate, GateAggregate::Passed);
        assert_eq!(evidence.project_id, "demo");
        assert_eq!(evidence.runtime, "gate-ok");
        assert_eq!(
            evidence.runtime_version.as_deref(),
            Some("driftwatch 0.1.0")
        );
        assert!(!evidence.dry_run);
    }

    #[test]
    fn dry_run_reports_plan_preview_without_evidence() {
        let tmp = project_dir();
        let script = write_fixture_script(
            &tmp,
            "gate-plan.sh",
            "printf 'gate plan (dry-run; nothing was executed)\\ncontract version: 1 | profile: minimal\\nchecks:\\n  - docs [required] via project-runtime\\n'\nexit 0\n",
        );
        let config = GateConfig {
            binary: Some(script.into_os_string()),
            timeout: Duration::from_secs(10),
        };
        let outcome = run_gate(tmp.path(), "demo", &config, true);
        let GateOutcome::PlanPreview { plan, .. } = outcome else {
            panic!("plan text must map to a preview: {outcome:?}");
        };
        assert!(plan[0].contains("nothing was executed"));
        assert_eq!(plan.len(), 4);
    }

    #[test]
    fn unparseable_real_run_is_unavailable_and_names_status() {
        let tmp = project_dir();
        let script = write_fixture_script(
            &tmp,
            "gate-text.sh",
            "echo 'driftwatch gate: no gate.toml or .ai-gate/gate.yaml; nothing to gate.'\nexit 0\n",
        );
        let config = GateConfig {
            binary: Some(script.into_os_string()),
            timeout: Duration::from_secs(10),
        };
        let outcome = run_gate(tmp.path(), "demo", &config, false);
        let GateOutcome::Unavailable { reason } = outcome else {
            panic!("no document is never evidence: {outcome:?}");
        };
        // The runtime answered with exit 0 text: the note is honest about
        // the missing document.
        assert!(
            reason.contains("nothing to gate") || reason.contains("no parseable"),
            "{reason}"
        );
    }

    #[test]
    fn timeout_is_bounded_and_names_the_wait() {
        let tmp = project_dir();
        let script = write_fixture_script(&tmp, "gate-hang.sh", "sleep 5\nexit 0\n");
        let config = GateConfig {
            binary: Some(script.into_os_string()),
            timeout: Duration::from_millis(300),
        };
        let outcome = run_gate(tmp.path(), "demo", &config, false);
        let GateOutcome::Unavailable { reason } = outcome else {
            panic!("hang is unavailable: {outcome:?}");
        };
        assert!(reason.contains("timeout"), "{reason}");
    }

    #[test]
    fn secrets_and_bounds_apply_to_every_captured_string() {
        let noisy = format!(
            "diagnostic: exit 1 with token=ghp_{} for CI at /home/operator/secret/project",
            "A".repeat(400)
        );
        let bounded = bound_note(&noisy);
        assert!(
            bounded.contains("[REDACTED]") || !bounded.contains("ghp_"),
            "{bounded}"
        );
        assert!(
            bounded.chars().count() <= MAX_NOTE_CHARS + 32,
            "{}",
            bounded.len()
        );
    }

    #[test]
    fn evidence_round_trips_atomically_and_rehearsals_refuse_to_persist() {
        let tmp = project_dir();
        let evidence = GateEvidence {
            contract: GATE_CONTRACT_VERSION.to_string(),
            project_id: "demo".to_string(),
            runtime: "driftwatchdog".to_string(),
            runtime_version: Some("driftwatch 0.1.0".to_string()),
            revision: Some("abc123def456abc".to_string()),
            manifest_digest: None,
            rule_pack_version: None,
            aggregate: GateAggregate::Passed,
            checks: vec![GateCheck {
                id: "build".to_string(),
                state: GateCheckState::Pass,
                note: None,
            }],
            observed_at: "2026-09-24T00:00:00Z".to_string(),
            dry_run: false,
            note: None,
        };
        let relative = save_evidence(tmp.path(), &evidence).unwrap();
        assert_eq!(relative, evidence_path("demo"));
        assert_eq!(
            load_latest_evidence(tmp.path()).unwrap().as_ref(),
            Some(&evidence)
        );
        // tmp artifact is gone after the rename.
        assert!(!tmp
            .path()
            .join(GATE_EVIDENCE_DIR)
            .join("demo")
            .join("evidence.json.tmp")
            .exists());
        let mut rehearsal = evidence.clone();
        rehearsal.dry_run = true;
        assert!(save_evidence(tmp.path(), &rehearsal).is_err());
    }

    #[test]
    fn corrupt_evidence_names_the_file_and_never_invents_absence() {
        let tmp = project_dir();
        fs::create_dir_all(tmp.path().join(GATE_EVIDENCE_DIR).join("demo")).unwrap();
        fs::write(
            tmp.path()
                .join(GATE_EVIDENCE_DIR)
                .join("demo")
                .join(GATE_EVIDENCE_FILE),
            "{ not json",
        )
        .unwrap();
        let err = load_latest_evidence(tmp.path()).unwrap_err();
        assert_eq!(err.code(), "gate-invalid");
        assert!(err.to_string().contains("evidence.json"), "{err}");
    }

    #[test]
    fn freshness_requires_an_exact_revision_binding() {
        let evidence = GateEvidence {
            contract: GATE_CONTRACT_VERSION.to_string(),
            project_id: "demo".to_string(),
            runtime: "driftwatchdog".to_string(),
            runtime_version: None,
            revision: Some("abc".to_string()),
            manifest_digest: None,
            rule_pack_version: None,
            aggregate: GateAggregate::Passed,
            checks: Vec::new(),
            observed_at: "now".to_string(),
            dry_run: false,
            note: None,
        };
        assert_eq!(
            evidence_freshness(&evidence, Some("abc")),
            GateFreshness::Fresh
        );
        assert_eq!(
            evidence_freshness(&evidence, Some("moved")),
            GateFreshness::Stale
        );
        assert_eq!(evidence_freshness(&evidence, None), GateFreshness::Stale);
        let mut unbound = evidence.clone();
        unbound.revision = None;
        assert_eq!(
            evidence_freshness(&unbound, Some("abc")),
            GateFreshness::Stale
        );
    }

    #[test]
    fn journal_verdicts_follow_the_design_mapping() {
        let evidence = |aggregate| {
            GateOutcome::Evidence(GateEvidence {
                contract: GATE_CONTRACT_VERSION.to_string(),
                project_id: "demo".to_string(),
                runtime: "driftwatchdog".to_string(),
                runtime_version: None,
                revision: None,
                manifest_digest: None,
                rule_pack_version: None,
                aggregate,
                checks: Vec::new(),
                observed_at: "now".to_string(),
                dry_run: false,
                note: None,
            })
        };
        assert_eq!(journal_verdict(&evidence(GateAggregate::Passed)), "done");
        assert_eq!(
            journal_verdict(&evidence(GateAggregate::Blocked)),
            "blocked"
        );
        assert_eq!(journal_verdict(&evidence(GateAggregate::Failed)), "failed");
        assert_eq!(journal_verdict(&evidence(GateAggregate::Unknown)), "failed");
        assert_eq!(
            journal_verdict(&GateOutcome::Unavailable {
                reason: "x".to_string()
            }),
            "failed"
        );
    }

    #[test]
    fn timeout_bounds_refuse_before_anything_runs() {
        assert_eq!(parse_timeout_secs(0).unwrap_err().code(), "gate-invalid");
        assert_eq!(
            parse_timeout_secs(86_401).unwrap_err().code(),
            "gate-invalid"
        );
        assert!(parse_timeout_secs(86_400).is_ok());
    }

    #[test]
    fn gate_managed_detection_matches_the_policy_surface() {
        let tmp = TempDir::new().unwrap();
        assert!(!is_gate_managed(tmp.path()));
        fs::write(tmp.path().join("gate.toml"), "[gate]\nrules = []\n").unwrap();
        assert!(is_gate_managed(tmp.path()));
    }
}
