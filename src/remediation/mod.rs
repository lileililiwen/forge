//! Local, deterministic remediation plans.
//!
//! Remediation is intentionally narrower than the doctor surface: it only
//! materializes Forge-owned standard assets. Planning is pure; applying a
//! plan requires explicit confirmation and rechecks every ownership
//! precondition before writing.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};

use crate::core::{manifest::Manifest, ForgeError};
use crate::registry::Registry;
use crate::standard;

pub const REMEDIATION_CONTRACT_VERSION: &str = "forge-remediation-plan/0.1.0";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActionKind {
    WriteOwnedFile,
    RefreshManifestField,
    InstallStandardAsset,
    AddDocLink,
    RefreshOwnershipReceipt,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PlanState {
    Proposed,
    Confirmed,
    Applied,
    Failed,
    RolledBack,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FindingClass {
    Automatic,
    Semantic,
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlanTarget {
    pub path: String,
    pub project_id: String,
    pub profile: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PackBinding {
    pub id: String,
    pub version: String,
    pub asset_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PathPrecondition {
    pub path: String,
    pub expected_digest: Option<String>,
    pub owned: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlanPreconditions {
    pub revision: Option<String>,
    pub files: Vec<PathPrecondition>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RemediationAction {
    pub kind: ActionKind,
    pub path: String,
    pub owner: String,
    pub expected_digest: Option<String>,
    pub content: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RemediationPlan {
    pub contract: String,
    pub plan_id: String,
    pub state: PlanState,
    pub target: PlanTarget,
    pub finding_id: String,
    pub finding_class: FindingClass,
    pub pack: Option<PackBinding>,
    pub actions: Vec<RemediationAction>,
    pub preconditions: PlanPreconditions,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlanOutcome {
    pub status: String,
    pub state: PlanState,
    pub written: Vec<String>,
    pub already_applied: bool,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiffEntry {
    pub path: String,
    pub state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScanFinding {
    pub finding_id: String,
    pub class: FindingClass,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScanReport {
    pub contract: String,
    pub target: PlanTarget,
    pub findings: Vec<ScanFinding>,
}

#[derive(Debug, Clone)]
struct TargetContext {
    dir: PathBuf,
    manifest: Manifest,
    receipt: Option<standard::Receipt>,
}

pub fn build_plan(
    target: &Path,
    finding: &str,
    pack_selector: Option<&str>,
) -> Result<RemediationPlan, ForgeError> {
    let context = load_target(target)?;
    let (class, pack_selector) =
        classify_finding(finding, &context.manifest.project.id, pack_selector)?;
    if class != FindingClass::Automatic {
        return Err(ForgeError::RemediationInvalid {
            reason: format!(
                "finding '{finding}' is {:?}; only automatic local standard repairs are supported",
                class
            ),
        });
    }
    let selector = pack_selector.ok_or_else(|| ForgeError::RemediationInvalid {
        reason: "automatic CI remediation requires --pack <pack>@<version>".to_string(),
    })?;
    let descriptor =
        standard::inspect_pack(selector).map_err(|err| ForgeError::RemediationInvalid {
            reason: err.to_string(),
        })?;
    let snapshot = standard::render_snapshot(
        &context.manifest.project.id,
        &context.manifest.project.profile,
        &descriptor,
        standard::DETERMINISTIC_TIMESTAMP,
        true,
    )
    .map_err(|err| ForgeError::RemediationInvalid {
        reason: err.to_string(),
    })?;

    let mut rendered = BTreeMap::new();
    for (path, content) in snapshot.files {
        rendered.insert(path, content);
    }
    let receipt = standard::receipt_text(&snapshot.receipt).map_err(|err| {
        ForgeError::RemediationInvalid {
            reason: err.to_string(),
        }
    })?;
    rendered.insert(standard::RECEIPT_PATH.to_string(), receipt);

    // Materialize the complete snapshot so the updated receipt cannot claim
    // ownership of files that the action set did not write.
    let paths: Vec<String> = rendered.keys().cloned().collect();
    let mut actions = Vec::new();
    let mut preconditions = Vec::new();
    for path in paths {
        let absolute = context.dir.join(&path);
        ensure_target_confined(&context.dir, &path)?;
        let current = read_optional_file(&absolute)?;
        let current_digest = current.as_deref().map(standard::sha256_hex);
        let owned_digest = context.receipt.as_ref().and_then(|receipt| {
            receipt
                .files
                .iter()
                .find(|file| file.path == path)
                .map(|file| file.digest.clone())
        });
        let owned = if path == standard::RECEIPT_PATH {
            context.receipt.is_some()
        } else {
            owned_digest.is_some() && owned_digest == current_digest
        };
        preconditions.push(PathPrecondition {
            path: path.clone(),
            expected_digest: current_digest.clone(),
            owned,
        });
        let content = rendered
            .get(&path)
            .ok_or_else(|| ForgeError::RemediationInvalid {
                reason: format!("standard renderer did not produce required asset '{path}'"),
            })?;
        actions.push(RemediationAction {
            kind: if path == standard::RECEIPT_PATH {
                ActionKind::RefreshOwnershipReceipt
            } else {
                ActionKind::InstallStandardAsset
            },
            path,
            owner: "forge.standard".to_string(),
            expected_digest: current_digest,
            content: content.clone(),
            source: format!("{}@{}", snapshot.pack, snapshot.version),
        });
    }
    let revision = git_revision(&context.dir);
    let mut plan = RemediationPlan {
        contract: REMEDIATION_CONTRACT_VERSION.to_string(),
        plan_id: String::new(),
        state: PlanState::Proposed,
        target: PlanTarget {
            path: context.dir.display().to_string(),
            project_id: context.manifest.project.id.clone(),
            profile: context.manifest.project.profile.clone(),
        },
        finding_id: finding.to_string(),
        finding_class: class,
        pack: Some(PackBinding {
            id: snapshot.pack,
            version: snapshot.version,
            asset_digest: snapshot.asset_digest,
        }),
        actions,
        preconditions: PlanPreconditions {
            revision,
            files: preconditions,
        },
    };
    plan.plan_id = plan_digest(&plan)?;
    Ok(plan)
}

pub fn scan(target: &Path) -> Result<ScanReport, ForgeError> {
    let context = load_target(target)?;
    let findings = if context.dir.join(standard::CI_PATH).is_file() {
        Vec::new()
    } else {
        vec![ScanFinding {
            finding_id: format!("gaps.ci.{}.ci", context.manifest.project.id),
            class: FindingClass::Automatic,
            detail: "standard CI verification asset is missing".to_string(),
        }]
    };
    Ok(ScanReport {
        contract: REMEDIATION_CONTRACT_VERSION.to_string(),
        target: PlanTarget {
            path: context.dir.display().to_string(),
            project_id: context.manifest.project.id,
            profile: context.manifest.project.profile,
        },
        findings,
    })
}

pub fn diff(plan: &RemediationPlan) -> Vec<DiffEntry> {
    plan.actions
        .iter()
        .map(|action| DiffEntry {
            path: action.path.clone(),
            state: if action.expected_digest.is_some() {
                "update".to_string()
            } else {
                "add".to_string()
            },
        })
        .collect()
}

pub fn apply(
    plan: &RemediationPlan,
    confirm: bool,
    registry_path: &Path,
) -> Result<PlanOutcome, ForgeError> {
    validate_plan(plan)?;
    if !confirm {
        return Err(ForgeError::RemediationInvalid {
            reason: "apply requires explicit --confirm".to_string(),
        });
    }
    let dir = PathBuf::from(&plan.target.path);
    let pack = plan
        .pack
        .as_ref()
        .ok_or_else(|| ForgeError::RemediationInvalid {
            reason: "an applicable plan must name a standard pack".to_string(),
        })?;
    let selector = format!("{}@{}", pack.id, pack.version);
    let rebuilt = build_plan(&dir, &plan.finding_id, Some(&selector))?;
    if rebuilt.actions != plan.actions || rebuilt.pack != plan.pack || rebuilt.target != plan.target
    {
        return Err(ForgeError::RemediationConflict {
            reason: "plan content or preconditions differ from the selected local standard pack"
                .to_string(),
        });
    }
    let context = load_target(&dir)?;
    check_target_identity_and_revision(plan, &context)?;
    let mut all_already = true;
    for action in &plan.actions {
        ensure_target_confined(&dir, &action.path)?;
        let current = read_optional_file(&dir.join(&action.path))?;
        all_already &= current
            .as_deref()
            .is_some_and(|bytes| bytes == action.content.as_bytes());
    }
    let registry = Registry::open(registry_path)?;
    if all_already {
        registry.record_operation(
            "remediation",
            &plan.target.project_id,
            "already_applied",
            &plan.plan_id,
        )?;
        return Ok(PlanOutcome {
            status: "already_applied".to_string(),
            state: PlanState::Applied,
            written: Vec::new(),
            already_applied: true,
            detail: "all planned bytes already match".to_string(),
        });
    }
    if rebuilt.preconditions != plan.preconditions {
        return Err(ForgeError::RemediationConflict {
            reason: "target file preconditions differ from the reviewed plan".to_string(),
        });
    }
    check_preconditions(plan, &context)?;

    let stage = tempfile::tempdir_in(&dir).map_err(|err| ForgeError::RemediationApplyFailed {
        reason: format!("cannot create target-local staging directory: {err}"),
    })?;
    for action in &plan.actions {
        let staged = stage.path().join(&action.path);
        if let Some(parent) = staged.parent() {
            fs::create_dir_all(parent).map_err(|err| ForgeError::RemediationApplyFailed {
                reason: format!("cannot stage '{}': {err}", action.path),
            })?;
        }
        fs::write(&staged, action.content.as_bytes()).map_err(|err| {
            ForgeError::RemediationApplyFailed {
                reason: format!("cannot stage '{}': {err}", action.path),
            }
        })?;
        let bytes = fs::read(&staged).map_err(|err| ForgeError::RemediationApplyFailed {
            reason: format!("cannot verify staged '{}': {err}", action.path),
        })?;
        if standard::sha256_hex(&bytes) != standard::sha256_hex(action.content.as_bytes()) {
            return Err(ForgeError::RemediationApplyFailed {
                reason: format!("staged digest mismatch for '{}'", action.path),
            });
        }
    }
    let (written, previous) = promote_staged(
        &plan.actions,
        stage.path(),
        &dir,
        &registry,
        &plan.target.project_id,
        &plan.plan_id,
        None,
    )?;
    if let Err(err) = registry.record_operation(
        "remediation",
        &plan.target.project_id,
        "applied",
        &plan.plan_id,
    ) {
        return Err(rollback_error(
            &previous,
            &format!("record remediation outcome: {err}"),
            &registry,
            &plan.target.project_id,
            &plan.plan_id,
        ));
    }
    Ok(PlanOutcome {
        status: "applied".to_string(),
        state: PlanState::Applied,
        written,
        already_applied: false,
        detail: "target-local standard assets promoted after precondition checks".to_string(),
    })
}

type PreviousFiles = Vec<(PathBuf, Option<Vec<u8>>)>;

fn promote_staged(
    actions: &[RemediationAction],
    stage: &Path,
    dir: &Path,
    registry: &Registry,
    project_id: &str,
    plan_id: &str,
    fail_after: Option<usize>,
) -> Result<(Vec<String>, PreviousFiles), ForgeError> {
    let mut previous = Vec::new();
    let mut written = Vec::new();
    for (index, action) in actions.iter().enumerate() {
        ensure_target_confined(dir, &action.path)?;
        let destination = dir.join(&action.path);
        let prior_bytes = match read_optional_file(&destination) {
            Ok(bytes) => bytes,
            Err(err) => {
                return Err(rollback_error(
                    &previous,
                    &err.to_string(),
                    registry,
                    project_id,
                    plan_id,
                ));
            }
        };
        let had_prior_file = prior_bytes.is_some();
        previous.push((destination.clone(), prior_bytes));
        if let Some(parent) = destination.parent() {
            if let Err(err) = fs::create_dir_all(parent) {
                return Err(rollback_error(
                    &previous,
                    &format!("create parent: {err}"),
                    registry,
                    project_id,
                    plan_id,
                ));
            }
        }
        let staged = stage.join(&action.path);
        if had_prior_file {
            let backup = stage.join(".backups").join(&action.path);
            if let Some(parent) = backup.parent() {
                if let Err(err) = fs::create_dir_all(parent) {
                    return Err(rollback_error(
                        &previous,
                        &format!("stage prior bytes for '{}': {err}", action.path),
                        registry,
                        project_id,
                        plan_id,
                    ));
                }
            }
            if let Err(err) = fs::rename(&destination, &backup) {
                return Err(rollback_error(
                    &previous,
                    &format!("backup prior bytes for '{}': {err}", action.path),
                    registry,
                    project_id,
                    plan_id,
                ));
            }
        }
        if fail_after == Some(index) {
            return Err(rollback_error(
                &previous,
                "injected promotion failure",
                registry,
                project_id,
                plan_id,
            ));
        }
        if let Err(err) = fs::rename(&staged, &destination) {
            return Err(rollback_error(
                &previous,
                &format!("promote '{}': {err}", action.path),
                registry,
                project_id,
                plan_id,
            ));
        }
        written.push(action.path.clone());
    }
    Ok((written, previous))
}

pub fn plan_digest(plan: &RemediationPlan) -> Result<String, ForgeError> {
    let mut copy = plan.clone();
    copy.plan_id.clear();
    let bytes = serde_json::to_vec(&copy).map_err(|err| ForgeError::RemediationInvalid {
        reason: format!("plan is not serializable: {err}"),
    })?;
    Ok(standard::sha256_hex(&bytes))
}

pub fn validate_plan(plan: &RemediationPlan) -> Result<(), ForgeError> {
    if plan.contract != REMEDIATION_CONTRACT_VERSION {
        return Err(ForgeError::RemediationInvalid {
            reason: format!("unsupported plan contract '{}'", plan.contract),
        });
    }
    if plan.actions.is_empty() || !Path::new(&plan.target.path).is_absolute() {
        return Err(ForgeError::RemediationInvalid {
            reason: "plan must contain a target and at least one action".to_string(),
        });
    }
    if plan_digest(plan)? != plan.plan_id {
        return Err(ForgeError::RemediationInvalid {
            reason: "plan_id does not match the plan contents".to_string(),
        });
    }
    if plan.state != PlanState::Proposed {
        return Err(ForgeError::RemediationInvalid {
            reason: "only proposed plans can be applied".to_string(),
        });
    }
    if plan.preconditions.files.len() != plan.actions.len() {
        return Err(ForgeError::RemediationInvalid {
            reason: "every action must have exactly one path precondition".to_string(),
        });
    }
    for action in &plan.actions {
        validate_relative_path(&action.path)?;
        if !matches!(
            action.kind,
            ActionKind::InstallStandardAsset | ActionKind::RefreshOwnershipReceipt
        ) {
            return Err(ForgeError::RemediationInvalid {
                reason: format!("unsupported remediation action kind for '{}'", action.path),
            });
        }
        let precondition = plan
            .preconditions
            .files
            .iter()
            .find(|precondition| precondition.path == action.path)
            .ok_or_else(|| ForgeError::RemediationInvalid {
                reason: format!("action '{}' has no matching path precondition", action.path),
            })?;
        if precondition.expected_digest != action.expected_digest {
            return Err(ForgeError::RemediationInvalid {
                reason: format!(
                    "action '{}' digest differs from its precondition",
                    action.path
                ),
            });
        }
    }
    Ok(())
}

pub fn load_plan(path: &Path) -> Result<RemediationPlan, ForgeError> {
    let bytes = fs::read(path).map_err(|err| ForgeError::RemediationInvalid {
        reason: format!("cannot read plan '{}': {err}", path.display()),
    })?;
    let plan: RemediationPlan =
        serde_json::from_slice(&bytes).map_err(|err| ForgeError::RemediationInvalid {
            reason: format!("invalid plan JSON: {err}"),
        })?;
    validate_plan(&plan)?;
    Ok(plan)
}

fn load_target(target: &Path) -> Result<TargetContext, ForgeError> {
    let dir = target
        .canonicalize()
        .map_err(|_| ForgeError::RemediationInvalid {
            reason: format!("target '{}' is not an existing directory", target.display()),
        })?;
    if !dir.is_dir() {
        return Err(ForgeError::RemediationInvalid {
            reason: "target is not a directory".to_string(),
        });
    }
    let (manifest, _) =
        Manifest::load_from_dir(&dir, None).map_err(|err| ForgeError::RemediationInvalid {
            reason: err.to_string(),
        })?;
    ensure_target_confined(&dir, standard::RECEIPT_PATH)?;
    let receipt =
        match fs::read(dir.join(standard::RECEIPT_PATH)) {
            Ok(bytes) => Some(serde_json::from_slice::<standard::Receipt>(&bytes).map_err(
                |err| ForgeError::RemediationInvalid {
                    reason: format!("invalid standard ownership receipt: {err}"),
                },
            )?),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => None,
            Err(err) => {
                return Err(ForgeError::RemediationInvalid {
                    reason: format!("cannot read standard receipt: {err}"),
                })
            }
        };
    if let Some(receipt) = &receipt {
        if receipt.project != manifest.project.id || receipt.profile != manifest.project.profile {
            return Err(ForgeError::RemediationConflict {
                reason: "standard ownership receipt belongs to another project or profile"
                    .to_string(),
            });
        }
    }
    Ok(TargetContext {
        dir,
        manifest,
        receipt,
    })
}

fn classify_finding<'a>(
    finding: &str,
    project_id: &str,
    pack: Option<&'a str>,
) -> Result<(FindingClass, Option<&'a str>), ForgeError> {
    let parts: Vec<&str> = finding.split('.').collect();
    if parts.len() == 4 && parts[0] == "gaps" && parts[1] == "ci" {
        if parts[2] != project_id || parts[3] != "ci" {
            return Err(ForgeError::RemediationInvalid {
                reason: format!("finding '{finding}' does not identify this target's CI gap"),
            });
        }
        return Ok((FindingClass::Automatic, pack));
    }
    let class = if finding.contains("description") || finding.contains("tags") {
        FindingClass::Semantic
    } else {
        FindingClass::Manual
    };
    Ok((class, pack))
}

fn check_preconditions(plan: &RemediationPlan, context: &TargetContext) -> Result<(), ForgeError> {
    check_target_identity_and_revision(plan, context)?;
    for precondition in &plan.preconditions.files {
        ensure_target_confined(&context.dir, &precondition.path)?;
        let current = read_optional_file(&context.dir.join(&precondition.path))?;
        let digest = current.as_deref().map(standard::sha256_hex);
        if digest != precondition.expected_digest {
            return Err(ForgeError::RemediationConflict {
                reason: format!("'{}' changed since planning", precondition.path),
            });
        }
        if current.is_some() && !precondition.owned && precondition.expected_digest.is_some() {
            return Err(ForgeError::RemediationConflict {
                reason: format!("'{}' exists but is not Forge-owned", precondition.path),
            });
        }
    }
    Ok(())
}

fn check_target_identity_and_revision(
    plan: &RemediationPlan,
    context: &TargetContext,
) -> Result<(), ForgeError> {
    if context.manifest.project.id != plan.target.project_id
        || context.manifest.project.profile != plan.target.profile
    {
        return Err(ForgeError::RemediationConflict {
            reason: "target manifest no longer matches the plan".to_string(),
        });
    }
    if let Some(expected) = plan.preconditions.revision.as_deref() {
        if git_revision(&context.dir).as_deref() != Some(expected) {
            return Err(ForgeError::RemediationConflict {
                reason: "target revision changed since planning".to_string(),
            });
        }
    }
    Ok(())
}

fn validate_relative_path(path: &str) -> Result<(), ForgeError> {
    let candidate = Path::new(path);
    if candidate.is_absolute()
        || candidate
            .components()
            .any(|component| matches!(component, Component::ParentDir))
        || ![
            standard::CI_PATH,
            standard::COMPOSE_PATH,
            standard::PROFILE_PATH,
            standard::QUALITY_PATH,
            standard::VERIFY_PATH,
            standard::RECEIPT_PATH,
        ]
        .contains(&path)
    {
        return Err(ForgeError::RemediationInvalid {
            reason: format!("action path '{path}' is outside .standard/"),
        });
    }
    Ok(())
}

fn ensure_target_confined(target: &Path, relative: &str) -> Result<(), ForgeError> {
    validate_relative_path(relative)?;
    let destination = target.join(relative);
    let mut ancestor = destination.as_path();
    while !ancestor.exists() {
        ancestor = ancestor
            .parent()
            .ok_or_else(|| ForgeError::RemediationInvalid {
                reason: format!("cannot resolve destination '{relative}'"),
            })?;
    }
    let canonical = ancestor
        .canonicalize()
        .map_err(|err| ForgeError::RemediationInvalid {
            reason: format!("cannot resolve destination '{relative}': {err}"),
        })?;
    if !canonical.starts_with(target) {
        return Err(ForgeError::RemediationConflict {
            reason: format!("destination '{relative}' escapes target through a symlink"),
        });
    }
    Ok(())
}

fn read_optional_file(path: &Path) -> Result<Option<Vec<u8>>, ForgeError> {
    match fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(ForgeError::RemediationConflict {
            reason: format!("cannot safely inspect '{}': {err}", path.display()),
        }),
    }
}

fn git_revision(dir: &Path) -> Option<String> {
    Command::new("git")
        .args(["-C", dir.to_str()?, "rev-parse", "HEAD"])
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
            } else {
                None
            }
        })
}

fn rollback_error(
    previous: &[(PathBuf, Option<Vec<u8>>)],
    reason: &str,
    registry: &Registry,
    project_id: &str,
    plan_id: &str,
) -> ForgeError {
    let mut restored = Vec::new();
    let mut failed = Vec::new();
    for (path, bytes) in previous.iter().rev() {
        let result = match bytes {
            Some(bytes) => fs::write(path, bytes),
            None => fs::remove_file(path).or_else(|err| {
                if err.kind() == std::io::ErrorKind::NotFound {
                    Ok(())
                } else {
                    Err(err)
                }
            }),
        };
        match result {
            Ok(()) => restored.push(path.display().to_string()),
            Err(err) => failed.push(format!("{}: {err}", path.display())),
        }
    }
    let journal_detail = format!("plan={plan_id}; restored={restored:?}; failed={failed:?}");
    let journal_state = if failed.is_empty() {
        "rolled_back"
    } else {
        "rollback_failed"
    };
    let journal = registry
        .record_operation("remediation", project_id, journal_state, &journal_detail)
        .err()
        .map(|err| format!("; rollback journal failed: {err}"))
        .unwrap_or_default();
    ForgeError::RemediationApplyFailed {
        reason: format!(
            "{reason}; rollback restored {} path(s), failed {} path(s){journal}",
            restored.len(),
            failed.len()
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rollback_restores_bytes_and_journals_the_recovery() {
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("owned.txt");
        fs::write(&file, b"partially promoted").unwrap();
        let registry_path = temp.path().join("registry.db");
        let registry = Registry::open(&registry_path).unwrap();
        let error = rollback_error(
            &[(file.clone(), Some(b"prior bytes".to_vec()))],
            "injected promotion failure",
            &registry,
            "demo",
            "plan-test",
        );
        assert_eq!(error.code(), "remediation-apply-failed");
        assert_eq!(fs::read(&file).unwrap(), b"prior bytes");
        let operation = registry.recent_operations(1).unwrap().remove(0);
        assert_eq!(operation.state, "rolled_back");
        assert!(operation.detail.unwrap().contains("plan=plan-test"));
    }

    #[test]
    fn rollback_reports_and_journals_paths_it_cannot_restore() {
        let temp = tempfile::tempdir().unwrap();
        let directory = temp.path().join("unexpected-directory");
        fs::create_dir(&directory).unwrap();
        let registry_path = temp.path().join("registry.db");
        let registry = Registry::open(&registry_path).unwrap();
        let error = rollback_error(
            &[(directory, None)],
            "injected promotion failure",
            &registry,
            "demo",
            "plan-rollback-failed",
        );
        assert!(error.to_string().contains("failed 1 path(s)"));
        let operation = registry.recent_operations(1).unwrap().remove(0);
        assert_eq!(operation.state, "rollback_failed");
        assert!(operation.detail.unwrap().contains("unexpected-directory"));
    }

    #[test]
    fn partial_promotion_failure_restores_prior_files() {
        let temp = tempfile::tempdir().unwrap();
        let target = temp.path().join("target");
        let stage = temp.path().join("stage");
        fs::create_dir_all(&target).unwrap();
        fs::create_dir_all(&stage).unwrap();
        let ci = target.join(standard::CI_PATH);
        fs::create_dir_all(ci.parent().unwrap()).unwrap();
        fs::write(&ci, b"prior CI bytes").unwrap();

        let actions = [
            RemediationAction {
                kind: ActionKind::InstallStandardAsset,
                path: standard::CI_PATH.to_string(),
                owner: "forge.standard".to_string(),
                expected_digest: Some(standard::sha256_hex(b"prior CI bytes")),
                content: "new CI bytes".to_string(),
                source: "baseline-service@1.1.0".to_string(),
            },
            RemediationAction {
                kind: ActionKind::InstallStandardAsset,
                path: standard::COMPOSE_PATH.to_string(),
                owner: "forge.standard".to_string(),
                expected_digest: None,
                content: "new Compose bytes".to_string(),
                source: "baseline-service@1.1.0".to_string(),
            },
        ];
        for action in &actions {
            let staged = stage.join(&action.path);
            fs::create_dir_all(staged.parent().unwrap()).unwrap();
            fs::write(staged, action.content.as_bytes()).unwrap();
        }
        let registry = Registry::open(&temp.path().join("registry.db")).unwrap();
        let result = promote_staged(
            &actions,
            &stage,
            &target,
            &registry,
            "demo",
            "plan-partial",
            Some(1),
        );
        assert!(matches!(
            result,
            Err(ForgeError::RemediationApplyFailed { .. })
        ));
        assert_eq!(fs::read(ci).unwrap(), b"prior CI bytes");
        assert!(!target.join(standard::COMPOSE_PATH).exists());
        let operation = registry.recent_operations(1).unwrap().remove(0);
        assert_eq!(operation.state, "rolled_back");
    }
}
