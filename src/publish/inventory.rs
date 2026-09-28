//! Portable project inventory and container fleet selection
//! (`forge-independent-project-inventory-fleet`).
//!
//! Forge consumes a versioned normalized project inventory from an
//! explicitly selected local file or external adapter; the
//! inventory is never silently derived from a sibling checkout. The
//! inventory carries every project, including ones that lack a
//! Compose file — `compose_missing`, `invalid`, and
//! `source_unavailable` are reported per project instead of being
//! filtered out. Only `compose_ready` entries start a Mac Docker
//! workload through the existing provider transport.
//!
//! ## Contract (`forge-project-inventory/0.1.0`)
//!
//! ```json
//! {
//!   "contract": "forge-project-inventory/0.1.0",
//!   "provider": "local",
//!   "generated_at": "RFC3339",
//!   "projects": [{
//!     "id": "example",
//!     "repository": "https://github.com/org/example.git",
//!     "revision": "40-hex-sha",
//!     "profile": "rust-product",
//!     "runtime": "web|worker|job|library",
//!     "compose_file": "docker-compose.yml",
//!     "source_path": "/invocation/only/example",
//!     "public_http": true,
//!     "public_port": 8080
//!   }]
//! }
//! ```
//!
//! `source_path` is never persisted as the project identity; it
//! resolves the staged source tree when Forge needs to compose it
//! through a local provider.
//!
//! ## Fleet classification
//!
//! Every declared project receives one explicit classification:
//!
//! - `compose_ready`     — `compose_file` exists at `source_path`.
//! - `compose_missing`   — `compose_file` is null or absent.
//! - `invalid`           — contract validation refused the entry.
//! - `source_unavailable`— `source_path` does not resolve on this host.
//!
//! `forge publish fleet` reports every entry; only `compose_ready`
//! entries invoke the provider, never the others. Missing Compose
//! and unresolvable sources are explicit fleet outcomes, never
//! silent omissions.
//!
//! ## Routing
//!
//! Each project carries `runtime` and optional `public_http` plus
//! `public_port`. Web runtimes may declare a public HTTP port and
//! receive `<project>.<domain>` routing through the existing
//! Caddy/cloudflare wildcard boundary on the Mac; non-web runtimes
//! run privately regardless of any other flag; databases,
//! Redis, Jenkins and explicit private workers never reach the
//! router. Forge owns the decision; the jenkins-local sibling owns
//! the Caddy renderer and the tunnel refresh.

use std::fmt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::core::ForgeError;
use crate::policy::redact_credentials;
use crate::publish::providers::validate_revision;

/// Versioned contract for the project-inventory surface.
pub const INVENTORY_CONTRACT_VERSION: &str = "forge-project-inventory/0.1.0";

/// Environment variable naming the inventory source when the CLI
/// flag is absent. Defaults to local-only — no implicit sibling
/// search.
pub const INVENTORY_SOURCE_ENV: &str = "FORGE_INVENTORY_SOURCE";

/// Bounded maximum project entries accepted by one inventory
/// document. Larger documents refuse as invalid so a runaway
/// adapter cannot pin the reader.
pub const MAX_INVENTORY_ENTRIES: usize = 1_024;

/// Bounded maximum document size in bytes.
pub const MAX_INVENTORY_BYTES: u64 = 4_096_000;

/// Bounded inventory adapter subprocess timeout (seconds). Generous
/// enough for a remote clone, bounded so a hostile adapter cannot
/// pin Forge indefinitely.
pub const INVENTORY_ADAPTER_TIMEOUT_SECS: u64 = 300;

/// Maximum length of an id or runtime class string.
pub const MAX_ID_CHARS: usize = 128;
pub const MAX_RUNTIME_CHARS: usize = 32;

/// Per-field display bound after redaction.
pub const MAX_FIELD_CHARS: usize = 200;

/// Public domain used for `<project>.<domain>` routing when the
/// inventory declares `public_http = true`.
pub const DEFAULT_DOMAIN: &str = "tooosall.uk";

/// The runtime class of a project. The vocabulary is intentionally
/// narrow: only these four strings are accepted; any other value
/// surfaces as `invalid`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeClass {
    Web,
    Worker,
    Job,
    Library,
}

