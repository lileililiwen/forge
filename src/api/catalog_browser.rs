//! Read-only project-catalog browser (`web-project-catalog-browser`).
//!
//! Six session-gated admin GETs over the existing Core services only:
//! the normalized catalog (`forge-project-catalog/0.1.0`), the
//! evidence-backed gaps (`forge-project-evidence/0.1.0`) and one fleet
//! registry entry. No write, no provider, no adapter, no shell, no
//! journal row. Transports stay thin: filtering, ordering, pagination
//! and finding synthesis live in Core (`src/catalog/`,
//! `src/doctor/gaps/`, `src/fleet/`).

use std::collections::BTreeSet;
use std::path::Path;

use chrono::{DateTime, Utc};
use serde_json::json;

use super::model::ApiResponse;
use super::server::{
    build_catalog_selection, catalog_filter_pairs_from, parse_catalog_query_params, percent_decode,
};
use super::API_CONTRACT_VERSION;
use crate::{catalog, doctor::gaps, fleet};

/// `GET /v1/admin/projects/catalog`: paginated list, same bytes as
/// `GET /v1/projects/catalog`.
pub fn list(db_path: &Path, query: Option<&str>, now: DateTime<Utc>) -> ApiResponse {
    let params = match parse_catalog_query_params(query) {
        Ok(params) => params,
        Err(response) => return response,
    };
    let selection = match build_catalog_selection(&params) {
        Ok(selection) => selection,
        Err(response) => return response,
    };
    if let Err(err) = catalog::validate_max_age(params.max_age) {
        return ApiResponse::from_error(&err);
    }
    let pairs = catalog_filter_pairs_from(&params);
    let query = match catalog::CatalogQuery::from_pairs(&pairs, params.limit, params.cursor.clone())
    {
        Ok(query) => query.normalize(),
        Err(err) => return ApiResponse::from_error(&err),
    };
    let bundle = catalog::collect(&catalog::CatalogRequest {
        selection: &selection,
        registry_path: db_path,
        max_age_seconds: params.max_age,
        now,
    });
    let mut page = match catalog::apply(&bundle.records, &query, &bundle.observed_at) {
        Ok(page) => page,
        Err(err) => return ApiResponse::from_error(&err),
    };
    page.sources = bundle.statuses.clone();
    ApiResponse::json(
        200,
        json!({
            "catalog": page,
            "contract": API_CONTRACT_VERSION,
        }),
    )
}

/// Shared filtered-record set for the tags/languages projections.
fn filtered_records(
    db_path: &Path,
    query: Option<&str>,
    now: DateTime<Utc>,
) -> Result<Vec<catalog::CatalogRecord>, ApiResponse> {
    let params = match parse_catalog_query_params(query) {
        Ok(params) => params,
        Err(response) => return Err(response),
    };
    let selection = match build_catalog_selection(&params) {
        Ok(selection) => selection,
        Err(response) => return Err(response),
    };
    if let Err(err) = catalog::validate_max_age(params.max_age) {
        return Err(ApiResponse::from_error(&err));
    }
    let pairs = catalog_filter_pairs_from(&params);
    let query = match catalog::CatalogQuery::from_pairs(&pairs, params.limit, params.cursor.clone())
    {
        Ok(query) => query.normalize(),
        Err(err) => return Err(ApiResponse::from_error(&err)),
    };
    let bundle = catalog::collect(&catalog::CatalogRequest {
        selection: &selection,
        registry_path: db_path,
        max_age_seconds: params.max_age,
        now,
    });
    Ok(catalog::filter(&bundle.records, &query))
}

/// `GET /v1/admin/projects/catalog/tags`.
pub fn tags(db_path: &Path, query: Option<&str>, now: DateTime<Utc>) -> ApiResponse {
    match filtered_records(db_path, query, now) {
        Ok(filtered) => {
            let counts = catalog::tag_counts(&filtered);
            ApiResponse::json(
                200,
                json!({
                    "catalog": {
                        "contract": catalog::CATALOG_CONTRACT_VERSION,
                        "tags": counts,
                    },
                    "contract": API_CONTRACT_VERSION,
                }),
            )
        }
        Err(response) => response,
    }
}

/// `GET /v1/admin/projects/catalog/languages`.
pub fn languages(db_path: &Path, query: Option<&str>, now: DateTime<Utc>) -> ApiResponse {
    match filtered_records(db_path, query, now) {
        Ok(filtered) => {
            let counts = catalog::language_counts(&filtered);
            ApiResponse::json(
                200,
                json!({
                    "catalog": {
                        "contract": catalog::CATALOG_CONTRACT_VERSION,
                        "languages": counts,
                    },
                    "contract": API_CONTRACT_VERSION,
                }),
            )
        }
        Err(response) => response,
    }
}

