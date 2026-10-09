//! API fleet: sources.

use super::super::fleet_data::load_fleet_list;
use crate::core::ForgeError;
use crate::fleet::{self, FleetFreshness};
use crate::publish::inventory;
use crate::registry::Registry;
use chrono::{DateTime, SecondsFormat, Utc};
use serde_json::Value;
use std::path::Path;

use super::contract::SELF_NAME;
use super::model::{CandidateRow, Management, RowFreshness, RowSource, SourceDescriptor};

use super::config::{
    configured_max_age, configured_publish_limit, publish_history_disabled, self_identity,
};
use super::envelope::{build_envelope, classify_generated_at, project_publish};

/// Load and aggregate the fleet for the authenticated admin response. Reads
/// only the explicitly configured external sources resolved from the
/// established environment path; nothing is scanned.
pub fn load(db_path: &Path) -> Result<Value, ForgeError> {
    let now = Utc::now();
    let (candidates, sources) = gather(
        db_path,
        inventory::resolve_source(None).as_deref(),
        fleet::resolve_registry_path(None).as_deref(),
        configured_max_age(),
        now,
    )?;
    Ok(build_envelope(candidates, sources, now))
}

/// Gather the real source data into candidate rows and source descriptors.
/// Split from [`build_envelope`] so the merge/sort/serialize logic stays
/// pure and independently testable.
pub fn gather(
    db_path: &Path,
    inventory_source: Option<&Path>,
    workspace_registry: Option<&Path>,
    max_age_seconds: i64,
    now: DateTime<Utc>,
) -> Result<(Vec<CandidateRow>, Vec<SourceDescriptor>), ForgeError> {
    let self_id = self_identity();
    let mut candidates: Vec<CandidateRow> = Vec::new();
    let mut sources: Vec<SourceDescriptor> = Vec::new();

    // --- registry (local managed projects + Forge-self merge) ---
    let fleet_view = load_fleet_list(db_path)?;
    let registry_rows = &fleet_view.rows;
    let local_ids: std::collections::BTreeSet<String> =
        registry_rows.iter().map(|row| row.id.clone()).collect();

    let registry_count;
    let mut merged_self: Option<CandidateRow> = None;
    for row in registry_rows {
        let evidence = row
            .evidence
            .iter()
            .map(|(source, status)| (source.clone(), status.label().to_string()))
            .collect::<Vec<_>>();
        let is_self_row = row.id == self_id;
        let candidate = CandidateRow {
            identity: row.id.clone(),
            name: row.id.clone(),
            profile: Some(row.profile.clone()),
            state: row.last_state.clone(),
            source: if is_self_row {
                RowSource::SelfRecord
            } else {
                RowSource::Registry
            },
            management: if is_self_row {
                Management::SelfRecord
            } else {
                Management::Managed
            },
            is_self: is_self_row,
            freshness: RowFreshness::Fresh,
            updated_at: Some(row.last_at.clone()),
            capabilities: vec!["inspect".to_string()],
            evidence: evidence.clone(),
            lifecycle: row
                .lifecycle
                .map(|value| format!("{value:?}").to_ascii_lowercase()),
            confidence: row
                .confidence
                .map(|value| format!("{value:?}").to_ascii_lowercase()),
            tags: row.tags.clone(),
            has_registry_ref: is_self_row,
            publish: None,
        };
        if is_self_row {
            merged_self = Some(candidate);
        } else {
            candidates.push(candidate);
        }
    }
    registry_count = candidates
        .iter()
        .filter(|candidate| candidate.source == RowSource::Registry)
        .count();

    let self_row = merged_self.unwrap_or_else(|| CandidateRow {
        identity: self_id.clone(),
        name: SELF_NAME.to_string(),
        profile: Some(env!("CARGO_PKG_NAME").to_string()),
        state: "self".to_string(),
        source: RowSource::SelfRecord,
        management: Management::SelfRecord,
        is_self: true,
        freshness: RowFreshness::Fresh,
        updated_at: Some(now.to_rfc3339_opts(SecondsFormat::Secs, true)),
        // Forge itself is only inspectable through the registry once it is a
        // registered project; an unregistered self record exposes no action.
        capabilities: Vec::new(),
        evidence: Vec::new(),
        lifecycle: None,
        confidence: None,
        tags: Vec::new(),
        has_registry_ref: false,
        publish: None,
    });
    // The self row always leads the candidate list and appears exactly once.
    candidates.insert(0, self_row);

    sources.push(SourceDescriptor {
        id: "registry",
        kind: "forge-registry",
        status: "available",
        count: registry_count,
        malformed: Vec::new(),
        reason: None,
        observed_at: Some(now.to_rfc3339_opts(SecondsFormat::Secs, true)),
        provider: None,
    });

    // --- local publish-history projection ---
    // Reads the same local, Forge-owned registry the fleet already opened.
    // A published id that is already registered (or the self id) is merged
    // into its managed row so no false identity conflict is introduced.
    let mut managed_ids = local_ids.clone();
    managed_ids.insert(self_id.clone());
    sources.push(read_published(
        db_path,
        max_age_seconds,
        &managed_ids,
        &mut candidates,
        now,
    ));

    // --- portable inventory external source ---
    let (inventory_rows, inventory_descriptor) =
        read_inventory(inventory_source, max_age_seconds, now);
    candidates.extend(inventory_rows);
    sources.push(inventory_descriptor);

    // --- workspace fleet registry external source ---
    let (fleet_rows, fleet_descriptor) =
        read_workspace_registry(workspace_registry, max_age_seconds, &local_ids);
    candidates.extend(fleet_rows);
    sources.push(fleet_descriptor);

    Ok((candidates, sources))
}

