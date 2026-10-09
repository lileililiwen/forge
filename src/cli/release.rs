//! Release, docs and deploy commands (`docs`/`release`/`deploy`).
//!
//! Typed CLI handlers for the docs translation, release lifecycle and
//! deploy surfaces. Bodies moved verbatim from the split of `src/main.rs`.
//!
//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use forge::core::ForgeError;
use forge::deploy::DeployRequest;
use forge::docs::{
    docs_config_from_manifest, run_translate, TranslateReport, TranslateRequest, TranslatorConfig,
};
use forge::policy::DriftWatchConfig;
use forge::release::engine::{
    apply_release, list_releases, prepare_release, read_release, PlanReport as EnginePlanReport,
    ReleaseListEntry,
};
use forge::release::{
    release_config_from_manifest, render_report_human as render_release_report_human,
    ReleaseAdapterConfig, ReleaseReport, ReleaseRequest, ReleaseState, Semver,
};
use std::path::{Path, PathBuf};

use super::commands::DocsCommands;
use super::commands_ops::DeployCommands;
use super::commands_services::ReleaseCommands;
use super::feature::truncate;
use super::fleet_exec::{deploy_list_output, deploy_plan_output, deploy_state_output};
use super::projects::{as_output, open_registry};
use super::publish::{
    cmd_deploy_apply, cmd_deploy_observe, cmd_deploy_status_watch, resolve_deploy_target_name,
};
use crate::{Format, Output};

pub(crate) fn cmd_docs(
    db_path: &Path,
    command: &DocsCommands,
    format: Format,
) -> Result<Output, ForgeError> {
    match command {
        DocsCommands::Translate {
            locale,
            all,
            project,
        } => cmd_docs_translate(db_path, project, locale.clone(), *all, format),
    }
}

fn cmd_docs_translate(
    db_path: &Path,
    project: &Path,
    locale: Option<String>,
    all: bool,
    format: Format,
) -> Result<Output, ForgeError> {
    let canonical = project
        .canonicalize()
        .map_err(|_| ForgeError::PathUnavailable {
            path: project.display().to_string(),
        })?;
    let (manifest, _) = forge::core::manifest::Manifest::load_from_dir(&canonical, None)?;
    let config = docs_config_from_manifest(&manifest)?;
    let translator = TranslatorConfig::from_env();
    let request = TranslateRequest {
        project_id: manifest.project.id.clone(),
        locale,
        all,
    };
    let report = run_translate(&canonical, &config, &request, &translator)?;
    let registry = open_registry(db_path)?;
    let statuses: Vec<String> = report
        .outcomes
        .iter()
        .map(|o| format!("{}={}", o.locale, o.status))
        .collect();
    let detail = format!(
        "docs translate `{}` locales={} healthy={} all={}",
        report.project_id,
        statuses.join(","),
        report.healthy,
        report.all
    );
    let state_label = if report.healthy() { "done" } else { "partial" };
    let _ = registry.record_operation("docs", &report.project_id, state_label, &detail);
    let output = docs_translate_output(&report, format)?;
    if report.healthy() {
        Ok(output)
    } else {
        // Partial failure: print the per-locale outcome JSON to
        // stdout so the caller sees exactly which locales were
        // translated and which failed, then surface a typed
        // exit-code error. The prior derivatives are intact.
        match &output {
            Output::Human(text) => println!("{text}"),
            Output::Json(value) => {
                println!("{}", serde_json::to_string_pretty(value).unwrap());
            }
            Output::Raw(text) => print!("{text}"),
        }
        Err(ForgeError::TranslationFailed {
            reason: report.note.clone(),
        })
    }
}

fn docs_translate_output(report: &TranslateReport, format: Format) -> Result<Output, ForgeError> {
    let json = serde_json::json!({"translate": report});
    let human = forge::docs::render_report_human(report);
    Ok(as_output(format, human, json))
}

