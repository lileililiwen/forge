//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use forge::core::ForgeError;
use forge::deploy::{DeployAdapterConfig, DeployRequest};
use forge::publish::{
    jenkins::JenkinsAdapter,
    providers::{
        invoke_provider, load_config as load_publish_provider_config, select_provider,
        ProviderOperation, PublishProviderRequest,
    },
    remote_compose::RemoteComposeAdapter,
    render_report_human as render_publish_report_human, request_from as build_publish_request,
    run_publish, PublishAction, SubprocessTransport, PUBLISH_CONTRACT_VERSION,
};
use forge::registry::PublishPhaseEvidence;
use std::path::{Path, PathBuf};

use super::commands::{PublishCommands, PublishProviderCommands};
use super::constants::FLEET_MAX_JOBS;
use super::fleet::cmd_publish_fleet;
use super::fleet_exec::{deploy_report_output, resolve_publish_target, use_legacy_publish_adapter};
use super::functions_13::render_output;
use super::projects::{as_output, open_registry};
use super::release::{render_deploy_status_human, resolve_deploy_target};
use crate::{Format, Output};

pub(super) fn cmd_deploy_status_watch(
    db_path: &Path,
    queue_id: &str,
    interval_secs: u64,
    deadline_secs: u64,
    format: Format,
) -> Result<Output, ForgeError> {
    let started = std::time::Instant::now();
    let deadline = std::time::Duration::from_secs(deadline_secs);
    let interval = std::time::Duration::from_secs(interval_secs);
    loop {
        let registry = open_registry(db_path)?;
        let entries = registry.operations_for_queue(queue_id, 1024)?;
        let entries: Vec<_> = entries
            .into_iter()
            .filter(|entry| entry.kind == "publish" || entry.kind == "deploy")
            .collect();
        let still_active = entries
            .iter()
            .any(|entry| entry.state == "pending" || entry.state == "running");
        let rendered_human = render_deploy_status_human(
            &entries,
            None,
            Some(queue_id),
            &format!("queue={queue_id}"),
        );
        let rendered_json = serde_json::json!({
            "contract": "forge-deploy-status/0.2.0",
            "queue": queue_id,
            "scope": format!("queue={queue_id}"),
            "state": if still_active { "active" } else { "terminal" },
            "entries": entries,
            "read_only": true,
        });
        render_output(as_output(format, rendered_human, rendered_json));
        if !still_active {
            let all_succeeded = entries
                .iter()
                .all(|entry| entry.state == "done" || entry.state == "succeeded");
            return Ok(if all_succeeded {
                as_output(format, String::new(), serde_json::json!({"watch":"done"}))
            } else {
                return Err(ForgeError::PublishDeployFailed {
                    reason: format!(
                        "watched queue `{queue_id}` ended with non-success terminal states"
                    ),
                });
            });
        }
        if started.elapsed() >= deadline {
            return Err(ForgeError::PublishInvalid {
                reason: format!(
                    "watch deadline {deadline_secs}s reached while queue `{queue_id}` was still active"
                ),
            });
        }
        std::thread::sleep(interval);
    }
}

pub(super) fn resolve_deploy_target_name(
    config: &forge::deploy::DeployConfig,
    requested: Option<&str>,
) -> Result<String, ForgeError> {
    if let Some(name) = requested {
        return Ok(config.target(name)?.name.clone());
    }
    Ok(config.default_target.clone())
}

