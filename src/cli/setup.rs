//! Project setup commands (`workspace`/`graduation`/`kit`/`profile`).
//!
//! Typed CLI handlers for workspace onboarding, graduation previews,
//! kit packing and profile inspection. Bodies moved verbatim from the
//! split of `src/main.rs`.
//!
//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use forge::core::ForgeError;
use forge::graduation::{
    adopt_graduation, build_proposal, render_preview_human, GraduationImport, GraduationPreview,
    GraduationRefusal, GRADUATION_CONTRACT_VERSION,
};
use forge::import::{sync_workspace, WORKSPACE_SYNC_CONTRACT};
use forge::profile::{inspect_profile, list_profiles, preflight_profile, resolve_profile};
use std::path::Path;
use std::process::ExitCode;

use super::commands::{GraduationCommands, WorkspaceCommands};
use super::commands_ops::{KitCommands, ProfileCommands};
use super::feature::{render_error, render_profile_human};
use crate::{Format, Output};

use super::projects::{as_output, open_registry};
pub(crate) fn cmd_workspace(
    db_path: &Path,
    command: &WorkspaceCommands,
    format: Format,
) -> ExitCode {
    match command {
        WorkspaceCommands::Sync { root } => cmd_workspace_sync(db_path, root, format),
    }
}

