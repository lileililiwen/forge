//! Project commands (`project`).
//!
//! Typed CLI handlers for the normalized project catalog surface.
//! Bodies moved verbatim from the split of `src/main.rs`.

use forge::catalog::{self, CatalogQuery, CatalogRequest};
use forge::core::ForgeError;
use std::path::Path;

use super::catalog::{
    catalog_counts_output, catalog_filter_pairs, catalog_page_output, catalog_records_output,
    catalog_selection,
};
use super::commands_ops::{GithubCommands, ProjectCommands};
use super::commands_services::CatalogFilterArgs;
use super::gaps::cmd_project_gaps;
use super::github::{
    cmd_github_cli_auth, cmd_github_cli_clone, cmd_github_cli_create, cmd_github_cli_pull_request,
};
use super::projects::as_output;
use crate::{Format, Output};

pub(crate) fn cmd_project(
    db_path: &Path,
    command: &ProjectCommands,
    format: Format,
) -> Result<Output, ForgeError> {
    match command {
        ProjectCommands::Gaps {
            project,
            filters,
            categories,
            statuses,
            remediation_classes,
        } => cmd_project_gaps(
            db_path,
            project.as_deref(),
            filters,
            categories,
            statuses,
            remediation_classes,
            format,
        ),
        ProjectCommands::List { filters } => cmd_project_list(db_path, filters, format),
        ProjectCommands::Inspect { project, filters } => {
            cmd_project_inspect(db_path, project, filters, format)
        }
        ProjectCommands::Tags { filters } => cmd_project_tags(db_path, filters, format),
        ProjectCommands::Languages { filters } => cmd_project_languages(db_path, filters, format),
        ProjectCommands::Github { command } => cmd_project_github(db_path, command, format),
    }
}

fn cmd_project_list(
    db_path: &Path,
    args: &CatalogFilterArgs,
    format: Format,
) -> Result<Output, ForgeError> {
    catalog::validate_max_age(args.max_age)?;
    let selection = catalog_selection(args)?;
    let bundle = catalog::collect(&CatalogRequest {
        selection: &selection,
        registry_path: db_path,
        max_age_seconds: args.max_age,
        now: chrono::Utc::now(),
    });
    let pairs = catalog_filter_pairs(args);
    let query = CatalogQuery::from_pairs(&pairs, args.limit, args.cursor.clone())?.normalize();
    let mut page = catalog::apply(&bundle.records, &query, &bundle.observed_at)?;
    page.sources = bundle.statuses.clone();
    catalog_page_output(page, format)
}

fn cmd_project_inspect(
    db_path: &Path,
    project: &str,
    args: &CatalogFilterArgs,
    format: Format,
) -> Result<Output, ForgeError> {
    catalog::validate_max_age(args.max_age)?;
    let selection = catalog_selection(args)?;
    let bundle = catalog::collect(&CatalogRequest {
        selection: &selection,
        registry_path: db_path,
        max_age_seconds: args.max_age,
        now: chrono::Utc::now(),
    });
    let records = catalog::inspect_records(&bundle, project)?;
    catalog_records_output(project, &records, format)
}

fn cmd_project_tags(
    db_path: &Path,
    args: &CatalogFilterArgs,
    format: Format,
) -> Result<Output, ForgeError> {
    catalog::validate_max_age(args.max_age)?;
    let selection = catalog_selection(args)?;
    let bundle = catalog::collect(&CatalogRequest {
        selection: &selection,
        registry_path: db_path,
        max_age_seconds: args.max_age,
        now: chrono::Utc::now(),
    });
    let pairs = catalog_filter_pairs(args);
    let query = CatalogQuery::from_pairs(&pairs, args.limit, args.cursor.clone())?.normalize();
    let filtered = catalog::filter(&bundle.records, &query);
    let counts = catalog::tag_counts(&filtered);
    catalog_counts_output("tags", &counts, format)
}

fn cmd_project_languages(
    db_path: &Path,
    args: &CatalogFilterArgs,
    format: Format,
) -> Result<Output, ForgeError> {
    catalog::validate_max_age(args.max_age)?;
    let selection = catalog_selection(args)?;
    let bundle = catalog::collect(&CatalogRequest {
        selection: &selection,
        registry_path: db_path,
        max_age_seconds: args.max_age,
        now: chrono::Utc::now(),
    });
    let pairs = catalog_filter_pairs(args);
    let query = CatalogQuery::from_pairs(&pairs, args.limit, args.cursor.clone())?.normalize();
    let filtered = catalog::filter(&bundle.records, &query);
    let counts = catalog::language_counts(&filtered);
    catalog_counts_output("languages", &counts, format)
}

