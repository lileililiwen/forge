//! Vendored `gh`-backed fallback for GitHub metadata observe/propose.
//!
//! Used only when no `forge-github-metadata-adapter` binary is
//! configured but the user's installed `gh` CLI is available.
//! Read-only observe runs `gh repo view --json`; direct single-topic
//! propose runs `gh repo edit --add-topic`. Pull-request mode and
//! every non-topic field stay adapter-only. Tokens are never logged:
//! fallback notes are fixed strings and every free-text value passes
//! through `redact_credentials`.

use crate::core::ForgeError;
use crate::github::adapter::{GithubObservation, GithubState, ProposeOutcome, ProposedChange};
use crate::github::cli::commands::{classify_failure, parse_repository};
use crate::github::cli::{GhCli, GhOutcome};
use crate::policy::redact_credentials;
use crate::process::spawn_with_timeout;
use chrono::Utc;
use std::path::Path;
use std::process::Command;

use super::adapter::GITHUB_CONTRACT_VERSION;
use super::adapter::MAX_TOPICS;
use super::cli::GITHUB_CLI_TIMEOUT;

/// Fields requested from `gh repo view --json` for the fallback read.
pub const FALLBACK_VIEW_FIELDS: &str =
    "nameWithOwner,description,repositoryTopics,defaultBranchRef,isArchived,primaryLanguage";

/// Envelope source label used when the fallback serves a request.
pub const FALLBACK_SOURCE_LABEL: &str = "gh-cli-fallback";

/// Build the bounded `gh repo view` command for one repository.
/// Allowlist: `repo view <owner/repo> --json <fields>`.
pub fn build_observe_command(binary: &Path, repository: &str) -> Command {
    let mut command = Command::new(binary);
    command
        .env("LC_ALL", "C")
        .arg("repo")
        .arg("view")
        .arg(repository)
        .arg("--json")
        .arg(FALLBACK_VIEW_FIELDS);
    command
}

/// Build the bounded `gh repo edit --add-topic` command.
/// Allowlist: `repo edit <owner/repo> --add-topic <topic>`.
pub fn build_topic_edit_command(binary: &Path, repository: &str, topic: &str) -> Command {
    let mut command = Command::new(binary);
    command
        .env("LC_ALL", "C")
        .arg("repo")
        .arg("edit")
        .arg(repository)
        .arg("--add-topic")
        .arg(topic);
    command
}

/// `true` when `changes` is exactly one `topic=<value>` change; returns
/// the trimmed topic value. Every other shape stays adapter-only.
pub fn single_topic_value(changes: &[ProposedChange]) -> Option<String> {
    if changes.len() != 1 {
        return None;
    }
    let change = &changes[0];
    if change.field.trim() != "topic" {
        return None;
    }
    let value = change.new_value.trim().to_string();
    if value.is_empty() || value.contains(char::is_whitespace) {
        return None;
    }
    if redact_credentials(&value).contains("[REDACTED]") {
        return None;
    }
    Some(value)
}

#[derive(Debug, serde::Deserialize)]
struct FallbackView {
    #[serde(default)]
    name_with_owner: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    repository_topics: Option<Vec<FallbackTopic>>,
    #[serde(default)]
    default_branch_ref: Option<FallbackBranch>,
    #[serde(default)]
    is_archived: Option<bool>,
    #[serde(default)]
    primary_language: Option<FallbackLanguage>,
}

#[derive(Debug, serde::Deserialize)]
struct FallbackTopic {
    #[serde(default)]
    name: String,
}

#[derive(Debug, serde::Deserialize)]
struct FallbackBranch {
    #[serde(default)]
    name: String,
}

#[derive(Debug, serde::Deserialize)]
struct FallbackLanguage {
    #[serde(default)]
    name: String,
}

/// Read-only observe of one repository through `gh`.
pub fn observe_one_via_gh(
    cli: &GhCli,
    repository: &str,
    host: &str,
) -> Result<GithubObservation, ForgeError> {
    parse_repository(repository).map_err(|reason| ForgeError::GithubInvalid {
        reason: format!("repository `{repository}`: {reason}"),
    })?;
    let now = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let binary = cli
        .binary
        .clone()
        .ok_or_else(|| ForgeError::GithubAdapterUnavailable {
            reason: format!("no `gh` binary is configured ({})", cli.source),
        })?;
    let mut command = build_observe_command(&binary, repository);
    let execution = match spawn_with_timeout(&mut command, GITHUB_CLI_TIMEOUT) {
        Ok(execution) => execution,
        Err(reason) => {
            let outcome = if reason.contains("timed out") {
                GhOutcome::Timeout
            } else {
                GhOutcome::Unavailable
            };
            return Ok(fallback_unavailable(
                repository, host, &now, outcome, &reason,
            ));
        }
    };
    match execution.exit_code {
        Some(0) => parse_view_stdout(repository, host, &now, &execution.stdout),
        Some(code) => {
            let stderr = execution.stderr.clone();
            let outcome = classify_failure(&stderr);
            let state = gh_outcome_to_state(&outcome, &stderr);
            Ok(GithubObservation {
                contract: GITHUB_CONTRACT_VERSION.to_string(),
                host: host.to_string(),
                repository: repository.to_string(),
                source_revision: None,
                observed_at: now,
                state,
                description: None,
                topics: Vec::new(),
                languages: Vec::new(),
                default_branch: None,
                archived: false,
                workflows: Vec::new(),
                releases: Vec::new(),
                custom_properties: Vec::new(),
                note: format!(
                    "gh fallback: `gh repo view` exited with status {code}; {}",
                    redact_credentials(&first_line(&stderr))
                ),
            })
        }
        None => Ok(fallback_unavailable(
            repository,
            host,
            &now,
            GhOutcome::Timeout,
            "gh fallback timed out",
        )),
    }
}

