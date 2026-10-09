//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use crate::core::manifest::Manifest;
use crate::core::ForgeError;
use crate::feature::{
    add_feature, expected_receipt, inspect_feature, upgrade_feature, FeatureDescriptor,
    TESTED_VERSION,
};
use crate::registry::Registry;
use std::fs;
use std::path::{Path, PathBuf};

use super::contract::UPGRADE_CONTRACT_VERSION;
use super::model::{
    FleetEntry, FleetReport, SemanticConflict, UpgradeOutcome, UpgradePlan, UpgradeStep,
};

/// Features whose upgrade includes a schema migration that filesystem
/// rollback cannot reverse. Declared here as the migration strategy
/// source of truth for v0.2 (only `postgres` ships a schema step).
pub fn requires_data_migration(feature_id: &str) -> bool {
    feature_id == "postgres"
}

fn step_kinds_for(descriptor: &FeatureDescriptor) -> Vec<String> {
    let mut kinds = match descriptor.install_strategy.as_str() {
        "package-plus-generator" => vec!["package", "configuration", "codemod"],
        _ => vec!["configuration", "codemod"],
    }
    .into_iter()
    .map(String::from)
    .collect::<Vec<_>>();
    if requires_data_migration(&descriptor.id) {
        kinds.push("schema".to_string());
    }
    kinds
}

fn recovery_for(descriptor: &FeatureDescriptor) -> (String, bool, String) {
    if requires_data_migration(&descriptor.id) {
        (
            format!(
                "IRREVERSIBLE data migration for '{}': filesystem rollback (forge.yaml + receipts) \
                 does NOT reverse applied database schema migrations; declared migration strategy \
                 'manifest-repin+manual-schema-review' requires a database backup before applying \
                 and manual data recovery on failure",
                descriptor.id
            ),
            false,
            "manifest-repin+manual-schema-review".to_string(),
        )
    } else {
        (
            format!(
                "reversible file recovery for '{}': restore forge.yaml bytes and \
                 .forge/features/{}.receipt, then re-run `forge register`; no database \
                 migration is involved",
                descriptor.id, descriptor.id
            ),
            true,
            descriptor.upgrade_strategy.clone(),
        )
    }
}

/// Resolve a lifecycle target (registered id or filesystem path) to a
/// project directory without changing anything.
fn resolve_target_dir(registry: &Registry, target: &str) -> Result<PathBuf, ForgeError> {
    let candidate = Path::new(target);
    if candidate.is_dir() {
        return candidate
            .canonicalize()
            .map_err(|_| ForgeError::PathUnavailable {
                path: target.to_string(),
            });
    }
    let record = registry.inspect(target)?;
    let dir = PathBuf::from(&record.path);
    if !dir.is_dir() {
        return Err(ForgeError::PathUnavailable { path: record.path });
    }
    Ok(dir)
}

/// Read-only pinned plan for the project in `dir`. `requested` names one
/// feature to upgrade (installing it when missing, per requirement.md
/// §26 Project B); `None` upgrades every outdated installed feature.
/// Never touches the filesystem beyond reads.
pub fn plan_for_dir(dir: &Path, requested: Option<&str>) -> Result<UpgradePlan, ForgeError> {
    let (manifest, _) = Manifest::load_from_dir(dir, None)?;
    plan_for_manifest(&manifest, requested)
}

