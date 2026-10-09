//! Component resolution and qualification.

use crate::core::ForgeError;
use chrono::{DateTime, Utc};
use std::fs;
use std::path::{Path, PathBuf};

use super::catalog::{component_catalog, inspect_component, is_primitive_id};
use super::contract::{
    CERTIFIED_FRESHNESS_DAYS, CERTIFIED_TEST_COVERAGE, COMPONENTS_DIR, COMPONENT_CATALOG_VERSION,
    MAX_KNOWN_ISSUES,
};
use super::model::{
    ComponentDescriptor, ComponentEvidenceSummary, ComponentPlan, ComponentQualifyEvidence,
    ComponentQualifyOutcome, ComponentQualifyRequest, ComponentQuality, ComponentRejection,
    ComponentRequest, ComponentResolveOutcome, ComponentStep,
};

/// Build a stable per-id evidence summary used by the resolver and
/// the inspector.
fn evidence_summary_for(descriptor: &ComponentDescriptor) -> ComponentEvidenceSummary {
    ComponentEvidenceSummary {
        id: descriptor.id.clone(),
        quality: descriptor.quality,
        evidence: descriptor.evidence.clone(),
    }
}

/// Quality precedence. Higher is preferred by the resolver. The
/// ordering is: `Certified` > `Verified` > `Experimental`; `Deprecated`
/// is intentionally not ordered and is handled separately so the
/// resolver reports a policy conflict rather than silently selecting
/// a deprecated candidate (R2 boundary scenario).
fn quality_rank(quality: ComponentQuality) -> u8 {
    match quality {
        ComponentQuality::Certified => 3,
        ComponentQuality::Verified => 2,
        ComponentQuality::Experimental => 1,
        ComponentQuality::Deprecated => 0,
    }
}

pub(super) fn select_strongest<'a>(
    candidates: &[&'a ComponentDescriptor],
) -> Option<&'a ComponentDescriptor> {
    candidates
        .iter()
        .copied()
        .filter(|c| c.quality != ComponentQuality::Deprecated)
        .max_by_key(|c| quality_rank(c.quality))
}

fn select_only_deprecated(candidates: &[&ComponentDescriptor]) -> Vec<ComponentRejection> {
    candidates
        .iter()
        .map(|c| ComponentRejection {
            id: c.id.to_string(),
            code: "component-quality-conflict".to_string(),
            reason: format!(
                "only candidate for component '{}' is deprecated; the planner refuses to \
                 silently select a deprecated descriptor",
                c.id
            ),
        })
        .collect()
}

/// Validate the request before any catalog lookup: profile must be
/// known, ids must be unique and non-empty, the request must contain
/// at least one id.
pub fn validate_request(request: &ComponentRequest) -> Result<(), ForgeError> {
    if request.profile.trim().is_empty() {
        return Err(ForgeError::ComponentInvalid {
            reason: "component request requires a profile".to_string(),
        });
    }
    if request.component_ids.is_empty() {
        return Err(ForgeError::ComponentInvalid {
            reason: "component request requires at least one component id".to_string(),
        });
    }
    let mut seen: Vec<&str> = Vec::with_capacity(request.component_ids.len());
    for id in &request.component_ids {
        let trimmed = id.trim();
        if trimmed.is_empty() {
            return Err(ForgeError::ComponentInvalid {
                reason: "component ids must not be empty".to_string(),
            });
        }
        if seen.contains(&trimmed) {
            return Err(ForgeError::ComponentInvalid {
                reason: format!("component id '{trimmed}' is duplicated in the request"),
            });
        }
        seen.push(trimmed);
        if is_primitive_id(trimmed) {
            return Err(ForgeError::ComponentInvalid {
                reason: format!(
                    "component '{trimmed}' is a programming primitive; the registry refuses \
                     to model language constructs"
                ),
            });
        }
    }
    Ok(())
}

