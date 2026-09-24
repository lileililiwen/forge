//! Read-only fleet observation from the external workspace registry
//! (`fleet-registry-observation`).
//!
//! Workspace Governance owns `projects.json`: its schema, adoption
//! decisions and any writes. This module owns only a read-only parser and
//! a projection of that document into timestamped fleet observations. The
//! local SQLite registry stays the only source of truth for managed
//! projects; fleet data is never copied into it and fleet reads are never
//! journaled operations, so staleness is expressed honestly by
//! `observed_at` instead of a drifting mirror table.
//!
//! ## Input contract (schema_version 1)
//!
//! Accepted document shape, mirroring the sibling registry schema:
//!
//! ```json
//! {"schema_version": 1,
//!  "workspace_root": null,
//!  "discovery": {"mode": "...", "unregistered_policy": "...", "exclude": ["..."]},
//!  "projects": [{"id": "...", "path": "...", "profile": "...",
//!                "lifecycle": "...", "adoption": "..."}]}
//! ```
//!
//! Every field the parser must tolerate (recorded by task 1.1): the
//! top-level keys `schema_version`, `workspace_root` (string or null),
//! `discovery` (`mode`, `unregistered_policy`, `exclude`) and `projects`;
//! per entry `id`, `path`, `profile`, `lifecycle` and the optional
//! nullable `adoption` (the real registry carries `"adopted"` or `null`);
//! and any unknown top-level or per-entry fields (tolerated, e.g. the
//! generated-deployment siblings `generated_by` / `generated_at`).
//!
//! `workspace_root` is deliberately never used to relocate the
//! confinement root: the document is untrusted input, and the sibling
//! convention is that the registry stores no machine-specific root (the
//! root is chosen at invocation). The confinement root is therefore the
//! canonicalized directory containing the registry document, and entry
//! paths resolve inside it.
//!
//! ## Rejection taxonomy (`fleet-registry-invalid`)
//!
//! - Report-level refusals (typed error, exit non-zero, never a partial
//!   silent drop): unreadable file, oversized file, malformed JSON,
//!   missing or unknown `schema_version`, non-object `discovery`,
//!   non-array or oversized `projects`, duplicate ids (naming the id).
//! - Per-entry malformed refusals (the entry is excluded and named with
//!   its reason; the remaining valid entries still report): blank or
//!   non-kebab id, oversized id, blank/oversized/absolute-escape path,
//!   path that resolves outside the root through traversal or a symlink,
//!   non-string or blank `profile`/`lifecycle`.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Component, Path, PathBuf};

use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::core::{validate_project_id, ForgeError};
use crate::policy::redact_credentials;

/// Versioned contract for the fleet observation surface.
pub const FLEET_CONTRACT_VERSION: &str = "0.1.0";

/// Environment variable naming the workspace registry document when the
/// `--workspace-registry` flag is absent.
pub const WORKSPACE_REGISTRY_ENV: &str = "FORGE_WORKSPACE_REGISTRY";

/// Schema version this parser understands.
pub const SUPPORTED_SCHEMA_VERSION: u64 = 1;

/// Default maximum registry age in seconds (matches the docs freshness
/// one-day vocabulary).
pub const DEFAULT_MAX_AGE_SECONDS: i64 = 86_400;

/// Smallest accepted `--max-age` value.
pub const MIN_MAX_AGE_SECONDS: i64 = 1;

/// Largest accepted `--max-age` value (one year).
pub const MAX_MAX_AGE_SECONDS: i64 = 31_536_000;

/// Registry files larger than this refuse as invalid so a runaway or
/// hostile document cannot pin the reader.
pub const MAX_REGISTRY_BYTES: u64 = 1_048_576;

/// Hard bound on the number of `projects` entries in one document.
pub const MAX_REGISTRY_ENTRIES: usize = 1_024;

/// Per-field display bound after redaction.
pub const MAX_FIELD_CHARS: usize = 200;

/// Maximum accepted id length. Fleet ids also surface in portal entries
/// prefixed with `fleet:`, which caps portal ids at 128 chars.
pub const MAX_FLEET_ID_CHARS: usize = 100;

/// Maximum accepted declared path length.
pub const MAX_FLEET_PATH_CHARS: usize = 1_024;

/// Registry freshness classification from the file mtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FleetFreshness {
    /// The registry document is newer than the configured max age.
    Fresh,
    /// The registry document is older than the configured max age.
    Stale,
    /// No registry path is configured; nothing was contacted.
    Unconfigured,
}

impl FleetFreshness {
    /// Stable kebab-case id.
    pub fn id(self) -> &'static str {
        match self {
            FleetFreshness::Fresh => "fresh",
            FleetFreshness::Stale => "stale",
            FleetFreshness::Unconfigured => "unconfigured",
        }
    }
}

/// Management state of a fleet entry: only the local registry decides.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FleetState {
    /// The same project id exists in the local registry.
    Managed,
    /// The local registry has no record of this id; mirroring never
    /// registers, imports or mutates it.
    Unmanaged,
}

impl FleetState {
    /// Stable kebab-case id.
    pub fn id(self) -> &'static str {
        match self {
            FleetState::Managed => "managed",
            FleetState::Unmanaged => "unmanaged",
        }
    }
}

/// One projected fleet entry. `profile` and `lifecycle` are surfaced
/// verbatim from the Workspace Governance vocabulary (never coerced to
/// Forge profiles) after credential redaction and length bounding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FleetEntry {
    pub id: String,
    pub path: String,
    pub profile: String,
    pub lifecycle: String,
    #[serde(default)]
    pub adoption: Option<String>,
    pub forge_yaml_present: bool,
    pub locally_registered: bool,
    pub state: FleetState,
}

/// One excluded malformed entry: named with the reason, never silently
/// dropped.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FleetMalformedEntry {
    pub name: String,
    pub reason: String,
}

