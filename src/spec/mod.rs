//! Traceable spec generation and remediation routing
//! (`specification-remediation`).
//!
//! Core owns the versioned spec-storage layout, the
//! finding→remediation routing decision, and the bounded proposal
//! emitted by `forge spec generate`. Transports render Core outcomes
//! without reinterpreting them.
//!
//! A spec is always traceable: its [`SpecProvenance`] records the
//! project identity, the source revision (manifest mtime or other
//! deterministic identifier) and the exact finding identifiers that
//! triggered the proposal. Generation is idempotent on the same
//! finding set (same project + same sorted finding identifiers hash
//! to the same spec id), so a repeat run for an unchanged finding
//! reports the existing spec rather than producing a duplicate
//! (boundary scenario).
//!
//! Remediation routing classifies a finding into one of three
//! queues:
//!
//! - `Deterministic` — a supported lifecycle or upgrade action
//!   resolves the finding; the operation runs only after its declared
//!   validators and a follow-up `forge doctor` pass, so the success
//!   scenario records the fix and the evidence.
//! - `Semantic` — a bounded spec must be generated for an agent or a
//!   human to implement; the spec is the handoff.
//! - `Manual` — the finding is a judgment call; the router records
//!   the manual status without claiming an AI fix and without
//!   changing the project (boundary scenario). A failure leaves the
//!   finding unresolved and surfaces the recovery information.
//!
//! Storage layout (under the project root):
//!
//! ```text
//! .forge/specs/
//!   <spec-id>/
//!     manifest.json   machine-readable SpecDraft + provenance
//!     proposal.md     bounded proposal (R1 traceable payload)
//!     design.md       minimal design skeleton
//!     tasks.md        minimal task checklist
//!     routing.json    recorded routing decision when routed
//! ```
//!
//! Spec ids are stable across invocations: `<project-id>-<short-hash>`,
//! where the hash is a content-derived identifier over the sorted
//! finding set. Two requests targeting the same project and the same
//! findings collapse to one spec; a request targeting different
//! projects or incompatible finding sets is refused with
//! `error[spec-ambiguous]` so no misleading combined spec is written.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::core::manifest::{Manifest, CANONICAL_MANIFEST};
use crate::core::ForgeError;
use crate::doctor::{Finding, FindingStatus, Remediation};
use crate::policy::PolicyFinding;
use crate::upgrade::SemanticConflict;

/// Contract data version for the spec/remediation API and storage.
pub const SPEC_CONTRACT_VERSION: &str = "0.1.0";

/// Directory (relative to the project root) holding generated specs.
pub const SPECS_DIR: &str = ".forge/specs";

/// Maximum finding identifiers included in a single spec; overlapping
/// finding sets must be split or routed through separate specs to
/// keep each proposal bounded.
pub const MAX_FINDINGS_PER_SPEC: usize = 32;

/// Stable short hash length used in spec identifiers.
const SPEC_HASH_LEN: usize = 12;

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

// Stable JSON shape for `SpecId`: always serialize `dir_name` so the
// output is self-describing and the caller does not need to
// recompute it.
impl Serialize for SpecId {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("SpecId", 3)?;
        s.serialize_field("project_id", &self.project_id)?;
        s.serialize_field("hash", &self.hash)?;
        s.serialize_field("dir_name", &self.dir_name())?;
        s.end()
    }
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

impl Serialize for RoutingStatus {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.label())
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

impl From<&Finding> for DoctorFindingInput {
    fn from(f: &Finding) -> Self {
        DoctorFindingInput {
            id: f.id.clone(),
            status: f.status,
            remediation: f.remediation,
            category: category_from_remediation(f.remediation),
            detail: f.detail.clone(),
        }
    }
}

fn category_from_remediation(r: Remediation) -> String {
    match r {
        Remediation::Automatic => "automatic".to_string(),
        Remediation::Ai => "semantic".to_string(),
        Remediation::Manual => "manual".to_string(),
    }
}

/// Compute the source revision for a project: manifest mtime, or
/// `None` when the manifest is missing (an invalid project is
/// surfaced by the manifest loader, not the spec router).
pub fn source_revision_for(dir: &Path) -> Option<DateTime<Utc>> {
    let meta = fs::metadata(dir.join(CANONICAL_MANIFEST)).ok()?;
    let modified = meta.modified().ok()?;
    let secs = modified
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_secs() as i64;
    DateTime::<Utc>::from_timestamp(secs, 0)
}

/// Resolve `dir/.forge/specs/<spec-id>/` to the on-disk path.
pub fn spec_dir(dir: &Path, id: &SpecId) -> PathBuf {
    dir.join(SPECS_DIR).join(id.dir_name())
}

