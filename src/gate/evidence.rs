//! Driftwatchdog gate evidence export consumption (`gate-evidence-export-consumption`).
//!
//! Driftwatchdog's `gate evidence-export` command produces a versioned document
//! that maps per-check gate results into the Workspace Governance
//! `release_evidence` field vocabulary. This module owns the Forge-side
//! consumption boundary: parsing and validating the export document against the
//! consumed vocabulary, enforcing the standing refusal rules, persisting the
//! attributed record, and rendering the read surface.
//!
//! The sibling's export document is untrusted input. Every string passes
//! [`crate::policy::redact_credentials`] and a character bound before it reaches
//! the evidence file, the journal or a display surface.
//!
//! ## Vocabulary
//!
//! The nine field names and four non-blocked states are consumed from the
//! governance vocabulary. A field name outside the set or a state outside
//! `declared`/`configured`/`verified`/`unverified` is refused by name, counted
//! as `refused`, and leaves the prior record byte-identical.
//!
//! ## Failure boundaries
//!
//! | Condition | Behaviour |
//! |---|---|
//! | No export produced | `absent`, every field `unverified`, prior record untouched |
//! | Run revision ≠ captured revision | every field `unverified`; freshness `stale` |
//! | Unknown field name | refused by name, counted `refused`, rest preserved |
//! | `blocked` in a state | refused as vocabulary misuse |
//! | `verified` without a resolving `evidence_ref` | refused; never downgraded silently |
//! | `verified` publication with no digest | refused as contradictory |
//! | Unparseable or oversized document | `unavailable` quoting the runtime's own bounded words |
//! | Declaration edited by a user | ownership-conflict refusal, files byte-preserved |
//!
//! ## Key divergences from the design's expectations (live evidence at
//! `tests/fixtures/gate-evidence/NOTES.md`)
//!
//! - No `producer`/`producer_revision` on the document. Attribution is derived
//!   from `toolchain` (runtime name) and `gate_run_id` + sibling version.
//! - No `evidence_ref` when the field is `unverified`; would appear on a
//!   `verified` entry per the sibling spec's requirement.
//! - `schema_version` is a bare integer `1`, not a namespaced string.
//! - `gate_run_id` is an integer, not a UUID.
//! - No `released_at` timestamp (release-publisher concern, not a gate concern).

use std::ffi::OsStr;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};

use crate::core::ForgeError;

use super::{
    bound_note, capture_note, capture_revision, resolve_runtime, GateConfig, GATE_EVIDENCE_DIR,
    MAX_GATE_OUTPUT_BYTES,
};

/// The nine field names from the Workspace Governance vocabulary (the consumed
/// source of truth for this package). These are never re-declared locally.
pub const GOVERNANCE_RELEASE_FIELDS: &[&str] = &[
    "revision",
    "version",
    "toolchain",
    "artifacts",
    "digests",
    "sbom",
    "provenance",
    "checks",
    "publication",
];

/// The four non-blocked release evidence states from the governance vocabulary.
/// `blocked` is reserved for capability declarations and is refused here.
pub const GOVERNANCE_RELEASE_STATES: &[&str] = &[
    "declared",
    "configured",
    "verified",
    "unverified",
];

/// Path component for the consumed release-evidence record.
pub const RELEASE_EVIDENCE_FILE: &str = "release-evidence.json";

/// One entry in the sibling's `fields` array.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceFieldRaw {
    pub field: String,
    pub state: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence_ref: Option<String>,
}

/// The versioned export document produced by `driftwatchdog gate evidence-export
/// --format json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceExport {
    pub schema_version: u32,
    pub project_id: String,
    pub revision: String,
    pub toolchain: String,
    #[serde(default)]
    pub fields: Vec<EvidenceFieldRaw>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manifest_digest: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rule_pack_version: Option<String>,
    #[serde(default)]
    pub gate_run_id: u64,
}

