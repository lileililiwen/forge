//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use crate::core::{validate_project_id, ForgeError};
use crate::policy::redact_credentials;
use chrono::{DateTime, SecondsFormat, Utc};
use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Component, Path, PathBuf};

use super::limits::{
    FLEET_CONTRACT_VERSION, MAX_FIELD_CHARS, MAX_FLEET_ID_CHARS, MAX_FLEET_PATH_CHARS,
    MAX_MAX_AGE_SECONDS, MAX_REGISTRY_BYTES, MAX_REGISTRY_ENTRIES, MIN_MAX_AGE_SECONDS,
    SUPPORTED_SCHEMA_VERSION, WORKSPACE_REGISTRY_ENV,
};
use super::model::{FleetEntry, FleetFreshness, FleetMalformedEntry, FleetReport, FleetState};

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
pub(super) fn classify_freshness(
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