/// `GET /v1/admin/projects/{id}/catalog`: every record for one id.
pub fn inspect(db_path: &Path, query: Option<&str>, id: &str, now: DateTime<Utc>) -> ApiResponse {
    if crate::core::validate_project_id(id).is_err() {
        return ApiResponse::json(
            400,
            json!({
                "error": {
                    "code": "admin-invalid-project-id",
                    "message": "that project name is not valid; use lowercase letters, numbers and dashes.",
                },
                "contract": API_CONTRACT_VERSION,
            }),
        );
    }
    let params = match parse_catalog_query_params(query) {
        Ok(params) => params,
        Err(response) => return response,
    };
    let selection = match build_catalog_selection(&params) {
        Ok(selection) => selection,
        Err(response) => return response,
    };
    if let Err(err) = catalog::validate_max_age(params.max_age) {
        return ApiResponse::from_error(&err);
    }
    let bundle = catalog::collect(&catalog::CatalogRequest {
        selection: &selection,
        registry_path: db_path,
        max_age_seconds: params.max_age,
        now,
    });
    match catalog::inspect_records(&bundle, id) {
        Ok(records) => ApiResponse::json(
            200,
            json!({
                "catalog": {
                    "contract": catalog::CATALOG_CONTRACT_VERSION,
                    "project_id": id,
                    "records": records,
                },
                "contract": API_CONTRACT_VERSION,
            }),
        ),
        Err(err) => ApiResponse::from_error(&err),
    }
}

fn query_values(query: Option<&str>, key: &str) -> Vec<String> {
    let mut out = Vec::new();
    for pair in query
        .unwrap_or_default()
        .split('&')
        .filter(|p| !p.is_empty())
    {
        let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
        if k.trim() == key {
            let decoded = percent_decode(v);
            if !decoded.trim().is_empty() {
                out.push(decoded);
            }
        }
    }
    out
}

/// Strip gap-only keys before the catalog parser sees them (it refuses
/// unknown keys with `api-invalid`).
fn strip_gap_keys(query: Option<&str>) -> Option<String> {
    let raw = query.unwrap_or_default();
    if raw.is_empty() {
        return None;
    }
    let kept: Vec<&str> = raw
        .split('&')
        .filter(|pair| {
            let (k, _) = pair.split_once('=').unwrap_or((pair, ""));
            !matches!(
                k.trim(),
                "project" | "category" | "status" | "remediation-class" | "remediation_class"
            )
        })
        .collect();
    if kept.is_empty() {
        None
    } else {
        Some(kept.join("&"))
    }
}

fn gap_filters(query: Option<&str>) -> Result<gaps::GapFilters, ApiResponse> {
    let mut categories = Vec::new();
    for raw in query_values(query, "category") {
        match gaps::GapCategory::parse(&raw) {
            Some(parsed) => categories.push(parsed),
            None => {
                return Err(ApiResponse::from_error(&catalog::catalog_invalid(format!(
                    "unknown --category `{raw}`; expected one of \
                         description|tags|ci|compose|manifest|docs|repository"
                ))));
            }
        }
    }
    let mut statuses = Vec::new();
    for raw in query_values(query, "status") {
        match gaps::GapStatus::parse(&raw) {
            Some(parsed) => statuses.push(parsed),
            None => {
                return Err(ApiResponse::from_error(&catalog::catalog_invalid(format!(
                    "unknown --status `{raw}`; expected one of \
                         pass|warn|fail|unavailable|not_applicable"
                ))));
            }
        }
    }
    let mut classes = Vec::new();
    for raw in query_values(query, "remediation-class")
        .into_iter()
        .chain(query_values(query, "remediation_class"))
    {
        match gaps::RemediationClass::parse(&raw) {
            Some(parsed) => classes.push(parsed),
            None => {
                return Err(ApiResponse::from_error(&catalog::catalog_invalid(format!(
                    "unknown --remediation-class `{raw}`; expected one of \
                         automatic|semantic|manual"
                ))));
            }
        }
    }
    Ok(gaps::GapFilters {
        categories,
        statuses,
        remediation_classes: classes,
    })
}

