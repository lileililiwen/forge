//! Versioned, read-only project catalog and composable query contract
//! (`project-catalog-query-contract`).
//!
//! The catalog is a **projection**, never a second registry: it does not
//! change the schema, does not migrate anything and never imports a
//! discovered project automatically. Every read is composed from
//! explicitly selected sources and carries the provenance of each value.
//!
//! - [`record`] owns [`CatalogRecord`] and the closed field set.
//! - [`query`] owns the pure filter/order/pagination model.
//! - [`source`] owns the read-only adapters over existing readers.

pub mod query;
pub mod record;
pub mod source;

use crate::core::ForgeError;

pub use query::{apply, filter, CatalogPage, CatalogQuery, DEFAULT_LIMIT, MAX_LIMIT};
pub use record::{
    catalog_invalid, CatalogRecord, EvidenceState, Freshness, SourceKind, CATALOG_CONTRACT_VERSION,
    DEFAULT_MAX_AGE_SECONDS,
};
pub use source::{
    absolute_display, collect, validate_max_age, CatalogRequest, CatalogSourceSelection,
    SourceBundle, SourceStatus,
};

/// One distinct value with how many records carried it.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CatalogValueCount {
    pub value: String,
    pub count: usize,
}

/// All records for one project id, across every selected source, ordered
/// by `(project_id, source)`. A duplicate id in two sources yields two
/// records: they are never merged silently.
pub fn inspect_records(
    bundle: &SourceBundle,
    project_id: &str,
) -> Result<Vec<CatalogRecord>, ForgeError> {
    let mut matches: Vec<CatalogRecord> = bundle
        .records
        .iter()
        .filter(|record| record.project_id == project_id)
        .cloned()
        .collect();
    matches.sort_by(|a, b| {
        a.project_id
            .cmp(&b.project_id)
            .then_with(|| a.source.cmp(&b.source))
    });
    if matches.is_empty() {
        return Err(ForgeError::UnknownProject {
            query: project_id.to_string(),
        });
    }
    Ok(matches)
}

/// Distinct tags across the given records, sorted by value.
pub fn tag_counts(records: &[CatalogRecord]) -> Vec<CatalogValueCount> {
    counts_for(records, |record| record.tags.iter())
}

/// Distinct languages across the given records, sorted by value.
pub fn language_counts(records: &[CatalogRecord]) -> Vec<CatalogValueCount> {
    counts_for(records, |record| record.languages.iter())
}

fn counts_for<'a, F>(records: &'a [CatalogRecord], values: F) -> Vec<CatalogValueCount>
where
    F: Fn(&'a CatalogRecord) -> std::slice::Iter<'a, String>,
{
    let mut counts: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    for record in records {
        for value in values(record) {
            let key = value.to_ascii_lowercase();
            *counts.entry(key).or_insert(0) += 1;
        }
    }
    counts
        .into_iter()
        .map(|(value, count)| CatalogValueCount { value, count })
        .collect()
}

/// Human-facing table for a catalog page. This layout is not the
/// machine contract; JSON and NDJSON are.
pub fn render_page_human(page: &CatalogPage) -> String {
    let mut out = format!("forge project list — contract {}\n", page.contract);
    if page.sources.is_empty() {
        out.push_str("sources: (none)\n");
    } else {
        for status in &page.sources {
            out.push_str(&format!(
                "source: {} [{}] state={} records={}{}\n",
                status.source,
                status.source_kind,
                status.state,
                status.records,
                status
                    .reason
                    .as_deref()
                    .map(|reason| format!(" reason={reason}"))
                    .unwrap_or_default(),
            ));
        }
    }
    out.push_str(&format!(
        "records: {} of {} (limit {}){}\n",
        page.records.len(),
        page.total,
        page.limit,
        page.next_cursor
            .as_deref()
            .map(|cursor| format!(" next_cursor={cursor}"))
            .unwrap_or_default(),
    ));
    out.push_str(&render_table(&page.records));
    out
}

/// Human-facing projection for `forge project inspect`.
pub fn render_records_human(title: &str, records: &[CatalogRecord]) -> String {
    let mut out = format!("forge project {title} — contract {CATALOG_CONTRACT_VERSION}\n");
    out.push_str(&format!("records: {}\n", records.len()));
    for record in records {
        out.push_str(&format!(
            "  project={} source={} kind={} freshness={} evidence={}\n",
            record.project_id, record.source, record.source_kind, record.freshness, record.evidence,
        ));
        out.push_str(&format!(
            "    profile={} lifecycle={} revision={}\n",
            record.profile.as_deref().unwrap_or("unknown"),
            record.lifecycle.as_deref().unwrap_or("unknown"),
            record.source_revision.as_deref().unwrap_or("unknown"),
        ));
        out.push_str(&format!(
            "    repository={} languages={} tags={} ci={} compose={} observed_at={}\n",
            record.repository.as_deref().unwrap_or("unknown"),
            join_or_unknown(&record.languages),
            join_or_unknown(&record.tags),
            record.ci.as_deref().unwrap_or("unknown"),
            record.compose.as_deref().unwrap_or("unknown"),
            record.observed_at,
        ));
    }
    out
}

/// Human-facing value/count projection for `forge project tags|languages`.
pub fn render_counts_human(title: &str, counts: &[CatalogValueCount]) -> String {
    let mut out = format!("forge project {title} — contract {CATALOG_CONTRACT_VERSION}\n");
    if counts.is_empty() {
        out.push_str("(none)\n");
        return out;
    }
    for entry in counts {
        out.push_str(&format!("{} {}\n", entry.value, entry.count));
    }
    out
}

fn render_table(records: &[CatalogRecord]) -> String {
    if records.is_empty() {
        return "(none)\n".to_string();
    }
    let mut out = format!(
        "{:<20} {:<20} {:<14} {:<12} {:<9} {}\n",
        "PROJECT", "SOURCE", "PROFILE", "LIFECYCLE", "FRESH", "EVIDENCE"
    );
    for record in records {
        out.push_str(&format!(
            "{:<20} {:<20} {:<14} {:<12} {:<9} {}\n",
            truncate(&record.project_id, 20),
            truncate(&record.source, 20),
            truncate(record.profile.as_deref().unwrap_or("unknown"), 14),
            truncate(record.lifecycle.as_deref().unwrap_or("unknown"), 12),
            record.freshness,
            record.evidence,
        ));
    }
    out
}

fn join_or_unknown(values: &[String]) -> String {
    if values.is_empty() {
        "unknown".to_string()
    } else {
        values.join(",")
    }
}

fn truncate(value: &str, max: usize) -> String {
    if value.chars().count() <= max {
        return value.to_string();
    }
    let kept: String = value.chars().take(max.saturating_sub(1)).collect();
    format!("{kept}…")
}
