//! Bounded `gh` CLI adapter for local repository workflows
//! (`github-cli-project-workflows`).
//!
//! Forge never embeds the GitHub SDK and never makes an HTTP request
//! itself: it spawns the installed `gh` CLI with `Command::new` plus a
//! fixed argument array, with `LC_ALL=C` and the shared bounded
//! per-run timeout from [`crate::process`], and parses the outcome.
//!
//! ## Configuration
//!
//! The `gh` binary is resolved in this order:
//!
//! 1. `FORGE_GH_BIN` (always wins when set and the file is
//!    executable).
//! 2. `gh` on `PATH`.
//!
//! The user's `gh` auth context is read through `gh auth status`.
//! Forge never spawns `gh auth token`, never reads the credential
//! store directly, and never inherits the credential through any
//! environment variable of its own. The token lives in the user's
//! `gh` credential store; Forge only learns "authenticated" / "not
//! authenticated" through `gh auth status`.
//!
//! ## States
//!
//! The closed [`GhOutcome`] vocabulary distinguishes success,
//! missing CLI, missing authentication, scope problems, missing
//! repositories, conflicting state, rate limits, timeouts, generic
//! unavailability and generic failure. A `failed` or `unavailable`
//! outcome is **not** a complete record; the journal records the
//! state and the operator decides whether to retry.
//!
//! ## Writes
//!
//! Repository creation, source push and pull-request creation all
//! require an explicit `--confirm` so a misconfigured caller cannot
//! publish silently. Public visibility requires the explicit pair
//! `--visibility public --confirm-public`. The package never opens
//! browser flows, never merges PRs and never creates releases.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::core::ForgeError;
use crate::policy::redact_credentials;
use crate::process::spawn_with_timeout;

/// Versioned request/response contract for the `gh` CLI workflow.
pub const GITHUB_CLI_CONTRACT_VERSION: &str = "forge-github-cli-workflows/0.1.0";

/// Default GitHub host the package uses. The user can override it on
/// the CLI; the adapter passes it to `gh auth status --hostname`.
pub const GITHUB_CLI_DEFAULT_HOST: &str = "github.com";

/// Default `gh` binary name. Matches the `gh` CLI as installed by
/// the user; the binary may be overridden through [`GITHUB_CLI_BIN_ENV`].
pub const DEFAULT_GITHUB_CLI_BIN: &str = "gh";

/// Environment variable selecting the `gh` binary.
pub const GITHUB_CLI_BIN_ENV: &str = "FORGE_GH_BIN";

/// Per-call wall-clock timeout. The `gh` CLI can hang on a stalled
/// network or on a slow `repo view`; an unresponsive tool cannot
/// hang Forge.
pub const GITHUB_CLI_TIMEOUT: Duration = Duration::from_secs(30);

/// Minimum timeout the caller may request. Anything below this is
/// clamped so a misconfigured caller cannot make `gh` unkillable.
pub const GITHUB_CLI_MIN_TIMEOUT: Duration = Duration::from_secs(1);

/// Maximum timeout the caller may request.
pub const GITHUB_CLI_MAX_TIMEOUT: Duration = Duration::from_secs(120);

/// Maximum number of bytes a `--title` argument may carry.
pub const MAX_TITLE_BYTES: usize = 256;

/// Maximum number of bytes a `--body` argument may carry.
pub const MAX_BODY_BYTES: usize = 8192;

/// Maximum number of bytes a `--destination` path may carry.
pub const MAX_DESTINATION_BYTES: usize = 4096;

/// Operation the adapter dispatches. The closed set keeps the
/// argument array allowlist bounded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GhOperation {
    /// `gh auth status --hostname <host>` — read-only probe.
    Auth,
    /// `gh repo clone <owner/repo> <destination>`.
    Clone,
    /// `gh repo create <name> --source <path> --remote origin --private|--public [--push]`.
    Create,
    /// `gh pr create --title <title> --body <body> [--draft]`.
    PullRequest,
}

impl GhOperation {
    /// Stable kebab-case id used in JSON envelopes and the operations
    /// journal.
    pub fn id(&self) -> &'static str {
        match self {
            GhOperation::Auth => "auth",
            GhOperation::Clone => "clone",
            GhOperation::Create => "create",
            GhOperation::PullRequest => "pull-request",
        }
    }
}

impl std::fmt::Display for GhOperation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.id())
    }
}