/// `GET /v1/admin/projects/catalog/gaps`: evidence-backed findings with
/// `?project=` plus repeatable `?category=`/`?status=`/
/// `?remediation-class=`, over the same catalog selection as `list`.
pub fn gaps(db_path: &Path, query: Option<&str>, now: DateTime<Utc>) -> ApiResponse {
    let project = query_values(query, "project").into_iter().next();
    if let Some(id) = project.as_deref() {
        if crate::core::validate_project_id(id).is_err() {
            return ApiResponse::json(
                400,
                json!({
                    "error": {
                        "code": "admin-invalid-project-id",
                        "message": "that project name is not valid; use lowercase letters, numbers and dashes.",
                    },
                    "contract": API_CONTRACT_VERSION,
                }),
            );
        }
    }
    let stripped = strip_gap_keys(query);
    let params = match parse_catalog_query_params(stripped.as_deref()) {
        Ok(params) => params,
        Err(response) => return response,
    };
    let selection = match build_catalog_selection(&params) {
        Ok(selection) => selection,
        Err(response) => return response,
    };
    if let Err(err) = catalog::validate_max_age(params.max_age) {
        return ApiResponse::from_error(&err);
    }
    let bundle = catalog::collect(&catalog::CatalogRequest {
        selection: &selection,
        registry_path: db_path,
        max_age_seconds: params.max_age,
        now,
    });
    let mut records: Vec<catalog::CatalogRecord> = if let Some(id) = project.as_deref() {
        match catalog::inspect_records(&bundle, id) {
            Ok(records) => records,
            Err(err) => return ApiResponse::from_error(&err),
        }
    } else {
        bundle.records.clone()
    };
    records.sort_by(|a, b| {
        a.project_id
            .cmp(&b.project_id)
            .then_with(|| a.source.cmp(&b.source))
    });
    let filters = match gap_filters(query) {
        Ok(filters) => filters,
        Err(response) => return response,
    };
    let report = gaps::build_report(&records, &bundle.statuses, project.as_deref());
    let filtered: Vec<&gaps::GapFinding> = report
        .findings
        .iter()
        .filter(|f| filters.matches(f))
        .collect();
    let total = report.findings.len();
    ApiResponse::json(
        200,
        json!({
            "gaps": {
                "contract": report.contract,
                "project_id": report.project_id,
                "sources": bundle.statuses,
                "summary": {
                    "total": total,
                    "returned": filtered.len(),
                    "filters": {
                        "categories": filters.categories.iter().map(|c| c.id()).collect::<Vec<_>>(),
                        "statuses": filters.statuses.iter().map(|s| s.id()).collect::<Vec<_>>(),
                        "remediation_classes": filters.remediation_classes.iter().map(|r| r.id()).collect::<Vec<_>>(),
                    },
                },
                "findings": filtered,
            },
            "contract": API_CONTRACT_VERSION,
        }),
    )
}

/// `GET /v1/admin/fleet/{entry}`: one fleet registry entry, read-only.
/// The workspace registry resolves server-side; the browser sends only
/// the entry id plus an optional `max-age`.
pub fn fleet_inspect(db_path: &Path, query: Option<&str>, entry: &str) -> ApiResponse {
    if crate::core::validate_project_id(entry).is_err() {
        return ApiResponse::json(
            400,
            json!({
                "error": {
                    "code": "admin-invalid-project-id",
                    "message": "that fleet entry name is not valid; use lowercase letters, numbers and dashes.",
                },
                "contract": API_CONTRACT_VERSION,
            }),
        );
    }
    let max_age = query_values(query, "max-age")
        .into_iter()
        .chain(query_values(query, "max_age"))
        .next()
        .and_then(|raw| raw.trim().parse::<i64>().ok())
        .unwrap_or(fleet::DEFAULT_MAX_AGE_SECONDS);
    if let Err(err) = fleet::validate_max_age(max_age) {
        return ApiResponse::from_error(&err);
    }
    let registry = match crate::registry::Registry::open_read_only(db_path) {
        Ok(registry) => registry,
        Err(err) => return ApiResponse::from_error(&err),
    };
    let records = match registry.list() {
        Ok(records) => records,
        Err(err) => return ApiResponse::from_error(&err),
    };
    let local_ids: BTreeSet<String> = records.iter().map(|r| r.id.clone()).collect();
    let path = fleet::resolve_registry_path(None);
    let report = match fleet::observe(path.as_deref(), max_age, &local_ids) {
        Ok(report) => report,
        Err(err) => return ApiResponse::from_error(&err),
    };
    match fleet::inspect_entry(&report, entry) {
        Ok(found) => ApiResponse::json(
            200,
            json!({
                "contract": fleet::FLEET_CONTRACT_VERSION,
                "entry": found,
                "source": report.source,
                "freshness": report.freshness,
                "observed_at": report.observed_at,
            }),
        ),
        Err(err) => ApiResponse::from_error(&err),
    }
}