/// The timestamped fleet report. Re-read on demand; never persisted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FleetReport {
    pub contract: String,
    /// Absolute path of the registry document, or `None` when unconfigured.
    pub source: Option<String>,
    pub observed_at: String,
    pub freshness: FleetFreshness,
    pub max_age_seconds: i64,
    /// Age of the registry document in whole seconds at observation time.
    pub age_seconds: Option<i64>,
    pub entries: Vec<FleetEntry>,
    pub malformed: Vec<FleetMalformedEntry>,
}

impl FleetReport {
    /// Total declared entries (valid plus malformed) so a consumer can
    /// tell projection from drop.
    pub fn declared_count(&self) -> usize {
        self.entries.len() + self.malformed.len()
    }
}

/// Current RFC 3339 timestamp for report generation.
pub fn now_rfc3339() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}

/// Resolve the registry document path from the CLI flag or the
/// `FORGE_WORKSPACE_REGISTRY` environment variable. `None` means the
/// fleet source is unconfigured.
pub fn resolve_registry_path(cli_flag: Option<&Path>) -> Option<PathBuf> {
    if let Some(path) = cli_flag {
        let trimmed = path.as_os_str().to_string_lossy();
        if !trimmed.trim().is_empty() {
            return Some(path.to_path_buf());
        }
    }
    std::env::var(WORKSPACE_REGISTRY_ENV)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

/// Validate the configured `--max-age` window.
pub fn validate_max_age(max_age_seconds: i64) -> Result<(), ForgeError> {
    if !(MIN_MAX_AGE_SECONDS..=MAX_MAX_AGE_SECONDS).contains(&max_age_seconds) {
        return Err(ForgeError::FleetRegistryInvalid {
            reason: format!(
                "--max-age {max_age_seconds} is outside the bounded range \
                 {MIN_MAX_AGE_SECONDS}..={MAX_MAX_AGE_SECONDS} seconds"
            ),
        });
    }
    Ok(())
}

/// Project the declared workspace registry into a timestamped fleet
/// report. `local_ids` is the set of ids in the local registry and is
/// only read for the managed/unmanaged join; this function never opens,
/// writes or journals anything.
pub fn observe(
    registry_file: Option<&Path>,
    max_age_seconds: i64,
    local_ids: &BTreeSet<String>,
) -> Result<FleetReport, ForgeError> {
    observe_at(registry_file, max_age_seconds, local_ids, Utc::now())
}

/// Deterministic core of [`observe`] with the clock threaded through so
/// tests can classify freshness without racing the filesystem.
pub fn observe_at(
    registry_file: Option<&Path>,
    max_age_seconds: i64,
    local_ids: &BTreeSet<String>,
    now: DateTime<Utc>,
) -> Result<FleetReport, ForgeError> {
    let observed_at = now.to_rfc3339_opts(SecondsFormat::Secs, true);
    let Some(path) = registry_file else {
        return Ok(FleetReport {
            contract: FLEET_CONTRACT_VERSION.to_string(),
            source: None,
            observed_at,
            freshness: FleetFreshness::Unconfigured,
            max_age_seconds,
            age_seconds: None,
            entries: Vec::new(),
            malformed: Vec::new(),
        });
    };
    validate_max_age(max_age_seconds)?;
    let document = read_document(path)?;
    let (entries, malformed) = project_entries(path, &document, local_ids)?;
    let (age_seconds, freshness) = classify_freshness(path, max_age_seconds, now)?;
    Ok(FleetReport {
        contract: FLEET_CONTRACT_VERSION.to_string(),
        source: Some(absolute_display(path)),
        observed_at,
        freshness,
        max_age_seconds,
        age_seconds,
        entries,
        malformed,
    })
}

fn fleet_invalid(reason: impl AsRef<str>) -> ForgeError {
    ForgeError::FleetRegistryInvalid {
        reason: clean_field(reason.as_ref()),
    }
}

/// Echo the operator-declared registry path back for the report
/// `source`, made absolute when the filesystem allows it.
fn absolute_display(path: &Path) -> String {
    fs::canonicalize(path)
        .unwrap_or_else(|_| {
            if path.is_absolute() {
                path.to_path_buf()
            } else {
                std::env::current_dir()
                    .map(|cwd| cwd.join(path))
                    .unwrap_or_else(|_| path.to_path_buf())
            }
        })
        .display()
        .to_string()
}

/// Size-bounded read and JSON parse of the registry document.
fn read_document(path: &Path) -> Result<Value, ForgeError> {
    let metadata = fs::metadata(path).map_err(|err| {
        fleet_invalid(format!(
            "cannot read registry file {}: {err}",
            path.display()
        ))
    })?;
    if metadata.len() > MAX_REGISTRY_BYTES {
        return Err(fleet_invalid(format!(
            "registry file is {} bytes; max {MAX_REGISTRY_BYTES}",
            metadata.len()
        )));
    }
    let text = fs::read_to_string(path).map_err(|err| {
        fleet_invalid(format!(
            "cannot read registry file {}: {err}",
            path.display()
        ))
    })?;
    let value: Value = serde_json::from_str(&text).map_err(|err| {
        fleet_invalid(format!(
            "registry file {}: malformed JSON: {err}",
            path.display()
        ))
    })?;
    Ok(value)
}

/// Enforce the version contract and return the `projects` array view.
fn validated_document(document: &Value) -> Result<&Vec<Value>, ForgeError> {
    let root = document
        .as_object()
        .ok_or_else(|| fleet_invalid("registry document is not a JSON object"))?;
    match root.get("schema_version") {
        None => {
            return Err(fleet_invalid(
                "registry document is missing `schema_version`; only \
                 schema_version {SUPPORTED_SCHEMA_VERSION} is supported",
            ))
        }
        Some(Value::Number(version)) => {
            let accepted = version
                .as_u64()
                .is_some_and(|value| value == SUPPORTED_SCHEMA_VERSION);
            if !accepted {
                return Err(fleet_invalid(format!(
                    "unknown registry schema_version {version}; only schema_version \
                     {SUPPORTED_SCHEMA_VERSION} is supported"
                )));
            }
        }
        Some(other) => {
            return Err(fleet_invalid(format!(
                "unknown registry schema_version {other}; only schema_version \
                 {SUPPORTED_SCHEMA_VERSION} is supported"
            )));
        }
    }
    if let Some(discovery) = root.get("discovery") {
        if !discovery.is_object() {
            return Err(fleet_invalid(
                "`discovery` must be an object when present; refusing to guess \
                 the workspace layout",
            ));
        }
    }
    let projects = root
        .get("projects")
        .and_then(|value| value.as_array())
        .ok_or_else(|| fleet_invalid("`projects` is missing or is not an array"))?;
    if projects.len() > MAX_REGISTRY_ENTRIES {
        return Err(fleet_invalid(format!(
            "registry declares {} entries; max {MAX_REGISTRY_ENTRIES}",
            projects.len()
        )));
    }
    Ok(projects)
}

/// Project entries with per-entry isolation, then enforce the report-
/// level duplicate-id rule.
fn project_entries(
    registry_path: &Path,
    document: &Value,
    local_ids: &BTreeSet<String>,
) -> Result<(Vec<FleetEntry>, Vec<FleetMalformedEntry>), ForgeError> {
    let projects = validated_document(document)?;
    let root = confinement_root(registry_path)?;
    let mut entries: Vec<FleetEntry> = Vec::new();
    let mut malformed: Vec<FleetMalformedEntry> = Vec::new();
    let mut seen: BTreeSet<String> = BTreeSet::new();
    for (index, raw) in projects.iter().enumerate() {
        match project_entry(&root, raw, local_ids) {
            Ok(entry) => {
                if !seen.insert(entry.id.clone()) {
                    return Err(fleet_invalid(format!(
                        "duplicate fleet id `{}`; the report refuses rather than \
                         choosing one entry silently",
                        entry.id
                    )));
                }
                entries.push(entry);
            }
            Err(reason) => malformed.push(FleetMalformedEntry {
                name: malformed_name(raw, index),
                reason,
            }),
        }
    }
    Ok((entries, malformed))
}

/// A malformed entry is named by its declared id when that id is a
/// printable string, else by its position in the document.
fn malformed_name(raw: &Value, index: usize) -> String {
    raw.as_object()
        .and_then(|object| object.get("id"))
        .and_then(|value| value.as_str())
        .filter(|value| !value.trim().is_empty() && value.chars().count() <= MAX_FLEET_ID_CHARS)
        .map(clean_field)
        .unwrap_or_else(|| format!("entry #{index}"))
}

fn project_entry(
    root: &Path,
    raw: &Value,
    local_ids: &BTreeSet<String>,
) -> Result<FleetEntry, String> {
    let object = raw
        .as_object()
        .ok_or_else(|| "entry is not a JSON object".to_string())?;
    let id = required_string(object, "id")
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "entry `id` is missing or blank".to_string())?;
    if id.chars().count() > MAX_FLEET_ID_CHARS {
        return Err(format!(
            "entry id is {} chars; max {MAX_FLEET_ID_CHARS}",
            id.chars().count()
        ));
    }
    validate_project_id(&id).map_err(|err| err.to_string())?;
    let declared_path = required_string(object, "path")
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("entry `{id}` has a blank path"))?;
    if declared_path.chars().count() > MAX_FLEET_PATH_CHARS || declared_path.contains('\0') {
        return Err(format!(
            "entry `{id}` path is oversized or contains a NUL byte"
        ));
    }
    let profile = required_string(object, "profile")
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("entry `{id}` has no declared profile"))?;
    let lifecycle = required_string(object, "lifecycle")
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("entry `{id}` has no declared lifecycle"))?;
    let adoption = object
        .get("adoption")
        .and_then(|value| value.as_str())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let confined =
        confine_path(root, &declared_path).map_err(|reason| format!("entry `{id}` {reason}"))?;
    let forge_yaml_present = confined.join("forge.yaml").is_file();
    let locally_registered = local_ids.contains(&id);
    Ok(FleetEntry {
        id,
        path: declared_path,
        profile: clean_field(&profile),
        lifecycle: clean_field(&lifecycle),
        adoption: adoption.map(|value| clean_field(&value)),
        forge_yaml_present,
        locally_registered,
        state: if locally_registered {
            FleetState::Managed
        } else {
            FleetState::Unmanaged
        },
    })
}