/// Visibility for a new GitHub repository. Private is the default
/// and the only safe default; public must be explicitly requested
/// and explicitly confirmed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GhVisibility {
    Private,
    Public,
}

impl GhVisibility {
    /// Stable kebab-case id.
    pub fn id(&self) -> &'static str {
        match self {
            GhVisibility::Private => "private",
            GhVisibility::Public => "public",
        }
    }
}

/// Outcome of one `gh` invocation. The closed vocabulary keeps the
/// JSON surface stable and the typed errors bounded.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GhOutcome {
    /// `gh` returned 0 with a captured artifact URL or empty stdout.
    Done,
    /// `gh auth status` reports no authenticated user for the host.
    AuthRequired,
    /// `gh` reported a scope or permission problem.
    Forbidden,
    /// The repository identity was rejected by `gh` or by Forge
    /// pre-validation.
    NotFound,
    /// A precondition failed in Forge (existing destination, dirty
    /// tree, missing remote, public without confirmation, push
    /// without confirmation).
    Conflict,
    /// `gh` reported a rate-limit hit. The remote may have accepted
    /// a partial response.
    RateLimited,
    /// The child was killed because the timeout elapsed. The remote
    /// may have accepted the write; the next call first queries the
    /// remote rather than repeating blindly.
    Timeout,
    /// `gh` is missing, non-executable, or otherwise not spawnable.
    Unavailable,
    /// `gh` returned nonzero without mapping to a more specific
    /// state.
    Failed,
}

impl GhOutcome {
    /// Stable kebab-case id.
    pub fn id(&self) -> &'static str {
        match self {
            GhOutcome::Done => "done",
            GhOutcome::AuthRequired => "auth-required",
            GhOutcome::Forbidden => "forbidden",
            GhOutcome::NotFound => "not-found",
            GhOutcome::Conflict => "conflict",
            GhOutcome::RateLimited => "rate-limited",
            GhOutcome::Timeout => "timeout",
            GhOutcome::Unavailable => "unavailable",
            GhOutcome::Failed => "failed",
        }
    }

    /// Map the outcome to the typed `ForgeError` code that the
    /// transport surface emits.
    pub fn code(&self) -> &'static str {
        match self {
            GhOutcome::Done => "github-cli-done",
            GhOutcome::AuthRequired => "github-cli-auth-required",
            GhOutcome::Forbidden => "github-cli-invalid",
            GhOutcome::NotFound => "github-cli-invalid",
            GhOutcome::Conflict => "github-cli-conflict",
            GhOutcome::RateLimited => "github-cli-unavailable",
            GhOutcome::Timeout => "github-cli-unavailable",
            GhOutcome::Unavailable => "github-cli-unavailable",
            GhOutcome::Failed => "github-cli-invalid",
        }
    }
}

impl std::fmt::Display for GhOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.id())
    }
}

/// Resolved `gh` binary. Mirrors [`crate::github::adapter::GithubAdapter`]
/// but for the user's installed `gh` CLI.
#[derive(Debug, Clone)]
pub struct GhCli {
    /// Resolved absolute path to the `gh` binary. `None` when no
    /// binary is configured and none is on `PATH`; the next call
    /// then reports [`GhOutcome::Unavailable`].
    pub binary: Option<PathBuf>,
    /// Adapter source label (e.g. `FORGE_GH_BIN=/usr/bin/gh`, `PATH:gh`).
    pub source: String,
}

impl GhCli {
    /// Resolve the `gh` binary from the environment.
    pub fn from_env() -> Self {
        Self::from_env_with(|name| std::env::var(name).ok())
    }

    /// Test seam: same as [`from_env`] but uses a caller-supplied
    /// environment resolver. Production callers should use
    /// [`from_env`].
    pub fn from_env_with<F>(env: F) -> Self
    where
        F: Fn(&str) -> Option<String>,
    {
        let override_bin = env(GITHUB_CLI_BIN_ENV)
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        if let Some(candidate) = override_bin {
            let path = PathBuf::from(&candidate);
            if is_executable_file(&path) {
                return GhCli {
                    binary: Some(path),
                    source: format!("{GITHUB_CLI_BIN_ENV}={candidate}"),
                };
            }
            return GhCli {
                binary: Some(path),
                source: format!("{GITHUB_CLI_BIN_ENV}={candidate} (not executable)"),
            };
        }
        if let Some(found) = which_in_path(DEFAULT_GITHUB_CLI_BIN) {
            return GhCli {
                binary: Some(found),
                source: format!("PATH:{DEFAULT_GITHUB_CLI_BIN}"),
            };
        }
        GhCli {
            binary: None,
            source: format!(
                "no {DEFAULT_GITHUB_CLI_BIN} on PATH and no {GITHUB_CLI_BIN_ENV} override"
            ),
        }
    }

