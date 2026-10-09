//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use forge::analytics::{
    aggregate_project_metrics, inspect_external_planes, load_config as load_analytics_config,
    metrics_summary_path, render_metrics_human, save_metrics_summary, AnalyticsConfig,
    AnalyticsInspectOptions, AnalyticsProvider, DoctorSummary, MetricsAggregateOptions,
    MetricsSummary, ANALYTICS_CONTRACT_VERSION, ANALYTICS_SYNTHETIC_PROJECT,
};
use forge::core::ForgeError;
use forge::gate::{
    self, evidence_freshness, evidence_summary, load_latest_evidence, render_evidence_human,
    GateAggregate, GateConfig, GateFreshness, GateOutcome, GATE_CONTRACT_VERSION,
};
use forge::registry::Registry;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use super::analytics::resolve_analytics_target;
use super::commands::ContractCommands;
use super::feature::render_error;
use super::projects::{as_output, open_registry};
use crate::{Format, Output};

pub(super) fn cmd_analytics_metrics(
    db_path: &Path,
    target: &str,
    window_days: Option<u32>,
    all: bool,
    format: Format,
) -> Result<Output, ForgeError> {
    let registry = open_registry(db_path)?;
    let project_id: String;
    let external_observations: Vec<forge::analytics::HealthObservation>;
    let default_window: u32;
    let summary_path_dir: Option<PathBuf>;
    if all {
        project_id = ANALYTICS_SYNTHETIC_PROJECT.to_string();
        external_observations = collect_external_observations_for_all(&registry, db_path)?;
        default_window = window_days.unwrap_or(forge::analytics::DEFAULT_WINDOW_DAYS);
        summary_path_dir = None;
    } else {
        let (dir, pid) = resolve_analytics_target(db_path, target)?;
        project_id = pid.clone();
        let config = load_analytics_config(&dir)?;
        let cfg = config.unwrap_or_else(|| AnalyticsConfig {
            enabled: false,
            default_window_days: forge::analytics::DEFAULT_WINDOW_DAYS,
            content: Vec::new(),
            repository: Vec::new(),
        });
        default_window = window_days.unwrap_or(cfg.default_window_days);
        if cfg.enabled && !cfg.all_providers().is_empty() {
            let report = inspect_external_planes(
                &project_id,
                &cfg,
                &AnalyticsInspectOptions { dry_run: false },
            )?;
            external_observations = report.observations;
        } else {
            external_observations = Vec::new();
        }
        summary_path_dir = Some(dir);
    }
    let doctor = DoctorSummary::default();
    let report = aggregate_project_metrics(
        &registry,
        Some(doctor),
        &MetricsAggregateOptions {
            default_window_days: default_window,
            external_observations,
        },
    )?;
    let summary = MetricsSummary {
        contract: report.contract.clone(),
        project_id: project_id.clone(),
        generated_at: report.generated_at.clone(),
        aggregates: report.aggregates.clone(),
    };
    if let Some(dir) = summary_path_dir {
        let path = metrics_summary_path(&dir, &project_id)?;
        let _ = save_metrics_summary(&path, &summary);
    }
    let detail = format!(
        "metrics: scope={} default_window={}d complete={} aggregates={}",
        if all { "all" } else { &project_id },
        default_window,
        report.complete,
        report.aggregates.len()
    );
    if let Ok(reg) = open_registry(db_path) {
        let journal_project = if all {
            ANALYTICS_SYNTHETIC_PROJECT
        } else {
            project_id.as_str()
        };
        let state = if report.complete { "done" } else { "partial" };
        let _ = reg.record_operation("analytics", journal_project, state, &detail);
    }
    let json = serde_json::json!({
        "contract": ANALYTICS_CONTRACT_VERSION,
        "metrics": report,
    });
    let human = render_metrics_human(&report);
    Ok(as_output(format, human, json))
}

