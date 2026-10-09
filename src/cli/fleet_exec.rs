//! Fleet execution helpers (`fleet_exec`).
//!
//! Fleet concurrency, inventory snapshots, deploy rendering and registry-fleet
//! observation handlers. Bodies moved verbatim from the split of `src/main.rs`.

use forge::core::ForgeError;
use forge::registry::{default_registry_path, Registry};
use std::path::Path;

use super::commands::FleetEntryOutcome;
use super::constants::FLEET_MAX_JOBS;
use super::feature::truncate;
use super::projects::{as_output, open_registry};
use crate::{Format, Output};

use forge::fleet::{
    self, health_json, inspect_entry, observe, render_entry_human,
    render_report_human as render_fleet_report_human, render_status_human, resolve_registry_path,
    DEFAULT_MAX_AGE_SECONDS, FLEET_CONTRACT_VERSION,
};
use forge::publish::{
    jenkins::JenkinsAdapter, remote_compose::RemoteComposeAdapter,
    request_from as build_publish_request, run_publish, PublishAction, SubprocessTransport,
};
use std::collections::BTreeSet;

use super::commands_ops::FleetCommands;
use super::inventory::cmd_fleet_online;

/// Schedule per-project fleet units across at most `jobs` worker
/// threads. Results return in roster order. With `fail_fast`,
/// scheduling stops once a worker reports failure — already-running
/// projects finish, no new projects start.
pub(super) fn run_fleet_concurrent<F>(
    eligible: &[forge::publish::inventory::InventoryFleetEntry],
    jobs: usize,
    fail_fast: bool,
    worker: F,
) -> Vec<FleetEntryOutcome>
where
    F: Fn(forge::publish::inventory::InventoryFleetEntry) -> FleetEntryOutcome + Sync,
{
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::mpsc;

    let width = jobs.max(1).min(FLEET_MAX_JOBS).min(eligible.len().max(1));
    let stop = AtomicBool::new(false);
    let (tx, rx) = mpsc::channel::<(usize, FleetEntryOutcome)>();
    // Atomic index dispenser: workers claim the next roster slot.
    let next: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    std::thread::scope(|scope| {
        for _ in 0..width {
            scope.spawn(|| {
                loop {
                    if fail_fast && stop.load(Ordering::SeqCst) {
                        return;
                    }
                    let index = next.fetch_add(1, Ordering::SeqCst);
                    let Some(entry) = eligible.get(index) else {
                        return;
                    };
                    if fail_fast && stop.load(Ordering::SeqCst) {
                        return;
                    }
                    let outcome = worker(entry.clone());
                    if fail_fast
                        && !matches!(outcome, FleetEntryOutcome::Published { healthy: true, .. })
                    {
                        stop.store(true, Ordering::SeqCst);
                    }
                    // The main thread always drains; a send failure
                    // means it went away, so the worker exits.
                    if tx.send((index, outcome)).is_err() {
                        return;
                    }
                }
            });
        }
    });
    drop(tx);
    let mut ordered: Vec<Option<FleetEntryOutcome>> = (0..eligible.len()).map(|_| None).collect();
    for (index, outcome) in rx {
        if index < ordered.len() {
            ordered[index] = Some(outcome);
        }
    }
    // Fail-fast may leave tail slots unclaimed; fill them with a
    // typed skip so result order still matches roster order.
    ordered
        .into_iter()
        .enumerate()
        .map(|(index, slot)| {
            slot.unwrap_or_else(|| FleetEntryOutcome::Failed {
                error: ForgeError::PublishInvalid {
                    reason: format!(
                        "fleet publish for `{}` was not scheduled after an earlier fail-fast failure",
                        eligible[index].id
                    ),
                },
                journal: Some(format!(
                    "fleet publish for `{}` skipped: fail-fast stopped scheduling",
                    eligible[index].id
                )),
            })
        })
        .collect()
}

