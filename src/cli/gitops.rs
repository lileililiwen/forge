//! GitOps commands (`test`/`commit`/`mcp`/`push`/`mirror`).
//!
//! Typed CLI handlers for the gitops surface. Bodies moved verbatim
//! from the split of `src/main.rs`.
//!
//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use forge::build::{run_build, BuildOutcome};
use forge::core::ForgeError;
use forge::distribution::{
    apply_mirror, distribution_config_from_manifest, plan_mirror, DistributionConfig, MirrorRequest,
};
use forge::gitops::{commit_paths, push_ref, run_test, CommitOutcome, PushOutcome, TestOutcome};
use forge::image::{run_image_build, verify_image_build, ImageBuildOutcome};
use forge::target::request_from_flags;
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

/// `forge build`: the profile build, a target-addressed build, or a
/// delegated image build.
#[derive(Debug, clap::Args)]
pub(crate) struct BuildArgs {
    /// Project directory (default: current directory).
    #[arg(default_value = ".")]
    pub path: std::path::PathBuf,
    /// Target operating system (requires --arch).
    #[arg(long, value_name = "OS")]
    pub os: Option<String>,
    /// Target architecture (requires --os).
    #[arg(long, value_name = "ARCH")]
    pub arch: Option<String>,
    /// Target runtime identifier (dotnet toolchains only).
    #[arg(long, value_name = "RID")]
    pub rid: Option<String>,
    /// Build a container image through the configured image-builder
    /// adapter instead of the profile's build command.
    #[arg(long)]
    pub image: bool,
    /// Image reference for `--image` (default `forge-<project-id>:<sha12>`).
    #[arg(long, value_name = "REF")]
    pub tag: Option<String>,
    /// Image-builder adapter binary (default $FORGE_IMAGE_BUILDER_BIN or
    /// `forge-image-builder`).
    #[arg(long = "builder-bin", value_name = "PATH")]
    pub builder_bin: Option<String>,
    /// Verify the image build record for the current revision instead of
    /// building. Read-only; starts no builder.
    #[arg(long = "verify-image")]
    pub verify_image: bool,
}

impl BuildArgs {
    pub(crate) fn run(&self, format: Format) -> Result<Output, ForgeError> {
        cmd_build(
            self.path.as_path(),
            self.os.clone(),
            self.arch.clone(),
            self.rid.clone(),
            self.image,
            self.tag.clone(),
            self.builder_bin.clone(),
            self.verify_image,
            format,
        )
    }
}