fn collect_external_observations_for_all(
    registry: &Registry,
    db_path: &Path,
) -> Result<Vec<forge::analytics::HealthObservation>, ForgeError> {
    let mut out: Vec<forge::analytics::HealthObservation> = Vec::new();
    let projects = registry.list()?;
    for record in projects {
        let dir = PathBuf::from(&record.path);
        if !dir.is_dir() {
            continue;
        }
        let config = match load_analytics_config(&dir) {
            Ok(Some(cfg)) => cfg,
            _ => continue,
        };
        if !config.enabled {
            continue;
        }
        let report = inspect_external_planes(
            &record.id,
            &config,
            &AnalyticsInspectOptions { dry_run: false },
        )?;
        out.extend(report.observations);
    }
    let _ = db_path;
    Ok(out)
}

fn _ensure_analytics_symbols_used(provider: AnalyticsProvider) -> &'static str {
    forge::analytics::provider_label(provider)
}

/// Resolve a target string (registered id or filesystem path) to a
/// project directory and the manifest project id. Mirrors the upgrade
/// resolution contract so the CLI accepts both spellings.
pub(super) fn resolve_identity_target(
    db_path: &Path,
    target: &str,
) -> Result<(PathBuf, String), ForgeError> {
    let candidate = Path::new(target);
    if candidate.is_dir() {
        let dir = candidate
            .canonicalize()
            .map_err(|_| ForgeError::PathUnavailable {
                path: target.to_string(),
            })?;
        let (manifest, _) = forge::core::manifest::Manifest::load_from_dir(&dir, None)?;
        return Ok((dir, manifest.project.id));
    }
    let registry = open_registry(db_path)?;
    let record = registry.inspect(target)?;
    let dir = PathBuf::from(&record.path);
    if !dir.is_dir() {
        return Err(ForgeError::PathUnavailable { path: record.path });
    }
    let (manifest, _) = forge::core::manifest::Manifest::load_from_dir(&dir, None)?;
    Ok((dir, manifest.project.id))
}

/// Resolve a gate target to a project directory and identity. A
/// registered project (by id or registered path) always wins so the
/// journal row names the real registry identity; an unregistered
/// directory falls back to the manifest id, the Workspace Governance
/// declaration id, or the directory name — gate is always
/// project-scoped, never a fleet operation.
fn resolve_gate_target(db_path: &Path, target: &str) -> Result<(PathBuf, String), ForgeError> {
    if let Ok(registry) = open_registry(db_path) {
        if let Ok(record) = registry.inspect(target) {
            let dir = PathBuf::from(&record.path);
            if !dir.is_dir() {
                return Err(ForgeError::PathUnavailable { path: record.path });
            }
            return Ok((dir, record.id));
        }
    }
    let candidate = Path::new(target);
    if candidate.is_dir() {
        let dir = candidate
            .canonicalize()
            .map_err(|_| ForgeError::PathUnavailable {
                path: target.to_string(),
            })?;
        if let Ok((manifest, _)) = forge::core::manifest::Manifest::load_from_dir(&dir, None) {
            return Ok((dir, manifest.project.id));
        }
        if let Some(id) = gate::declared_project_id(&dir) {
            if forge::core::validate_project_id(&id).is_ok() {
                return Ok((dir, id));
            }
        }
        let name = dir
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| ".".to_string());
        return Ok((dir, name));
    }
    let registry = open_registry(db_path)?;
    let record = registry.inspect(target)?;
    let dir = PathBuf::from(&record.path);
    if !dir.is_dir() {
        return Err(ForgeError::PathUnavailable { path: record.path });
    }
    Ok((dir, record.id))
}

pub(super) fn render_output(output: Output) {
    match output {
        Output::Human(text) => println!("{text}"),
        Output::Json(value) => println!("{}", serde_json::to_string_pretty(&value).unwrap()),
        Output::Raw(text) => print!("{text}"),
    }
}

fn gate_fail(err: &ForgeError, format: Format) -> ExitCode {
    render_error(err, format);
    ExitCode::from(1)
}