impl RuntimeClass {
    pub fn as_str(self) -> &'static str {
        match self {
            RuntimeClass::Web => "web",
            RuntimeClass::Worker => "worker",
            RuntimeClass::Job => "job",
            RuntimeClass::Library => "library",
        }
    }
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "web" => Some(RuntimeClass::Web),
            "worker" => Some(RuntimeClass::Worker),
            "job" => Some(RuntimeClass::Job),
            "library" => Some(RuntimeClass::Library),
            _ => None,
        }
    }
}

impl fmt::Display for RuntimeClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One inventory entry, normalized from the document. Fields that
/// did not parse are captured in [`InventoryMalformedEntry`]; this
/// struct only carries the validated shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InventoryEntry {
    pub id: String,
    pub repository: String,
    pub revision: String,
    pub profile: String,
    pub runtime: RuntimeClass,
    /// Compose file name relative to `source_path` (typically
    /// `docker-compose.yml`). `None` means the entry is reported
    /// as `compose_missing`; it is never silently dropped.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compose_file: Option<String>,
    /// Optional local source path; not used as a project identity.
    /// `None` for a remote Git source.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_path: Option<String>,
    /// Whether this project declares a public HTTP service. Only
    /// honoured when `runtime == Web`.
    #[serde(default)]
    pub public_http: bool,
    /// Public HTTP port the router forwards. Only honoured when
    /// `public_http = true`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub public_port: Option<u16>,
}

impl InventoryEntry {
    /// The subdomain this entry receives when it is `compose_ready`
    /// and `public_http`. Empty string for non-public services.
    pub fn subdomain(&self, domain: &str) -> Option<String> {
        if self.public_http && matches!(self.runtime, RuntimeClass::Web) {
            Some(format!("{}.{}", self.id, domain))
        } else {
            None
        }
    }
}

/// One entry the document declared but the parser refused. The
/// entry is named with its declared id when that id is printable;
/// otherwise by its position in the document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InventoryMalformedEntry {
    pub name: String,
    pub reason: String,
}

/// The normalized inventory snapshot returned by every source
/// (local file or external adapter). Stable across sources so the
/// fleet summary can render the same shape regardless of provider.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InventorySnapshot {
    pub contract: String,
    pub provider: String,
    pub generated_at: String,
    pub source: Option<String>,
    pub projects: Vec<InventoryEntry>,
    pub malformed: Vec<InventoryMalformedEntry>,
}

impl InventorySnapshot {
    pub fn declared_count(&self) -> usize {
        self.projects.len() + self.malformed.len()
    }
}

/// Per-project fleet classification. The four values are the
/// stable vocab used in `forge publish fleet` output and journal
/// rows; never widen without updating renderers and tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InventoryClassification {
    /// Compose file present at the declared source path.
    ComposeReady,
    /// Compose file absent or null — entry reported, not invoked.
    ComposeMissing,
    /// Contract validation refused the entry — reason in the
    /// accompanying [`InventoryMalformedEntry`].
    Invalid,
    /// Declared source path does not resolve on this host.
    SourceUnavailable,
}

impl InventoryClassification {
    pub fn as_str(self) -> &'static str {
        match self {
            InventoryClassification::ComposeReady => "compose_ready",
            InventoryClassification::ComposeMissing => "compose_missing",
            InventoryClassification::Invalid => "invalid",
            InventoryClassification::SourceUnavailable => "source_unavailable",
        }
    }
}

impl fmt::Display for InventoryClassification {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One fleet outcome. Compose-ready entries carry the resolved
/// Compose file path; the other classes carry enough metadata for
/// the operator to understand what was reported.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InventoryFleetEntry {
    pub id: String,
    pub runtime: RuntimeClass,
    pub profile: String,
    pub revision: String,
    pub classification: InventoryClassification,
    pub compose_file: Option<String>,
    pub source_path: Option<String>,
    pub public_http: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub public_port: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subdomain: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// The full fleet classification result. Every declared entry is
/// present exactly once; only `compose_ready` entries are eligible
/// for provider invocation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InventoryFleetReport {
    pub contract: String,
    pub provider: String,
    pub source: Option<String>,
    pub generated_at: String,
    pub domain: String,
    pub entries: Vec<InventoryFleetEntry>,
}

impl InventoryFleetReport {
    pub fn compose_ready(&self) -> impl Iterator<Item = &InventoryFleetEntry> {
        self.entries
            .iter()
            .filter(|e| e.classification == InventoryClassification::ComposeReady)
    }
}