#[cfg(test)]
pub(super) mod fleet_jobs_tests {
    use super::super::commands::FleetEntryOutcome;
    use super::super::publish::validate_fleet_jobs;
    use super::deploy_needs_prepare_materialization;
    use super::run_fleet_concurrent;
    use forge::core::ForgeError;
    use forge::publish::PublishAction;

    #[test]
    fn jobs_flag_is_bounded() {
        assert!(validate_fleet_jobs(1).is_ok());
        assert!(validate_fleet_jobs(4).is_ok());
        assert!(validate_fleet_jobs(32).is_ok());
        assert!(validate_fleet_jobs(0).is_err());
        assert!(validate_fleet_jobs(33).is_err());
        assert!(validate_fleet_jobs(usize::MAX).is_err());
    }

    #[test]
    fn concurrent_scheduler_preserves_roster_order() {
        let entries: Vec<forge::publish::inventory::InventoryFleetEntry> = (0..8)
            .map(|i| forge::publish::inventory::InventoryFleetEntry {
                id: format!("proj-{i:02}"),
                runtime: forge::publish::inventory::RuntimeClass::Web,
                profile: String::new(),
                revision: "0".repeat(40),
                classification: forge::publish::inventory::InventoryClassification::ComposeReady,
                compose_file: Some("docker-compose.yml".to_string()),
                source_path: None,
                public_http: false,
                public_port: None,
                subdomain: None,
                reason: None,
            })
            .collect();
        let outcomes =
            run_fleet_concurrent(&entries, 4, false, |entry| FleetEntryOutcome::Published {
                human: entry.id.clone(),
                value: serde_json::json!({}),
                healthy: true,
                stage_count: 4,
                subdomain: None,
                journal: String::new(),
            });
        let ids: Vec<String> = outcomes
            .iter()
            .map(|o| match o {
                FleetEntryOutcome::Published { human, .. } => human.clone(),
                FleetEntryOutcome::Failed { .. } => "failed".to_string(),
                FleetEntryOutcome::Phased { .. } => "phased".to_string(),
            })
            .collect();
        assert_eq!(
            ids,
            (0..8).map(|i| format!("proj-{i:02}")).collect::<Vec<_>>()
        );
    }

    #[test]
    fn phased_outcomes_preserve_roster_order() {
        let entries: Vec<forge::publish::inventory::InventoryFleetEntry> = (0..5)
            .map(|i| forge::publish::inventory::InventoryFleetEntry {
                id: format!("proj-{i:02}"),
                runtime: forge::publish::inventory::RuntimeClass::Web,
                profile: String::new(),
                revision: "0".repeat(40),
                classification: forge::publish::inventory::InventoryClassification::ComposeReady,
                compose_file: Some("docker-compose.yml".to_string()),
                source_path: None,
                public_http: false,
                public_port: None,
                subdomain: None,
                reason: None,
            })
            .collect();
        let outcomes =
            run_fleet_concurrent(&entries, 3, false, |_entry| FleetEntryOutcome::Phased {
                reports: Vec::new(),
                error: None,
            });
        assert_eq!(outcomes.len(), 5);
        assert!(outcomes.iter().all(|o| matches!(
            o,
            FleetEntryOutcome::Phased {
                reports: _,
                error: None
            }
        )));
    }

    #[test]
    fn concurrent_scheduler_stops_scheduling_on_fail_fast() {
        let entries: Vec<forge::publish::inventory::InventoryFleetEntry> = (0..6)
            .map(|i| forge::publish::inventory::InventoryFleetEntry {
                id: format!("proj-{i:02}"),
                runtime: forge::publish::inventory::RuntimeClass::Web,
                profile: String::new(),
                revision: "0".repeat(40),
                classification: forge::publish::inventory::InventoryClassification::ComposeReady,
                compose_file: Some("docker-compose.yml".to_string()),
                source_path: None,
                public_http: false,
                public_port: None,
                subdomain: None,
                reason: None,
            })
            .collect();
        let outcomes = run_fleet_concurrent(&entries, 1, true, |entry| {
            if entry.id == "proj-00" {
                FleetEntryOutcome::Failed {
                    error: ForgeError::PublishInvalid {
                        reason: "boom".to_string(),
                    },
                    journal: None,
                }
            } else {
                FleetEntryOutcome::Published {
                    human: entry.id.clone(),
                    value: serde_json::json!({}),
                    healthy: true,
                    stage_count: 4,
                    subdomain: None,
                    journal: String::new(),
                }
            }
        });
        assert!(matches!(outcomes[0], FleetEntryOutcome::Failed { .. }));
        // Tail slots were never scheduled: the scheduler fills them
        // with a typed skip rather than running them.
        assert!(matches!(outcomes[1], FleetEntryOutcome::Failed { .. }));
    }

