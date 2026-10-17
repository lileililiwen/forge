//! One shared Gate read projection (`gate-declaration-migration`).
//!
//! Every reader — `forge gate status`, doctor, the checker document,
//! release preflight, the admin API and the workbench — consumes the
//! projection built here instead of loading evidence, attributing rows or
//! branching on freshness itself. [`read_gate_status`] preflights the
//! declarations *first* (malformed declarations win over absent
//! evidence), then loads the optional persisted record, then projects it
//! with the already validated context. [`project_gate_evidence`] performs
//! no further declaration or filesystem I/O, and it never rewrites the
//! stored record.

use std::fs;
use std::path::Path;

use crate::core::ForgeError;

use super::declaration::{preflight_gate_declarations, GateDeclarationContext};
use super::engine::evidence_path;
use super::model::{
    GateAggregate, GateCheck, GateCheckView, GateEvidence, GateEvidenceView, GateFreshness,
    GateFreshnessAssessment, GateStatusProjection, PackExpectation,
};

/// The detail every reader shows when no record exists: absence is
/// `unverified`, and `unverified` is never a pass.
const NEVER_RUN_DETAIL: &str = "the gate has never run; unverified is not a pass";

/// Read the shared Gate projection for one project.
///
/// Declaration preflight runs first, so a malformed current declaration
/// returns its typed error before evidence is loaded or absence is
/// projected. Valid declarations with no persisted record produce the
/// unverified/absent projection.
pub fn read_gate_status(
    project_root: &Path,
    project_id: &str,
    current_revision: Option<&str>,
) -> Result<GateStatusProjection, ForgeError> {
    let context = preflight_gate_declarations(project_root)?;
    match load_project_evidence(project_root, project_id)? {
        Some(evidence) => {
            let view = project_gate_evidence(&evidence, current_revision, &context);
            Ok(GateStatusProjection {
                freshness: view.freshness,
                aggregate: evidence.aggregate.label().to_string(),
                detail: projection_detail(&view),
                gate: Some(view),
            })
        }
        None => Ok(GateStatusProjection {
            gate: None,
            freshness: GateFreshness::Absent,
            aggregate: "unverified".to_string(),
            detail: NEVER_RUN_DETAIL.to_string(),
        }),
    }
}

/// Project one persisted record with the already validated declarations.
/// No declaration or filesystem read happens here.
pub fn project_gate_evidence(
    evidence: &GateEvidence,
    current_revision: Option<&str>,
    context: &GateDeclarationContext,
) -> GateEvidenceView {
    let assessment =
        assess_evidence_freshness(evidence, current_revision, context.pack_expectation());
    GateEvidenceView {
        contract: evidence.contract.clone(),
        project_id: evidence.project_id.clone(),
        runtime: evidence.runtime.clone(),
        runtime_version: evidence.runtime_version.clone(),
        revision: evidence.revision.clone(),
        manifest_digest: evidence.manifest_digest.clone(),
        rule_pack_version: evidence.rule_pack_version.clone(),
        aggregate: evidence.aggregate,
        checks: evidence
            .checks
            .iter()
            .map(project_check)
            .collect::<Vec<GateCheckView>>(),
        observed_at: evidence.observed_at.clone(),
        dry_run: evidence.dry_run,
        note: evidence.note.clone(),
        runtime_status: evidence.runtime_status.clone(),
        runtime_blocked: evidence.runtime_blocked,
        runtime_exit_code: evidence.runtime_exit_code,
        failures: evidence.failures.clone(),
        pending_reviews: evidence.pending_reviews.clone(),
        not_applicable: evidence.not_applicable.clone(),
        unbound: evidence.unbound.clone(),
        freshness: assessment.state,
        freshness_reasons: assessment.reasons,
        pack_expectation: assessment.pack_expectation,
    }
}

/// Assess persisted evidence against the current revision and the
/// declared pack expectation, with reasons in deterministic order.
///
/// The revision/aggregate rule is preserved exactly, then the pack
/// expectation applies: a matching declaration adds no reason, a
/// contradicting or unrecorded observed identity forces `stale`, and an
/// undeclared expectation changes nothing while saying so.
pub fn assess_evidence_freshness(
    evidence: &GateEvidence,
    current_revision: Option<&str>,
    expectation: &PackExpectation,
) -> GateFreshnessAssessment {
    let mut reasons: Vec<String> = Vec::new();
    let mut state = match evidence.revision.as_deref().filter(|sha| !sha.is_empty()) {
        None => {
            reasons.push("evidence revision unrecorded".to_string());
            GateFreshness::Stale
        }
        Some(bound) => match current_revision.filter(|sha| !sha.is_empty()) {
            None => {
                reasons.push("current revision unavailable".to_string());
                GateFreshness::Stale
            }
            Some(current) if current != bound => {
                reasons.push(format!(
                    "revision mismatch: evidence {bound}, current {current}"
                ));
                GateFreshness::Stale
            }
            Some(_) => match evidence.aggregate {
                GateAggregate::Passed | GateAggregate::Blocked | GateAggregate::Failed => {
                    GateFreshness::Fresh
                }
                // An unclassifiable aggregate is never current verification.
                GateAggregate::Unknown => {
                    reasons.push("aggregate unknown".to_string());
                    GateFreshness::Stale
                }
            },
        },
    };

    match expectation.identity.as_deref() {
        Some(expected) => {
            if evidence.rule_pack_version.as_deref() != Some(expected) {
                state = GateFreshness::Stale;
                reasons.push(format!(
                    "rule pack mismatch: expected {expected} from {} ({}), observed {}",
                    expectation.source.label(),
                    expectation.provenance.as_deref().unwrap_or("<unrecorded>"),
                    evidence
                        .rule_pack_version
                        .as_deref()
                        .unwrap_or("<unrecorded>")
                ));
            }
        }
        None => reasons.push("pack expectation undeclared".to_string()),
    }

    GateFreshnessAssessment {
        state,
        reasons,
        pack_expectation: expectation.clone(),
    }
}

