//! Portfolio interest: the one path from an importer's batch to
//! stored evidence, and the one path from stored evidence back out as a
//! comparison.
//!
//! Every transport — the CLI, the JSON API and (later) a browser —
//! enters through this module. Nothing else is allowed to write an
//! interest row: a writer that could bypass [`import_snapshots`] would
//! bypass the overlap rule, and an overlap is the one mistake that
//! makes a later number quietly wrong.
//!
//! The import is deliberately *per-record* rather than
//! all-or-nothing. A batch of fifty windows where three are malformed
//! should store the forty-seven that are sound and say plainly what
//! happened to the other three — refusing the whole batch would teach
//! an importer to validate locally instead, which is exactly where the
//! privacy rules have to hold. The rejections come back with the field
//! and the rule and never with the value.

use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::core::ForgeError;
use crate::portfolio::interest::{
    self, bound_stale_after_days, build_comparison, build_readiness, build_trend, refusal,
    ActivationReport, Comparison, Freshness, InterestImport, InterestMetric, InterestRejection,
    InterestSnapshot, RawSnapshot, RequestedWindow, Trend, DEFAULT_STALE_AFTER_DAYS,
    MAX_COMPARE_PROJECTS, MAX_SNAPSHOTS_PER_IMPORT, MAX_TREND_POINTS,
};
use crate::registry::{Registry, SnapshotOutcome};

/// What one import attempt did across the whole batch.
#[derive(Debug, Clone, Serialize)]
pub struct ImportReport {
    pub received: usize,
    pub accepted: Vec<InterestSnapshot>,
    /// Snapshots that were re-sent unchanged. Each entry is an
    /// idempotent success, not an error: an adapter that retries after
    /// a timeout must not have to guess whether it landed.
    pub already_present: Vec<InterestSnapshot>,
    /// Snapshots that landed and replaced an earlier revision.
    pub supersessions: Vec<Supersession>,
    pub rejected: Vec<InterestRejection>,
    pub actor: String,
}

impl ImportReport {
    /// The number of records the store now holds from this batch.
    pub fn stored(&self) -> usize {
        self.accepted.len() + self.already_present.len() + self.supersessions.len()
    }

    /// The number of records the batch refused.
    pub fn refused(&self) -> usize {
        self.rejected.len()
    }

    pub fn is_complete_failure(&self) -> bool {
        self.rejected.len() == self.received && self.received > 0
    }
}

/// One accepted snapshot together with the revision it replaced.
#[derive(Debug, Clone, Serialize)]
pub struct Supersession {
    pub snapshot: InterestSnapshot,
    pub superseded: InterestSnapshot,
}

/// What one project's stored interest evidence looks like.
///
/// This is the *history* view: it carries superseded revisions so an
/// operator can see what a source reported and what replaced it. The
/// comparison and trend views deliberately narrow to current evidence
/// instead, so one window never answers the same question twice.
#[derive(Debug, Clone, Serialize)]
pub struct ProjectInterest {
    pub project_id: String,
    pub snapshots: Vec<InterestSnapshot>,
    pub stale_after_days: i64,
    /// How many *current* windows read as stale. Superseded revisions
    /// are counted separately: their staleness describes no live
    /// evidence.
    pub stale_snapshots: usize,
    /// How many stored revisions were replaced by a later source
    /// revision.
    pub superseded_snapshots: usize,
    pub findings: Vec<InterestRejection>,
}

