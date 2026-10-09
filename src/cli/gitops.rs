//! GitOps commands (`test`/`commit`/`mcp`/`push`/`mirror`).
//!
//! Typed CLI handlers for the gitops surface. Bodies moved verbatim
//! from the split of `src/main.rs`.
//!
//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use forge::core::ForgeError;
use forge::distribution::{
    apply_mirror, distribution_config_from_manifest, plan_mirror, DistributionConfig, MirrorRequest,
};
use forge::gitops::{commit_paths, push_ref, run_test, CommitOutcome, PushOutcome, TestOutcome};
use std::path::Path;

use super::commands_services::McpCommands;
use super::projects::{as_output, open_registry};
use crate::{Format, Output};

pub(crate) fn cmd_test(path: &Path, format: Format) -> Result<Output, ForgeError> {
    let canonical = path
        .canonicalize()
        .map_err(|_| ForgeError::PathUnavailable {
            path: path.display().to_string(),
        })?;
    let (manifest, _) = forge::core::manifest::Manifest::load_from_dir(&canonical, None)?;
    let descriptor = forge::profile::inspect_profile(&manifest.project.profile)?;
    let test_command = descriptor.test_command.clone();
    let outcome = run_test(
        &manifest.project.id,
        &manifest.project.profile,
        Some(&test_command),
        &canonical,
    )?;
    test_output(&outcome, format)
}

fn test_output(outcome: &TestOutcome, format: Format) -> Result<Output, ForgeError> {
    let json = serde_json::json!({
        "test": {
            "contract": outcome.contract,
            "project_id": outcome.project_id,
            "profile": outcome.profile,
            "command": outcome.command,
            "status": outcome.status,
            "exit_code": outcome.exit_code,
            "note": outcome.note,
            "evidence": outcome.evidence,
        }
    });
    let human = format!(
        "project: {}\nprofile: {}\ncommand: {}\nstatus: {}\nexit_code: {}\nnote: {}\nevidence: {}",
        outcome.project_id,
        outcome.profile,
        outcome.command,
        outcome.status,
        outcome
            .exit_code
            .map(|c| c.to_string())
            .unwrap_or_else(|| "(none)".to_string()),
        outcome.note,
        if outcome.evidence.is_empty() {
            "(none)".to_string()
        } else {
            outcome.evidence.join(" | ")
        }
    );
    Ok(as_output(format, human, json))
}

pub(crate) fn cmd_commit(
    path: &Path,
    paths: Vec<String>,
    message: String,
    format: Format,
) -> Result<Output, ForgeError> {
    let canonical = path
        .canonicalize()
        .map_err(|_| ForgeError::PathUnavailable {
            path: path.display().to_string(),
        })?;
    let (manifest, _) = forge::core::manifest::Manifest::load_from_dir(&canonical, None)?;
    let outcome = commit_paths(&manifest.project.id, &canonical, &paths, &message)?;
    commit_output(&outcome, format)
}

fn commit_output(outcome: &CommitOutcome, format: Format) -> Result<Output, ForgeError> {
    let json = serde_json::json!({
        "commit": {
            "contract": outcome.contract,
            "project_id": outcome.project_id,
            "paths": outcome.paths,
            "message": outcome.message,
            "commit_sha": outcome.commit_sha,
            "files_changed": outcome.files_changed,
            "note": outcome.note,
            "evidence": outcome.evidence,
        }
    });
    let human = format!(
        "project: {}\npaths: {}\nfiles_changed: {}\ncommit: {}\nnote: {}",
        outcome.project_id,
        outcome.paths.join(", "),
        outcome.files_changed.join(", "),
        outcome
            .commit_sha
            .clone()
            .unwrap_or_else(|| "(none)".to_string()),
        outcome.note
    );
    Ok(as_output(format, human, json))
}

pub(crate) fn cmd_mcp(db_path: &Path, command: &McpCommands) -> Result<Output, ForgeError> {
    match command {
        McpCommands::Serve => {
            forge::mcp::serve_stdio(Some(db_path)).map_err(|err| ForgeError::McpInvalid {
                reason: err.to_string(),
            })?;
            Ok(Output::Human("mcp server exited cleanly".to_string()))
        }
    }
}

