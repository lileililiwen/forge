//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

/// Whether the row can be operated by Forge. Only `managed` and `self` rows
/// may ever expose an operation link; `observed` rows are inspection-only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Management {
    Managed,
    Observed,
    SelfRecord,
}
impl Management {
    pub fn id(self) -> &'static str {
        match self {
            Management::Managed => "managed",
            Management::Observed => "observed",
            Management::SelfRecord => "self",
        }
    }
}
/// Truthful per-row freshness label derived from its source status. Rows are
/// only emitted by sources that actually returned data, so an unavailable
/// source contributes no rows and its state lives on the source descriptor
/// (`sources[].status = "unavailable"`), never as a per-row label.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowFreshness {
    Fresh,
    Stale,
    Unconfigured,
}
impl RowFreshness {
    pub fn id(self) -> &'static str {
        match self {
            RowFreshness::Fresh => "fresh",
            RowFreshness::Stale => "stale",
            RowFreshness::Unconfigured => "unconfigured",
        }
    }
}
/// Which provenance produced one row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowSource {
    SelfRecord,
    Registry,
    Inventory,
    WorkspaceRegistry,
    Published,
}
impl RowSource {
    pub fn id(self) -> &'static str {
        match self {
            RowSource::SelfRecord => "self",
            RowSource::Registry => "registry",
            RowSource::Inventory => "inventory",
            RowSource::WorkspaceRegistry => "fleet",
            RowSource::Published => "published",
        }
    }
}
/// One aggregated candidate row. Built by [`gather`] (real sources) or by
/// tests (fixture data); [`build_envelope`] then merges, sorts and renders it.
#[derive(Debug, Clone)]
pub struct CandidateRow {
    pub identity: String,
    pub name: String,
    pub profile: Option<String>,
    pub state: String,
    pub source: RowSource,
    pub management: Management,
    pub is_self: bool,
    pub freshness: RowFreshness,
    pub updated_at: Option<String>,
    pub capabilities: Vec<String>,
    pub evidence: Vec<(String, String)>,
    pub lifecycle: Option<String>,
    pub confidence: Option<String>,
    pub tags: Vec<String>,
    /// True only on the self row when Forge is also a registered project;
    /// the registry row is merged into the self row rather than duplicated.
    pub has_registry_ref: bool,
    /// Most recent local publish operation, when one exists for this
    /// identity. Attached to a registered/self row (merge) or carried by a
    /// standalone observed `published` row.
    pub publish: Option<PublishProjection>,
}
/// One declared source's status for the `sources` array. Carries a safe
/// reason and malformed names only — never an absolute filesystem path.
#[derive(Debug, Clone)]
pub struct SourceDescriptor {
    pub id: &'static str,
    pub kind: &'static str,
    pub status: &'static str,
    pub count: usize,
    pub malformed: Vec<(String, String)>,
    pub reason: Option<String>,
    pub observed_at: Option<String>,
    pub provider: Option<String>,
}
impl SourceDescriptor {
    pub(super) fn unconfigured(id: &'static str, kind: &'static str) -> Self {
        SourceDescriptor {
            id,
            kind,
            status: "unconfigured",
            count: 0,
            malformed: Vec::new(),
            reason: None,
            observed_at: None,
            provider: None,
        }
    }
}
/// Bounded projection of one project's most recent publish operation, as
/// rendered on a fleet row. `detail` is redacted before it is stored;
/// `healthy`/`stages` are parsed from that redacted detail and stay `None`
/// when the detail does not carry them (e.g. a legacy row).
#[derive(Debug, Clone, Default)]
pub struct PublishProjection {
    pub state: String,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub detail: Option<String>,
    pub target: Option<String>,
    pub revision: Option<String>,
    pub build_status: Option<String>,
    pub run_status: Option<String>,
    pub container_identity: Option<String>,
    pub healthy: Option<bool>,
    pub stages: Option<i64>,
}