fn plan_for_manifest(
    manifest: &Manifest,
    requested: Option<&str>,
) -> Result<UpgradePlan, ForgeError> {
    let catalog = crate::feature::feature_catalog();
    let lookup = |id: &str| {
        catalog
            .iter()
            .find(|f| f.id == id)
            .cloned()
            .ok_or_else(|| ForgeError::UnknownFeature { id: id.to_string() })
    };

    // Target set: the requested feature, or every installed feature.
    let mut targets: Vec<String> = match requested {
        Some(id) => {
            lookup(id)?;
            vec![id.to_string()]
        }
        None => manifest.features.keys().cloned().collect(),
    };
    targets.sort();
    targets.dedup();

    // Compatibility check before any edit: the full closure (installed
    // plus requested) must still resolve for this profile.
    let mut closure: Vec<String> = manifest.features.keys().cloned().collect();
    for id in &targets {
        if !closure.contains(id) {
            closure.push(id.clone());
        }
    }
    closure.sort();
    closure.dedup();
    let ordered = if closure.is_empty() {
        Vec::new()
    } else {
        crate::feature::resolve_plan(&manifest.project.profile, &closure)?
            .steps
            .into_iter()
            .map(|s| s.feature)
            .collect::<Vec<_>>()
    };
    // Execution order follows dependency order; restrict to targets.
    let mut exec: Vec<String> = ordered
        .into_iter()
        .filter(|id| targets.contains(id))
        .collect();
    // A requested-but-uninstalled feature is always part of the plan even
    // when resolution lists only installed features (it cannot, since the
    // closure includes it, but keep the guard for clarity).
    for id in &targets {
        if !exec.contains(id) {
            exec.push(id.clone());
        }
    }

    let mut steps = Vec::new();
    let mut validators: Vec<String> = Vec::new();
    for id in exec {
        let descriptor = lookup(&id)?;
        let installed = manifest.features.get(&id).cloned();
        let action = match &installed {
            None => "install",
            Some(v) if v == TESTED_VERSION => "noop",
            Some(_) => "upgrade",
        };
        let (recovery, reversible, strategy) = recovery_for(&descriptor);
        let mut step_validators = descriptor.validation.clone();
        if step_validators.is_empty() {
            step_validators.push("manifest re-parse and graph re-resolution".to_string());
        }
        for v in &step_validators {
            if !validators.contains(v) {
                validators.push(v.clone());
            }
        }
        steps.push(UpgradeStep {
            feature: id.clone(),
            old_version: installed,
            new_version: TESTED_VERSION.to_string(),
            action: action.to_string(),
            kinds: if action == "noop" {
                Vec::new()
            } else {
                step_kinds_for(&descriptor)
            },
            assets: vec![
                "forge.yaml".to_string(),
                format!("{}/{id}.receipt", crate::feature::RECEIPT_DIR),
            ],
            validators: step_validators,
            migration_strategy: strategy,
            recovery,
            reversible,
        });
    }

    Ok(UpgradePlan {
        contract: UPGRADE_CONTRACT_VERSION.to_string(),
        project_id: manifest.project.id.clone(),
        profile: manifest.project.profile.clone(),
        requested: targets,
        steps,
        validators,
    })
}

/// Detect a semantic conflict for `feature`: a present receipt that no
/// longer matches the installed version means user-owned edits block the
/// upgrade. Returns the structured handoff, or `None` when clean.
fn detect_conflict(
    dir: &Path,
    manifest: &Manifest,
    descriptor: &FeatureDescriptor,
) -> Option<SemanticConflict> {
    let installed = manifest.features.get(&descriptor.id)?;
    let path = dir
        .join(crate::feature::RECEIPT_DIR)
        .join(format!("{}.receipt", descriptor.id));
    let actual = fs::read_to_string(&path).ok()?;
    if actual == expected_receipt(descriptor, installed) {
        return None;
    }
    let reason = format!(
        "custom edits invalidate migration preconditions for feature '{}': owned file '{}' \
         differs from the deterministic receipt for installed version '{installed}'",
        descriptor.id,
        path.display()
    );
    Some(SemanticConflict {
        project_id: manifest.project.id.clone(),
        feature: descriptor.id.clone(),
        owned_file: path.display().to_string(),
        suggested_spec: format!(
            "forge spec generate --project {}  # semantic migration for '{}': resolve owned-file edits, then re-run `forge upgrade {}`",
            manifest.project.id, descriptor.id, descriptor.id
        ),
        reason,
    })
}

fn conflict_error(conflict: &SemanticConflict) -> ForgeError {
    ForgeError::FeatureOwnershipConflict {
        reason: format!(
            "semantic-conflict handoff for project '{}' feature '{}': {}; owned file '{}'; \
             suggested follow-up `{}`; no files were changed",
            conflict.project_id,
            conflict.feature,
            conflict.reason,
            conflict.owned_file,
            conflict.suggested_spec
        ),
    }
}