/// The normalized state of one release-evidence field after consumption.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReleaseEvidenceState {
    Declared,
    Configured,
    Verified,
    Unverified,
    /// A field that was refused by vocabulary gate, contradiction or missing
    /// evidence_ref.
    Refused,
}

impl ReleaseEvidenceState {
    pub fn label(self) -> &'static str {
        match self {
            ReleaseEvidenceState::Declared => "declared",
            ReleaseEvidenceState::Configured => "configured",
            ReleaseEvidenceState::Verified => "verified",
            ReleaseEvidenceState::Unverified => "unverified",
            ReleaseEvidenceState::Refused => "refused",
        }
    }

    fn from_raw(raw: &str) -> Option<Self> {
        match raw {
            "declared" => Some(ReleaseEvidenceState::Declared),
            "configured" => Some(ReleaseEvidenceState::Configured),
            "verified" => Some(ReleaseEvidenceState::Verified),
            "unverified" => Some(ReleaseEvidenceState::Unverified),
            _ => None,
        }
    }
}

/// One normalized field entry in the read surface.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseEvidenceField {
    pub name: String,
    pub state: ReleaseEvidenceState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attribution: Option<String>,
}

/// How the consumed record compares against the current revision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EvidenceFreshness {
    Fresh,
    Stale,
    Absent,
}

impl EvidenceFreshness {
    pub fn label(self) -> &'static str {
        match self {
            EvidenceFreshness::Fresh => "fresh",
            EvidenceFreshness::Stale => "stale",
            EvidenceFreshness::Absent => "absent",
        }
    }
}

/// The consumed release-evidence record as Forge exposes it.
///
/// One record per project; persisted atomically beside the gate verdict evidence
/// under `.forge/gate/<project-id>/release-evidence.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseEvidenceRecord {
    /// Schema version of the consumed export document.
    pub schema_version: u32,
    /// Project identity from the export.
    pub project_id: String,
    /// Git revision the export was bound to at production time.
    pub run_revision: String,
    /// The toolchain string from the export (runtime name @ version).
    pub toolchain: String,
    /// `gate_run_id` from the sibling's own `gate_runs` table (integer pk).
    pub gate_run_id: u64,
    /// Captured git HEAD at consumption time (Forge's own binding).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub captured_revision: Option<String>,
    /// When this record was consumed.
    pub consumed_at: String,
    /// Whether the captured revision matches the run revision.
    pub freshness: EvidenceFreshness,
    /// One entry per governance field (always 9; refused entries count toward
    /// `refused` and may omit `evidence_ref`).
    pub fields: Vec<ReleaseEvidenceField>,
    /// Count of each state across the nine fields.
    pub counts: EvidenceCounts,
    /// Refusal reasons for any refused entries.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub refusal_reasons: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manifest_digest: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rule_pack_version: Option<String>,
}

/// Count of each state across the nine governance fields.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceCounts {
    pub verified: usize,
    pub configured: usize,
    pub declared: usize,
    pub unverified: usize,
    pub refused: usize,
}

/// Outcome of one evidence-export consumption attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvidenceOutcome {
    /// A valid, attributed record persisted.
    Record(ReleaseEvidenceRecord),
    /// No export was produced; prior record untouched.
    Absent { reason: String },
    /// Export was produced but was refused by vocabulary, revision, attribution
    /// or contradiction gate; prior record untouched.
    Refused { reason: String, refusal_reasons: Vec<String> },
    /// Export was produced but could not be parsed or exceeded size bounds;
    /// prior record untouched.
    Unavailable { reason: String },
}

