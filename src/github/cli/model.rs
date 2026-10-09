//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use crate::core::ForgeError;
use crate::policy::redact_credentials;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use super::commands::{
    classify_failure, first_nonempty_line, is_executable_file, parse_repo_url, which_in_path,
};
use super::limits::{DEFAULT_GITHUB_CLI_BIN, GITHUB_CLI_BIN_ENV, GITHUB_CLI_CONTRACT_VERSION};

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
            Some(repo) if !repo.is_empty() => {
                format!("{context} `{repo}`: {}", self.note)
            }
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