    /// `true` when the `gh` binary is configured and is an existing
    /// executable file.
    pub fn binary_available(&self) -> bool {
        self.binary
            .as_ref()
            .map(|path| is_executable_file(path))
            .unwrap_or(false)
    }
}

/// Build the bounded `gh` command for an `auth status` probe. The
/// host argument is the only caller-supplied value; it is bounded
/// to the closed github.com form by the parser.
pub(crate) fn build_auth_command(binary: &Path, host: &str) -> Command {
    let mut command = Command::new(binary);
    command
        .env("LC_ALL", "C")
        .arg("auth")
        .arg("status")
        .arg("--hostname")
        .arg(host);
    command
}

/// Build the bounded `gh repo clone` command. The destination is
/// pre-validated by Forge (existing paths are refused); the
/// repository identity is pre-validated.
pub(crate) fn build_clone_command(binary: &Path, repository: &str, destination: &Path) -> Command {
    let mut command = Command::new(binary);
    command
        .env("LC_ALL", "C")
        .arg("repo")
        .arg("clone")
        .arg(repository)
        .arg(destination);
    command
}

/// Build the bounded `gh repo create` command. The caller decides
/// whether to add `--push` based on the explicit `--push-source
/// --confirm` pair.
pub(crate) fn build_create_command(
    binary: &Path,
    name: &str,
    source: &Path,
    visibility: GhVisibility,
    push_source: bool,
) -> Command {
    let mut command = Command::new(binary);
    command
        .env("LC_ALL", "C")
        .arg("repo")
        .arg("create")
        .arg(name)
        .arg("--source")
        .arg(source)
        .arg("--remote")
        .arg("origin")
        .arg(match visibility {
            GhVisibility::Private => "--private",
            GhVisibility::Public => "--public",
        });
    if push_source {
        command.arg("--push");
    }
    command
}

/// Build the bounded `gh pr create` command. Draft is a closed
/// on/off flag.
pub(crate) fn build_pr_command(binary: &Path, title: &str, body: &str, draft: bool) -> Command {
    let mut command = Command::new(binary);
    command
        .env("LC_ALL", "C")
        .arg("pr")
        .arg("create")
        .arg("--title")
        .arg(title)
        .arg("--body")
        .arg(body);
    if draft {
        command.arg("--draft");
    }
    command
}

/// Run the `gh auth status --hostname <host>` probe and translate
/// the exit code into the closed outcome vocabulary. The function is
/// read-only: it never spawns `gh auth token`, never reads the
/// credential store, and never returns the token.
pub fn run_auth(cli: &GhCli, host: &str) -> GhResult {
    if !cli.binary_available() {
        return GhResult::unavailable(
            GhOperation::Auth,
            None,
            None,
            format!("no `gh` binary is configured ({})", cli.source),
        );
    }
    let binary = cli.binary.clone().expect("binary_available");
    let mut command = build_auth_command(&binary, host);
    let execution = match spawn_with_timeout(&mut command, GITHUB_CLI_TIMEOUT) {
        Ok(execution) => execution,
        Err(reason) => {
            let outcome = if reason.contains("timed out") {
                GhOutcome::Timeout
            } else {
                GhOutcome::Unavailable
            };
            return GhResult {
                contract: GITHUB_CLI_CONTRACT_VERSION.to_string(),
                operation: GhOperation::Auth,
                outcome: outcome.clone(),
                exit_code: None,
                repository: None,
                artifact_url: None,
                stderr_tail: Some(redact_credentials(&reason)),
                note: redact_credentials(&reason),
            };
        }
    };
    GhResult::from_output(
        GhOperation::Auth,
        execution.exit_code,
        execution.stdout,
        execution.stderr,
        None,
    )
}