/// Import one batch of raw records.
///
/// Each record goes through [`interest::validate_snapshot`] and then
/// through the registry's overlap rule. A refused record persists a
/// finding and appears in [`ImportReport::rejected`]; a batch whose
/// records are *all* refused is still a report, not an exception, so a
/// caller can render exactly which records failed and why.
pub fn import_snapshots(
    registry: &Registry,
    import: &InterestImport,
    actor: &str,
    now: DateTime<Utc>,
) -> Result<ImportReport, ForgeError> {
    let actor = interest_validate_actor(actor)?;
    if import.records.is_empty() {
        return Err(ForgeError::PortfolioInterestInvalid {
            reason: "an import must carry at least one snapshot".to_string(),
        });
    }
    if import.records.len() > MAX_SNAPSHOTS_PER_IMPORT {
        return Err(ForgeError::PortfolioInterestInvalid {
            reason: format!("an import carries more than {MAX_SNAPSHOTS_PER_IMPORT} snapshots"),
        });
    }
    // The reception time belongs to the import, so a retry of the same
    // batch produces the same `received_at` and a re-read of the store
    // is reproducible.
    let received_at = now.to_rfc3339();

    let mut report = ImportReport {
        received: import.records.len(),
        accepted: Vec::new(),
        already_present: Vec::new(),
        supersessions: Vec::new(),
        rejected: Vec::new(),
        actor,
    };

    for RawSnapshot { index, record } in &import.records {
        // A project id is needed for the finding row, so it is read
        // before validation rather than after: a refused record still
        // needs somewhere to say *why* it was refused.
        let project_id = record
            .get("project_id")
            .and_then(|value| value.as_str())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("<unknown>")
            .to_string();

        let write = match interest::validate_snapshot(record) {
            Ok(write) => write,
            Err(rejected) => {
                report.rejected.push(record_refusal(
                    registry,
                    index,
                    &project_id,
                    rejected.code,
                    &rejected.detail,
                ));
                continue;
            }
        };

        match registry.interest_insert_snapshot(&write, &report.actor, &received_at) {
            Ok(SnapshotOutcome::Accepted { snapshot }) => report.accepted.push(snapshot),
            Ok(SnapshotOutcome::AlreadyPresent { snapshot }) => {
                report.already_present.push(snapshot)
            }
            Ok(SnapshotOutcome::AcceptedWithSupersession {
                snapshot,
                superseded,
            }) => report.supersessions.push(Supersession {
                snapshot,
                superseded: *superseded,
            }),
            Err(err) => {
                // An unknown project is a refusal of the *request*,
                // not of one record: reporting it per record would let
                // a typo'd project id look like a batch of bad data,
                // and every other record in the batch would be stored
                // against a project that does not exist.
                if err.code() == "unknown-project" {
                    return Err(err);
                }
                let code = if err.code() == "portfolio-interest-conflict" {
                    refusal::OVERLAP
                } else {
                    refusal::STORE
                };
                report.rejected.push(record_refusal(
                    registry,
                    index,
                    &write.project_id,
                    code,
                    reason_of(&err),
                ));
            }
        }
    }

    Ok(report)
}

/// Persist one finding and build the caller-facing rejection.
///
/// The finding write is best effort: an audit-trail failure must not
/// turn a well-reported refusal into an unrelated registry error, and
/// the rejection still names the field and the rule. The code and the
/// detail are kept apart so a caller can branch on the former and read
/// the latter.
fn record_refusal(
    registry: &Registry,
    index: &usize,
    project_id: &str,
    code: &'static str,
    detail: &str,
) -> InterestRejection {
    let _ = registry.record_interest_finding(project_id, "snapshot", code, detail);
    InterestRejection::new(*index, project_id, "snapshot", code, detail)
}

/// The bare reason of a typed conflict, without the error's own
/// prefix and its "nothing was changed" clause.
///
/// A finding is read on its own rather than as part of a terminal
/// message, so wrapping it in the `Display` of the error would repeat
/// the code the caller already has and add a guarantee that belongs to
/// the request, not to the record.
fn reason_of(err: &ForgeError) -> &str {
    match err {
        ForgeError::PortfolioInterestConflict { reason } => reason,
        other => other.code(),
    }
}