/// `forge gate [status] [TARGET] [--dry-run] [--timeout-secs N]`.
///
/// Exit codes mirror the sibling's blocking semantics: 0 only for a
/// fresh passing aggregate, 1 for blocked, failed, unknown, stale or
/// absent evidence, and an unavailable runtime. Evidence documents are
/// printed to stdout even when the verdict is non-zero (the fleet
/// precedent); only Forge-side failures use the typed stderr path.
pub(crate) fn cmd_gate(
    db_path: &Path,
    args: &[String],
    dry_run: bool,
    timeout_secs: Option<u64>,
    format: Format,
) -> ExitCode {
    // Parse the `status | evidence [status] | TARGET` positional forms.
    // Three mutually-exclusive actions:
    //   - `status [TARGET]` → read gate verdict evidence
    //   - `evidence [TARGET]` → run export and persist release evidence
    //   - `evidence status [TARGET]` → read persisted release evidence
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum GateCommand {
        Run,
        Status,
        EvidenceRun,
        EvidenceStatus,
    }
    let (command, target) = match args.split_first() {
        None => (GateCommand::Run, ".".to_string()),
        Some((first, rest)) if first == "status" => {
            if rest.len() > 1 {
                return gate_fail(
                    &ForgeError::GateInvalid {
                        reason: format!(
                            "`forge gate status` takes at most one TARGET; got {} extra arguments",
                            rest.len()
                        ),
                    },
                    format,
                );
            }
            (
                GateCommand::Status,
                rest.first().cloned().unwrap_or_else(|| ".".to_string()),
            )
        }
        Some((first, rest)) if first == "evidence" => {
            // `evidence [TARGET]` → run export and persist.
            // `evidence status [TARGET]` → read persisted release evidence.
            match rest.split_first() {
                Some((second, rest2)) if second == "status" => {
                    // `evidence status [TARGET]` — read without running.
                    if rest2.len() > 1 {
                        return gate_fail(
                            &ForgeError::GateInvalid {
                                reason: format!(
                                    "`forge gate evidence status` takes at most one TARGET; got {} extra arguments",
                                    rest2.len()
                                ),
                            },
                            format,
                        );
                    }
                    (
                        GateCommand::EvidenceStatus,
                        rest2.first().cloned().unwrap_or_else(|| ".".to_string()),
                    )
                }
                _ => {
                    // `evidence [TARGET]` — run export and persist.
                    if rest.len() > 1 {
                        return gate_fail(
                            &ForgeError::GateInvalid {
                                reason: format!(
                                    "`forge gate evidence [TARGET]` takes at most one TARGET; got {} extra arguments",
                                    rest.len()
                                ),
                            },
                            format,
                        );
                    }
                    (
                        GateCommand::EvidenceRun,
                        rest.first().cloned().unwrap_or_else(|| ".".to_string()),
                    )
                }
            }
        }
        Some((first, rest)) => {
            if !rest.is_empty() {
                return gate_fail(
                    &ForgeError::GateInvalid {
                        reason: format!(
                            "unexpected arguments {:?}; usage: `forge gate [TARGET]`, `forge gate status [TARGET]`, `forge gate evidence [TARGET]`, or `forge gate evidence status [TARGET]`",
                            rest
                        ),
                    },
                    format,
                );
            }
            (GateCommand::Run, first.clone())
        }
    };
    if matches!(command, GateCommand::Status) && dry_run {
        return gate_fail(
            &ForgeError::GateInvalid {
                reason:
                    "--dry-run does not apply to `forge gate status` (it never runs the runtime)"
                        .to_string(),
            },
            format,
        );
    }
    if matches!(command, GateCommand::Status) && timeout_secs.is_some() {
        return gate_fail(
            &ForgeError::GateInvalid {
                reason: "--timeout-secs does not apply to `forge gate status` (it never runs the runtime)".to_string(),
            },
            format,
        );
    }
    if matches!(command, GateCommand::EvidenceRun) && (dry_run || timeout_secs.is_some()) {
        return gate_fail(
            &ForgeError::GateInvalid {
                reason: "--dry-run and --timeout-secs do not apply to `forge gate evidence` (it consumes the sibling's export verb)"
                    .to_string(),
            },
            format,
        );
    }
    let config = match timeout_secs {
        Some(raw) => {
            let parsed = match gate::parse_timeout_secs(raw) {
                Ok(timeout) => timeout,
                Err(err) => return gate_fail(&err, format),
            };
            let mut config = GateConfig::from_env();
            config.timeout = parsed;
            config
        }
        None => GateConfig::from_env(),
    };
    let (dir, project_id) = match resolve_gate_target(db_path, &target) {
        Ok(resolved) => resolved,
        Err(err) => return gate_fail(&err, format),
    };
    match command {
        GateCommand::Status => cmd_gate_status(&dir, &project_id, format),
        GateCommand::EvidenceRun => cmd_gate_evidence(db_path, &dir, &project_id, &config, format),
        GateCommand::EvidenceStatus => cmd_gate_evidence_status(&dir, &project_id, format),
        GateCommand::Run => cmd_gate_run(db_path, &dir, &project_id, &config, dry_run, format),
    }
}

