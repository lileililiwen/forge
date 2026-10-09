//! Catalog query helpers (`catalog`).
//!
//! Source selection and output rendering for the project catalog surface.
//! Bodies moved verbatim from the split of `src/main.rs`.

use forge::catalog::{self, CatalogSourceSelection, CATALOG_CONTRACT_VERSION};
use forge::core::ForgeError;
use forge::fleet::{self};

use super::commands_services::CatalogFilterArgs;
use crate::{Format, Output};

/// Build the source selection from the flags, defaulting to the local
/// registry. Only declared sources are read.
pub(super) fn catalog_selection(
    args: &CatalogFilterArgs,
) -> Result<CatalogSourceSelection, ForgeError> {
    let mut kinds: Vec<forge::catalog::SourceKind> = Vec::new();
    if args.sources.is_empty() {
        kinds.push(forge::catalog::SourceKind::Local);
    } else {
        for raw in &args.sources {
            let kind = forge::catalog::SourceKind::parse(raw).ok_or_else(|| {
                catalog::catalog_invalid(format!(
                    "unknown --source `{raw}`; expected one of \
                     local|git|workspace-registry|inventory|github"
                ))
            })?;
            if !kinds.contains(&kind) {
                kinds.push(kind);
            }
        }
    }
    let workspace_registry = args
        .workspace_registry
        .clone()
        .or_else(|| fleet::resolve_registry_path(None));
    let inventory = args
        .inventory
        .clone()
        .or_else(|| forge::publish::inventory::resolve_source(None));
    Ok(CatalogSourceSelection {
        kinds,
        git_repositories: args.git_repositories.clone(),
        workspace_registry,
        inventory,
        github_repositories: args.github_repositories.clone(),
    })
}

/// Flatten the typed filters plus `key=value` filters into one ordered
/// pair list. `CatalogQuery::from_pairs` refuses an unknown key.
pub(super) fn catalog_filter_pairs(args: &CatalogFilterArgs) -> Vec<String> {
    let mut pairs = Vec::new();
    for value in &args.tags {
        pairs.push(format!("tag={value}"));
    }
    for value in &args.languages {
        pairs.push(format!("language={value}"));
    }
    for value in &args.profiles {
        pairs.push(format!("profile={value}"));
    }
    for value in &args.lifecycles {
        pairs.push(format!("lifecycle={value}"));
    }
    for value in &args.repositories {
        pairs.push(format!("repository={value}"));
    }
    for value in &args.ci {
        pairs.push(format!("ci={value}"));
    }
    for value in &args.compose {
        pairs.push(format!("compose={value}"));
    }
    for value in &args.evidence {
        pairs.push(format!("evidence={value}"));
    }
    pairs.extend(args.filters.iter().cloned());
    pairs
}

pub(super) fn catalog_page_output(
    page: forge::catalog::CatalogPage,
    format: Format,
) -> Result<Output, ForgeError> {
    match format {
        Format::Human | Format::Table => Ok(Output::Human(catalog::render_page_human(&page))),
        Format::Json => Ok(Output::Json(serde_json::json!({"catalog": page}))),
        Format::Ndjson => Ok(Output::Raw(catalog_ndjson(&page.records)?)),
    }
}

pub(super) fn catalog_records_output(
    project: &str,
    records: &[forge::catalog::CatalogRecord],
    format: Format,
) -> Result<Output, ForgeError> {
    match format {
        Format::Human | Format::Table => Ok(Output::Human(catalog::render_records_human(
            "inspect", records,
        ))),
        Format::Json => Ok(Output::Json(serde_json::json!({
            "catalog": {
                "contract": CATALOG_CONTRACT_VERSION,
                "project_id": project,
                "records": records,
            }
        }))),
        Format::Ndjson => Ok(Output::Raw(catalog_ndjson(records)?)),
    }
}

pub(super) fn catalog_counts_output(
    title: &str,
    counts: &[forge::catalog::CatalogValueCount],
    format: Format,
) -> Result<Output, ForgeError> {
    match format {
        Format::Human | Format::Table => {
            Ok(Output::Human(catalog::render_counts_human(title, counts)))
        }
        Format::Json => {
            let mut catalog = serde_json::Map::new();
            catalog.insert(
                "contract".to_string(),
                serde_json::json!(CATALOG_CONTRACT_VERSION),
            );
            catalog.insert(
                title.to_string(),
                serde_json::to_value(counts).map_err(|err| {
                    catalog::catalog_invalid(format!("cannot serialize {title}: {err}"))
                })?,
            );
            Ok(Output::Json(serde_json::json!({ "catalog": catalog })))
        }
        Format::Ndjson => {
            let mut out = String::new();
            for entry in counts {
                out.push_str(&serde_json::to_string(entry).map_err(|err| {
                    catalog::catalog_invalid(format!("cannot serialize {title}: {err}"))
                })?);
                out.push('\n');
            }
            Ok(Output::Raw(out))
        }
    }
}

fn catalog_ndjson(records: &[forge::catalog::CatalogRecord]) -> Result<String, ForgeError> {
    let mut out = String::new();
    for record in records {
        out.push_str(&serde_json::to_string(record).map_err(|err| {
            catalog::catalog_invalid(format!("cannot serialize catalog record: {err}"))
        })?);
        out.push('\n');
    }
    Ok(out)
}