    #[test]
    fn deploy_only_phases_materialize_prepare_inputs() {
        // Phase C runs Deploy alone on a fresh adapter, which must
        // materialise Prepare's inputs first (`fleet-live-rollout`
        // live evidence 2026-09-30).
        assert!(deploy_needs_prepare_materialization(&[
            PublishAction::Deploy
        ]));
        assert!(deploy_needs_prepare_materialization(&[
            PublishAction::Sync,
            PublishAction::Db,
            PublishAction::Deploy,
        ]));
        // A subset that runs Prepare already has the state; Sync/Db
        // never need it.
        assert!(!deploy_needs_prepare_materialization(&[
            PublishAction::Prepare
        ]));
        assert!(!deploy_needs_prepare_materialization(&[
            PublishAction::Prepare,
            PublishAction::Deploy,
        ]));
        assert!(!deploy_needs_prepare_materialization(&[
            PublishAction::Sync,
            PublishAction::Db,
        ]));
    }
}

/// Load an inventory snapshot from a local JSON file or an
/// external adapter executable. The path is checked for `is_file`
/// so an external adapter is distinguishable from a regular file.
pub(super) fn load_inventory_snapshot(
    path: &std::path::Path,
) -> Result<forge::publish::inventory::InventorySnapshot, ForgeError> {
    use forge::publish::inventory::invoke_external;
    if path.is_file() {
        // Distinguish the JSON contract from an external adapter by
        // checking the suffix: `.json` always means local; anything
        // else is an adapter executable.
        let is_json = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("json"))
            .unwrap_or(false);
        if is_json {
            return forge::publish::inventory::load_local(path);
        }
        return invoke_external(path, std::path::Path::new("."));
    }
    Err(ForgeError::PublishInvalid {
        reason: format!(
            "inventory source `{}` is not a local file or adapter executable",
            path.display()
        ),
    })
}

/// Compatibility adapter that synthesizes an inventory snapshot
/// from the legacy workspace-governance `projects.json` registry.
/// Used when no `--inventory` is supplied so the seven-project
/// handoff keeps working during migration.
pub(super) fn legacy_inventory_snapshot(
    registry_path: &std::path::Path,
    workspace_root: &std::path::Path,
    lifecycle: &str,
) -> Result<forge::publish::inventory::InventorySnapshot, ForgeError> {
    use forge::publish::fleet::{filter_eligible, load_registry};
    use forge::publish::inventory::{
        InventoryEntry, InventoryMalformedEntry, InventorySnapshot, RuntimeClass,
        INVENTORY_CONTRACT_VERSION,
    };

    let registry = load_registry(registry_path).map_err(|err| match err {
        ForgeError::PublishInvalid { reason } => ForgeError::PublishInvalid {
            reason: format!(
                "legacy fleet registry `{registry_path}`: {reason}",
                registry_path = registry_path.display()
            ),
        },
        other => other,
    })?;
    let eligible = filter_eligible(&registry, workspace_root, lifecycle);
    let now = forge::publish::inventory::now_rfc3339();
    let mut projects = Vec::with_capacity(eligible.len());
    let mut malformed = Vec::new();
    let mut seen: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    // Surface every declared entry — even those the legacy filter
    // excluded — so the operator sees the complete fleet.
    for entry in &registry.projects {
        if !seen.insert(entry.id.clone()) {
            malformed.push(InventoryMalformedEntry {
                name: entry.id.clone(),
                reason: "duplicate fleet registry id".to_string(),
            });
            continue;
        }
        let declared_lifecycle = entry.lifecycle.as_deref().unwrap_or("active");
        let compose = if eligible.iter().any(|e| e.id == entry.id) {
            "docker-compose.yml"
        } else if declared_lifecycle != lifecycle {
            continue; // non-matching lifecycle — skip without claiming invalid
        } else {
            malformed.push(InventoryMalformedEntry {
                name: entry.id.clone(),
                reason: format!(
                    "legacy registry entry has no Compose file at `{}`",
                    entry.path.as_deref().unwrap_or("<unspecified>")
                ),
            });
            continue;
        };
        // Revision is unknown from the legacy registry; a future
        // git-aware adapter replaces this. Synthesize an explicit
        // sentinel so the contract field stays populated.
        let revision = "0".repeat(40);
        projects.push(InventoryEntry {
            id: entry.id.clone(),
            repository: format!("legacy-registry:{}", registry_path.display()),
            revision,
            profile: entry.profile.clone().unwrap_or_default(),
            runtime: RuntimeClass::Web,
            compose_file: Some(compose.to_string()),
            source_path: entry
                .path
                .as_deref()
                .map(|path| workspace_root.join(path).display().to_string()),
            public_http: false,
            public_port: None,
        });
    }
    Ok(InventorySnapshot {
        contract: INVENTORY_CONTRACT_VERSION.to_string(),
        provider: "legacy-fleet-registry".to_string(),
        generated_at: now,
        source: Some(registry_path.display().to_string()),
        projects,
        malformed,
    })
}