pub(super) fn cmd_deploy_apply(
    db_path: &Path,
    target: &str,
    target_name: Option<&str>,
    confirm: bool,
    dry_run: bool,
    format: Format,
) -> Result<Output, ForgeError> {
    let (project_dir, project_id) = resolve_deploy_target(target)?;
    let (manifest, config) = forge::deploy::engine::load_config(&project_dir)?;
    let target_name = resolve_deploy_target_name(&config, target_name)?;
    let request = DeployRequest {
        project_id: project_id.clone(),
        target: target_name,
        confirm,
        dry_run,
    };
    let adapters = DeployAdapterConfig::from_env();
    let report =
        forge::deploy::engine::apply_deploy(&project_dir, &manifest, &config, &request, &adapters)?;
    let registry = open_registry(db_path)?;
    let detail = forge::deploy::engine::journal_report(&report, report.healthy);
    let state_label = if report.healthy() { "done" } else { "partial" };
    let _ = registry.record_operation("deploy", &project_id, state_label, &detail);
    let output = deploy_report_output(&report, format)?;
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
        Err(ForgeError::DeployHealthFailed {
            reason: report.note.clone(),
        })
    }
}

pub(super) fn cmd_deploy_observe(
    db_path: &Path,
    target: &str,
    target_name: Option<&str>,
    confirm: bool,
    format: Format,
) -> Result<Output, ForgeError> {
    let (project_dir, project_id) = resolve_deploy_target(target)?;
    let (manifest, config) = forge::deploy::engine::load_config(&project_dir)?;
    let target_name = resolve_deploy_target_name(&config, target_name)?;
    let request = DeployRequest {
        project_id: project_id.clone(),
        target: target_name,
        confirm,
        dry_run: false,
    };
    let adapters = DeployAdapterConfig::from_env();
    let report = forge::deploy::engine::observe_deploy(
        &project_dir,
        &manifest,
        &config,
        &request,
        &adapters,
    )?;
    let registry = open_registry(db_path)?;
    let detail = forge::deploy::engine::journal_report(&report, report.healthy);
    let state_label = if report.healthy() { "done" } else { "partial" };
    let _ = registry.record_operation("deploy", &project_id, state_label, &detail);
    let output = deploy_report_output(&report, format)?;
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
        Err(ForgeError::DeployHealthFailed {
            reason: report.note.clone(),
        })
    }
}

fn discover_cwd_publish_target(cwd: Option<&Path>) -> Result<(PathBuf, String), ForgeError> {
    let raw = if let Some(path) = cwd {
        path.to_path_buf()
    } else {
        std::env::current_dir().map_err(|error| ForgeError::PublishInvalid {
            reason: format!("cannot determine current directory: {error}"),
        })?
    };
    let dir = std::fs::canonicalize(&raw).map_err(|error| ForgeError::PublishInvalid {
        reason: format!("cannot resolve publish cwd {}: {error}", raw.display()),
    })?;
    let project_json = dir.join(".project.json");
    if project_json.is_file() {
        let content =
            std::fs::read_to_string(&project_json).map_err(|error| ForgeError::PublishInvalid {
                reason: format!("cannot read {}: {error}", project_json.display()),
            })?;
        let value: serde_json::Value =
            serde_json::from_str(&content).map_err(|error| ForgeError::PublishInvalid {
                reason: format!("malformed {}: {error}", project_json.display()),
            })?;
        let id = value
            .get("id")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .unwrap_or("");
        if id.is_empty() {
            return Err(ForgeError::PublishInvalid {
                reason: format!("{} has no usable `id`", project_json.display()),
            });
        }
        validate_cwd_project_id(id)?;
        return Ok((dir, id.to_string()));
    }
    let forge_yaml = dir.join("forge.yaml");
    if forge_yaml.is_file() {
        let content =
            std::fs::read_to_string(&forge_yaml).map_err(|error| ForgeError::PublishInvalid {
                reason: format!("cannot read {}: {error}", forge_yaml.display()),
            })?;
        let value: serde_yaml::Value =
            serde_yaml::from_str(&content).map_err(|error| ForgeError::PublishInvalid {
                reason: format!("malformed {}: {error}", forge_yaml.display()),
            })?;
        if let Some(id) = value
            .get("project")
            .and_then(|p| p.get("id"))
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            validate_cwd_project_id(id)?;
            return Ok((dir, id.to_string()));
        }
    }
    let basename = dir
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| ForgeError::PublishInvalid {
            reason: format!(
                "publish cwd `{}` has no usable project id; use --folder or add .project.json",
                dir.display()
            ),
        })?
        .to_string();
    validate_cwd_project_id(&basename).map_err(|_| ForgeError::PublishInvalid {
        reason: format!(
            "publish cwd `{}` has no usable project id `{basename}`; use --folder or add .project.json with a valid id",
            dir.display()
        ),
    })?;
    Ok((dir, basename))
}