/// Converge one workspace root into the registry and report every
/// directory. The report always prints (human table or the versioned JSON
/// envelope); the exit code is 0 iff no directory failed.
fn cmd_workspace_sync(db_path: &Path, root: &Path, format: Format) -> ExitCode {
    // A missing or non-directory ROOT is refused before the registry is
    // even opened, so the failed run touches nothing.
    if !root.is_dir() {
        let err = ForgeError::PathUnavailable {
            path: root.display().to_string(),
        };
        render_error(&err, format);
        return ExitCode::from(err.exit_code() as u8);
    }
    let mut registry = match open_registry(db_path) {
        Ok(registry) => registry,
        Err(err) => {
            render_error(&err, format);
            return ExitCode::from(err.exit_code() as u8);
        }
    };
    let report = match sync_workspace(&mut registry, root) {
        Ok(report) => report,
        Err(err) => {
            render_error(&err, format);
            return ExitCode::from(err.exit_code() as u8);
        }
    };
    match format {
        Format::Human | Format::Table => {
            println!("{}", render_workspace_sync_human(&report));
        }
        Format::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(&workspace_sync_json(&report)).unwrap()
            );
        }
        Format::Ndjson => {
            println!(
                "{}",
                serde_json::to_string(&workspace_sync_json(&report)).unwrap()
            );
        }
    }
    if report.failed() {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

/// One `ok|already|skipped|failed <leaf> [<detail>]` line per directory in
/// sorted order, then the summary count line.
fn render_workspace_sync_human(report: &forge::import::WorkspaceSyncReport) -> String {
    let mut lines = Vec::with_capacity(report.entries.len() + 1);
    for entry in &report.entries {
        let detail = match entry.outcome.as_str() {
            "ok" | "already" => entry.id.clone().unwrap_or_default(),
            "skipped" if entry.reason.as_deref() == Some("ambiguous") => {
                "ambiguous (re-run one directory with --profile)".to_string()
            }
            _ => entry.reason.clone().unwrap_or_default(),
        };
        if detail.is_empty() {
            lines.push(entry.outcome.clone() + " " + &entry.directory);
        } else {
            lines.push(format!("{} {} {detail}", entry.outcome, entry.directory));
        }
    }
    lines.push(format!(
        "synced {}, already {}, skipped {}, failed {}",
        report.summary.ok, report.summary.already, report.summary.skipped, report.summary.failed
    ));
    lines.join("\n")
}

fn workspace_sync_json(report: &forge::import::WorkspaceSyncReport) -> serde_json::Value {
    serde_json::json!({
        "contract": WORKSPACE_SYNC_CONTRACT,
        "root": report.root,
        "entries": report.entries,
        "summary": report.summary,
    })
}

/// Map a graduation refusal to the single typed Core error. The
/// refusal code is part of the reason so a caller sees both the class
/// (`graduation-not-validated`) and the named field.
fn graduation_invalid(refusal: GraduationRefusal) -> ForgeError {
    ForgeError::GraduationInvalid {
        reason: format!("{}: {}", refusal.code, refusal.detail),
    }
}

/// Read, decode and validate an artifact. This is the only path into
/// the graduation surface, so `preview`, the dry run and the confirmed
/// import all apply the same gate.
fn load_graduation(artifact: &str) -> Result<GraduationImport, ForgeError> {
    use forge::graduation::{parse_artifact, read_artifact, validate_graduation};
    let raw = read_artifact(artifact)?;
    let record = parse_artifact(&raw).map_err(graduation_invalid)?;
    validate_graduation(&record).map_err(graduation_invalid)
}

fn render_preview_output(
    preview: &GraduationPreview,
    format: Format,
) -> Result<Output, ForgeError> {
    let human = render_preview_human(preview);
    let proposed = match &preview.proposal {
        Some(proposal) => serde_json::to_value(proposal).map_err(|err| ForgeError::Registry {
            reason: err.to_string(),
        })?,
        None => serde_json::Value::Null,
    };
    let json = serde_json::json!({
        "contract": GRADUATION_CONTRACT_VERSION,
        "preview": {
            "artifact": preview.artifact,
            "source": preview.import.source,
            "brief": preview.import.brief,
            "experiment": preview.import.experiment,
            "proposed": proposed,
        },
    });
    Ok(as_output(format, human, json))
}

pub(crate) fn cmd_graduation(
    db_path: &Path,
    command: &GraduationCommands,
    format: Format,
) -> Result<Output, ForgeError> {
    match command {
        GraduationCommands::Preview { artifact } => cmd_graduation_preview(artifact, format),
        GraduationCommands::Import {
            artifact,
            path,
            profile,
            id,
            actor,
            confirm,
        } => cmd_graduation_import(
            db_path,
            artifact,
            path,
            profile,
            id.as_deref(),
            actor,
            *confirm,
            format,
        ),
    }
}

fn cmd_graduation_preview(artifact: &str, format: Format) -> Result<Output, ForgeError> {
    let import = load_graduation(artifact)?;
    let preview = GraduationPreview {
        artifact: artifact.to_string(),
        import,
        proposal: None,
    };
    render_preview_output(&preview, format)
}

#[allow(clippy::too_many_arguments)]
fn cmd_graduation_import(
    db_path: &Path,
    artifact: &str,
    path: &Path,
    profile: &str,
    id: Option<&str>,
    actor: &str,
    confirm: bool,
    format: Format,
) -> Result<Output, ForgeError> {
    let import = load_graduation(artifact)?;
    let proposal = build_proposal(&import, profile, path, id)?;
    if !confirm {
        let preview = GraduationPreview {
            artifact: artifact.to_string(),
            import,
            proposal: Some(proposal),
        };
        return render_preview_output(&preview, format);
    }
    let mut registry = open_registry(db_path)?;
    let now = chrono::Utc::now();
    let adoption = adopt_graduation(&mut registry, &proposal, &import, actor, now)?;
    let human = format!(
        "imported {} ({})\nprofile: {}\nreceipt: {}\nsource: {} hypora_revision={}\nNo scaffold, deploy, network call or gate approval was performed.",
        adoption.record.id,
        adoption.record.path,
        proposal.profile,
        adoption.receipt_path,
        import.source.contract,
        import.source.hypora_revision,
    );
    let json = serde_json::json!({
        "imported": adoption.record,
        "graduation": {
            "contract": GRADUATION_CONTRACT_VERSION,
            "receipt": adoption.receipt_path,
            "source": import.source,
        },
    });
    Ok(as_output(format, human, json))
}

/// Repack or verify the committed shared-layer feed.
///
/// Both commands are deliberately local-only. `pack` reads a sibling checkout
/// and writes only inside this repository; `verify` reads a manifest and the
/// feed it names. Neither touches the network.
pub(crate) fn cmd_kit(command: &KitCommands, format: Format) -> Result<Output, ForgeError> {
    match command {
        KitCommands::Pack { platform_libs } => {
            let report = forge::kit::pack_platform_feed(platform_libs.as_deref())?;
            let mut human = format!(
                "packed {} package(s) for kit {}@{} from {}\n",
                report.packed.len(),
                report.kit,
                report.version,
                report.sibling
            );
            for package in &report.packed {
                human.push_str(&format!("  packed  {package}\n"));
            }
            for package in &report.skipped {
                // A skipped package is reported, never counted as packed: a
                // feed missing a member of the closure does not restore.
                human.push_str(&format!(
                    "  SKIPPED {package} (no project in the sibling)\n"
                ));
            }
            human.push_str(&format!(
                "recorded {} digest(s) in kits/manifest.json\n",
                report.digest_entries
            ));
            for file in &report.removed {
                human.push_str(&format!("  removed {file} (not in the declared set)\n"));
            }
            human.push_str("verify with `forge kit verify`\n");
            let json = serde_json::to_value(&report).map_err(|err| ForgeError::Registry {
                reason: err.to_string(),
            })?;
            Ok(as_output(format, human, json))
        }
        KitCommands::Verify { path } => {
            let descriptor = forge::kit::inspect_kit("platform-dotnet", None)?;
            let trimmed = path.trim();
            let (declared_version, declared_by, feed_dir) = if trimmed.is_empty() {
                // No project given: check Forge's own committed feed against
                // the version the compiled-in descriptor pins — the same value
                // that lands in a generated `forge.yaml` `kit.version`.
                let version = descriptor
                    .reference
                    .version
                    .clone()
                    .unwrap_or_else(|| "none".to_string());
                (
                    version,
                    "the compiled-in kit descriptor".to_string(),
                    forge::kit::assets::kits_dir().join(forge::kit::KITS_FEED_DIR),
                )
            } else {
                let dir = std::path::Path::new(trimmed);
                let manifest_path = dir.join("forge.yaml");
                let bytes =
                    std::fs::read(&manifest_path).map_err(|err| ForgeError::KitFeedIncomplete {
                        reason: format!(
                            "cannot read {}: {err}. A project's own `kit.version` is the declared \
                         version this check must use",
                            manifest_path.display()
                        ),
                    })?;
                let manifest: serde_yaml::Value =
                    serde_yaml::from_slice(&bytes).map_err(|err| {
                        ForgeError::KitFeedIncomplete {
                            reason: format!("{} is not valid YAML: {err}", manifest_path.display()),
                        }
                    })?;
                let (version, feed_path) =
                    forge::kit::declared_kit_version(&manifest).ok_or_else(|| {
                        ForgeError::KitFeedIncomplete {
                            reason: format!(
                            "{} declares no `kit` block, so there is no `kit.version` to check the \
                             committed feed against",
                            manifest_path.display()
                        ),
                        }
                    })?;
                let version = version.ok_or_else(|| ForgeError::KitFeedIncomplete {
                    reason: format!(
                        "{} declares a kit with no `kit.version`; a declared zero has no feed to \
                         check",
                        manifest_path.display()
                    ),
                })?;
                let feed_path = feed_path.ok_or_else(|| ForgeError::KitFeedIncomplete {
                    reason: format!(
                        "{} declares no `kit.feed.path`, so the committed feed's location is \
                         unknown",
                        manifest_path.display()
                    ),
                })?;
                (
                    version,
                    manifest_path.display().to_string(),
                    dir.join(feed_path),
                )
            };
            let report = forge::kit::verify_committed_feed(
                &descriptor,
                &declared_version,
                &declared_by,
                &feed_dir,
            )?;
            let mut human = format!(
                "verified {} feed at {}\nkit: {}@{} (declared by {})\n",
                report.packages.len(),
                report.feed_path,
                report.kit,
                report.declared_version,
                report.declared_by
            );
            for entry in &report.packages {
                human.push_str(&format!(
                    "  {} {} {} ({})\n",
                    entry.state, entry.package, entry.version, entry.role
                ));
            }
            if !report.digests_verified {
                human.push_str(
                    "note: no digest is recorded for this feed's packages, so the check above is \
                     a version check only and not a byte-level verification\n",
                );
            }
            let json = serde_json::to_value(&report).map_err(|err| ForgeError::Registry {
                reason: err.to_string(),
            })?;
            Ok(as_output(format, human, json))
        }
        KitCommands::Upgrade {
            path,
            to,
            confirm,
            force,
        } => {
            let dir = path.as_path();
            if !confirm {
                // Review mode. Read-only, and it is the default: an upgrade is
                // never applied because someone ran a command that looks like
                // it might apply one.
                let plan = forge::kit::diff_kit_snapshot(dir, Some(to))?.map_err(unavailable)?;
                let json = serde_json::to_value(&plan).map_err(|err| ForgeError::Registry {
                    reason: err.to_string(),
                })?;
                let mut human = format!(
                    "kit upgrade plan for {} ({} -> {}); nothing was written\n",
                    plan.path, plan.from, plan.against
                );
                for entry in &plan.entries {
                    let change = serde_json::to_value(entry.change)
                        .ok()
                        .and_then(|v| v.as_str().map(str::to_string))
                        .unwrap_or_else(|| "unknown".to_string());
                    human.push_str(&format!("  {change:<10} {}\n", entry.path));
                }
                if !plan.conflicts.is_empty() {
                    human.push_str(&format!(
                        "refused: {} owned file(s) were edited; re-run with --force to replace \
                         them\n",
                        plan.conflicts.len()
                    ));
                }
                human.push_str("apply with `forge kit upgrade --confirm`\n");
                return Ok(as_output(format, human, json));
            }
            let report = forge::kit::upgrade_kit_snapshot(
                dir,
                to,
                true,
                *force,
                &chrono::Utc::now().to_rfc3339(),
            )?
            .map_err(unavailable)?;
            let mut human = format!("upgraded {} to {}\n", report.path, report.to);
            for path in &report.written {
                human.push_str(&format!("  wrote    {path}\n"));
            }
            for path in &report.forced {
                human.push_str(&format!("  REPLACED {path} (edited owned file, --force)\n"));
            }
            for path in &report.orphaned {
                human.push_str(&format!("  preserved {path} (not in the target kit)\n"));
            }
            human.push_str(&format!(
                "recorded the new pin in {}/forge.yaml; check it with `forge kit verify .`\n",
                report.path
            ));
            let json = serde_json::to_value(&report).map_err(|err| ForgeError::Registry {
                reason: err.to_string(),
            })?;
            Ok(as_output(format, human, json))
        }
    }
}

/// An unavailable upgrade path is a refusal, not a success.
///
/// The library returns it as a value so a caller can choose its own
/// presentation; on the command line the honest presentation is a non-zero
/// exit. Reporting "nothing to do, already current" would be the one thing
/// this state must never become.
pub(super) fn unavailable(err: forge::kit::KitUnavailable) -> ForgeError {
    ForgeError::KitUnknown {
        reason: format!(
            "kit upgrade unavailable: {}. The project still reports its previously pinned kit \
             version and was not reported as current",
            err.reason
        ),
    }
}

pub(crate) fn cmd_profile(command: &ProfileCommands, format: Format) -> Result<Output, ForgeError> {
    match command {
        ProfileCommands::List => {
            let profiles = list_profiles();
            let mut human = format!(
                "{:<16} {:<10} {:<18} {}",
                "Profile", "Version", "Adapter", "Language"
            );
            for p in &profiles {
                human.push_str(&format!(
                    "\n{:<16} {:<10} {:<18} {}",
                    p.id, p.version, p.adapter, p.language
                ));
            }
            let json = serde_json::json!({"profiles": profiles});
            Ok(as_output(format, human, json))
        }
        ProfileCommands::Inspect { id } => {
            let p = inspect_profile(id)?;
            let human = render_profile_human(&p);
            let json = serde_json::to_value(&p).map_err(|err| ForgeError::Registry {
                reason: err.to_string(),
            })?;
            Ok(as_output(format, human, json))
        }
        ProfileCommands::Resolve { id, features } => {
            let resolved = resolve_profile(id, features)?;
            let human = format!(
                "resolved {}@{} via {} (language {}, toolchain {})",
                resolved.id,
                resolved.version,
                resolved.adapter,
                resolved.language,
                resolved.toolchain
            );
            let json = serde_json::json!({"resolved": resolved});
            Ok(as_output(format, human, json))
        }
        ProfileCommands::Preflight { id } => {
            let report = preflight_profile(id, None)?;
            let human = format!(
                "preflight ok: toolchain '{}' available for profile '{}' (version {})",
                report.toolchain, report.id, report.version
            );
            let json = serde_json::json!({"preflight": report});
            Ok(as_output(format, human, json))
        }
    }
}