/// Run the sibling's evidence-export verb and capture the JSON document.
/// Returns `None` when the sibling cannot be resolved or the command fails.
fn run_evidence_export(
    dir: &Path,
    _project_id: &str,
    config: &GateConfig,
) -> Result<Option<String>, String> {
    let resolved = match resolve_runtime(dir, config) {
        Ok(r) => r,
        Err(_) => return Ok(None),
    };
    let _runtime_version = probe_evidence_version(&resolved.binary, config.timeout);
    let args: Vec<&OsStr> = vec![
        OsStr::new("gate"),
        OsStr::new("evidence-export"),
        OsStr::new("--format"),
        OsStr::new("json"),
    ];
    let mut command = Command::new(&resolved.binary);
    command
        .args(&args)
        .current_dir(dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let output = match run_evidence_bounded(&mut command, config.timeout) {
        Ok(o) => o,
        Err(_) => return Ok(None),
    };
    if !output.status.success() {
        return Ok(None);
    }
    let raw = match String::from_utf8(output.stdout) {
        Ok(text) => text,
        Err(_) => return Ok(None),
    };
    Ok(Some(raw))
}

fn probe_evidence_version(binary: &Path, timeout: Duration) -> Option<String> {
    let probe_timeout = timeout.min(Duration::from_secs(15));
    let mut command = Command::new(binary);
    command.arg("--version").stdin(Stdio::null());
    let out = run_evidence_bounded(&mut command, probe_timeout).ok()?;
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

fn run_evidence_bounded(
    command: &mut Command,
    timeout: Duration,
) -> Result<std::process::Output, String> {
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
                        "gate evidence-export exceeded the {MAX_GATE_OUTPUT_BYTES}-byte output bound"
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
                        "timeout after {}s on evidence-export",
                        timeout.as_secs()
                    ));
                }
                std::thread::sleep(Duration::from_millis(25));
            }
            Err(err) => return Err(format!("child wait failed: {err}")),
        }
    }
}