pub(crate) fn cmd_release(
    db_path: &Path,
    command: &ReleaseCommands,
    format: Format,
) -> Result<Output, ForgeError> {
    match command {
        ReleaseCommands::Prepare {
            target,
            version,
            dry_run,
        } => cmd_release_prepare(db_path, target, version, *dry_run, format),
        ReleaseCommands::Apply {
            target,
            version,
            confirm,
            retry,
            dry_run,
            stages,
        } => cmd_release_apply(
            db_path, target, version, *confirm, *retry, *dry_run, stages, format,
        ),
        ReleaseCommands::List { target } => cmd_release_list(db_path, target, format),
        ReleaseCommands::Inspect { release_id, target } => {
            cmd_release_inspect(db_path, target, release_id, format)
        }
    }
}

fn resolve_release_target(target: &str) -> Result<(PathBuf, String), ForgeError> {
    let candidate = Path::new(target);
    if candidate.is_dir() {
        let canonical = candidate
            .canonicalize()
            .map_err(|_| ForgeError::PathUnavailable {
                path: target.to_string(),
            })?;
        let (manifest, _) = forge::core::manifest::Manifest::load_from_dir(&canonical, None)?;
        return Ok((canonical, manifest.project.id));
    }
    Err(ForgeError::PathUnavailable {
        path: target.to_string(),
    })
}

fn cmd_release_prepare(
    db_path: &Path,
    target: &str,
    version: &str,
    dry_run: bool,
    format: Format,
) -> Result<Output, ForgeError> {
    let (project_dir, project_id) = resolve_release_target(target)?;
    let (manifest, _) = forge::core::manifest::Manifest::load_from_dir(&project_dir, None)?;
    let config = release_config_from_manifest(&manifest)?;
    let semver = Semver::parse(version)?;
    let request = ReleaseRequest {
        project_id: project_id.clone(),
        version: semver,
        confirm: false,
        dry_run,
        retry: false,
        stages: config.stages.clone(),
    };
    let policy = DriftWatchConfig::from_env();
    let plan = prepare_release(&project_dir, &manifest, &config, &request, &policy)?;
    let registry = open_registry(db_path)?;
    let detail = format!(
        "release prepare `{}` v{} revision=`{}` ready={} dry_run={}",
        project_id, plan.identity.version, plan.identity.source_revision, plan.ready, dry_run
    );
    let state_label = if plan.healthy() { "done" } else { "blocked" };
    let _ = registry.record_operation("release", &project_id, state_label, &detail);
    release_plan_output(&plan, format)
}

fn release_plan_output(plan: &EnginePlanReport, format: Format) -> Result<Output, ForgeError> {
    let json = serde_json::json!({"plan": plan});
    let human = forge::release::engine::render_plan_human(plan);
    Ok(as_output(format, human, json))
}

#[allow(clippy::too_many_arguments)]
fn cmd_release_apply(
    db_path: &Path,
    target: &str,
    version: &str,
    confirm: bool,
    retry: bool,
    dry_run: bool,
    stages: &[String],
    format: Format,
) -> Result<Output, ForgeError> {
    let (project_dir, project_id) = resolve_release_target(target)?;
    let (manifest, _) = forge::core::manifest::Manifest::load_from_dir(&project_dir, None)?;
    let mut config = release_config_from_manifest(&manifest)?;
    if !stages.is_empty() {
        config = config.with_stages(stages.to_vec());
    }
    let semver = Semver::parse(version)?;
    let request = ReleaseRequest {
        project_id: project_id.clone(),
        version: semver,
        confirm,
        dry_run,
        retry,
        stages: config.stages.clone(),
    };
    let adapters = ReleaseAdapterConfig::from_env();
    let report = match apply_release(&project_dir, &manifest, &config, &request, &adapters) {
        Ok(report) => report,
        Err(ForgeError::ReleaseCheckFailed { reason }) => {
            return Err(ForgeError::ReleaseCheckFailed { reason });
        }
        Err(err) => return Err(err),
    };
    let registry = open_registry(db_path)?;
    let detail = format!(
        "release apply `{}` v{} revision=`{}` stages={} dry_run={} retry={} healthy={}",
        report.project_id,
        report.identity.version,
        report.identity.source_revision,
        report.stage_outcomes.len(),
        report.dry_run,
        report.retry,
        report.healthy
    );
    let state_label = if report.healthy() { "done" } else { "partial" };
    let _ = registry.record_operation("release", &project_id, state_label, &detail);
    let output = release_report_output(&report, format)?;
    if report.healthy() {
        Ok(output)
    } else {
        match &output {
            Output::Human(text) => println!("{text}"),
            Output::Json(value) => {
                println!("{}", serde_json::to_string_pretty(value).unwrap());
            }
            Output::Raw(text) => print!("{text}"),
        }
        Err(ForgeError::ReleaseCheckFailed {
            reason: report.note.clone(),
        })
    }
}