/// Standard ownership-conflict refusal for a user-edited generated
/// workspace declaration: the edited file is preserved, nothing changes.
fn metadata_conflict_error(project_id: &str) -> ForgeError {
    ForgeError::FeatureOwnershipConflict {
        reason: format!(
            "ownership-conflict handoff for project '{project_id}' workspace metadata: owned \
             file '{}' differs from the content recorded at generation; manual edits block the \
             upgrade and are preserved; suggested follow-up `forge spec generate --project \
             {project_id}  # resolve .project.json edits, then re-run `forge upgrade {project_id}``; \
             no files were changed",
            crate::generate::workspace::METADATA_PATH
        ),
    }
}

/// Whether an upgrade error message reports partial application (some
/// features applied before the failure). Fleet maps partial errors to
/// `failure` and pre-mutation errors to `blocked`.
pub fn error_reports_partial(err: &ForgeError) -> bool {
    err.to_string().starts_with("partial upgrade:")
}

/// Apply a read-only `plan` to the project in `dir`, mutating via the
/// feature lifecycle operations in dependency order. No-op plans change
/// nothing and journal nothing. Precondition conflicts journal a
/// `blocked` entry and change no files. Mid-run failures leave earlier
/// applied features in place (partial state) and journal `failed`.
fn execute_plan(
    registry: &mut Registry,
    dir: &Path,
    plan: UpgradePlan,
) -> Result<UpgradeOutcome, ForgeError> {
    let (manifest, _) = Manifest::load_from_dir(dir, None)?;
    let catalog = crate::feature::feature_catalog();
    let mut mutating: Vec<&UpgradeStep> =
        plan.steps.iter().filter(|s| s.action != "noop").collect();

    if mutating.is_empty() {
        let features = manifest.features.clone();
        let validation = plan.validators.clone();
        let recovery = plan.steps.iter().map(|s| s.recovery.clone()).collect();
        return Ok(UpgradeOutcome {
            project_id: manifest.project.id.clone(),
            profile: manifest.project.profile.clone(),
            operation: "upgrade".to_string(),
            plan,
            changed: false,
            note: "already satisfied: every requested feature is at its pinned version; no files were rewritten".to_string(),
            files_changed: Vec::new(),
            features,
            validation,
            recovery,
        });
    }

    // Precondition sweep before any mutation: drifted receipts block with
    // a semantic-conflict handoff and all files preserved.
    for step in &mutating {
        let descriptor = catalog
            .iter()
            .find(|f| f.id == step.feature)
            .cloned()
            .ok_or_else(|| ForgeError::UnknownFeature {
                id: step.feature.clone(),
            })?;
        // Only installed features can have drifted receipts; missing
        // features install cleanly.
        if manifest.features.contains_key(&step.feature) {
            if let Some(conflict) = detect_conflict(dir, &manifest, &descriptor) {
                let err = conflict_error(&conflict);
                let _ = registry.record_operation(
                    "upgrade",
                    &manifest.project.id,
                    "blocked",
                    &err.to_string(),
                );
                return Err(err);
            }
        }
    }

    // Workspace metadata ownership: a user-edited generated declaration
    // blocks the upgrade exactly like a drifted feature receipt. The
    // edited file is preserved and nothing mutates.
    if let Ok(descriptor) = crate::profile::inspect_profile(&manifest.project.profile) {
        if crate::generate::workspace::plan_action(dir, &manifest.project.id, &descriptor)
            == crate::generate::workspace::MetadataAction::Conflict
        {
            let err = metadata_conflict_error(&manifest.project.id);
            let _ = registry.record_operation(
                "upgrade",
                &manifest.project.id,
                "blocked",
                &err.to_string(),
            );
            return Err(err);
        }
    }

    mutating.sort_by(|a, b| a.feature.cmp(&b.feature));
    // Re-order into plan (dependency) order for execution.
    let order: Vec<String> = plan.steps.iter().map(|s| s.feature.clone()).collect();
    mutating.sort_by_key(|s| order.iter().position(|id| id == &s.feature));

    let target = dir.display().to_string();
    let mut applied: Vec<String> = Vec::new();
    for step in &mutating {
        // Re-read the live manifest: an earlier step may have installed a
        // dependency of this step, turning an install into a no-op.
        let (live, _) = Manifest::load_from_dir(dir, None)?;
        let result = if !live.features.contains_key(&step.feature) {
            add_feature(registry, &target, &step.feature, None)
        } else if live
            .features
            .get(&step.feature)
            .is_some_and(|v| v == TESTED_VERSION)
        {
            applied.push(step.feature.clone());
            continue;
        } else {
            upgrade_feature(registry, &target, &step.feature, None)
        };
        match result {
            Ok(_) => applied.push(step.feature.clone()),
            Err(err) => {
                let partial = !applied.is_empty();
                let (live_features, live_note) = match Manifest::load_from_dir(dir, None) {
                    Ok((live, _)) => (
                        live.features.clone(),
                        format!(
                            "partial upgrade: {} applied before failure ({}); project '{}' is left \
                             in its partial state; recovery: re-plan with `forge upgrade --dry-run`, \
                             restore applied features via their receipts if the failure must be undone, \
                             then retry; underlying error: {err}",
                            applied.len(),
                            if applied.is_empty() {
                                "none".to_string()
                            } else {
                                applied.join(", ")
                            },
                            manifest.project.id
                        ),
                    ),
                    Err(_) => (
                        manifest.features.clone(),
                        format!("partial upgrade: manifest unreadable after failure: {err}"),
                    ),
                };
                let _ = registry.record_operation(
                    "upgrade",
                    &manifest.project.id,
                    "failed",
                    &live_note,
                );
                if partial {
                    return Err(ForgeError::IncompatibleFeature { reason: live_note });
                }
                // Preserve the original error kind (e.g. ownership
                // conflict on a later feature) when nothing applied yet.
                let _ = live_features;
                return Err(err);
            }
        }
    }

    let (live, _) = Manifest::load_from_dir(dir, None)?;
    let mut files = vec!["forge.yaml".to_string()];
    for id in &applied {
        files.push(format!("{}/{id}.receipt", crate::feature::RECEIPT_DIR));
    }
    // An unedited generated declaration whose manifest or descriptor
    // moved since generation is refreshed to the honest current content.
    // User-edited files already blocked the upgrade above; a declaration
    // without a Forge receipt stays foreign content and is never written.
    if let Ok(descriptor) = crate::profile::inspect_profile(&live.project.profile) {
        let action = crate::generate::workspace::plan_action(dir, &live.project.id, &descriptor);
        match crate::generate::workspace::apply_action(dir, &action) {
            Ok(changed) => {
                for rel in changed {
                    if !files.contains(&rel) {
                        files.push(rel);
                    }
                }
            }
            Err(err) => {
                let note = format!(
                    "partial upgrade: features applied ({}); workspace metadata refresh for \
                     project '{}' could not write: {err}",
                    applied.join(", "),
                    live.project.id
                );
                let _ = registry.record_operation("upgrade", &live.project.id, "failed", &note);
                return Err(ForgeError::IncompatibleFeature { reason: note });
            }
        }
    }
    files.sort();
    files.dedup();
    let validation = plan.validators.clone();
    let recovery = plan
        .steps
        .iter()
        .map(|s| s.recovery.clone())
        .collect::<Vec<_>>();
    let note = format!(
        "upgraded {} to pinned versions ({})",
        manifest.project.id,
        applied.join(", ")
    );
    let _ = registry.record_operation("upgrade", &manifest.project.id, "done", &note);
    Ok(UpgradeOutcome {
        project_id: live.project.id.clone(),
        profile: live.project.profile.clone(),
        operation: "upgrade".to_string(),
        plan,
        changed: true,
        note,
        files_changed: files,
        features: live.features.clone(),
        validation,
        recovery,
    })
}