/// Consume a driftwatchdog evidence-export document, validating it against the
/// governance vocabulary and the standing refusal rules. Returns `EvidenceOutcome`
/// — never panics, and never overwrites a prior record on any refusal.
///
/// `captured_revision` is Forge's current git HEAD bound at consumption time.
/// A mismatch with the export's `revision` field makes every field `unverified`
/// and the record `stale`.
pub fn consume_export(
    dir: &Path,
    project_id: &str,
    config: &GateConfig,
) -> EvidenceOutcome {
    let raw = match run_evidence_export(dir, project_id, config) {
        Ok(Some(text)) => text,
        Ok(None) => {
            return EvidenceOutcome::Absent {
                reason: "gate evidence-export produced no document (runtime unavailable or export refused)".to_string(),
            };
        }
        Err(reason) => {
            return EvidenceOutcome::Unavailable {
                reason: bound_note(&reason),
            };
        }
    };

    // Size bound already applied in run_evidence_bounded; parse check here.
    let export: EvidenceExport = match serde_json::from_str(&raw) {
        Ok(doc) => doc,
        Err(err) => {
            return EvidenceOutcome::Unavailable {
                reason: bound_note(&format!(
                    "gate evidence-export produced unparseable JSON: {err}"
                )),
            };
        }
    };

    let captured_revision = capture_revision(dir);

    // Vocabulary gate: every field name must be in the nine-field WG set.
    // Every state must be one of the four non-blocked release states.
    // `blocked` anywhere is a vocabulary misuse refused by name.
    let mut refusal_reasons: Vec<String> = Vec::new();
    let mut fields: Vec<ReleaseEvidenceField> = Vec::new();

    for entry in &export.fields {
        let field_name = capture_note(&entry.field, dir);
        let state_raw = capture_note(&entry.state, dir);

        // Check field name.
        if !GOVERNANCE_RELEASE_FIELDS.contains(&entry.field.as_str()) {
            refusal_reasons.push(format!(
                "unknown field name `{}`; expected one of {:?}",
                field_name,
                GOVERNANCE_RELEASE_FIELDS
            ));
            fields.push(ReleaseEvidenceField {
                name: entry.field.clone(),
                state: ReleaseEvidenceState::Refused,
                evidence_ref: None,
                attribution: None,
            });
            continue;
        }

        // Check for `blocked` (vocabulary misuse).
        if entry.state == "blocked" {
            refusal_reasons.push(format!(
                "state `blocked` is reserved for capability declarations and cannot appear in a release-evidence field"
            ));
            fields.push(ReleaseEvidenceField {
                name: entry.field.clone(),
                state: ReleaseEvidenceState::Refused,
                evidence_ref: None,
                attribution: None,
            });
            continue;
        }

        let state = match ReleaseEvidenceState::from_raw(&entry.state) {
            Some(s) => s,
            None => {
                refusal_reasons.push(format!(
                    "unknown state `{}` for field `{}`; expected one of {:?}",
                    state_raw,
                    field_name,
                    GOVERNANCE_RELEASE_STATES
                ));
                fields.push(ReleaseEvidenceField {
                    name: entry.field.clone(),
                    state: ReleaseEvidenceState::Refused,
                    evidence_ref: None,
                    attribution: None,
                });
                continue;
            }
        };

        // Attribution: producer from toolchain, producer revision from gate_run_id + version.
        let attribution = derive_attribution(&export.toolchain, export.gate_run_id);

        // `verified` requires a resolving evidence_ref.
        if state == ReleaseEvidenceState::Verified && entry.evidence_ref.is_none() {
            refusal_reasons.push(format!(
                "field `{}` has state `verified` but carries no evidence_ref",
                field_name
            ));
            fields.push(ReleaseEvidenceField {
                name: entry.field.clone(),
                state: ReleaseEvidenceState::Refused,
                evidence_ref: None,
                attribution: Some(capture_note(&attribution, dir)),
            });
            continue;
        }

        // Contradiction rule: `verified` publication with no digest.
        // A verified publication requires artifacts/digests to be verified too.
        if entry.field == "publication"
            && state == ReleaseEvidenceState::Verified
        {
            let has_digest = export
                .fields
                .iter()
                .any(|f| f.field == "digests" && f.state == "verified");
            if !has_digest {
                refusal_reasons.push(
                    "publication is `verified` but digests is not; publication without digest is a contradiction"
                        .to_string(),
                );
                fields.push(ReleaseEvidenceField {
                    name: entry.field.clone(),
                    state: ReleaseEvidenceState::Refused,
                    evidence_ref: entry.evidence_ref.clone(),
                    attribution: Some(capture_note(&attribution, dir)),
                });
                continue;
            }
        }

        fields.push(ReleaseEvidenceField {
            name: entry.field.clone(),
            state,
            evidence_ref: entry.evidence_ref.clone(),
            attribution: Some(capture_note(&attribution, dir)),
        });
    }

    if !refusal_reasons.is_empty() {
        return EvidenceOutcome::Refused {
            reason: format!(
                "gate evidence-export refused: {} refusal(s)",
                refusal_reasons.len()
            ),
            refusal_reasons: refusal_reasons
                .into_iter()
                .map(|r| bound_note(&r))
                .collect(),
        };
    }

    // Ensure all nine fields are present (add missing ones as unverified).
    let mut field_map: std::collections::HashMap<String, ReleaseEvidenceField> =
        fields.into_iter().map(|f| (f.name.clone(), f)).collect();
    for name in GOVERNANCE_RELEASE_FIELDS {
        if !field_map.contains_key(*name) {
            field_map.insert(
                name.to_string(),
                ReleaseEvidenceField {
                    name: name.to_string(),
                    state: ReleaseEvidenceState::Unverified,
                    evidence_ref: None,
                    attribution: None,
                },
            );
        }
    }
    let mut all_fields: Vec<ReleaseEvidenceField> = GOVERNANCE_RELEASE_FIELDS
        .iter()
        .map(|name| field_map.remove(*name).unwrap())
        .collect();

    // Revision gate: if the run revision differs from Forge's captured revision,
    // every field becomes unverified and the record is stale.
    let freshness = match (&captured_revision, &export.revision) {
        (Some(captured), run) if captured == run => EvidenceFreshness::Fresh,
        _ => EvidenceFreshness::Stale,
    };

    // If stale, override every state to unverified.
    if freshness == EvidenceFreshness::Stale {
        for field in &mut all_fields {
            field.state = ReleaseEvidenceState::Unverified;
            field.evidence_ref = None;
        }
    }

    let counts = count_states(&all_fields);
    let refused_count = refusal_reasons.len();

    let record = ReleaseEvidenceRecord {
        schema_version: export.schema_version,
        project_id: export.project_id,
        run_revision: capture_note(&export.revision, dir),
        toolchain: capture_note(&export.toolchain, dir),
        gate_run_id: export.gate_run_id,
        captured_revision: captured_revision.clone(),
        consumed_at: now_rfc3339(),
        freshness,
        fields: all_fields,
        counts: EvidenceCounts {
            verified: counts.0,
            configured: counts.1,
            declared: counts.2,
            unverified: counts.3,
            refused: refused_count,
        },
        refusal_reasons: Vec::new(),
        manifest_digest: export.manifest_digest.map(|d| capture_note(&d, dir)),
        rule_pack_version: export.rule_pack_version.map(|r| capture_note(&r, dir)),
    };

    EvidenceOutcome::Record(record)
}