fn release_report_output(report: &ReleaseReport, format: Format) -> Result<Output, ForgeError> {
    let json = serde_json::json!({"release": report});
    let human = render_release_report_human(report);
    Ok(as_output(format, human, json))
}

fn cmd_release_list(db_path: &Path, target: &str, format: Format) -> Result<Output, ForgeError> {
    let _ = db_path;
    let (project_dir, project_id) = resolve_release_target(target)?;
    let entries = list_releases(&project_dir, &project_id)?;
    release_list_output(&entries, &project_id, format)
}

fn release_list_output(
    entries: &[ReleaseListEntry],
    project_id: &str,
    format: Format,
) -> Result<Output, ForgeError> {
    let json = serde_json::json!({"releases": entries, "project_id": project_id});
    if entries.is_empty() {
        let human = format!("no releases for project `{project_id}`");
        return Ok(as_output(format, human, json));
    }
    let mut human = format!(
        "{:<48} {:<12} {:<14} {:<8} {}",
        "Release", "Project", "Version", "Stages", "Last Run"
    );
    for entry in entries {
        human.push_str(&format!(
            "\n{:<48} {:<12} {:<14} {:<8} {}",
            entry.release_id,
            truncate(&entry.project_id, 12),
            entry.version,
            entry.stage_count.to_string(),
            entry.last_run_at
        ));
    }
    Ok(as_output(format, human, json))
}

fn cmd_release_inspect(
    db_path: &Path,
    target: &str,
    release_id: &str,
    format: Format,
) -> Result<Output, ForgeError> {
    let _ = db_path;
    let (project_dir, project_id) = resolve_release_target(target)?;
    let state = match read_release(&project_dir, &project_id, release_id)? {
        Some(state) => state,
        None => {
            return Err(ForgeError::ReleaseInvalid {
                reason: format!(
                    "release `{release_id}` was not found under `.forge/release/{project_id}/`"
                ),
            });
        }
    };
    release_state_output(&state, format)
}

fn release_state_output(state: &ReleaseState, format: Format) -> Result<Output, ForgeError> {
    let json = serde_json::json!({"release": state});
    let human = render_release_report_human_for_state(state);
    Ok(as_output(format, human, json))
}

fn render_release_report_human_for_state(state: &ReleaseState) -> String {
    let mut lines: Vec<String> = Vec::new();
    lines.push(format!("project: {}", state.identity.project_id));
    lines.push(format!("release_id: {}", state.identity.id));
    lines.push(format!("version: {}", state.identity.version));
    lines.push(format!(
        "source_revision: {}",
        state.identity.source_revision
    ));
    if let Some(changelog) = &state.changelog {
        lines.push(format!("changelog: {}", changelog.path));
        lines.push(format!("changelog_hash: {}", changelog.content_hash));
    }
    if !state.docs_locales.is_empty() {
        lines.push(format!("docs_locales: {}", state.docs_locales.join(", ")));
    }
    lines.push(format!("stages: {}", state.stages.join(", ")));
    lines.push(format!("last_run_at: {}", state.last_run_at));
    if !state.checks.is_empty() {
        lines.push("checks:".to_string());
        for check in &state.checks {
            lines.push(format!(
                "  - {} {} applicable={} revision={}: {}",
                check.kind, check.status, check.applicable, check.source_revision, check.detail
            ));
        }
    }
    if !state.stage_outcomes.is_empty() {
        lines.push("stage_outcomes:".to_string());
        for outcome in &state.stage_outcomes {
            lines.push(format!(
                "  - {} target=`{}` {}: {}",
                outcome.stage, outcome.target, outcome.status, outcome.note
            ));
            for line in &outcome.evidence {
                lines.push(format!("      evidence: {line}"));
            }
            for line in &outcome.recovery {
                lines.push(format!("      recovery: {line}"));
            }
        }
    }
    lines.join("\n")
}