/// Select the publish adapter without a new CLI flag (`decoupled-remote-publish`
/// task 2.6). `FORGE_PUBLISH_ADAPTER=jenkins` (or `legacy`/`mac-scripts`)
/// restores the Mac-script lane for one-cycle rollback; every other value, including
/// unset, selects `remote-compose`. Both adapters stay compiled.
pub(super) fn use_legacy_publish_adapter() -> bool {
    matches!(
        std::env::var(forge::publish::remote_compose::ADAPTER_ENV)
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase()
            .as_str(),
        "jenkins" | "legacy" | "mac-scripts"
    )
}

pub(super) fn resolve_publish_target(
    target: &str,
) -> Result<(std::path::PathBuf, String), ForgeError> {
    let candidate = std::path::Path::new(target);
    if candidate.is_dir() {
        let canonical = candidate
            .canonicalize()
            .map_err(|_| ForgeError::PathUnavailable {
                path: target.to_string(),
            })?;
        // Manifest is optional for publish: the Jenkins adapter only
        // needs the project id (derived from the directory name) and
        // the docker-compose file inside. Skip the manifest lookup so
        // legacy projects without forge.yaml can still be published.
        let id = canonical
            .file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())
            .ok_or_else(|| ForgeError::PathUnavailable {
                path: target.to_string(),
            })?;
        return Ok((canonical, id));
    }
    // Fall back to the registry so callers can pass a project id.
    let db_path = default_registry_path();
    if let Ok(registry) = Registry::open(&db_path) {
        if let Ok(record) = registry.inspect(target) {
            let path = std::path::PathBuf::from(&record.path);
            if path.is_dir() {
                return Ok((path, record.id));
            }
        }
    }
    Err(ForgeError::PathUnavailable {
        path: target.to_string(),
    })
}

pub(super) fn deploy_plan_output(
    plan: &forge::deploy::DeployPlan,
    format: Format,
) -> Result<Output, ForgeError> {
    let json = serde_json::json!({"plan": plan});
    let human = forge::deploy::engine::render_plan_human(plan);
    Ok(as_output(format, human, json))
}

pub(super) fn deploy_report_output(
    report: &forge::deploy::DeployReport,
    format: Format,
) -> Result<Output, ForgeError> {
    let json = serde_json::json!({"deploy": report});
    let human = forge::deploy::render_report_human(report);
    Ok(as_output(format, human, json))
}