/// Render the projected record as human text. Rows show id, state,
/// runtime source, severity, implementation source, the optional stable
/// path and the note; the raw runtime facts, the freshness reasons and
/// the pack expectation follow the existing facts.
pub fn render_gate_view_human(view: &GateEvidenceView, persisted: Option<&str>) -> String {
    let mut lines = Vec::new();
    lines.push(format!("project: {}", view.project_id));
    lines.push(format!("aggregate: {}", view.aggregate.label()));
    lines.push(format!("freshness: {}", view.freshness.label()));
    lines.push(format!(
        "runtime: {} ({})",
        view.runtime,
        view.runtime_version.as_deref().unwrap_or("unknown-version")
    ));
    lines.push(format!(
        "revision: {}",
        view.revision
            .as_deref()
            .map(|sha| sha.chars().take(12).collect::<String>())
            .unwrap_or_else(|| "unbound".to_string())
    ));
    lines.push(format!("observed_at: {}", view.observed_at));
    lines.push(format!(
        "runtime_status: {}",
        view.runtime_status.as_deref().unwrap_or("unrecorded")
    ));
    lines.push(format!(
        "runtime_blocked: {}",
        view.runtime_blocked
            .map(|blocked| blocked.to_string())
            .unwrap_or_else(|| "unrecorded".to_string())
    ));
    lines.push(format!(
        "runtime_exit_code: {}",
        view.runtime_exit_code
            .map(|code| code.to_string())
            .unwrap_or_else(|| "unrecorded".to_string())
    ));
    if let Some(digest) = &view.manifest_digest {
        lines.push(format!("manifest_digest: {digest}"));
    }
    if let Some(pack) = &view.rule_pack_version {
        lines.push(format!("rule_pack_version: {pack}"));
    }
    if !view.checks.is_empty() {
        lines.push("checks:".to_string());
        for check in &view.checks {
            let mut row = format!(
                "  - {} {} (runtime_source={} severity={} implementation_source={}",
                check.id,
                check.state.label(),
                check.runtime_source.as_deref().unwrap_or("unrecorded"),
                check.severity.as_deref().unwrap_or("unrecorded"),
                check.implementation_source
            );
            if let Some(path) = &check.implementation_path {
                row.push_str(&format!(" path={path}"));
            }
            row.push(')');
            lines.push(row);
            if let Some(note) = &check.note {
                lines.push(format!("    note: {note}"));
            }
        }
    }
    if view.freshness_reasons.is_empty() {
        lines.push("freshness_reasons: none".to_string());
    } else {
        lines.push("freshness_reasons:".to_string());
        for reason in &view.freshness_reasons {
            lines.push(format!("  - {reason}"));
        }
    }
    match (
        &view.pack_expectation.identity,
        &view.pack_expectation.provenance,
    ) {
        (Some(identity), provenance) => lines.push(format!(
            "pack_expectation: {identity} ({}) from {}",
            view.pack_expectation.source.label(),
            provenance.as_deref().unwrap_or("<unrecorded>")
        )),
        (None, _) => lines.push(format!(
            "pack_expectation: undeclared ({})",
            view.pack_expectation.source.label()
        )),
    }
    if let Some(note) = &view.note {
        lines.push(format!("note: {note}"));
    }
    if let Some(path) = persisted {
        lines.push(format!("persisted: {path}"));
    }
    lines.join("\n")
}

/// Project one stored row. A missing stored attribution renders
/// `unrecorded`; the stored record itself is never touched.
fn project_check(check: &GateCheck) -> GateCheckView {
    GateCheckView {
        id: check.id.clone(),
        state: check.state,
        runtime_source: check.runtime_source.clone(),
        severity: check.severity.clone(),
        implementation_source: check
            .implementation_source
            .map(|source| source.label().to_string())
            .unwrap_or_else(|| "unrecorded".to_string()),
        implementation_path: check.implementation_path.clone(),
        note: check.note.clone(),
    }
}

/// Load the persisted record for exactly this project identity, or `None`
/// when the project has never produced one. A present but unreadable
/// record is `gate-invalid` naming the file — never silently absent.
fn load_project_evidence(
    project_root: &Path,
    project_id: &str,
) -> Result<Option<GateEvidence>, ForgeError> {
    let path = project_root.join(evidence_path(project_id));
    if !path.is_file() {
        return Ok(None);
    }
    let bytes = fs::read(&path).map_err(|err| ForgeError::GateInvalid {
        reason: format!("cannot read gate evidence {}: {err}", path.display()),
    })?;
    let evidence = serde_json::from_slice(&bytes).map_err(|err| ForgeError::GateInvalid {
        reason: format!(
            "gate evidence at {} is not a valid GateEvidence record: {err}",
            path.display()
        ),
    })?;
    Ok(Some(evidence))
}

/// The one-line detail for a record that exists.
pub fn projection_detail(view: &GateEvidenceView) -> String {
    match view.freshness {
        GateFreshness::Fresh => format!(
            "gate evidence is fresh at revision {}",
            view.revision.as_deref().unwrap_or("unrecorded")
        ),
        GateFreshness::Stale if view.freshness_reasons.is_empty() => {
            "gate evidence is stale".to_string()
        }
        GateFreshness::Stale => format!(
            "gate evidence is stale: {}",
            view.freshness_reasons.join("; ")
        ),
        GateFreshness::Absent => NEVER_RUN_DETAIL.to_string(),
    }
}