fn required_string(object: &serde_json::Map<String, Value>, key: &str) -> Option<String> {
    object
        .get(key)
        .and_then(|value| value.as_str())
        .map(|value| value.to_string())
}

/// The confinement root is the canonicalized directory that contains the
/// registry document; the document's own `workspace_root` field is
/// tolerated but never relocates this root.
fn confinement_root(registry_path: &Path) -> Result<PathBuf, ForgeError> {
    let parent = registry_path.parent().unwrap_or_else(|| Path::new("."));
    fs::canonicalize(parent).map_err(|err| {
        fleet_invalid(format!(
            "cannot canonicalize the registry directory {}: {err}",
            parent.display()
        ))
    })
}

/// Resolve a declared path inside `root`, refusing traversal and
/// symlinked escapes. A missing directory is honest (it projects with
/// `forge_yaml_present: false`), not malformed.
fn confine_path(root: &Path, declared: &str) -> Result<PathBuf, String> {
    let candidate = Path::new(declared);
    if candidate.is_absolute() {
        return if candidate.starts_with(root) {
            Ok(candidate.to_path_buf())
        } else {
            Err("declares an absolute path outside the registry workspace root".to_string())
        };
    }
    let mut current = root.to_path_buf();
    for component in candidate.components() {
        match component {
            Component::Normal(segment) => {
                current.push(segment);
                let metadata = match fs::symlink_metadata(&current) {
                    Ok(metadata) => metadata,
                    Err(_) => continue,
                };
                if metadata.file_type().is_symlink() {
                    let resolved = current
                        .canonicalize()
                        .map_err(|_| "resolves through a dangling symlinked path".to_string())?;
                    if !resolved.starts_with(root) {
                        return Err(
                            "resolves through a symlinked path outside the registry root"
                                .to_string(),
                        );
                    }
                    current = resolved;
                }
            }
            Component::CurDir => {}
            Component::ParentDir => {
                current.pop();
                if !current.starts_with(root) {
                    return Err("traverses outside the registry workspace root".to_string());
                }
            }
            Component::RootDir | Component::Prefix(_) => {
                return Err("declares an absolute path outside the registry root".to_string());
            }
        }
    }
    if let Ok(real) = fs::canonicalize(&current) {
        if !real.starts_with(root) {
            return Err("canonicalizes outside the registry workspace root".to_string());
        }
        return Ok(real);
    }
    Ok(current)
}