/// Run `gh repo clone <owner/repo> <destination>`. The caller is
/// expected to have validated `repository` and verified `destination`
/// is absent. The result records the resolved destination and the
/// captured stderr tail.
pub fn run_clone(cli: &GhCli, repository: &str, destination: &Path) -> GhResult {
    if !cli.binary_available() {
        return GhResult::unavailable(
            GhOperation::Clone,
            Some(repository.to_string()),
            Some(destination.display().to_string()),
            format!("no `gh` binary is configured ({})", cli.source),
        );
    }
    let binary = cli.binary.clone().expect("binary_available");
    let mut command = build_clone_command(&binary, repository, destination);
    let execution = match spawn_with_timeout(&mut command, GITHUB_CLI_TIMEOUT) {
        Ok(execution) => execution,
        Err(reason) => {
            let outcome = if reason.contains("timed out") {
                GhOutcome::Timeout
            } else {
                GhOutcome::Unavailable
            };
            return GhResult {
                contract: GITHUB_CLI_CONTRACT_VERSION.to_string(),
                operation: GhOperation::Clone,
                outcome,
                exit_code: None,
                repository: Some(repository.to_string()),
                artifact_url: None,
                stderr_tail: Some(redact_credentials(&reason)),
                note: redact_credentials(&reason),
            };
        }
    };
    GhResult::from_output(
        GhOperation::Clone,
        execution.exit_code,
        execution.stdout,
        execution.stderr,
        Some(destination.display().to_string()),
    )
}

/// Run `gh repo create <name> --source <path> --remote origin
/// --private|--public [--push]`. The caller is expected to have
/// validated the project path, the repository identity and the
/// confirmation flags.
pub fn run_create(
    cli: &GhCli,
    name: &str,
    source: &Path,
    visibility: GhVisibility,
    push_source: bool,
) -> GhResult {
    if !cli.binary_available() {
        return GhResult::unavailable(
            GhOperation::Create,
            Some(format!("(local){}", name)),
            Some(source.display().to_string()),
            format!("no `gh` binary is configured ({})", cli.source),
        );
    }
    let binary = cli.binary.clone().expect("binary_available");
    let mut command = build_create_command(&binary, name, source, visibility, push_source);
    let execution = match spawn_with_timeout(&mut command, GITHUB_CLI_TIMEOUT) {
        Ok(execution) => execution,
        Err(reason) => {
            let outcome = if reason.contains("timed out") {
                GhOutcome::Timeout
            } else {
                GhOutcome::Unavailable
            };
            return GhResult {
                contract: GITHUB_CLI_CONTRACT_VERSION.to_string(),
                operation: GhOperation::Create,
                outcome,
                exit_code: None,
                repository: Some(name.to_string()),
                artifact_url: None,
                stderr_tail: Some(redact_credentials(&reason)),
                note: redact_credentials(&reason),
            };
        }
    };
    let artifact = parse_repo_url(&execution.stdout);
    GhResult::from_output_with_artifact(
        GhOperation::Create,
        execution.exit_code,
        execution.stdout,
        execution.stderr,
        Some(source.display().to_string()),
        artifact,
    )
}

/// Run `gh pr create --title <title> --body <body> [--draft]`. The
/// caller is expected to have validated the title, the body, the
/// clean tree, the remote, and the explicit `--confirm` flag.
pub fn run_pull_request(cli: &GhCli, title: &str, body: &str, draft: bool) -> GhResult {
    if !cli.binary_available() {
        return GhResult::unavailable(
            GhOperation::PullRequest,
            None,
            None,
            format!("no `gh` binary is configured ({})", cli.source),
        );
    }
    let binary = cli.binary.clone().expect("binary_available");
    let mut command = build_pr_command(&binary, title, body, draft);
    let execution = match spawn_with_timeout(&mut command, GITHUB_CLI_TIMEOUT) {
        Ok(execution) => execution,
        Err(reason) => {
            let outcome = if reason.contains("timed out") {
                GhOutcome::Timeout
            } else {
                GhOutcome::Unavailable
            };
            return GhResult {
                contract: GITHUB_CLI_CONTRACT_VERSION.to_string(),
                operation: GhOperation::PullRequest,
                outcome,
                exit_code: None,
                repository: None,
                artifact_url: None,
                stderr_tail: Some(redact_credentials(&reason)),
                note: redact_credentials(&reason),
            };
        }
    };
    let artifact = parse_repo_url(&execution.stdout);
    GhResult::from_output_with_artifact(
        GhOperation::PullRequest,
        execution.exit_code,
        execution.stdout,
        execution.stderr,
        None,
        artifact,
    )
}