fn validate_cwd_project_id(id: &str) -> Result<(), ForgeError> {
    if id.is_empty() {
        return Err(ForgeError::PublishInvalid {
            reason: "project id must not be empty".to_string(),
        });
    }
    if !id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(ForgeError::PublishInvalid {
            reason: format!(
                "project id `{id}` must be kebab/snake-case (letters, digits, dash, underscore)"
            ),
        });
    }
    Ok(())
}

pub(crate) fn cmd_publish(
    db_path: &Path,
    command: Option<&PublishCommands>,
    project: Option<&str>,
    folder: Option<&Path>,
    cwd: Option<&Path>,
    provider: Option<&str>,
    revision: Option<&str>,
    dry_run: bool,
    format: Format,
) -> Result<Output, ForgeError> {
    if project.is_some() || folder.is_some() {
        return cmd_publish_provider(
            db_path, project, folder, provider, revision, dry_run, format,
        );
    }
    if let Some(cmd) = command {
        match cmd {
            PublishCommands::Provider { command } => {
                return cmd_publish_provider_lifecycle(command, format)
            }
            PublishCommands::Fleet {
                fleet_registry,
                workspace_root,
                inventory,
                dry_run,
                lifecycle,
                fail_fast,
                jobs,
                provider,
            } => {
                return cmd_publish_fleet(
                    db_path,
                    inventory.clone(),
                    fleet_registry.clone(),
                    workspace_root.clone(),
                    *dry_run,
                    lifecycle.clone(),
                    *fail_fast,
                    *jobs,
                    provider.clone(),
                    format,
                )
            }
            _ => return cmd_publish_single(db_path, cmd, format),
        }
    }
    let (project_dir, project_id) = discover_cwd_publish_target(cwd)?;
    return publish_via_provider_dir(
        db_path,
        &project_dir,
        &project_id,
        provider,
        revision,
        dry_run,
        format,
    );
}

fn publish_via_provider_dir(
    db_path: &Path,
    project_dir: &Path,
    project_id: &str,
    provider: Option<&str>,
    revision: Option<&str>,
    dry_run: bool,
    format: Format,
) -> Result<Output, ForgeError> {
    let provider_id = provider
        .map(str::to_string)
        .or_else(|| std::env::var("FORGE_PUBLISH_PROVIDER").ok())
        .ok_or_else(|| ForgeError::PublishInvalid {
            reason: "publish requires --provider or FORGE_PUBLISH_PROVIDER".to_string(),
        })?;
    let config_path = std::env::var_os("FORGE_PUBLISH_PROVIDER_CONFIG")
        .map(PathBuf::from)
        .unwrap_or_else(|| project_dir.join(".forge/providers.yaml"));
    let config = load_publish_provider_config(&config_path)?;
    let entry = select_provider(&config, &provider_id)?;
    let revision = revision
        .map(str::to_string)
        .or_else(|| git_revision(project_dir))
        .unwrap_or_else(|| "unknown".to_string());
    forge::publish::providers::validate_revision(&revision).map_err(|error| {
        ForgeError::PublishInvalid {
            reason: format!(
                "publish requires a 40-character hex revision; got `{revision}` ({error})"
            ),
        }
    })?;
    let operation_id = format!(
        "publish-{project_id}-{}",
        &revision[..revision.len().min(12)]
    );
    let request = PublishProviderRequest {
        contract: forge::publish::providers::PUBLISH_PROVIDER_CONTRACT.to_string(),
        operation: ProviderOperation::Publish,
        provider: provider_id.clone(),
        project_id: project_id.to_string(),
        revision: revision.clone(),
        operation_id,
        folder: Some(project_dir.display().to_string()),
        dry_run,
        queue_id: None,
    };
    let response = if dry_run {
        serde_json::Value::from(serde_json::to_value(&request).map_err(|error| {
            ForgeError::PublishInvalid {
                reason: format!("cannot encode publish request: {error}"),
            }
        })?)
    } else {
        let response = invoke_provider(&entry, &request, project_dir)?;
        let registry = open_registry(db_path)?;
        let phase_revision = response
            .revision
            .clone()
            .unwrap_or_else(|| revision.clone());
        let container_identity = response.container_identity.clone().unwrap_or_else(|| {
            forge::publish::providers::compose_project_name(project_id, &phase_revision)
        });
        registry.record_publish_phase(
            project_id,
            &response.status,
            PublishPhaseEvidence::new()
                .revision(&phase_revision)
                .container_identity(&container_identity)
                .build_status_opt(response.build_status.as_deref())
                .run_status_opt(response.run_status.as_deref()),
            Some(&format!(
                "provider={} health={}",
                response.provider, response.health
            )),
        )?;
        serde_json::to_value(response).map_err(|error| ForgeError::PublishInvalid {
            reason: format!("cannot encode publish response: {error}"),
        })?
    };
    let human = if dry_run {
        format!("publish dry-run: provider={provider_id} project={project_id}")
    } else {
        format!("publish: provider={provider_id} project={project_id}")
    };
    Ok(as_output(format, human, response))
}