/// Read one project's whole interest projection.
pub fn project_interest(
    registry: &Registry,
    project_id: &str,
    stale_after_days: i64,
    now: DateTime<Utc>,
) -> Result<ProjectInterest, ForgeError> {
    registry.require_project(project_id)?;
    let stale_after_days = bound_stale_after_days(stale_after_days)
        .map_err(|reason| ForgeError::PortfolioInterestInvalid { reason })?;
    let snapshots = registry.interest_snapshots(project_id, MAX_TREND_POINTS)?;
    let stale_snapshots = snapshots
        .iter()
        .filter(|snapshot| {
            snapshot.state.is_current()
                && snapshot.freshness(now, stale_after_days) == Freshness::Stale
        })
        .count();
    let superseded_snapshots = snapshots
        .iter()
        .filter(|snapshot| !snapshot.state.is_current())
        .count();
    // Findings are scoped to the project: an audit answer about one
    // project must not enumerate every other project's refusals.
    let findings = registry
        .interest_findings(MAX_TREND_POINTS)?
        .into_iter()
        .filter(|finding| finding.project_id == project_id)
        .map(|finding| InterestRejection {
            index: 0,
            project_id: finding.project_id,
            field: finding.field,
            code: finding.code,
            detail: finding.detail,
        })
        .collect();
    Ok(ProjectInterest {
        project_id: project_id.to_string(),
        snapshots,
        stale_after_days,
        stale_snapshots,
        superseded_snapshots,
        findings,
    })
}

/// Compare several projects on the allowlisted metrics.
///
/// The comparison is built from *current* snapshots only and carries no
/// total, no average and no rank. When the projects did not report the
/// same window the result says so instead of ordering them anyway.
pub fn compare_projects(
    registry: &Registry,
    project_ids: &[String],
    metrics: &[InterestMetric],
    source: Option<&str>,
    stale_after_days: i64,
    now: DateTime<Utc>,
) -> Result<Comparison, ForgeError> {
    let stale_after_days = bound_stale_after_days(stale_after_days)
        .map_err(|reason| ForgeError::PortfolioInterestInvalid { reason })?;
    if project_ids.is_empty() {
        return Err(ForgeError::PortfolioInterestInvalid {
            reason: "a comparison needs at least one project".to_string(),
        });
    }
    if project_ids.len() > MAX_COMPARE_PROJECTS {
        return Err(ForgeError::PortfolioInterestInvalid {
            reason: format!(
                "a comparison spans at most {MAX_COMPARE_PROJECTS} projects; \
                 Forge will not silently narrow the list"
            ),
        });
    }
    if metrics.is_empty() {
        return Err(ForgeError::PortfolioInterestInvalid {
            reason: format!(
                "a comparison needs at least one metric; expected one of {}",
                InterestMetric::labels().join(", ")
            ),
        });
    }
    let mut sorted = project_ids.to_vec();
    sorted.sort();
    sorted.dedup();
    // Every project must exist before any read: a typo'd id would
    // otherwise produce a comparison that silently omits it.
    for project_id in &sorted {
        registry.require_project(project_id)?;
    }
    let snapshots = registry.interest_current_snapshots_for_projects(&sorted, source)?;
    Ok(build_comparison(&snapshots, metrics, now, stale_after_days))
}

/// One metric's windowed history for one project.
///
/// The series is built from *current* snapshots only: a superseded
/// revision describes the same window as the revision that replaced
/// it, and plotting both would show two answers to one question. The
/// full history, superseded revisions included, is available through
/// [`project_interest`].
pub fn interest_trend(
    registry: &Registry,
    project_id: &str,
    metric: InterestMetric,
    limit: usize,
    stale_after_days: i64,
    now: DateTime<Utc>,
) -> Result<Trend, ForgeError> {
    registry.require_project(project_id)?;
    let stale_after_days = bound_stale_after_days(stale_after_days)
        .map_err(|reason| ForgeError::PortfolioInterestInvalid { reason })?;
    if limit == 0 || limit > MAX_TREND_POINTS {
        return Err(ForgeError::PortfolioInterestInvalid {
            reason: format!("limit must be between 1 and {MAX_TREND_POINTS}"),
        });
    }
    let snapshots = registry.interest_current_snapshots(project_id)?;
    Ok(build_trend(
        project_id,
        metric,
        &snapshots,
        limit,
        now,
        stale_after_days,
    ))
}

