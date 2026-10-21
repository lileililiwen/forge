//! The target-addressed build operation.
//!
//! Forge Core builds a project with the build command its profile
//! declares, in the project directory, and reports the outcome: the
//! command that ran, the target the build addressed, the status, the
//! exit code and the captured evidence. A build addressed to a target
//! carries that target through the toolchain's own environment
//! variable; a build without a target runs for the host and says so.
//!
//! A failed build is an error, as a failed test is: `build-failed` with
//! the reason and the evidence. A request Forge cannot honour is
//! refused: `build-invalid` with the reason. A refused or failed build
//! reports no build. A build that overruns the shared runner's
//! wall-clock bound is a failure too: the reason names the timeout
//! rather than an exit status, and the command's whole process group
//! is killed.

pub mod build_tests;

use std::path::Path;

use crate::core::ForgeError;
use crate::native::{run_native, split_command, NATIVE_COMMAND_TIMEOUT};
use crate::profile::inspect_profile;
use crate::target::{resolve_target, TargetRequest, HOST_TARGET};

/// The contract name and version of the build outcome.
pub const BUILD_CONTRACT: &str = "forge-build/0.1.0";

/// The outcome of one build.
#[derive(Debug, Clone, serde::Serialize)]
pub struct BuildOutcome {
    /// The contract name and version.
    pub contract: String,
    /// The project's id.
    pub project_id: String,
    /// The profile the build command came from.
    pub profile: String,
    /// The command that ran.
    pub command: String,
    /// The resolved target, or `host`.
    pub target: String,
    /// The build status: `passed`.
    pub status: String,
    /// The command's exit code.
    pub exit_code: Option<i32>,
    /// One line describing what happened.
    pub note: String,
    /// The redacted, bounded stdout and stderr.
    pub evidence: Vec<String>,
}

/// Build a project with the build command its profile declares.
///
/// The command runs as an argument array in `dir`, with the requested
/// target bound through the toolchain's own environment variable. A
/// profile without a build command is refused, a command that cannot
/// start fails with the reason, and a command that exits non-zero fails
/// with the reason and the evidence.
pub fn run_build(
    project_id: &str,
    profile_id: &str,
    build_command: &str,
    dir: &Path,
    request: &TargetRequest,
) -> Result<BuildOutcome, ForgeError> {
    if build_command.trim().is_empty() {
        return Err(ForgeError::BuildInvalid {
            reason: format!("profile `{profile_id}` declares no build command"),
        });
    }
    let descriptor = inspect_profile(profile_id)?;
    let target = resolve_target(profile_id, &descriptor.toolchain, request)?;
    let (program, args) = split_command(build_command).map_err(|err| ForgeError::BuildInvalid {
        reason: format!("profile `{profile_id}` declares an unusable build command: {err}"),
    })?;
    let outcome = run_native(dir, &program, &args, &target.env)?;
    if let Some(start_error) = outcome.start_error {
        return Err(ForgeError::BuildFailed {
            reason: format!(
                "build command `{build_command}` for project `{project_id}` could not start: \
                 {start_error}"
            ),
        });
    }
    let evidence = outcome.evidence();
    if !outcome.success {
        if outcome.timed_out {
            return Err(ForgeError::BuildFailed {
                reason: format!(
                    "build command `{build_command}` for project `{project_id}` exceeded the \
                     {} timeout; the command's process group was killed",
                    NATIVE_COMMAND_TIMEOUT.as_secs()
                ),
            });
        }
        let status = match outcome.exit_code {
            Some(code) => format!("status {code}"),
            None => "no exit status".to_string(),
        };
        return Err(ForgeError::BuildFailed {
            reason: format!(
                "build command `{build_command}` for project `{project_id}` exited with {status}; \
                 see evidence: {}",
                evidence.join(" | ")
            ),
        });
    }
    let note = if target.label == HOST_TARGET {
        format!("build command `{build_command}` passed for project `{project_id}`")
    } else {
        format!(
            "build command `{build_command}` passed for project `{project_id}` at target `{}`",
            target.label
        )
    };
    Ok(BuildOutcome {
        contract: BUILD_CONTRACT.to_string(),
        project_id: project_id.to_string(),
        profile: profile_id.to_string(),
        command: build_command.to_string(),
        target: target.label,
        status: "passed".to_string(),
        exit_code: outcome.exit_code,
        note,
        evidence,
    })
}
