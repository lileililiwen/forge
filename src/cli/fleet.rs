//! Fleet publish orchestration (`fleet`).
//!
//! Inventory-driven per-project fleet publish over the publish pipeline.
//! Bodies moved verbatim from the split of `src/main.rs`.

use forge::core::ForgeError;
use forge::publish::{
    jenkins::JenkinsAdapter, remote_compose::RemoteComposeAdapter,
    render_report_human as render_publish_report_human, request_from as build_publish_request,
    run_publish, PublishAction, SubprocessTransport, PUBLISH_CONTRACT_VERSION,
};
use forge::registry::PublishPhaseEvidence;
use std::path::Path;

use super::commands::FleetEntryOutcome;
use super::fleet_exec::{
    entry_index, legacy_inventory_snapshot, load_inventory_snapshot, replay_publish_journal,
    run_fleet_concurrent, run_fleet_entry_phases, use_legacy_publish_adapter,
};
use super::functions_13::render_output;
use super::projects::{as_output, open_registry};
use super::publish::{cmd_publish_provider, validate_fleet_jobs};
use crate::{Format, Output};

pub(super) fn cmd_publish_fleet(
    db_path: &Path,
    inventory: Option<std::path::PathBuf>,
    registry_path: Option<std::path::PathBuf>,
    workspace_root: Option<std::path::PathBuf>,
    dry_run: bool,
    lifecycle: String,
    fail_fast: bool,
    jobs: usize,
    provider: Option<String>,
    format: Format,
) -> Result<Output, ForgeError> {
    use forge::publish::fleet::{default_registry_path, default_workspace_root};
    use forge::publish::inventory::{
        classify, resolve_source, InventoryClassification, DEFAULT_DOMAIN,
    };

    let jobs = validate_fleet_jobs(jobs)?;
    // An explicit `--fleet-registry <path>` always routes through
    // the legacy `projects.json` compatibility adapter — never the
    // inventory branch — so a valid `projects.json` classifies its
    // entries instead of refusing a contract error
    // (`fleet-live-rollout` D3). `--inventory` (flag or env) keeps
    // precedence; the default registry path applies only when no
    // explicit source exists at all.
    let (snapshot, source_label) = if let Some(path) = registry_path.clone() {
        let workspace_root = workspace_root
            .clone()
            .unwrap_or_else(default_workspace_root);
        let snapshot = legacy_inventory_snapshot(&path, &workspace_root, &lifecycle)?;
        (snapshot, format!("registry:{}", path.display()))
    } else if let Some(path) = inventory
        .as_ref()
        .map(|p| p.to_path_buf())
        .or_else(|| resolve_source(None))
    {
        let snapshot = load_inventory_snapshot(&path)?;
        let label = format!("inventory:{}", path.display());
        (snapshot, label)
    } else {
        // No explicit source — synthesise an inventory from the
        // legacy workspace-governance registry so the seven-project
        // handoff keeps working without configuration.
        let workspace_root = workspace_root.unwrap_or_else(default_workspace_root);
        let registry_path = default_registry_path(Some(&workspace_root));
        let snapshot = legacy_inventory_snapshot(&registry_path, &workspace_root, &lifecycle)?;
        (snapshot, format!("registry:{}", registry_path.display()))
    };

    let fleet_report = classify(&snapshot, DEFAULT_DOMAIN);
    let eligible: Vec<forge::publish::inventory::InventoryFleetEntry> = fleet_report
        .entries
        .iter()
        .filter(|entry| entry.classification == InventoryClassification::ComposeReady)
        .cloned()
        .collect();
    let skipped: Vec<&forge::publish::inventory::InventoryFleetEntry> = fleet_report
        .entries
        .iter()
        .filter(|entry| entry.classification != InventoryClassification::ComposeReady)
        .collect();

    if eligible.is_empty() {
        let declared = snapshot.declared_count();
        let summary = serde_json::json!({
            "contract": forge::publish::inventory::INVENTORY_CONTRACT_VERSION,
            "inventory_source": source_label,
            "provider": snapshot.provider,
            "generated_at": snapshot.generated_at,
            "declared": declared,
            "compose_ready": 0,
            "skipped": skipped.iter().map(|entry| {
                serde_json::json!({
                    "id": entry.id,
                    "classification": entry.classification.as_str(),
                    "reason": entry.reason,
                })
            }).collect::<Vec<_>>(),
        });
        let human = format!(
            "fleet publish: 0/{declared} compose_ready in {source_label}; nothing to publish"
        );
        render_output(as_output(format, human, summary));
        return Err(ForgeError::PublishInvalid {
            reason: format!(
                "no compose_ready entries in inventory source `{source_label}`; \
                 check that every project declares a Compose file at a resolvable source path"
            ),
        });
    }

    // One fleet run, one stable queue id. The id is part of every
    // queued/running/terminal journal row this loop writes so
    // `forge deploy status --queue <id> --watch` can poll the
    // single fleet invocation end to end. ASCII alphanumeric plus
    // `-`/`_` characters only — `validate_queue_id` re-checks before
    // the loop starts so a clock skew never escapes into the
    // journal.
    let queue_id = format!(
        "fleet-{}-{:08x}",
        chrono::Utc::now().format("%Y%m%dT%H%M%SZ"),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| (d.as_nanos() as u64) & 0xffff_ffff)
            .unwrap_or(0)
    );
    forge::publish::providers::validate_queue_id(&queue_id).map_err(|_| {
        ForgeError::PublishInvalid {
            reason: format!("generated queue id `{queue_id}` failed shape validation"),
        }
    })?;
    let mut queue_op_ids: Vec<(String, i64)> = Vec::with_capacity(eligible.len());

    let mut summaries: Vec<serde_json::Value> = Vec::new();
    let mut first_failure: Option<ForgeError> = None;
    let mut success_count = 0usize;
    let mut failure_count = 0usize;

    // The external `--provider` branch always stays sequential: it
    // shells through `cmd_publish_provider` (which re-opens the
    // registry per project), so no thread owns it
    // (`fleet-live-rollout` D1).
    let use_parallel = provider.is_none() && jobs > 1 && !dry_run;
    if use_parallel {
        // Concurrent fleet publish (`fleet-live-rollout` D1, phased
        // refinement): worker threads own phase execution
        // (per-thread adapter + transport, both cheap to construct
        // and holding `RefCell` state that is never shared). The
        // main thread replays every journal row serially from the
        // returned reports and renders all output — row shapes and
        // queue id semantics are unchanged.
        //
        // Phases exist because Prepare renders *global* target
        // state (the shared `port-registry.json`, Caddyfile and
        // index) from whatever the target currently holds: running
        // N Prepares concurrently is last-writer-wins on shared
        // files and each render misses the sibling projects. So:
        // Phase A runs Sync in parallel (per-project remote dirs),
        // Phase B runs Db + Prepare serially in roster order — Db is
        // the shared-Postgres ensure whose command is identical for
        // every project, and recreating the single
        // `production-postgres` container concurrently is a race
        // (live evidence 2026-09-30: parallel Db produced `removal
        // ... already in progress` / `Conflict ... name already in
        // use`); Prepare runs serially so the registry converges
        // (each render sees every prior project). Phase C runs
        // Deploy in parallel (per-project `forge-<id>` compose
        // projects). Per-project order Sync -> Db -> Prepare ->
        // Deploy is preserved; with fail-fast, later phases never
        // start a project whose earlier phase failed, and Phase B/C
        // stop scheduling new projects after the first failure they
        // observe.
        let legacy_lane = use_legacy_publish_adapter();
        let db_path_owned = db_path.to_path_buf();
        let phase_a = move |entry: forge::publish::inventory::InventoryFleetEntry| {
            run_fleet_entry_phases(
                entry,
                &db_path_owned,
                legacy_lane,
                dry_run,
                &[PublishAction::Sync],
            )
        };
        let phase_a_outcomes = run_fleet_concurrent(&eligible, jobs, fail_fast, phase_a);
        // Per-project accumulated state across phases.
        struct FleetProgress {
            reports: Vec<forge::publish::PublishReport>,
            failed: bool,
            fail_error: Option<ForgeError>,
            skipped: bool,
        }
        let mut progress: Vec<FleetProgress> = phase_a_outcomes
            .into_iter()
            .map(|outcome| match outcome {
                FleetEntryOutcome::Phased { reports, error } => FleetProgress {
                    reports,
                    failed: error.is_some(),
                    fail_error: error,
                    skipped: false,
                },
                FleetEntryOutcome::Failed { error, .. } => FleetProgress {
                    reports: Vec::new(),
                    failed: true,
                    fail_error: Some(error),
                    skipped: false,
                },
                FleetEntryOutcome::Published { .. } => FleetProgress {
                    reports: Vec::new(),
                    failed: true,
                    fail_error: Some(ForgeError::PublishInvalid {
                        reason: "internal fleet phase error: unexpected outcome unit".to_string(),
                    }),
                    skipped: false,
                },
            })
            .collect();
        let core_registry = open_registry(db_path)?;
        // Replay Phase A journal rows serially (per-stage + summary
        // rows exactly as `run_publish` writes them) and render.
        for (entry, prog) in eligible.iter().zip(progress.iter()) {
            for report in &prog.reports {
                replay_publish_journal(&core_registry, report);
                let human = render_publish_report_human(report);
                let mut value =
                    serde_json::to_value(report).map_err(|err| ForgeError::PublishInvalid {
                        reason: format!("cannot encode report for {}: {err}", entry.id),
                    })?;
                if let Some(obj) = value.as_object_mut() {
                    obj.insert(
                        "contract".to_string(),
                        serde_json::json!(PUBLISH_CONTRACT_VERSION),
                    );
                    if let Some(subdomain) = entry.subdomain.as_deref() {
                        obj.insert(
                            "inventory_subdomain".to_string(),
                            serde_json::json!(subdomain),
                        );
                    }
                }
                render_output(as_output(format, human, value));
            }
        }
        // Phase B: serial Db + Prepare in roster order. The shared
        // adapter/transport pair is main-thread owned, exactly like
        // the sequential path.
        let jenkins_adapter = JenkinsAdapter::from_env();
        let remote_adapter = RemoteComposeAdapter::from_env();
        let adapter: &dyn forge::publish::PublishAdapter = if legacy_lane {
            &jenkins_adapter
        } else {
            &remote_adapter
        };
        let transport = SubprocessTransport::default();
        let mut phase_b_stopped = false;
        for (entry, prog) in eligible.iter().zip(progress.iter_mut()) {
            if prog.failed || (fail_fast && phase_b_stopped) {
                if !prog.failed {
                    prog.skipped = true;
                }
                continue;
            }
            let source_path = entry
                .source_path
                .as_deref()
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|| std::path::PathBuf::from("."));
            // Db (shared-Postgres ensure) then Prepare, both serial
            // and journaled through the main-thread registry. See
            // the phase comment above for why Db cannot run in the
            // parallel phase.
            for action in [PublishAction::Db, PublishAction::Prepare] {
                let request =
                    build_publish_request(entry.id.clone(), source_path.clone(), action, dry_run);
                match run_publish(&request, adapter, &transport, Some(&core_registry)) {
                    Ok(report) => {
                        let human = render_publish_report_human(&report);
                        let mut value = serde_json::to_value(&report).map_err(|err| {
                            ForgeError::PublishInvalid {
                                reason: format!("cannot encode report for {}: {err}", entry.id),
                            }
                        })?;
                        if let Some(obj) = value.as_object_mut() {
                            obj.insert(
                                "contract".to_string(),
                                serde_json::json!(PUBLISH_CONTRACT_VERSION),
                            );
                            if let Some(subdomain) = entry.subdomain.as_deref() {
                                obj.insert(
                                    "inventory_subdomain".to_string(),
                                    serde_json::json!(subdomain),
                                );
                            }
                        }
                        render_output(as_output(format, human, value));
                        let healthy = report.healthy;
                        prog.reports.push(report);
                        if !healthy {
                            prog.failed = true;
                            prog.fail_error = Some(ForgeError::PublishDeployFailed {
                                reason: format!("fleet prepare failed at `{}`", entry.id),
                            });
                            if fail_fast {
                                phase_b_stopped = true;
                            }
                            break;
                        }
                    }
                    Err(err) => {
                        prog.failed = true;
                        prog.fail_error = Some(err);
                        if fail_fast {
                            phase_b_stopped = true;
                        }
                        break;
                    }
                }
            }
        }
        // Phase C: parallel Deploy for projects whose A+B succeeded.
        let deployable: Vec<(usize, forge::publish::inventory::InventoryFleetEntry)> = eligible
            .iter()
            .enumerate()
            .filter(|(_, entry)| {
                progress
                    .get(entry_index(eligible.as_slice(), &entry.id))
                    .map(|p| !p.failed && !p.skipped)
                    .unwrap_or(false)
            })
            .map(|(i, e)| (i, e.clone()))
            .collect();
        let db_path_owned_c = db_path.to_path_buf();
        let phase_c = move |entry: forge::publish::inventory::InventoryFleetEntry| {
            run_fleet_entry_phases(
                entry,
                &db_path_owned_c,
                legacy_lane,
                dry_run,
                &[PublishAction::Deploy],
            )
        };
        let deploy_entries: Vec<forge::publish::inventory::InventoryFleetEntry> =
            deployable.iter().map(|(_, e)| e.clone()).collect();
        let phase_c_outcomes = run_fleet_concurrent(&deploy_entries, jobs, fail_fast, phase_c);
        for ((index, _), outcome) in deployable.iter().zip(phase_c_outcomes.into_iter()) {
            let prog = &mut progress[*index];
            match outcome {
                FleetEntryOutcome::Phased { reports, error } => {
                    for report in &reports {
                        replay_publish_journal(&core_registry, report);
                        let human = render_publish_report_human(report);
                        let entry = &eligible[*index];
                        let mut value = serde_json::to_value(report).map_err(|err| {
                            ForgeError::PublishInvalid {
                                reason: format!("cannot encode report for {}: {err}", entry.id),
                            }
                        })?;
                        if let Some(obj) = value.as_object_mut() {
                            obj.insert(
                                "contract".to_string(),
                                serde_json::json!(PUBLISH_CONTRACT_VERSION),
                            );
                            if let Some(subdomain) = entry.subdomain.as_deref() {
                                obj.insert(
                                    "inventory_subdomain".to_string(),
                                    serde_json::json!(subdomain),
                                );
                            }
                        }
                        render_output(as_output(format, human, value));
                    }
                    prog.reports.extend(reports);
                    if let Some(err) = error {
                        prog.failed = true;
                        prog.fail_error = Some(err);
                    }
                }
                FleetEntryOutcome::Failed { error, .. } => {
                    prog.failed = true;
                    prog.fail_error = Some(error);
                }
                FleetEntryOutcome::Published { .. } => {
                    prog.failed = true;
                    prog.fail_error = Some(ForgeError::PublishInvalid {
                        reason: "internal fleet phase error: unexpected outcome unit".to_string(),
                    });
                }
            }
        }
        // Fleet summary: one queue-tagged terminal row per project,
        // serially, in roster order — identical shape to the
        // sequential path.
        for (entry, prog) in eligible.iter().zip(progress.iter_mut()) {
            let project_id = entry.id.clone();
            if prog.skipped {
                failure_count += 1;
                let detail = format!(
                    "fleet publish for `{project_id}` skipped: fail-fast stopped scheduling"
                );
                summaries.push(serde_json::json!({
                    "project": project_id,
                    "healthy": false,
                    "error": "fail-fast-skipped",
                }));
                if let Ok(op_id) = core_registry.record_queue_operation(
                    "publish",
                    &project_id,
                    &queue_id,
                    "failed",
                    Some(&detail),
                ) {
                    queue_op_ids.push((project_id.clone(), op_id));
                }
                if first_failure.is_none() {
                    first_failure = Some(ForgeError::PublishInvalid { reason: detail });
                }
                continue;
            }
            let healthy = !prog.failed && prog.reports.iter().all(|report| report.healthy);
            let stage_count: usize = prog.reports.iter().map(|r| r.stages.len()).sum();
            let subdomain = prog
                .reports
                .iter()
                .filter_map(|r| r.subdomain.clone())
                .next();
            summaries.push(serde_json::json!({
                "project": project_id,
                "healthy": healthy,
                "subdomain": subdomain,
                "stages": stage_count,
            }));
            let terminal_state = if healthy { "done" } else { "failed" };
            let journal = if prog.reports.is_empty() {
                format!(
                    "fleet publish failed before any stage: {}",
                    prog.fail_error
                        .as_ref()
                        .map(|e| e.to_string())
                        .unwrap_or_else(|| "unknown error".to_string())
                )
            } else {
                format!("fleet stages={stage_count} healthy={healthy}")
            };
            if let Ok(op_id) = core_registry.record_queue_operation(
                "publish",
                &project_id,
                &queue_id,
                terminal_state,
                Some(&journal),
            ) {
                queue_op_ids.push((project_id.clone(), op_id));
            }
            if healthy {
                success_count += 1;
            } else {
                failure_count += 1;
                if fail_fast && first_failure.is_none() {
                    first_failure = prog.fail_error.take().or_else(|| {
                        Some(ForgeError::PublishDeployFailed {
                            reason: format!("fleet publish failed at `{project_id}`"),
                        })
                    });
                }
            }
        }
    }

    if !use_parallel {
        let jenkins_adapter = JenkinsAdapter::from_env();
        let remote_adapter = RemoteComposeAdapter::from_env();
        let adapter: &dyn forge::publish::PublishAdapter = if use_legacy_publish_adapter() {
            &jenkins_adapter
        } else {
            &remote_adapter
        };
        let core_registry = open_registry(db_path)?;
        let transport = SubprocessTransport::default();

        for entry in &eligible {
            let project_id = entry.id.clone();
            let source_path = entry
                .source_path
                .as_deref()
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|| std::path::PathBuf::from("."));
            // The provider transport needs an on-disk folder to compose.
            // Entries declared as remote-only git sources fail closed
            // here rather than fabricating a synthetic workdir; the
            // classification step above already surfaced the gap so the
            // operator can remediate.
            if !source_path.is_dir() {
                failure_count += 1;
                let detail = format!(
                    "fleet publish: project `{project_id}` source `{}` is not a directory",
                    source_path.display()
                );
                summaries.push(serde_json::json!({
                    "project": project_id,
                    "healthy": false,
                    "error": "source_unavailable_at_invoke",
                }));
                if let Ok(op_id) = core_registry.record_queue_operation(
                    "publish",
                    &project_id,
                    &queue_id,
                    "failed",
                    Some(&detail),
                ) {
                    queue_op_ids.push((project_id.clone(), op_id));
                }
                if fail_fast && first_failure.is_none() {
                    first_failure = Some(ForgeError::PublishInvalid { reason: detail });
                    break;
                }
                continue;
            }

            // Optional external `--provider` branch. Honours the same
            // phase-evidence contract as the manual path so the journal
            // rows are byte-equivalent regardless of provider selection.
            if let Some(provider_id) = provider.as_deref() {
                let outcome = cmd_publish_provider(
                    db_path,
                    None,
                    Some(&source_path),
                    Some(provider_id),
                    None,
                    dry_run,
                    Format::Json,
                );
                match outcome {
                    Ok(Output::Json(value)) => {
                        let healthy = value
                            .get("health")
                            .and_then(|v| v.as_str())
                            .map(|v| v == "healthy")
                            .unwrap_or(dry_run);
                        render_output(as_output(
                            format,
                            format!("publish {project_id}: provider={provider_id}"),
                            value.clone(),
                        ));
                        summaries.push(serde_json::json!({
                            "project": project_id,
                            "provider": provider_id,
                            "healthy": healthy,
                            "status": value.get("status"),
                            "build_status": value.get("build_status"),
                            "run_status": value.get("run_status"),
                            "container_identity": value.get("container_identity"),
                        }));
                        let terminal_state = if healthy { "done" } else { "failed" };
                        let phase_revision = value.get("revision").and_then(|v| v.as_str());
                        let build_status = value.get("build_status").and_then(|v| v.as_str());
                        let run_status = value.get("run_status").and_then(|v| v.as_str());
                        let container_identity = value
                            .get("container_identity")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string())
                            .or_else(|| {
                                phase_revision.map(|rev| {
                                    forge::publish::providers::compose_project_name(
                                        &project_id,
                                        rev,
                                    )
                                })
                            });
                        let mut phase = PublishPhaseEvidence::new();
                        if let Some(rev) = phase_revision {
                            phase = phase.revision(rev);
                        }
                        if let Some(b) = build_status {
                            phase = phase.build_status(b);
                        }
                        if let Some(r) = run_status {
                            phase = phase.run_status(r);
                        }
                        if let Some(c) = container_identity.as_deref() {
                            phase = phase.container_identity(c);
                        }
                        if let Ok(op_id) = core_registry.record_queue_publish_phase(
                            &project_id,
                            &queue_id,
                            terminal_state,
                            phase,
                            Some(&format!("fleet provider={provider_id}")),
                        ) {
                            queue_op_ids.push((project_id.clone(), op_id));
                        }
                        if healthy {
                            success_count += 1;
                        } else {
                            failure_count += 1;
                            if fail_fast && first_failure.is_none() {
                                first_failure = Some(ForgeError::PublishDeployFailed {
                                    reason: format!(
                                        "fleet provider publish failed at `{project_id}`"
                                    ),
                                });
                                break;
                            }
                        }
                    }
                    Ok(Output::Human(text)) | Ok(Output::Raw(text)) => {
                        println!("{text}");
                        success_count += 1;
                        if let Ok(op_id) = core_registry.record_queue_operation(
                            "publish",
                            &project_id,
                            &queue_id,
                            "done",
                            Some(&format!("fleet provider={provider_id}")),
                        ) {
                            queue_op_ids.push((project_id.clone(), op_id));
                        }
                    }
                    Err(error) => {
                        failure_count += 1;
                        let detail = format!("fleet provider={provider_id} failed: {error}");
                        render_output(as_output(
                            format,
                            format!("publish {project_id} failed: {error}"),
                            serde_json::json!({"project": project_id, "error": error.to_string()}),
                        ));
                        if let Ok(op_id) = core_registry.record_queue_operation(
                            "publish",
                            &project_id,
                            &queue_id,
                            "failed",
                            Some(&detail),
                        ) {
                            queue_op_ids.push((project_id.clone(), op_id));
                        }
                        if fail_fast && first_failure.is_none() {
                            first_failure = Some(error);
                            break;
                        }
                    }
                }
                continue;
            }

            let request =
                build_publish_request(project_id.clone(), source_path, PublishAction::All, dry_run);
            let outcome = run_publish(&request, adapter, &transport, Some(&core_registry));
            match outcome {
                Ok(report) => {
                    let human = render_publish_report_human(&report);
                    let mut value = serde_json::to_value(&report).map_err(|err| {
                        ForgeError::PublishInvalid {
                            reason: format!("cannot encode report for {project_id}: {err}"),
                        }
                    })?;
                    if let Some(obj) = value.as_object_mut() {
                        obj.insert(
                            "contract".to_string(),
                            serde_json::json!(PUBLISH_CONTRACT_VERSION),
                        );
                        if let Some(subdomain) = entry.subdomain.as_deref() {
                            obj.insert(
                                "inventory_subdomain".to_string(),
                                serde_json::json!(subdomain),
                            );
                        }
                    }
                    render_output(as_output(format, human.clone(), value.clone()));
                    let healthy = report.healthy;
                    summaries.push(serde_json::json!({
                        "project": project_id,
                        "healthy": healthy,
                        "subdomain": report.subdomain,
                        "stages": report.stages.len(),
                    }));
                    let terminal_state = if healthy { "done" } else { "failed" };
                    if let Ok(op_id) = core_registry.record_queue_operation(
                        "publish",
                        &project_id,
                        &queue_id,
                        terminal_state,
                        Some(&format!(
                            "fleet stages={} healthy={}",
                            report.stages.len(),
                            healthy
                        )),
                    ) {
                        queue_op_ids.push((project_id.clone(), op_id));
                    }
                    if healthy {
                        success_count += 1;
                    } else {
                        failure_count += 1;
                        if fail_fast && first_failure.is_none() {
                            first_failure = Some(ForgeError::PublishDeployFailed {
                                reason: format!("fleet publish failed at `{project_id}`"),
                            });
                            break;
                        }
                    }
                }
                Err(err) => {
                    failure_count += 1;
                    let detail = format!("fleet publish failed: {err}");
                    let summary = serde_json::json!({
                        "project": project_id,
                        "healthy": false,
                        "error": err.to_string(),
                    });
                    summaries.push(summary);
                    if let Ok(op_id) = core_registry.record_queue_operation(
                        "publish",
                        &project_id,
                        &queue_id,
                        "failed",
                        Some(&detail),
                    ) {
                        queue_op_ids.push((project_id.clone(), op_id));
                    }
                    if fail_fast && first_failure.is_none() {
                        first_failure = Some(err);
                        break;
                    }
                }
            }
        }
    }

    let skipped_summary: Vec<serde_json::Value> = skipped
        .iter()
        .map(|entry| {
            serde_json::json!({
                "id": entry.id,
                "runtime": entry.runtime.as_str(),
                "classification": entry.classification.as_str(),
                "reason": entry.reason,
            })
        })
        .collect();
    let summary = serde_json::json!({
        "contract": forge::publish::inventory::INVENTORY_CONTRACT_VERSION,
        "inventory_source": source_label,
        "provider": snapshot.provider,
        "generated_at": snapshot.generated_at,
        "fleet": {
            "queue_id": queue_id,
            "declared": snapshot.declared_count(),
            "compose_ready": eligible.len(),
            "skipped": skipped.len(),
            "success": success_count,
            "failed": failure_count,
            "projects": summaries,
            "skipped_entries": skipped_summary,
        },
    });
    let human = format!(
        "fleet publish: {}/{} compose_ready succeeded, {} skipped ({} in {source_label})",
        success_count,
        eligible.len(),
        skipped.len(),
        snapshot.declared_count(),
    );

    let summary = {
        let mut summary = summary;
        if let Some(fleet) = summary.get_mut("fleet").and_then(|f| f.as_object_mut()) {
            fleet.insert("jobs".to_string(), serde_json::json!(jobs));
        }
        summary
    };
    render_output(as_output(format, human, summary.clone()));
    if let Some(err) = first_failure {
        Err(err)
    } else if failure_count > 0 {
        Err(ForgeError::PublishDeployFailed {
            reason: format!(
                "fleet publish finished with {failure_count} failure(s); see per-project reports"
            ),
        })
    } else {
        Ok(Output::Human(String::new()))
    }
}