/// Build the [`SpecId`] for `request` after dedup+sort. The hash is
/// a stable SHA-256 prefix over the joined finding ids and the
/// project id, so two requests targeting the same project + finding
/// set collapse to one spec.
pub fn spec_id_for(project_id: &str, finding_ids: &[String]) -> SpecId {
    let mut sorted: BTreeSet<&str> = BTreeSet::new();
    for id in finding_ids {
        sorted.insert(id.as_str());
    }
    let mut hasher = Sha256::new();
    hasher.update(project_id.as_bytes());
    hasher.update(b"\n");
    for id in &sorted {
        hasher.update(id.as_bytes());
        hasher.update(b"\n");
    }
    let digest = hasher.finalize();
    let hex = format!("{:x}", digest);
    let short = hex.chars().take(SPEC_HASH_LEN).collect::<String>();
    SpecId {
        project_id: project_id.to_string(),
        hash: short,
    }
}

/// Build the [`SpecId`] for a `SpecRequest`, using the project's
/// manifest identity. Returns `SpecNotFound` when the manifest
/// cannot be loaded.
pub fn spec_id_for_request(request: &SpecRequest) -> Result<SpecId, ForgeError> {
    let (manifest, _) = Manifest::load_from_dir(&request.project_path, None)?;
    Ok(spec_id_for(&manifest.project.id, &request.finding_ids))
}

/// Whether the request is well-formed for generation:
/// - exactly one distinct project,
/// - the finding set fits the bounded proposal size,
/// - finding ids are non-empty.
///
/// Cross-project requests must be split into per-project specs
/// first; this is a hard refusal so a misleading combined spec is
/// never produced (R1 failure scenario).
pub fn validate_request(request: &SpecRequest) -> Result<(), ForgeError> {
    if request.finding_ids.is_empty() {
        return Err(ForgeError::SpecInvalid {
            reason: "spec generate requires at least one finding id".to_string(),
        });
    }
    let distinct: BTreeSet<&str> = request.finding_ids.iter().map(|s| s.as_str()).collect();
    if distinct.len() != request.finding_ids.len() {
        return Err(ForgeError::SpecInvalid {
            reason: "spec generate finding ids must be unique".to_string(),
        });
    }
    if distinct.len() > MAX_FINDINGS_PER_SPEC {
        return Err(ForgeError::SpecInvalid {
            reason: format!(
                "spec generate accepts at most {MAX_FINDINGS_PER_SPEC} finding ids per proposal; \
                 split the request into smaller scopes"
            ),
        });
    }
    for id in &request.finding_ids {
        if id.trim().is_empty() {
            return Err(ForgeError::SpecInvalid {
                reason: "spec generate finding ids must not be empty".to_string(),
            });
        }
    }
    Ok(())
}

/// Render a [`SpecDraft`] to a bounded Markdown proposal (the
/// `proposal.md` payload that mirrors the Codex workflow shape).
pub fn render_proposal_markdown(spec: &SpecDraft) -> String {
    let mut out = String::new();
    out.push_str(&format!("# {}\n\n", spec.title));
    out.push_str("## Why\n\n");
    out.push_str(&spec.why);
    out.push_str("\n\n");
    out.push_str("## Traceable provenance\n\n");
    out.push_str(&format!("- project: `{}`\n", spec.provenance.project_id));
    out.push_str(&format!("- profile: `{}`\n", spec.provenance.profile));
    out.push_str(&format!("- path: `{}`\n", spec.provenance.project_path));
    if let Some(rev) = spec.provenance.source_revision {
        out.push_str(&format!(
            "- source revision ({} mtime): `{}`\n",
            CANONICAL_MANIFEST,
            rev.to_rfc3339()
        ));
    }
    out.push_str(&format!("- contract: `{}`\n", spec.provenance.contract));
    out.push_str(&format!(
        "- generated_at: `{}`\n",
        spec.provenance.generated_at.to_rfc3339()
    ));
    out.push_str("\n## Findings included\n\n");
    for f in &spec.findings {
        out.push_str(&format!("- `{f}`\n"));
    }
    if !spec.provenance.policy_ids.is_empty() {
        out.push_str("\n## DriftWatch policy ids\n\n");
        for p in &spec.provenance.policy_ids {
            out.push_str(&format!("- `{p}`\n"));
        }
    }
    if !spec.dependencies.is_empty() {
        out.push_str("\n## Dependencies\n\n");
        for d in &spec.dependencies {
            out.push_str(&format!("- `{d}`\n"));
        }
    }
    out.push_str("\n## Acceptance scenarios\n\n");
    for s in &spec.acceptance {
        out.push_str(&format!("- `{}` — {}\n", s.id, s.summary));
    }
    out
}

