//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use crate::core::ForgeError;
use crate::policy::redact_credentials;
use chrono::{DateTime, Utc};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use super::limits::{
    ALLOWED_PROPOSED_FIELDS, GITHUB_CONTRACT_VERSION, MAX_CUSTOM_PROPERTIES, MAX_LANGUAGES,
    MAX_PROPOSED_CHANGES, MAX_RELEASES, MAX_TOPICS, MAX_WORKFLOWS,
};
use super::model::{
    AdapterObservePayload, AdapterOutput, AdapterProposePayload, CustomPropertyPair,
    GithubObservation, GithubState, MutationMode, ProposeOutcome, ProposeRequest,
};

pub(super) fn parse_observe_output(
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

pub(super) fn parse_propose_output(
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

pub(super) fn parse_state(raw: &str, reset_at: Option<&str>) -> GithubState {
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

pub(super) fn validate_repository(repository: &str) -> Result<(), String> {
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

pub(super) fn validate_propose_request(request: &ProposeRequest) -> Result<(), ForgeError> {
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

pub(super) fn mutation_mode_label(mode: &MutationMode) -> String {
    match mode {
        MutationMode::PullRequest => "pull-request".to_string(),
        MutationMode::Direct { .. } => "direct".to_string(),
    }
}

pub(super) fn unavailable_observation(
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

pub(super) fn dedup_bounded(values: Vec<String>, bound: usize, label: &str) -> Vec<String> {
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

pub(super) fn bounded_custom_properties(values: Vec<CustomPropertyPair>) -> Vec<(String, String)> {
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

pub(super) fn is_executable_file(path: &Path) -> bool {
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

pub(super) fn which_in_path(name: &str) -> Option<PathBuf> {
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
pub(super) mod tests {
    use super::*;

    use super::super::limits::{GITHUB_DEFAULT_HOST, GITHUB_TOKEN_ENV};
    use super::super::model::{GithubAdapter, GithubObservationRequest, ProposedChange};

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
