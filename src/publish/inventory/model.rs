//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use serde::{Deserialize, Serialize};

use crate::runtime::{HostModel, RuntimeKind};

/// The reason a shared-host entry carries. A module is hosted by the
/// project that adopts it, so the fleet never starts one.
///
/// The wording lives in [`crate::runtime`], which owns the host model;
/// it is re-exported here so the publish lane reads it from one place.
pub use crate::runtime::SHARED_HOST_REASON;

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
    pub runtime: RuntimeKind,
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
        if self.public_http && matches!(self.runtime, RuntimeKind::Web) {
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
    /// The runtime kind this entry declares. `None` only when the
    /// entry was refused before its runtime could be read — the row
    /// then carries no kind at all rather than a placeholder that
    /// would read as a claim.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime: Option<RuntimeKind>,
    /// How this entry's runtime kind is hosted. An own-process kind
    /// runs as a workload of its own; a shared-host kind is a module
    /// hosted by the project that adopts it. `None` when the runtime
    /// kind could not be read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host_model: Option<HostModel>,
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
impl InventoryFleetEntry {
    /// Whether the fleet may publish this entry.
    ///
    /// An entry is publishable when it is compose-ready **and** its
    /// runtime kind is an own-process kind. A shared-host entry is a
    /// module: the project that adopts it owns its lifecycle, so the
    /// fleet never starts it.
    pub fn is_publishable(&self) -> bool {
        is_publishable(self.classification, self.host_model)
    }
}

/// The fleet's publishing rule, in one place.
///
/// An entry is publishable when it is compose-ready **and** its
/// runtime kind is an own-process kind. A shared-host entry is a
/// module: the project that adopts it owns its lifecycle, so the
/// fleet never starts it. An entry whose runtime kind could not be
/// read is never publishable.
///
/// The publish lane and the observe lane both read this rule, so a
/// module the fleet never started is never probed either.
pub fn is_publishable(
    classification: InventoryClassification,
    host_model: Option<HostModel>,
) -> bool {
    classification == InventoryClassification::ComposeReady
        && host_model == Some(HostModel::OwnProcess)
}
/// The full fleet classification result. Every declared entry is
/// present exactly once; only the entries that are compose-ready and
/// hosted by their own process are eligible for provider invocation.
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
    /// The entries the fleet may publish: compose-ready and hosted by
    /// their own process.
    pub fn publishable(&self) -> impl Iterator<Item = &InventoryFleetEntry> {
        self.entries.iter().filter(|entry| entry.is_publishable())
    }
}
