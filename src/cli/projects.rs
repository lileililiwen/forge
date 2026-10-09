//! Project lifecycle commands (`list`/`inspect`/`register`/`import`/`new`/`doctor`/`check`).
//!
//! Typed CLI handlers for the core project surface plus the shared
//! registry/open/output helpers. Bodies moved verbatim from the split
//! of `src/main.rs`.
//!
//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use forge::checker::{self, PROTOCOL_MAX_ALERTS};
use forge::core::ForgeError;
use forge::doctor::{parse_target_level, render_report_human, run_doctor, RegistryObservation};
use forge::fleet::render_list_block_human;
use forge::generate::{generate, normalize_explicit, parse_interactive, verify_native};
use forge::governance::inspect as inspect_governance_project;
use forge::import::{adopt_import, inspect_import, render_proposal_human};
use forge::policy::{run_driftwatch, DriftWatchConfig};
use forge::registry::Registry;
use std::path::{Path, PathBuf};

use super::feature::{render_record_human, truncate};
use super::fleet_exec::portal_fleet_projection;
use crate::{Format, Output};

pub(super) fn open_registry(db_path: &Path) -> Result<Registry, ForgeError> {
    Registry::open(db_path)
}

pub(super) fn as_output(format: Format, human: String, json: serde_json::Value) -> Output {
    match format {
        Format::Human | Format::Table => Output::Human(human),
        Format::Json | Format::Ndjson => Output::Json(json),
    }
}

pub(crate) fn cmd_list(db_path: &Path, format: Format) -> Result<Output, ForgeError> {
    let registry = open_registry(db_path)?;
    let projects = registry.list()?;
    // Optional read-only fleet block: present only when a workspace
    // registry is configured. An unconfigured surface stays byte-
    // identical to the local list, and a configured-but-unobservable
    // registry renders the typed failure rather than masking it.
    let fleet = portal_fleet_projection(&registry);
    let human_base = if projects.is_empty() {
        "No projects registered.".to_string()
    } else {
        let mut human = format!(
            "{:<20} {:<12} {:<7} {}",
            "Project", "Stack", "Level", "Health"
        );
        for p in &projects {
            human.push_str(&format!(
                "\n{:<20} {:<12} {:<7} {}",
                truncate(&p.id, 20),
                truncate(p.stack.as_deref().unwrap_or("unknown"), 12),
                p.maturity.as_deref().unwrap_or("unknown"),
                p.health(),
            ));
        }
        human
    };
    let json_base = if projects.is_empty() {
        serde_json::json!({"projects": []})
    } else {
        serde_json::json!({"projects": projects})
    };
    let (human, json) = match &fleet {
        None => (human_base, json_base),
        Some(Ok(report)) => {
            let mut json = json_base;
            json["fleet"] = serde_json::to_value(report).map_err(|err| ForgeError::Registry {
                reason: err.to_string(),
            })?;
            (human_base + &render_list_block_human(report), json)
        }
        Some(Err(err)) => {
            let line = format!("\nfleet: unavailable — error[{}]: {}", err.code(), err);
            let json = serde_json::json!({
                "projects": projects,
                "fleet_error": {"code": err.code(), "message": err.to_string()},
            });
            (human_base + &line, json)
        }
    };
    Ok(as_output(format, human, json))
}

pub(crate) fn cmd_inspect(
    db_path: &Path,
    target: &str,
    format: Format,
) -> Result<Output, ForgeError> {
    let registry = open_registry(db_path)?;
    let p = registry.inspect(target)?;
    let human = render_record_human(&p);
    let json = serde_json::to_value(&p).map_err(|err| ForgeError::Registry {
        reason: err.to_string(),
    })?;
    Ok(as_output(format, human, json))
}

pub(crate) fn cmd_register(
    db_path: &Path,
    path: &Path,
    manifest: Option<&std::path::Path>,
    format: Format,
) -> Result<Output, ForgeError> {
    let mut registry = open_registry(db_path)?;
    let p = registry.register(path, manifest)?;
    let human = format!("registered {} ({})", p.id, p.path);
    let json = serde_json::json!({"registered": p});
    Ok(as_output(format, human, json))
}