fn read_inventory(
    source: Option<&Path>,
    max_age_seconds: i64,
    now: DateTime<Utc>,
) -> (Vec<CandidateRow>, SourceDescriptor) {
    let Some(path) = source else {
        return (
            Vec::new(),
            SourceDescriptor::unconfigured("inventory", "forge-project-inventory"),
        );
    };
    match inventory::load_local(path) {
        Ok(snapshot) => {
            let freshness = classify_generated_at(&snapshot.generated_at, max_age_seconds, now);
            let status = match freshness {
                RowFreshness::Stale => "stale",
                _ => "available",
            };
            let malformed = snapshot
                .malformed
                .iter()
                .map(|entry| (entry.name.clone(), entry.reason.clone()))
                .collect::<Vec<_>>();
            let rows = snapshot
                .projects
                .iter()
                .map(|entry| CandidateRow {
                    identity: entry.id.clone(),
                    name: entry.id.clone(),
                    profile: Some(entry.profile.clone()),
                    state: "observed".to_string(),
                    source: RowSource::Inventory,
                    management: Management::Observed,
                    is_self: false,
                    freshness,
                    updated_at: Some(snapshot.generated_at.clone()),
                    capabilities: Vec::new(),
                    evidence: Vec::new(),
                    lifecycle: None,
                    confidence: None,
                    tags: Vec::new(),
                    has_registry_ref: false,
                    publish: None,
                })
                .collect();
            (
                rows,
                SourceDescriptor {
                    id: "inventory",
                    kind: "forge-project-inventory",
                    status,
                    count: snapshot.projects.len(),
                    malformed,
                    reason: None,
                    observed_at: Some(snapshot.generated_at.clone()),
                    provider: Some(snapshot.provider.clone()),
                },
            )
        }
        // A configured source that cannot be read is unavailable with a safe
        // reason; the underlying error text (which may name a path) is never
        // surfaced. Other sources remain visible.
        Err(_) => (
            Vec::new(),
            SourceDescriptor {
                id: "inventory",
                kind: "forge-project-inventory",
                status: "unavailable",
                count: 0,
                malformed: Vec::new(),
                reason: Some("the configured inventory source could not be read".to_string()),
                observed_at: None,
                provider: None,
            },
        ),
    }
}

