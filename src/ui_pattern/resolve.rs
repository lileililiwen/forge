//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use crate::core::ForgeError;
use std::path::{Path, PathBuf};

use super::constants::UI_PATTERNS_DIR;
use super::model::{
    UiPatternAdapter, UiPatternDescriptor, UiPatternEvidenceSummary, UiPatternPlan,
    UiPatternQuality, UiPatternRejection, UiPatternRequest, UiPatternResolveOutcome, UiPatternStep,
};
use super::patterns::ui_pattern_catalog;
use super::validate::is_primitive_id;

/// Look up one descriptor by id. Returns [`UiPatternInvalid`]
/// with the `unknown-ui-pattern` wording so a caller can
/// distinguish "no such pattern" from a contract-shape failure.
pub fn inspect_ui_pattern(id: &str) -> Result<UiPatternDescriptor, ForgeError> {
    ui_pattern_catalog()
        .into_iter()
        .find(|c| c.id == id)
        .ok_or_else(|| ForgeError::UiPatternInvalid {
            reason: format!("ui pattern '{id}' is not in the catalog"),
        })
}

/// Build a stable per-id evidence summary used by the resolver
/// and the inspector.
fn evidence_summary_for(descriptor: &UiPatternDescriptor) -> UiPatternEvidenceSummary {
    UiPatternEvidenceSummary {
        id: descriptor.id.clone(),
        quality: descriptor.quality,
        evidence: descriptor.evidence.clone(),
    }
}

/// Quality precedence. Higher is preferred by the resolver.
/// `Deprecated` is intentionally not ordered and is handled
/// separately so the resolver reports a policy conflict rather
/// than silently selecting a deprecated candidate.
fn quality_rank(quality: UiPatternQuality) -> u8 {
    match quality {
        UiPatternQuality::Certified => 3,
        UiPatternQuality::Verified => 2,
        UiPatternQuality::Experimental => 1,
        UiPatternQuality::Deprecated => 0,
    }
}

fn select_strongest<'a>(candidates: &[&'a UiPatternDescriptor]) -> Option<&'a UiPatternDescriptor> {
    candidates
        .iter()
        .copied()
        .filter(|c| c.quality != UiPatternQuality::Deprecated)
        .max_by_key(|c| quality_rank(c.quality))
}

fn select_only_deprecated(candidates: &[&UiPatternDescriptor]) -> Vec<UiPatternRejection> {
    candidates
        .iter()
        .map(|c| UiPatternRejection {
            id: c.id.clone(),
            code: "ui-pattern-quality-conflict".to_string(),
            reason: format!(
                "only candidate for ui pattern '{}' is deprecated; the planner refuses to \
                 silently select a deprecated pattern",
                c.id
            ),
        })
        .collect()
}

/// Validate the request before any catalog lookup: profile
/// must be known, ids must be unique and non-empty, the
/// request must contain at least one id, and every id must
/// not be a programming primitive.
pub fn validate_request(request: &UiPatternRequest) -> Result<(), ForgeError> {
    if request.profile.trim().is_empty() {
        return Err(ForgeError::UiPatternInvalid {
            reason: "ui pattern request requires a profile".to_string(),
        });
    }
    if request.pattern_ids.is_empty() {
        return Err(ForgeError::UiPatternInvalid {
            reason: "ui pattern request requires at least one pattern id".to_string(),
        });
    }
    let mut seen: Vec<&str> = Vec::with_capacity(request.pattern_ids.len());
    for id in &request.pattern_ids {
        let trimmed = id.trim();
        if trimmed.is_empty() {
            return Err(ForgeError::UiPatternInvalid {
                reason: "ui pattern ids must not be empty".to_string(),
            });
        }
        if seen.contains(&trimmed) {
            return Err(ForgeError::UiPatternInvalid {
                reason: format!("ui pattern id '{trimmed}' is duplicated in the request"),
            });
        }
        seen.push(trimmed);
        if is_primitive_id(trimmed) {
            return Err(ForgeError::UiPatternInvalid {
                reason: format!(
                    "ui pattern '{trimmed}' is a programming primitive or a generic \
                     template placeholder; the registry refuses to model language \
                     constructs or copied markup fragments"
                ),
            });
        }
    }
    Ok(())
}