pub(crate) fn cmd_build(
    path: &Path,
    os: Option<String>,
    arch: Option<String>,
    rid: Option<String>,
    image: bool,
    tag: Option<String>,
    builder_bin: Option<String>,
    verify_image: bool,
    format: Format,
) -> Result<Output, ForgeError> {
    let canonical = path
        .canonicalize()
        .map_err(|_| ForgeError::PathUnavailable {
            path: path.display().to_string(),
        })?;
    let (manifest, _) = forge::core::manifest::Manifest::load_from_dir(&canonical, None)?;
    if verify_image {
        if image
            || tag.is_some()
            || builder_bin.is_some()
            || os.is_some()
            || arch.is_some()
            || rid.is_some()
        {
            return Err(ForgeError::BuildInvalid {
                reason: "`--verify-image` verifies one recorded build; it does not accept \
                         `--image`, `--tag`, `--builder-bin`, `--os`, `--arch` or `--rid`"
                    .to_string(),
            });
        }
        let record = verify_image_build(&manifest.project.id, &canonical)?;
        return image_verify_output(&record, format);
    }
    if !image {
        if tag.is_some() || builder_bin.is_some() {
            return Err(ForgeError::BuildInvalid {
                reason: "`--tag` and `--builder-bin` describe an image build; pass `--image`"
                    .to_string(),
            });
        }
        let descriptor = forge::profile::inspect_profile(&manifest.project.profile)?;
        let build_command = descriptor.build_command.clone();
        let request = request_from_flags(os.as_deref(), arch.as_deref(), rid.as_deref())?;
        let outcome = run_build(
            &manifest.project.id,
            &manifest.project.profile,
            &build_command,
            &canonical,
            &request,
        )?;
        return build_output(&outcome, format);
    }
    if os.is_some() || arch.is_some() || rid.is_some() {
        return Err(ForgeError::BuildInvalid {
            reason: "an image build addresses its platform through the builder; `--os`, `--arch` \
                     and `--rid` are not accepted with `--image`"
                .to_string(),
        });
    }
    let outcome = run_image_build(
        &manifest.project.id,
        &canonical,
        tag.as_deref(),
        builder_bin.as_deref(),
    )?;
    image_output(&outcome, format)
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

fn build_output(outcome: &BuildOutcome, format: Format) -> Result<Output, ForgeError> {
    let json = serde_json::json!({
        "build": {
            "contract": outcome.contract,
            "project_id": outcome.project_id,
            "profile": outcome.profile,
            "command": outcome.command,
            "target": outcome.target,
            "status": outcome.status,
            "exit_code": outcome.exit_code,
            "note": outcome.note,
            "evidence": outcome.evidence,
        }
    });
    let human = format!(
        "project: {}\nprofile: {}\ncommand: {}\ntarget: {}\nstatus: {}\nexit_code: {}\nnote: {}\nevidence: {}",
        outcome.project_id,
        outcome.profile,
        outcome.command,
        outcome.target,
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

fn image_build_json(outcome: &ImageBuildOutcome) -> serde_json::Value {
    serde_json::json!({
        "contract": outcome.contract,
        "project_id": outcome.project_id,
        "revision": outcome.revision,
        "tag": outcome.tag,
        "builder": outcome.builder,
        "builder_version": outcome.builder_version,
        "digest": outcome.digest,
        "sbom": outcome.sbom,
        "evidence": outcome.evidence,
        "observed_at": outcome.observed_at,
    })
}

fn image_output(outcome: &ImageBuildOutcome, format: Format) -> Result<Output, ForgeError> {
    let json = serde_json::json!({ "image_build": image_build_json(outcome) });
    let mut human = format!(
        "project: {}\nrevision: {}\ntag: {}\nbuilder: {}\ndigest: {}\nobserved_at: {}",
        outcome.project_id,
        outcome.revision,
        outcome.tag,
        outcome.builder,
        outcome.digest,
        outcome.observed_at
    );
    if let Some(version) = outcome.builder_version.as_deref() {
        human.push_str(&format!("\nbuilder_version: {version}"));
    }
    if let Some(sbom) = outcome.sbom.as_ref() {
        human.push_str(&format!(
            "\nsbom: {} ({} bytes, sha256:{})",
            sbom.path, sbom.bytes, sbom.sha256
        ));
    }
    if !outcome.evidence.is_empty() {
        human.push_str(&format!("\nevidence: {}", outcome.evidence.join(" | ")));
    }
    Ok(as_output(format, human, json))
}

fn image_verify_output(record: &ImageBuildOutcome, format: Format) -> Result<Output, ForgeError> {
    let json = serde_json::json!({
        "image_build": image_build_json(record),
        "verification": {
            "verified": true,
            "revision": record.revision,
            "digest": record.digest,
        },
    });
    let mut human = format!(
        "verified: yes\nproject: {}\nrevision: {}\ntag: {}\nbuilder: {}\ndigest: {}",
        record.project_id, record.revision, record.tag, record.builder, record.digest
    );
    if let Some(sbom) = record.sbom.as_ref() {
        human.push_str(&format!("\nsbom: {} (sha256:{})", sbom.path, sbom.sha256));
    }
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
        "mirror `{}` refs={} primary={} verdict={} healthy={} dry_run={} retry={}",
        report.project_id,
        report.refs.join(","),
        report
            .primary
            .clone()
            .unwrap_or_else(|| "(none)".to_string()),
        report.verdict.label(),
        report.healthy,
        report.dry_run,
        report.retry_failed
    );
    // The journal state label follows the verdict: a plan is
    // journaled as a plan, never as a completed mirror.
    let state_label = match report.verdict {
        forge::distribution::MirrorVerdict::Delivered => "done",
        forge::distribution::MirrorVerdict::Partial => "partial",
        forge::distribution::MirrorVerdict::Planned => "planned",
    };
    let _ = registry.record_operation("mirror", &report.project_id, state_label, &detail);
    let output = mirror_output(&report, format)?;
    match report.verdict {
        forge::distribution::MirrorVerdict::Delivered
        | forge::distribution::MirrorVerdict::Planned => Ok(output),
        forge::distribution::MirrorVerdict::Partial => {
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
}

fn mirror_output(
    report: &forge::distribution::MirrorReport,
    format: Format,
) -> Result<Output, ForgeError> {
    let json = serde_json::json!({"mirror": report});
    let human = forge::distribution::render_report_human(report);
    Ok(as_output(format, human, json))
}