/// Resolve the inventory source path. `None` means the inventory is
/// unconfigured; the caller is responsible for refusing rather
/// than silently scanning a sibling checkout.
pub fn resolve_source(cli_flag: Option<&str>) -> Option<PathBuf> {
    if let Some(value) = cli_flag {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return Some(PathBuf::from(trimmed));
        }
    }
    std::env::var(INVENTORY_SOURCE_ENV)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
}

/// RFC 3339 timestamp used as `generated_at`.
pub fn now_rfc3339() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}

/// Local-file inventory source. The file must parse as JSON,
/// conform to `forge-project-inventory/0.1.0`, and carry every
/// declared entry. This provider never searches the filesystem
/// beyond the path supplied.
pub fn load_local(path: &Path) -> Result<InventorySnapshot, ForgeError> {
    let metadata = std::fs::metadata(path).map_err(|err| {
        inventory_invalid(format!(
            "cannot read inventory file {}: {err}",
            path.display()
        ))
    })?;
    if metadata.len() > MAX_INVENTORY_BYTES {
        return Err(inventory_invalid(format!(
            "inventory file is {} bytes; max {MAX_INVENTORY_BYTES}",
            metadata.len()
        )));
    }
    let text = std::fs::read_to_string(path).map_err(|err| {
        inventory_invalid(format!(
            "cannot read inventory file {}: {err}",
            path.display()
        ))
    })?;
    let value: Value = serde_json::from_str(&text).map_err(|err| {
        inventory_invalid(format!(
            "inventory file {}: malformed JSON: {err}",
            path.display()
        ))
    })?;
    let (provider, generated_at, entries) = validated_document(&value)?;
    Ok(InventorySnapshot {
        contract: INVENTORY_CONTRACT_VERSION.to_string(),
        provider,
        generated_at,
        source: Some(absolute_display(path)),
        projects: entries.0,
        malformed: entries.1,
    })
}

/// External-adapter inventory source. The adapter is an executable
/// that reads a JSON envelope on stdin and writes one JSON document
/// on stdout. The envelope matches the validated-document shape so
/// adapters cannot widen the contract; the adapter is bounded by
/// a timeout, an output size cap, and a credential redaction pass.
pub fn invoke_external(
    executable: &Path,
    working_dir: &Path,
) -> Result<InventorySnapshot, ForgeError> {
    if !executable.is_file() {
        return Err(inventory_invalid(format!(
            "inventory adapter `{}` is not an existing file",
            executable.display()
        )));
    }
    let request = serde_json::json!({
        "contract": INVENTORY_CONTRACT_VERSION,
        "operation": "inventory",
    });
    let input = serde_json::to_vec(&request).map_err(|err| {
        inventory_invalid(format!("cannot encode inventory adapter request: {err}"))
    })?;
    let mut child = Command::new(executable)
        .current_dir(working_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|err| {
            inventory_invalid(format!(
                "cannot start inventory adapter `{}`: {err}",
                executable.display()
            ))
        })?;
    use std::io::Write;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(&input).map_err(|err| {
            inventory_invalid(format!("cannot write inventory adapter request: {err}"))
        })?;
    }
    let start = std::time::Instant::now();
    let timeout = std::time::Duration::from_secs(INVENTORY_ADAPTER_TIMEOUT_SECS);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if start.elapsed() > timeout => {
                let _ = child.kill();
                return Err(inventory_invalid(format!(
                    "inventory adapter `{}` timed out after {INVENTORY_ADAPTER_TIMEOUT_SECS}s",
                    executable.display()
                )));
            }
            Ok(None) => std::thread::sleep(std::time::Duration::from_millis(25)),
            Err(err) => {
                return Err(inventory_invalid(format!(
                    "inventory adapter `{}` wait failed: {err}",
                    executable.display()
                )))
            }
        }
    }
    let output = child.wait_with_output().map_err(|err| {
        inventory_invalid(format!("cannot collect inventory adapter output: {err}"))
    })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(inventory_invalid(format!(
            "inventory adapter `{}` exited non-zero (status {}): {}",
            executable.display(),
            output.status,
            clean_field(&stderr)
        )));
    }
    if output.stdout.len() as u64 > MAX_INVENTORY_BYTES {
        return Err(inventory_invalid(format!(
            "inventory adapter `{}` returned {} bytes; max {MAX_INVENTORY_BYTES}",
            executable.display(),
            output.stdout.len()
        )));
    }
    let value: Value = serde_json::from_slice(&output.stdout).map_err(|err| {
        inventory_invalid(format!("inventory adapter returned invalid JSON: {err}"))
    })?;
    let (provider, generated_at, entries) = validated_document(&value)?;
    Ok(InventorySnapshot {
        contract: INVENTORY_CONTRACT_VERSION.to_string(),
        provider: redact_credentials(&provider),
        generated_at,
        source: Some(format!("adapter:{}", executable.display())),
        projects: entries.0,
        malformed: entries.1,
    })
}