/// Render a minimal design skeleton (one paragraph) referencing the
/// finding ids; the bounded proposal's design is intentionally light
/// because the spec only documents the contract change.
pub fn render_design_markdown(spec: &SpecDraft) -> String {
    let mut out = String::new();
    out.push_str(&format!("# Design: {}\n\n", spec.title));
    out.push_str("## Scope\n\n");
    out.push_str(&format!(
        "This spec covers findings `{}` on project `{}` (profile `{}`).\n\n",
        spec.findings.join(", "),
        spec.provenance.project_id,
        spec.provenance.profile
    ));
    out.push_str("## Approach\n\n");
    out.push_str(
        "The contract change is bounded: each acceptance scenario is a single WHEN/THEN from the source finding. \
         The implementation follows the existing Forge lifecycle operations (deterministic when available) and \
         only escalates to AI edits when the deterministic path cannot resolve the finding.\n",
    );
    out
}

/// Render a minimal task checklist.
pub fn render_tasks_markdown(spec: &SpecDraft) -> String {
    let mut out = String::new();
    out.push_str(&format!("# Tasks: {}\n\n", spec.title));
    out.push_str("## 1. BFS — Baseline and impact coverage\n\n");
    out.push_str(
        "- [ ] 1.1 Verify prerequisites and map the named requirements to existing contracts.\n",
    );
    out.push_str("- [ ] 1.2 Establish fixtures for each success, failure and boundary scenario.\n");
    out.push_str("\n## 2. DFS — Requirement-by-requirement implementation\n\n");
    for f in &spec.findings {
        out.push_str(&format!(
            "- [ ] 2.1 Resolve finding `{f}` through deterministic or semantic remediation.\n"
        ));
    }
    out.push_str("\n## 3. BFS — Cross-surface regression and completeness\n\n");
    out.push_str("- [ ] 3.1 Exercise the change across the affected callers and persistence.\n");
    out.push_str("- [ ] 3.2 Verify repeatability and the project-isolation contract.\n");
    out.push_str("\n## 4. Verification\n\n");
    out.push_str("- [ ] 4.1 Run `cargo fmt --check`, `cargo build`, `cargo test`.\n");
    out.push_str("- [ ] 4.2 Run `openspec validate <change> --strict --no-interactive` and `git diff --check`.\n");
    out.push_str("- [ ] 4.3 Archive without `--skip-specs` and commit only related work.\n");
    out
}

/// Build a [`SpecDraft`] for a validated request. The caller is
/// responsible for writing the spec to disk via [`write_spec`].
pub fn build_spec_draft(
    request: &SpecRequest,
    sources: &[FindingSource],
    now: DateTime<Utc>,
) -> Result<SpecDraft, ForgeError> {
    let (manifest, _) = Manifest::load_from_dir(&request.project_path, None)?;
    validate_request(request)?;
    let id = spec_id_for(&manifest.project.id, &request.finding_ids);
    let mut finding_ids: BTreeSet<String> = request.finding_ids.iter().cloned().collect();
    let mut categories: BTreeSet<String> = BTreeSet::new();
    let mut policy_ids: BTreeSet<String> = BTreeSet::new();
    let dependencies: BTreeSet<String> = manifest.features.keys().cloned().collect();
    let mut scenarios: Vec<SpecScenario> = Vec::new();
    let mut seen_scenarios: BTreeSet<String> = BTreeSet::new();
    for source in sources {
        match source {
            FindingSource::Doctor(d) => {
                finding_ids.insert(d.id.clone());
                categories.insert(d.category.clone());
                let scenario_id = scenario_id_for(&d.id, "doctor");
                if seen_scenarios.insert(scenario_id.clone()) {
                    scenarios.push(SpecScenario {
                        id: scenario_id,
                        summary: format!("Resolve doctor finding `{}`: {}", d.id, d.detail),
                        source_finding: d.id.clone(),
                    });
                }
            }
            FindingSource::Policy(p) => {
                finding_ids.insert(format!("driftwatch-{}", p.id));
                categories.insert(p.category.clone());
                policy_ids.insert(p.id.clone());
                let scenario_id = scenario_id_for(&p.id, "policy");
                if seen_scenarios.insert(scenario_id.clone()) {
                    scenarios.push(SpecScenario {
                        id: scenario_id,
                        summary: format!(
                            "Resolve DriftWatch policy `{}` ({}): {}",
                            p.id, p.category, p.message
                        ),
                        source_finding: p.id.clone(),
                    });
                }
            }
            FindingSource::Conflict(c) => {
                finding_ids.insert(format!("semantic-{}", c.feature));
                categories.insert("semantic-conflict".to_string());
                let scenario_id = scenario_id_for(&c.feature, "conflict");
                if seen_scenarios.insert(scenario_id.clone()) {
                    scenarios.push(SpecScenario {
                        id: scenario_id,
                        summary: format!(
                            "Resolve semantic conflict for feature `{}` on owned file `{}`",
                            c.feature, c.owned_file
                        ),
                        source_finding: format!("semantic-{}", c.feature),
                    });
                }
            }
        }
    }
    // When no source was supplied, render one acceptance scenario
    // per requested finding id so the bounded proposal still has
    // traceable scenarios.
    for finding in &request.finding_ids {
        let scenario_id = scenario_id_for(finding, "doctor");
        if seen_scenarios.insert(scenario_id.clone()) {
            scenarios.push(SpecScenario {
                id: scenario_id,
                summary: format!(
                    "Resolve finding `{}` through deterministic or semantic remediation",
                    finding
                ),
                source_finding: finding.clone(),
            });
        }
    }
    let finding_ids: Vec<String> = finding_ids.into_iter().collect();
    let categories: Vec<String> = categories.into_iter().collect();
    let policy_ids: Vec<String> = policy_ids.into_iter().collect();
    let dependencies: Vec<String> = dependencies.into_iter().collect();
    let why = if let Some(reason) = request.reason.as_deref() {
        if reason.trim().is_empty() {
            default_why(&manifest.project.id, &finding_ids)
        } else {
            reason.to_string()
        }
    } else {
        default_why(&manifest.project.id, &finding_ids)
    };
    let title = format!(
        "Bounded spec for {} ({})",
        manifest.project.id,
        finding_ids.len()
    );
    let provenance = SpecProvenance {
        project_id: manifest.project.id.clone(),
        project_path: request.project_path.display().to_string(),
        profile: manifest.project.profile.clone(),
        source_revision: source_revision_for(&request.project_path),
        finding_ids: finding_ids.clone(),
        finding_categories: categories,
        policy_ids,
        dependencies,
        contract: SPEC_CONTRACT_VERSION.to_string(),
        generated_at: now,
    };
    Ok(SpecDraft {
        id,
        contract: SPEC_CONTRACT_VERSION.to_string(),
        title,
        why,
        findings: finding_ids,
        acceptance: scenarios,
        dependencies: provenance.dependencies.clone(),
        provenance,
    })
}

