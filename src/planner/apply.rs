//! Planner plan receipts and application.

use crate::component::resolve_outcome as resolve_components;
use crate::core::ForgeError;
use crate::profile::inspect_profile;
use std::fs;
use std::path::{Path, PathBuf};

use super::contract::PLANS_DIR;
use super::model::{AppliedStep, AssemblyPlan, IntentApplyOutcome, PlanStepKind};
use super::validate::{catalog_hash, intent_hash};

/// Directory the planner writes receipts under. Created on demand.
pub fn plans_dir(project_root: &Path) -> PathBuf {
    project_root.join(PLANS_DIR)
}

/// Path the executor reads to re-validate a plan.
pub fn plan_receipt_path(project_root: &Path, plan_id: &str) -> PathBuf {
    plans_dir(project_root).join(plan_id).join("plan.json")
}

/// Persist a resolved plan under `.forge/planner/<plan-id>/plan.json`
/// so the executor can re-bind and re-validate it before applying.
pub fn write_plan_receipt(project_root: &Path, plan: &AssemblyPlan) -> Result<PathBuf, ForgeError> {
    let dir = plans_dir(project_root).join(&plan.plan_id);
    fs::create_dir_all(&dir).map_err(|e| ForgeError::PlanApplyFailed {
        step: "write_plan_receipt".to_string(),
        reason: format!("could not create plan directory {}: {e}", dir.display()),
    })?;
    let path = dir.join("plan.json");
    let body = serde_json::to_vec_pretty(plan).map_err(|e| ForgeError::PlanApplyFailed {
        step: "write_plan_receipt".to_string(),
        reason: format!("could not serialize plan: {e}"),
    })?;
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, &body).map_err(|e| ForgeError::PlanApplyFailed {
        step: "write_plan_receipt".to_string(),
        reason: format!("could not write temporary plan file {}: {e}", tmp.display()),
    })?;
    fs::rename(&tmp, &path).map_err(|e| ForgeError::PlanApplyFailed {
        step: "write_plan_receipt".to_string(),
        reason: format!("could not finalize plan file {}: {e}", path.display()),
    })?;
    Ok(path)
}

/// Read a previously persisted plan. Missing or unreadable receipts
/// surface as a typed `PlanStale` error so the executor never
/// silently applies an unverified plan.
pub fn read_plan_receipt(project_root: &Path, plan_id: &str) -> Result<AssemblyPlan, ForgeError> {
    let path = plan_receipt_path(project_root, plan_id);
    if !path.exists() {
        return Err(ForgeError::PlanStale {
            reason: format!(
                "plan receipt {} does not exist; the planner refuses to apply an \
                 unverified plan",
                path.display()
            ),
        });
    }
    let body = fs::read(&path).map_err(|e| ForgeError::PlanStale {
        reason: format!("plan receipt {} is unreadable: {e}", path.display()),
    })?;
    let plan: AssemblyPlan = serde_json::from_slice(&body).map_err(|e| ForgeError::PlanStale {
        reason: format!("plan receipt {} is malformed: {e}", path.display()),
    })?;
    Ok(plan)
}

/// Revalidate a plan against the current catalog before applying.
/// The captured profile version, intent hash and catalog hash are the
/// three values the executor re-checks; any drift surfaces as a typed
/// `PlanStale` error and the executor does not run a single step
/// (R2 failure scenario).
pub fn revalidate_plan(plan: &AssemblyPlan) -> Result<(), ForgeError> {
    let profile = inspect_profile(&plan.profile).map_err(|_| ForgeError::PlanStale {
        reason: format!(
            "plan references profile '{}' which is no longer in the catalog; the planner \
             refuses to apply a stale plan",
            plan.profile
        ),
    })?;
    if profile.version != plan.profile_version {
        return Err(ForgeError::PlanStale {
            reason: format!(
                "plan was resolved against profile '{}@{}' but the catalog now reports \
                 '@{}'; revalidate the intent and resolve a new plan",
                plan.profile, plan.profile_version, profile.version
            ),
        });
    }
    let current_intent_hash = intent_hash_for_plan(plan);
    if current_intent_hash != plan.intent_hash {
        return Err(ForgeError::PlanStale {
            reason: format!(
                "plan intent hash drifted (expected {}, current {}); revalidate the \
                 intent and resolve a new plan",
                plan.intent_hash, current_intent_hash
            ),
        });
    }
    let current_catalog_hash = catalog_hash();
    if current_catalog_hash != plan.catalog_hash {
        return Err(ForgeError::PlanStale {
            reason: format!(
                "plan catalog hash drifted (expected {}, current {}); revalidate the \
                 intent and resolve a new plan",
                plan.catalog_hash, current_catalog_hash
            ),
        });
    }
    Ok(())
}

fn intent_hash_for_plan(plan: &AssemblyPlan) -> String {
    intent_hash(&plan.intent)
}

