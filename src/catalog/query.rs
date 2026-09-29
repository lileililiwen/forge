//! Composable, read-only catalog query (`project-catalog-query-contract`).
//!
//! [`apply`] is a pure function over an already-collected record set:
//! every predicate combines as AND, a repeated filter value is OR within
//! that predicate, ordering is stable by `(project_id, source)` and never
//! by observation time, and pagination is deterministic regardless of the
//! order sources arrived in.

use serde::{Deserialize, Serialize};

use crate::core::ForgeError;

use super::record::{catalog_invalid, CatalogRecord, CATALOG_CONTRACT_VERSION};
use super::source::SourceStatus;

/// Default page size when `--limit` is absent.
pub const DEFAULT_LIMIT: usize = 50;

/// Largest accepted page size.
pub const MAX_LIMIT: usize = 1_000;

/// Version tag of the opaque pagination cursor.
const CURSOR_PREFIX: &str = "v1";

/// The composable filter/order/pagination model. Every vector is a
/// disjunction (OR) within its predicate; predicates combine as AND.
#[derive(Debug, Clone, Default)]
pub struct CatalogQuery {
    pub tags: Vec<String>,
    pub languages: Vec<String>,
    pub profiles: Vec<String>,
    pub lifecycles: Vec<String>,
    pub repositories: Vec<String>,
    pub ci: Vec<String>,
    pub compose: Vec<String>,
    pub evidence: Vec<String>,
    pub limit: usize,
    pub cursor: Option<String>,
}

impl CatalogQuery {
    /// Build a query from raw `key=value` filter pairs, refusing an
    /// unknown key before any source is contacted.
    pub fn from_pairs(
        pairs: &[String],
        limit: usize,
        cursor: Option<String>,
    ) -> Result<Self, ForgeError> {
        let mut query = CatalogQuery {
            limit,
            cursor,
            ..CatalogQuery::default()
        };
        for pair in pairs {
            let (key, value) = pair.split_once('=').ok_or_else(|| {
                catalog_invalid(format!("filter `{pair}` must be written as key=value"))
            })?;
            let value = value.trim().to_ascii_lowercase();
            if value.is_empty() {
                return Err(catalog_invalid(format!(
                    "filter `{key}` has an empty value"
                )));
            }
            match key.trim().to_ascii_lowercase().as_str() {
                "tag" | "tags" => query.tags.push(value),
                "language" | "languages" => query.languages.push(value),
                "profile" | "profiles" => query.profiles.push(value),
                "lifecycle" => query.lifecycles.push(value),
                "repository" | "repo" => query.repositories.push(value),
                "ci" => query.ci.push(value),
                "compose" => query.compose.push(value),
                "evidence" => query.evidence.push(value),
                unknown => {
                    return Err(catalog_invalid(format!(
                        "unknown catalog filter key `{unknown}`; expected one of \
                         tag|language|profile|lifecycle|repository|ci|compose|evidence"
                    )))
                }
            }
        }
        Ok(query)
    }

    /// Normalize case on the typed predicate vectors.
    pub fn normalize(mut self) -> Self {
        for list in [
            &mut self.tags,
            &mut self.languages,
            &mut self.profiles,
            &mut self.lifecycles,
            &mut self.repositories,
            &mut self.ci,
            &mut self.compose,
            &mut self.evidence,
        ] {
            for value in list.iter_mut() {
                *value = value.trim().to_ascii_lowercase();
            }
        }
        self
    }

    fn matches(&self, record: &CatalogRecord) -> bool {
        predicate_any(&record.predicate_values("tag"), &self.tags)
            && predicate_any(&record.predicate_values("language"), &self.languages)
            && predicate_any(&record.predicate_values("profile"), &self.profiles)
            && predicate_any(&record.predicate_values("lifecycle"), &self.lifecycles)
            && predicate_substring(&record.predicate_values("repository"), &self.repositories)
            && predicate_any(&record.predicate_values("ci"), &self.ci)
            && predicate_any(&record.predicate_values("compose"), &self.compose)
            && predicate_any(&record.predicate_values("evidence"), &self.evidence)
    }
}

fn predicate_any(values: &[String], filters: &[String]) -> bool {
    filters.is_empty()
        || filters
            .iter()
            .any(|filter| values.iter().any(|v| v == filter))
}

