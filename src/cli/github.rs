//! GitHub CLI commands (`github`).
//!
//! Typed CLI handlers over the `gh` CLI adapter.
//! Bodies moved verbatim from the split of `src/main.rs`.

use forge::core::ForgeError;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::projects::as_output;
use crate::{Format, Output};

pub(super) fn cmd_github_cli_auth(host: &str, format: Format) -> Result<Output, ForgeError> {
    use forge::github::{run_auth, GhCli};
    let host_trimmed = host.trim();
    let host = if host_trimmed.is_empty() {
        forge::github::GITHUB_CLI_DEFAULT_HOST
    } else {
        host_trimmed
    };
    let cli = GhCli::from_env();
    let result = run_auth(&cli, host);
    let json = serde_json::json!({
        "contract": forge::github::GITHUB_CLI_CONTRACT_VERSION,
        "operation": result.operation.id(),
        "outcome": result.outcome.id(),
        "exit_code": result.exit_code,
        "stderr_tail": result.stderr_tail,
        "note": result.note,
        "cli_source": cli.source,
    });
    let human = format!(
        "forge project github auth — contract {}\n\
         cli_source={}\n\
         outcome={}\n",
        forge::github::GITHUB_CLI_CONTRACT_VERSION,
        cli.source,
        result.outcome.id(),
    );
    if !result.note.is_empty() {
        return Err(result.to_error("github cli auth"));
    }
    Ok(as_output(format, human, json))
}

pub(super) fn cmd_github_cli_clone(
    _db_path: &Path,
    repository: &str,
    destination: &Path,
    confirm: bool,
    format: Format,
) -> Result<Output, ForgeError> {
    use forge::github::{parse_repository, run_clone, GhCli, GITHUB_CLI_CONTRACT_VERSION};
    if !confirm {
        return Err(ForgeError::GithubCliInvalid {
            reason: format!(
                "github cli clone requires explicit --confirm; refusing to clone `{repository}` without it"
            ),
        });
    }
    parse_repository(repository).map_err(|reason| ForgeError::GithubCliInvalid {
        reason: format!("repository `{repository}`: {reason}"),
    })?;
    if destination.exists() {
        if destination.is_dir() {
            let read =
                std::fs::read_dir(destination).map_err(|err| ForgeError::GithubCliConflict {
                    reason: format!(
                        "destination `{}` is an unreadable existing directory: {err}",
                        destination.display()
                    ),
                })?;
            if read.count() > 0 {
                return Err(ForgeError::GithubCliConflict {
                    reason: format!(
                        "destination `{}` already exists and is not empty; refusing to clone over it",
                        destination.display()
                    ),
                });
            }
        } else {
            return Err(ForgeError::GithubCliConflict {
                reason: format!(
                    "destination `{}` already exists and is not a directory",
                    destination.display()
                ),
            });
        }
    }
    let cli = GhCli::from_env();
    let result = run_clone(&cli, repository, destination);
    let artifact = result.artifact_url.clone();
    let json = serde_json::json!({
        "contract": GITHUB_CLI_CONTRACT_VERSION,
        "operation": result.operation.id(),
        "outcome": result.outcome.id(),
        "exit_code": result.exit_code,
        "repository": repository,
        "destination": destination.display().to_string(),
        "artifact_url": artifact,
        "stderr_tail": result.stderr_tail,
        "note": result.note,
        "cli_source": cli.source,
    });
    if result.outcome != forge::github::GhOutcome::Done {
        return Err(clone_error(&result));
    }
    let human = format!(
        "forge project github clone — contract {}\n\
         cli_source={}\n\
         outcome=done\n\
         repository={}\n\
         destination={}\n",
        GITHUB_CLI_CONTRACT_VERSION,
        cli.source,
        repository,
        destination.display(),
    );
    Ok(as_output(format, human, json))
}

