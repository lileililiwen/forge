//! The shared native-command runner.
//!
//! One runner serves every Core operation that runs a native command as
//! an argument array: the generator's post-generation verification, the
//! readiness fixtures and the build step. The command never runs through
//! a shell, the working directory is the caller's, the environment
//! overrides are layered on top of the inherited environment, and the
//! captured output is redacted through the policy plane before it
//! reaches any evidence.
//!
//! The runner has no timeout. `forge test` has none either; a bounded
//! runner with a process-group kill is a recorded follow-up.

pub mod native_tests;

use std::path::Path;
use std::process::Command;

use crate::core::ForgeError;
use crate::policy::redact_credentials;

/// The maximum number of characters kept from each captured stream.
pub const MAX_CAPTURE_CHARS: usize = 8000;

/// The maximum number of characters kept in one evidence line.
pub const MAX_EVIDENCE_CHARS: usize = 800;

/// The result of one native command.
///
/// The evidence of the run is the redacted, bounded stdout and stderr
/// the outcome carries; [`NativeOutcome::evidence`] renders them as the
/// report's evidence lines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeOutcome {
    /// The command's exit code, when the process reported one.
    pub exit_code: Option<i32>,
    /// True when the command exited with status zero.
    pub success: bool,
    /// The captured stdout, redacted and bounded.
    pub stdout: String,
    /// The captured stderr, redacted and bounded.
    pub stderr: String,
    /// Why the command did not start, when it did not start.
    pub start_error: Option<String>,
}

impl NativeOutcome {
    /// The failing tail of the captured output: stderr when it carries
    /// content, stdout otherwise.
    pub fn tail(&self, limit: usize) -> String {
        let detail = if self.stderr.trim().is_empty() {
            &self.stdout
        } else {
            &self.stderr
        };
        truncate_tail(detail, limit)
    }

    /// The evidence lines of the run: stdout and stderr, each bounded.
    pub fn evidence(&self) -> Vec<String> {
        let mut lines = Vec::new();
        if !self.stdout.trim().is_empty() {
            lines.push(format!(
                "stdout: {}",
                truncate_head(&self.stdout, MAX_EVIDENCE_CHARS)
            ));
        }
        if !self.stderr.trim().is_empty() {
            lines.push(format!(
                "stderr: {}",
                truncate_head(&self.stderr, MAX_EVIDENCE_CHARS)
            ));
        }
        lines
    }
}

/// Split a command string into a program and its arguments.
///
/// The command is split on whitespace, so it carries no shell syntax. An
/// empty command is refused with a reason; each caller wraps that reason in
/// its own error variant, so the shared runner names no caller.
pub fn split_command(command: &str) -> Result<(String, Vec<String>), String> {
    let mut parts: Vec<String> = command.split_whitespace().map(str::to_string).collect();
    if parts.is_empty() {
        return Err("empty native command; nothing was verified".to_string());
    }
    let program = parts.remove(0);
    Ok((program, parts))
}

/// Run a native command as an argument array in `dir`.
///
/// `env` is layered on top of the inherited environment: every entry
/// replaces the inherited value of that variable. A command that cannot
/// start is reported through [`NativeOutcome::start_error`] rather than
/// as an error, so each caller renders the failure in its own error
/// variant. The `Result` is reserved for a future refusal; the runner
/// itself refuses nothing.
pub fn run_native(
    dir: &Path,
    program: &str,
    args: &[String],
    env: &[(String, String)],
) -> Result<NativeOutcome, ForgeError> {
    let mut command = Command::new(program);
    command.args(args).current_dir(dir);
    for (key, value) in env {
        command.env(key, value);
    }
    match command.output() {
        Ok(output) => Ok(NativeOutcome {
            exit_code: output.status.code(),
            success: output.status.success(),
            stdout: bound(&redact_credentials(&String::from_utf8_lossy(
                &output.stdout,
            ))),
            stderr: bound(&redact_credentials(&String::from_utf8_lossy(
                &output.stderr,
            ))),
            start_error: None,
        }),
        Err(err) => Ok(NativeOutcome {
            exit_code: None,
            success: false,
            stdout: String::new(),
            stderr: String::new(),
            start_error: Some(err.to_string()),
        }),
    }
}

/// Bound a captured stream to the last [`MAX_CAPTURE_CHARS`] characters.
///
/// The tail is kept because a failure is reported at the end of the
/// output: a bound never changes the last characters a reader sees.
fn bound(text: &str) -> String {
    let count = text.chars().count();
    if count <= MAX_CAPTURE_CHARS {
        return text.to_string();
    }
    text.chars().skip(count - MAX_CAPTURE_CHARS).collect()
}

/// The last `limit` characters of `text`, with the surrounding
/// whitespace removed.
fn truncate_tail(text: &str, limit: usize) -> String {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    trimmed
        .chars()
        .rev()
        .take(limit)
        .collect::<String>()
        .chars()
        .rev()
        .collect()
}

/// The first `limit` characters of `text`, marked when the text was
/// longer.
fn truncate_head(text: &str, limit: usize) -> String {
    if text.chars().count() <= limit {
        return text.to_string();
    }
    let head: String = text.chars().take(limit).collect();
    format!("{head}…")
}
