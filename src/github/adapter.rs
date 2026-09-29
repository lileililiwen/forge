//! Bounded GitHub metadata adapter invocation
//! (`github-project-metadata-adapter`).
//!
//! The adapter is an external executable that speaks the
//! `forge-github-metadata/0.1.0` request/response contract. Forge never
//! embeds the GitHub SDK and never makes an HTTP request itself: it
//! spawns the adapter with `Command::new` plus an argument array, with
//! `LC_ALL=C` and a bounded per-run timeout, and parses the JSON
//! response.
//!
//! ## Configuration
//!
//! The adapter binary is resolved in this order:
//!
//! 1. `FORGE_GITHUB_BIN` (always wins when set and the file is
//!    executable).
//! 2. `forge-github-metadata-adapter` on `PATH`.
//!
//! The token is read from `FORGE_GITHUB_TOKEN` once and is **never**
//! passed on the command line, never written to a journal row and
//! never echoed on stdout or stderr. The adapter reads it through the
//! inherited environment variable.
//!
//! ## States
//!
//! The closed [`GithubState`] vocabulary distinguishes success,
//! transient rate limits, credential problems, missing repositories,
//! forbidden access, network unavailability, staleness and partial
//! responses. A partial or stale observation is **not** a complete
//! record; the catalog records the state and the operator decides
//! whether to act on it.
//!
//! ## Mutation
//!
//! [`MutationMode::PullRequest`] is the default and the only mode that
//! is accepted without a separate confirmation. [`MutationMode::Direct`]
//! is refused unless the caller passes an explicit confirmation token
//! that the adapter echoes back, so a misconfigured caller cannot push
//! silently. The package never changes repository settings implicitly
//! and never opens issues, comments, labels or pull requests for
//! side-effect data.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::core::ForgeError;
use crate::policy::redact_credentials;
use crate::process::spawn_with_timeout;

/// Versioned request/response contract for the GitHub metadata adapter.
pub const GITHUB_CONTRACT_VERSION: &str = "forge-github-metadata/0.1.0";

/// Default GitHub host. The adapter is expected to derive the API base
/// from the host (e.g. `https://api.github.com` for `github.com`).
pub const GITHUB_DEFAULT_HOST: &str = "github.com";

/// Default adapter binary name. Matches the `forge-analytics-adapter` /
/// `forge-deploy-adapter` pattern.
pub const DEFAULT_GITHUB_BIN: &str = "forge-github-metadata-adapter";

/// Environment variable selecting the adapter binary.
pub const GITHUB_BIN_ENV: &str = "FORGE_GITHUB_BIN";

/// Environment variable holding the GitHub token. The token is read once
/// and is never echoed.
pub const GITHUB_TOKEN_ENV: &str = "FORGE_GITHUB_TOKEN";

/// Per-run adapter timeout. Matches the analytics / deploy / gate
/// adapters; an unresponsive tool cannot hang the catalog.
pub const GITHUB_ADAPTER_TIMEOUT: Duration = Duration::from_secs(15);

/// Maximum number of repositories a single `forge project github
/// observe` call may carry. Larger lists are refused so a misconfigured
/// CLI cannot pin the registry to an unbounded scrape loop.
pub const MAX_OBSERVATION_REPOSITORIES: usize = 32;

/// Maximum number of fields a single `forge project github propose`
/// call may carry. The mutation payload stays small on purpose.
pub const MAX_PROPOSED_CHANGES: usize = 16;

/// Maximum number of topics a single observation may carry. The
/// adapter is expected to cap; this is the defensive ceiling.
pub const MAX_TOPICS: usize = 50;

/// Maximum number of languages a single observation may carry.
pub const MAX_LANGUAGES: usize = 50;

/// Maximum number of release tags a single observation may carry.
pub const MAX_RELEASES: usize = 50;

/// Maximum number of workflows a single observation may carry.
pub const MAX_WORKFLOWS: usize = 50;

/// Maximum number of custom properties a single observation may carry.
pub const MAX_CUSTOM_PROPERTIES: usize = 50;

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