fn cmd_publish_provider_lifecycle(
    command: &PublishProviderCommands,
    format: Format,
) -> Result<Output, ForgeError> {
    let config_path = match command {
        PublishProviderCommands::List { config }
        | PublishProviderCommands::Inspect { config, .. }
        | PublishProviderCommands::Enable { config, .. }
        | PublishProviderCommands::Disable { config, .. } => config
            .clone()
            .unwrap_or_else(|| PathBuf::from(".forge/providers.yaml")),
    };
    let mut config = load_publish_provider_config(&config_path)?;
    match command {
        PublishProviderCommands::List { .. } => {
            let value =
                serde_json::to_value(&config).map_err(|error| ForgeError::PublishInvalid {
                    reason: format!("cannot encode provider list: {error}"),
                })?;
            let human = if config.providers.is_empty() {
                "No publish providers configured.".to_string()
            } else {
                config
                    .providers
                    .iter()
                    .map(|entry| {
                        format!("{}\t{}", entry.id, if entry.enabled { "on" } else { "off" })
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            };
            Ok(as_output(format, human, value))
        }
        PublishProviderCommands::Inspect { id, .. } => {
            let entry = config
                .providers
                .iter()
                .find(|entry| entry.id == *id)
                .ok_or_else(|| ForgeError::PublishInvalid {
                    reason: format!("publish provider `{id}` is not configured"),
                })?;
            let value =
                serde_json::to_value(entry).map_err(|error| ForgeError::PublishInvalid {
                    reason: format!("cannot encode provider inspection: {error}"),
                })?;
            Ok(as_output(
                format,
                format!(
                    "publish provider `{id}`: {} ({})",
                    if entry.enabled { "enabled" } else { "disabled" },
                    entry.command.display()
                ),
                value,
            ))
        }
        PublishProviderCommands::Enable { id, .. }
        | PublishProviderCommands::Disable { id, .. } => {
            let enabled = matches!(command, PublishProviderCommands::Enable { .. });
            let entry = config
                .providers
                .iter_mut()
                .find(|entry| entry.id == *id)
                .ok_or_else(|| ForgeError::PublishInvalid {
                    reason: format!("publish provider `{id}` is not configured"),
                })?;
            entry.enabled = enabled;
            let bytes =
                serde_yaml::to_string(&config).map_err(|error| ForgeError::PublishInvalid {
                    reason: format!("cannot encode provider config: {error}"),
                })?;
            std::fs::write(&config_path, bytes).map_err(|error| ForgeError::PublishInvalid {
                reason: format!(
                    "cannot write provider config {}: {error}",
                    config_path.display()
                ),
            })?;
            let value = serde_json::json!({"provider": id, "enabled": enabled});
            Ok(as_output(
                format,
                format!(
                    "publish provider `{id}` {}",
                    if enabled { "enabled" } else { "disabled" }
                ),
                value,
            ))
        }
    }
}

pub(super) fn cmd_publish_provider(
    db_path: &Path,
    project: Option<&str>,
    folder: Option<&Path>,
    provider: Option<&str>,
    revision: Option<&str>,
    dry_run: bool,
    format: Format,
) -> Result<Output, ForgeError> {
    let (project_dir, project_id) = if let Some(project) = project {
        resolve_publish_target(project)?
    } else if let Some(folder) = folder {
        let dir = std::fs::canonicalize(folder).map_err(|error| ForgeError::PublishInvalid {
            reason: format!(
                "cannot resolve publish folder {}: {error}",
                folder.display()
            ),
        })?;
        let id = dir
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| ForgeError::PublishInvalid {
                reason: format!(
                    "publish folder `{}` has no usable project id",
                    dir.display()
                ),
            })?
            .to_string();
        (dir, id)
    } else {
        return Err(ForgeError::PublishInvalid {
            reason: "publish requires --project or --folder".to_string(),
        });
    };
    let provider_id = provider
        .map(str::to_string)
        .or_else(|| std::env::var("FORGE_PUBLISH_PROVIDER").ok())
        .ok_or_else(|| ForgeError::PublishInvalid {
            reason: "publish requires --provider or FORGE_PUBLISH_PROVIDER".to_string(),
        })?;
    let config_path = std::env::var_os("FORGE_PUBLISH_PROVIDER_CONFIG")
        .map(PathBuf::from)
        .unwrap_or_else(|| project_dir.join(".forge/providers.yaml"));
    let config = load_publish_provider_config(&config_path)?;
    let entry = select_provider(&config, &provider_id)?;
    let revision = revision
        .map(str::to_string)
        .or_else(|| git_revision(&project_dir))
        .unwrap_or_else(|| "unknown".to_string());
    forge::publish::providers::validate_revision(&revision).map_err(|error| {
        ForgeError::PublishInvalid {
            reason: format!(
                "publish requires a 40-character hex revision; got `{revision}` ({error})"
            ),
        }
    })?;
    let operation_id = format!(
        "publish-{project_id}-{}",
        &revision[..revision.len().min(12)]
    );
    let request = PublishProviderRequest {
        contract: forge::publish::providers::PUBLISH_PROVIDER_CONTRACT.to_string(),
        operation: ProviderOperation::Publish,
        provider: provider_id.clone(),
        project_id: project_id.clone(),
        revision: revision.clone(),
        operation_id,
        folder: Some(project_dir.display().to_string()),
        dry_run,
        queue_id: None,
    };
    let response = if dry_run {
        serde_json::Value::from(serde_json::to_value(&request).map_err(|error| {
            ForgeError::PublishInvalid {
                reason: format!("cannot encode publish request: {error}"),
            }
        })?)
    } else {
        let response = invoke_provider(&entry, &request, &project_dir)?;
        let registry = open_registry(db_path)?;
        let phase_revision = response
            .revision
            .clone()
            .unwrap_or_else(|| revision.clone());
        let container_identity = response.container_identity.clone().unwrap_or_else(|| {
            forge::publish::providers::compose_project_name(&project_id, &phase_revision)
        });
        registry.record_publish_phase(
            &project_id,
            &response.status,
            PublishPhaseEvidence::new()
                .revision(&phase_revision)
                .container_identity(&container_identity)
                .build_status_opt(response.build_status.as_deref())
                .run_status_opt(response.run_status.as_deref()),
            Some(&format!(
                "provider={} health={}",
                response.provider, response.health
            )),
        )?;
        serde_json::to_value(response).map_err(|error| ForgeError::PublishInvalid {
            reason: format!("cannot encode publish response: {error}"),
        })?
    };
    let human = if dry_run {
        format!("publish dry-run: provider={provider_id} project={project_id}")
    } else {
        format!("publish: provider={provider_id} project={project_id}")
    };
    Ok(as_output(format, human, response))
}

fn git_revision(project_dir: &Path) -> Option<String> {
    std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(project_dir)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .filter(|revision| !revision.is_empty())
}

fn cmd_publish_single(
    db_path: &Path,
    command: &PublishCommands,
    format: Format,
) -> Result<Output, ForgeError> {
    let (project_dir, project_id, action, dry_run) = match command {
        PublishCommands::Provider { .. } => unreachable!("Provider handled by cmd_publish"),
        PublishCommands::Sync { project, dry_run } => {
            let (dir, id) = resolve_publish_target(project)?;
            (dir, id, PublishAction::Sync, *dry_run)
        }
        PublishCommands::Db { project, dry_run } => {
            let (dir, id) = resolve_publish_target(project)?;
            (dir, id, PublishAction::Db, *dry_run)
        }
        PublishCommands::Prepare { project, dry_run } => {
            let (dir, id) = resolve_publish_target(project)?;
            (dir, id, PublishAction::Prepare, *dry_run)
        }
        PublishCommands::Deploy { project, dry_run } => {
            let (dir, id) = resolve_publish_target(project)?;
            (dir, id, PublishAction::Deploy, *dry_run)
        }
        PublishCommands::All { project, dry_run } => {
            let (dir, id) = resolve_publish_target(project)?;
            (dir, id, PublishAction::All, *dry_run)
        }
        PublishCommands::Fleet { .. } => unreachable!("Fleet handled by cmd_publish_fleet"),
    };

    let request = build_publish_request(project_id.clone(), project_dir, action, dry_run);
    // Decoupled default with one-cycle legacy rollback (`decoupled-remote-publish`
    // task 2.6/3.5): `FORGE_PUBLISH_ADAPTER=jenkins` restores the Mac-script
    // lane; every other value (including unset) selects `remote-compose`.
    // No new CLI flags; both adapters stay compiled.
    let jenkins_adapter = JenkinsAdapter::from_env();
    let remote_adapter = RemoteComposeAdapter::from_env();
    let adapter: &dyn forge::publish::PublishAdapter = if use_legacy_publish_adapter() {
        &jenkins_adapter
    } else {
        &remote_adapter
    };
    let registry = open_registry(db_path)?;
    let transport = SubprocessTransport::default();
    let report = run_publish(&request, adapter, &transport, Some(&registry))?;

    let human = render_publish_report_human(&report);
    let mut value = match serde_json::to_value(&report) {
        Ok(v) => v,
        Err(err) => {
            return Err(ForgeError::PublishInvalid {
                reason: format!("cannot encode publish report: {err}"),
            })
        }
    };
    if let Some(obj) = value.as_object_mut() {
        obj.insert(
            "contract".to_string(),
            serde_json::json!(PUBLISH_CONTRACT_VERSION),
        );
    }
    if !report.healthy {
        // Surface the report so the operator sees which stage failed
        // even when the aggregate is unhealthy.
        render_output(as_output(format, human, value));
        return Err(ForgeError::PublishDeployFailed {
            reason: report.note.clone(),
        });
    }
    Ok(as_output(format, human, value))
}

/// Validate the `--jobs` flag: `1..=FLEET_MAX_JOBS`. Refuses
/// out-of-range values with a typed error before anything runs.
pub fn validate_fleet_jobs(jobs: usize) -> Result<usize, ForgeError> {
    if !(1..=FLEET_MAX_JOBS).contains(&jobs) {
        return Err(ForgeError::PublishInvalid {
            reason: format!("fleet --jobs {jobs} is out of bounds; expected 1..={FLEET_MAX_JOBS}"),
        });
    }
    Ok(jobs)
}
