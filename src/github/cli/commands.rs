//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use crate::policy::redact_credentials;
use crate::process::spawn_with_timeout;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::limits::{GITHUB_CLI_CONTRACT_VERSION, GITHUB_CLI_TIMEOUT};
use super::model::{GhCli, GhOperation, GhOutcome, GhResult, GhVisibility};

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

pub(super) fn classify_failure(stderr: &str) -> GhOutcome {
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

pub(super) fn first_nonempty_line(text: &str) -> String {
    for line in text.lines() {
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }
    text.trim().to_string()
}

pub(super) fn parse_repo_url(stdout: &[u8]) -> Option<String> {
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
pub(super) mod tests {
    use super::*;
    use crate::github::cli::limits::GITHUB_CLI_BIN_ENV;

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