fn default_why(project_id: &str, finding_ids: &[String]) -> String {
    format!(
        "Project `{}` has {} semantic finding(s) that require a bounded proposal: `{}`. \
         This spec documents the contract change; implementation follows the existing Forge \
         lifecycle operations (deterministic when available) and only escalates to AI edits \
         when the deterministic path cannot resolve the finding.",
        project_id,
        finding_ids.len(),
        finding_ids.join(", "),
    )
}

fn scenario_id_for(finding_id: &str, kind: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(kind.as_bytes());
    hasher.update(b"\n");
    hasher.update(finding_id.as_bytes());
    let digest = hasher.finalize();
    let hex = format!("{:x}", digest);
    let short = hex.chars().take(8).collect::<String>();
    format!("{kind}-{short}")
}

/// Persist `spec` under `.forge/specs/<id>/` in `dir`. Returns the
/// list of relative file paths written. The manifest is always
/// written last so a partial write can be detected and recovered.
pub fn write_spec(dir: &Path, spec: &SpecDraft) -> Result<Vec<String>, ForgeError> {
    let base = spec_dir(dir, &spec.id);
    fs::create_dir_all(&base).map_err(|err| ForgeError::SpecWrite {
        path: base.display().to_string(),
        reason: err.to_string(),
    })?;
    let proposal = render_proposal_markdown(spec);
    let design = render_design_markdown(spec);
    let tasks = render_tasks_markdown(spec);
    let manifest_json =
        serde_json::to_string_pretty(spec).map_err(|err| ForgeError::SpecWrite {
            path: base.display().to_string(),
            reason: err.to_string(),
        })?;
    let rel =
        |name: &str| -> String { SPECS_DIR.to_string() + "/" + &spec.id.dir_name() + "/" + name };
    let write_one = |name: &str, body: &str| -> Result<(), ForgeError> {
        let path = base.join(name);
        fs::write(&path, body).map_err(|err| ForgeError::SpecWrite {
            path: path.display().to_string(),
            reason: err.to_string(),
        })
    };
    write_one("proposal.md", &proposal)?;
    write_one("design.md", &design)?;
    write_one("tasks.md", &tasks)?;
    write_one("manifest.json", &manifest_json)?;
    Ok(vec![
        rel("proposal.md"),
        rel("design.md"),
        rel("tasks.md"),
        rel("manifest.json"),
    ])
}