fn cmd_project_github(
    db_path: &Path,
    command: &GithubCommands,
    format: Format,
) -> Result<Output, ForgeError> {
    match command {
        GithubCommands::Observe { repositories, host } => {
            cmd_github_observe(db_path, repositories, host, format)
        }
        GithubCommands::Propose {
            repository,
            host,
            mode,
            confirm,
            sets,
        } => cmd_github_propose(repository, host, mode, confirm.as_deref(), sets, format),
        GithubCommands::Auth { host } => cmd_github_cli_auth(host, format),
        GithubCommands::Clone {
            repository,
            destination,
            confirm,
        } => cmd_github_cli_clone(db_path, repository, destination, *confirm, format),
        GithubCommands::Create {
            project,
            repo,
            visibility,
            confirm_public,
            push_source,
            confirm,
        } => cmd_github_cli_create(
            db_path,
            project,
            repo,
            visibility,
            *confirm_public,
            *push_source,
            *confirm,
            format,
        ),
        GithubCommands::PullRequest {
            project,
            title,
            body,
            draft,
            confirm,
        } => cmd_github_cli_pull_request(db_path, project, title, body, *draft, *confirm, format),
    }
}

fn cmd_github_observe(
    db_path: &Path,
    repositories: &[String],
    host: &str,
    format: Format,
) -> Result<Output, ForgeError> {
    use forge::github::{
        normalize_observation, GithubAdapter, GithubObservationRequest, GITHUB_CONTRACT_VERSION,
    };
    let adapter = GithubAdapter::from_env();
    if !adapter.binary_available() {
        return Err(ForgeError::GithubAdapterUnavailable {
            reason: format!(
                "no GitHub adapter binary is configured ({})",
                adapter.source
            ),
        });
    }
    if adapter.token.is_none() {
        return Err(ForgeError::GithubInvalid {
            reason: format!(
                "{} is not set; refusing to send an unauthenticated observation request",
                forge::github::GITHUB_TOKEN_ENV
            ),
        });
    }
    let explicit = repositories
        .iter()
        .filter(|value| !value.trim().is_empty())
        .cloned()
        .collect::<Vec<_>>();
    let selection_repos = if explicit.is_empty() {
        github_repositories_from_registry(db_path)?
    } else {
        explicit
    };
    if selection_repos.is_empty() {
        return Err(ForgeError::GithubInvalid {
            reason: "no repositories to observe; pass `owner/repo` arguments or register a \
                     project whose git_remote points to github.com"
                .to_string(),
        });
    }
    let request = GithubObservationRequest {
        host: host.to_string(),
        repositories: selection_repos,
    };
    let observations = adapter.observe(&request)?;
    let now = chrono::Utc::now();
    let max_age = forge::catalog::DEFAULT_MAX_AGE_SECONDS;
    let records: Vec<forge::catalog::CatalogRecord> = observations
        .iter()
        .map(|observation| normalize_observation(observation, max_age, now))
        .collect();
    let human = render_github_observe_human(host, &observations, &records);
    let json = serde_json::json!({
        "contract": GITHUB_CONTRACT_VERSION,
        "host": host,
        "adapter_source": adapter.source,
        "observations": observations,
        "records": records,
    });
    Ok(as_output(format, human, json))
}