/// Plan a single-project upgrade without changing anything.
pub fn plan_upgrade(
    registry: &Registry,
    target: &str,
    requested: Option<&str>,
) -> Result<UpgradePlan, ForgeError> {
    if let Some(id) = requested {
        inspect_feature(id)?;
    }
    let dir = resolve_target_dir(registry, target)?;
    plan_for_dir(&dir, requested)
}

/// Apply a single-project upgrade: plan, check preconditions, then mutate
/// in dependency order via the feature lifecycle.
pub fn apply_upgrade(
    registry: &mut Registry,
    target: &str,
    requested: Option<&str>,
) -> Result<UpgradeOutcome, ForgeError> {
    if let Some(id) = requested {
        inspect_feature(id)?;
    }
    let dir = resolve_target_dir(registry, target)?;
    let plan = plan_for_dir(&dir, requested)?;
    execute_plan(registry, &dir, plan)
}

/// Run a fleet upgrade over an explicit captured registry selection.
/// The selection is snapshotted (ordered by id) before any mutation; each
/// project is planned and applied independently with its own journal
/// entries. One project's failure never aborts the remaining projects.
/// `dry_run` plans every selected project without changing files,
/// registry rows or journals.
pub fn run_fleet(
    registry: &mut Registry,
    requested: Option<&str>,
    dry_run: bool,
) -> Result<FleetReport, ForgeError> {
    if let Some(id) = requested {
        inspect_feature(id)?;
    }
    let selection: Vec<String> = registry.list()?.into_iter().map(|p| p.id).collect();
    let mut entries: Vec<FleetEntry> = Vec::new();

    for project_id in &selection {
        let record = match registry.inspect(project_id) {
            Ok(record) => record,
            Err(_) => {
                entries.push(FleetEntry {
                    project_id: project_id.clone(),
                    status: "skipped".to_string(),
                    changed: false,
                    note: "skipped: project record unreadable at selection time".to_string(),
                    features: std::collections::BTreeMap::new(),
                    validation: Vec::new(),
                    recovery: Vec::new(),
                    conflict: None,
                });
                continue;
            }
        };
        let dir = PathBuf::from(&record.path);
        if !dir.is_dir() {
            let _ = registry.record_operation(
                "upgrade",
                project_id,
                "skipped",
                "skipped: project path unavailable; selection preserved and nothing was changed",
            );
            entries.push(FleetEntry {
                project_id: project_id.clone(),
                status: "skipped".to_string(),
                changed: false,
                note: "skipped: project path unavailable; nothing was changed".to_string(),
                features: record.features.clone(),
                validation: Vec::new(),
                recovery: Vec::new(),
                conflict: None,
            });
            continue;
        }
        let plan = match plan_for_dir(&dir, requested) {
            Ok(plan) => plan,
            Err(err) => {
                let _ =
                    registry.record_operation("upgrade", project_id, "blocked", &err.to_string());
                entries.push(FleetEntry {
                    project_id: project_id.clone(),
                    status: "blocked".to_string(),
                    changed: false,
                    note: format!("blocked before mutations: {err}"),
                    features: record.features.clone(),
                    validation: Vec::new(),
                    recovery: Vec::new(),
                    conflict: None,
                });
                continue;
            }
        };
        if dry_run {
            let noop = plan.steps.iter().all(|s| s.action == "noop");
            entries.push(FleetEntry {
                project_id: project_id.clone(),
                status: "skipped".to_string(),
                changed: false,
                note: if noop {
                    "dry-run: already satisfied; no changes would be made".to_string()
                } else {
                    format!(
                        "dry-run: plan only; {} step(s) would run; nothing was changed",
                        plan.steps.iter().filter(|s| s.action != "noop").count()
                    )
                },
                features: record.features.clone(),
                validation: plan.validators.clone(),
                recovery: plan.steps.iter().map(|s| s.recovery.clone()).collect(),
                conflict: None,
            });
            continue;
        }
        match execute_plan(registry, &dir, plan) {
            Ok(outcome) => {
                if outcome.changed {
                    entries.push(FleetEntry {
                        project_id: outcome.project_id.clone(),
                        status: "success".to_string(),
                        changed: true,
                        note: outcome.note.clone(),
                        features: outcome.features.clone(),
                        validation: outcome.validation.clone(),
                        recovery: outcome.recovery.clone(),
                        conflict: None,
                    });
                } else {
                    let _ = registry.record_operation(
                        "upgrade",
                        project_id,
                        "skipped",
                        "skipped: already satisfied; no files were rewritten",
                    );
                    entries.push(FleetEntry {
                        project_id: outcome.project_id.clone(),
                        status: "skipped".to_string(),
                        changed: false,
                        note: outcome.note.clone(),
                        features: outcome.features.clone(),
                        validation: outcome.validation.clone(),
                        recovery: outcome.recovery.clone(),
                        conflict: None,
                    });
                }
            }
            Err(err) => {
                // Re-read live state for partial-state reporting.
                let live_features = Manifest::load_from_dir(&dir, None)
                    .map(|(m, _)| m.features)
                    .unwrap_or_default();
                if error_reports_partial(&err) {
                    entries.push(FleetEntry {
                        project_id: project_id.clone(),
                        status: "failure".to_string(),
                        changed: true,
                        note: err.to_string(),
                        features: live_features,
                        validation: Vec::new(),
                        recovery: vec![data_recovery_note()],
                        conflict: None,
                    });
                } else {
                    let conflict = conflict_from_error(project_id, requested, &err);
                    entries.push(FleetEntry {
                        project_id: project_id.clone(),
                        status: "blocked".to_string(),
                        changed: false,
                        note: format!("blocked before further mutations: {err}"),
                        features: live_features,
                        validation: Vec::new(),
                        recovery: vec![file_recovery_note()],
                        conflict,
                    });
                }
            }
        }
    }

    let succeeded = entries.iter().filter(|e| e.status == "success").count();
    let failed = entries.iter().filter(|e| e.status == "failure").count();
    let blocked = entries.iter().filter(|e| e.status == "blocked").count();
    let skipped = entries.iter().filter(|e| e.status == "skipped").count();
    Ok(FleetReport {
        contract: UPGRADE_CONTRACT_VERSION.to_string(),
        selection: selection.clone(),
        requested: requested.map(|s| vec![s.to_string()]).unwrap_or_default(),
        dry_run,
        entries,
        succeeded,
        failed,
        blocked,
        skipped,
    })
}