pub(crate) fn cmd_push(
    path: &Path,
    remote: String,
    ref_name: String,
    confirm: bool,
    format: Format,
) -> Result<Output, ForgeError> {
    let canonical = path
        .canonicalize()
        .map_err(|_| ForgeError::PathUnavailable {
            path: path.display().to_string(),
        })?;
    let (manifest, _) = forge::core::manifest::Manifest::load_from_dir(&canonical, None)?;
    let outcome = push_ref(
        &manifest.project.id,
        &canonical,
        &remote,
        &ref_name,
        confirm,
    )?;
    push_output(&outcome, format)
}

fn push_output(outcome: &PushOutcome, format: Format) -> Result<Output, ForgeError> {
    let json = serde_json::json!({
        "push": {
            "contract": outcome.contract,
            "project_id": outcome.project_id,
            "remote": outcome.remote,
            "ref": outcome.ref_name,
            "status": outcome.status,
            "commit_sha": outcome.commit_sha,
            "note": outcome.note,
            "evidence": outcome.evidence,
        }
    });
    let human = format!(
        "project: {}\nremote: {}\nref: {}\nstatus: {}\ncommit: {}\nnote: {}",
        outcome.project_id,
        outcome.remote,
        outcome.ref_name,
        outcome.status,
        outcome
            .commit_sha
            .clone()
            .unwrap_or_else(|| "(none)".to_string()),
        outcome.note
    );
    Ok(as_output(format, human, json))
}

pub(crate) fn cmd_mirror(
    db_path: &Path,
    target: &str,
    refs: Vec<String>,
    confirm: bool,
    dry_run: bool,
    retry_failed: bool,
    format: Format,
) -> Result<Output, ForgeError> {
    let candidate = Path::new(target);
    if !candidate.is_dir() {
        return Err(ForgeError::PathUnavailable {
            path: target.to_string(),
        });
    }
    let canonical = candidate
        .canonicalize()
        .map_err(|_| ForgeError::PathUnavailable {
            path: target.to_string(),
        })?;
    let (manifest, _) = forge::core::manifest::Manifest::load_from_dir(&canonical, None)?;
    let config: DistributionConfig = distribution_config_from_manifest(&manifest)?;
    let request = MirrorRequest {
        project_id: manifest.project.id.clone(),
        refs,
        confirm,
        dry_run,
        retry_failed,
    };
    let report = if dry_run {
        let state_path = forge::distribution::state_path_for(&canonical, &manifest.project.id)?;
        let state = forge::distribution::load_mirror_state(&state_path)?;
        plan_mirror(&config, &request, &state, &state_path)?
    } else {
        apply_mirror(&canonical, &config, &request)?
    };
    let registry = open_registry(db_path)?;
    let detail = format!(
        "mirror `{}` refs={} primary={} healthy={} dry_run={} retry={}",
        report.project_id,
        report.refs.join(","),
        report
            .primary
            .clone()
            .unwrap_or_else(|| "(none)".to_string()),
        report.healthy,
        report.dry_run,
        report.retry_failed
    );
    let state_label = if report.healthy() { "done" } else { "partial" };
    let _ = registry.record_operation("mirror", &report.project_id, state_label, &detail);
    let output = mirror_output(&report, format)?;
    if report.healthy() {
        Ok(output)
    } else {
        // Partial failure: print the per-remote outcome
        // JSON to stdout so the caller sees exactly which
        // remotes succeeded and which failed, then surface
        // a typed exit-code error referencing the
        // distribution verdict. The error is rendered on
        // stderr by the generic error path so the exit
        // code matches the verdict without hiding the
        // per-remote evidence.
        match &output {
            Output::Human(text) => println!("{text}"),
            Output::Json(value) => {
                println!("{}", serde_json::to_string_pretty(value).unwrap());
            }
            Output::Raw(text) => print!("{text}"),
        }
        Err(ForgeError::DistributionInvalid {
            reason: report.note.clone(),
        })
    }
}

fn mirror_output(
    report: &forge::distribution::MirrorReport,
    format: Format,
) -> Result<Output, ForgeError> {
    let json = serde_json::json!({"mirror": report});
    let human = forge::distribution::render_report_human(report);
    Ok(as_output(format, human, json))
}
