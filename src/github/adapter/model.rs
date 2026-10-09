//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use crate::core::ForgeError;
use crate::policy::redact_credentials;
use crate::process::spawn_with_timeout;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;

use super::limits::{
    GITHUB_ADAPTER_TIMEOUT, GITHUB_BIN_ENV, GITHUB_TOKEN_ENV, MAX_OBSERVATION_REPOSITORIES,
};
use super::support::{
    is_executable_file, mutation_mode_label, parse_observe_output, parse_propose_output,
    unavailable_observation, validate_propose_request, validate_repository, which_in_path,
};

/// One approved metadata change. The package never merges arbitrary
/// fields: only the closed list of `topic`, `description`, `language`
/// label and `homepage` is accepted by the adapter, and the package
/// validates the field set on the way in.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProposedChange {
    /// Closed field name. The adapter refuses anything outside the
    /// closed set.
    pub field: String,
    /// New value. Refused when empty or credential-shaped.
    pub new_value: String,
}
/// What the adapter reported back for a `propose` invocation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProposeOutcome {
    /// Mode the adapter actually executed.
    pub mode: String,
    /// Provider-reported identifier for the produced artifact
    /// (pull-request URL for `pull-request` mode, commit SHA for
    /// `direct` mode). `None` when the adapter refused.
    pub artifact_id: Option<String>,
    /// The adapter's reported state (`current`, `unavailable`,
    /// `forbidden`, `rate-limited`, …). The package never treats a
    /// non-`current` state as success.
    pub state: GithubState,
    /// Adapter-reported note, redacted.
    pub note: String,
}
/// Raw adapter output. Exposed so tests can drive a stub without
/// touching the real `PATH`.
#[derive(Debug, Clone)]
pub struct AdapterOutput {
    pub exit_code: Option<i32>,
    pub stdout: Vec<u8>,
    pub stderr: String,
    pub timed_out: bool,
}
impl AdapterOutput {
    /// Wrap a [`crate::process::ChildOutput`] as the adapter-shaped
    /// output the parsers consume. The shared spawn helper already
    /// bounds stderr and tracks the timeout flag.
    pub fn from_child(output: crate::process::ChildOutput) -> Self {
        Self {
            exit_code: output.exit_code,
            stdout: output.stdout,
            stderr: output.stderr,
            timed_out: output.timed_out,
        }
    }
}
#[derive(Debug, Clone, Deserialize)]
pub(super) struct AdapterProposePayload {
    #[serde(default)]
    pub(super) mode: Option<String>,
    #[serde(default)]
    pub(super) artifact_id: Option<String>,
    #[serde(default)]
    pub(super) state: Option<String>,
    #[serde(default)]
    pub(super) rate_limit_reset_at: Option<String>,
    #[serde(default)]
    pub(super) note: Option<String>,
    #[serde(default)]
    pub(super) confirmation: Option<String>,
}
/// Resolved adapter binary + the request the caller asked for. The
/// struct is what tests use to drive a local stub without touching the
/// real `PATH`.
#[derive(Debug, Clone)]
pub struct GithubAdapter {
    /// Resolved absolute path to the adapter binary. `None` when no
    /// binary is configured and none is on `PATH`; the next call then
    /// reports [`GithubState::Unavailable`].
    pub binary: Option<PathBuf>,
    /// Adapter source label (e.g. `FORGE_GITHUB_BIN`, `PATH:forge-github-metadata-adapter`).
    pub source: String,
    /// Token resolved from [`GITHUB_TOKEN_ENV`]. `None` when the
    /// variable is unset or empty; the observation surfaces
    /// [`GithubState::Unauthorized`] and never makes a request.
    pub token: Option<String>,
}
impl GithubAdapter {
    /// Resolve the adapter binary and the token from the environment.
    /// A missing binary is recorded as `binary: None`; the next call
    /// reports the unavailable state with the exact path that was
    /// tried. A missing token is recorded as `token: None`; the next
    /// call reports the unauthorized state without ever making a
    /// request.
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
        let token = env(GITHUB_TOKEN_ENV)
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        let override_bin = env(GITHUB_BIN_ENV)
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        if let Some(candidate) = override_bin {
            let path = PathBuf::from(&candidate);
            if is_executable_file(&path) {
                return GithubAdapter {
                    binary: Some(path),
                    source: format!("{GITHUB_BIN_ENV}={candidate}"),
                    token,
                };
            }
            return GithubAdapter {
                binary: Some(path),
                source: format!("{GITHUB_BIN_ENV}={candidate} (not executable)"),
                token,
            };
        }
        if let Some(found) = which_in_path("forge-github-metadata-adapter") {
            return GithubAdapter {
                binary: Some(found),
                source: "PATH:forge-github-metadata-adapter".to_string(),
                token,
            };
        }
        GithubAdapter {
            binary: None,
            source: "no forge-github-metadata-adapter on PATH and no FORGE_GITHUB_BIN override"
                .to_string(),
            token,
        }
    }
    /// `true` when the adapter binary is configured and is an existing
    /// executable file. A non-executable override is a misconfiguration
    /// but the package still reports it honestly instead of silently
    /// dropping the entry.
    pub fn binary_available(&self) -> bool {
        self.binary
            .as_ref()
            .map(|path| is_executable_file(path))
            .unwrap_or(false)
    }
    /// Observe a list of repositories. The token is read from the
    /// environment; the request body never contains it.
    pub fn observe(
        &self,
        request: &GithubObservationRequest,
    ) -> Result<Vec<GithubObservation>, ForgeError> {
        if request.repositories.is_empty() {
            return Err(ForgeError::GithubInvalid {
                reason: "no repositories were requested for observation; the adapter cannot \
                         guess which project to look at"
                    .to_string(),
            });
        }
        if request.repositories.len() > MAX_OBSERVATION_REPOSITORIES {
            return Err(ForgeError::GithubInvalid {
                reason: format!(
                    "observation list has {} entries; the maximum is {MAX_OBSERVATION_REPOSITORIES}",
                    request.repositories.len()
                ),
            });
        }
        for repository in &request.repositories {
            validate_repository(repository).map_err(|reason| ForgeError::GithubInvalid {
                reason: format!("repository `{repository}`: {reason}"),
            })?;
        }
        let Some(binary) = self.binary.as_ref() else {
            return Err(ForgeError::GithubAdapterUnavailable {
                reason: format!("no GitHub adapter binary is configured ({})", self.source),
            });
        };
        if !self.binary_available() {
            return Err(ForgeError::GithubAdapterUnavailable {
                reason: format!(
                    "GitHub adapter binary `{}` is not an executable file ({})",
                    binary.display(),
                    self.source
                ),
            });
        }
        if self.token.is_none() {
            return Err(ForgeError::GithubInvalid {
                reason: format!(
                    "{GITHUB_TOKEN_ENV} is not set; refusing to send an unauthenticated request"
                ),
            });
        }
        let now = Utc::now();
        let mut observations: Vec<GithubObservation> = Vec::new();
        for repository in &request.repositories {
            let observation = self.observe_one(binary, request.host.as_str(), repository, now)?;
            observations.push(observation);
        }
        Ok(observations)
    }
    pub(super) fn observe_one(
        &self,
        binary: &Path,
        host: &str,
        repository: &str,
        now: DateTime<Utc>,
    ) -> Result<GithubObservation, ForgeError> {
        let mut command = Command::new(binary);
        command
            .env("LC_ALL", "C")
            .arg("observe")
            .arg("--host")
            .arg(host)
            .arg("--repository")
            .arg(repository);
        if let Some(token) = self.token.as_ref() {
            command.env(GITHUB_TOKEN_ENV, token);
        } else {
            command.env_remove(GITHUB_TOKEN_ENV);
        }
        let output = match spawn_with_timeout(&mut command, GITHUB_ADAPTER_TIMEOUT) {
            Ok(output) => AdapterOutput::from_child(output),
            Err(reason) => {
                return Ok(unavailable_observation(
                    host,
                    repository,
                    now,
                    format!("cannot execute adapter: {reason}"),
                ));
            }
        };
        parse_observe_output(binary, host, repository, output, now)
    }
    /// Propose a mutation. The default mode is pull request; direct
    /// mode requires an explicit confirmation string and the adapter
    /// echoes it back.
    pub fn propose(&self, request: &ProposeRequest) -> Result<ProposeOutcome, ForgeError> {
        validate_propose_request(request)?;
        let Some(binary) = self.binary.as_ref() else {
            return Err(ForgeError::GithubAdapterUnavailable {
                reason: format!("no GitHub adapter binary is configured ({})", self.source),
            });
        };
        if !self.binary_available() {
            return Err(ForgeError::GithubAdapterUnavailable {
                reason: format!(
                    "GitHub adapter binary `{}` is not an executable file ({})",
                    binary.display(),
                    self.source
                ),
            });
        }
        if self.token.is_none() {
            return Err(ForgeError::GithubInvalid {
                reason: format!(
                    "{GITHUB_TOKEN_ENV} is not set; refusing to mutate without authentication"
                ),
            });
        }
        let mut command = Command::new(binary);
        command
            .env("LC_ALL", "C")
            .arg("propose")
            .arg("--host")
            .arg(&request.host)
            .arg("--repository")
            .arg(&request.repository)
            .arg("--mode")
            .arg(match &request.mode {
                MutationMode::PullRequest => "pull-request",
                MutationMode::Direct { .. } => "direct",
            });
        if let Some(token) = self.token.as_ref() {
            command.env(GITHUB_TOKEN_ENV, token);
        } else {
            command.env_remove(GITHUB_TOKEN_ENV);
        }
        if let MutationMode::Direct { confirmation } = &request.mode {
            command.arg("--confirm").arg(confirmation);
        }
        for change in &request.changes {
            command
                .arg("--set")
                .arg(format!("{}={}", change.field, change.new_value));
        }
        let output = match spawn_with_timeout(&mut command, GITHUB_ADAPTER_TIMEOUT) {
            Ok(output) => AdapterOutput::from_child(output),
            Err(reason) => {
                return Ok(ProposeOutcome {
                    mode: mutation_mode_label(&request.mode),
                    artifact_id: None,
                    state: GithubState::Unavailable {
                        reason: redact_credentials(&reason),
                    },
                    note: format!("cannot execute adapter: {}", redact_credentials(&reason)),
                });
            }
        };
        parse_propose_output(binary, &request.mode, output)
    }
}
/// Request to mutate a repository's metadata. The package never opens a
/// pull request for side-effect data (analytics, deploy state, registry
/// observations) and never opens issues, comments or labels.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProposeRequest {
    /// GitHub host (e.g. `github.com`).
    pub host: String,
    /// Repository identity (`owner/repo`).
    pub repository: String,
    /// Mutation mode.
    pub mode: MutationMode,
    /// List of approved changes.
    pub changes: Vec<ProposedChange>,
}
/// One observed repository, with provenance. Topics, release tags and
/// the registry's portfolio tags are kept as **three separate**
/// namespaces: topics live in [`topics`], release tags live in
/// [`releases`], and the catalog tag list is *never* derived from
/// either of them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GithubObservation {
    /// The contract version the adapter answered.
    pub contract: String,
    /// GitHub host the observation came from.
    pub host: String,
    /// Repository identity (`owner/repo`).
    pub repository: String,
    /// Adapter-reported revision (etag / commit SHA) so two reads of an
    /// unchanged repository produce identical bytes.
    pub source_revision: Option<String>,
    /// ISO 8601 observation timestamp.
    pub observed_at: String,
    /// Closed provider state.
    pub state: GithubState,
    /// Repository description (free text, credential-redacted).
    pub description: Option<String>,
    /// GitHub topics. The namespace `topics` is GitHub's and never
    /// mixes with the catalog's tag list.
    pub topics: Vec<String>,
    /// Languages reported by GitHub. **Never** a build-evidence
    /// input.
    pub languages: Vec<String>,
    /// Default branch.
    pub default_branch: Option<String>,
    /// Archived flag (`true` for archived repositories).
    pub archived: bool,
    /// Workflow filenames present in the repository.
    pub workflows: Vec<String>,
    /// Release tag names. The namespace `releases` is GitHub's and
    /// never mixes with the catalog's tag list.
    pub releases: Vec<String>,
    /// Custom properties (`key=value` pairs reported by the adapter).
    pub custom_properties: Vec<(String, String)>,
    /// Adapter-reported human note (redacted).
    pub note: String,
}
#[derive(Debug, Clone, Deserialize)]
pub(super) struct AdapterObservePayload {
    #[serde(default)]
    pub(super) contract: Option<String>,
    #[serde(default)]
    pub(super) host: Option<String>,
    #[serde(default)]
    pub(super) repository: Option<String>,
    #[serde(default)]
    pub(super) source_revision: Option<String>,
    #[serde(default)]
    pub(super) observed_at: Option<String>,
    #[serde(default)]
    pub(super) state: Option<String>,
    #[serde(default)]
    pub(super) description: Option<String>,
    #[serde(default)]
    pub(super) topics: Vec<String>,
    #[serde(default)]
    pub(super) languages: Vec<String>,
    #[serde(default)]
    pub(super) default_branch: Option<String>,
    #[serde(default)]
    pub(super) archived: Option<bool>,
    #[serde(default)]
    pub(super) workflows: Vec<String>,
    #[serde(default)]
    pub(super) releases: Vec<String>,
    #[serde(default)]
    pub(super) custom_properties: Vec<CustomPropertyPair>,
    #[serde(default)]
    pub(super) rate_limit_reset_at: Option<String>,
    #[serde(default)]
    pub(super) note: Option<String>,
}
/// The closed state vocabulary. Each variant carries the
/// machine-readable state and (where useful) the auxiliary information
/// the adapter returned: a `RateLimited` state carries the reset hint
/// so the caller can decide when to retry, a `Stale` state carries the
/// last observed timestamp, and a `Unavailable` state carries the
/// adapter's reason.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GithubState {
    /// A current observation exists.
    Current,
    /// An observation exists but is beyond the staleness window.
    Stale,
    /// The adapter could not be reached (missing binary, network
    /// failure, or refused connection).
    Unavailable {
        /// Adapter-reported reason, redacted.
        reason: String,
    },
    /// No token is configured. The observation never leaves Forge and
    /// no request is made with an empty credential.
    Unauthorized,
    /// The token is configured but the adapter reports it cannot read
    /// the requested resource.
    Forbidden,
    /// The repository does not exist.
    NotFound,
    /// The GitHub API rate limit was reached; the observation carries
    /// the reset hint and **no partial data**.
    RateLimited {
        /// RFC 3339 reset timestamp as reported by the adapter.
        reset_at: String,
    },
    /// The adapter reported that the response was partial. The
    /// observation never claims completeness.
    Partial,
}
impl GithubState {
    /// Stable kebab-case id used in reports and the catalog source
    /// state field.
    pub fn id(&self) -> &'static str {
        match self {
            GithubState::Current => "current",
            GithubState::Stale => "stale",
            GithubState::Unavailable { .. } => "unavailable",
            GithubState::Unauthorized => "unauthorized",
            GithubState::Forbidden => "forbidden",
            GithubState::NotFound => "not-found",
            GithubState::RateLimited { .. } => "rate-limited",
            GithubState::Partial => "partial",
        }
    }
}
/// What a caller hands to the adapter to observe one or more
/// repositories. The token is read from the environment, never from
/// this struct.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GithubObservationRequest {
    /// GitHub host (e.g. `github.com`).
    pub host: String,
    /// Repository identities (`owner/repo`), one per requested
    /// observation. Empty lists are refused.
    pub repositories: Vec<String>,
}
/// The mutation mode for an approved metadata change. Pull-request
/// mode is the default and produces a reviewable artifact; direct mode
/// requires an explicit confirmation and is **not** the default.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MutationMode {
    /// Open a reviewable pull request on the repository. The default
    /// path; the proposed change is a `forge-github-metadata/0.1.0`
    /// `propose` payload.
    PullRequest,
    /// Apply the change directly to the repository's default branch.
    /// This mode **must** carry a separate confirmation token so a
    /// misconfigured caller cannot push silently. The package never
    /// changes repository settings implicitly.
    Direct {
        /// Caller-supplied confirmation string. The adapter echoes it
        /// back; the call is refused if the strings do not match.
        confirmation: String,
    },
}
#[derive(Debug, Clone, Deserialize)]
pub(super) struct CustomPropertyPair {
    pub(super) key: String,
    pub(super) value: String,
}
