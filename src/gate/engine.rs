//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use crate::core::ForgeError;
use crate::policy::{first_binary_on_path, redact_credentials, DRIFTWATCH_BINARY_CANDIDATES};
use chrono::{SecondsFormat, Utc};
use std::ffi::OsStr;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread::sleep;
use std::time::{Duration, Instant};

use super::contract::{
    GATE_BIN_ENV, GATE_CONTRACT_VERSION, GATE_EVIDENCE_DIR, GATE_EVIDENCE_FILE,
    MAX_GATE_OUTPUT_BYTES, MAX_GATE_TIMEOUT_SECS, MAX_NOTE_CHARS, MAX_PLAN_LINES,
    MIN_GATE_TIMEOUT_SECS, SUPPORTED_GATE_RUNTIMES,
};
use super::model::{
    DeclaredRuntime, GateAggregate, GateCheck, GateCheckState, GateConfig, GateEvidence,
    GateFreshness, GateOutcome, MappedDocument, ResolvedRuntime,
};

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
pub(super) fn capture_note(text: &str, dir: &Path) -> String {
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

pub(super) fn parse_gate_document(value: &serde_json::Value, dir: &Path) -> Option<MappedDocument> {
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
pub(super) fn classify_aggregate(status: &str, blocked: bool, exit_success: bool) -> GateAggregate {
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
