//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use super::super::{
    redact_release_evidence, state_path_for, ReleaseAdapterConfig, ReleaseConfig, ReleaseIdentity,
    ReleaseReport, ReleaseRequest, ReleaseState, StageOutcome, CHECK_PASS,
    RELEASE_CONTRACT_VERSION, STAGE_COMMIT, STAGE_CONTAINER, STAGE_DOCS, STAGE_MIRROR, STAGE_NOTES,
    STAGE_PACKAGE, STAGE_PUSH, STAGE_TAG, STATUS_CONFLICT, STATUS_DELIVERED, STATUS_DISABLED,
    STATUS_FAILED, STATUS_SKIPPED,
};
use crate::core::manifest::Manifest;
use crate::core::ForgeError;
use crate::gitops::commit_paths;
use crate::policy::DriftWatchConfig;
use std::path::Path;
use std::process::Command;

use super::prepare::{
    collect_captured_checks, effective_docs_locales, effective_stages, load_release_changelog,
    short_revision,
};
use super::stages::{
    run_container_stage, run_docs_stage, run_mirror_stage, run_notes_stage, run_package_stage,
    run_push_stage,
};

/// Apply a verified plan. The plan must be `ready` and the
/// captured source revision must match the working-tree
/// revision; otherwise the apply step refuses with
/// `release-check-failed` and a recovery note. Each stage is
/// recorded with a stable status so a retry resumes from the
/// last delivered stage.
pub fn apply_release(
    project_dir: &Path,
    manifest: &Manifest,
    config: &ReleaseConfig,
    request: &ReleaseRequest,
    adapters: &ReleaseAdapterConfig,
) -> Result<ReleaseReport, ForgeError> {
    request.validate()?;
    let revision = capture_source_revision(project_dir)?;
    let identity = ReleaseIdentity::derive(&request.project_id, &request.version, &revision);
    let state_path = state_path_for(project_dir, &request.project_id, &identity)?;
    let mut state = super::super::load_release_state(&state_path)?;
    let changelog = load_release_changelog(project_dir, config)?;
    let docs_locales = effective_docs_locales(manifest, config);
    let stages = effective_stages(config, request);

    if state.contract.is_empty() {
        state.contract = RELEASE_CONTRACT_VERSION.to_string();
        state.identity = identity.clone();
        state.changelog = Some(changelog.clone());
        state.docs_locales = docs_locales.clone();
        state.stages = stages.clone();
    } else {
        // Identity bound: the persisted state belongs to the
        // exact project+version+revision triple. Anything else
        // is a different release attempt.
        if state.identity != identity {
            return Err(ForgeError::ReleaseInvalid {
                reason: format!(
                    "release state at `{}` belongs to release id `{}`, not `{}`; rerun with the matching version or revision",
                    state_path.display(),
                    state.identity.id,
                    identity.id
                ),
            });
        }
        if !stages.is_empty() {
            state.stages = stages.clone();
        }
    }

    // Re-run the captured checks so a stale plan cannot be
    // applied after the working tree has moved on.
    let policy = DriftWatchConfig::from_env();
    let checks = collect_captured_checks(project_dir, manifest, config, &revision, &policy)?;
    let ready = checks
        .iter()
        .filter(|c| c.applicable)
        .all(|c| c.status == CHECK_PASS);
    if !ready {
        let failing: Vec<String> = checks
            .iter()
            .filter(|c| c.applicable && c.status != CHECK_PASS)
            .map(|c| format!("{}:{}", c.kind, c.status))
            .collect();
        return Err(ForgeError::ReleaseCheckFailed {
            reason: format!(
                "release check(s) {} at revision `{}` are not passing; release execution blocks before tag or publication",
                failing.join(", "),
                short_revision(&revision)
            ),
        });
    }
    if !request.confirm && !request.dry_run {
        return Err(ForgeError::ReleaseInvalid {
            reason: "release apply requires --confirm; refusing implicit side effects".to_string(),
        });
    }
    state.checks = checks.clone();

    let mut stage_outcomes: Vec<StageOutcome> = Vec::new();
    let mut deliverable_count = 0usize;
    for stage in &stages {
        let outcome = if request.retry && already_delivered(&state, stage, &revision) {
            StageOutcome {
                stage: stage.to_string(),
                target: stage_target(stage, manifest, config),
                status: STATUS_SKIPPED.to_string(),
                identity: Some(identity.id.clone()),
                note: format!(
                    "stage `{stage}` already delivered at revision `{}`; retry skips",
                    short_revision(&revision)
                ),
                evidence: Vec::new(),
                recovery: Vec::new(),
            }
        } else {
            let outcome = run_stage(
                project_dir,
                manifest,
                config,
                stage,
                &identity,
                &revision,
                adapters,
                request,
            )?;
            if outcome.status == STATUS_DELIVERED {
                deliverable_count += 1;
            }
            outcome
        };
        state.stage_outcomes.push(outcome.clone());
        stage_outcomes.push(outcome);
    }
    state.last_run_at = chrono::Utc::now().to_rfc3339();
    super::super::save_release_state(&state_path, &state)?;

    let dry_run = request.dry_run;
    let retry = request.retry;
    let healthy = stage_outcomes.iter().all(|o| {
        matches!(
            o.status.as_str(),
            STATUS_DELIVERED | STATUS_SKIPPED | STATUS_DISABLED
        )
    }) && !stage_outcomes.is_empty();
    let note = if dry_run {
        "dry-run: no side effects were attempted".to_string()
    } else if healthy {
        format!(
            "release `{}` delivered; {deliverable_count} stage(s) recorded for revision `{}`",
            identity.id,
            short_revision(&revision)
        )
    } else {
        let failing: Vec<String> = stage_outcomes
            .iter()
            .filter(|o| {
                !matches!(
                    o.status.as_str(),
                    STATUS_DELIVERED | STATUS_SKIPPED | STATUS_DISABLED
                )
            })
            .map(|o| format!("{}:{}", o.stage, o.status))
            .collect();
        format!(
            "release `{}` is partial; failing stage(s) {}; prior stage state is preserved",
            identity.id,
            failing.join(", ")
        )
    };
    Ok(ReleaseReport {
        contract: RELEASE_CONTRACT_VERSION.to_string(),
        project_id: request.project_id.clone(),
        identity,
        changelog: Some(changelog),
        docs_locales,
        checks,
        stages,
        stage_outcomes,
        dry_run,
        retry,
        state_path: state_path.display().to_string(),
        note,
        healthy,
    })
}