fn parse_view_stdout(
    repository: &str,
    host: &str,
    now: &str,
    stdout: &[u8],
) -> Result<GithubObservation, ForgeError> {
    let view: FallbackView =
        serde_json::from_slice(stdout).map_err(|err| ForgeError::GithubInvalid {
            reason: format!("gh fallback returned a non-JSON payload: {err}"),
        })?;
    let mut topics: Vec<String> = Vec::new();
    if let Some(list) = view.repository_topics {
        for entry in list {
            let name = entry.name.trim().to_string();
            if name.is_empty() {
                continue;
            }
            if redact_credentials(&name).contains("[REDACTED]") {
                continue;
            }
            let key = name.to_ascii_lowercase();
            if !topics
                .iter()
                .any(|v: &String| v.to_ascii_lowercase() == key)
            {
                topics.push(name);
                if topics.len() >= MAX_TOPICS {
                    break;
                }
            }
        }
    }
    let mut languages: Vec<String> = Vec::new();
    if let Some(lang) = view.primary_language {
        let name = lang.name.trim().to_string();
        if !name.is_empty() && !redact_credentials(&name).contains("[REDACTED]") {
            languages.push(name);
        }
    }
    Ok(GithubObservation {
        contract: GITHUB_CONTRACT_VERSION.to_string(),
        host: host.to_string(),
        repository: repository.to_string(),
        source_revision: None,
        observed_at: now.to_string(),
        state: GithubState::Current,
        description: view
            .description
            .as_deref()
            .map(|v| redact_credentials(v).trim().to_string())
            .filter(|v| !v.is_empty()),
        topics,
        languages,
        default_branch: view
            .default_branch_ref
            .as_ref()
            .map(|b| b.name.trim().to_string())
            .filter(|v| !v.is_empty()),
        archived: view.is_archived.unwrap_or(false),
        workflows: Vec::new(),
        releases: Vec::new(),
        custom_properties: Vec::new(),
        note: format!("observed via {FALLBACK_SOURCE_LABEL} (`gh repo view --json`)"),
    })
}

/// Direct single-topic propose through `gh`. The `confirmation` value
/// is checked for non-emptiness and never copied into the outcome.
pub fn propose_topic_via_gh(
    cli: &GhCli,
    repository: &str,
    topic: &str,
    confirmation: &str,
) -> Result<ProposeOutcome, ForgeError> {
    parse_repository(repository).map_err(|reason| ForgeError::GithubInvalid {
        reason: format!("repository `{repository}`: {reason}"),
    })?;
    if confirmation.trim().is_empty() {
        return Err(ForgeError::GithubInvalid {
            reason: "direct mode requires a non-empty --confirm token; refusing the mutation"
                .to_string(),
        });
    }
    let topic = topic.trim().to_string();
    if topic.is_empty() || topic.contains(char::is_whitespace) {
        return Err(ForgeError::GithubInvalid {
            reason: "proposed topic is empty or contains whitespace; refusing the mutation"
                .to_string(),
        });
    }
    if redact_credentials(&topic).contains("[REDACTED]") {
        return Err(ForgeError::GithubInvalid {
            reason: "proposed topic value looks like a credential; refusing to push it".to_string(),
        });
    }
    let binary = cli
        .binary
        .clone()
        .ok_or_else(|| ForgeError::GithubAdapterUnavailable {
            reason: format!("no `gh` binary is configured ({})", cli.source),
        })?;
    let mut command = build_topic_edit_command(&binary, repository, &topic);
    let execution = match spawn_with_timeout(&mut command, GITHUB_CLI_TIMEOUT) {
        Ok(execution) => execution,
        Err(reason) => {
            let outcome = if reason.contains("timed out") {
                GhOutcome::Timeout
            } else {
                GhOutcome::Unavailable
            };
            return Ok(ProposeOutcome {
                mode: "direct".to_string(),
                artifact_id: None,
                state: gh_outcome_to_state(&outcome, &reason),
                note: format!(
                    "gh fallback: cannot execute `gh repo edit`: {}",
                    redact_credentials(&reason)
                ),
            });
        }
    };
    match execution.exit_code {
        Some(0) => Ok(ProposeOutcome {
            mode: "direct".to_string(),
            artifact_id: None,
            state: GithubState::Current,
            note: "direct topic update via gh (confirmation verified locally; value not logged)"
                .to_string(),
        }),
        Some(code) => {
            let stderr = execution.stderr.clone();
            let outcome = classify_failure(&stderr);
            Ok(ProposeOutcome {
                mode: "direct".to_string(),
                artifact_id: None,
                state: gh_outcome_to_state(&outcome, &stderr),
                note: format!(
                    "gh fallback: `gh repo edit` exited with status {code}; {}",
                    redact_credentials(&first_line(&stderr))
                ),
            })
        }
        None => Ok(ProposeOutcome {
            mode: "direct".to_string(),
            artifact_id: None,
            state: GithubState::Unavailable {
                reason: "gh fallback timed out".to_string(),
            },
            note: "gh fallback: `gh repo edit` timed out; remote may or may not have applied the topic".to_string(),
        }),
    }
}