/// Project payload returned by [`validated_document`]: the
/// declared provider id, RFC 3339 generation timestamp, and the
/// (valid projects, malformed entries) pair.
type ValidatedPayload = (
    String,
    String,
    (Vec<InventoryEntry>, Vec<InventoryMalformedEntry>),
);

/// Validate the JSON value conforms to the inventory contract and
/// return the normalized projects plus the malformed list.
fn validated_document(value: &Value) -> Result<ValidatedPayload, ForgeError> {
    let root = value
        .as_object()
        .ok_or_else(|| inventory_invalid("inventory document is not a JSON object"))?;
    let contract = root
        .get("contract")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            inventory_invalid(format!(
                "inventory contract must be `{INVENTORY_CONTRACT_VERSION}`"
            ))
        })?;
    if contract != INVENTORY_CONTRACT_VERSION {
        return Err(inventory_invalid(format!(
            "unknown inventory contract `{contract}`; only `{INVENTORY_CONTRACT_VERSION}` is supported"
        )));
    }
    let provider = root
        .get("provider")
        .and_then(Value::as_str)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| inventory_invalid("inventory document is missing `provider`"))?;
    let generated_at = root
        .get("generated_at")
        .and_then(Value::as_str)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| inventory_invalid("inventory document is missing `generated_at`"))?;
    let projects = root
        .get("projects")
        .and_then(Value::as_array)
        .ok_or_else(|| inventory_invalid("`projects` is missing or is not an array"))?;
    if projects.len() > MAX_INVENTORY_ENTRIES {
        return Err(inventory_invalid(format!(
            "inventory declares {} entries; max {MAX_INVENTORY_ENTRIES}",
            projects.len()
        )));
    }
    let mut entries: Vec<InventoryEntry> = Vec::new();
    let mut malformed: Vec<InventoryMalformedEntry> = Vec::new();
    let mut seen: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for (index, raw) in projects.iter().enumerate() {
        match project_entry(raw) {
            Ok(entry) => {
                if !seen.insert(entry.id.clone()) {
                    return Err(inventory_invalid(format!(
                        "duplicate inventory id `{}`; refusing to choose silently",
                        entry.id
                    )));
                }
                entries.push(entry);
            }
            Err(reason) => malformed.push(InventoryMalformedEntry {
                name: malformed_name(raw, index),
                reason,
            }),
        }
    }
    Ok((provider, generated_at, (entries, malformed)))
}