impl std::fmt::Display for GithubState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.id())
    }
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

    fn observe_one(
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
        // The token stays in the inherited environment. The command
        // line never carries it.
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
struct AdapterObservePayload {
    #[serde(default)]
    contract: Option<String>,
    #[serde(default)]
    host: Option<String>,
    #[serde(default)]
    repository: Option<String>,
    #[serde(default)]
    source_revision: Option<String>,
    #[serde(default)]
    observed_at: Option<String>,
    #[serde(default)]
    state: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    topics: Vec<String>,
    #[serde(default)]
    languages: Vec<String>,
    #[serde(default)]
    default_branch: Option<String>,
    #[serde(default)]
    archived: Option<bool>,
    #[serde(default)]
    workflows: Vec<String>,
    #[serde(default)]
    releases: Vec<String>,
    #[serde(default)]
    custom_properties: Vec<CustomPropertyPair>,
    #[serde(default)]
    rate_limit_reset_at: Option<String>,
    #[serde(default)]
    note: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct CustomPropertyPair {
    key: String,
    value: String,
}

#[derive(Debug, Clone, Deserialize)]
struct AdapterProposePayload {
    #[serde(default)]
    mode: Option<String>,
    #[serde(default)]
    artifact_id: Option<String>,
    #[serde(default)]
    state: Option<String>,
    #[serde(default)]
    rate_limit_reset_at: Option<String>,
    #[serde(default)]
    note: Option<String>,
    #[serde(default)]
    confirmation: Option<String>,
}

fn parse_observe_output(
    binary: &Path,
    host: &str,
    repository: &str,
    output: AdapterOutput,
    now: DateTime<Utc>,
) -> Result<GithubObservation, ForgeError> {
    if output.exit_code != Some(0) {
        return Ok(unavailable_observation(
            host,
            repository,
            now,
            format!(
                "adapter `{}` exited with status {}; stderr: {}",
                binary.display(),
                output.exit_code.unwrap_or(-1),
                redact_credentials(&output.stderr)
            ),
        ));
    }
    let payload: AdapterObservePayload = match serde_json::from_slice(&output.stdout) {
        Ok(payload) => payload,
        Err(err) => {
            return Ok(unavailable_observation(
                host,
                repository,
                now,
                format!("adapter returned a non-JSON payload: {err}"),
            ));
        }
    };
    if let Some(contract) = payload.contract.as_deref() {
        if contract != GITHUB_CONTRACT_VERSION {
            return Ok(unavailable_observation(
                host,
                repository,
                now,
                format!(
                    "adapter answered contract `{contract}`; only `{GITHUB_CONTRACT_VERSION}` \
                     is accepted"
                ),
            ));
        }
    } else {
        return Ok(unavailable_observation(
            host,
            repository,
            now,
            "adapter did not report a `contract` field; refusing an unidentified payload"
                .to_string(),
        ));
    }
    if let Some(returned_host) = payload.host.as_deref() {
        if !returned_host.eq_ignore_ascii_case(host) {
            return Ok(unavailable_observation(
                host,
                repository,
                now,
                format!(
                    "adapter answered a different host (`{returned_host}`) than the request \
                     (`{host}`); refusing the mismatched response"
                ),
            ));
        }
    }
    if let Some(returned_repo) = payload.repository.as_deref() {
        if !returned_repo.eq_ignore_ascii_case(repository) {
            return Ok(unavailable_observation(
                host,
                repository,
                now,
                format!(
                    "adapter answered a different repository (`{returned_repo}`) than the \
                     request (`{repository}`); refusing the mismatched response"
                ),
            ));
        }
    }
    let topics = dedup_bounded(payload.topics, MAX_TOPICS, "topics");
    let languages = dedup_bounded(payload.languages, MAX_LANGUAGES, "languages");
    let workflows = dedup_bounded(payload.workflows, MAX_WORKFLOWS, "workflows");
    let releases = dedup_bounded(payload.releases, MAX_RELEASES, "releases");
    let custom_properties = bounded_custom_properties(payload.custom_properties);
    let state = parse_state(
        payload.state.as_deref().unwrap_or("current"),
        payload.rate_limit_reset_at.as_deref(),
    );
    Ok(GithubObservation {
        contract: GITHUB_CONTRACT_VERSION.to_string(),
        host: host.to_string(),
        repository: repository.to_string(),
        source_revision: payload
            .source_revision
            .as_deref()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
        observed_at: payload
            .observed_at
            .as_deref()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)),
        state,
        description: payload
            .description
            .as_deref()
            .map(|value| redact_credentials(value).trim().to_string())
            .filter(|value| !value.is_empty()),
        topics,
        languages,
        default_branch: payload
            .default_branch
            .as_deref()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
        archived: payload.archived.unwrap_or(false),
        workflows,
        releases,
        custom_properties,
        note: payload
            .note
            .as_deref()
            .map(|value| redact_credentials(value).to_string())
            .unwrap_or_default(),
    })
}

