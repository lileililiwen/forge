//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use super::super::{
    redact_release_evidence, ReleaseAdapterConfig, ReleaseConfig, ReleaseIdentity, ReleaseRequest,
    StageOutcome, STAGE_CONTAINER, STAGE_DOCS, STAGE_MIRROR, STAGE_NOTES, STAGE_PACKAGE,
    STAGE_PUSH, STATUS_DELIVERED, STATUS_DISABLED, STATUS_DIVERGED, STATUS_FAILED,
};
use crate::core::manifest::Manifest;
use crate::core::ForgeError;
use crate::distribution::{
    apply_mirror, distribution_config_from_manifest, plan_mirror as plan_distribution,
    DistributionConfig, MirrorRequest,
};
use crate::gitops::push_ref;
use std::path::Path;

use super::operations::{run_container_adapter, run_notes_adapter, run_package_adapter};
use super::prepare::effective_docs_locales;

pub(super) fn run_push_stage(
    project_dir: &Path,
    manifest: &Manifest,
    identity: &ReleaseIdentity,
    revision: &str,
    request: &ReleaseRequest,
) -> Result<StageOutcome, ForgeError> {
    let tag = format!("v{}", identity.version);
    if !request.confirm {
        return Ok(StageOutcome {
            stage: STAGE_PUSH.to_string(),
            target: format!("origin:{tag}"),
            status: STATUS_DISABLED.to_string(),
            identity: Some(identity.id.clone()),
            note: "push stage requires --confirm; treating as disabled".to_string(),
            evidence: Vec::new(),
            recovery: vec![
                "re-run with --confirm to push the release tag to the primary remote".to_string(),
            ],
        });
    }
    let outcome = match push_ref(&manifest.project.id, project_dir, "origin", &tag, true) {
        Ok(outcome) => outcome,
        Err(err) => {
            return Ok(StageOutcome {
                stage: STAGE_PUSH.to_string(),
                target: format!("origin:{tag}"),
                status: STATUS_FAILED.to_string(),
                identity: Some(identity.id.clone()),
                note: format!("push stage failed: {err}"),
                evidence: vec![redact_release_evidence(&err.to_string())],
                recovery: vec![
                    "verify the `origin` remote and the credential helper, then re-run forge release apply".to_string()
                ],
            });
        }
    };
    let _ = revision;
    Ok(StageOutcome {
        stage: STAGE_PUSH.to_string(),
        target: format!("origin:{tag}"),
        status: STATUS_DELIVERED.to_string(),
        identity: Some(identity.id.clone()),
        note: format!(
            "push stage delivered `{tag}` to `origin` for release `{}`",
            identity.id
        ),
        evidence: outcome
            .commit_sha
            .map(|s| format!("commit: {s}"))
            .into_iter()
            .collect(),
        recovery: Vec::new(),
    })
}