fn conflict_from_error(
    project_id: &str,
    requested: Option<&str>,
    err: &ForgeError,
) -> Option<SemanticConflict> {
    match err {
        ForgeError::FeatureOwnershipConflict { .. } => Some(SemanticConflict {
            project_id: project_id.to_string(),
            feature: requested.unwrap_or("project").to_string(),
            owned_file: String::new(),
            reason: err.to_string(),
            suggested_spec: format!(
                "forge spec generate --project {project_id}  # resolve owned-file edits, then retry `forge upgrade`"
            ),
        }),
        _ => None,
    }
}

fn file_recovery_note() -> String {
    "reversible file recovery: restore forge.yaml bytes and affected receipts, then re-run `forge register`".to_string()
}

fn data_recovery_note() -> String {
    "partial execution: applied features remain; filesystem rollback does NOT reverse database schema migrations; back up databases before retrying and recover data manually where a schema step applied".to_string()
}

/// Render a plan for human output.
pub fn render_plan_human(plan: &UpgradePlan) -> String {
    let mut lines = vec![format!(
        "upgrade plan for '{}' (profile {}, contract {})",
        plan.project_id, plan.profile, plan.contract
    )];
    if plan.steps.is_empty() {
        lines.push("steps: none (no features installed)".to_string());
    } else {
        for step in &plan.steps {
            let old = step.old_version.as_deref().unwrap_or("missing");
            lines.push(format!(
                "  {} {}: {} -> {} [{}]",
                step.action,
                step.feature,
                old,
                step.new_version,
                if step.kinds.is_empty() {
                    "no-op".to_string()
                } else {
                    step.kinds.join("+")
                }
            ));
            lines.push(format!("    assets: {}", step.assets.join(", ")));
            lines.push(format!("    recovery: {}", step.recovery));
        }
    }
    if plan.validators.is_empty() {
        lines.push("validators: manifest re-parse and graph re-resolution".to_string());
    } else {
        lines.push(format!("validators: {}", plan.validators.join(", ")));
    }
    lines.join("\n")
}

/// Render an outcome for human output.
pub fn render_outcome_human(outcome: &UpgradeOutcome) -> String {
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

/// Render a fleet report for human output.
pub fn render_fleet_human(report: &FleetReport) -> String {
    let mut lines = vec![format!(
        "fleet upgrade{}: {} success, {} failure, {} blocked, {} skipped (selection: {})",
        if report.dry_run { " (dry-run)" } else { "" },
        report.succeeded,
        report.failed,
        report.blocked,
        report.skipped,
        if report.selection.is_empty() {
            "empty".to_string()
        } else {
            report.selection.join(", ")
        }
    )];
    for entry in &report.entries {
        lines.push(format!(
            "  [{}] {}: {}",
            entry.status, entry.project_id, entry.note
        ));
    }
    lines.push(format!(
        "verdict: {}",
        if report.healthy() {
            "healthy"
        } else {
            "not healthy"
        }
    ));
    lines.join("\n")
}