/// Read-only readiness verdicts for several projects on one metric.
///
/// The report is built from *current* snapshots only, carries no
/// stored state, and writes no row: absence of evidence is a
/// `not-ready` verdict, never an invented zero. Every id is resolved
/// before any read so a typo cannot produce a silently narrowed
/// report, and verdicts come back in project id order so repeated
/// reads are byte-identical.
#[allow(clippy::too_many_arguments)]
pub fn activation_readiness(
    registry: &Registry,
    project_ids: &[String],
    metric: InterestMetric,
    threshold: Option<u64>,
    source: Option<&str>,
    requested_window: Option<(&str, &str)>,
    stale_after_days: i64,
    now: DateTime<Utc>,
) -> Result<ActivationReport, ForgeError> {
    let stale_after_days = bound_stale_after_days(stale_after_days)
        .map_err(|reason| ForgeError::PortfolioInterestInvalid { reason })?;
    if project_ids.is_empty() {
        return Err(ForgeError::PortfolioInterestInvalid {
            reason: "readiness needs at least one project to evaluate".to_string(),
        });
    }
    let mut sorted = project_ids.to_vec();
    sorted.sort();
    sorted.dedup();
    for project_id in &sorted {
        registry.require_project(project_id)?;
    }
    let source = source.map(str::to_string);
    let mut verdicts = Vec::with_capacity(sorted.len());
    for project_id in &sorted {
        let counts = registry.interest_snapshot_counts(project_id)?;
        let snapshots = registry.interest_current_snapshots(project_id)?;
        verdicts.push(build_readiness(
            project_id,
            metric,
            threshold,
            source.as_deref(),
            requested_window,
            counts,
            &snapshots,
            now,
            stale_after_days,
        ));
    }
    let ready_count = verdicts.iter().filter(|v| v.readiness.is_ready()).count();
    let not_ready_count = verdicts.len() - ready_count;
    Ok(ActivationReport {
        metric: metric.label().to_string(),
        threshold,
        stale_after_days,
        requested_window: requested_window.map(|(start, end)| RequestedWindow {
            start: start.to_string(),
            end: end.to_string(),
        }),
        requested_source: source,
        verdicts,
        ready_count,
        not_ready_count,
    })
}

/// Bound an import document against the size an operator can send.
pub fn bound_import_bytes(len: usize) -> Result<(), ForgeError> {
    if len > interest::MAX_IMPORT_BYTES {
        return Err(ForgeError::PortfolioInterestInvalid {
            reason: format!(
                "import document is larger than {} bytes",
                interest::MAX_IMPORT_BYTES
            ),
        });
    }
    Ok(())
}

/// Validate the importing actor. Provenance is the point of the store:
/// an import whose actor cannot be named has no audit value, so it is
/// refused rather than recorded as an unattributed fact.
fn interest_validate_actor(raw: &str) -> Result<String, ForgeError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.chars().count() > 128 {
        return Err(ForgeError::PortfolioInterestInvalid {
            reason: "actor is required and must be at most 128 characters".to_string(),
        });
    }
    if trimmed.chars().any(|c| c.is_control() || c.is_whitespace()) {
        return Err(ForgeError::PortfolioInterestInvalid {
            reason: "actor must not contain whitespace or control characters".to_string(),
        });
    }
    if interest::looks_like_secret(trimmed) {
        return Err(ForgeError::PortfolioInterestInvalid {
            reason: "actor was refused because it carries a credential-shaped value; \
                     the value itself was not recorded"
                .to_string(),
        });
    }
    Ok(trimmed.to_string())
}

/// The house staleness bound, for callers that do not declare one.
pub fn default_stale_after_days() -> i64 {
    DEFAULT_STALE_AFTER_DAYS
}