fn cmd_github_propose(
    repository: &str,
    host: &str,
    mode: &str,
    confirm: Option<&str>,
    sets: &[String],
    format: Format,
) -> Result<Output, ForgeError> {
    use forge::github::{
        GithubAdapter, MutationMode, ProposeRequest, ProposedChange, GITHUB_CONTRACT_VERSION,
    };
    let adapter = GithubAdapter::from_env();
    if !adapter.binary_available() {
        return Err(ForgeError::GithubAdapterUnavailable {
            reason: format!(
                "no GitHub adapter binary is configured ({})",
                adapter.source
            ),
        });
    }
    if adapter.token.is_none() {
        return Err(ForgeError::GithubInvalid {
            reason: format!(
                "{} is not set; refusing to mutate without authentication",
                forge::github::GITHUB_TOKEN_ENV
            ),
        });
    }
    let mode_trimmed = mode.trim();
    let mutation_mode = match mode_trimmed {
        "pull-request" | "pr" | "" => MutationMode::PullRequest,
        "direct" => match confirm {
            Some(token) if !token.trim().is_empty() => MutationMode::Direct {
                confirmation: token.trim().to_string(),
            },
            _ => {
                return Err(ForgeError::GithubInvalid {
                    reason: "direct mode requires a non-empty --confirm token; refusing the \
                             mutation"
                        .to_string(),
                });
            }
        },
        other => {
            return Err(ForgeError::GithubInvalid {
                reason: format!(
                    "unknown --mode `{other}`; expected `pull-request` (default) or `direct`"
                ),
            });
        }
    };
    let mut changes: Vec<ProposedChange> = Vec::new();
    for raw in sets {
        let (field, value) = raw
            .split_once('=')
            .ok_or_else(|| ForgeError::GithubInvalid {
                reason: format!("proposed change `{raw}` is not in `field=value` form"),
            })?;
        changes.push(ProposedChange {
            field: field.trim().to_string(),
            new_value: value.trim().to_string(),
        });
    }
    let request = ProposeRequest {
        host: host.to_string(),
        repository: repository.to_string(),
        mode: mutation_mode,
        changes,
    };
    let outcome = adapter.propose(&request)?;
    let human = render_github_propose_human(&outcome);
    let json = serde_json::json!({
        "contract": GITHUB_CONTRACT_VERSION,
        "host": host,
        "repository": repository,
        "adapter_source": adapter.source,
        "outcome": outcome,
    });
    Ok(as_output(format, human, json))
}

fn github_repositories_from_registry(db_path: &Path) -> Result<Vec<String>, ForgeError> {
    if !db_path.is_file() {
        return Ok(Vec::new());
    }
    let registry = forge::registry::Registry::open_read_only(db_path)?;
    let projects = registry.list()?;
    let mut out: Vec<String> = Vec::new();
    for project in projects {
        let Some(remote) = project.git_remote.as_deref() else {
            continue;
        };
        if let Some(repository) = forge::catalog::source::github_repository_from_remote(remote) {
            if !out.contains(&repository) {
                out.push(repository);
            }
        }
    }
    Ok(out)
}

fn render_github_observe_human(
    host: &str,
    observations: &[forge::github::GithubObservation],
    records: &[forge::catalog::CatalogRecord],
) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "forge project github observe — contract {} host={}\n",
        forge::github::GITHUB_CONTRACT_VERSION,
        host
    ));
    out.push_str(&format!("observations: {}\n", observations.len()));
    for observation in observations {
        out.push_str(&format!(
            "  repository={} state={} observed_at={} revision={}\n",
            observation.repository,
            observation.state.id(),
            observation.observed_at,
            observation.source_revision.as_deref().unwrap_or("unknown"),
        ));
        if let Some(description) = observation.description.as_deref() {
            out.push_str(&format!("    description: {description}\n"));
        }
        if !observation.topics.is_empty() {
            out.push_str(&format!("    topics: {}\n", observation.topics.join(",")));
        }
        if !observation.languages.is_empty() {
            out.push_str(&format!(
                "    languages: {}\n",
                observation.languages.join(",")
            ));
        }
        if let Some(branch) = observation.default_branch.as_deref() {
            out.push_str(&format!("    default_branch: {branch}\n"));
        }
        if observation.archived {
            out.push_str("    archived: true\n");
        }
        if !observation.workflows.is_empty() {
            out.push_str(&format!(
                "    workflows: {}\n",
                observation.workflows.join(",")
            ));
        }
        if !observation.releases.is_empty() {
            out.push_str(&format!(
                "    releases: {}\n",
                observation.releases.join(",")
            ));
        }
        if !observation.note.is_empty() {
            out.push_str(&format!("    note: {}\n", observation.note));
        }
    }
    out.push_str(&format!("records: {}\n", records.len()));
    for record in records {
        out.push_str(&format!(
            "  project_id={} source={} evidence={} freshness={}\n",
            record.project_id,
            record.source,
            record.evidence.id(),
            record.freshness.id(),
        ));
    }
    out.trim_end().to_string()
}

fn render_github_propose_human(outcome: &forge::github::ProposeOutcome) -> String {
    let mut text = String::new();
    text.push_str(&format!(
        "forge project github propose — contract {}\n",
        forge::github::GITHUB_CONTRACT_VERSION
    ));
    text.push_str(&format!(
        "mode={} state={}",
        outcome.mode,
        outcome.state.id()
    ));
    if let Some(artifact) = outcome.artifact_id.as_deref() {
        text.push_str(&format!(" artifact={artifact}"));
    }
    text.push('\n');
    if !outcome.note.is_empty() {
        text.push_str(&format!("note: {}\n", outcome.note));
    }
    text.trim_end().to_string()
}