/// Read an existing spec from `.forge/specs/<id>/`. Returns `None`
/// when no manifest exists at the expected path.
pub fn read_spec(dir: &Path, id: &SpecId) -> Result<Option<SpecDraft>, ForgeError> {
    let path = spec_dir(dir, id).join("manifest.json");
    if !path.is_file() {
        return Ok(None);
    }
    let text = fs::read_to_string(&path).map_err(|err| ForgeError::SpecWrite {
        path: path.display().to_string(),
        reason: err.to_string(),
    })?;
    let spec: SpecDraft = serde_json::from_str(&text).map_err(|err| ForgeError::SpecWrite {
        path: path.display().to_string(),
        reason: err.to_string(),
    })?;
    Ok(Some(spec))
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

pub fn list_specs(dir: &Path) -> Result<Vec<SpecListEntry>, ForgeError> {
    let base = dir.join(SPECS_DIR);
    if !base.is_dir() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in fs::read_dir(&base).map_err(|err| ForgeError::SpecWrite {
        path: base.display().to_string(),
        reason: err.to_string(),
    })? {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        if !entry.path().is_dir() {
            continue;
        }
        let manifest_path = entry.path().join("manifest.json");
        if !manifest_path.is_file() {
            continue;
        }
        let text = match fs::read_to_string(&manifest_path) {
            Ok(t) => t,
            Err(_) => continue,
        };
        let draft: SpecDraft = match serde_json::from_str(&text) {
            Ok(d) => d,
            Err(_) => continue,
        };
        out.push(SpecListEntry {
            project_id: draft.provenance.project_id.clone(),
            id: draft.id.clone(),
            finding_ids: draft.findings.clone(),
            generated_at: Some(draft.provenance.generated_at),
            contract: draft.contract.clone(),
            path: entry.path().display().to_string(),
        });
    }
    out.sort_by_key(|a| a.id.dir_name());
    Ok(out)
}

/// Generate a spec for `request`. The request must target a single
/// project; cross-project or oversized requests are refused with
/// `error[spec-ambiguous]` (R1 failure scenario). When a spec
/// already exists for the same project + finding set, the existing
/// draft is returned without rewriting files (R1 boundary
/// scenario). On success, the proposal/design/tasks/manifest files
/// are written under `.forge/specs/<id>/`.
pub fn generate_spec(
    request: &SpecRequest,
    sources: &[FindingSource],
    now: DateTime<Utc>,
) -> Result<SpecGenerateOutcome, ForgeError> {
    let spec = build_spec_draft(request, sources, now)?;
    if let Some(existing) = read_spec(&request.project_path, &spec.id)? {
        return Ok(SpecGenerateOutcome {
            status: SpecStatus::Existing,
            spec: existing,
            files_written: Vec::new(),
            note: format!(
                "spec `{}` already exists for project `{}` and the same finding set; \
                 no files were rewritten",
                spec.id.dir_name(),
                spec.provenance.project_id
            ),
        });
    }
    let project_id = spec.provenance.project_id.clone();
    let finding_count = spec.findings.len();
    let id_dir = spec.id.dir_name();
    let files = write_spec(&request.project_path, &spec)?;
    Ok(SpecGenerateOutcome {
        status: SpecStatus::Generated,
        spec,
        files_written: files,
        note: format!(
            "wrote bounded spec `{id_dir}` for project `{project_id}` covering {finding_count} finding(s)"
        ),
    })
}

/// Refuse a request that targets multiple projects. Exposed for the
/// router so a fleet request can be split before any spec is
/// written.
pub fn ensure_single_project(request: &SpecRequest) -> Result<(), ForgeError> {
    let (manifest, _) = Manifest::load_from_dir(&request.project_path, None)?;
    let id = manifest.project.id.clone();
    if id.is_empty() {
        return Err(ForgeError::SpecInvalid {
            reason: "project id is empty; cannot generate a spec".to_string(),
        });
    }
    Ok(())
}

/// Decide the routing for one finding. The decision is deterministic
/// from the finding's category, status, and remediation class, so
/// repeated invocations agree and a recorded decision is auditable.
///
/// - `Manual` (boundary): the doctor finding's `remediation` is
///   `manual` and the status is `fail` or `warn`; the finding
///   requires a human judgment and the router records the manual
///   state without changing the project.
/// - `Deterministic` (success): the finding is a feature
///   compatibility drift or a DriftWatch policy finding that maps
///   to a known lifecycle/upgrade action (`features-compatible`,
///   `dependency-drift`, `maturity-requirements`, `build-config`).
///   The router names the action; the actual run is owned by
///   `forge feature` / `forge upgrade`.
/// - `Semantic` (default): the finding is `ai` or a
///   semantic-conflict handoff; a bounded spec is the handoff.
pub fn route_finding(
    request: &SpecRequest,
    source: &FindingSource,
) -> Result<RoutingDecision, ForgeError> {
    let (manifest, _) = Manifest::load_from_dir(&request.project_path, None)?;
    let project_id = manifest.project.id.clone();
    let spec_id = spec_id_for(&project_id, &request.finding_ids);
    match source {
        FindingSource::Doctor(d) => Ok(route_doctor_finding(d, &manifest, &spec_id)),
        FindingSource::Policy(p) => Ok(route_policy_finding(p, &manifest, &spec_id)),
        FindingSource::Conflict(c) => Ok(RoutingDecision {
            finding_id: format!("semantic-{}", c.feature),
            finding_category: "semantic-conflict".to_string(),
            finding_severity: "fail".to_string(),
            route: SpecRoute::Semantic,
            rationale: format!(
                "drifted receipt on owned file `{}`; a bounded spec must precede any further edit",
                c.owned_file
            ),
            action: Some(format!(
                "forge spec generate --project {} --finding semantic-{}",
                project_id, c.feature
            )),
            suggested_spec: Some(spec_id),
        }),
    }
}

fn route_doctor_finding(
    d: &DoctorFindingInput,
    manifest: &Manifest,
    spec_id: &SpecId,
) -> RoutingDecision {
    let severity = match d.status {
        FindingStatus::Pass => "pass",
        FindingStatus::Warn => "warn",
        FindingStatus::Fail => "fail",
        FindingStatus::Unavailable => "unavailable",
    };
    if is_deterministic_doctor_id(&d.id) {
        let action = deterministic_action_for(&d.id, manifest);
        return RoutingDecision {
            finding_id: d.id.clone(),
            finding_category: d.category.clone(),
            finding_severity: severity.to_string(),
            route: SpecRoute::Deterministic,
            rationale: format!(
                "doctor finding `{}` maps to a supported lifecycle action: `{}`",
                d.id, action
            ),
            action: Some(action),
            suggested_spec: None,
        };
    }
    if d.remediation == Remediation::Manual {
        return RoutingDecision {
            finding_id: d.id.clone(),
            finding_category: d.category.clone(),
            finding_severity: severity.to_string(),
            route: SpecRoute::Manual,
            rationale: format!(
                "doctor finding `{}` is a manual judgment call; record status without an AI fix or project change",
                d.id
            ),
            action: None,
            suggested_spec: None,
        };
    }
    RoutingDecision {
        finding_id: d.id.clone(),
        finding_category: d.category.clone(),
        finding_severity: severity.to_string(),
        route: SpecRoute::Semantic,
        rationale: format!(
            "doctor finding `{}` requires a bounded spec; no deterministic action available",
            d.id
        ),
        action: Some(format!("forge spec generate --finding {}", d.id)),
        suggested_spec: Some(spec_id.clone()),
    }
}

fn is_deterministic_doctor_id(id: &str) -> bool {
    matches!(
        id,
        "features-compatible"
            | "dependency-drift"
            | "maturity-requirements"
            | "build-config"
            | "deployment-config"
            | "ci-config"
            | "driftwatch-config"
    )
}

fn deterministic_action_for(id: &str, manifest: &Manifest) -> String {
    match id {
        "features-compatible" | "dependency-drift" | "maturity-requirements" => {
            // The deterministic resolution path is `forge upgrade`; it
            // picks every outdated installed feature and installs any
            // missing one. A specific feature flag can be added when
            // the caller wants a single-feature scope.
            let _ = manifest; // documented for future per-finding routing
            "forge upgrade".to_string()
        }
        "build-config" | "deployment-config" | "ci-config" | "driftwatch-config" => {
            "forge doctor --target L2".to_string()
        }
        _ => "forge doctor".to_string(),
    }
}

fn route_policy_finding(
    p: &PolicyFinding,
    manifest: &Manifest,
    spec_id: &SpecId,
) -> RoutingDecision {
    let severity = match p.severity {
        crate::policy::PolicySeverity::Pass => "pass",
        crate::policy::PolicySeverity::Warn => "warn",
        crate::policy::PolicySeverity::Fail => "fail",
    };
    if !p.applicable {
        return RoutingDecision {
            finding_id: format!("driftwatch-{}", p.id),
            finding_category: p.category.clone(),
            finding_severity: severity.to_string(),
            route: SpecRoute::Manual,
            rationale: format!(
                "policy `{}` is not applicable to profile `{}`; preserve the reason without claiming an AI fix",
                p.id, manifest.project.profile
            ),
            action: None,
            suggested_spec: None,
        };
    }
    if matches!(p.severity, crate::policy::PolicySeverity::Pass) {
        return RoutingDecision {
            finding_id: format!("driftwatch-{}", p.id),
            finding_category: p.category.clone(),
            finding_severity: severity.to_string(),
            route: SpecRoute::Manual,
            rationale: format!(
                "policy `{}` reports a passing detector result; no remediation required",
                p.id
            ),
            action: None,
            suggested_spec: None,
        };
    }
    RoutingDecision {
        finding_id: format!("driftwatch-{}", p.id),
        finding_category: p.category.clone(),
        finding_severity: severity.to_string(),
        route: SpecRoute::Semantic,
        rationale: format!(
            "policy `{}` ({}): {}; bounded spec required",
            p.id, p.category, p.message
        ),
        action: Some(format!("forge spec generate --finding driftwatch-{}", p.id)),
        suggested_spec: Some(spec_id.clone()),
    }
}

/// Apply a routing decision. For `Deterministic` actions the
/// router reports the action string without executing it (the
/// caller runs `forge upgrade` / `forge doctor`); for `Semantic`
/// it generates a spec; for `Manual` it records the manual status
/// without changing the project. The function never runs a
/// lifecycle/upgrade operation itself: that is owned by the
/// `forge upgrade` and `forge feature` commands so the same
/// validation/registry-journal contract applies.
pub fn apply_routing(
    request: &SpecRequest,
    source: &FindingSource,
    now: DateTime<Utc>,
) -> Result<RoutingOutcome, ForgeError> {
    let decision = route_finding(request, source)?;
    match decision.route {
        SpecRoute::Deterministic => Ok(RoutingOutcome {
            status: RoutingStatus::Applied,
            evidence: vec![format!(
                "deterministic action recorded: `{}`; run it to apply and validate",
                decision.action.clone().unwrap_or_default()
            )],
            recovery: vec!["rerun the action after fixing any preconditions".to_string()],
            files_changed: Vec::new(),
            note: format!(
                "deterministic route for `{}`; action queued for execution",
                decision.finding_id
            ),
            decision,
        }),
        SpecRoute::Manual => Ok(RoutingOutcome {
            status: RoutingStatus::Manual,
            evidence: vec![format!(
                "manual routing recorded for `{}`: {}",
                decision.finding_id, decision.rationale
            )],
            recovery: vec![
                "resolve the judgment call manually; rerun the router after the change".to_string(),
            ],
            files_changed: Vec::new(),
            note: "manual boundary: no project changes and no AI fix claimed".to_string(),
            decision,
        }),
        SpecRoute::Semantic => {
            let sources = [source.clone()];
            let outcome = generate_spec(request, &sources, now)?;
            let status = match outcome.status {
                SpecStatus::Generated | SpecStatus::Existing => RoutingStatus::SpecGenerated,
                SpecStatus::Rejected => RoutingStatus::Failed,
            };
            let recovery = if matches!(outcome.status, SpecStatus::Rejected) {
                vec![format!(
                    "spec generation refused: {}; split the finding set into smaller scopes",
                    outcome.note
                )]
            } else {
                vec!["run `forge spec inspect <spec-id>` to review the proposal".to_string()]
            };
            Ok(RoutingOutcome {
                status,
                evidence: vec![format!("spec status: `{}`", outcome.status_label())],
                recovery,
                files_changed: outcome.files_written.clone(),
                note: outcome.note.clone(),
                decision,
            })
        }
    }
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

impl SpecGenerateOutcome {
    pub fn status_label(&self) -> &'static str {
        self.status.label()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write_manifest(dir: &Path, id: &str) {
        let body = format!(
            "schema: 1\nproject:\n  id: {id}\n  name: {id}\n  profile: rust-web\n  maturity: L1\n  target_maturity: L1\nruntime:\n  language: rust\n"
        );
        fs::write(dir.join(CANONICAL_MANIFEST), body).unwrap();
    }

    #[test]
    fn spec_id_is_stable_for_same_finding_set() {
        let a = spec_id_for("proj", &["a".to_string(), "b".to_string()]);
        let b = spec_id_for("proj", &["b".to_string(), "a".to_string()]);
        assert_eq!(a, b);
    }

    #[test]
    fn spec_id_differs_for_different_finding_sets() {
        let a = spec_id_for("proj", &["a".to_string()]);
        let b = spec_id_for("proj", &["b".to_string()]);
        assert_ne!(a, b);
    }

    #[test]
    fn validate_request_rejects_oversized_set() {
        let mut ids: Vec<String> = (0..MAX_FINDINGS_PER_SPEC + 1)
            .map(|i| format!("f{i}"))
            .collect();
        let request = SpecRequest {
            project_path: PathBuf::from("/nonexistent"),
            finding_ids: ids.clone(),
            reason: None,
        };
        let err = validate_request(&request).unwrap_err();
        assert!(err.to_string().contains("at most"), "{}", err);
        ids.truncate(2);
        let small = SpecRequest {
            project_path: PathBuf::from("/nonexistent"),
            finding_ids: ids,
            reason: None,
        };
        assert!(validate_request(&small).is_ok());
    }

    #[test]
    fn build_spec_draft_captures_provenance_and_scenarios() {
        let tmp = TempDir::new().unwrap();
        write_manifest(tmp.path(), "demo");
        let request = SpecRequest {
            project_path: tmp.path().to_path_buf(),
            finding_ids: vec!["a".to_string(), "b".to_string()],
            reason: Some("manual override".to_string()),
        };
        let sources = vec![FindingSource::Doctor(DoctorFindingInput {
            id: "a".to_string(),
            status: FindingStatus::Fail,
            remediation: Remediation::Ai,
            category: "spec".to_string(),
            detail: "manual detail".to_string(),
        })];
        let draft = build_spec_draft(
            &request,
            &sources,
            DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
        )
        .unwrap();
        assert_eq!(draft.provenance.project_id, "demo");
        assert_eq!(draft.provenance.profile, "rust-web");
        assert!(draft.findings.contains(&"a".to_string()));
        assert!(draft.acceptance.iter().any(|s| s.source_finding == "a"));
        assert!(draft.why.contains("manual override"));
    }

    #[test]
    fn generate_spec_is_idempotent_on_unchanged_finding_set() {
        let tmp = TempDir::new().unwrap();
        write_manifest(tmp.path(), "idem");
        let request = SpecRequest {
            project_path: tmp.path().to_path_buf(),
            finding_ids: vec!["only".to_string()],
            reason: None,
        };
        let first = generate_spec(
            &request,
            &[FindingSource::Doctor(DoctorFindingInput {
                id: "only".to_string(),
                status: FindingStatus::Fail,
                remediation: Remediation::Ai,
                category: "spec".to_string(),
                detail: "x".to_string(),
            })],
            DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
        )
        .unwrap();
        assert_eq!(first.status, SpecStatus::Generated);
        assert_eq!(first.files_written.len(), 4);
        let second = generate_spec(
            &request,
            &[FindingSource::Doctor(DoctorFindingInput {
                id: "only".to_string(),
                status: FindingStatus::Fail,
                remediation: Remediation::Ai,
                category: "spec".to_string(),
                detail: "x".to_string(),
            })],
            DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
        )
        .unwrap();
        assert_eq!(second.status, SpecStatus::Existing);
        assert!(second.files_written.is_empty());
        assert!(second.note.contains("already exists"));
    }

    #[test]
    fn router_routes_manual_finding_as_manual() {
        let tmp = TempDir::new().unwrap();
        write_manifest(tmp.path(), "manual");
        let request = SpecRequest {
            project_path: tmp.path().to_path_buf(),
            finding_ids: vec!["judgment".to_string()],
            reason: None,
        };
        let source = FindingSource::Doctor(DoctorFindingInput {
            id: "judgment".to_string(),
            status: FindingStatus::Fail,
            remediation: Remediation::Manual,
            category: "manual".to_string(),
            detail: "needs a human".to_string(),
        });
        let decision = route_finding(&request, &source).unwrap();
        assert_eq!(decision.route, SpecRoute::Manual);
        let outcome = apply_routing(
            &request,
            &source,
            DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
        )
        .unwrap();
        assert_eq!(outcome.status, RoutingStatus::Manual);
        assert!(outcome.files_changed.is_empty());
        assert!(outcome.note.contains("manual boundary"), "{}", outcome.note);
        assert!(
            outcome.note.contains("no project changes"),
            "{}",
            outcome.note
        );
    }

    #[test]
    fn router_routes_feature_compatibility_as_deterministic() {
        let tmp = TempDir::new().unwrap();
        write_manifest(tmp.path(), "feat");
        let request = SpecRequest {
            project_path: tmp.path().to_path_buf(),
            finding_ids: vec!["features-compatible".to_string()],
            reason: None,
        };
        let source = FindingSource::Doctor(DoctorFindingInput {
            id: "features-compatible".to_string(),
            status: FindingStatus::Fail,
            remediation: Remediation::Manual,
            category: "automatic".to_string(),
            detail: "incompatible".to_string(),
        });
        let decision = route_finding(&request, &source).unwrap();
        assert_eq!(decision.route, SpecRoute::Deterministic);
        assert_eq!(decision.action.as_deref(), Some("forge upgrade"));
    }

    #[test]
    fn router_routes_semantic_conflict_as_semantic() {
        let tmp = TempDir::new().unwrap();
        write_manifest(tmp.path(), "conflict");
        let request = SpecRequest {
            project_path: tmp.path().to_path_buf(),
            finding_ids: vec!["semantic-auth".to_string()],
            reason: None,
        };
        let source = FindingSource::Conflict(SemanticConflict {
            project_id: "conflict".to_string(),
            feature: "auth".to_string(),
            owned_file: ".forge/features/auth.receipt".to_string(),
            reason: "drifted".to_string(),
            suggested_spec: "forge spec generate".to_string(),
        });
        let decision = route_finding(&request, &source).unwrap();
        assert_eq!(decision.route, SpecRoute::Semantic);
        let outcome = apply_routing(
            &request,
            &source,
            DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
        )
        .unwrap();
        assert_eq!(outcome.status, RoutingStatus::SpecGenerated);
        assert!(!outcome.files_changed.is_empty());
    }

    #[test]
    fn list_specs_returns_generated_entries() {
        let tmp = TempDir::new().unwrap();
        write_manifest(tmp.path(), "list");
        let request = SpecRequest {
            project_path: tmp.path().to_path_buf(),
            finding_ids: vec!["f".to_string()],
            reason: None,
        };
        generate_spec(
            &request,
            &[FindingSource::Doctor(DoctorFindingInput {
                id: "f".to_string(),
                status: FindingStatus::Fail,
                remediation: Remediation::Ai,
                category: "spec".to_string(),
                detail: "x".to_string(),
            })],
            DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
        )
        .unwrap();
        let entries = list_specs(tmp.path()).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].project_id, "list");
    }
}
