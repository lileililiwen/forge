//! Project gap commands (`gaps`).
//!
//! Handlers for the doctor gap query surface.
//! Bodies moved verbatim from the split of `src/main.rs`.

use forge::catalog::{self, CatalogRequest};
use forge::core::ForgeError;
use std::path::Path;

use super::catalog::catalog_selection;
use super::commands_services::CatalogFilterArgs;
use crate::{Format, Output};

pub(super) fn cmd_project_gaps(
    db_path: &Path,
    project: Option<&str>,
    args: &CatalogFilterArgs,
    categories: &[String],
    statuses: &[String],
    remediation_classes: &[String],
    format: Format,
) -> Result<Output, ForgeError> {
    use forge::doctor::gaps::{self, GapCategory, GapFilters, GapStatus, RemediationClass};

    catalog::validate_max_age(args.max_age)?;
    let selection = catalog_selection(args)?;
    let bundle = catalog::collect(&CatalogRequest {
        selection: &selection,
        registry_path: db_path,
        max_age_seconds: args.max_age,
        now: chrono::Utc::now(),
    });

    // When a project id is supplied, scope the records to that id; an
    // unknown project is an `unknown-project` refusal, matching
    // `forge project inspect`.
    let mut records: Vec<forge::catalog::CatalogRecord> = if let Some(id) = project {
        catalog::inspect_records(&bundle, id)?
    } else {
        bundle.records.clone()
    };
    // Stable order: by (project_id, source), matching `forge project list`.
    records.sort_by(|a, b| {
        a.project_id
            .cmp(&b.project_id)
            .then_with(|| a.source.cmp(&b.source))
    });

    let parsed_categories: Vec<GapCategory> = categories
        .iter()
        .map(|raw| {
            GapCategory::parse(raw).ok_or_else(|| {
                catalog::catalog_invalid(format!(
                    "unknown --category `{raw}`; expected one of \
                     description|tags|ci|compose|manifest|docs|repository"
                ))
            })
        })
        .collect::<Result<_, _>>()?;
    let parsed_statuses: Vec<GapStatus> = statuses
        .iter()
        .map(|raw| {
            GapStatus::parse(raw).ok_or_else(|| {
                catalog::catalog_invalid(format!(
                    "unknown --status `{raw}`; expected one of \
                     pass|warn|fail|unavailable|not_applicable"
                ))
            })
        })
        .collect::<Result<_, _>>()?;
    let parsed_classes: Vec<RemediationClass> = remediation_classes
        .iter()
        .map(|raw| {
            RemediationClass::parse(raw).ok_or_else(|| {
                catalog::catalog_invalid(format!(
                    "unknown --remediation-class `{raw}`; expected one of \
                     automatic|semantic|manual"
                ))
            })
        })
        .collect::<Result<_, _>>()?;
    let filters = GapFilters {
        categories: parsed_categories,
        statuses: parsed_statuses,
        remediation_classes: parsed_classes,
    };
    let report = gaps::build_report(&records, &bundle.statuses, project);
    gaps_output(&report, &filters, &bundle.statuses, format)
}

fn gaps_output(
    report: &forge::doctor::gaps::GapReport,
    filters: &forge::doctor::gaps::GapFilters,
    sources: &[forge::catalog::SourceStatus],
    format: Format,
) -> Result<Output, ForgeError> {
    let filtered: Vec<&forge::doctor::gaps::GapFinding> = report
        .findings
        .iter()
        .filter(|f| filters.matches(f))
        .collect();
    let total = report.findings.len();
    match format {
        Format::Human | Format::Table => Ok(Output::Human(
            forge::doctor::gaps::render_report_human(report, &filtered, sources, total, filters),
        )),
        Format::Json => Ok(Output::Json(serde_json::json!({
            "gaps": {
                "contract": report.contract,
                "project_id": report.project_id,
                "sources": sources,
                "summary": {
                    "total": total,
                    "returned": filtered.len(),
                    "filters": filters_summary(filters),
                },
                "findings": filtered,
            }
        }))),
        Format::Ndjson => Ok(Output::Raw(gaps_ndjson(&filtered)?)),
    }
}

fn filters_summary(filters: &forge::doctor::gaps::GapFilters) -> serde_json::Value {
    serde_json::json!({
        "categories": filters.categories.iter().map(|c| c.id()).collect::<Vec<_>>(),
        "statuses": filters.statuses.iter().map(|s| s.id()).collect::<Vec<_>>(),
        "remediation_classes": filters.remediation_classes.iter().map(|r| r.id()).collect::<Vec<_>>(),
    })
}

fn gaps_ndjson(findings: &[&forge::doctor::gaps::GapFinding]) -> Result<String, ForgeError> {
    let mut out = String::new();
    for finding in findings {
        let value = serde_json::to_value(finding).map_err(|err| {
            catalog::catalog_invalid(format!("gap finding failed to serialize: {err}"))
        })?;
        out.push_str(&serde_json::to_string(&value).map_err(|err| {
            catalog::catalog_invalid(format!("gap finding failed to serialize: {err}"))
        })?);
        out.push('\n');
    }
    Ok(out)
}