pub(super) fn run_mirror_stage(
    project_dir: &Path,
    manifest: &Manifest,
    identity: &ReleaseIdentity,
    revision: &str,
    request: &ReleaseRequest,
) -> Result<StageOutcome, ForgeError> {
    let config: DistributionConfig = match manifest.distribution.as_ref() {
        Some(_) => match distribution_config_from_manifest(manifest) {
            Ok(c) => c,
            Err(err) => {
                return Ok(StageOutcome {
                    stage: STAGE_MIRROR.to_string(),
                    target: "distribution/mirrors".to_string(),
                    status: STATUS_FAILED.to_string(),
                    identity: Some(identity.id.clone()),
                    note: format!("mirror stage failed: {err}"),
                    evidence: vec![err.to_string()],
                    recovery: vec![
                        "declare a `distribution` section in the manifest with a primary or at least one mirror".to_string(),
                    ],
                });
            }
        },
        None => {
            return Ok(StageOutcome {
                stage: STAGE_MIRROR.to_string(),
                target: "distribution/mirrors".to_string(),
                status: STATUS_DISABLED.to_string(),
                identity: Some(identity.id.clone()),
                note: "no `distribution` section in the manifest; mirror stage skipped".to_string(),
                evidence: Vec::new(),
                recovery: vec![
                    "add a `distribution` section to the manifest to publish mirrors as part of the release".to_string(),
                ],
            });
        }
    };
    let tag = format!("v{}", identity.version);
    let mirror_request = MirrorRequest {
        project_id: manifest.project.id.clone(),
        refs: vec![tag.clone()],
        confirm: request.confirm,
        dry_run: request.dry_run,
        retry_failed: request.retry,
    };
    if !request.confirm {
        // Plan-only path: surface what would happen.
        let state_path = crate::distribution::state_path_for(project_dir, &manifest.project.id)
            .unwrap_or_else(|_| project_dir.join(".forge/distribution/state.json"));
        let state = crate::distribution::load_mirror_state(&state_path).unwrap_or_default();
        let plan = plan_distribution(&config, &mirror_request, &state, &state_path)?;
        let primary = plan
            .outcomes
            .iter()
            .find(|o| o.role == "primary")
            .map(|o| o.status.clone())
            .unwrap_or_else(|| "absent".to_string());
        let mirror_count = plan
            .outcomes
            .iter()
            .filter(|o| o.role == "mirror" && o.status == "would-push")
            .count();
        return Ok(StageOutcome {
            stage: STAGE_MIRROR.to_string(),
            target: "distribution/mirrors".to_string(),
            status: if plan.healthy() {
                STATUS_DELIVERED.to_string()
            } else {
                STATUS_FAILED.to_string()
            },
            identity: Some(identity.id.clone()),
            note: format!(
                "mirror plan only: primary `{primary}`, {mirror_count} mirror(s) scheduled for `{tag}` (retry)"
            ),
            evidence: plan
                .outcomes
                .iter()
                .map(|o| format!("{} {} {}", o.role, o.provider, o.status))
                .collect(),
            recovery: Vec::new(),
        });
    }
    let _ = revision;
    let report = apply_mirror(project_dir, &config, &mirror_request)?;
    let primary = report
        .outcomes
        .iter()
        .find(|o| o.role == "primary")
        .map(|o| o.status.clone())
        .unwrap_or_else(|| "absent".to_string());
    let delivered_mirrors = report
        .outcomes
        .iter()
        .filter(|o| o.role == "mirror" && o.status == STATUS_DELIVERED)
        .count();
    let overall = if report.healthy() {
        STATUS_DELIVERED.to_string()
    } else if primary == STATUS_DIVERGED {
        STATUS_DIVERGED.to_string()
    } else {
        STATUS_FAILED.to_string()
    };
    Ok(StageOutcome {
        stage: STAGE_MIRROR.to_string(),
        target: "distribution/mirrors".to_string(),
        status: overall,
        identity: Some(identity.id.clone()),
        note: format!(
            "mirror stage: primary `{primary}`, {delivered_mirrors} mirror(s) delivered for `{tag}`"
        ),
        evidence: report
            .outcomes
            .iter()
            .map(|o| format!("{} {} {}", o.role, o.provider, o.status))
            .collect(),
        recovery: report
            .outcomes
            .iter()
            .filter(|o| o.status == STATUS_FAILED || o.status == STATUS_DIVERGED)
            .flat_map(|o| o.recovery.clone())
            .collect(),
    })
}