pub(super) fn resolve_deploy_target(target: &str) -> Result<(PathBuf, String), ForgeError> {
    let candidate = Path::new(target);
    if candidate.is_dir() {
        let canonical = candidate
            .canonicalize()
            .map_err(|_| ForgeError::PathUnavailable {
                path: target.to_string(),
            })?;
        let (manifest, _) = forge::core::manifest::Manifest::load_from_dir(&canonical, None)?;
        return Ok((canonical, manifest.project.id));
    }
    Err(ForgeError::PathUnavailable {
        path: target.to_string(),
    })
}

pub(crate) fn cmd_deploy(
    db_path: &Path,
    command: &DeployCommands,
    format: Format,
) -> Result<Output, ForgeError> {
    match command {
        DeployCommands::Plan {
            target,
            target_name,
        } => {
            let (project_dir, project_id) = resolve_deploy_target(target)?;
            let (manifest, config) = forge::deploy::engine::load_config(&project_dir)?;
            let target_name = resolve_deploy_target_name(&config, target_name.as_deref())?;
            let request = DeployRequest {
                project_id: project_id.clone(),
                target: target_name,
                confirm: false,
                dry_run: true,
            };
            let plan =
                forge::deploy::engine::prepare_deploy(&project_dir, &manifest, &config, &request)?;
            let registry = open_registry(db_path)?;
            let detail = format!(
                "deploy plan `{}` target=`{}` adapter=`{}` ready={}",
                plan.identity.id, plan.target.name, plan.target.kind, plan.ready
            );
            let state_label = if plan.healthy() { "done" } else { "blocked" };
            let _ = registry.record_operation("deploy", &project_id, state_label, &detail);
            deploy_plan_output(&plan, format)
        }
        DeployCommands::Apply {
            target,
            target_name,
            confirm,
            dry_run,
        } => cmd_deploy_apply(
            db_path,
            target,
            target_name.as_deref(),
            *confirm,
            *dry_run,
            format,
        ),
        DeployCommands::Observe {
            target,
            target_name,
            confirm,
        } => cmd_deploy_observe(db_path, target, target_name.as_deref(), *confirm, format),
        DeployCommands::List { target } => {
            let (project_dir, project_id) = resolve_deploy_target(target)?;
            let entries = forge::deploy::engine::list_deploys(&project_dir, &project_id)?;
            deploy_list_output(&entries, &project_id, format)
        }
        DeployCommands::Inspect { deploy_id, target } => {
            let (project_dir, project_id) = resolve_deploy_target(target)?;
            let state =
                match forge::deploy::engine::read_deploy(&project_dir, &project_id, deploy_id)? {
                    Some(state) => state,
                    None => {
                        return Err(ForgeError::DeployInvalid {
                            reason: format!(
                            "deploy `{deploy_id}` was not found under `.forge/deploy/{project_id}/`"
                        ),
                        });
                    }
                };
            deploy_state_output(&state, format)
        }
        DeployCommands::Status {
            project,
            queue,
            limit,
            watch,
            interval_secs,
            deadline_secs,
        } => cmd_deploy_status(
            db_path,
            project.as_deref(),
            queue.as_deref(),
            *limit,
            *watch,
            *interval_secs,
            *deadline_secs,
            format,
        ),
    }
}

