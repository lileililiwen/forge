//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use super::contract::FEATURE_CATALOG_VERSION;
use super::model::{FeaturePlan, LifecycleOutcome};

/// Render a plan for human output.
pub fn render_plan_human(plan: &FeaturePlan) -> String {
    let mut lines = vec![format!(
        "plan for profile '{}' (catalog {})",
        plan.profile, FEATURE_CATALOG_VERSION
    )];
    if plan.steps.is_empty() {
        lines.push("steps: none".to_string());
    } else {
        for step in &plan.steps {
            lines.push(format!(
                "  {} {}@{}",
                step.action, step.feature, step.version
            ));
        }
    }
    if plan.validators.is_empty() {
        lines.push("validators: manifest re-parse and graph re-resolution".to_string());
    } else {
        lines.push(format!("validators: {}", plan.validators.join(", ")));
    }
    lines.join("\n")
}

/// Render a lifecycle outcome for human output.
pub fn render_outcome_human(outcome: &LifecycleOutcome) -> String {
    let mut lines = vec![
        format!(
            "{}: {} (profile {})",
            outcome.operation, outcome.note, outcome.profile
        ),
        format!("project: {}", outcome.project_id),
    ];
    if outcome.changed {
        lines.push(format!("files: {}", outcome.files_changed.join(", ")));
    } else {
        lines.push("files: unchanged".to_string());
    }
    if outcome.features.is_empty() {
        lines.push("features: none".to_string());
    } else {
        let feats: Vec<String> = outcome
            .features
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect();
        lines.push(format!("features: {}", feats.join(", ")));
    }
    lines.push(render_plan_human(&outcome.plan));
    lines.join("\n")
}