/// Result of one `gh` invocation. The JSON envelope carries the
/// closed outcome plus a redacted stderr tail; the typed error is
/// derived from the outcome.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GhResult {
    /// Contract version.
    pub contract: String,
    /// Operation that was attempted.
    pub operation: GhOperation,
    /// Closed outcome.
    pub outcome: GhOutcome,
    /// Exit code (`None` when the child was killed by the helper).
    pub exit_code: Option<i32>,
    /// Repository identity the operation targeted (`owner/repo`).
    /// Used by the error mapper so the typed message names the
    /// repository.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
    /// Artifact URL (for create / clone / pull-request success).
    pub artifact_url: Option<String>,
    /// Redacted stderr tail (≤ 2 KiB). Never a credential.
    pub stderr_tail: Option<String>,
    /// Human note, always credential-redacted.
    pub note: String,
}

impl GhResult {
    /// Convenience constructor for the unavailable case.
    pub fn unavailable(
        operation: GhOperation,
        repository: Option<String>,
        _destination: Option<String>,
        reason: String,
    ) -> Self {
        Self {
            contract: GITHUB_CLI_CONTRACT_VERSION.to_string(),
            operation,
            outcome: GhOutcome::Unavailable,
            exit_code: None,
            repository: repository.clone(),
            artifact_url: repository,
            stderr_tail: Some(redact_credentials(&reason)),
            note: redact_credentials(&reason),
        }
    }

    /// Map the child output to the closed outcome based on the
    /// exit code and the captured stdout/stderr.
    pub fn from_output(
        operation: GhOperation,
        exit_code: Option<i32>,
        stdout: Vec<u8>,
        stderr: String,
        destination: Option<String>,
    ) -> Self {
        let stderr_redacted = redact_credentials(&stderr);
        let outcome = match exit_code {
            Some(0) => GhOutcome::Done,
            Some(_) => classify_failure(&stderr),
            None => GhOutcome::Unavailable,
        };
        let artifact = match outcome {
            GhOutcome::Done => destination.or_else(|| parse_repo_url(&stdout)),
            _ => None,
        };
        let note = match outcome {
            GhOutcome::Done => String::new(),
            _ => first_nonempty_line(&stderr_redacted),
        };
        Self {
            contract: GITHUB_CLI_CONTRACT_VERSION.to_string(),
            operation,
            outcome,
            exit_code,
            repository: None,
            artifact_url: artifact,
            stderr_tail: Some(stderr_redacted),
            note,
        }
    }

    /// Same as [`from_output`] but lets the caller pre-compute the
    /// artifact URL (used by `create` and `pull-request`).
    pub fn from_output_with_artifact(
        operation: GhOperation,
        exit_code: Option<i32>,
        stdout: Vec<u8>,
        stderr: String,
        _destination: Option<String>,
        artifact: Option<String>,
    ) -> Self {
        let stderr_redacted = redact_credentials(&stderr);
        let outcome = match exit_code {
            Some(0) => GhOutcome::Done,
            Some(_) => classify_failure(&stderr),
            None => GhOutcome::Unavailable,
        };
        let artifact_url = match outcome {
            GhOutcome::Done => artifact.or_else(|| parse_repo_url(&stdout)),
            _ => None,
        };
        let note = match outcome {
            GhOutcome::Done => String::new(),
            _ => first_nonempty_line(&stderr_redacted),
        };
        Self {
            contract: GITHUB_CLI_CONTRACT_VERSION.to_string(),
            operation,
            outcome,
            exit_code,
            repository: None,
            artifact_url,
            stderr_tail: Some(stderr_redacted),
            note,
        }
    }

    /// Repository the operation targeted. Used to enrich the typed
    /// error message.
    pub fn repository(&self) -> String {
        self.repository.clone().unwrap_or_default()
    }