fn project_entry(raw: &Value) -> Result<InventoryEntry, String> {
    let object = raw
        .as_object()
        .ok_or_else(|| "entry is not a JSON object".to_string())?;
    let id =
        string_field(object, "id").ok_or_else(|| "entry `id` is missing or blank".to_string())?;
    if id.chars().count() > MAX_ID_CHARS {
        return Err(format!(
            "entry id is {} chars; max {MAX_ID_CHARS}",
            id.chars().count()
        ));
    }
    if !id
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Err(format!(
            "entry id `{id}` must be kebab/snake-case (letters, digits, dash, underscore)"
        ));
    }
    let repository = string_field(object, "repository")
        .ok_or_else(|| format!("entry `{id}` is missing `repository`"))?;
    let revision = string_field(object, "revision")
        .ok_or_else(|| format!("entry `{id}` is missing `revision`"))?;
    if validate_revision(&revision).is_err() {
        return Err(format!(
            "entry `{id}` revision `{revision}` is not a 40-character hex SHA"
        ));
    }
    let profile = string_field(object, "profile").unwrap_or_default();
    let runtime_raw = string_field(object, "runtime")
        .ok_or_else(|| format!("entry `{id}` is missing `runtime` (web|worker|job|library)"))?;
    let runtime = RuntimeClass::parse(&runtime_raw).ok_or_else(|| {
        format!("entry `{id}` runtime `{runtime_raw}` is not one of web|worker|job|library")
    })?;
    let compose_file = object
        .get("compose_file")
        .and_then(Value::as_str)
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty() && !v.contains('/') && !v.contains(".."));
    let source_path = object
        .get("source_path")
        .and_then(Value::as_str)
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty());
    let public_http = object
        .get("public_http")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let public_port = object
        .get("public_port")
        .and_then(Value::as_u64)
        .map(|v| {
            if v == 0 || v > u16::MAX as u64 {
                None
            } else {
                Some(v as u16)
            }
        })
        .unwrap_or(None);
    if public_http && !matches!(runtime, RuntimeClass::Web) {
        return Err(format!(
            "entry `{id}` declares public_http but runtime is `{runtime}`; only web may expose HTTP"
        ));
    }
    if public_http && public_port.is_none() {
        return Err(format!(
            "entry `{id}` declares public_http but no public_port"
        ));
    }
    if !public_http && public_port.is_some() {
        return Err(format!(
            "entry `{id}` declares public_port without public_http"
        ));
    }
    Ok(InventoryEntry {
        id,
        repository: clean_field(&repository),
        revision,
        profile: clean_field(&profile),
        runtime,
        compose_file,
        source_path,
        public_http,
        public_port,
    })
}

fn string_field(object: &serde_json::Map<String, Value>, key: &str) -> Option<String> {
    object
        .get(key)
        .and_then(Value::as_str)
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

fn malformed_name(raw: &Value, index: usize) -> String {
    raw.as_object()
        .and_then(|object| object.get("id"))
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty() && value.chars().count() <= MAX_ID_CHARS)
        .map(clean_field)
        .unwrap_or_else(|| format!("entry #{index}"))
}

fn inventory_invalid(reason: impl AsRef<str>) -> ForgeError {
    ForgeError::PublishInvalid {
        reason: format!("inventory invalid: {}", clean_field(reason.as_ref())),
    }
}

fn clean_field(value: &str) -> String {
    let redacted = redact_credentials(value);
    if redacted.chars().count() <= MAX_FIELD_CHARS {
        redacted
    } else {
        let truncated: String = redacted.chars().take(MAX_FIELD_CHARS).collect();
        format!("{truncated}…")
    }
}

