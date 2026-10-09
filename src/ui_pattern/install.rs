//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use crate::core::ForgeError;
use std::fs;
use std::path::Path;

use super::constants::{UI_PATTERNS_DIR, UI_PATTERN_CATALOG_VERSION};
use super::model::{
    UiPatternInstallOutcome, UiPatternInstallRequest, UiPatternPlan, UiPatternResolveOutcome,
};
use super::resolve::{adapter_for_profile, inspect_ui_pattern, receipt_path};

fn atomic_write(path: &Path, body: &str) -> Result<(), ForgeError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| ForgeError::UiPatternOwnershipConflict {
            reason: format!(
                "cannot create artifact directory {}: {err}; prior state left unchanged",
                parent.display()
            ),
        })?;
    }
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, body).map_err(|err| ForgeError::UiPatternOwnershipConflict {
        reason: format!(
            "cannot stage artifact {}: {err}; prior state left unchanged",
            tmp.display()
        ),
    })?;
    fs::rename(&tmp, path).map_err(|err| ForgeError::UiPatternOwnershipConflict {
        reason: format!(
            "cannot publish artifact {}: {err}; prior state left unchanged",
            path.display()
        ),
    })?;
    Ok(())
}

/// Install one pattern for the named profile. The installer
/// never overwrites a customized file: a destination that
/// already exists (and is not byte-identical to the artifact
/// the catalog ships) is refused with a typed
/// `ui-pattern-ownership-conflict`. The receipt is written
/// under `.forge/ui-patterns/<id>/install.json` and the
/// source artifact is written at the adapter's
/// `artifact_path` so a `forge` removal after the install
/// leaves the project compiling through its native toolchain
/// (R2 boundary).
pub fn install_pattern(
    dir: &Path,
    request: &UiPatternInstallRequest,
) -> Result<UiPatternInstallOutcome, ForgeError> {
    let descriptor = inspect_ui_pattern(&request.pattern_id)?;
    let adapter = adapter_for_profile(&descriptor, &request.profile).ok_or_else(|| {
        ForgeError::UiPatternUnsupportedPlatform {
            reason: format!(
                "ui pattern '{}' has no adapter for profile '{}'; the planner refuses to \
                 substitute copied web markup for an unsupported platform",
                request.pattern_id, request.profile
            ),
        }
    })?;
    let artifact_abs = dir.join(&adapter.artifact_path);
    if artifact_abs.exists() {
        let existing = fs::read_to_string(&artifact_abs).map_err(|err| {
            ForgeError::UiPatternOwnershipConflict {
                reason: format!(
                    "cannot read existing artifact {}: {err}; install refused to avoid \
                     overwriting a customized file",
                    artifact_abs.display()
                ),
            }
        })?;
        if existing != adapter.artifact_source {
            return Err(ForgeError::UiPatternOwnershipConflict {
                reason: format!(
                    "ui pattern '{}' would overwrite a customized file at {}; remove or \
                     rename the file before installing",
                    request.pattern_id,
                    artifact_abs.display()
                ),
            });
        }
    }
    atomic_write(&artifact_abs, &adapter.artifact_source)?;
    let receipt = receipt_path(dir, &request.pattern_id);
    let payload = serde_json::json!({
        "contract": UI_PATTERN_CATALOG_VERSION,
        "pattern_id": request.pattern_id,
        "profile": request.profile,
        "version": descriptor.version,
        "intent": descriptor.intent,
        "quality": descriptor.quality.label(),
        "install_strategy": descriptor.install_strategy,
        "reason": request.reason,
        "artifact_path": adapter.artifact_path,
        "artifact_tests": adapter.tests,
        "states": descriptor.states.iter().map(|s| s.name.clone()).collect::<Vec<_>>(),
    });
    let body = serde_json::to_string_pretty(&payload).map_err(|err| {
        ForgeError::UiPatternOwnershipConflict {
            reason: format!("cannot serialize install receipt: {err}"),
        }
    })?;
    atomic_write(&receipt, &body)?;
    Ok(UiPatternInstallOutcome {
        pattern_id: request.pattern_id.clone(),
        profile: request.profile.clone(),
        installed: true,
        files_written: vec![
            adapter.artifact_path.clone(),
            format!("{UI_PATTERNS_DIR}/{}/install.json", request.pattern_id),
        ],
        note: format!(
            "installed ui pattern '{}' for profile '{}' at {} (reason: {})",
            request.pattern_id, request.profile, adapter.artifact_path, request.reason
        ),
    })
}

/// Render a plan for human output.
pub fn render_plan_human(plan: &UiPatternPlan) -> String {
    let mut lines = vec![format!(
        "ui-pattern plan for profile '{}' (catalog {})",
        plan.profile, UI_PATTERN_CATALOG_VERSION
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
            lines.push(format!(
                "  {} [{}] {}",
                rejection.id, rejection.code, rejection.reason
            ));
        }
    }
    lines.join("\n")
}

/// Render an outcome for human output.
pub fn render_outcome_human(outcome: &UiPatternResolveOutcome) -> String {
    let mut lines = vec![
        format!("ui-pattern resolve: {}", outcome.note),
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

/// Render an install outcome for human output.
pub fn render_install_human(outcome: &UiPatternInstallOutcome) -> String {
    let mut lines = vec![outcome.note.clone()];
    if !outcome.files_written.is_empty() {
        lines.push(format!("files: {}", outcome.files_written.join(", ")));
    } else {
        lines.push("files: (none)".to_string());
    }
    lines.push(format!(
        "installed: {} (pattern='{}' profile='{}')",
        outcome.installed, outcome.pattern_id, outcome.profile
    ));
    lines.join("\n")
}