fn clone_error(result: &forge::github::GhResult) -> ForgeError {
    use forge::github::{GhOutcome, GhResult};
    let ctx = format!("github cli clone {}", result.repository());
    match result {
        GhResult {
            outcome: GhOutcome::Done,
            ..
        } => ForgeError::GithubCliInvalid {
            reason: "internal: done outcome cannot be turned into an error".to_string(),
        },
        _ => result.to_error(&ctx),
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn cmd_github_cli_create(
    db_path: &Path,
    project: &str,
    repo: &str,
    visibility: &str,
    confirm_public: bool,
    push_source: bool,
    confirm: bool,
    register_if_missing: bool,
    format: Format,
) -> Result<Output, ForgeError> {
    use forge::github::{
        parse_repository, run_create, GhCli, GhOutcome, GhVisibility, GITHUB_CLI_CONTRACT_VERSION,
    };
    if !confirm {
        return Err(ForgeError::GithubCliInvalid {
            reason: format!(
                "github cli create requires explicit --confirm; refusing to create the remote for `{project}` without it"
            ),
        });
    }
    parse_repository(repo).map_err(|reason| ForgeError::GithubCliInvalid {
        reason: format!("--repo `{repo}`: {reason}"),
    })?;
    let visibility = match visibility.trim() {
        "private" => GhVisibility::Private,
        "public" => GhVisibility::Public,
        other => {
            return Err(ForgeError::GithubCliInvalid {
                reason: format!(
                    "unknown --visibility `{other}`; expected `private` (default) or `public`"
                ),
            });
        }
    };
    if matches!(visibility, GhVisibility::Public) && !confirm_public {
        return Err(ForgeError::GithubCliInvalid {
            reason: "github cli create with --visibility public requires --confirm-public; refusing to publish a public repository without it".to_string(),
        });
    }
    if push_source {
        // The CLI confirm flag is the same one we already required;
        // when --push-source is set, the caller is explicitly opting
        // in to the initial push. The `--confirm` flag remains the
        // single gate.
    }
    let project_path = PathBuf::from(project);
    let source = if project_path.exists() && project_path.is_dir() {
        project_path
    } else {
        return Err(ForgeError::GithubCliInvalid {
            reason: format!(
                "github cli create expects an existing local project directory; `{project}` is not a directory"
            ),
        });
    };
    let canonical = source
        .canonicalize()
        .map_err(|_| ForgeError::GithubCliInvalid {
            reason: format!(
                "github cli create cannot resolve `{}` to a canonical path",
                source.display()
            ),
        })?;
    let mut registered = false;
    if register_if_missing {
        registered = ensure_project_registered(db_path, &canonical)?;
    }
    let cli = GhCli::from_env();
    let name = repo.split_once('/').map(|(_, n)| n).unwrap_or(repo);
    let result = run_create(&cli, name, &source, visibility, push_source);
    let outcome = result.outcome.clone();
    let json = serde_json::json!({
        "contract": GITHUB_CLI_CONTRACT_VERSION,
        "operation": result.operation.id(),
        "outcome": outcome.id(),
        "exit_code": result.exit_code,
        "repository": repo,
        "visibility": visibility.id(),
        "push_source": push_source,
        "source": source.display().to_string(),
        "artifact_url": result.artifact_url,
        "stderr_tail": result.stderr_tail,
        "note": result.note,
        "cli_source": cli.source,
        "register_if_missing": register_if_missing,
        "registered": registered,
    });
    if outcome != GhOutcome::Done {
        return Err(create_error(&result));
    }
    let human = format!(
        "forge project github create — contract {}\n\
         cli_source={}\n\
         outcome=done\n\
         repository={}\n\
         visibility={}\n\
         push_source={}\n\
         registered={}\n",
        GITHUB_CLI_CONTRACT_VERSION,
        cli.source,
        repo,
        visibility.id(),
        push_source,
        registered,
    );
    Ok(as_output(format, human, json))
}

/// Validate `forge.yaml` and register the canonical path when it is
/// not yet in the registry. Returns `true` when a registration write
/// happened, `false` when the project was already registered.
fn ensure_project_registered(db_path: &Path, canonical: &Path) -> Result<bool, ForgeError> {
    let canonical_text = canonical.display().to_string();
    let already = (|| -> Result<bool, ForgeError> {
        if !db_path.is_file() {
            return Ok(false);
        }
        let registry = forge::registry::Registry::open_read_only(db_path)?;
        match registry.inspect(&canonical_text) {
            Ok(_) => Ok(true),
            Err(ForgeError::UnknownProject { .. }) => Ok(false),
            Err(err) => Err(err),
        }
    })()?;
    if already {
        return Ok(false);
    }
    let mut registry = forge::registry::Registry::open(db_path)?;
    match registry.register(canonical, None) {
        Ok(_) => Ok(true),
        Err(ForgeError::IdCollision { id }) => Err(ForgeError::GithubCliConflict {
            reason: format!(
                "register-if-missing: project id `{id}` is already registered at another path; resolve the collision before creating the remote"
            ),
        }),
        Err(ForgeError::PathCollision { path }) => Err(ForgeError::GithubCliConflict {
            reason: format!(
                "register-if-missing: path `{path}` is already registered under another id; resolve the collision before creating the remote"
            ),
        }),
        Err(err) => Err(ForgeError::GithubCliInvalid {
            reason: format!("register-if-missing: invalid manifest: {err}"),
        }),
    }
}

fn create_error(result: &forge::github::GhResult) -> ForgeError {
    use forge::github::{GhOutcome, GhResult};
    let ctx = format!("github cli create {}", result.repository());
    match result {
        GhResult {
            outcome: GhOutcome::Done,
            ..
        } => ForgeError::GithubCliInvalid {
            reason: "internal: done outcome cannot be turned into an error".to_string(),
        },
        _ => result.to_error(&ctx),
    }
}

pub(super) fn cmd_github_cli_pull_request(
    _db_path: &Path,
    project: &str,
    title: &str,
    body: &str,
    draft: bool,
    confirm: bool,
    format: Format,
) -> Result<Output, ForgeError> {
    use forge::github::{
        run_pull_request, GhCli, GhOutcome, GITHUB_CLI_CONTRACT_VERSION, MAX_BODY_BYTES,
        MAX_TITLE_BYTES,
    };
    if !confirm {
        return Err(ForgeError::GithubCliInvalid {
            reason: format!(
                "github cli pull-request requires explicit --confirm; refusing to open a PR for `{project}` without it"
            ),
        });
    }
    if title.trim().is_empty() {
        return Err(ForgeError::GithubCliInvalid {
            reason: "--title is required and must not be empty".to_string(),
        });
    }
    if title.len() > MAX_TITLE_BYTES {
        return Err(ForgeError::GithubCliInvalid {
            reason: format!(
                "--title exceeds the {MAX_TITLE_BYTES}-byte cap (was {} bytes)",
                title.len()
            ),
        });
    }
    if body.len() > MAX_BODY_BYTES {
        return Err(ForgeError::GithubCliInvalid {
            reason: format!(
                "--body exceeds the {MAX_BODY_BYTES}-byte cap (was {} bytes)",
                body.len()
            ),
        });
    }
    if std::path::Path::new(project).exists() {
        // Local working tree preflight: clean tree + remote present.
        let tree = Command::new("git")
            .arg("-C")
            .arg(project)
            .args(["status", "--porcelain"])
            .output();
        match tree {
            Ok(output) if output.status.success() => {
                let porcelain = String::from_utf8_lossy(&output.stdout);
                if !porcelain.trim().is_empty() {
                    return Err(ForgeError::GithubCliConflict {
                        reason: format!(
                            "github cli pull-request requires a clean working tree; `{project}` has uncommitted changes"
                        ),
                    });
                }
            }
            _ => {
                return Err(ForgeError::GithubCliConflict {
                    reason: format!(
                        "github cli pull-request requires `{project}` to be a git working tree"
                    ),
                });
            }
        }
        let remote = Command::new("git")
            .arg("-C")
            .arg(project)
            .args(["remote", "get-url", "origin"])
            .output();
        match remote {
            Ok(output) if output.status.success() => {
                let url = String::from_utf8_lossy(&output.stdout);
                let url = url.trim();
                if url.is_empty() {
                    return Err(ForgeError::GithubCliConflict {
                        reason: format!(
                            "github cli pull-request requires `{project}` to have an `origin` remote pointing at GitHub"
                        ),
                    });
                }
                if !url.contains("github.com") && !url.contains("github:") {
                    return Err(ForgeError::GithubCliConflict {
                        reason: format!(
                            "github cli pull-request requires the `origin` remote to point at GitHub; `{project}` points at `{url}`"
                        ),
                    });
                }
            }
            _ => {
                return Err(ForgeError::GithubCliConflict {
                    reason: format!(
                        "github cli pull-request requires `{project}` to have an `origin` remote"
                    ),
                });
            }
        }
    } else {
        return Err(ForgeError::GithubCliInvalid {
            reason: format!(
                "github cli pull-request expects an existing local project directory; `{project}` is not a directory"
            ),
        });
    }
    let cli = GhCli::from_env();
    let result = run_pull_request(&cli, title, body, draft);
    let outcome = result.outcome.clone();
    let json = serde_json::json!({
        "contract": GITHUB_CLI_CONTRACT_VERSION,
        "operation": result.operation.id(),
        "outcome": outcome.id(),
        "exit_code": result.exit_code,
        "title": title,
        "draft": draft,
        "artifact_url": result.artifact_url,
        "stderr_tail": result.stderr_tail,
        "note": result.note,
        "cli_source": cli.source,
    });
    if outcome != GhOutcome::Done {
        return Err(pr_error(&result));
    }
    let human = format!(
        "forge project github pull-request — contract {}\n\
         cli_source={}\n\
         outcome=done\n\
         title={}\n\
         draft={}\n",
        GITHUB_CLI_CONTRACT_VERSION, cli.source, title, draft,
    );
    Ok(as_output(format, human, json))
}

fn pr_error(result: &forge::github::GhResult) -> ForgeError {
    use forge::github::{GhOutcome, GhResult};
    let ctx = "github cli pull-request".to_string();
    match result {
        GhResult {
            outcome: GhOutcome::Done,
            ..
        } => ForgeError::GithubCliInvalid {
            reason: "internal: done outcome cannot be turned into an error".to_string(),
        },
        _ => result.to_error(&ctx),
    }
}