/// Classify the document's mtime against the max-age window.
fn classify_freshness(
    path: &Path,
    max_age_seconds: i64,
    now: DateTime<Utc>,
) -> Result<(Option<i64>, FleetFreshness), ForgeError> {
    let metadata = fs::metadata(path).map_err(|err| {
        fleet_invalid(format!(
            "cannot stat registry file {}: {err}",
            path.display()
        ))
    })?;
    let modified = metadata.modified().map_err(|err| {
        fleet_invalid(format!(
            "registry file {} has no readable mtime: {err}",
            path.display()
        ))
    })?;
    let modified: DateTime<Utc> = modified.into();
    let age = now.signed_duration_since(modified).num_seconds().max(0);
    let freshness = if age > max_age_seconds {
        FleetFreshness::Stale
    } else {
        FleetFreshness::Fresh
    };
    Ok((Some(age), freshness))
}

/// Find one entry by id. A malformed entry is named with its exclusion
/// reason; a missing id refuses with the typed code.
pub fn inspect_entry<'a>(report: &'a FleetReport, id: &str) -> Result<&'a FleetEntry, ForgeError> {
    if let Some(entry) = report.entries.iter().find(|entry| entry.id == id) {
        return Ok(entry);
    }
    if let Some(malformed) = report.malformed.iter().find(|entry| entry.name == id) {
        return Err(fleet_invalid(format!(
            "fleet entry `{id}` was refused as malformed: {}",
            malformed.reason
        )));
    }
    Err(fleet_invalid(format!(
        "no fleet entry `{id}` in the declared registry ({} valid, {} malformed)",
        report.entries.len(),
        report.malformed.len()
    )))
}

/// Normalized per-entry fields shared by every read surface (CLI,
/// `forge list` block, portal) so the surfaces stay byte-equivalent.
pub fn entry_fields(entry: &FleetEntry) -> Vec<(&'static str, String)> {
    vec![
        ("path", entry.path.clone()),
        ("profile", entry.profile.clone()),
        ("lifecycle", entry.lifecycle.clone()),
        (
            "adoption",
            entry
                .adoption
                .clone()
                .unwrap_or_else(|| "unknown".to_string()),
        ),
        (
            "forge_yaml",
            if entry.forge_yaml_present {
                "present".to_string()
            } else {
                "missing".to_string()
            },
        ),
        ("state", entry.state.id().to_string()),
    ]
}

/// One normalized summary line for an entry, identical on every surface.
pub fn entry_summary_line(entry: &FleetEntry) -> String {
    let fields = entry_fields(entry)
        .into_iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join(" ");
    format!("fleet entry `{}`: {fields}", entry.id)
}

/// Redact and bound one display string so no credential shape or
/// oversized text survives.
fn clean_field(text: &str) -> String {
    let redacted = redact_credentials(text);
    if redacted.chars().count() <= MAX_FIELD_CHARS {
        return redacted;
    }
    let kept: String = redacted.chars().take(MAX_FIELD_CHARS - 3).collect();
    format!("{kept}...")
}

/// Header line naming the source, freshness and counts. `forge list`
/// and `forge fleet list` share it for parity.
pub fn source_line(report: &FleetReport) -> String {
    match &report.source {
        None => "fleet registry: unconfigured (set --workspace-registry or \
                 FORGE_WORKSPACE_REGISTRY); local workflows continue unaffected"
            .to_string(),
        Some(source) => format!(
            "fleet registry: source={source} freshness={} observed_at={} entries={} \
             malformed={} max_age={}s age={}",
            report.freshness.id(),
            report.observed_at,
            report.entries.len(),
            report.malformed.len(),
            report.max_age_seconds,
            report
                .age_seconds
                .map(|age| format!("{age}s"))
                .unwrap_or_else(|| "unknown".to_string()),
        ),
    }
}

/// Render the full `forge fleet list` report for humans.
pub fn render_report_human(report: &FleetReport) -> String {
    let mut out = format!("forge fleet list — contract {}\n", report.contract);
    out.push_str(&source_line(report));
    out.push('\n');
    if report.entries.is_empty() {
        out.push_str("entries: (none)\n");
    } else {
        for entry in &report.entries {
            out.push_str(&format!("  {}\n", entry_summary_line(entry)));
        }
    }
    if !report.malformed.is_empty() {
        out.push_str("malformed entries (excluded, named with reason):\n");
        for entry in &report.malformed {
            out.push_str(&format!(
                "  - name={} reason={}\n",
                entry.name, entry.reason
            ));
        }
    }
    if report.source.is_some() {
        out.push_str("read-only: fleet mirroring never registers, imports or mutates entries\n");
    }
    out
}

/// Render the `forge fleet status` registry-health view (no entries).
pub fn render_status_human(report: &FleetReport) -> String {
    let mut out = format!("forge fleet status — contract {}\n", report.contract);
    out.push_str(&source_line(report));
    out.push('\n');
    out.push_str(&format!(
        "declared: {} ({}/{} valid/malformed)\n",
        report.declared_count(),
        report.entries.len(),
        report.malformed.len()
    ));
    match report.freshness {
        FleetFreshness::Fresh => out.push_str("health: ok (registry within the max-age window)\n"),
        FleetFreshness::Stale => {
            out.push_str("health: stale (re-discover the registry; never rendered as healthy)\n")
        }
        FleetFreshness::Unconfigured => {
            out.push_str("health: unconfigured (no fleet source was contacted)\n")
        }
    }
    out
}