pub(super) fn run_package_stage(
    project_dir: &Path,
    config: &ReleaseConfig,
    identity: &ReleaseIdentity,
    revision: &str,
    adapters: &ReleaseAdapterConfig,
    request: &ReleaseRequest,
) -> Result<StageOutcome, ForgeError> {
    if config.packages.is_empty() {
        return Ok(StageOutcome {
            stage: STAGE_PACKAGE.to_string(),
            target: "(none)".to_string(),
            status: STATUS_DISABLED.to_string(),
            identity: Some(identity.id.clone()),
            note: "no packages configured in `release.packages`; package stage skipped".to_string(),
            evidence: Vec::new(),
            recovery: vec![
                "declare `release.packages` entries to publish one package per release".to_string(),
            ],
        });
    }
    if !request.confirm {
        return Ok(StageOutcome {
            stage: STAGE_PACKAGE.to_string(),
            target: config
                .packages
                .first()
                .map(|p| p.name.clone())
                .unwrap_or_else(|| "(none)".to_string()),
            status: STATUS_DISABLED.to_string(),
            identity: Some(identity.id.clone()),
            note: "package stage requires --confirm; treating as disabled".to_string(),
            evidence: Vec::new(),
            recovery: vec![
                "re-run with --confirm to publish packages through the package adapter".to_string(),
            ],
        });
    }
    let _ = revision;
    let mut outcomes: Vec<StageOutcome> = Vec::new();
    for package in &config.packages {
        let target = format!("{}@{}", package.name, package.version);
        match run_package_adapter(&adapters.package_bin, project_dir, package, identity) {
            Ok(receipt) => outcomes.push(StageOutcome {
                stage: STAGE_PACKAGE.to_string(),
                target: target.clone(),
                status: STATUS_DELIVERED.to_string(),
                identity: Some(identity.id.clone()),
                note: format!("package `{target}` delivered for release `{}`", identity.id),
                evidence: vec![format!("receipt: {receipt}")],
                recovery: Vec::new(),
            }),
            Err(err) => outcomes.push(StageOutcome {
                stage: STAGE_PACKAGE.to_string(),
                target,
                status: STATUS_FAILED.to_string(),
                identity: Some(identity.id.clone()),
                note: format!("package stage failed: {err}"),
                evidence: vec![redact_release_evidence(&err)],
                recovery: vec![format!(
                    "verify the package adapter `{}` and re-run forge release apply",
                    adapters.package_bin
                )],
            }),
        }
    }
    // Aggregate to one StageOutcome so the per-stage record
    // is stable; the per-package evidence is preserved in
    // `evidence` lines.
    let all_delivered = outcomes.iter().all(|o| o.status == STATUS_DELIVERED);
    let note = if all_delivered {
        format!(
            "{} package(s) delivered for release `{}`",
            outcomes.len(),
            identity.id
        )
    } else {
        let failing: Vec<String> = outcomes
            .iter()
            .filter(|o| o.status != STATUS_DELIVERED)
            .map(|o| format!("{}:{}", o.target, o.status))
            .collect();
        format!(
            "{} package(s) failed: {}",
            outcomes.len(),
            failing.join(", ")
        )
    };
    let evidence: Vec<String> = outcomes
        .iter()
        .map(|o| format!("{} -> {}", o.target, o.status))
        .collect();
    let recovery: Vec<String> = outcomes
        .iter()
        .filter(|o| !o.recovery.is_empty())
        .flat_map(|o| o.recovery.clone())
        .collect();
    let status = if all_delivered {
        STATUS_DELIVERED.to_string()
    } else {
        STATUS_FAILED.to_string()
    };
    Ok(StageOutcome {
        stage: STAGE_PACKAGE.to_string(),
        target: "packages".to_string(),
        status,
        identity: Some(identity.id.clone()),
        note,
        evidence,
        recovery,
    })
}