fn cmd_deploy_status(
    db_path: &Path,
    project: Option<&str>,
    queue: Option<&str>,
    limit: usize,
    watch: bool,
    interval_secs: u64,
    deadline_secs: u64,
    format: Format,
) -> Result<Output, ForgeError> {
    if watch && queue.is_none() {
        return Err(ForgeError::PublishInvalid {
            reason: "deploy status --watch requires --queue <id>".to_string(),
        });
    }
    if interval_secs == 0 || interval_secs > 60 {
        return Err(ForgeError::PublishInvalid {
            reason: "deploy status --interval-secs must be in 1..=60".to_string(),
        });
    }
    if deadline_secs == 0 || deadline_secs > 86400 {
        return Err(ForgeError::PublishInvalid {
            reason: "deploy status --deadline-secs must be in 1..=86400".to_string(),
        });
    }
    let registry = open_registry(db_path)?;
    let limit = limit.clamp(1, 500);
    let (entries, read_only_label) = match (project, queue) {
        (Some(_), Some(_)) => {
            return Err(ForgeError::PublishInvalid {
                reason: "deploy status accepts --project or --queue, not both".to_string(),
            });
        }
        (Some(project_id), None) => {
            let rows = registry.operations_for_project(project_id, limit)?;
            (filter_publish_deploy(rows), format!("project={project_id}"))
        }
        (None, Some(queue_id)) => {
            forge::publish::providers::validate_queue_id(queue_id).map_err(|_| {
                ForgeError::PublishInvalid {
                    reason: format!("queue id `{queue_id}` is not 1..=128 ASCII"),
                }
            })?;
            let rows = registry.operations_for_queue(queue_id, limit)?;
            (filter_publish_deploy(rows), format!("queue={queue_id}"))
        }
        (None, None) => {
            let rows = registry.recent_operations(limit)?;
            (filter_publish_deploy(rows), "scope=recent".to_string())
        }
    };
    let latest = entries.first();
    let aggregate_state = latest.map(|entry| entry.state.as_str()).unwrap_or("empty");
    let value = serde_json::json!({
        "contract": "forge-deploy-status/0.2.0",
        "project": project,
        "queue": queue,
        "scope": read_only_label,
        "state": aggregate_state,
        "entries": entries,
        "read_only": true,
    });
    let human = render_deploy_status_human(&entries, project, queue, &read_only_label);
    if watch {
        return cmd_deploy_status_watch(
            db_path,
            queue.unwrap(),
            interval_secs,
            deadline_secs,
            format,
        );
    }
    Ok(as_output(format, human, value))
}

fn filter_publish_deploy(
    entries: Vec<forge::registry::OperationEntry>,
) -> Vec<forge::registry::OperationEntry> {
    // Includes every variant of the publish surface — the manual
    // `forge publish` path (`publish`), the legacy stage publish
    // (`deploy`), and the GitHub-push reserve id (`publish.github`)
    // — so the additive `revision` / `build_status` / `run_status` /
    // `container_identity` evidence stays visible through one
    // `forge deploy status` query regardless of which path produced
    // the row.
    entries
        .into_iter()
        .filter(|entry| {
            entry.kind == "publish" || entry.kind == "deploy" || entry.kind == "publish.github"
        })
        .collect()
}

pub(super) fn render_deploy_status_human(
    entries: &[forge::registry::OperationEntry],
    project: Option<&str>,
    queue: Option<&str>,
    scope: &str,
) -> String {
    if entries.is_empty() {
        return match (project, queue) {
            (Some(project_id), _) => {
                format!("deploy status: no publish/deploy history for {project_id}")
            }
            (None, Some(queue_id)) => {
                format!("deploy status: no history for queue `{queue_id}`")
            }
            (None, None) => "deploy status: no publish/deploy history".to_string(),
        };
    }
    let mut lines = vec![format!("deploy status: {scope}")];
    for entry in entries {
        let queue_label = entry
            .queue_id
            .as_deref()
            .map(|q| format!(" queue={q}"))
            .unwrap_or_default();
        let revision_label = entry
            .revision
            .as_deref()
            .map(|r| format!(" revision={r}"))
            .unwrap_or_default();
        let build_label = entry
            .build_status
            .as_deref()
            .map(|b| format!(" build={b}"))
            .unwrap_or_default();
        let run_label = entry
            .run_status
            .as_deref()
            .map(|r| format!(" run={r}"))
            .unwrap_or_default();
        let identity_label = entry
            .container_identity
            .as_deref()
            .map(|c| format!(" container={c}"))
            .unwrap_or_default();
        lines.push(format!(
            "  #{} {} {} {}{}{}{}{}{}",
            entry.op_id,
            entry.project_id,
            entry.kind,
            entry.state,
            queue_label,
            revision_label,
            build_label,
            run_label,
            identity_label,
        ));
    }
    lines.join("\n")
}