pub(super) fn capture_source_revision(dir: &Path) -> Result<String, ForgeError> {
    super::super::capture_source_revision(dir)
}

fn already_delivered(state: &ReleaseState, stage: &str, revision: &str) -> bool {
    state
        .stage_outcomes
        .iter()
        .any(|o| o.stage == stage && (o.status == STATUS_DELIVERED || o.status == STATUS_SKIPPED))
        && state.identity.source_revision == revision
}

fn stage_target(stage: &str, manifest: &Manifest, config: &ReleaseConfig) -> String {
    match stage {
        STAGE_COMMIT => "working-tree".to_string(),
        STAGE_TAG => format!("refs/tags/{}", manifest.project.id),
        STAGE_PUSH => "origin".to_string(),
        STAGE_MIRROR => "distribution/mirrors".to_string(),
        STAGE_PACKAGE => config
            .packages
            .first()
            .map(|p| p.name.clone())
            .unwrap_or_else(|| "(none)".to_string()),
        STAGE_CONTAINER => config
            .containers
            .first()
            .map(|c| c.name.clone())
            .unwrap_or_else(|| "(none)".to_string()),
        STAGE_DOCS => "docs/translate".to_string(),
        STAGE_NOTES => config
            .notes
            .as_ref()
            .map(|n| n.output.clone())
            .unwrap_or_else(|| "(none)".to_string()),
        _ => "(unknown)".to_string(),
    }
}