fn gh_outcome_to_state(outcome: &GhOutcome, detail: &str) -> GithubState {
    let redacted = redact_credentials(&first_line(detail));
    match outcome {
        GhOutcome::Done => GithubState::Current,
        GhOutcome::AuthRequired => GithubState::Unauthorized,
        GhOutcome::Forbidden => GithubState::Forbidden,
        GhOutcome::NotFound => GithubState::NotFound,
        GhOutcome::RateLimited => GithubState::RateLimited {
            reset_at: "unknown".to_string(),
        },
        GhOutcome::Conflict => GithubState::Unavailable {
            reason: format!("gh fallback conflict: {redacted}"),
        },
        GhOutcome::Timeout => GithubState::Unavailable {
            reason: "gh fallback timed out".to_string(),
        },
        GhOutcome::Unavailable => GithubState::Unavailable {
            reason: format!("gh fallback unavailable: {redacted}"),
        },
        GhOutcome::Failed => GithubState::Unavailable {
            reason: format!("gh fallback failed: {redacted}"),
        },
    }
}

fn fallback_unavailable(
    repository: &str,
    host: &str,
    now: &str,
    outcome: GhOutcome,
    reason: &str,
) -> GithubObservation {
    let _ = repository;
    let state = gh_outcome_to_state(&outcome, reason);
    GithubObservation {
        contract: GITHUB_CONTRACT_VERSION.to_string(),
        host: host.to_string(),
        repository: repository.to_string(),
        source_revision: None,
        observed_at: now.to_string(),
        state,
        description: None,
        topics: Vec::new(),
        languages: Vec::new(),
        default_branch: None,
        archived: false,
        workflows: Vec::new(),
        releases: Vec::new(),
        custom_properties: Vec::new(),
        note: format!("gh fallback: {}", redact_credentials(&first_line(reason))),
    }
}

fn first_line(text: &str) -> String {
    for line in text.lines() {
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }
    text.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observe_command_is_allowlisted() {
        let binary = Path::new("/usr/bin/gh");
        let command = build_observe_command(binary, "octocat/hello-world");
        let args: Vec<String> = command
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        assert_eq!(args[0], "repo");
        assert_eq!(args[1], "view");
        assert_eq!(args[2], "octocat/hello-world");
        assert_eq!(args[3], "--json");
        assert!(args[4].contains("repositoryTopics"));
    }

    #[test]
    fn topic_edit_command_is_allowlisted() {
        let binary = Path::new("/usr/bin/gh");
        let command = build_topic_edit_command(binary, "octocat/hello-world", "forge-dev");
        let args: Vec<String> = command
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            args,
            vec![
                "repo",
                "edit",
                "octocat/hello-world",
                "--add-topic",
                "forge-dev"
            ]
        );
    }

    #[test]
    fn single_topic_gate_accepts_only_one_topic() {
        let one = vec![ProposedChange {
            field: "topic".to_string(),
            new_value: "forge-dev".to_string(),
        }];
        assert_eq!(single_topic_value(&one).as_deref(), Some("forge-dev"));
        let two = vec![
            ProposedChange {
                field: "topic".to_string(),
                new_value: "a".to_string(),
            },
            ProposedChange {
                field: "description".to_string(),
                new_value: "b".to_string(),
            },
        ];
        assert!(single_topic_value(&two).is_none());
        let other = vec![ProposedChange {
            field: "description".to_string(),
            new_value: "b".to_string(),
        }];
        assert!(single_topic_value(&other).is_none());
    }

    #[test]
    fn credential_shaped_topic_is_rejected() {
        let bad = vec![ProposedChange {
            field: "topic".to_string(),
            new_value: "github_pat_11ABCDEFG0abcdefghijklmnopqrstuv".to_string(),
        }];
        assert!(single_topic_value(&bad).is_none());
    }
}