pub(super) fn run_container_stage(
    project_dir: &Path,
    config: &ReleaseConfig,
    identity: &ReleaseIdentity,
    revision: &str,
    adapters: &ReleaseAdapterConfig,
    request: &ReleaseRequest,
) -> Result<StageOutcome, ForgeError> {
    if config.containers.is_empty() {
        return Ok(StageOutcome {
            stage: STAGE_CONTAINER.to_string(),
            target: "(none)".to_string(),
            status: STATUS_DISABLED.to_string(),
            identity: Some(identity.id.clone()),
            note: "no containers configured in `release.containers`; container stage skipped"
                .to_string(),
            evidence: Vec::new(),
            recovery: vec![
                "declare `release.containers` entries to publish one image per release".to_string(),
            ],
        });
    }
    if !request.confirm {
        return Ok(StageOutcome {
            stage: STAGE_CONTAINER.to_string(),
            target: config
                .containers
                .first()
                .map(|c| c.name.clone())
                .unwrap_or_else(|| "(none)".to_string()),
            status: STATUS_DISABLED.to_string(),
            identity: Some(identity.id.clone()),
            note: "container stage requires --confirm; treating as disabled".to_string(),
            evidence: Vec::new(),
            recovery: vec![
                "re-run with --confirm to publish container images through the container adapter"
                    .to_string(),
            ],
        });
    }
    let _ = revision;
    let mut outcomes: Vec<StageOutcome> = Vec::new();
    for container in &config.containers {
        let target = format!("{}:{}", container.name, container.tag);
        match run_container_adapter(&adapters.container_bin, project_dir, container, identity) {
            Ok(receipt) => outcomes.push(StageOutcome {
                stage: STAGE_CONTAINER.to_string(),
                target: target.clone(),
                status: STATUS_DELIVERED.to_string(),
                identity: Some(identity.id.clone()),
                note: format!(
                    "container `{target}` delivered for release `{}`",
                    identity.id
                ),
                evidence: vec![format!("receipt: {receipt}")],
                recovery: Vec::new(),
            }),
            Err(err) => outcomes.push(StageOutcome {
                stage: STAGE_CONTAINER.to_string(),
                target,
                status: STATUS_FAILED.to_string(),
                identity: Some(identity.id.clone()),
                note: format!("container stage failed: {err}"),
                evidence: vec![redact_release_evidence(&err)],
                recovery: vec![format!(
                    "verify the container adapter `{}` and re-run forge release apply",
                    adapters.container_bin
                )],
            }),
        }
    }
    let all_delivered = outcomes.iter().all(|o| o.status == STATUS_DELIVERED);
    let evidence: Vec<String> = outcomes
        .iter()
        .map(|o| format!("{} -> {}", o.target, o.status))
        .collect();
    let recovery: Vec<String> = outcomes
        .iter()
        .filter(|o| !o.recovery.is_empty())
        .flat_map(|o| o.recovery.clone())
        .collect();
    let status = if all_delivered {
        STATUS_DELIVERED.to_string()
    } else {
        STATUS_FAILED.to_string()
    };
    let note = if all_delivered {
        format!(
            "{} container(s) delivered for release `{}`",
            outcomes.len(),
            identity.id
        )
    } else {
        let failing: Vec<String> = outcomes
            .iter()
            .filter(|o| o.status != STATUS_DELIVERED)
            .map(|o| format!("{}:{}", o.target, o.status))
            .collect();
        format!(
            "{} container(s) failed: {}",
            outcomes.len(),
            failing.join(", ")
        )
    };
    Ok(StageOutcome {
        stage: STAGE_CONTAINER.to_string(),
        target: "containers".to_string(),
        status,
        identity: Some(identity.id.clone()),
        note,
        evidence,
        recovery,
    })
}

pub(super) fn run_docs_stage(
    project_dir: &Path,
    manifest: &Manifest,
    config: &ReleaseConfig,
    identity: &ReleaseIdentity,
    revision: &str,
) -> Result<StageOutcome, ForgeError> {
    let locales: Vec<String> = effective_docs_locales(manifest, config);
    if locales.is_empty() {
        return Ok(StageOutcome {
            stage: STAGE_DOCS.to_string(),
            target: "docs/translate".to_string(),
            status: STATUS_DISABLED.to_string(),
            identity: Some(identity.id.clone()),
            note: "no enabled translation locale configured; docs stage skipped".to_string(),
            evidence: Vec::new(),
            recovery: vec![
                "enable a locale in `docs.translations.<locale>.enabled` and add it to `release.docs.translate` to translate during the release".to_string(),
            ],
        });
    }
    let _ = revision;
    let translator = crate::docs::TranslatorConfig::from_env();
    let translator_default = translator.binary.to_string_lossy().to_string();
    let mut outcomes: Vec<StageOutcome> = Vec::new();
    for locale in &locales {
        let request = crate::docs::TranslateRequest {
            project_id: manifest.project.id.clone(),
            locale: Some(locale.clone()),
            all: false,
        };
        let docs_config = match crate::docs::docs_config_from_manifest(manifest) {
            Ok(c) => c,
            Err(err) => {
                outcomes.push(StageOutcome {
                    stage: STAGE_DOCS.to_string(),
                    target: format!("docs/translate:{locale}"),
                    status: STATUS_FAILED.to_string(),
                    identity: Some(identity.id.clone()),
                    note: format!("docs stage failed to read config: {err}"),
                    evidence: vec![err.to_string()],
                    recovery: vec!["verify the `docs` section in the manifest".to_string()],
                });
                continue;
            }
        };
        let report =
            match crate::docs::run_translate(project_dir, &docs_config, &request, &translator) {
                Ok(report) => report,
                Err(err) => {
                    outcomes.push(StageOutcome {
                        stage: STAGE_DOCS.to_string(),
                        target: format!("docs/translate:{locale}"),
                        status: STATUS_FAILED.to_string(),
                        identity: Some(identity.id.clone()),
                        note: format!("docs stage failed: {err}"),
                        evidence: vec![redact_release_evidence(&err.to_string())],
                        recovery: vec![format!(
                            "verify the translator `{}` and re-run forge release apply",
                            translator_default
                        )],
                    });
                    continue;
                }
            };
        let outcome = report
            .outcomes
            .iter()
            .find(|o| o.locale == *locale)
            .cloned()
            .unwrap_or(crate::docs::TranslateOutcome {
                locale: locale.clone(),
                status: crate::docs::STATUS_FAILED.to_string(),
                source_hash: None,
                segments_translated: 0,
                segments_reused: 0,
                derivative: None,
                review: "failed".to_string(),
                review_reasons: Vec::new(),
                note: "locale was not present in the docs report".to_string(),
                evidence: Vec::new(),
                recovery: Vec::new(),
            });
        let status = if outcome.status == crate::docs::STATUS_TRANSLATED
            || outcome.status == crate::docs::STATUS_CURRENT
        {
            STATUS_DELIVERED.to_string()
        } else {
            STATUS_FAILED.to_string()
        };
        outcomes.push(StageOutcome {
            stage: STAGE_DOCS.to_string(),
            target: format!("docs/translate:{locale}"),
            status,
            identity: Some(identity.id.clone()),
            note: format!("docs stage for locale `{locale}`: {}", outcome.note),
            evidence: outcome.evidence,
            recovery: outcome.recovery,
        });
    }
    let all_delivered = outcomes.iter().all(|o| o.status == STATUS_DELIVERED);
    let evidence: Vec<String> = outcomes
        .iter()
        .map(|o| format!("{} -> {}", o.target, o.status))
        .collect();
    let recovery: Vec<String> = outcomes
        .iter()
        .filter(|o| !o.recovery.is_empty())
        .flat_map(|o| o.recovery.clone())
        .collect();
    let status = if all_delivered {
        STATUS_DELIVERED.to_string()
    } else {
        STATUS_FAILED.to_string()
    };
    let note = if all_delivered {
        format!(
            "{} docs locale(s) delivered for release `{}`",
            outcomes.len(),
            identity.id
        )
    } else {
        let failing: Vec<String> = outcomes
            .iter()
            .filter(|o| o.status != STATUS_DELIVERED)
            .map(|o| format!("{}:{}", o.target, o.status))
            .collect();
        format!(
            "{} docs locale(s) failed: {}",
            outcomes.len(),
            failing.join(", ")
        )
    };
    Ok(StageOutcome {
        stage: STAGE_DOCS.to_string(),
        target: "docs/translate".to_string(),
        status,
        identity: Some(identity.id.clone()),
        note,
        evidence,
        recovery,
    })
}