pub(super) fn deploy_list_output(
    entries: &[forge::deploy::DeployListEntry],
    project_id: &str,
    format: Format,
) -> Result<Output, ForgeError> {
    let json = serde_json::json!({"deploys": entries, "project_id": project_id});
    if entries.is_empty() {
        let human = format!("no deploys for project `{project_id}`");
        return Ok(as_output(format, human, json));
    }
    let mut human = format!(
        "{:<48} {:<12} {:<10} {:<10} {}",
        "Deploy", "Project", "Target", "State", "Last Run"
    );
    for entry in entries {
        human.push_str(&format!(
            "\n{:<48} {:<12} {:<10} {:<10} {}",
            entry.deploy_id,
            truncate(&entry.project_id, 12),
            entry.target,
            entry.current_state,
            entry.last_run_at
        ));
    }
    Ok(as_output(format, human, json))
}

pub(super) fn deploy_state_output(
    state: &forge::deploy::DeployState,
    format: Format,
) -> Result<Output, ForgeError> {
    let current_state = state.current_state();
    let mut deploy_value = serde_json::to_value(state).map_err(|err| ForgeError::Registry {
        reason: err.to_string(),
    })?;
    if let Some(obj) = deploy_value.as_object_mut() {
        obj.insert(
            "current_state".to_string(),
            serde_json::Value::String(current_state),
        );
    }
    let json = serde_json::json!({"deploy": deploy_value});
    let human = render_deploy_state_human(state);
    Ok(as_output(format, human, json))
}