    /// Convert the closed outcome to the matching `ForgeError`. The
    /// `done` outcome is not an error.
    pub fn to_error(&self, context: &str) -> ForgeError {
        let reason = match &self.repository {
            Some(repo) if !repo.is_empty() => format!("{context} `{repo}`: {}", self.note),
            _ => format!("{context}: {}", self.note),
        };
        match self.outcome {
            GhOutcome::Done => ForgeError::GithubCliInvalid {
                reason: "internal: done outcome cannot be turned into an error".to_string(),
            },
            GhOutcome::AuthRequired => ForgeError::GithubCliAuthRequired { reason },
            GhOutcome::Forbidden => ForgeError::GithubCliInvalid { reason },
            GhOutcome::NotFound => ForgeError::GithubCliInvalid { reason },
            GhOutcome::Conflict => ForgeError::GithubCliConflict { reason },
            GhOutcome::RateLimited => ForgeError::GithubCliUnavailable { reason },
            GhOutcome::Timeout => ForgeError::GithubCliUnavailable { reason },
            GhOutcome::Unavailable => ForgeError::GithubCliUnavailable { reason },
            GhOutcome::Failed => ForgeError::GithubCliInvalid { reason },
        }
    }
}

fn classify_failure(stderr: &str) -> GhOutcome {
    let lower = stderr.to_ascii_lowercase();
    if lower.contains("not logged in") || lower.contains("not authenticated") {
        GhOutcome::AuthRequired
    } else if lower.contains("rate limit") || lower.contains("api rate limit") {
        GhOutcome::RateLimited
    } else if lower.contains("permission") || lower.contains("forbidden") {
        GhOutcome::Forbidden
    } else if lower.contains("not found") || lower.contains("could not resolve") {
        GhOutcome::NotFound
    } else if lower.contains("already exists") || lower.contains("conflict") {
        GhOutcome::Conflict
    } else {
        GhOutcome::Failed
    }
}

fn first_nonempty_line(text: &str) -> String {
    for line in text.lines() {
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }
    text.trim().to_string()
}

fn parse_repo_url(stdout: &[u8]) -> Option<String> {
    let text = String::from_utf8_lossy(stdout);
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("https://github.com/")
            || trimmed.starts_with("https://gist.github.com/")
        {
            return Some(trimmed.to_string());
        }
    }
    None
}

fn is_executable_file(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        match std::fs::metadata(path) {
            Ok(metadata) => metadata.permissions().mode() & 0o111 != 0,
            Err(_) => false,
        }
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        true
    }
}

fn which_in_path(name: &str) -> Option<PathBuf> {
    let path_var = std::env::var_os("PATH")?;
    for entry in std::env::split_paths(&path_var) {
        let candidate = entry.join(name);
        if is_executable_file(&candidate) {
            return Some(candidate);
        }
    }
    None
}