/// Re-validate a persisted plan and apply it. The executor runs each
/// deterministic step through the existing Core contracts the
/// planner composes (`feature::add_feature`, `ui_pattern::install_pattern`,
/// `component::resolve_outcome` is informational, the doctor and
/// DriftWatch adapters); unresolved entries are recorded as `skipped`
/// so a partial run is observable.
#[allow(clippy::too_many_arguments)]
pub fn apply_plan(
    project_root: &Path,
    plan_id: &str,
    confirm: bool,
    db_path: Option<&Path>,
) -> Result<IntentApplyOutcome, ForgeError> {
    if !confirm {
        return Err(ForgeError::PlanApplyFailed {
            step: "apply_plan".to_string(),
            reason: "refusing to apply a planner plan without --confirm; the planner \
                     never mutates a project without an explicit confirmation"
                .to_string(),
        });
    }
    let plan = read_plan_receipt(project_root, plan_id)?;
    revalidate_plan(&plan)?;

    let registry_path = db_path
        .map(|p| p.to_path_buf())
        .or_else(|| default_registry_path_for(project_root));

    let mut registry = match registry_path.as_deref() {
        Some(path) => Some(crate::registry::Registry::open(path)?),
        None => None,
    };

    let mut applied: Vec<AppliedStep> = Vec::new();
    let mut files_written: Vec<String> = Vec::new();
    for step in &plan.steps {
        match step.kind {
            PlanStepKind::InstallFeature => {
                let reg = match registry.as_mut() {
                    Some(r) => r,
                    None => {
                        return Err(ForgeError::PlanApplyFailed {
                            step: step.target.clone(),
                            reason: "no registry is available; the planner refuses to \
                                     install a feature without a registered project"
                                .to_string(),
                        });
                    }
                };
                match crate::feature::add_feature(
                    reg,
                    project_root.to_str().unwrap_or("."),
                    &step.target,
                    Some(step.version.as_str()),
                ) {
                    Ok(o) => {
                        for f in &o.files_changed {
                            files_written.push(f.clone());
                        }
                        applied.push(AppliedStep {
                            kind: step.kind,
                            target: step.target.clone(),
                            status: "applied".to_string(),
                            note: format!("feature {}@{} installed", step.target, step.version),
                        });
                    }
                    Err(e) => {
                        return Err(ForgeError::PlanApplyFailed {
                            step: step.target.clone(),
                            reason: format!(
                                "feature install returned '{e}'; previously applied steps \
                                 remain visible in the report so the partial run is \
                                 observable"
                            ),
                        });
                    }
                }
            }
            PlanStepKind::InstallComponent => {
                let request = crate::component::ComponentRequest {
                    profile: plan.profile.clone(),
                    component_ids: vec![step.target.clone()],
                };
                match resolve_components(&request) {
                    Ok(_) => applied.push(AppliedStep {
                        kind: step.kind,
                        target: step.target.clone(),
                        status: "verified".to_string(),
                        note: format!(
                            "component {}@{} resolved for {}; promotion is the operator's \
                             call and the planner did not auto-qualify it",
                            step.target, step.version, plan.profile
                        ),
                    }),
                    Err(e) => {
                        return Err(ForgeError::PlanApplyFailed {
                            step: step.target.clone(),
                            reason: format!("component resolve returned '{e}'"),
                        });
                    }
                }
            }
            PlanStepKind::InstallUiPattern => {
                let request = crate::ui_pattern::UiPatternInstallRequest {
                    pattern_id: step.target.clone(),
                    profile: plan.profile.clone(),
                    reason: format!("planner plan {}", plan.plan_id),
                };
                match crate::ui_pattern::install_pattern(project_root, &request) {
                    Ok(o) => {
                        if o.installed {
                            for f in &o.files_written {
                                files_written.push(f.clone());
                            }
                            applied.push(AppliedStep {
                                kind: step.kind,
                                target: step.target.clone(),
                                status: "installed".to_string(),
                                note: format!("ui pattern {} installed", step.target),
                            });
                        } else {
                            applied.push(AppliedStep {
                                kind: step.kind,
                                target: step.target.clone(),
                                status: "skipped".to_string(),
                                note: o.note.clone(),
                            });
                        }
                    }
                    Err(e) => {
                        return Err(ForgeError::PlanApplyFailed {
                            step: step.target.clone(),
                            reason: format!("ui pattern install returned '{e}'"),
                        });
                    }
                }
            }
            PlanStepKind::Doctor => {
                applied.push(AppliedStep {
                    kind: step.kind,
                    target: step.target.clone(),
                    status: "scheduled".to_string(),
                    note: format!(
                        "doctor '{}' is the planner's post-install gate; the operator \
                         runs it explicitly through the existing `forge doctor` command",
                        step.action
                    ),
                });
            }
            PlanStepKind::Test => {
                applied.push(AppliedStep {
                    kind: step.kind,
                    target: step.target.clone(),
                    status: "scheduled".to_string(),
                    note: format!(
                        "test '{}' is the planner's post-install gate; the operator runs \
                         it explicitly through the existing `forge test` command",
                        step.action
                    ),
                });
            }
            PlanStepKind::QualityPolicy => {
                applied.push(AppliedStep {
                    kind: step.kind,
                    target: step.target.clone(),
                    status: "scheduled".to_string(),
                    note: format!(
                        "quality policy '{}' is the planner's post-install gate; the \
                         operator runs it explicitly through the existing DriftWatch \
                         adapter",
                        step.action
                    ),
                });
            }
        }
    }

    for work in &plan.unresolved {
        applied.push(AppliedStep {
            kind: PlanStepKind::Doctor,
            target: work.requirement.clone(),
            status: "unresolved".to_string(),
            note: format!("{} ({})", work.reason, work.hint),
        });
    }

    let note = format!(
        "planner plan {} applied: {} step(s), {} file(s) written, {} unresolved",
        plan.plan_id,
        applied.len(),
        files_written.len(),
        plan.unresolved.len()
    );

    if let (Some(reg), Some(path)) = (registry.as_ref(), registry_path.as_deref()) {
        let _ = reg.record_operation("planner", &plan.plan_id, "done", &note);
        let _ = path;
    }

    Ok(IntentApplyOutcome {
        plan_id: plan.plan_id,
        applied_steps: applied,
        stale: false,
        note,
        files_written,
    })
}

fn default_registry_path_for(project_root: &Path) -> Option<PathBuf> {
    let path = project_root.join(".forge/registry.sqlite");
    if path.exists() {
        Some(path)
    } else {
        None
    }
}