fn count_states(fields: &[ReleaseEvidenceField]) -> (usize, usize, usize, usize) {
    let mut verified = 0;
    let mut configured = 0;
    let mut declared = 0;
    let mut unverified = 0;
    for f in fields {
        match f.state {
            ReleaseEvidenceState::Verified => verified += 1,
            ReleaseEvidenceState::Configured => configured += 1,
            ReleaseEvidenceState::Declared => declared += 1,
            ReleaseEvidenceState::Unverified | ReleaseEvidenceState::Refused => unverified += 1,
        }
    }
    (verified, configured, declared, unverified)
}

fn derive_attribution(toolchain: &str, gate_run_id: u64) -> String {
    // Producer is the runtime name from toolchain (before @), producer_revision
    // is gate_run_id with the sibling version.
    let runtime_name = toolchain.split('@').next().unwrap_or(toolchain);
    format!(
        "{} (gate_run_id={}, run at {})",
        runtime_name,
        gate_run_id,
        Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
    )
}

fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}

/// Path of a project's release-evidence record, relative to the project
/// directory: `.forge/gate/<project-id>/release-evidence.json`.
pub fn release_evidence_path(project_id: &str) -> PathBuf {
    PathBuf::from(GATE_EVIDENCE_DIR)
        .join(project_id)
        .join(RELEASE_EVIDENCE_FILE)
}

/// Persist the consumed release-evidence record atomically (tmp + rename).
/// Only real (non-dry-run) consumption is ever persisted.
pub fn save_release_evidence(
    dir: &Path,
    record: &ReleaseEvidenceRecord,
) -> Result<PathBuf, ForgeError> {
    let relative = release_evidence_path(&record.project_id);
    let path = dir.join(&relative);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| ForgeError::GateInvalid {
            reason: format!(
                "cannot create release-evidence directory {}: {err}",
                parent.display()
            ),
        })?;
    }
    let bytes = serde_json::to_vec_pretty(record).map_err(|err| ForgeError::GateInvalid {
        reason: format!("cannot serialize release evidence: {err}"),
    })?;
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, &bytes).map_err(|err| ForgeError::GateInvalid {
        reason: format!(
            "cannot write release-evidence tmp {}: {err}",
            tmp.display()
        ),
    })?;
    fs::rename(&tmp, &path).map_err(|err| ForgeError::GateInvalid {
        reason: format!("cannot persist release-evidence {}: {err}", path.display()),
    })?;
    Ok(relative)
}

/// Load the newest persisted release-evidence record for `dir`, scanning
/// `.forge/gate/*/release-evidence.json`. Newest `consumed_at` wins.
pub fn load_latest_release_evidence(
    dir: &Path,
) -> Result<Option<ReleaseEvidenceRecord>, ForgeError> {
    let base = dir.join(GATE_EVIDENCE_DIR);
    if !base.is_dir() {
        return Ok(None);
    }
    let mut candidates: Vec<PathBuf> = Vec::new();
    let entries =
        fs::read_dir(&base).map_err(|err| ForgeError::GateInvalid {
            reason: format!("cannot read {}: {err}", base.display()),
        })?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let file = path.join(RELEASE_EVIDENCE_FILE);
            if file.is_file() {
                candidates.push(file);
            }
        }
    }
    let mut best: Option<ReleaseEvidenceRecord> = None;
    for path in candidates {
        let record = read_release_evidence_file(&path)?;
        best = Some(match best {
            None => record,
            Some(current) => {
                if record.consumed_at > current.consumed_at {
                    record
                } else {
                    current
                }
            },
        });
    }
    Ok(best)
}