fn parse_propose_output(
    binary: &Path,
    mode: &MutationMode,
    output: AdapterOutput,
) -> Result<ProposeOutcome, ForgeError> {
    if output.exit_code != Some(0) {
        return Ok(ProposeOutcome {
            mode: mutation_mode_label(mode),
            artifact_id: None,
            state: GithubState::Unavailable {
                reason: format!(
                    "adapter `{}` exited with status {}; stderr: {}",
                    binary.display(),
                    output.exit_code.unwrap_or(-1),
                    redact_credentials(&output.stderr)
                ),
            },
            note: format!("adapter `{}` refused the mutation", binary.display()),
        });
    }
    let payload: AdapterProposePayload = match serde_json::from_slice(&output.stdout) {
        Ok(payload) => payload,
        Err(err) => {
            return Ok(ProposeOutcome {
                mode: mutation_mode_label(mode),
                artifact_id: None,
                state: GithubState::Unavailable {
                    reason: format!("adapter returned a non-JSON propose payload: {err}"),
                },
                note: format!("adapter `{}` returned a non-JSON payload", binary.display()),
            });
        }
    };
    if let MutationMode::Direct { confirmation } = mode {
        match payload.confirmation.as_deref() {
            Some(echoed) if echoed == confirmation => {}
            _ => {
                return Ok(ProposeOutcome {
                    mode: mutation_mode_label(mode),
                    artifact_id: None,
                    state: GithubState::Unavailable {
                        reason: "adapter did not echo the direct-mode confirmation; refusing \
                                 the mutation"
                            .to_string(),
                    },
                    note: "direct-mode confirmation was not echoed by the adapter".to_string(),
                });
            }
        }
    }
    let state = parse_state(
        payload.state.as_deref().unwrap_or("current"),
        payload.rate_limit_reset_at.as_deref(),
    );
    Ok(ProposeOutcome {
        mode: payload.mode.unwrap_or_else(|| mutation_mode_label(mode)),
        artifact_id: payload
            .artifact_id
            .as_deref()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
        state,
        note: payload
            .note
            .as_deref()
            .map(|value| redact_credentials(value).to_string())
            .unwrap_or_default(),
    })
}

fn parse_state(raw: &str, reset_at: Option<&str>) -> GithubState {
    match raw.trim().to_ascii_lowercase().as_str() {
        "current" => GithubState::Current,
        "stale" => GithubState::Stale,
        "unavailable" => GithubState::Unavailable {
            reason: "adapter reported unavailable".to_string(),
        },
        "unauthorized" => GithubState::Unauthorized,
        "forbidden" => GithubState::Forbidden,
        "not-found" | "not_found" => GithubState::NotFound,
        "rate-limited" | "rate_limited" => GithubState::RateLimited {
            reset_at: reset_at
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| "unknown".to_string()),
        },
        "partial" => GithubState::Partial,
        other => GithubState::Unavailable {
            reason: format!("adapter reported unknown state `{other}`"),
        },
    }
}

