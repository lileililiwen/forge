//! Planner plan resolution.

use crate::component::{component_catalog, resolve_outcome as resolve_components};
use crate::core::ForgeError;
use crate::feature::{feature_catalog, resolve_plan as resolve_features};
use crate::profile::inspect_profile;
use crate::ui_pattern::{resolve_outcome as resolve_ui_patterns, ui_pattern_catalog};
use std::collections::BTreeSet;

use super::contract::MAX_UNRESOLVED_PER_PLAN;
use super::model::{AssemblyPlan, PlanStep, PlanStepKind, UnresolvedWork, ValidatedIntent};
use super::validate::{catalog_hash, intent_hash, profile_is_client_only};

/// Resolve a validated intent into a reviewable assembly plan. The
/// plan is deterministic from the validated intent and the catalog
/// (a re-resolve of the same intent produces a plan with the same
/// `intent_hash` and `catalog_hash`); the executor re-checks the
/// captured hashes before applying the plan (R2 failure scenario).
pub fn resolve_plan(
    validated: &ValidatedIntent,
    source_revision: Option<String>,
) -> Result<AssemblyPlan, ForgeError> {
    let profile = inspect_profile(&validated.profile_id).map_err(|_| ForgeError::PlanConflict {
        reason: format!(
            "validated intent profile '{}' is no longer in the catalog; revalidate the \
             intent before resolving",
            validated.profile_id
        ),
    })?;
    if profile.version != validated.profile_version {
        return Err(ForgeError::PlanConflict {
            reason: format!(
                "validated intent profile '{}' is at version '{}' but the catalog is at \
                 version '{}'; revalidate the intent before resolving",
                validated.profile_id, validated.profile_version, profile.version
            ),
        });
    }

    let mut steps: Vec<PlanStep> = Vec::new();
    let mut unresolved: Vec<UnresolvedWork> = Vec::new();

    let features: Vec<String> = validated
        .intent
        .required_capabilities
        .iter()
        .filter(|c| feature_catalog().iter().any(|f| &f.id == *c))
        .cloned()
        .collect();
    let components: Vec<String> = validated
        .intent
        .required_capabilities
        .iter()
        .filter(|c| component_catalog().iter().any(|cmp| &cmp.id == *c))
        .cloned()
        .collect();
    let ui_patterns: Vec<String> = validated
        .intent
        .required_capabilities
        .iter()
        .filter(|c| ui_pattern_catalog().iter().any(|p| &p.id == *c))
        .cloned()
        .collect();
    let mut accounted: BTreeSet<String> = BTreeSet::new();
    for f in &features {
        accounted.insert(f.clone());
    }
    for c in &components {
        accounted.insert(c.clone());
    }
    for p in &ui_patterns {
        accounted.insert(p.clone());
    }
    for cap in &validated.intent.required_capabilities {
        if !accounted.contains(cap) {
            if unresolved.len() >= MAX_UNRESOLVED_PER_PLAN {
                return Err(ForgeError::PlanConflict {
                    reason: format!(
                        "validated intent has more than {MAX_UNRESOLVED_PER_PLAN} \
                         unresolved requirements; the planner refuses to emit a truncated plan"
                    ),
                });
            }
            unresolved.push(UnresolvedWork {
                requirement: cap.clone(),
                reason: format!(
                    "capability '{cap}' has no deterministic component, feature or UI \
                     pattern in the catalog"
                ),
                hint: format!(
                    "record a semantic spec for '{cap}' or add a deterministic descriptor; \
                     the planner will not invent an AI substitution"
                ),
            });
        }
    }

    if !features.is_empty() {
        let plan = resolve_features(&validated.profile_id, &features).map_err(|e| {
            ForgeError::PlanConflict {
                reason: format!(
                    "feature resolver refused the validated intent: {e}; the planner did \
                     not produce an assembly plan"
                ),
            }
        })?;
        for step in plan.steps {
            steps.push(PlanStep {
                kind: PlanStepKind::InstallFeature,
                target: step.feature.clone(),
                version: step.version.clone(),
                action: step.action.clone(),
                evidence: format!(
                    "feature descriptor {}@{} installed by '{}'",
                    step.feature, step.version, step.action
                ),
            });
        }
    }

    if !components.is_empty() {
        let request = crate::component::ComponentRequest {
            profile: validated.profile_id.clone(),
            component_ids: components.clone(),
        };
        let outcome = resolve_components(&request).map_err(|e| ForgeError::PlanConflict {
            reason: format!(
                "component resolver refused the validated intent: {e}; the planner did \
                 not produce an assembly plan"
            ),
        })?;
        for step in outcome.plan.steps {
            steps.push(PlanStep {
                kind: PlanStepKind::InstallComponent,
                target: step.id.clone(),
                version: step.version.clone(),
                action: step.action.clone(),
                evidence: format!(
                    "component descriptor {}@{} (quality {}) selected by the resolver",
                    step.id,
                    step.version,
                    step.quality.label()
                ),
            });
        }
        for rejection in &outcome.plan.rejections {
            if unresolved.len() >= MAX_UNRESOLVED_PER_PLAN {
                return Err(ForgeError::PlanConflict {
                    reason: format!(
                        "validated intent produced more than {MAX_UNRESOLVED_PER_PLAN} \
                         unresolved requirements; the planner refuses to emit a truncated plan"
                    ),
                });
            }
            unresolved.push(UnresolvedWork {
                requirement: rejection.id.clone(),
                reason: format!(
                    "component resolver rejected '{}' with code '{}': {}",
                    rejection.id, rejection.code, rejection.reason
                ),
                hint: format!(
                    "record a semantic spec for '{}' or remove it from the intent; the \
                     planner will not invent a substitute",
                    rejection.id
                ),
            });
        }
    }

    if !ui_patterns.is_empty() {
        let request = crate::ui_pattern::UiPatternRequest {
            profile: validated.profile_id.clone(),
            pattern_ids: ui_patterns.clone(),
        };
        let outcome = resolve_ui_patterns(&request).map_err(|e| ForgeError::PlanConflict {
            reason: format!(
                "ui pattern resolver refused the validated intent: {e}; the planner did \
                 not produce an assembly plan"
            ),
        })?;
        for step in outcome.plan.steps {
            steps.push(PlanStep {
                kind: PlanStepKind::InstallUiPattern,
                target: step.id.clone(),
                version: step.version.clone(),
                action: step.action.clone(),
                evidence: format!(
                    "ui pattern descriptor {}@{} (quality {}) selected by the resolver",
                    step.id,
                    step.version,
                    step.quality.label()
                ),
            });
        }
        for rejection in &outcome.plan.rejections {
            if unresolved.len() >= MAX_UNRESOLVED_PER_PLAN {
                return Err(ForgeError::PlanConflict {
                    reason: format!(
                        "validated intent produced more than {MAX_UNRESOLVED_PER_PLAN} \
                         unresolved requirements; the planner refuses to emit a truncated \
                         plan"
                    ),
                });
            }
            unresolved.push(UnresolvedWork {
                requirement: rejection.id.clone(),
                reason: format!(
                    "ui pattern resolver rejected '{}' with code '{}': {}",
                    rejection.id, rejection.code, rejection.reason
                ),
                hint: format!(
                    "record a semantic spec for '{}' or remove it from the intent; the \
                     planner will not invent a substitute",
                    rejection.id
                ),
            });
        }
    }

    if profile.build_command.trim().is_empty() {
        if unresolved.len() >= MAX_UNRESOLVED_PER_PLAN {
            return Err(ForgeError::PlanConflict {
                reason: format!(
                    "validated intent produced more than {MAX_UNRESOLVED_PER_PLAN} \
                     unresolved requirements; the planner refuses to emit a truncated plan"
                ),
            });
        }
        unresolved.push(UnresolvedWork {
            requirement: format!("build:{}", profile.id),
            reason: format!(
                "profile '{}' has no native build command in its descriptor",
                profile.id
            ),
            hint: "register a tested build command in the profile descriptor".to_string(),
        });
    } else {
        steps.push(PlanStep {
            kind: PlanStepKind::Test,
            target: profile.id.clone(),
            version: profile.version.clone(),
            action: profile.test_command.clone(),
            evidence: format!(
                "profile '{}' test command '{}' is the planner's validation gate",
                profile.id, profile.test_command
            ),
        });
    }

    if !profile.quality_policies.is_empty() {
        steps.push(PlanStep {
            kind: PlanStepKind::QualityPolicy,
            target: profile.id.clone(),
            version: profile.version.clone(),
            action: format!(
                "driftwatch --project . --policies {}",
                profile.quality_policies.join(",")
            ),
            evidence: format!(
                "profile '{}' declares {} quality policy(ies); the planner schedules \
                 DriftWatch after the deterministic assets are installed",
                profile.id,
                profile.quality_policies.len()
            ),
        });
    }

    steps.push(PlanStep {
        kind: PlanStepKind::Doctor,
        target: profile.id.clone(),
        version: profile.version.clone(),
        action: format!(
            "forge doctor --target L{}",
            profile_default_target(&profile.id)
        ),
        evidence: format!(
            "doctor runs after the deterministic assets and the configured quality \
             policies; the maturity target is the profile's default L{}",
            profile_default_target(&profile.id)
        ),
    });

    let intent_hash_value = intent_hash(&validated.intent);
    let catalog_hash_value = catalog_hash();
    let plan_id = format!(
        "{}-{}-{}",
        validated.profile_id,
        &intent_hash_value[..8],
        &catalog_hash_value[..8]
    );
    let note = format!(
        "deterministic assembly plan for {}: {} step(s), {} unresolved, profile {}@{}",
        validated.intent.action.label(),
        steps.len(),
        unresolved.len(),
        profile.id,
        profile.version
    );
    Ok(AssemblyPlan {
        plan_id,
        profile: profile.id,
        profile_version: profile.version,
        intent: validated.intent.clone(),
        intent_hash: intent_hash_value,
        catalog_hash: catalog_hash_value,
        source_revision,
        steps,
        unresolved,
        note,
    })
}

fn profile_default_target(profile_id: &str) -> &'static str {
    if profile_is_client_only(profile_id) {
        "1"
    } else {
        "2"
    }
}