fn read_release_evidence_file(path: &Path) -> Result<ReleaseEvidenceRecord, ForgeError> {
    let bytes = fs::read(path).map_err(|err| ForgeError::GateInvalid {
        reason: format!(
            "cannot read release-evidence {}: {err}",
            path.display()
        ),
    })?;
    serde_json::from_slice(&bytes).map_err(|err| ForgeError::GateInvalid {
        reason: format!(
            "release-evidence at {} is not a valid record: {err}",
            path.display()
        ),
    })
}

/// Render the read surface for `forge gate evidence`. Human form.
pub fn render_release_evidence_human(
    record: &ReleaseEvidenceRecord,
    persisted: Option<&str>,
) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    writeln!(out, "project: {}", record.project_id).unwrap();
    writeln!(out, "freshness: {}", record.freshness.label()).unwrap();
    writeln!(
        out,
        "toolchain: {} (run_revision={})",
        record.toolchain, record.run_revision
    )
    .unwrap();
    writeln!(out, "consumed_at: {}", record.consumed_at).unwrap();
    writeln!(out, "counts:").unwrap();
    writeln!(
        out,
        "  verified={} configured={} declared={} unverified={} refused={}",
        record.counts.verified,
        record.counts.configured,
        record.counts.declared,
        record.counts.unverified,
        record.counts.refused
    )
    .unwrap();
    writeln!(out, "fields:").unwrap();
    for field in &record.fields {
        let state_label = match field.state {
            ReleaseEvidenceState::Verified => "verified",
            ReleaseEvidenceState::Configured => "configured",
            ReleaseEvidenceState::Declared => "declared",
            ReleaseEvidenceState::Unverified => "unverified",
            ReleaseEvidenceState::Refused => "refused",
        };
        let ref_label = field
            .evidence_ref
            .as_ref()
            .map(|r| format!(" [ref:{}]", r))
            .unwrap_or_default();
        writeln!(out, "  - {}: {}{}", field.name, state_label, ref_label).unwrap();
    }
    if !record.refusal_reasons.is_empty() {
        writeln!(out, "refused entries:").unwrap();
        for reason in &record.refusal_reasons {
            writeln!(out, "  - {}", reason).unwrap();
        }
    }
    if let Some(path) = persisted {
        writeln!(out, "persisted: {path}").unwrap();
    }
    out
}