#[allow(clippy::too_many_arguments)]
fn run_stage(
    project_dir: &Path,
    manifest: &Manifest,
    config: &ReleaseConfig,
    stage: &str,
    identity: &ReleaseIdentity,
    revision: &str,
    adapters: &ReleaseAdapterConfig,
    request: &ReleaseRequest,
) -> Result<StageOutcome, ForgeError> {
    match stage {
        STAGE_COMMIT => run_commit_stage(project_dir, manifest, config, identity, revision),
        STAGE_TAG => run_tag_stage(project_dir, manifest, identity, revision, request),
        STAGE_PUSH => run_push_stage(project_dir, manifest, identity, revision, request),
        STAGE_MIRROR => run_mirror_stage(project_dir, manifest, identity, revision, request),
        STAGE_PACKAGE => {
            run_package_stage(project_dir, config, identity, revision, adapters, request)
        }
        STAGE_CONTAINER => {
            run_container_stage(project_dir, config, identity, revision, adapters, request)
        }
        STAGE_DOCS => run_docs_stage(project_dir, manifest, config, identity, revision),
        STAGE_NOTES => run_notes_stage(project_dir, config, identity, revision, adapters, request),
        other => Ok(StageOutcome {
            stage: other.to_string(),
            target: "(unknown)".to_string(),
            status: STATUS_DISABLED.to_string(),
            identity: Some(identity.id.clone()),
            note: format!("stage `{other}` is not recognized; treating as disabled"),
            evidence: Vec::new(),
            recovery: Vec::new(),
        }),
    }
}

fn run_commit_stage(
    project_dir: &Path,
    manifest: &Manifest,
    config: &ReleaseConfig,
    identity: &ReleaseIdentity,
    revision: &str,
) -> Result<StageOutcome, ForgeError> {
    // The release commit bundles the changelog (and any
    // release-managed files) into one commit. The user can
    // override the path list with `release.commit.paths` is
    // out of scope; the default is the changelog only so
    // unrelated working-tree edits are never silently pulled
    // in.
    let path = config.changelog.clone();
    if !project_dir.join(&path).is_file() {
        return Ok(StageOutcome {
            stage: STAGE_COMMIT.to_string(),
            target: path.clone(),
            status: STATUS_DISABLED.to_string(),
            identity: Some(identity.id.clone()),
            note: format!(
                "no changelog to commit at `{}`; stage `{}` skipped",
                path, STAGE_COMMIT
            ),
            evidence: Vec::new(),
            recovery: Vec::new(),
        });
    }
    let message = format!(
        "release {} ({})",
        identity.version,
        short_revision(revision)
    );
    let outcome = match commit_paths(
        &manifest.project.id,
        project_dir,
        std::slice::from_ref(&path),
        &message,
    ) {
        Ok(outcome) => outcome,
        Err(err) => {
            return Ok(StageOutcome {
                stage: STAGE_COMMIT.to_string(),
                target: path,
                status: STATUS_FAILED.to_string(),
                identity: Some(identity.id.clone()),
                note: format!("commit stage failed: {err}"),
                evidence: vec![err.to_string()],
                recovery: vec![
                    "stage the changelog explicitly and re-run forge release apply".to_string(),
                ],
            });
        }
    };
    Ok(StageOutcome {
        stage: STAGE_COMMIT.to_string(),
        target: path,
        status: STATUS_DELIVERED.to_string(),
        identity: Some(identity.id.clone()),
        note: format!(
            "commit stage delivered release `{}` for project `{}`",
            identity.id, manifest.project.id
        ),
        evidence: vec![format!(
            "files: {}",
            if outcome.files_changed.is_empty() {
                "(none)".to_string()
            } else {
                outcome.files_changed.join(", ")
            }
        )],
        recovery: Vec::new(),
    })
}