/// Resolve `request` into a [`ComponentPlan`]. The resolver never
/// executes a side effect: it is a deterministic function of the
/// catalog, the request and the profile. Refusals are returned as
/// `rejections` so a partial plan stays reviewable; the request is
/// only refused outright when the input is malformed (R1 failure
/// scenario).
pub fn resolve_components(request: &ComponentRequest) -> Result<ComponentPlan, ForgeError> {
    validate_request(request)?;
    let catalog = component_catalog();
    let mut steps: Vec<ComponentStep> = Vec::new();
    let mut rejections: Vec<ComponentRejection> = Vec::new();
    let mut requested = request.component_ids.clone();
    requested.sort();
    requested.dedup();
    for id in &requested {
        let candidates: Vec<&ComponentDescriptor> =
            catalog.iter().filter(|c| &c.id == id).collect();
        if candidates.is_empty() {
            rejections.push(ComponentRejection {
                id: id.clone(),
                code: "component-invalid".to_string(),
                reason: format!("component '{id}' is not in the catalog"),
            });
            continue;
        }
        let compatible: Vec<&ComponentDescriptor> = candidates
            .iter()
            .copied()
            .filter(|c| c.profiles.iter().any(|p| p == &request.profile))
            .collect();
        if compatible.is_empty() {
            let tested: Vec<String> = candidates
                .iter()
                .flat_map(|c| c.profiles.iter().cloned())
                .collect();
            rejections.push(ComponentRejection {
                id: id.clone(),
                code: "component-invalid".to_string(),
                reason: format!(
                    "component '{id}' has no implementation for profile '{}' (tested: {})",
                    request.profile,
                    if tested.is_empty() {
                        "none".to_string()
                    } else {
                        tested.join(", ")
                    }
                ),
            });
            continue;
        }
        match select_strongest(&compatible) {
            Some(selected) => steps.push(ComponentStep {
                id: selected.id.clone(),
                version: selected.version.clone(),
                quality: selected.quality,
                action: "install".to_string(),
            }),
            None => rejections.extend(select_only_deprecated(&compatible)),
        }
    }
    steps.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(ComponentPlan {
        profile: request.profile.clone(),
        steps,
        rejections,
    })
}

/// Build a [`ComponentResolveOutcome`] from a request. The outcome
/// carries the plan plus the per-step evidence summary so the
/// transport can render the planner's reasoning.
pub fn resolve_outcome(request: &ComponentRequest) -> Result<ComponentResolveOutcome, ForgeError> {
    let plan = resolve_components(request)?;
    let catalog = component_catalog();
    let mut evidence_summary: Vec<ComponentEvidenceSummary> = Vec::new();
    for step in &plan.steps {
        if let Some(descriptor) = catalog.iter().find(|c| c.id == step.id) {
            evidence_summary.push(evidence_summary_for(descriptor));
        }
    }
    let note = if plan.steps.is_empty() && plan.rejections.is_empty() {
        "no components requested".to_string()
    } else if plan.steps.is_empty() {
        format!(
            "no components could be resolved for profile '{}' ({} rejection(s))",
            request.profile,
            plan.rejections.len()
        )
    } else {
        format!(
            "resolved {} component(s) for profile '{}'{}",
            plan.steps.len(),
            request.profile,
            if plan.rejections.is_empty() {
                String::new()
            } else {
                format!("; {} rejection(s)", plan.rejections.len())
            }
        )
    };
    Ok(ComponentResolveOutcome {
        profile: request.profile.clone(),
        plan,
        evidence_summary,
        note,
    })
}

fn receipt_path(dir: &Path, id: &str) -> PathBuf {
    dir.join(COMPONENTS_DIR).join(id).join("qualify.json")
}

fn evidence_is_fresh(last_verified: DateTime<Utc>) -> bool {
    let now = Utc::now().timestamp();
    let stamp = last_verified.timestamp();
    let age_days = (now - stamp).max(0) / 86_400;
    age_days <= CERTIFIED_FRESHNESS_DAYS
}