pub(crate) fn cmd_import(
    db_path: &Path,
    path: &Path,
    profile: Option<&str>,
    accept: bool,
    id: Option<&str>,
    format: Format,
) -> Result<Output, ForgeError> {
    if accept {
        let mut registry = open_registry(db_path)?;
        let record = adopt_import(&mut registry, path, profile, id)?;
        let human = format!("imported {} ({})", record.id, record.path);
        let json = serde_json::json!({"imported": record});
        return Ok(as_output(format, human, json));
    }
    let proposal = inspect_import(path, profile)?;
    let human = render_proposal_human(&proposal);
    let json = serde_json::json!({"proposal": proposal});
    Ok(as_output(format, human, json))
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn cmd_new(
    db_path: &Path,
    path: &Path,
    profile: Option<&str>,
    id: Option<&str>,
    name: Option<&str>,
    features: &[String],
    verify_native_flag: bool,
    workspace_metadata: bool,
    standard_pack: Option<&str>,
    kit_exception: Option<&str>,
    format: Format,
) -> Result<Output, ForgeError> {
    let mut request = if profile.is_some() {
        normalize_explicit(profile, id, name, features, path, standard_pack)?
    } else {
        let stdin = std::io::stdin();
        let mut reader = std::io::BufReader::new(stdin.lock());
        let mut writer = std::io::stderr();
        parse_interactive(
            &mut reader,
            &mut writer,
            path,
            profile,
            id,
            name,
            features,
            standard_pack,
        )?
    };
    request.workspace_metadata = workspace_metadata;
    // An explicit `--kit-exception` is explicit input on both paths, so the
    // interactive path carries it too and stays byte-equivalent to the
    // flag-driven one. It is never prompted for: an exception is never
    // inferred, defaulted or generated.
    request.kit_exception = kit_exception.map(str::to_string);
    let mut registry = open_registry(db_path)?;
    let mut generated = generate(&mut registry, &request)?;
    if verify_native_flag {
        let report = verify_native(&request.profile, &request.destination, None)?;
        generated.native_verified = report.verified;
        generated.native_note = format!(
            "native build '{}' and test '{}' succeeded for profile '{}'",
            report.build_command, report.test_command, report.profile
        );
    }
    let mut human = format!(
        "created {} ({}) from {}@{}\nfiles: {}\n{}",
        generated.record.id,
        generated.record.path,
        request.profile,
        forge::generate::GENERATOR_VERSION,
        generated.files.join(", "),
        generated.native_note
    );
    for note in &generated.notes {
        human.push_str(&format!("\n{note}"));
    }
    if let Some(warning) = &generated.kit_warning {
        human.push_str(&format!("\n{warning}"));
    }
    let mut json = serde_json::json!({
        "created": generated.record,
        "profile": request.profile,
        "generator": forge::generate::GENERATOR_VERSION,
        "files": generated.files,
        "native_verified": generated.native_verified,
        "native_note": generated.native_note,
    });
    if !generated.notes.is_empty() {
        json["notes"] = serde_json::json!(generated.notes);
    }
    if let Some(warning) = &generated.kit_warning {
        json["kit_warning"] = serde_json::json!(warning);
    }
    Ok(as_output(format, human, json))
}

pub(crate) fn cmd_doctor(
    db_path: &Path,
    path: &Path,
    target: Option<&str>,
    format: Format,
) -> Result<Output, ForgeError> {
    let level = match target {
        Some(raw) => Some(parse_target_level(raw)?),
        None => None,
    };
    // Registry is consulted read-only for observation freshness; a project
    // that is unknown there is still assessed from local evidence.
    let observation = open_registry(db_path)
        .ok()
        .and_then(|registry| observation_for(registry, path));
    // DriftWatch execution is delegated to the policy adapter. The CLI
    // runs the adapter itself (rather than going through the registry)
    // so the project-scoped invocation cannot leak across the open
    // registry. The adapter is configured through environment
    // variables; an absent or non-functional binary surfaces as an
    // `unavailable` finding instead of a hard error.
    let policy_outcome = run_driftwatch(path, &DriftWatchConfig::from_env());
    let report = run_doctor(path, level, observation.as_ref(), Some(&policy_outcome))?;
    let human = render_report_human(&report);
    let json = serde_json::json!({"doctor": report});
    Ok(as_output(format, human, json))
}

/// Best-effort read-only registry observation for `path`: matches the
/// registered record whose canonical path equals `path`, if any.
fn observation_for(registry: Registry, path: &Path) -> Option<RegistryObservation> {
    let canonical = path.canonicalize().ok()?.display().to_string();
    let record = registry
        .list()
        .ok()?
        .into_iter()
        .find(|p| p.path == canonical)?;
    Some(RegistryObservation {
        registered: true,
        observed_at: Some(record.observed_at),
    })
}

/// Render the external-checker document for one registered project.
///
/// The command exists for machines (Driftwatchdog checker protocol):
/// stdout carries exactly one compact JSON document, diagnostics go to
/// stderr, findings never change the exit code, and nothing about the
/// run is journaled or persisted. Both output formats print the
/// identical document, so `--format json` cannot alter checker stdout.
pub(crate) fn cmd_check(
    db_path: &Path,
    target: &str,
    include_policy: bool,
    max_alerts: u32,
) -> Result<Output, ForgeError> {
    if max_alerts == 0 || max_alerts as usize > PROTOCOL_MAX_ALERTS {
        return Err(ForgeError::CheckInvalid {
            reason: format!(
                "--max-alerts {max_alerts} is outside the bounded range 1..={PROTOCOL_MAX_ALERTS}"
            ),
        });
    }
    // Resolve through the registry so an unregistered target fails with
    // the typed unknown-project error before anything is written to
    // stdout; a partial document must never be parseable by the checker.
    let registry = open_registry(db_path)?;
    let record = registry.inspect(target)?;
    let dir = PathBuf::from(&record.path);
    if !dir.is_dir() {
        return Err(ForgeError::PathUnavailable {
            path: record.path.clone(),
        });
    }
    let observation = Some(RegistryObservation {
        registered: true,
        observed_at: Some(record.observed_at),
    });
    // The DriftWatch policy plane stays off by default: a checker run
    // must not drive DriftWatch, which may drive Forge back through its
    // checker registration. Only an explicit --include-policy invokes
    // the adapter; without it the policy plane contributes no findings.
    let policy_outcome = if include_policy {
        Some(run_driftwatch(&dir, &DriftWatchConfig::from_env()))
    } else {
        None
    };
    let report = run_doctor(&dir, None, observation.as_ref(), policy_outcome.as_ref())?;
    // The governance plane is evaluated read-only (no observation is
    // persisted); a plane that cannot be evaluated projects into a
    // warning naming the gap instead of a hard failure.
    let governance = inspect_governance_project(&dir);
    let document = checker::build_document(
        &dir,
        &report,
        &governance,
        &checker::now_rfc3339(),
        max_alerts as usize,
    );
    Ok(Output::Human(checker::render_document(&document)?))
}