pub(super) fn run_notes_stage(
    project_dir: &Path,
    config: &ReleaseConfig,
    identity: &ReleaseIdentity,
    revision: &str,
    adapters: &ReleaseAdapterConfig,
    request: &ReleaseRequest,
) -> Result<StageOutcome, ForgeError> {
    let notes = match &config.notes {
        Some(notes) => notes,
        None => {
            return Ok(StageOutcome {
                stage: STAGE_NOTES.to_string(),
                target: "(none)".to_string(),
                status: STATUS_DISABLED.to_string(),
                identity: Some(identity.id.clone()),
                note: "no `release.notes` configured; notes stage skipped".to_string(),
                evidence: Vec::new(),
                recovery: vec![
                    "declare a `release.notes.template` and `release.notes.output` to render notes during the release".to_string(),
                ],
            });
        }
    };
    let _ = (revision, request);
    match run_notes_adapter(&adapters.notes_bin, project_dir, notes, identity, request) {
        Ok(receipt) => Ok(StageOutcome {
            stage: STAGE_NOTES.to_string(),
            target: notes.output.clone(),
            status: STATUS_DELIVERED.to_string(),
            identity: Some(identity.id.clone()),
            note: format!(
                "notes rendered to `{}` for release `{}`",
                notes.output, identity.id
            ),
            evidence: vec![format!("receipt: {receipt}")],
            recovery: Vec::new(),
        }),
        Err(err) => Ok(StageOutcome {
            stage: STAGE_NOTES.to_string(),
            target: notes.output.clone(),
            status: STATUS_FAILED.to_string(),
            identity: Some(identity.id.clone()),
            note: format!("notes stage failed: {err}"),
            evidence: vec![redact_release_evidence(&err)],
            recovery: vec![format!(
                "verify the notes adapter `{}` and re-run forge release apply",
                adapters.notes_bin
            )],
        }),
    }
}