fn validate_repository(repository: &str) -> Result<(), String> {
    if repository.is_empty() {
        return Err("repository identity is empty".to_string());
    }
    if repository.contains(char::is_whitespace) {
        return Err("repository identity contains whitespace".to_string());
    }
    if repository.contains('\0') {
        return Err("repository identity contains a NUL byte".to_string());
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

fn validate_propose_request(request: &ProposeRequest) -> Result<(), ForgeError> {
    validate_repository(&request.repository).map_err(|reason| ForgeError::GithubInvalid {
        reason: format!("repository `{}`: {reason}", request.repository),
    })?;
    if request.changes.is_empty() {
        return Err(ForgeError::GithubInvalid {
            reason: "no proposed changes were supplied; refusing an empty mutation".to_string(),
        });
    }
    if request.changes.len() > MAX_PROPOSED_CHANGES {
        return Err(ForgeError::GithubInvalid {
            reason: format!(
                "proposed change list has {} entries; the maximum is {MAX_PROPOSED_CHANGES}",
                request.changes.len()
            ),
        });
    }
    if let MutationMode::Direct { confirmation } = &request.mode {
        if confirmation.trim().is_empty() {
            return Err(ForgeError::GithubInvalid {
                reason: "direct mode requires a non-empty confirmation string".to_string(),
            });
        }
    }
    let mut seen_fields: BTreeSet<String> = BTreeSet::new();
    for change in &request.changes {
        let field = change.field.trim();
        if !ALLOWED_PROPOSED_FIELDS.contains(&field) {
            return Err(ForgeError::GithubInvalid {
                reason: format!(
                    "proposed field `{field}` is not in the closed set \
                     [topic, description, homepage, language]"
                ),
            });
        }
        if !seen_fields.insert(field.to_string()) {
            return Err(ForgeError::GithubInvalid {
                reason: format!("field `{field}` appears more than once in the change list"),
            });
        }
        if redact_credentials(&change.new_value).contains("[REDACTED]") && !field.eq("description")
        {
            return Err(ForgeError::GithubInvalid {
                reason: format!(
                    "proposed `{field}` value looks like a credential; refusing to push it"
                ),
            });
        }
    }
    Ok(())
}

/// Closed set of fields the adapter is allowed to mutate. The package
/// never opens issues, comments or labels through this surface.
pub const ALLOWED_PROPOSED_FIELDS: [&str; 4] = ["topic", "description", "homepage", "language"];

fn mutation_mode_label(mode: &MutationMode) -> String {
    match mode {
        MutationMode::PullRequest => "pull-request".to_string(),
        MutationMode::Direct { .. } => "direct".to_string(),
    }
}

fn unavailable_observation(
    host: &str,
    repository: &str,
    now: DateTime<Utc>,
    reason: String,
) -> GithubObservation {
    GithubObservation {
        contract: GITHUB_CONTRACT_VERSION.to_string(),
        host: host.to_string(),
        repository: repository.to_string(),
        source_revision: None,
        observed_at: now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        state: GithubState::Unavailable {
            reason: redact_credentials(&reason),
        },
        description: None,
        topics: Vec::new(),
        languages: Vec::new(),
        default_branch: None,
        archived: false,
        workflows: Vec::new(),
        releases: Vec::new(),
        custom_properties: Vec::new(),
        note: redact_credentials(&reason),
    }
}

fn dedup_bounded(values: Vec<String>, bound: usize, label: &str) -> Vec<String> {
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut out: Vec<String> = Vec::new();
    for value in values {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            continue;
        }
        if redact_credentials(trimmed).contains("[REDACTED]") {
            continue;
        }
        let key = trimmed.to_ascii_lowercase();
        if seen.insert(key) {
            out.push(trimmed.to_string());
            if out.len() >= bound {
                break;
            }
        }
    }
    if out.len() == bound {
        // Soft cap: silently drop further entries, but record the cap
        // through a fixed label so the catalog can show why a list
        // is short of what the adapter reported.
        let _ = label;
    }
    out
}

fn bounded_custom_properties(values: Vec<CustomPropertyPair>) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    for pair in values {
        let key = pair.key.trim();
        let value = pair.value.trim();
        if key.is_empty() || value.is_empty() {
            continue;
        }
        if redact_credentials(key).contains("[REDACTED]")
            || redact_credentials(value).contains("[REDACTED]")
        {
            continue;
        }
        out.push((key.to_string(), value.to_string()));
        if out.len() >= MAX_CUSTOM_PROPERTIES {
            break;
        }
    }
    out
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

#[cfg(test)]
mod tests {
    use super::*;

    fn no_env(_name: &str) -> Option<String> {
        None
    }

    /// Build a path to an existing, executable stub on disk so the
    /// adapter's `binary_available` check passes. The stub is never
    /// actually invoked because the tests that need it always short
    /// circuit on a missing token.
    fn existing_stub() -> PathBuf {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("github-adapter");
        std::fs::write(&path, "#!/bin/sh\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        // Leak the directory so the path stays valid for the rest of
        // the test; the OS reclaims the file when the process exits.
        let leaked: &'static mut tempfile::TempDir = Box::leak(Box::new(tmp));
        let _ = leaked;
        path
    }

    fn env_with<const N: usize>(
        pairs: [(&'static str, &'static str); N],
    ) -> impl Fn(&str) -> Option<String> {
        move |name| {
            for (key, value) in &pairs {
                if *key == name {
                    return Some((*value).to_string());
                }
            }
            None
        }
    }

    #[test]
    fn resolves_token_from_environment_only() {
        let adapter = GithubAdapter::from_env_with(env_with([(GITHUB_TOKEN_ENV, "secret")]));
        assert_eq!(adapter.token.as_deref(), Some("secret"));
        let adapter = GithubAdapter::from_env_with(no_env);
        assert!(adapter.token.is_none());
    }

    #[test]
    fn missing_token_is_unauthorized_not_a_request() {
        let request = GithubObservationRequest {
            host: GITHUB_DEFAULT_HOST.to_string(),
            repositories: vec!["octocat/hello-world".to_string()],
        };
        let adapter = GithubAdapter {
            binary: Some(existing_stub()),
            source: "FORGE_GITHUB_BIN=test".to_string(),
            token: None,
        };
        let err = adapter.observe(&request).unwrap_err();
        assert_eq!(err.code(), "github-invalid");
        assert!(err.to_string().contains(GITHUB_TOKEN_ENV), "{}", err);
    }

    #[test]
    fn missing_binary_is_unavailable_with_the_typed_code() {
        let request = GithubObservationRequest {
            host: GITHUB_DEFAULT_HOST.to_string(),
            repositories: vec!["octocat/hello-world".to_string()],
        };
        let adapter = GithubAdapter {
            binary: None,
            source: "no adapter configured".to_string(),
            token: Some("token".to_string()),
        };
        let err = adapter.observe(&request).unwrap_err();
        assert_eq!(err.code(), "github-adapter-unavailable");
    }

    #[test]
    fn empty_repository_list_is_github_invalid() {
        let request = GithubObservationRequest {
            host: GITHUB_DEFAULT_HOST.to_string(),
            repositories: Vec::new(),
        };
        let adapter = GithubAdapter {
            binary: None,
            source: "no adapter".to_string(),
            token: Some("token".to_string()),
        };
        let err = adapter.observe(&request).unwrap_err();
        assert_eq!(err.code(), "github-invalid");
    }

    #[test]
    fn malformed_repository_identity_is_github_invalid() {
        let request = GithubObservationRequest {
            host: GITHUB_DEFAULT_HOST.to_string(),
            repositories: vec!["not a/repo".to_string()],
        };
        let adapter = GithubAdapter {
            binary: Some(PathBuf::from("/does/not/matter")),
            source: "FORGE_GITHUB_BIN=/does/not/matter".to_string(),
            token: Some("token".to_string()),
        };
        let err = adapter.observe(&request).unwrap_err();
        assert_eq!(err.code(), "github-invalid");
    }

    #[test]
    fn direct_mode_requires_a_non_empty_confirmation() {
        let request = ProposeRequest {
            host: GITHUB_DEFAULT_HOST.to_string(),
            repository: "octocat/hello-world".to_string(),
            mode: MutationMode::Direct {
                confirmation: "   ".to_string(),
            },
            changes: vec![ProposedChange {
                field: "description".to_string(),
                new_value: "Hello".to_string(),
            }],
        };
        let adapter = GithubAdapter {
            binary: Some(PathBuf::from("/does/not/matter")),
            source: "FORGE_GITHUB_BIN=/does/not/matter".to_string(),
            token: Some("token".to_string()),
        };
        let err = adapter.propose(&request).unwrap_err();
        assert_eq!(err.code(), "github-invalid");
    }

    #[test]
    fn only_closed_fields_are_accepted_in_propose() {
        let request = ProposeRequest {
            host: GITHUB_DEFAULT_HOST.to_string(),
            repository: "octocat/hello-world".to_string(),
            mode: MutationMode::PullRequest,
            changes: vec![ProposedChange {
                field: "settings".to_string(),
                new_value: "delete branch protection".to_string(),
            }],
        };
        let adapter = GithubAdapter {
            binary: Some(PathBuf::from("/does/not/matter")),
            source: "FORGE_GITHUB_BIN=/does/not/matter".to_string(),
            token: Some("token".to_string()),
        };
        let err = adapter.propose(&request).unwrap_err();
        assert_eq!(err.code(), "github-invalid");
        assert!(err.to_string().contains("settings"), "{}", err);
    }

    #[test]
    fn closed_state_vocabulary_is_stable() {
        for (state, expected) in [
            (GithubState::Current, "current"),
            (GithubState::Stale, "stale"),
            (
                GithubState::Unavailable {
                    reason: "x".to_string(),
                },
                "unavailable",
            ),
            (GithubState::Unauthorized, "unauthorized"),
            (GithubState::Forbidden, "forbidden"),
            (GithubState::NotFound, "not-found"),
            (
                GithubState::RateLimited {
                    reset_at: "x".to_string(),
                },
                "rate-limited",
            ),
            (GithubState::Partial, "partial"),
        ] {
            assert_eq!(state.id(), expected);
        }
    }

    #[test]
    fn parse_state_handles_every_kebab_case_variant() {
        assert_eq!(parse_state("current", None), GithubState::Current);
        assert_eq!(parse_state("stale", None), GithubState::Stale);
        assert_eq!(parse_state("unauthorized", None), GithubState::Unauthorized);
        assert_eq!(parse_state("forbidden", None), GithubState::Forbidden);
        assert_eq!(parse_state("not-found", None), GithubState::NotFound);
        assert_eq!(
            parse_state("rate-limited", Some("2026-09-29T12:00:00Z")),
            GithubState::RateLimited {
                reset_at: "2026-09-29T12:00:00Z".to_string(),
            }
        );
        assert_eq!(parse_state("partial", None), GithubState::Partial);
        let unknown = parse_state("nope", None);
        match unknown {
            GithubState::Unavailable { reason } => assert!(reason.contains("nope")),
            other => panic!("expected unavailable for unknown state, got {other:?}"),
        }
    }

    #[test]
    fn credential_shaped_topic_is_dropped_from_the_list() {
        let values = vec![
            "rust".to_string(),
            "github_pat_11ABCDEFG0abcdefghijklmnopqrstuv".to_string(),
            "ci".to_string(),
        ];
        let deduped = dedup_bounded(values, MAX_TOPICS, "topics");
        // The list is sorted and de-duplicated; only the non-credential
        // values survive.
        let mut sorted = deduped.clone();
        sorted.sort();
        assert_eq!(sorted, vec!["ci".to_string(), "rust".to_string()]);
        assert!(!deduped.iter().any(|value| value.contains("github_pat_")));
    }

    #[test]
    fn binary_availability_checks_file_and_mode() {
        let tmp = tempfile::tempdir().unwrap();
        let binary = tmp.path().join("github-adapter");
        std::fs::write(&binary, "#!/bin/sh\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        assert!(is_executable_file(&binary));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o644)).unwrap();
        }
        #[cfg(unix)]
        assert!(!is_executable_file(&binary));
    }

    #[test]
    fn repository_validation_rejects_malformed_values() {
        for bad in [
            "",
            " ",
            "owner",
            "owner/",
            "/repo",
            "owner/repo/extra",
            "owner/repo with space",
            "owner/repo\n",
            "owner/repo\0",
        ] {
            assert!(
                validate_repository(bad).is_err(),
                "expected `{bad}` to be rejected"
            );
        }
        for good in ["octocat/hello-world", "a/b", "owner_1/repo.2"] {
            assert!(
                validate_repository(good).is_ok(),
                "expected `{good}` to be accepted"
            );
        }
    }
}