/// Render one entry for `forge fleet inspect <ID>`.
pub fn render_entry_human(report: &FleetReport, entry: &FleetEntry) -> String {
    let mut out = format!("forge fleet inspect — contract {}\n", report.contract);
    out.push_str(&entry_summary_line(entry));
    out.push('\n');
    out.push_str(&format!(
        "  id={} locally_registered={} state={}\n",
        entry.id,
        entry.locally_registered,
        entry.state.id()
    ));
    out.push_str(&format!(
        "  source={} freshness={} observed_at={}\n",
        report.source.as_deref().unwrap_or("unconfigured"),
        report.freshness.id(),
        report.observed_at
    ));
    out.push_str(
        "read-only: unmanaged entries can only be inspected, never adopted by mirroring\n",
    );
    out
}

/// Render the compact fleet block appended to `forge list` when a
/// registry is configured.
pub fn render_list_block_human(report: &FleetReport) -> String {
    let mut out = String::new();
    out.push('\n');
    out.push_str(&source_line(report));
    out.push('\n');
    for entry in &report.entries {
        out.push_str(&format!("  {}\n", entry_summary_line(entry)));
    }
    for entry in &report.malformed {
        out.push_str(&format!(
            "  malformed: name={} reason={}\n",
            entry.name, entry.reason
        ));
    }
    out
}

