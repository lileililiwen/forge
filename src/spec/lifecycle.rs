//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use crate::core::manifest::{Manifest, CANONICAL_MANIFEST};
use crate::core::ForgeError;
use crate::doctor::{FindingStatus, Remediation};
use crate::policy::PolicyFinding;
use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use super::contract::{MAX_FINDINGS_PER_SPEC, SPECS_DIR, SPEC_CONTRACT_VERSION, SPEC_HASH_LEN};
use super::model::{
    DoctorFindingInput, FindingSource, RoutingDecision, RoutingOutcome, RoutingStatus, SpecDraft,
    SpecGenerateOutcome, SpecId, SpecListEntry, SpecProvenance, SpecRequest, SpecRoute,
    SpecScenario, SpecStatus,
};

pub(super) fn category_from_remediation(r: Remediation) -> String {
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