fn evidence_meets_certified_gate(evidence: &ComponentQualifyEvidence) -> Result<(), String> {
    if !evidence.security_review {
        return Err("missing security review".to_string());
    }
    if evidence.test_coverage < CERTIFIED_TEST_COVERAGE {
        return Err(format!(
            "test coverage {} is below the certified minimum {}",
            evidence.test_coverage, CERTIFIED_TEST_COVERAGE
        ));
    }
    if evidence.known_issues.len() > MAX_KNOWN_ISSUES {
        return Err(format!(
            "known issues list ({} entries) exceeds the certified maximum {}",
            evidence.known_issues.len(),
            MAX_KNOWN_ISSUES
        ));
    }
    if !evidence_is_fresh(evidence.last_verified) {
        return Err(format!(
            "last_verified {} is older than the certified freshness window of \
             {CERTIFIED_FRESHNESS_DAYS} days",
            evidence.last_verified.to_rfc3339()
        ));
    }
    Ok(())
}

/// Persist the promotion receipt under
/// `.forge/components/<id>/qualify.json`. The receipt is a pure
/// function of the request: a drifted receipt means the caller
/// tampered with the evidence record and the next promotion is
/// refused until the receipt is restored.
pub fn write_qualify_receipt(
    dir: &Path,
    request: &ComponentQualifyRequest,
    evidence: &ComponentQualifyEvidence,
) -> Result<Vec<String>, ForgeError> {
    let target = receipt_path(dir, &request.component_id);
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|err| ForgeError::ComponentQualityConflict {
            reason: format!(
                "cannot create receipt directory {}: {err}; prior state left unchanged",
                parent.display()
            ),
        })?;
    }
    let payload = serde_json::json!({
        "contract": COMPONENT_CATALOG_VERSION,
        "component_id": request.component_id,
        "target_quality": request.target_quality.label(),
        "reason": request.reason,
        "evidence": {
            "test_coverage": evidence.test_coverage,
            "last_verified": evidence.last_verified.to_rfc3339(),
            "known_issues": evidence.known_issues,
            "security_review": evidence.security_review,
        },
    });
    let body = serde_json::to_string_pretty(&payload).map_err(|err| {
        ForgeError::ComponentQualityConflict {
            reason: format!("cannot serialize qualify receipt: {err}"),
        }
    })?;
    fs::write(&target, body).map_err(|err| ForgeError::ComponentQualityConflict {
        reason: format!(
            "cannot write qualify receipt {}: {err}; prior state left unchanged",
            target.display()
        ),
    })?;
    Ok(vec![format!(
        "{COMPONENTS_DIR}/{}/qualify.json",
        request.component_id
    )])
}

/// Attempt to promote a component's quality level. The promotion is
/// gated by the new quality's evidence requirements. On failure the
/// receipt is not written and the descriptor's prior quality level
/// stays untouched (R2 failure scenario).
pub fn qualify_component(
    request: &ComponentQualifyRequest,
    evidence: &ComponentQualifyEvidence,
) -> Result<ComponentQualifyOutcome, ForgeError> {
    let descriptor = inspect_component(&request.component_id)?;
    let prior_quality = descriptor.quality;
    if request.target_quality == prior_quality {
        return Ok(ComponentQualifyOutcome {
            component_id: request.component_id.clone(),
            prior_quality,
            target_quality: request.target_quality,
            promoted: false,
            files_written: Vec::new(),
            note: format!(
                "component '{}' is already at quality '{}'; no change",
                request.component_id,
                request.target_quality.label()
            ),
        });
    }
    if request.target_quality == ComponentQuality::Deprecated {
        return Ok(ComponentQualifyOutcome {
            component_id: request.component_id.clone(),
            prior_quality,
            target_quality: request.target_quality,
            promoted: true,
            files_written: Vec::new(),
            note: format!(
                "component '{}' marked deprecated; no evidence gate required",
                request.component_id
            ),
        });
    }
    if request.target_quality == ComponentQuality::Certified {
        if let Err(reason) = evidence_meets_certified_gate(evidence) {
            return Ok(ComponentQualifyOutcome {
                component_id: request.component_id.clone(),
                prior_quality,
                target_quality: request.target_quality,
                promoted: false,
                files_written: Vec::new(),
                note: format!(
                    "promotion refused: {reason}; component '{}' remains at quality '{}'",
                    request.component_id,
                    prior_quality.label()
                ),
            });
        }
    }
    Ok(ComponentQualifyOutcome {
        component_id: request.component_id.clone(),
        prior_quality,
        target_quality: request.target_quality,
        promoted: true,
        files_written: Vec::new(),
        note: format!(
            "promotion accepted: component '{}' moves from '{}' to '{}' (reason: {})",
            request.component_id,
            prior_quality.label(),
            request.target_quality.label(),
            request.reason
        ),
    })
}