fn predicate_substring(values: &[String], filters: &[String]) -> bool {
    filters.is_empty()
        || filters
            .iter()
            .any(|filter| values.iter().any(|v| v.contains(filter.as_str())))
}

/// The paginated, provenance-carrying answer to a catalog query.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogPage {
    pub contract: String,
    pub observed_at: String,
    pub records: Vec<CatalogRecord>,
    pub total: usize,
    pub limit: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
    pub sources: Vec<SourceStatus>,
}

impl CatalogPage {
    /// Page metadata only, with no source statuses filled in.
    fn new(observed_at: &str, limit: usize) -> Self {
        CatalogPage {
            contract: CATALOG_CONTRACT_VERSION.to_string(),
            observed_at: observed_at.to_string(),
            records: Vec::new(),
            total: 0,
            limit,
            next_cursor: None,
            sources: Vec::new(),
        }
    }
}

/// Filter, order and paginate an already-collected record set. Pure:
/// the same records and query always produce the same page.
pub fn apply(
    records: &[CatalogRecord],
    query: &CatalogQuery,
    observed_at: &str,
) -> Result<CatalogPage, ForgeError> {
    if !(1..=MAX_LIMIT).contains(&query.limit) {
        return Err(catalog_invalid(format!(
            "--limit {} is outside the bounded range 1..={MAX_LIMIT}",
            query.limit
        )));
    }
    let offset = match query.cursor.as_deref() {
        None => 0,
        Some(cursor) => parse_cursor(cursor)?,
    };
    let filtered = filter(records, query);
    let total = filtered.len();
    let start = offset.min(total);
    let end = (start + query.limit).min(total);
    let page_records: Vec<CatalogRecord> = filtered[start..end].to_vec();
    let mut page = CatalogPage::new(observed_at, query.limit);
    page.records = page_records;
    page.total = total;
    page.next_cursor = if end < total {
        Some(format!("{CURSOR_PREFIX}:{end}"))
    } else {
        None
    };
    Ok(page)
}

/// Filter and order a record set without paginating it. Used by the
/// `tags`/`languages` projections, which summarize the whole filtered
/// catalog rather than one page.
pub fn filter(records: &[CatalogRecord], query: &CatalogQuery) -> Vec<CatalogRecord> {
    let mut filtered: Vec<CatalogRecord> = records
        .iter()
        .filter(|record| query.matches(record))
        .cloned()
        .collect();
    // Stable order by (project_id, source): never by observation time, and
    // never by source arrival order.
    filtered.sort_by(|a, b| {
        a.project_id
            .cmp(&b.project_id)
            .then_with(|| a.source.cmp(&b.source))
    });
    filtered
}