/// Registry-health JSON for `forge fleet status`.
pub fn health_json(report: &FleetReport) -> Value {
    serde_json::json!({
        "source": report.source,
        "observed_at": report.observed_at,
        "freshness": report.freshness.id(),
        "max_age_seconds": report.max_age_seconds,
        "age_seconds": report.age_seconds,
        "declared_count": report.declared_count(),
        "entries": report.entries.len(),
        "malformed": report.malformed.len(),
        "malformed_entries": report.malformed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn now() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-09-24T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc)
    }

    fn ids(values: &[&str]) -> BTreeSet<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    /// Write a registry document into a fixture tree that mirrors the
    /// real Workspace Governance shape.
    fn workspace(parent: &Path) -> PathBuf {
        let root = parent.join("ws");
        let registry_dir = root.join("workspace-governance");
        fs::create_dir_all(registry_dir.join("alpha")).unwrap();
        fs::create_dir_all(registry_dir.join("beta")).unwrap();
        fs::create_dir_all(registry_dir.join("gamma")).unwrap();
        fs::write(registry_dir.join("alpha/forge.yaml"), "schema: 1\n").unwrap();
        fs::write(registry_dir.join("alpha/README.md"), "# alpha\n").unwrap();
        fs::write(registry_dir.join("beta/forge.yaml"), "schema: 1\n").unwrap();
        registry_dir
    }

    fn write_registry(dir: &Path, body: &str) -> PathBuf {
        let path = dir.join("projects.json");
        fs::write(&path, body).unwrap();
        path
    }

    fn clean_body() -> String {
        r#"{
  "schema_version": 1,
  "workspace_root": null,
  "discovery": {
    "mode": "explicit-plus-immediate-directories",
    "unregistered_policy": "report",
    "exclude": ["build", "dist", "node_modules", "target", "vendor"]
  },
  "generated_by": "workspace-governance/scripts/whatever.py",
  "projects": [
    {"id": "alpha", "path": "alpha", "profile": "rust-product", "lifecycle": "active", "adoption": "adopted"},
    {"id": "beta", "path": "beta", "profile": "dotnet-library", "lifecycle": "planning", "adoption": null},
    {"id": "gamma", "path": "gamma", "profile": "typescript-monorepo", "lifecycle": "active"}
  ]
}"#
        .to_string()
    }

    #[test]
    fn parses_the_real_wg_shape_and_tolerates_unknown_fields() {
        let tmp = TempDir::new().unwrap();
        let dir = workspace(tmp.path());
        let registry = write_registry(&dir, &clean_body());
        let report = observe_at(
            Some(&registry),
            DEFAULT_MAX_AGE_SECONDS,
            &ids(&["beta"]),
            now(),
        )
        .unwrap();
        assert_eq!(report.contract, FLEET_CONTRACT_VERSION);
        assert_eq!(report.freshness, FleetFreshness::Fresh);
        assert_eq!(report.entries.len(), 3);
        assert!(report.malformed.is_empty());
        let alpha = &report.entries[0];
        assert_eq!(alpha.id, "alpha");
        assert_eq!(alpha.path, "alpha");
        assert_eq!(alpha.profile, "rust-product");
        assert_eq!(alpha.lifecycle, "active");
        assert_eq!(alpha.adoption.as_deref(), Some("adopted"));
        assert!(alpha.forge_yaml_present);
        assert_eq!(alpha.state, FleetState::Unmanaged);
        let beta = &report.entries[1];
        assert!(beta.forge_yaml_present);
        assert!(beta.adoption.is_none(), "null adoption projects as none");
        assert_eq!(beta.state, FleetState::Managed, "joined by id only");
        assert!(beta.locally_registered);
        let gamma = &report.entries[2];
        assert!(!gamma.forge_yaml_present);
        assert!(gamma.adoption.is_none());
        assert!(report.source.as_ref().unwrap().ends_with("projects.json"));
        assert!(report.age_seconds.is_some());
    }

    #[test]
    fn profiles_are_surfaced_verbatim_never_coerced() {
        let tmp = TempDir::new().unwrap();
        let dir = workspace(tmp.path());
        let registry = write_registry(&dir, &clean_body());
        let report =
            observe_at(Some(&registry), DEFAULT_MAX_AGE_SECONDS, &ids(&[]), now()).unwrap();
        let profiles: Vec<&str> = report.entries.iter().map(|e| e.profile.as_str()).collect();
        assert_eq!(
            profiles,
            vec!["rust-product", "dotnet-library", "typescript-monorepo"]
        );
    }

    #[test]
    fn unknown_schema_version_refuses_the_report() {
        let tmp = TempDir::new().unwrap();
        let dir = workspace(tmp.path());
        let registry = write_registry(
            &dir,
            &clean_body().replace("\"schema_version\": 1", "\"schema_version\": 2"),
        );
        let err =
            observe_at(Some(&registry), DEFAULT_MAX_AGE_SECONDS, &ids(&[]), now()).unwrap_err();
        assert_eq!(err.code(), "fleet-registry-invalid");
        assert!(err.to_string().contains("schema_version"), "{err}");
    }

    #[test]
    fn missing_schema_version_and_malformed_json_refuse() {
        let tmp = TempDir::new().unwrap();
        let dir = workspace(tmp.path());
        let registry = write_registry(&dir, r#"{"projects": []}"#);
        let err =
            observe_at(Some(&registry), DEFAULT_MAX_AGE_SECONDS, &ids(&[]), now()).unwrap_err();
        assert_eq!(err.code(), "fleet-registry-invalid");
        let registry = write_registry(&dir, "{not json");
        let err =
            observe_at(Some(&registry), DEFAULT_MAX_AGE_SECONDS, &ids(&[]), now()).unwrap_err();
        assert_eq!(err.code(), "fleet-registry-invalid");
        assert!(err.to_string().contains("malformed JSON"), "{err}");
    }

    #[test]
    fn duplicate_ids_refuse_the_report_naming_the_id() {
        let tmp = TempDir::new().unwrap();
        let dir = workspace(tmp.path());
        let registry = write_registry(
            &dir,
            r#"{"schema_version": 1, "workspace_root": null, "projects": [
  {"id": "dup", "path": "alpha", "profile": "product", "lifecycle": "active"},
  {"id": "dup", "path": "beta", "profile": "product", "lifecycle": "active"}
]}"#,
        );
        let err =
            observe_at(Some(&registry), DEFAULT_MAX_AGE_SECONDS, &ids(&[]), now()).unwrap_err();
        assert_eq!(err.code(), "fleet-registry-invalid");
        assert!(err.to_string().contains("`dup`"), "{err}");
    }

    #[test]
    fn traversing_entry_is_malformed_and_the_rest_still_reports() {
        let tmp = TempDir::new().unwrap();
        let dir = workspace(tmp.path());
        let registry = write_registry(
            &dir,
            r#"{"schema_version": 1, "projects": [
  {"id": "runner", "path": "../outside-root", "profile": "product", "lifecycle": "active"},
  {"id": "alpha", "path": "alpha", "profile": "rust-product", "lifecycle": "active"}
]}"#,
        );
        let report =
            observe_at(Some(&registry), DEFAULT_MAX_AGE_SECONDS, &ids(&[]), now()).unwrap();
        assert_eq!(report.entries.len(), 1);
        assert_eq!(report.entries[0].id, "alpha");
        assert_eq!(report.malformed.len(), 1);
        assert_eq!(report.malformed[0].name, "runner");
        assert!(
            report.malformed[0].reason.contains("outside"),
            "{:?}",
            report.malformed[0]
        );
    }

    #[test]
    fn internal_parent_segments_stay_confined() {
        let tmp = TempDir::new().unwrap();
        let dir = workspace(tmp.path());
        fs::create_dir_all(dir.join("packages/inner")).unwrap();
        let registry = write_registry(
            &dir,
            r#"{"schema_version": 1, "projects": [
  {"id": "inner", "path": "packages/../packages/inner", "profile": "product", "lifecycle": "active"}
]}"#,
        );
        let report =
            observe_at(Some(&registry), DEFAULT_MAX_AGE_SECONDS, &ids(&[]), now()).unwrap();
        assert!(report.malformed.is_empty(), "{:?}", report.malformed);
        assert_eq!(report.entries.len(), 1);
    }

    #[test]
    fn symlinked_escape_is_malformed() {
        let tmp = TempDir::new().unwrap();
        let dir = workspace(tmp.path());
        let outside = tmp.path().join("outside-target");
        fs::create_dir_all(&outside).unwrap();
        fs::write(outside.join("forge.yaml"), "schema: 1\n").unwrap();
        std::os::unix::fs::symlink(&outside, dir.join("sneaky")).unwrap();
        let registry = write_registry(
            &dir,
            r#"{"schema_version": 1, "projects": [
  {"id": "sneaky", "path": "sneaky", "profile": "product", "lifecycle": "active"},
  {"id": "alpha", "path": "alpha", "profile": "product", "lifecycle": "active"}
]}"#,
        );
        let report =
            observe_at(Some(&registry), DEFAULT_MAX_AGE_SECONDS, &ids(&[]), now()).unwrap();
        assert_eq!(report.entries.len(), 1);
        assert_eq!(report.malformed[0].name, "sneaky");
        assert!(
            report.malformed[0].reason.contains("symlink"),
            "{:?}",
            report.malformed[0]
        );
    }

    #[test]
    fn absolute_entry_path_inside_root_is_accepted() {
        let tmp = TempDir::new().unwrap();
        let dir = workspace(tmp.path());
        let absolute = format!(r#""path": "{}""#, dir.join("alpha").display());
        let body = clean_body().replace(r#""path": "alpha""#, &absolute);
        let registry = write_registry(&dir, &body);
        let report =
            observe_at(Some(&registry), DEFAULT_MAX_AGE_SECONDS, &ids(&[]), now()).unwrap();
        assert_eq!(report.entries.len(), 3);
        assert!(report
            .entries
            .iter()
            .any(|e| e.id == "alpha" && e.forge_yaml_present));
        assert!(report.malformed.is_empty());
    }

    #[test]
    fn blank_and_oversized_entry_fields_are_malformed_isolated() {
        let tmp = TempDir::new().unwrap();
        let dir = workspace(tmp.path());
        let registry = write_registry(
            &dir,
            r#"{"schema_version": 1, "projects": [
  {"id": "no-path", "path": "", "profile": "product", "lifecycle": "active"},
  {"id": "no-profile", "path": "alpha", "lifecycle": "active"},
  {"id": "Bad ID", "path": "alpha", "profile": "product", "lifecycle": "active"},
  {"id": "no-lifecycle", "path": "alpha", "profile": "product"},
  {"id": "alpha", "path": "alpha", "profile": "rust-product", "lifecycle": "active"}
]}"#,
        );
        let report =
            observe_at(Some(&registry), DEFAULT_MAX_AGE_SECONDS, &ids(&[]), now()).unwrap();
        assert_eq!(report.entries.len(), 1, "only the clean entry survives");
        assert_eq!(report.malformed.len(), 4);
        let names: Vec<&str> = report.malformed.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(
            names,
            vec!["no-path", "no-profile", "Bad ID", "no-lifecycle"]
        );
    }

    #[test]
    fn freshness_classifies_from_the_document_mtime() {
        let tmp = TempDir::new().unwrap();
        let dir = workspace(tmp.path());
        let registry = write_registry(&dir, &clean_body());
        let (age, freshness) = classify_freshness(&registry, 60, Utc::now()).unwrap();
        assert_eq!(freshness, FleetFreshness::Fresh, "age {age:?}");
        // Move the mtime 10 days into the past through the filesystem.
        let old = Utc::now() - chrono::Duration::days(10);
        let file_time = std::time::SystemTime::UNIX_EPOCH
            + std::time::Duration::from_secs(old.timestamp().max(0) as u64);
        let file = fs::OpenOptions::new().write(true).open(&registry).unwrap();
        file.set_times(fs::FileTimes::new().set_modified(file_time))
            .unwrap();
        drop(file);
        let (age, freshness) = classify_freshness(&registry, 60, Utc::now()).unwrap();
        assert_eq!(freshness, FleetFreshness::Stale);
        assert!(age.unwrap() > 60);
    }

    #[test]
    fn unconfigured_when_no_registry_path_is_given() {
        let report = observe_at(None, DEFAULT_MAX_AGE_SECONDS, &ids(&[]), now()).unwrap();
        assert_eq!(report.freshness, FleetFreshness::Unconfigured);
        assert!(report.source.is_none());
        assert!(report.entries.is_empty());
        assert!(report.malformed.is_empty());
        let text = render_status_human(&report);
        assert!(text.contains("unconfigured"), "{text}");
        assert!(
            text.contains("local workflows continue unaffected")
                || text.contains("no fleet source"),
            "{text}"
        );
    }

    #[test]
    fn oversized_document_refuses() {
        let tmp = TempDir::new().unwrap();
        let dir = workspace(tmp.path());
        let fat = format!(
            r#"{{"schema_version": 1, "projects": [{{"id": "alpha", "path": "alpha", "profile": "{}", "lifecycle": "active"}}]}}"#,
            "p".repeat(2_000_000)
        );
        let registry = write_registry(&dir, &fat);
        let err =
            observe_at(Some(&registry), DEFAULT_MAX_AGE_SECONDS, &ids(&[]), now()).unwrap_err();
        assert_eq!(err.code(), "fleet-registry-invalid");
        assert!(err.to_string().contains("bytes"), "{err}");
    }

    #[test]
    fn entry_count_over_the_bound_refuses() {
        let tmp = TempDir::new().unwrap();
        let dir = workspace(tmp.path());
        let entries: Vec<String> = (0..=MAX_REGISTRY_ENTRIES)
            .map(|i| {
                format!(r#"{{"id": "p{i}", "path": "alpha", "profile": "x", "lifecycle": "y"}}"#)
            })
            .collect();
        let registry = write_registry(
            &dir,
            &format!(
                r#"{{"schema_version": 1, "projects": [{}]}}"#,
                entries.join(",")
            ),
        );
        let err =
            observe_at(Some(&registry), DEFAULT_MAX_AGE_SECONDS, &ids(&[]), now()).unwrap_err();
        assert_eq!(err.code(), "fleet-registry-invalid");
        assert!(err.to_string().contains("entries"), "{err}");
    }

    #[test]
    fn credential_shaped_fields_are_redacted_and_bounded() {
        let tmp = TempDir::new().unwrap();
        let dir = workspace(tmp.path());
        let registry = write_registry(
            &dir,
            &format!(
                r#"{{"schema_version": 1, "projects": [
  {{ "id": "leak", "path": "alpha", "profile": "ghp_abcdefghijklmnopqrstuvwxyz0123456789", "lifecycle": "{}" }}
]}}"#,
                "l".repeat(2_000)
            ),
        );
        let report =
            observe_at(Some(&registry), DEFAULT_MAX_AGE_SECONDS, &ids(&[]), now()).unwrap();
        let entry = report.entries.iter().find(|e| e.id == "leak").unwrap();
        assert!(!entry.profile.contains("ghp_abcdef"), "{}", entry.profile);
        assert!(entry.profile.contains("[REDACTED]"), "{}", entry.profile);
        assert!(entry.lifecycle.chars().count() <= MAX_FIELD_CHARS);
        assert!(entry.lifecycle.ends_with("..."));
    }

    #[test]
    fn max_age_window_is_bounded() {
        assert_eq!(
            validate_max_age(0).unwrap_err().code(),
            "fleet-registry-invalid"
        );
        assert_eq!(
            validate_max_age(MAX_MAX_AGE_SECONDS + 1)
                .unwrap_err()
                .code(),
            "fleet-registry-invalid"
        );
        assert!(validate_max_age(DEFAULT_MAX_AGE_SECONDS).is_ok());
    }

    #[test]
    fn reports_are_deterministic_except_for_observed_at() {
        let tmp = TempDir::new().unwrap();
        let dir = workspace(tmp.path());
        let registry = write_registry(&dir, &clean_body());
        // Pin the document mtime to a fixed instant 10s before the
        // observation clock below; a wall-clock mtime would make the age
        // assertions depend on the run time (and fail once the real clock
        // passes the fixed observations), so the filesystem — not the
        // clock — carries the determinism.
        let mtime = std::time::SystemTime::UNIX_EPOCH
            + std::time::Duration::from_secs(
                (now() - chrono::Duration::seconds(10)).timestamp().max(0) as u64,
            );
        let file = fs::OpenOptions::new().write(true).open(&registry).unwrap();
        file.set_times(fs::FileTimes::new().set_modified(mtime))
            .unwrap();
        drop(file);
        let earlier = now();
        let later = earlier + chrono::Duration::seconds(60);
        let a = observe_at(Some(&registry), DEFAULT_MAX_AGE_SECONDS, &ids(&[]), earlier).unwrap();
        let b = observe_at(Some(&registry), DEFAULT_MAX_AGE_SECONDS, &ids(&[]), later).unwrap();
        assert_eq!(a.entries, b.entries);
        assert_eq!(a.malformed, b.malformed);
        assert_eq!(a.source, b.source);
        assert_ne!(a.observed_at, b.observed_at);
        assert_eq!(a.age_seconds, Some(10));
        assert_eq!(b.age_seconds, Some(70));
    }

    #[test]
    fn inspect_entry_covers_found_malformed_and_missing() {
        let tmp = TempDir::new().unwrap();
        let dir = workspace(tmp.path());
        let registry = write_registry(
            &dir,
            r#"{"schema_version": 1, "projects": [
  {"id": "runner", "path": "../outside-root", "profile": "product", "lifecycle": "active"},
  {"id": "alpha", "path": "alpha", "profile": "rust-product", "lifecycle": "active"}
]}"#,
        );
        let report =
            observe_at(Some(&registry), DEFAULT_MAX_AGE_SECONDS, &ids(&[]), now()).unwrap();
        let entry = inspect_entry(&report, "alpha").unwrap();
        assert_eq!(entry.id, "alpha");
        let err = inspect_entry(&report, "runner").unwrap_err();
        assert_eq!(err.code(), "fleet-registry-invalid");
        assert!(err.to_string().contains("malformed"), "{err}");
        let err = inspect_entry(&report, "ghost").unwrap_err();
        assert_eq!(err.code(), "fleet-registry-invalid");
        assert!(err.to_string().contains("ghost"), "{err}");
    }

    #[test]
    fn entry_fields_are_the_shared_normalized_projection() {
        let entry = FleetEntry {
            id: "alpha".to_string(),
            path: "alpha".to_string(),
            profile: "rust-product".to_string(),
            lifecycle: "active".to_string(),
            adoption: Some("adopted".to_string()),
            forge_yaml_present: true,
            locally_registered: false,
            state: FleetState::Unmanaged,
        };
        let fields = entry_fields(&entry);
        let map: BTreeSet<String> = fields
            .iter()
            .map(|(key, value)| format!("{key}={value}"))
            .collect();
        for needle in [
            "path=alpha",
            "profile=rust-product",
            "lifecycle=active",
            "adoption=adopted",
            "forge_yaml=present",
            "state=unmanaged",
        ] {
            assert!(map.contains(needle), "{needle} missing in {map:?}");
        }
        let line = entry_summary_line(&entry);
        assert!(line.starts_with("fleet entry `alpha`: "), "{line}");
        assert!(line.contains("forge_yaml=present"), "{line}");
    }

    #[test]
    fn human_renderers_carry_the_required_fields() {
        let tmp = TempDir::new().unwrap();
        let dir = workspace(tmp.path());
        let registry = write_registry(&dir, &clean_body());
        let report = observe_at(
            Some(&registry),
            DEFAULT_MAX_AGE_SECONDS,
            &ids(&["alpha"]),
            now(),
        )
        .unwrap();
        let list = render_report_human(&report);
        for needle in [
            FLEET_CONTRACT_VERSION,
            "freshness=fresh",
            "observed_at=",
            "alpha",
            "rust-product",
            "forge_yaml=present",
            "state=managed",
            "read-only",
        ] {
            assert!(list.contains(needle), "{needle} missing in {list}");
        }
        let status = render_status_human(&report);
        for needle in [FLEET_CONTRACT_VERSION, "health: ok", "max_age="] {
            assert!(status.contains(needle), "{needle} missing in {status}");
        }
        let entry = inspect_entry(&report, "alpha").unwrap();
        let inspect = render_entry_human(&report, entry);
        for needle in [
            FLEET_CONTRACT_VERSION,
            "id=alpha",
            "locally_registered=true",
            "state=managed",
        ] {
            assert!(inspect.contains(needle), "{needle} missing in {inspect}");
        }
        let block = render_list_block_human(&report);
        assert!(block.contains("fleet registry:"), "{block}");
        assert!(block.contains("fleet entry `alpha`"), "{block}");
    }

    #[test]
    fn malformed_entries_render_with_reasons_and_never_silently_drop() {
        let tmp = TempDir::new().unwrap();
        let dir = workspace(tmp.path());
        let registry = write_registry(
            &dir,
            r#"{"schema_version": 1, "projects": [
  {"id": "runner", "path": "../../etc", "profile": "product", "lifecycle": "active"}
]}"#,
        );
        let report =
            observe_at(Some(&registry), DEFAULT_MAX_AGE_SECONDS, &ids(&[]), now()).unwrap();
        assert_eq!(report.declared_count(), 1);
        assert!(report.entries.is_empty());
        let text = render_report_human(&report);
        assert!(text.contains("malformed"), "{text}");
        assert!(text.contains("runner"), "{text}");
        let health = health_json(&report);
        assert_eq!(health["malformed"], serde_json::json!(1));
        assert_eq!(health["declared_count"], serde_json::json!(1));
    }

    #[test]
    fn resolve_registry_path_prefers_the_flag_then_the_env() {
        // The env var is process-global; test the two orders without
        // racing other tests by restoring it afterwards.
        std::env::remove_var(WORKSPACE_REGISTRY_ENV);
        assert_eq!(resolve_registry_path(None), None);
        let explicit = PathBuf::from("/flag/projects.json");
        assert_eq!(
            resolve_registry_path(Some(&explicit)),
            Some(explicit.clone())
        );
        std::env::set_var(WORKSPACE_REGISTRY_ENV, "/env/projects.json");
        assert_eq!(
            resolve_registry_path(None),
            Some(PathBuf::from("/env/projects.json"))
        );
        assert_eq!(resolve_registry_path(Some(&explicit)), Some(explicit));
        std::env::set_var(WORKSPACE_REGISTRY_ENV, "   ");
        assert_eq!(resolve_registry_path(None), None);
        std::env::remove_var(WORKSPACE_REGISTRY_ENV);
    }
}