fn read_workspace_registry(
    source: Option<&Path>,
    max_age_seconds: i64,
    local_ids: &std::collections::BTreeSet<String>,
) -> (Vec<CandidateRow>, SourceDescriptor) {
    let Some(path) = source else {
        return (
            Vec::new(),
            SourceDescriptor::unconfigured("fleet", "workspace-registry"),
        );
    };
    match fleet::observe(Some(path), max_age_seconds, local_ids) {
        Ok(report) => {
            let freshness = match report.freshness {
                FleetFreshness::Stale => RowFreshness::Stale,
                FleetFreshness::Fresh => RowFreshness::Fresh,
                FleetFreshness::Unconfigured => RowFreshness::Unconfigured,
            };
            let status = match report.freshness {
                FleetFreshness::Stale => "stale",
                FleetFreshness::Fresh => "available",
                FleetFreshness::Unconfigured => "unconfigured",
            };
            let malformed = report
                .malformed
                .iter()
                .map(|entry| (entry.name.clone(), entry.reason.clone()))
                .collect::<Vec<_>>();
            let rows = report
                .entries
                .iter()
                .map(|entry| CandidateRow {
                    identity: entry.id.clone(),
                    name: entry.id.clone(),
                    profile: Some(entry.profile.clone()),
                    state: entry.lifecycle.clone(),
                    source: RowSource::WorkspaceRegistry,
                    // A fleet entry is this source's view; even when the
                    // same id is locally registered, the registry row is the
                    // managed one and this row stays observed. The shared
                    // identity is surfaced as a conflict by build_envelope.
                    management: Management::Observed,
                    is_self: false,
                    freshness,
                    updated_at: Some(report.observed_at.clone()),
                    capabilities: Vec::new(),
                    evidence: Vec::new(),
                    lifecycle: Some(entry.lifecycle.clone()),
                    confidence: None,
                    tags: Vec::new(),
                    has_registry_ref: false,
                    publish: None,
                })
                .collect();
            (
                rows,
                SourceDescriptor {
                    id: "fleet",
                    kind: "workspace-registry",
                    status,
                    count: report.entries.len(),
                    malformed,
                    reason: None,
                    observed_at: Some(report.observed_at.clone()),
                    provider: None,
                },
            )
        }
        Err(_) => (
            Vec::new(),
            SourceDescriptor {
                id: "fleet",
                kind: "workspace-registry",
                status: "unavailable",
                count: 0,
                malformed: Vec::new(),
                reason: Some(
                    "the configured workspace registry source could not be read".to_string(),
                ),
                observed_at: None,
                provider: None,
            },
        ),
    }
}

/// Project the local publish journal into the fleet. Standalone rows are
/// pushed onto `candidates`; an identity that is already managed (registered
/// or self) is enriched in place. Never reads an external path and never
/// writes. A failure is reported as an unavailable source descriptor and
/// leaves every other source intact.
fn read_published(
    db_path: &Path,
    max_age_seconds: i64,
    managed_ids: &std::collections::BTreeSet<String>,
    candidates: &mut Vec<CandidateRow>,
    now: DateTime<Utc>,
) -> SourceDescriptor {
    let descriptor = |status: &'static str, reason: Option<String>| SourceDescriptor {
        id: "published",
        kind: "forge-publish-history",
        status,
        count: 0,
        malformed: Vec::new(),
        reason,
        observed_at: None,
        provider: None,
    };
    if publish_history_disabled() {
        return descriptor(
            "unconfigured",
            Some("publish history projection is disabled by FORGE_PUBLISH_HISTORY".to_string()),
        );
    }
    let registry = match Registry::open(db_path) {
        Ok(registry) => registry,
        Err(_) => {
            return descriptor(
                "unavailable",
                Some("the local publish journal could not be read".to_string()),
            )
        }
    };
    let publishes = match registry.latest_publishes(configured_publish_limit()) {
        Ok(publishes) => publishes,
        Err(_) => {
            return descriptor(
                "unavailable",
                Some("the local publish journal could not be projected".to_string()),
            )
        }
    };

    let count = publishes.len();
    for publish in publishes {
        let projection = project_publish(&publish);
        let merged = managed_ids.contains(&publish.project_id);
        if merged {
            let target = candidates.iter_mut().find(|candidate| {
                candidate.identity == publish.project_id
                    && (candidate.source == RowSource::Registry || candidate.is_self)
            });
            if let Some(row) = target {
                row.publish = Some(projection);
                continue;
            }
        }
        // Published-only project: an observed row, inspection only. A stale
        // observation keeps its own freshness label; the row is never
        // downgraded on a guess.
        let freshness = classify_generated_at(&publish.started_at, max_age_seconds, now);
        candidates.push(CandidateRow {
            identity: publish.project_id.clone(),
            name: publish.project_id.clone(),
            profile: None,
            state: publish.state.clone(),
            source: RowSource::Published,
            management: Management::Observed,
            is_self: false,
            freshness,
            updated_at: Some(publish.started_at.clone()),
            capabilities: Vec::new(),
            evidence: Vec::new(),
            lifecycle: None,
            confidence: None,
            tags: Vec::new(),
            has_registry_ref: false,
            publish: Some(projection),
        });
    }

    SourceDescriptor {
        id: "published",
        kind: "forge-publish-history",
        status: "available",
        count,
        malformed: Vec::new(),
        reason: None,
        observed_at: Some(now.to_rfc3339_opts(SecondsFormat::Secs, true)),
        provider: None,
    }
}
