//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use serde::{Deserialize, Serialize};

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