fn cmd_gate_status(dir: &Path, project_id: &str, format: Format) -> ExitCode {
    let evidence = match load_latest_evidence(dir) {
        Ok(Some(evidence)) => evidence,
        Ok(None) => {
            let json = serde_json::json!({
                "contract": GATE_CONTRACT_VERSION,
                "gate": {
                    "project": project_id,
                    "freshness": GateFreshness::Absent.label(),
                    "aggregate": "unverified",
                    "checks": [],
                },
            });
            let human = format!(
                "project: {project_id}\nfreshness: {}\naggregate: unverified\ndetail: the gate has never run; unverified is not a pass",
                GateFreshness::Absent.label()
            );
            render_output(as_output(format, human, json));
            return ExitCode::from(1);
        }
        Err(err) => return gate_fail(&err, format),
    };
    let current = gate::capture_revision(dir);
    let freshness = evidence_freshness(&evidence, current.as_deref());
    let passed =
        matches!(evidence.aggregate, GateAggregate::Passed) && freshness == GateFreshness::Fresh;
    // The present-evidence envelope flattens the record fields onto
    // `gate` (plus freshness) so every read surface reports the same
    // aggregate, revision, runtime name and timestamp bytes.
    let mut value = match serde_json::to_value(&evidence) {
        Ok(value) => value,
        Err(err) => {
            return gate_fail(
                &ForgeError::GateInvalid {
                    reason: format!("cannot encode gate evidence: {err}"),
                },
                format,
            )
        }
    };
    if let Some(object) = value.as_object_mut() {
        object.insert(
            "freshness".to_string(),
            serde_json::json!(freshness.label()),
        );
    }
    let json = serde_json::json!({
        "contract": GATE_CONTRACT_VERSION,
        "gate": value,
    });
    let human = render_evidence_human(&evidence, Some(freshness), None);
    render_output(as_output(format, human, json));
    if passed {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

/// `forge gate evidence [TARGET]` — run the sibling's export verb, validate,
/// and persist the attributed release-evidence record.
fn cmd_gate_evidence(
    db_path: &Path,
    dir: &Path,
    project_id: &str,
    config: &GateConfig,
    format: Format,
) -> ExitCode {
    let registry = match open_registry(db_path) {
        Ok(registry) => registry,
        Err(err) => return gate_fail(&err, format),
    };
    let outcome = gate::evidence::consume_export(dir, project_id, config);
    match outcome {
        gate::evidence::EvidenceOutcome::Record(record) => {
            let relative = match gate::evidence::save_release_evidence(dir, &record) {
                Ok(rel) => rel,
                Err(err) => return gate_fail(&err, format),
            };
            // Journal the consumption attempt.
            let verdict = if record.freshness == gate::evidence::EvidenceFreshness::Fresh {
                "done"
            } else {
                "done"
            };
            let _ = registry.record_operation(
                "gate",
                project_id,
                verdict,
                &format!(
                    "gate evidence consumed: freshness={} verified={} configured={} declared={} unverified={}",
                    record.freshness.label(),
                    record.counts.verified,
                    record.counts.configured,
                    record.counts.declared,
                    record.counts.unverified
                ),
            );
            let json = serde_json::json!({
                "contract": "release-evidence/0.1.0",
                "release_evidence": &record,
                "persisted": relative.display().to_string(),
            });
            let human = gate::evidence::render_release_evidence_human(
                &record,
                Some(relative.display().to_string().as_str()),
            );
            render_output(as_output(format, human, json));
            ExitCode::SUCCESS
        }
        gate::evidence::EvidenceOutcome::Absent { reason } => {
            let bound = gate::bound_note(&reason);
            let _ = registry.record_operation(
                "gate",
                project_id,
                "failed",
                &format!("gate evidence export unavailable: {bound}"),
            );
            gate_fail(
                &ForgeError::GateEvidenceUnavailable { reason: bound },
                format,
            )
        }
        gate::evidence::EvidenceOutcome::Refused {
            reason,
            refusal_reasons,
        } => {
            let bound = gate::bound_note(&reason);
            let _ = registry.record_operation(
                "gate",
                project_id,
                "failed",
                &format!("gate evidence export refused: {bound}"),
            );
            gate_fail(
                &ForgeError::GateEvidenceUnavailable {
                    reason: format!(
                        "{} ({} refusal reason(s): {})",
                        bound,
                        refusal_reasons.len(),
                        refusal_reasons.join("; ")
                    ),
                },
                format,
            )
        }
        gate::evidence::EvidenceOutcome::Unavailable { reason } => {
            let bound = gate::bound_note(&reason);
            let _ = registry.record_operation(
                "gate",
                project_id,
                "failed",
                &format!("gate evidence export unavailable: {bound}"),
            );
            gate_fail(
                &ForgeError::GateEvidenceUnavailable { reason: bound },
                format,
            )
        }
    }
}

/// `forge gate evidence status [TARGET]` — read persisted release-evidence
/// without running the export.
fn cmd_gate_evidence_status(dir: &Path, project_id: &str, format: Format) -> ExitCode {
    match gate::evidence::load_latest_release_evidence(dir) {
        Ok(Some(record)) => {
            let json = serde_json::json!({
                "contract": "release-evidence/0.1.0",
                "release_evidence": &record,
            });
            let human = gate::evidence::render_release_evidence_human(&record, None);
            render_output(as_output(format, human, json));
            ExitCode::SUCCESS
        }
        Ok(None) => {
            let reason =
                "no release-evidence record persisted; run `forge gate evidence` first".to_string();
            let json = serde_json::json!({
                "contract": "release-evidence/0.1.0",
                "release_evidence": null,
                "freshness": "absent",
                "detail": reason,
            });
            let human = gate::evidence::render_absent_human(project_id, &reason);
            render_output(as_output(format, human, json));
            ExitCode::from(1)
        }
        Err(err) => gate_fail(&err, format),
    }
}

fn cmd_gate_run(
    db_path: &Path,
    dir: &Path,
    project_id: &str,
    config: &GateConfig,
    dry_run: bool,
    format: Format,
) -> ExitCode {
    // The registry is required for journaling every attempted real run;
    // it is opened before the runtime is contacted so an unwritable
    // registry never leaves an unrecorded gate execution.
    let registry = match open_registry(db_path) {
        Ok(registry) => registry,
        Err(err) => return gate_fail(&err, format),
    };
    let outcome = gate::run_gate(dir, project_id, config, dry_run);
    let (output, verdict) = match outcome {
        GateOutcome::PlanPreview {
            runtime,
            runtime_version,
            plan,
        } => {
            let json = serde_json::json!({
                "contract": GATE_CONTRACT_VERSION,
                "dry_run": true,
                "gate": {
                    "project": project_id,
                    "runtime": runtime,
                    "runtime_version": runtime_version,
                    "plan": plan,
                    "persisted": false,
                    "journaled": false,
                },
            });
            let mut human = format!(
                "dry-run: gate plan preview for {project_id} (nothing was executed, persisted or journaled)\nruntime: {}",
                runtime_version
                    .as_deref()
                    .map(|version| format!("{runtime} ({version})"))
                    .unwrap_or_else(|| runtime.clone())
            );
            for line in &plan {
                human.push_str(&format!("\n  {line}"));
            }
            (as_output(format, human, json), ExitCode::SUCCESS)
        }
        GateOutcome::Evidence(evidence) => {
            let current = gate::capture_revision(dir);
            let freshness = evidence_freshness(&evidence, current.as_deref());
            let mut value = match serde_json::to_value(&evidence) {
                Ok(value) => value,
                Err(err) => {
                    return gate_fail(
                        &ForgeError::GateInvalid {
                            reason: format!("cannot encode gate evidence: {err}"),
                        },
                        format,
                    )
                }
            };
            let persisted: Option<String> = if evidence.dry_run {
                None
            } else {
                match gate::save_evidence(dir, &evidence) {
                    Ok(relative) => Some(relative.display().to_string()),
                    Err(err) => return gate_fail(&err, format),
                }
            };
            let verdict = gate::journal_verdict(&GateOutcome::Evidence(evidence.clone()));
            if !evidence.dry_run {
                let _ = registry.record_operation(
                    "gate",
                    project_id,
                    verdict,
                    &format!("gate {verdict}: {}", evidence_summary(&evidence)),
                );
            }
            if let Some(object) = value.as_object_mut() {
                object.insert(
                    "freshness".to_string(),
                    serde_json::json!(freshness.label()),
                );
            }
            let json = serde_json::json!({
                "contract": GATE_CONTRACT_VERSION,
                "dry_run": evidence.dry_run,
                "gate": value,
                "persisted": persisted,
                "journaled": !evidence.dry_run,
            });
            let human = if evidence.dry_run {
                format!(
                    "{}\ndry-run: the rehearsal document was not persisted and not journaled",
                    render_evidence_human(&evidence, Some(freshness), None)
                )
            } else {
                render_evidence_human(&evidence, Some(freshness), persisted.as_deref())
            };
            let code = if matches!(evidence.aggregate, GateAggregate::Passed) {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            };
            (as_output(format, human, json), code)
        }
        GateOutcome::Unavailable { reason } => {
            let bound = gate::bound_note(&reason);
            // A refused rehearsal leaves no shadow: dry-runs never
            // journal (the deploy-rehearsal precedent). Real attempted
            // runs journal `failed` so the history survives the gap.
            if !dry_run {
                let _ = registry.record_operation(
                    "gate",
                    project_id,
                    "failed",
                    &format!("gate failed: runtime unavailable; {bound}"),
                );
            }
            return gate_fail(
                &ForgeError::GateRuntimeUnavailable { reason: bound },
                format,
            );
        }
    };
    render_output(output);
    verdict
}

pub(crate) fn cmd_contract(
    db_path: &Path,
    command: &ContractCommands,
    format: Format,
) -> Result<Output, ForgeError> {
    match command {
        ContractCommands::List => {
            let manifest = forge::contract::load_manifest()?;
            let mut human = format!(
                "{:<18} {:<28} {:<10} {}",
                "Module", "Constant", "Version", "Family"
            );
            for spec in forge::contract::CONTRACTS {
                human.push_str(&format!(
                    "\n{:<18} {:<28} {:<10} {}",
                    spec.module,
                    spec.constant,
                    spec.version,
                    spec.platform_family.unwrap_or("-")
                ));
            }
            human.push_str(&format!(
                "\nsource: {}@{}",
                manifest.source,
                &manifest.revision[..12.min(manifest.revision.len())]
            ));
            let json = serde_json::json!({
                "contracts": forge::contract::CONTRACTS.iter().map(|c| serde_json::json!({
                    "module": c.module, "constant": c.constant, "discriminator": c.discriminator,
                    "version": c.version, "platform_family": c.platform_family, "doc": c.doc
                })).collect::<Vec<_>>(),
                "manifest": manifest,
            });
            Ok(as_output(format, human, json))
        }
        ContractCommands::Inspect { family } => {
            let manifest = forge::contract::load_manifest()?;
            let entry = manifest.files.iter().find(|f| {
                let fam_file = family.strip_prefix("platform.").unwrap_or(family);
                f.path.contains(fam_file)
            });
            let schema_path = forge::contract::family_schema_path(family).ok_or_else(|| {
                ForgeError::ContractInvalid {
                    reason: format!(
                        "unknown family '{family}'; supported: {}",
                        forge::contract::supported_families().join(", ")
                    ),
                }
            })?;
            let schema_text =
                std::fs::read_to_string(forge::contract::contracts_dir().join(schema_path))
                    .map_err(|e| ForgeError::ContractInvalid {
                        reason: format!("cannot read schema {schema_path}: {e}"),
                    })?;
            let schema: serde_json::Value =
                serde_json::from_str(&schema_text).map_err(|e| ForgeError::ContractInvalid {
                    reason: format!("invalid schema: {e}"),
                })?;
            let required = schema
                .get("required")
                .and_then(|v| v.as_array())
                .map(|a| a.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>())
                .unwrap_or_default();
            let human = format!(
                "family: {family}\ncontract: {}/0.1.0\nschema: {schema_path}\nrequired: {}\n{}",
                family,
                required.join(", "),
                schema_text.lines().take(20).collect::<Vec<_>>().join("\n")
            );
            let _ = entry;
            let json =
                serde_json::json!({"family": family, "schema": schema, "manifest": manifest});
            Ok(as_output(format, human, json))
        }
        ContractCommands::Emit { family, target } => {
            let doc = match family.as_str() {
                "platform.gate-result" => {
                    let (dir, pid) = resolve_gate_target(db_path, target)?;
                    forge::contract::emit_gate_result(&dir, &pid)?
                }
                "platform.readiness" => {
                    let docs = forge::contract::emit_readiness(Path::new(target), None)?;
                    if docs.is_empty() {
                        return Err(ForgeError::ContractInvalid {
                            reason: "no readiness data".to_string(),
                        });
                    }
                    docs.into_iter().next().unwrap()
                }
                "platform.release-evidence" => {
                    let dir = Path::new(target);
                    let (manifest, _) = forge::core::manifest::Manifest::load_from_dir(dir, None)
                        .map_err(|e| ForgeError::ContractInvalid {
                        reason: e.to_string(),
                    })?;
                    let pid = manifest.project.id.clone();
                    let releases =
                        forge::release::engine::list_releases(dir, &pid).map_err(|e| {
                            ForgeError::ContractInvalid {
                                reason: e.to_string(),
                            }
                        })?;
                    let latest = releases
                        .first()
                        .ok_or_else(|| ForgeError::ContractInvalid {
                            reason: "no release state for this project".to_string(),
                        })?;
                    forge::contract::emit_release_evidence(dir, &pid, &latest.release_id)?
                }
                "platform.capability" => {
                    let dir = Path::new(target);
                    let (manifest, _) = forge::core::manifest::Manifest::load_from_dir(dir, None)
                        .map_err(|e| ForgeError::ContractInvalid {
                        reason: e.to_string(),
                    })?;
                    let pid = manifest.project.id.clone();
                    forge::contract::emit_capability(dir, &pid)?.ok_or_else(|| {
                        ForgeError::ContractInvalid {
                            reason: "no capabilities block in .project.json".to_string(),
                        }
                    })?
                }
                "platform.audit-event" => {
                    let docs = forge::contract::emit_audit_events(db_path, 64)?;
                    if docs.is_empty() {
                        return Err(ForgeError::ContractInvalid {
                            reason: "no audit events to emit".to_string(),
                        });
                    }
                    docs.into_iter().next().unwrap()
                }
                "platform.job-outcome" => {
                    return Err(ForgeError::ContractInvalid {
                        reason: "platform.job-outcome has no source record in this release"
                            .to_string(),
                    });
                }
                _ => {
                    return Err(ForgeError::ContractInvalid {
                        reason: format!(
                            "unknown family '{family}'; supported: {}",
                            forge::contract::supported_families().join(", ")
                        ),
                    })
                }
            };
            forge::contract::validate_envelope(&doc)
                .map_err(|e| ForgeError::ContractInvalid { reason: e })?;
            let human = serde_json::to_string_pretty(&doc).unwrap_or_else(|_| doc.to_string());
            Ok(as_output(format, human, doc))
        }
        ContractCommands::Validate { file } => {
            let text = if file == "-" {
                use std::io::Read;
                let mut buf = String::new();
                std::io::stdin().read_to_string(&mut buf).map_err(|e| {
                    ForgeError::ContractInvalid {
                        reason: e.to_string(),
                    }
                })?;
                buf
            } else {
                std::fs::read_to_string(file).map_err(|e| ForgeError::ContractInvalid {
                    reason: format!("cannot read {file}: {e}"),
                })?
            };
            let doc: serde_json::Value =
                serde_json::from_str(&text).map_err(|e| ForgeError::ContractInvalid {
                    reason: format!("invalid JSON: {e}"),
                })?;
            forge::contract::validate_envelope(&doc)
                .map_err(|e| ForgeError::ContractInvalid { reason: e })?;
            let human = format!(
                "valid: {}",
                doc.get("contract")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
            );
            let json = serde_json::json!({"valid": true, "contract": doc.get("contract")});
            Ok(as_output(format, human, json))
        }
    }
}
