//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use crate::doctor::{FindingStatus, Remediation};
use crate::policy::PolicyFinding;
use crate::upgrade::SemanticConflict;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Status of a remediation application.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoutingStatus {
    /// Deterministic fix applied and validated.
    Applied,
    /// A bounded spec was generated for follow-up by an agent or a
    /// human.
    SpecGenerated,
    /// Manual routing recorded; no project changes.
    Manual,
    /// The remediation failed validation; the finding stays
    /// unresolved.
    Failed,
}
impl RoutingStatus {
    pub fn label(&self) -> &'static str {
        match self {
            RoutingStatus::Applied => "applied",
            RoutingStatus::SpecGenerated => "spec-generated",
            RoutingStatus::Manual => "manual",
            RoutingStatus::Failed => "failed",
        }
    }
}
/// One acceptance scenario in the bounded proposal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpecScenario {
    /// Stable scenario id derived from the finding id.
    pub id: String,
    /// Human-readable scenario summary.
    pub summary: String,
    /// Source finding id the scenario was derived from.
    pub source_finding: String,
}
/// Identifier for a generated spec. Stable across invocations and
/// suitable as a directory name.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct SpecId {
    /// The project identifier the spec is anchored to.
    pub project_id: String,
    /// Stable short hash of the sorted finding identifier set; the
    /// same findings always produce the same id.
    pub hash: String,
}
impl SpecId {
    /// Directory name (relative to `.forge/specs/`).
    pub fn dir_name(&self) -> String {
        format!("{}-{}", self.project_id, self.hash)
    }
}
/// Outcome of a `forge spec generate` invocation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SpecGenerateOutcome {
    /// Whether a new spec was written, an existing one was
    /// returned, or the request was refused.
    pub status: SpecStatus,
    /// The spec draft (always present, even on rejection; rejection
    /// carries the conflicting finding set so the caller can
    /// resolve it).
    pub spec: SpecDraft,
    /// Files written by this invocation (empty on rejection or when
    /// the existing spec is returned).
    pub files_written: Vec<String>,
    /// Human-readable note for the transport.
    pub note: String,
}
impl SpecGenerateOutcome {
    pub fn status_label(&self) -> &'static str {
        self.status.label()
    }
}
/// Recorded routing decision for a finding, with the rationale and
/// the concrete next step the router recommends.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RoutingDecision {
    pub finding_id: String,
    pub finding_category: String,
    pub finding_severity: String,
    pub route: SpecRoute,
    pub rationale: String,
    /// For `Deterministic` routes: the next command/action to run.
    pub action: Option<String>,
    /// For `Semantic` routes: the spec id the router would generate
    /// (if known), so the caller can plan the follow-up.
    pub suggested_spec: Option<SpecId>,
}
/// Outcome of a `forge spec apply` invocation. The status reflects
/// the remediation result, and the evidence/recovery fields match
/// the spec's traceability contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RoutingOutcome {
    pub decision: RoutingDecision,
    pub status: RoutingStatus,
    pub evidence: Vec<String>,
    pub recovery: Vec<String>,
    pub files_changed: Vec<String>,
    pub note: String,
}
/// The bounded proposal emitted by `forge spec generate`. Mirrors
/// the canonical OpenSpec change shape so a generated spec can be
/// reviewed in the existing Codex workflow.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpecDraft {
    /// Spec id (== directory name).
    pub id: SpecId,
    /// Contract data version.
    pub contract: String,
    /// Short human title.
    pub title: String,
    /// One-paragraph rationale (rendered into the proposal body).
    pub why: String,
    /// Sorted, deduplicated finding ids this spec addresses.
    pub findings: Vec<String>,
    /// Acceptance scenarios rendered from the findings.
    pub acceptance: Vec<SpecScenario>,
    /// Feature dependencies the spec requires or constrains.
    pub dependencies: Vec<String>,
    /// Full provenance.
    pub provenance: SpecProvenance,
}
/// Provenance for a generated spec: every fact a future reader needs
/// to verify what produced the proposal and against which project
/// state. Stays separate from the proposal text so the machine-
/// readable layer survives manual edits to the proposal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpecProvenance {
    /// Project id the spec is anchored to.
    pub project_id: String,
    /// Canonical filesystem path of the project at generation time.
    pub project_path: String,
    /// Profile id at generation time.
    pub profile: String,
    /// Source revision (manifest mtime) used to detect drift after
    /// generation; a stale source is reported, not silently
    /// overwritten.
    pub source_revision: Option<DateTime<Utc>>,
    /// Sorted, deduplicated finding identifiers the spec covers.
    pub finding_ids: Vec<String>,
    /// Sorted, deduplicated finding categories the spec covers.
    pub finding_categories: Vec<String>,
    /// Optional DriftWatch policy ids this spec covers (when the
    /// generation source is a policy finding).
    pub policy_ids: Vec<String>,
    /// Project features that constrain or are required by the spec.
    pub dependencies: Vec<String>,
    /// Contract data version.
    pub contract: String,
    /// When the spec was generated.
    pub generated_at: DateTime<Utc>,
}
/// Routing decision for one finding: deterministic, semantic, or
/// manual remediation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SpecRoute {
    /// A supported lifecycle or upgrade operation resolves the
    /// finding; the operation must pass its validators plus a
    /// follow-up `forge doctor` check.
    Deterministic,
    /// A bounded spec is required; `forge spec generate` is the
    /// handoff.
    Semantic,
    /// Manual judgment required; the router records the manual
    /// status without claiming an AI fix or changing the project.
    Manual,
}
impl SpecRoute {
    pub fn label(&self) -> &'static str {
        match self {
            SpecRoute::Deterministic => "deterministic",
            SpecRoute::Semantic => "semantic",
            SpecRoute::Manual => "manual",
        }
    }
}
/// Inputs to `generate_spec`: an explicit project selection plus
/// the findings the caller wants a spec to cover. Findings are
/// grouped (per category/remediation class) so a request that
/// targets different projects or mutually incompatible resolutions
/// can be refused before any file change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpecRequest {
    /// Project directory.
    pub project_path: PathBuf,
    /// Finding ids to include. Order does not matter; the router
    /// sorts and deduplicates them.
    pub finding_ids: Vec<String>,
    /// Optional human reason for the generation, used as the
    /// proposal's `why` paragraph when no finding-derived summary
    /// is available.
    pub reason: Option<String>,
}
/// Status of a spec generation request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SpecStatus {
    /// A new bounded proposal was written under `.forge/specs/`.
    Generated,
    /// The same project + finding set already has an active spec;
    /// the existing draft was returned (boundary scenario).
    Existing,
    /// The request was refused (ambiguous scope, conflicting
    /// projects, or finding set too large to keep the proposal
    /// bounded).
    Rejected,
}
impl SpecStatus {
    pub fn label(&self) -> &'static str {
        match self {
            SpecStatus::Generated => "generated",
            SpecStatus::Existing => "existing",
            SpecStatus::Rejected => "rejected",
        }
    }
}
/// One finding source for spec generation. The router accepts
/// doctor findings, DriftWatch policy findings and upgrade
/// semantic-conflict handoffs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FindingSource {
    Doctor(DoctorFindingInput),
    Policy(PolicyFinding),
    Conflict(SemanticConflict),
}
/// Minimal projection of a doctor finding used for routing and
/// provenance. We don't depend on the doctor module's full type to
/// keep this module readable from tests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoctorFindingInput {
    pub id: String,
    pub status: FindingStatus,
    pub remediation: Remediation,
    pub category: String,
    pub detail: String,
}
/// List all generated specs under `.forge/specs/` in `dir`. Each
/// spec is identified by its directory name; malformed entries are
/// skipped with a note in the returned list, so a partial write
/// never breaks the listing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SpecListEntry {
    pub id: SpecId,
    pub project_id: String,
    pub finding_ids: Vec<String>,
    pub generated_at: Option<DateTime<Utc>>,
    pub contract: String,
    pub path: String,
}