fn absolute_display(path: &Path) -> String {
    std::fs::canonicalize(path)
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

/// Classify every entry in a snapshot and produce the fleet
/// report. The classification respects the inventory contract
/// without making any assumption about Mac availability; it is a
/// pure function of the snapshot and the declared source paths.
pub fn classify(snapshot: &InventorySnapshot, domain: &str) -> InventoryFleetReport {
    let mut entries = Vec::with_capacity(snapshot.declared_count());
    for project in &snapshot.projects {
        entries.push(classify_entry(project, domain));
    }
    for malformed in &snapshot.malformed {
        entries.push(InventoryFleetEntry {
            id: malformed.name.clone(),
            runtime: RuntimeClass::Library,
            profile: String::new(),
            revision: String::new(),
            classification: InventoryClassification::Invalid,
            compose_file: None,
            source_path: None,
            public_http: false,
            public_port: None,
            subdomain: None,
            reason: Some(clean_field(&malformed.reason)),
        });
    }
    // Stable, deterministic order: by id so the operator can
    // predict the fleet sequence.
    entries.sort_by(|a, b| a.id.cmp(&b.id));
    InventoryFleetReport {
        contract: INVENTORY_CONTRACT_VERSION.to_string(),
        provider: snapshot.provider.clone(),
        source: snapshot.source.clone(),
        generated_at: snapshot.generated_at.clone(),
        domain: domain.to_string(),
        entries,
    }
}

fn classify_entry(entry: &InventoryEntry, domain: &str) -> InventoryFleetEntry {
    let compose_file = entry.compose_file.clone();
    let source_path = entry.source_path.clone();
    let source_candidate = source_path.as_deref().map(Path::new);
    let compose_present = match (compose_file.as_deref(), source_candidate) {
        (Some(file), Some(root)) => root.join(file).is_file(),
        _ => false,
    };
    let classification = if !compose_present {
        // Distinguish `source_unavailable` (root declared but does
        // not exist) from `compose_missing` (Compose file simply
        // absent).
        match (compose_file.as_deref(), source_candidate) {
            (Some(_), Some(root)) if !root.is_dir() => InventoryClassification::SourceUnavailable,
            (None, _) => InventoryClassification::ComposeMissing,
            (Some(_), None) => InventoryClassification::SourceUnavailable,
            (Some(_), Some(_)) => InventoryClassification::ComposeMissing,
        }
    } else {
        InventoryClassification::ComposeReady
    };
    InventoryFleetEntry {
        id: entry.id.clone(),
        runtime: entry.runtime,
        profile: entry.profile.clone(),
        revision: entry.revision.clone(),
        classification,
        compose_file,
        source_path,
        public_http: entry.public_http,
        public_port: entry.public_port,
        subdomain: entry.subdomain(domain),
        reason: None,
    }
}

/// Optional deterministic clock for tests.
pub fn fleet_report(
    snapshot: &InventorySnapshot,
    domain: &str,
    now: DateTime<Utc>,
) -> InventoryFleetReport {
    let mut report = classify(snapshot, domain);
    report.generated_at = now.to_rfc3339_opts(SecondsFormat::Secs, true);
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    const SHA: &str = "0123456789abcdef0123456789abcdef01234567";

    fn entry_json(id: &str, runtime: &str, compose: Option<&str>) -> Value {
        let mut value = serde_json::json!({
            "id": id,
            "repository": format!("https://example.invalid/{id}.git"),
            "revision": SHA,
            "profile": "rust-product",
            "runtime": runtime,
        });
        if let Some(name) = compose {
            value["compose_file"] = serde_json::json!(name);
        }
        value
    }

    fn snapshot_from(projects: Vec<Value>) -> InventorySnapshot {
        let document = serde_json::json!({
            "contract": INVENTORY_CONTRACT_VERSION,
            "provider": "local",
            "generated_at": "2026-09-28T00:00:00Z",
            "projects": projects,
        });
        let (_, _, entries) = validated_document(&document).unwrap();
        InventorySnapshot {
            contract: INVENTORY_CONTRACT_VERSION.to_string(),
            provider: "local".to_string(),
            generated_at: "2026-09-28T00:00:00Z".to_string(),
            source: None,
            projects: entries.0,
            malformed: entries.1,
        }
    }

    #[test]
    fn runtime_class_parses_known_values() {
        assert_eq!(RuntimeClass::parse("web"), Some(RuntimeClass::Web));
        assert_eq!(RuntimeClass::parse("WORKER"), Some(RuntimeClass::Worker));
        assert_eq!(RuntimeClass::parse("job"), Some(RuntimeClass::Job));
        assert_eq!(RuntimeClass::parse("library"), Some(RuntimeClass::Library));
        assert_eq!(RuntimeClass::parse("database"), None);
    }

    #[test]
    fn validates_minimal_document() {
        let document = serde_json::json!({
            "contract": INVENTORY_CONTRACT_VERSION,
            "provider": "local",
            "generated_at": "2026-09-28T00:00:00Z",
            "projects": [],
        });
        let (provider, generated_at, entries) = validated_document(&document).unwrap();
        assert_eq!(provider, "local");
        assert_eq!(generated_at, "2026-09-28T00:00:00Z");
        assert!(entries.0.is_empty());
        assert!(entries.1.is_empty());
    }

    #[test]
    fn refuses_wrong_contract() {
        let document = serde_json::json!({
            "contract": "wrong/0.1.0",
            "provider": "local",
            "generated_at": "2026-09-28T00:00:00Z",
            "projects": [],
        });
        let err = validated_document(&document).unwrap_err();
        assert_eq!(err.code(), "publish-invalid");
    }

    #[test]
    fn rejects_missing_required_fields() {
        let document = serde_json::json!({
            "contract": INVENTORY_CONTRACT_VERSION,
            "provider": "local",
            "generated_at": "2026-09-28T00:00:00Z",
            "projects": [{
                "id": "demo",
                "repository": "https://example.invalid/demo.git",
                "profile": "rust-product",
                "runtime": "web"
            }]
        });
        // Per-entry validation goes to the malformed list, not a
        // typed document rejection — the document still loads so
        // the operator sees every declared entry.
        let (_, _, entries) = validated_document(&document).unwrap();
        assert_eq!(entries.0.len(), 0);
        assert_eq!(entries.1.len(), 1);
        assert!(entries.1[0].reason.contains("revision"));
    }

    #[test]
    fn rejects_non_hex_revision() {
        let document = serde_json::json!({
            "contract": INVENTORY_CONTRACT_VERSION,
            "provider": "local",
            "generated_at": "2026-09-28T00:00:00Z",
            "projects": [{
                "id": "demo",
                "repository": "https://example.invalid/demo.git",
                "revision": "short",
                "profile": "rust-product",
                "runtime": "web"
            }]
        });
        let (_, _, entries) = validated_document(&document).unwrap();
        assert_eq!(entries.0.len(), 0);
        assert_eq!(entries.1.len(), 1);
        assert!(entries.1[0].reason.contains("40-character"));
    }

    #[test]
    fn rejects_unknown_runtime() {
        let document = serde_json::json!({
            "contract": INVENTORY_CONTRACT_VERSION,
            "provider": "local",
            "generated_at": "2026-09-28T00:00:00Z",
            "projects": [{
                "id": "demo",
                "repository": "https://example.invalid/demo.git",
                "revision": SHA,
                "profile": "rust-product",
                "runtime": "database"
            }]
        });
        let (_, _, entries) = validated_document(&document).unwrap();
        assert_eq!(entries.1.len(), 1);
        assert!(entries.1[0].reason.contains("runtime"));
    }

    #[test]
    fn rejects_public_http_without_runtime_web() {
        let document = serde_json::json!({
            "contract": INVENTORY_CONTRACT_VERSION,
            "provider": "local",
            "generated_at": "2026-09-28T00:00:00Z",
            "projects": [{
                "id": "demo",
                "repository": "https://example.invalid/demo.git",
                "revision": SHA,
                "profile": "rust-product",
                "runtime": "worker",
                "public_http": true,
                "public_port": 8080
            }]
        });
        let (_, _, entries) = validated_document(&document).unwrap();
        assert_eq!(entries.1.len(), 1);
        assert!(entries.1[0].reason.contains("public_http"));
    }

    #[test]
    fn rejects_public_port_without_public_http() {
        let document = serde_json::json!({
            "contract": INVENTORY_CONTRACT_VERSION,
            "provider": "local",
            "generated_at": "2026-09-28T00:00:00Z",
            "projects": [{
                "id": "demo",
                "repository": "https://example.invalid/demo.git",
                "revision": SHA,
                "profile": "rust-product",
                "runtime": "web",
                "public_port": 8080
            }]
        });
        let (_, _, entries) = validated_document(&document).unwrap();
        assert_eq!(entries.1.len(), 1);
        assert!(entries.1[0].reason.contains("public_port"));
    }

    #[test]
    fn rejects_duplicate_ids() {
        let document = serde_json::json!({
            "contract": INVENTORY_CONTRACT_VERSION,
            "provider": "local",
            "generated_at": "2026-09-28T00:00:00Z",
            "projects": [
                entry_json("dup", "web", None),
                entry_json("dup", "web", None),
            ]
        });
        let err = validated_document(&document).unwrap_err();
        assert!(format!("{err}").contains("duplicate"));
    }

    #[test]
    fn malformed_entries_named_with_reason() {
        let document = serde_json::json!({
            "contract": INVENTORY_CONTRACT_VERSION,
            "provider": "local",
            "generated_at": "2026-09-28T00:00:00Z",
            "projects": [
                entry_json("ok", "web", None),
                serde_json::json!({
                    "id": "bad",
                    "repository": "https://example.invalid/bad.git",
                    "revision": "short",
                    "profile": "rust-product",
                    "runtime": "web",
                }),
            ]
        });
        let (_, _, entries) = validated_document(&document).unwrap();
        assert_eq!(entries.0.len(), 1);
        assert_eq!(entries.1.len(), 1);
        assert_eq!(entries.1[0].name, "bad");
        assert!(entries.1[0].reason.contains("40-character"));
    }

    #[test]
    fn classify_marks_compose_ready_when_file_present() {
        let tmp = TempDir::new().unwrap();
        let project = tmp.path().join("webapp");
        std::fs::create_dir(&project).unwrap();
        std::fs::write(project.join("docker-compose.yml"), "services: {}").unwrap();
        let snapshot = snapshot_from(vec![{
            let mut value = entry_json("webapp", "web", Some("docker-compose.yml"));
            value["source_path"] = serde_json::json!(project.display().to_string());
            value["public_http"] = serde_json::json!(true);
            value["public_port"] = serde_json::json!(8080);
            value
        }]);
        let report = classify(&snapshot, DEFAULT_DOMAIN);
        assert_eq!(report.entries.len(), 1);
        assert_eq!(
            report.entries[0].classification,
            InventoryClassification::ComposeReady
        );
        assert_eq!(
            report.entries[0].subdomain.as_deref(),
            Some("webapp.tooosall.uk")
        );
    }

    #[test]
    fn classify_marks_compose_missing_when_no_compose_field() {
        let snapshot = snapshot_from(vec![entry_json("worker", "worker", None)]);
        let report = classify(&snapshot, DEFAULT_DOMAIN);
        assert_eq!(
            report.entries[0].classification,
            InventoryClassification::ComposeMissing
        );
        assert!(report.entries[0].subdomain.is_none());
    }

    #[test]
    fn classify_marks_compose_missing_when_file_absent() {
        let tmp = TempDir::new().unwrap();
        let project = tmp.path().join("demo");
        std::fs::create_dir(&project).unwrap();
        let snapshot = snapshot_from(vec![{
            let mut value = entry_json("demo", "web", Some("docker-compose.yml"));
            value["source_path"] = serde_json::json!(project.display().to_string());
            value
        }]);
        let report = classify(&snapshot, DEFAULT_DOMAIN);
        assert_eq!(
            report.entries[0].classification,
            InventoryClassification::ComposeMissing
        );
    }

    #[test]
    fn classify_marks_source_unavailable_when_path_missing() {
        let snapshot = snapshot_from(vec![{
            let mut value = entry_json("demo", "web", Some("docker-compose.yml"));
            value["source_path"] = serde_json::json!("/nonexistent/path/never");
            value
        }]);
        let report = classify(&snapshot, DEFAULT_DOMAIN);
        assert_eq!(
            report.entries[0].classification,
            InventoryClassification::SourceUnavailable
        );
    }

    #[test]
    fn classify_never_routes_non_web_runtime() {
        let snapshot = snapshot_from(vec![{
            let mut value = entry_json("db", "worker", None);
            value["public_http"] = serde_json::json!(false);
            value
        }]);
        let report = classify(&snapshot, DEFAULT_DOMAIN);
        assert!(report.entries[0].subdomain.is_none());
    }

    #[test]
    fn classify_sorts_by_id_for_deterministic_fleet_sequence() {
        let snapshot = snapshot_from(vec![
            entry_json("zeta", "web", None),
            entry_json("alpha", "web", None),
            entry_json("mu", "web", None),
        ]);
        let report = classify(&snapshot, DEFAULT_DOMAIN);
        let ids: Vec<&str> = report.entries.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, vec!["alpha", "mu", "zeta"]);
    }

    #[test]
    fn load_local_round_trips_minimal_document() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("inventory.json");
        std::fs::write(
            &path,
            serde_json::to_string(&serde_json::json!({
                "contract": INVENTORY_CONTRACT_VERSION,
                "provider": "local",
                "generated_at": "2026-09-28T00:00:00Z",
                "projects": [entry_json("demo", "web", Some("docker-compose.yml"))]
            }))
            .unwrap(),
        )
        .unwrap();
        let snapshot = load_local(&path).unwrap();
        assert_eq!(snapshot.projects.len(), 1);
        assert_eq!(snapshot.projects[0].id, "demo");
        assert!(snapshot.malformed.is_empty());
    }

    #[test]
    fn load_local_refuses_oversized_file() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("inventory.json");
        let mut buffer = vec![b'x'; (MAX_INVENTORY_BYTES + 1) as usize];
        buffer[0] = b'{';
        std::fs::write(&path, &buffer).unwrap();
        let err = load_local(&path).unwrap_err();
        assert_eq!(err.code(), "publish-invalid");
    }

    #[test]
    fn load_local_refuses_missing_file() {
        let err = load_local(Path::new("/no/such/inventory.json")).unwrap_err();
        assert_eq!(err.code(), "publish-invalid");
    }

    #[test]
    fn invoke_external_refuses_missing_executable() {
        let err = invoke_external(Path::new("/no/such/inventory-adapter"), Path::new("/tmp"))
            .unwrap_err();
        assert_eq!(err.code(), "publish-invalid");
        assert!(format!("{err}").contains("not an existing file"));
    }
}