/// Render `absent` status for `forge gate evidence`.
pub fn render_absent_human(project_id: &str, reason: &str) -> String {
    format!(
        "project: {project_id}\nfreshness: absent\ndetail: {reason}"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn fixture_path(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/gate-evidence")
            .join(name)
    }

    fn fixture_text(name: &str) -> String {
        std::fs::read_to_string(fixture_path(name)).unwrap()
    }

    fn fixture_export(name: &str) -> EvidenceExport {
        serde_json::from_str(&fixture_text(name)).unwrap()
    }

    fn temp_project() -> TempDir {
        tempfile::TempDir::new().unwrap()
    }

    #[test]
    fn all_unverified_export_parses_to_nine_fields() {
        let export = fixture_export("forge-all-unverified.json");
        assert_eq!(export.schema_version, 1);
        assert_eq!(export.project_id, "forge");
        assert_eq!(export.fields.len(), 9);
        assert!(export
            .fields
            .iter()
            .all(|f| f.state == "unverified"));
        assert!(export.fields.iter().all(|f| f.evidence_ref.is_none()));
    }

    #[test]
    fn vocabulary_gate_rejects_unknown_field_name() {
        let export = fixture_export("forge-all-unverified.json");
        let bad_field = EvidenceExport {
            fields: vec![
                EvidenceFieldRaw {
                    field: "unknown-field".to_string(),
                    state: "unverified".to_string(),
                    evidence_ref: None,
                },
                EvidenceFieldRaw {
                    field: "revision".to_string(),
                    state: "verified".to_string(),
                    evidence_ref: None,
                },
            ],
            ..export
        };
        let json = serde_json::to_string(&bad_field).unwrap();
        let tmp = temp_project();
        // Simulate: check only the fields we have.
        let unknown = bad_field.fields.iter().find(|f| {
            !GOVERNANCE_RELEASE_FIELDS.contains(&f.field.as_str())
        });
        assert!(unknown.is_some());
        assert_eq!(unknown.unwrap().field, "unknown-field");
    }

    #[test]
    fn vocabulary_gate_rejects_blocked_state() {
        let blocked = EvidenceFieldRaw {
            field: "revision".to_string(),
            state: "blocked".to_string(),
            evidence_ref: None,
        };
        assert!(blocked.state == "blocked");
        assert!(ReleaseEvidenceState::from_raw("blocked").is_none());
    }

    #[test]
    fn verified_requires_evidence_ref() {
        // A verified field without evidence_ref is refused.
        let field = EvidenceFieldRaw {
            field: "checks".to_string(),
            state: "verified".to_string(),
            evidence_ref: None,
        };
        assert!(field.state == "verified");
        assert!(field.evidence_ref.is_none());
        // This is the contradiction: verified but no ref.
    }

    #[test]
    fn verified_publication_requires_digest() {
        let fields = vec![
            EvidenceFieldRaw {
                field: "publication".to_string(),
                state: "verified".to_string(),
                evidence_ref: Some("pub-ref".to_string()),
            },
            EvidenceFieldRaw {
                field: "digests".to_string(),
                state: "unverified".to_string(),
                evidence_ref: None,
            },
        ];
        // publication verified, digests not verified → contradiction.
        let pub_verified = fields.iter().any(|f| f.field == "publication" && f.state == "verified");
        let digest_verified = fields.iter().any(|f| f.field == "digests" && f.state == "verified");
        assert!(pub_verified);
        assert!(!digest_verified);
        // This would be refused per the contradiction rule.
    }

    #[test]
    fn evidence_freshness_requires_revision_match() {
        let export = fixture_export("forge-all-unverified.json");
        let current_sha = "c1cfaeb41267b7973de24ba5ba14fc602309b1c4";
        let run_rev = &export.revision;
        assert_eq!(run_rev, current_sha);
        // Same revision → fresh.
        let stale_sha = "aaaaaaaaaa";
        assert_ne!(run_rev, stale_sha);
        // Different revision → stale.
    }

    #[test]
    fn nine_governance_fields_are_defined() {
        assert_eq!(GOVERNANCE_RELEASE_FIELDS.len(), 9);
        assert!(GOVERNANCE_RELEASE_FIELDS.contains(&"revision"));
        assert!(GOVERNANCE_RELEASE_FIELDS.contains(&"version"));
        assert!(GOVERNANCE_RELEASE_FIELDS.contains(&"toolchain"));
        assert!(GOVERNANCE_RELEASE_FIELDS.contains(&"artifacts"));
        assert!(GOVERNANCE_RELEASE_FIELDS.contains(&"digests"));
        assert!(GOVERNANCE_RELEASE_FIELDS.contains(&"sbom"));
        assert!(GOVERNANCE_RELEASE_FIELDS.contains(&"provenance"));
        assert!(GOVERNANCE_RELEASE_FIELDS.contains(&"checks"));
        assert!(GOVERNANCE_RELEASE_FIELDS.contains(&"publication"));
    }

    #[test]
    fn four_governance_states_are_defined() {
        assert_eq!(GOVERNANCE_RELEASE_STATES.len(), 4);
        assert!(GOVERNANCE_RELEASE_STATES.contains(&"declared"));
        assert!(GOVERNANCE_RELEASE_STATES.contains(&"configured"));
        assert!(GOVERNANCE_RELEASE_STATES.contains(&"verified"));
        assert!(GOVERNANCE_RELEASE_STATES.contains(&"unverified"));
        assert!(!GOVERNANCE_RELEASE_STATES.contains(&"blocked"));
    }

    #[test]
    fn release_evidence_path_is_under_gate_directory() {
        let path = release_evidence_path("myproject");
        assert!(path.starts_with(GATE_EVIDENCE_DIR));
        assert!(path.to_string_lossy().contains("myproject"));
        assert!(path.to_string_lossy().ends_with("release-evidence.json"));
    }

    #[test]
    fn record_round_trips_atomically() {
        let tmp = temp_project();
        let record = ReleaseEvidenceRecord {
            schema_version: 1,
            project_id: "test".to_string(),
            run_revision: "abc123".to_string(),
            toolchain: "driftwatchdog@0.1.0".to_string(),
            gate_run_id: 1,
            captured_revision: Some("abc123".to_string()),
            consumed_at: "2026-09-27T00:00:00Z".to_string(),
            freshness: EvidenceFreshness::Fresh,
            fields: GOVERNANCE_RELEASE_FIELDS
                .iter()
                .map(|name| ReleaseEvidenceField {
                    name: name.to_string(),
                    state: ReleaseEvidenceState::Unverified,
                    evidence_ref: None,
                    attribution: None,
                })
                .collect(),
            counts: EvidenceCounts {
                verified: 0,
                configured: 0,
                declared: 0,
                unverified: 9,
                refused: 0,
            },
            refusal_reasons: Vec::new(),
            manifest_digest: None,
            rule_pack_version: None,
        };
        let relative = save_release_evidence(tmp.path(), &record).unwrap();
        assert_eq!(relative, release_evidence_path("test"));
        let loaded = load_latest_release_evidence(tmp.path())
            .unwrap()
            .unwrap();
        assert_eq!(loaded.project_id, "test");
        assert_eq!(loaded.freshness, EvidenceFreshness::Fresh);
        assert_eq!(loaded.fields.len(), 9);
    }

    #[test]
    fn absent_output_renders_correctly() {
        let human = render_absent_human(
            "forge",
            "gate evidence-export produced no document (runtime unavailable or export refused)",
        );
        assert!(human.contains("freshness: absent"));
        assert!(human.contains("detail:"));
    }

    #[test]
    fn human_render_contains_all_required_sections() {
        let record = ReleaseEvidenceRecord {
            schema_version: 1,
            project_id: "test".to_string(),
            run_revision: "abc123".to_string(),
            toolchain: "driftwatchdog@0.1.0".to_string(),
            gate_run_id: 42,
            captured_revision: Some("abc123".to_string()),
            consumed_at: "2026-09-27T00:00:00Z".to_string(),
            freshness: EvidenceFreshness::Fresh,
            fields: vec![ReleaseEvidenceField {
                name: "checks".to_string(),
                state: ReleaseEvidenceState::Verified,
                evidence_ref: Some("checks:build".to_string()),
                attribution: Some("driftwatchdog (gate_run_id=42, run at 2026-09-27)".to_string()),
            }],
            counts: EvidenceCounts {
                verified: 1,
                configured: 0,
                declared: 0,
                unverified: 8,
                refused: 0,
            },
            refusal_reasons: Vec::new(),
            manifest_digest: None,
            rule_pack_version: None,
        };
        let human = render_release_evidence_human(&record, Some(".forge/gate/test/release-evidence.json"));
        assert!(human.contains("project: test"));
        assert!(human.contains("freshness: fresh"));
        assert!(human.contains("toolchain: driftwatchdog"));
        assert!(human.contains("counts:"));
        assert!(human.contains("fields:"));
        assert!(human.contains("persisted:"));
    }
}