/// Decode an opaque cursor, refusing a tampered or malformed one.
fn parse_cursor(cursor: &str) -> Result<usize, ForgeError> {
    let rest = cursor
        .strip_prefix(&format!("{CURSOR_PREFIX}:"))
        .ok_or_else(|| catalog_invalid(format!("cursor `{cursor}` is not a catalog cursor")))?;
    rest.parse::<usize>()
        .map_err(|_| catalog_invalid(format!("cursor `{cursor}` is malformed")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::record::{EvidenceState, Freshness, SourceKind};

    fn record(id: &str, source: &str, kind: SourceKind, tags: &[&str]) -> CatalogRecord {
        CatalogRecord {
            project_id: id.to_string(),
            name: None,
            source: source.to_string(),
            source_kind: kind,
            source_revision: None,
            observed_at: "2026-09-29T00:00:00Z".to_string(),
            freshness: Freshness::Current,
            profile: Some("rust-web".to_string()),
            lifecycle: Some("operational".to_string()),
            repository: Some(format!("https://example.invalid/{id}.git")),
            tags: tags.iter().map(|t| t.to_string()).collect(),
            languages: vec!["rust".to_string()],
            ci: None,
            compose: None,
            evidence: EvidenceState::Present,
        }
    }

    #[test]
    fn orders_by_project_then_source_independent_of_arrival() {
        let a = vec![
            record("zeta", "local", SourceKind::Local, &[]),
            record(
                "alpha",
                "workspace-registry",
                SourceKind::WorkspaceRegistry,
                &[],
            ),
            record("alpha", "local", SourceKind::Local, &[]),
        ];
        let b = vec![
            record("alpha", "local", SourceKind::Local, &[]),
            record("zeta", "local", SourceKind::Local, &[]),
            record(
                "alpha",
                "workspace-registry",
                SourceKind::WorkspaceRegistry,
                &[],
            ),
        ];
        let query = CatalogQuery::default().normalize();
        let query = CatalogQuery { limit: 10, ..query };
        let page_a = apply(&a, &query, "now").unwrap();
        let page_b = apply(&b, &query, "now").unwrap();
        assert_eq!(page_a.records, page_b.records);
        let keys: Vec<(String, String)> = page_a
            .records
            .iter()
            .map(|r| (r.project_id.clone(), r.source.clone()))
            .collect();
        assert_eq!(
            keys,
            vec![
                ("alpha".to_string(), "local".to_string()),
                ("alpha".to_string(), "workspace-registry".to_string()),
                ("zeta".to_string(), "local".to_string()),
            ]
        );
    }

    #[test]
    fn predicates_combine_as_and_and_values_as_or() {
        let records = vec![
            record("alpha", "local", SourceKind::Local, &["product", "web"]),
            record("beta", "local", SourceKind::Local, &["internal"]),
            record("gamma", "local", SourceKind::Local, &["product"]),
        ];
        let query = CatalogQuery {
            tags: vec!["product".to_string(), "internal".to_string()],
            profiles: vec!["rust-web".to_string()],
            limit: 10,
            ..CatalogQuery::default()
        };
        let page = apply(&records, &query, "now").unwrap();
        let ids: Vec<&str> = page.records.iter().map(|r| r.project_id.as_str()).collect();
        assert_eq!(ids, vec!["alpha", "beta", "gamma"]);
        let query = CatalogQuery {
            tags: vec!["product".to_string()],
            lifecycles: vec!["archived".to_string()],
            limit: 10,
            ..CatalogQuery::default()
        };
        let page = apply(&records, &query, "now").unwrap();
        assert!(page.records.is_empty());
        assert_eq!(page.total, 0);
    }

    #[test]
    fn pagination_is_deterministic_and_cursor_is_opaque() {
        let records: Vec<CatalogRecord> = (0..5)
            .map(|i| record(&format!("p{i}"), "local", SourceKind::Local, &[]))
            .collect();
        let query = CatalogQuery {
            limit: 2,
            ..CatalogQuery::default()
        };
        let first = apply(&records, &query, "now").unwrap();
        assert_eq!(first.total, 5);
        assert_eq!(first.records.len(), 2);
        let cursor = first.next_cursor.clone().unwrap();
        assert!(cursor.starts_with("v1:"));
        let second = apply(
            &records,
            &CatalogQuery {
                limit: 2,
                cursor: Some(cursor),
                ..CatalogQuery::default()
            },
            "now",
        )
        .unwrap();
        assert_eq!(second.records[0].project_id, "p2");
        assert!(second.next_cursor.is_some());
    }

    #[test]
    fn malformed_inputs_refuse() {
        let records = vec![record("alpha", "local", SourceKind::Local, &[])];
        let bad_limit = CatalogQuery {
            limit: 0,
            ..CatalogQuery::default()
        };
        assert_eq!(
            apply(&records, &bad_limit, "now").unwrap_err().code(),
            "catalog-invalid"
        );
        let bad_cursor = CatalogQuery {
            limit: 10,
            cursor: Some("nope".to_string()),
            ..CatalogQuery::default()
        };
        assert_eq!(
            apply(&records, &bad_cursor, "now").unwrap_err().code(),
            "catalog-invalid"
        );
    }

    #[test]
    fn unknown_filter_key_is_refused() {
        let err = CatalogQuery::from_pairs(&["colour=red".to_string()], 10, None).unwrap_err();
        assert_eq!(err.code(), "catalog-invalid");
        assert!(err.to_string().contains("colour"), "{err}");
        let err = CatalogQuery::from_pairs(&["no-equals".to_string()], 10, None).unwrap_err();
        assert_eq!(err.code(), "catalog-invalid");
    }
}