fn render_deploy_state_human(state: &forge::deploy::DeployState) -> String {
    let mut lines: Vec<String> = Vec::new();
    lines.push(format!("project: {}", state.identity.project_id));
    lines.push(format!("deploy_id: {}", state.identity.id));
    lines.push(format!(
        "target: {} ({})",
        state.target.name, state.target.kind
    ));
    lines.push(format!(
        "source_revision: {}",
        state.identity.source_revision
    ));
    lines.push(format!("adapter: {}", state.adapter));
    if let Some(artifact) = &state.artifact {
        lines.push(format!("artifact: {}", artifact.path));
        lines.push(format!("artifact_hash: {}", artifact.content_hash));
    }
    if let Some(health) = &state.health {
        lines.push(format!(
            "health: {} ({})",
            health.kind,
            health_label_for_state(health)
        ));
    }
    lines.push(format!("current_state: {}", state.current_state()));
    lines.push(format!("last_run_at: {}", state.last_run_at));
    if let Some(obs) = &state.last_observation {
        lines.push(format!(
            "last_observation: {} at {}: {}",
            obs.status, obs.observed_at, obs.detail
        ));
        for line in &obs.evidence {
            lines.push(format!("      evidence: {line}"));
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

fn health_label_for_state(health: &forge::deploy::DeployHealthSpec) -> String {
    match health.kind.as_str() {
        forge::deploy::HEALTH_DOCKER => {
            format!("service={}", health.service.as_deref().unwrap_or("?"))
        }
        forge::deploy::HEALTH_HTTP => {
            format!("url={}", health.url.as_deref().unwrap_or("?"))
        }
        forge::deploy::HEALTH_PROCESS => {
            format!("process={}", health.process.as_deref().unwrap_or("?"))
        }
        other => other.to_string(),
    }
}

/// Whether a phase subset needs Prepare's materialised adapter state
/// before it can run. Deploy builds its plan from the Compose file,
/// profiles, shared-DB overlay and router documents that Prepare
/// materialises; a subset that already runs Prepare (or never runs
/// Deploy) needs nothing extra.
pub(super) fn deploy_needs_prepare_materialization(phases: &[PublishAction]) -> bool {
    phases.contains(&PublishAction::Deploy) && !phases.contains(&PublishAction::Prepare)
}

/// Roster index of a project id. The phased fleet loop keeps worker
/// results aligned to roster order through indices, not clones.
pub(super) fn entry_index(
    eligible: &[forge::publish::inventory::InventoryFleetEntry],
    project_id: &str,
) -> usize {
    eligible
        .iter()
        .position(|entry| entry.id == project_id)
        .unwrap_or(0)
}

/// Replay the exact journal rows `run_publish` would have written
/// for one report: one per-stage row plus the summary row. The main
/// thread calls this serially for every worker-returned report, so
/// parallel runs persist byte-identical row shapes to sequential
/// runs (only timestamps differ, as between any two runs).
pub(super) fn replay_publish_journal(registry: &Registry, report: &forge::publish::PublishReport) {
    use forge::publish::publish_journal_state;
    for outcome in &report.stages {
        let _ = registry.record_operation(
            "publish",
            &report.project_id,
            publish_journal_state(&outcome.status),
            &format!(
                "publish {} via {}: {} ({}, {}ms)",
                outcome.stage, report.adapter, outcome.note, outcome.status, outcome.elapsed_ms
            ),
        );
    }
    let verdict = if report.dry_run || report.healthy {
        "done"
    } else {
        "failed"
    };
    let _ = registry.record_operation(
        "publish",
        &report.project_id,
        verdict,
        &format!(
            "publish {} summary via {}: healthy={} stages={}",
            report.action,
            report.adapter,
            report.healthy,
            report.stages.len()
        ),
    );
}

/// Run a subset of publish phases for one eligible fleet entry on
/// the calling thread. A fresh adapter + transport pair is built
/// per call (adapters hold `RefCell` state: not shareable, cheap
/// to construct) and the publish runs with `registry = None` so no
/// row escapes the worker; the main thread replays every journal
/// row serially from the returned reports.
pub(super) fn run_fleet_entry_phases(
    entry: forge::publish::inventory::InventoryFleetEntry,
    db_path: &std::path::Path,
    legacy_lane: bool,
    dry_run: bool,
    phases: &[PublishAction],
) -> FleetEntryOutcome {
    let project_id = entry.id.clone();
    let source_path = entry
        .source_path
        .as_deref()
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    if !source_path.is_dir() {
        let detail = format!(
            "fleet publish: project `{project_id}` source `{}` is not a directory",
            source_path.display()
        );
        return FleetEntryOutcome::Failed {
            error: ForgeError::PublishInvalid {
                reason: detail.clone(),
            },
            journal: Some(detail),
        };
    }
    // Touch the registry once so a missing/unusable database fails
    // closed on the worker with the same typed error the
    // sequential path surfaces — without writing any row.
    if let Err(err) = open_registry(db_path) {
        return FleetEntryOutcome::Failed {
            error: err,
            journal: None,
        };
    }
    let jenkins_adapter = JenkinsAdapter::from_env();
    let remote_adapter = RemoteComposeAdapter::from_env();
    let adapter: &dyn forge::publish::PublishAdapter = if legacy_lane {
        &jenkins_adapter
    } else {
        &remote_adapter
    };
    let transport = SubprocessTransport::default();
    let mut reports = Vec::with_capacity(phases.len());
    // Deploy consumes state that Prepare materialises on the adapter:
    // the resolved Compose file and its profiles, the shared-DB
    // overlay flag, the env-file probes and the rendered router
    // documents. The phased fleet loop runs Prepare (Phase B, serial)
    // and Deploy (Phase C, parallel) on different adapter instances,
    // so a Deploy-only call must materialise those inputs itself.
    // `materialize` only reads the target and renders locally — the
    // shared registry is written exclusively by Prepare's shipping
    // step, which stays serial — so this is side-effect free on the
    // target and safe across workers (`fleet-live-rollout` live
    // evidence 2026-09-30: a Deploy-only worker previously failed
    // with `publish did not resolve a Compose file`).
    if deploy_needs_prepare_materialization(phases) {
        let prepare_request = build_publish_request(
            project_id.clone(),
            source_path.clone(),
            PublishAction::Prepare,
            dry_run,
        );
        if let Err(err) = adapter.materialize(&prepare_request, &transport, dry_run) {
            return FleetEntryOutcome::Phased {
                reports,
                error: Some(err),
            };
        }
    }
    for action in phases {
        let request = build_publish_request(
            project_id.clone(),
            source_path.clone(),
            action.clone(),
            dry_run,
        );
        match run_publish(&request, adapter, &transport, None) {
            Ok(report) => {
                let healthy = report.healthy;
                reports.push(report);
                // Phase order is Sync -> Db -> Prepare -> Deploy: a
                // failed phase never starts the next one for this
                // project (same fail-closed rule the All action
                // applies inside `run_publish`).
                if !healthy && !dry_run {
                    break;
                }
            }
            Err(err) => {
                return FleetEntryOutcome::Phased {
                    reports,
                    error: Some(err),
                };
            }
        }
    }
    FleetEntryOutcome::Phased {
        reports,
        error: None,
    }
}

/// Ids currently present in the local registry, used only as the
/// managed/unmanaged join set for a fleet observation. Read-only.
fn fleet_local_ids(registry: &Registry) -> Result<BTreeSet<String>, ForgeError> {
    Ok(registry.list()?.into_iter().map(|r| r.id).collect())
}

/// Observe the configured workspace registry (if any) for portal and
/// `forge list` read surfaces. A configured-but-unobservable registry
/// projects as `Some(Err(..))` so the surfaces render `unavailable`
/// rather than masking the gap; an unconfigured registry projects as
/// `None` and leaves every local surface byte-identical.
pub(super) fn portal_fleet_projection(
    registry: &Registry,
) -> Option<Result<forge::fleet::FleetReport, ForgeError>> {
    let path = resolve_registry_path(None)?;
    let local_ids = match fleet_local_ids(registry) {
        Ok(ids) => ids,
        Err(err) => return Some(Err(err)),
    };
    Some(observe(Some(&path), DEFAULT_MAX_AGE_SECONDS, &local_ids))
}

pub(crate) fn cmd_fleet(
    db_path: &Path,
    command: &FleetCommands,
    format: Format,
) -> Result<Output, ForgeError> {
    let (workspace_registry, max_age) = match command {
        FleetCommands::List {
            workspace_registry,
            max_age,
        }
        | FleetCommands::Status {
            workspace_registry,
            max_age,
        }
        | FleetCommands::Inspect {
            workspace_registry,
            max_age,
            ..
        } => (workspace_registry.as_deref(), *max_age),
        FleetCommands::Online { .. } => (None, DEFAULT_MAX_AGE_SECONDS),
    };
    fleet::validate_max_age(max_age)?;
    // The local registry is opened only as the read side of the
    // managed/unmanaged join; fleet mirroring never registers, imports
    // or mutates anything and never journals.
    let registry = open_registry(db_path)?;
    let local_ids = fleet_local_ids(&registry)?;
    let path = resolve_registry_path(workspace_registry);
    let report = observe(path.as_deref(), max_age, &local_ids)?;
    match command {
        FleetCommands::List { .. } => {
            let human = render_fleet_report_human(&report);
            let json = serde_json::json!({
                "contract": FLEET_CONTRACT_VERSION,
                "fleet": report,
            });
            Ok(as_output(format, human, json))
        }
        FleetCommands::Status { .. } => {
            let human = render_status_human(&report);
            let json = serde_json::json!({
                "contract": FLEET_CONTRACT_VERSION,
                "health": health_json(&report),
            });
            Ok(as_output(format, human, json))
        }
        FleetCommands::Inspect { entry, .. } => {
            let found = inspect_entry(&report, entry)?;
            let human = render_entry_human(&report, found);
            let json = serde_json::json!({
                "contract": FLEET_CONTRACT_VERSION,
                "entry": found,
                "source": report.source,
                "freshness": report.freshness,
                "observed_at": report.observed_at,
            });
            Ok(as_output(format, human, json))
        }
        FleetCommands::Online {
            inventory,
            fleet_registry,
            workspace_root,
            domain,
            timeout_secs,
            dry_run,
        } => cmd_fleet_online(
            inventory.as_deref(),
            fleet_registry.as_deref(),
            workspace_root.as_deref(),
            domain.clone(),
            *timeout_secs,
            *dry_run,
            format,
        ),
    }
}