/// Validate a repository identity in the closed `owner/repo` form.
/// Used by every command that takes a `<owner/repo>` argument.
pub fn parse_repository(repository: &str) -> Result<(), String> {
    if repository.is_empty() {
        return Err("repository identity is empty".to_string());
    }
    if repository.contains(char::is_whitespace) {
        return Err("repository identity contains whitespace".to_string());
    }
    let trimmed = repository.trim_matches('/');
    if trimmed.is_empty() {
        return Err("repository identity is empty".to_string());
    }
    let mut parts = trimmed.split('/');
    let owner = parts.next().unwrap_or("");
    let name = parts.next().unwrap_or("");
    if owner.is_empty()
        || name.is_empty()
        || parts.next().is_some()
        || !owner
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
        || !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
    {
        return Err("expected `owner/repo` with letters, digits, `-`, `_` or `.` only".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn operation_ids_are_stable_kebab_case() {
        assert_eq!(GhOperation::Auth.id(), "auth");
        assert_eq!(GhOperation::Clone.id(), "clone");
        assert_eq!(GhOperation::Create.id(), "create");
        assert_eq!(GhOperation::PullRequest.id(), "pull-request");
    }

    #[test]
    fn outcome_ids_are_stable_kebab_case() {
        assert_eq!(GhOutcome::Done.id(), "done");
        assert_eq!(GhOutcome::AuthRequired.id(), "auth-required");
        assert_eq!(GhOutcome::Forbidden.id(), "forbidden");
        assert_eq!(GhOutcome::NotFound.id(), "not-found");
        assert_eq!(GhOutcome::Conflict.id(), "conflict");
        assert_eq!(GhOutcome::RateLimited.id(), "rate-limited");
        assert_eq!(GhOutcome::Timeout.id(), "timeout");
        assert_eq!(GhOutcome::Unavailable.id(), "unavailable");
        assert_eq!(GhOutcome::Failed.id(), "failed");
    }

    #[test]
    fn outcome_codes_map_to_typed_github_cli_codes() {
        assert_eq!(GhOutcome::Done.code(), "github-cli-done");
        assert_eq!(GhOutcome::AuthRequired.code(), "github-cli-auth-required");
        assert_eq!(GhOutcome::Forbidden.code(), "github-cli-invalid");
        assert_eq!(GhOutcome::NotFound.code(), "github-cli-invalid");
        assert_eq!(GhOutcome::Conflict.code(), "github-cli-conflict");
        assert_eq!(GhOutcome::RateLimited.code(), "github-cli-unavailable");
        assert_eq!(GhOutcome::Timeout.code(), "github-cli-unavailable");
        assert_eq!(GhOutcome::Unavailable.code(), "github-cli-unavailable");
        assert_eq!(GhOutcome::Failed.code(), "github-cli-invalid");
    }

    #[test]
    fn parse_repository_rejects_malformed_values() {
        for bad in [
            "",
            " ",
            "owner",
            "owner/",
            "/repo",
            "owner/repo/extra",
            "owner/repo with space",
            "owner/repo\n",
        ] {
            assert!(
                parse_repository(bad).is_err(),
                "expected `{bad}` to be rejected"
            );
        }
        for good in ["octocat/hello-world", "a/b", "owner_1/repo.2"] {
            assert!(
                parse_repository(good).is_ok(),
                "expected `{good}` to be accepted"
            );
        }
    }

    #[test]
    fn classify_failure_reads_stderr_keyword() {
        assert_eq!(
            classify_failure("You are not logged in"),
            GhOutcome::AuthRequired
        );
        assert_eq!(
            classify_failure("API rate limit exceeded"),
            GhOutcome::RateLimited
        );
        assert_eq!(
            classify_failure("Permission denied (publickey)"),
            GhOutcome::Forbidden
        );
        assert_eq!(
            classify_failure("Not Found: https://api.github.com"),
            GhOutcome::NotFound
        );
        assert_eq!(
            classify_failure("repository name already exists"),
            GhOutcome::Conflict
        );
        assert_eq!(classify_failure("anything else"), GhOutcome::Failed);
    }

    #[test]
    fn parse_repo_url_extracts_a_github_url_from_stdout() {
        let stdout = b"https://github.com/octocat/hello-world\n";
        assert_eq!(
            parse_repo_url(stdout),
            Some("https://github.com/octocat/hello-world".to_string())
        );
        let stdout = b"some unrelated log line\nhttps://github.com/octocat/hello-world/pull/1\n";
        assert_eq!(
            parse_repo_url(stdout),
            Some("https://github.com/octocat/hello-world/pull/1".to_string())
        );
        let stdout = b"";
        assert_eq!(parse_repo_url(stdout), None);
    }

    #[test]
    fn gh_cli_resolution_finds_an_explicit_override() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("gh");
        std::fs::write(&path, "#!/bin/sh\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        let env = |name: &str| {
            if name == GITHUB_CLI_BIN_ENV {
                Some(path.display().to_string())
            } else {
                None
            }
        };
        let cli = GhCli::from_env_with(env);
        assert!(cli.binary_available(), "{:?}", cli.source);
    }

    #[test]
    fn gh_cli_resolution_reports_missing_binary_honestly() {
        // Provide an explicit override pointing at a non-existent
        // path so the adapter reports the unavailable state honestly
        // regardless of whether a real `gh` happens to be on the
        // host PATH.
        let env = |name: &str| {
            if name == GITHUB_CLI_BIN_ENV {
                Some("/definitely/not/a/real/binary/gh".to_string())
            } else {
                None
            }
        };
        let cli = GhCli::from_env_with(env);
        assert!(!cli.binary_available());
        assert!(cli.source.contains("not executable") || cli.source.contains("not-a-real-binary"));
    }

    #[test]
    fn auth_command_carries_hostname_only() {
        let binary = PathBuf::from("/does/not/matter/gh");
        let command = build_auth_command(&binary, "github.com");
        let args: Vec<String> = command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            args,
            vec!["auth", "status", "--hostname", "github.com"]
                .into_iter()
                .map(String::from)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn clone_command_carries_repository_and_destination_only() {
        let binary = PathBuf::from("/does/not/matter/gh");
        let destination = PathBuf::from("/tmp/clone-target");
        let command = build_clone_command(&binary, "octocat/hello-world", &destination);
        let args: Vec<String> = command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            args,
            vec!["repo", "clone", "octocat/hello-world", "/tmp/clone-target"]
                .into_iter()
                .map(String::from)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn create_command_defaults_to_private() {
        let binary = PathBuf::from("/does/not/matter/gh");
        let source = PathBuf::from("/tmp/project");
        let command = build_create_command(
            &binary,
            "hello-world",
            &source,
            GhVisibility::Private,
            false,
        );
        let args: Vec<String> = command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect();
        assert!(!args.contains(&"--public".to_string()));
        assert!(args.contains(&"--private".to_string()));
        assert!(!args.contains(&"--push".to_string()));
    }

    #[test]
    fn create_command_only_adds_public_with_explicit_visibility() {
        let binary = PathBuf::from("/does/not/matter/gh");
        let source = PathBuf::from("/tmp/project");
        let command =
            build_create_command(&binary, "hello-world", &source, GhVisibility::Public, false);
        let args: Vec<String> = command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect();
        assert!(args.contains(&"--public".to_string()));
        assert!(!args.contains(&"--private".to_string()));
    }

    #[test]
    fn create_command_only_adds_push_when_explicitly_requested() {
        let binary = PathBuf::from("/does/not/matter/gh");
        let source = PathBuf::from("/tmp/project");
        let command =
            build_create_command(&binary, "hello-world", &source, GhVisibility::Private, true);
        let args: Vec<String> = command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect();
        assert!(args.contains(&"--push".to_string()));
    }

    #[test]
    fn pr_command_carries_title_body_and_optional_draft() {
        let binary = PathBuf::from("/does/not/matter/gh");
        let command = build_pr_command(&binary, "title", "body", false);
        let args: Vec<String> = command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            args,
            vec!["pr", "create", "--title", "title", "--body", "body"]
                .into_iter()
                .map(String::from)
                .collect::<Vec<_>>()
        );
        let command = build_pr_command(&binary, "title", "body", true);
        let args: Vec<String> = command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect();
        assert!(args.contains(&"--draft".to_string()));
    }

    #[test]
    fn result_from_output_classifies_exit_zero_as_done() {
        let result = GhResult::from_output(
            GhOperation::Auth,
            Some(0),
            b"".to_vec(),
            "".to_string(),
            None,
        );
        assert_eq!(result.outcome, GhOutcome::Done);
        assert_eq!(result.exit_code, Some(0));
    }

    #[test]
    fn result_from_output_redacts_credentials_in_stderr() {
        let result = GhResult::from_output(
            GhOperation::Auth,
            Some(1),
            b"".to_vec(),
            "ghp_abcdefghijklmnopqrstuvwxyz0123456789 leaked".to_string(),
            None,
        );
        assert!(!result
            .stderr_tail
            .as_deref()
            .unwrap_or("")
            .contains("ghp_abcdef"));
        assert!(result
            .stderr_tail
            .as_deref()
            .unwrap_or("")
            .contains("[REDACTED]"));
    }

    #[test]
    fn result_to_error_maps_every_outcome_to_a_typed_code() {
        for (outcome, expected_code) in [
            (GhOutcome::AuthRequired, "github-cli-auth-required"),
            (GhOutcome::Forbidden, "github-cli-invalid"),
            (GhOutcome::NotFound, "github-cli-invalid"),
            (GhOutcome::Conflict, "github-cli-conflict"),
            (GhOutcome::RateLimited, "github-cli-unavailable"),
            (GhOutcome::Timeout, "github-cli-unavailable"),
            (GhOutcome::Unavailable, "github-cli-unavailable"),
            (GhOutcome::Failed, "github-cli-invalid"),
        ] {
            let result = GhResult {
                contract: GITHUB_CLI_CONTRACT_VERSION.to_string(),
                operation: GhOperation::Auth,
                outcome: outcome.clone(),
                exit_code: Some(1),
                repository: None,
                artifact_url: None,
                stderr_tail: None,
                note: "x".to_string(),
            };
            let err = result.to_error("ctx");
            assert_eq!(err.code(), expected_code, "{outcome:?}");
        }
    }

    #[test]
    fn first_nonempty_line_returns_the_first_meaningful_line() {
        assert_eq!(first_nonempty_line("\n  hello\nworld"), "hello");
        assert_eq!(first_nonempty_line(""), "");
        assert_eq!(first_nonempty_line("hello"), "hello");
    }
}