/// Persist a qualified promotion. The outcome's `files_written` is
/// filled in with the relative receipt path on success; on refusal
/// the receipt is not written and the prior quality level remains
/// (R2 failure scenario).
pub fn record_qualification(
    dir: &Path,
    request: &ComponentQualifyRequest,
    evidence: &ComponentQualifyEvidence,
) -> Result<ComponentQualifyOutcome, ForgeError> {
    let outcome = qualify_component(request, evidence)?;
    if !outcome.promoted {
        return Ok(outcome);
    }
    if request.target_quality == ComponentQuality::Deprecated {
        return Ok(outcome);
    }
    let written = write_qualify_receipt(dir, request, evidence)?;
    Ok(ComponentQualifyOutcome {
        files_written: written,
        ..outcome
    })
}

/// Render a plan for human output.
pub fn render_plan_human(plan: &ComponentPlan) -> String {
    let mut lines = vec![format!(
        "plan for profile '{}' (catalog {})",
        plan.profile, COMPONENT_CATALOG_VERSION
    )];
    if plan.steps.is_empty() {
        lines.push("steps: none".to_string());
    } else {
        for step in &plan.steps {
            lines.push(format!(
                "  {} {}@{} ({})",
                step.action,
                step.id,
                step.version,
                step.quality.label()
            ));
        }
    }
    if !plan.rejections.is_empty() {
        lines.push("rejections:".to_string());
        for rejection in &plan.rejections {
            lines.push(format!("  {} -> {}", rejection.id, rejection.reason));
        }
    }
    lines.join("\n")
}

/// Render an outcome for human output.
pub fn render_outcome_human(outcome: &ComponentResolveOutcome) -> String {
    let mut lines = vec![
        format!("component resolve: {}", outcome.note),
        render_plan_human(&outcome.plan),
    ];
    if !outcome.evidence_summary.is_empty() {
        lines.push("evidence:".to_string());
        for entry in &outcome.evidence_summary {
            lines.push(format!(
                "  {} ({}) usage={} coverage={:.2} security_review={} last_verified={}",
                entry.id,
                entry.quality.label(),
                entry.evidence.usage_count,
                entry.evidence.test_coverage,
                entry.evidence.security_review,
                entry.evidence.last_verified.to_rfc3339()
            ));
        }
    }
    lines.join("\n")
}

/// Render a promotion outcome for human output.
pub fn render_qualify_human(outcome: &ComponentQualifyOutcome) -> String {
    let mut lines = vec![outcome.note.clone()];
    if !outcome.files_written.is_empty() {
        lines.push(format!("files: {}", outcome.files_written.join(", ")));
    } else {
        lines.push("files: (none)".to_string());
    }
    lines.push(format!(
        "prior: {} target: {} promoted: {}",
        outcome.prior_quality.label(),
        outcome.target_quality.label(),
        outcome.promoted
    ));
    lines.join("\n")
}