fn run_tag_stage(
    project_dir: &Path,
    manifest: &Manifest,
    identity: &ReleaseIdentity,
    revision: &str,
    _request: &ReleaseRequest,
) -> Result<StageOutcome, ForgeError> {
    let tag = format!("v{}", identity.version);
    // The tag is annotated so the release identity is
    // preserved alongside the commit. A pre-existing tag at a
    // different commit is a `release-identity-conflict`; a
    // pre-existing tag at the same commit is `skipped` so
    // retries are idempotent.
    let existing = read_tag_target(project_dir, &tag)?;
    if let Some(existing_sha) = existing {
        if existing_sha != revision {
            return Ok(StageOutcome {
                stage: STAGE_TAG.to_string(),
                target: tag.clone(),
                status: STATUS_CONFLICT.to_string(),
                identity: Some(identity.id.clone()),
                note: format!(
                    "tag `{tag}` already exists at commit `{existing_sha}`; refusing to replace it with `{revision}`"
                ),
                evidence: vec![format!("existing: {existing_sha}"), format!("requested: {revision}")],
                recovery: vec![
                    "remove the conflicting tag with `git tag -d {tag}` and re-run forge release apply".to_string(),
                    "or release at a different semver to keep history immutable".to_string(),
                ],
            });
        }
        return Ok(StageOutcome {
            stage: STAGE_TAG.to_string(),
            target: tag.clone(),
            status: STATUS_SKIPPED.to_string(),
            identity: Some(identity.id.clone()),
            note: format!("tag `{tag}` already points at `{revision}`; tag stage skipped"),
            evidence: Vec::new(),
            recovery: Vec::new(),
        });
    }
    let output = Command::new("git")
        .arg("-C")
        .arg(project_dir)
        .arg("tag")
        .arg("-a")
        .arg(&tag)
        .arg("-m")
        .arg(format!(
            "release {} for {}",
            identity.version, manifest.project.id
        ))
        .arg(revision)
        .output()
        .map_err(|err| ForgeError::ReleaseInvalid {
            reason: format!("git tag invocation failed: {err}"),
        })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Ok(StageOutcome {
            stage: STAGE_TAG.to_string(),
            target: tag.clone(),
            status: STATUS_FAILED.to_string(),
            identity: Some(identity.id.clone()),
            note: format!("git tag -a `{tag}` failed"),
            evidence: vec![redact_release_evidence(&stderr)],
            recovery: vec![format!(
                "verify the working tree at `{revision}` is reachable"
            )],
        });
    }
    Ok(StageOutcome {
        stage: STAGE_TAG.to_string(),
        target: tag.clone(),
        status: STATUS_DELIVERED.to_string(),
        identity: Some(identity.id.clone()),
        note: format!("tag `{tag}` delivered for release `{}`", identity.id),
        evidence: vec![format!("annotated tag at {revision}")],
        recovery: Vec::new(),
    })
}

fn read_tag_target(project_dir: &Path, tag: &str) -> Result<Option<String>, ForgeError> {
    // Peel annotated tags to the underlying commit so the
    // comparison against the working-tree revision is the
    // commit SHA, not the tag object SHA.
    let output = Command::new("git")
        .arg("-C")
        .arg(project_dir)
        .arg("rev-parse")
        .arg("--verify")
        .arg(format!("refs/tags/{tag}^{{commit}}"))
        .output()
        .map_err(|err| ForgeError::ReleaseInvalid {
            reason: format!("git rev-parse failed: {err}"),
        })?;
    if !output.status.success() {
        // Lightweight tags do not peel; fall back to the
        // raw SHA so the comparison still works.
        let output = Command::new("git")
            .arg("-C")
            .arg(project_dir)
            .arg("rev-parse")
            .arg("--verify")
            .arg(format!("refs/tags/{tag}"))
            .output()
            .map_err(|err| ForgeError::ReleaseInvalid {
                reason: format!("git rev-parse failed: {err}"),
            })?;
        if !output.status.success() {
            return Ok(None);
        }
        let sha = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if sha.is_empty() {
            return Ok(None);
        }
        return Ok(Some(sha));
    }
    let sha = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if sha.is_empty() {
        Ok(None)
    } else {
        Ok(Some(sha))
    }
}