/// Resolve `request` into a [`UiPatternPlan`]. The resolver
/// never executes a side effect: it is a deterministic
/// function of the catalog, the request and the profile.
/// Refusals are returned as `rejections` so a partial plan
/// stays reviewable; the request is only refused outright
/// when the input is malformed.
pub fn resolve_patterns(request: &UiPatternRequest) -> Result<UiPatternPlan, ForgeError> {
    validate_request(request)?;
    let catalog = ui_pattern_catalog();
    let mut steps: Vec<UiPatternStep> = Vec::new();
    let mut rejections: Vec<UiPatternRejection> = Vec::new();
    let mut requested = request.pattern_ids.clone();
    requested.sort();
    requested.dedup();
    for id in &requested {
        let candidates: Vec<&UiPatternDescriptor> =
            catalog.iter().filter(|c| &c.id == id).collect();
        if candidates.is_empty() {
            rejections.push(UiPatternRejection {
                id: id.clone(),
                code: "ui-pattern-invalid".to_string(),
                reason: format!("ui pattern '{id}' is not in the catalog"),
            });
            continue;
        }
        let compatible: Vec<&UiPatternDescriptor> = candidates
            .iter()
            .copied()
            .filter(|c| c.adapters.iter().any(|a| a.profile == request.profile))
            .collect();
        if compatible.is_empty() {
            let tested: Vec<String> = candidates
                .iter()
                .flat_map(|c| c.adapters.iter().map(|a| a.profile.clone()))
                .collect();
            rejections.push(UiPatternRejection {
                id: id.clone(),
                code: "ui-pattern-unsupported-platform".to_string(),
                reason: format!(
                    "ui pattern '{id}' has no implementation for profile '{}' (tested: {})",
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
            Some(selected) => steps.push(UiPatternStep {
                id: selected.id.clone(),
                version: selected.version.clone(),
                quality: selected.quality,
                action: "install".to_string(),
            }),
            None => rejections.extend(select_only_deprecated(&compatible)),
        }
    }
    steps.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(UiPatternPlan {
        profile: request.profile.clone(),
        steps,
        rejections,
    })
}

/// Build a [`UiPatternResolveOutcome`] from a request. The
/// outcome carries the plan plus the per-step evidence summary
/// so the transport can render the planner's reasoning.
pub fn resolve_outcome(request: &UiPatternRequest) -> Result<UiPatternResolveOutcome, ForgeError> {
    let plan = resolve_patterns(request)?;
    let catalog = ui_pattern_catalog();
    let mut evidence_summary: Vec<UiPatternEvidenceSummary> = Vec::new();
    for step in &plan.steps {
        if let Some(descriptor) = catalog.iter().find(|c| c.id == step.id) {
            evidence_summary.push(evidence_summary_for(descriptor));
        }
    }
    let note = if plan.steps.is_empty() && plan.rejections.is_empty() {
        "no ui patterns requested".to_string()
    } else if plan.steps.is_empty() {
        format!(
            "no ui patterns could be resolved for profile '{}' ({} rejection(s))",
            request.profile,
            plan.rejections.len()
        )
    } else {
        format!(
            "resolved {} ui pattern(s) for profile '{}'{}",
            plan.steps.len(),
            request.profile,
            if plan.rejections.is_empty() {
                String::new()
            } else {
                format!("; {} rejection(s)", plan.rejections.len())
            }
        )
    };
    Ok(UiPatternResolveOutcome {
        profile: request.profile.clone(),
        plan,
        evidence_summary,
        note,
    })
}

pub(super) fn receipt_path(dir: &Path, id: &str) -> PathBuf {
    dir.join(UI_PATTERNS_DIR).join(id).join("install.json")
}

pub(super) fn adapter_for_profile<'a>(
    descriptor: &'a UiPatternDescriptor,
    profile: &str,
) -> Option<&'a UiPatternAdapter> {
    descriptor.adapters.iter().find(|a| a.profile == profile)
}
