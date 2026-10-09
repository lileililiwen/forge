//! Portfolio interest persistence: immutable aggregate snapshots,
//! their metrics and the refusals that kept bad ones out.
//!
//! These tables share the registry's SQLite file and its migration
//! lifecycle with the rest of the portfolio domain but never a row
//! with it. Four boundaries are deliberate:
//!
//! - **Snapshots are append-only.** There is no `UPDATE` that changes
//!   a stored value, no delete, and no edit path. The single state a
//!   row ever moves through is `accepted` → `superseded`, and only
//!   when a *later source revision* declares the replacement. The
//!   earlier row stays readable so the history of what a source said
//!   and when is never lost.
//! - **Identity is a uniqueness constraint, not a convention.**
//!   `(project_id, source, source_revision, window_start,
//!   window_end)` is what the source attested to; re-importing it is
//!   an idempotent success, and re-importing it with *different*
//!   values is a contradiction Forge refuses rather than resolves.
//! - **Metrics are rows, not a JSON blob.** A `metrics` column would
//!   be an arbitrary map — exactly the shape this package refuses on
//!   input. One row per metric keeps the allowlist enforced by a
//!   foreign-key-shaped uniqueness rule instead of by convention.
//! - **Findings are evidence, not state.** A refused record persists
//!   the field and the rule. The offending value never reaches a row.

use chrono::Utc;
use rusqlite::{params, OptionalExtension};

use crate::core::ForgeError;
use crate::portfolio::interest::{
    self, InterestFinding, InterestMetric, InterestSnapshot, InterestWrite, MetricValue,
    SnapshotState,
};

pub mod model;

pub(crate) use model::{conflict, invalid, row_to_interest_snapshot};
pub use model::{SnapshotOutcome, PORTFOLIO_INTEREST_SCHEMA_SQL};

impl crate::registry::Registry {
    // --- snapshots -------------------------------------------------------

    /// Store one validated snapshot, or resolve it as an idempotent
    /// repeat.
    ///
    /// Validation runs to completion before the transaction opens, so
    /// no partially-validated row exists. The overlap rule is the
    /// interesting part and runs against the *stored* evidence: a new
    /// window from a source that already has a window covering the
    /// same instants is refused unless the source declares which
    /// revision it replaces. Storing both would let a later read
    /// double-count the shared days.
    pub fn interest_insert_snapshot(
        &self,
        write: &InterestWrite,
        actor: &str,
        received_at: &str,
    ) -> Result<SnapshotOutcome, ForgeError> {
        self.require_project(&write.project_id)?;

        let existing = self.interest_snapshot_by_identity(write)?;
        if let Some(existing) = existing {
            let stored = interest::metric_map(&existing.metrics);
            let incoming = interest::metric_map(&write.metrics);
            if stored != incoming {
                return Err(conflict(format!(
                    "source `{}` already attested revision `{}` for the window \
                     `{}`..`{}` of project `{}` with different values; an accepted snapshot is \
                     immutable, so this is refused rather than merged",
                    write.source,
                    write.source_revision,
                    write.window_start,
                    write.window_end,
                    write.project_id
                )));
            }
            return Ok(SnapshotOutcome::AlreadyPresent { snapshot: existing });
        }

        let superseded = self.interest_resolve_overlap(write)?;

        let tx = self.interest_transaction()?;
        tx.execute(
            "INSERT INTO portfolio_interest_snapshots
                (project_id, source, source_revision, window_start, window_end,
                 privacy_mode, coverage, state, replaces_source_revision, actor, received_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'accepted', ?8, ?9, ?10)",
            params![
                write.project_id,
                write.source,
                write.source_revision,
                write.window_start,
                write.window_end,
                write.privacy_mode.label(),
                write.coverage.label(),
                write.replaces_source_revision,
                actor,
                received_at,
            ],
        )?;
        let snapshot_id = tx.last_insert_rowid();
        for value in &write.metrics {
            tx.execute(
                "INSERT INTO portfolio_interest_metrics (snapshot_id, metric, value)
                 VALUES (?1, ?2, ?3)",
                params![snapshot_id, value.metric, value.value as i64],
            )?;
        }
        // A declared replacement only becomes superseded once the
        // replacement is durably in the same transaction, so a failed
        // write can never demote live evidence.
        if let Some(replaced) = &superseded {
            tx.execute(
                "UPDATE portfolio_interest_snapshots SET state = ?1 WHERE id = ?2",
                params![SnapshotState::Superseded.label(), replaced.id],
            )?;
        }
        tx.commit()?;

        let snapshot = self.interest_snapshot(snapshot_id)?.ok_or_else(|| {
            invalid("interest snapshot disappeared after an accepted write".to_string())
        })?;
        Ok(match superseded {
            // Re-read rather than reuse: the row read before the
            // transaction still carried `accepted`, and a caller that
            // rendered the outcome would report live evidence as
            // superseded history.
            Some(replaced) => {
                let superseded = self.interest_snapshot(replaced.id)?.ok_or_else(|| {
                    invalid("superseded interest snapshot disappeared".to_string())
                })?;
                SnapshotOutcome::AcceptedWithSupersession {
                    snapshot,
                    superseded: Box::new(superseded),
                }
            }
            None => SnapshotOutcome::Accepted { snapshot },
        })
    }

    /// Decide whether a new window collides with stored evidence from
    /// the same project and source, and return the snapshot it
    /// declares as replaced.
    ///
    /// `None` means "no collision, nothing to supersede". An
    /// overlap without a declaration is a conflict; a declaration
    /// that names nothing overlapping is *also* a conflict, because a
    /// replacement whose target is absent would silently demote
    /// evidence for an unrelated reason.
    fn interest_resolve_overlap(
        &self,
        write: &InterestWrite,
    ) -> Result<Option<InterestSnapshot>, ForgeError> {
        let stored = self.interest_snapshots_for_source(&write.project_id, &write.source)?;
        let overlapping: Vec<InterestSnapshot> = stored
            .iter()
            .filter(|snapshot| snapshot.state.is_current() && snapshot.overlaps(write))
            .cloned()
            .collect();
        if overlapping.is_empty() {
            return Ok(None);
        }
        match &write.replaces_source_revision {
            None => Err(conflict(format!(
                "source `{}` already reported the window `{}`..`{}` for project `{}`, and the \
                 new window `{}`..`{}` overlaps it; overlapping windows are refused because a \
                 later read would count the shared days twice. Declare the replaced revision \
                 with `replaces_source_revision` to re-measure deliberately",
                write.source,
                overlapping[0].window_start,
                overlapping[0].window_end,
                write.project_id,
                write.window_start,
                write.window_end
            ))),
            Some(declared) => {
                let replaced = overlapping
                    .iter()
                    .find(|snapshot| &snapshot.source_revision == declared)
                    .cloned()
                    .ok_or_else(|| {
                        conflict(format!(
                            "source `{}` declared `replaces_source_revision: {declared}` but the \
                             overlapping window `{}`..`{}` belongs to revision `{}`; Forge \
                             supersedes only the revision the source named",
                            write.source,
                            overlapping[0].window_start,
                            overlapping[0].window_end,
                            overlapping[0].source_revision
                        ))
                    })?;
                Ok(Some(replaced))
            }
        }
    }

    /// Read one snapshot by the identity its source attested to.
    pub fn interest_snapshot_by_identity(
        &self,
        write: &InterestWrite,
    ) -> Result<Option<InterestSnapshot>, ForgeError> {
        let row = self
            .registry_conn()
            .query_row(
                "SELECT id, project_id, source, source_revision, window_start, window_end,
                        privacy_mode, coverage, state, replaces_source_revision, actor, received_at
                 FROM portfolio_interest_snapshots
                 WHERE project_id = ?1 AND source = ?2 AND source_revision = ?3
                   AND window_start = ?4 AND window_end = ?5",
                params![
                    write.project_id,
                    write.source,
                    write.source_revision,
                    write.window_start,
                    write.window_end
                ],
                row_to_interest_snapshot,
            )
            .optional()?;
        match row {
            Some(snapshot) => Ok(Some(self.interest_hydrate(snapshot)?)),
            None => Ok(None),
        }
    }

    /// Read one snapshot by row id, with its metric values.
    pub fn interest_snapshot(&self, id: i64) -> Result<Option<InterestSnapshot>, ForgeError> {
        let row = self
            .registry_conn()
            .query_row(
                "SELECT id, project_id, source, source_revision, window_start, window_end,
                        privacy_mode, coverage, state, replaces_source_revision, actor, received_at
                 FROM portfolio_interest_snapshots WHERE id = ?1",
                params![id],
                row_to_interest_snapshot,
            )
            .optional()?;
        Ok(match row {
            Some(snapshot) => {
                let metrics = self.interest_metrics(snapshot.id)?;
                Some(InterestSnapshot {
                    metrics,
                    ..snapshot
                })
            }
            None => None,
        })
    }

    /// Every snapshot for one project, oldest window first, including
    /// superseded ones — the history is the point.
    pub fn interest_snapshots(
        &self,
        project_id: &str,
        limit: usize,
    ) -> Result<Vec<InterestSnapshot>, ForgeError> {
        self.interest_collect(
            "SELECT id, project_id, source, source_revision, window_start, window_end,
                    privacy_mode, coverage, state, replaces_source_revision, actor, received_at
             FROM portfolio_interest_snapshots WHERE project_id = ?1
             ORDER BY window_start, window_end, id LIMIT ?2",
            params![project_id, limit as i64],
        )
    }

    /// Only the current snapshots for one project. A superseded
    /// revision is history, and a comparison must not present it
    /// beside the revision that replaced it.
    pub fn interest_current_snapshots(
        &self,
        project_id: &str,
    ) -> Result<Vec<InterestSnapshot>, ForgeError> {
        self.interest_collect(
            "SELECT id, project_id, source, source_revision, window_start, window_end,
                    privacy_mode, coverage, state, replaces_source_revision, actor, received_at
             FROM portfolio_interest_snapshots
             WHERE project_id = ?1 AND state = 'accepted'
             ORDER BY window_start, window_end, id",
            params![project_id],
        )
    }

    /// The current snapshots of one project for one source, used by
    /// the overlap rule.
    fn interest_snapshots_for_source(
        &self,
        project_id: &str,
        source: &str,
    ) -> Result<Vec<InterestSnapshot>, ForgeError> {
        self.interest_collect(
            "SELECT id, project_id, source, source_revision, window_start, window_end,
                    privacy_mode, coverage, state, replaces_source_revision, actor, received_at
             FROM portfolio_interest_snapshots
             WHERE project_id = ?1 AND source = ?2
             ORDER BY window_start, window_end, id",
            params![project_id, source],
        )
    }

    /// The current snapshots of several projects, optionally narrowed
    /// to one source, for a fleet-wide comparison.
    ///
    /// `?1` is always the optional source filter and the project list
    /// follows as `?2..?N`, so no parameter can shift position and the
    /// result is already in the canonical read order. The filter is
    /// `(?1 IS NULL OR source = ?1)` rather than `source IS ?1`:
    /// `IS` compares its operands literally, so `source IS NULL` would
    /// select only the rows that have no source at all.
    pub fn interest_current_snapshots_for_projects(
        &self,
        project_ids: &[String],
        source: Option<&str>,
    ) -> Result<Vec<InterestSnapshot>, ForgeError> {
        if project_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders: Vec<String> = (0..project_ids.len())
            .map(|index| format!("?{}", index + 2))
            .collect();
        let sql = format!(
            "SELECT id, project_id, source, source_revision, window_start, window_end,
                    privacy_mode, coverage, state, replaces_source_revision, actor, received_at
             FROM portfolio_interest_snapshots
             WHERE state = 'accepted' AND (?1 IS NULL OR source = ?1) AND project_id IN ({})
             ORDER BY project_id, window_start, window_end, id",
            placeholders.join(", ")
        );
        let mut statement = self.registry_conn().prepare(&sql)?;
        let mut bound: Vec<&dyn rusqlite::ToSql> = vec![&source];
        for project_id in project_ids {
            bound.push(project_id);
        }
        let rows =
            statement.query_map(rusqlite::params_from_iter(bound), row_to_interest_snapshot)?;
        let mut snapshots = Vec::new();
        for row in rows {
            snapshots.push(self.interest_hydrate(row?)?);
        }
        Ok(snapshots)
    }

    /// How many snapshots one project holds: `(total, current)`.
    ///
    /// Two `COUNT(*)` queries so `no-evidence` (nothing stored) is
    /// distinguishable from `superseded-only` (history but no live
    /// evidence). Read-only: no row is written.
    pub fn interest_snapshot_counts(&self, project_id: &str) -> Result<(usize, usize), ForgeError> {
        let total: i64 = self.registry_conn().query_row(
            "SELECT COUNT(*) FROM portfolio_interest_snapshots WHERE project_id = ?1",
            params![project_id],
            |row| row.get(0),
        )?;
        let current: i64 = self.registry_conn().query_row(
            "SELECT COUNT(*) FROM portfolio_interest_snapshots
              WHERE project_id = ?1 AND state = 'accepted'",
            params![project_id],
            |row| row.get(0),
        )?;
        Ok((total.max(0) as usize, current.max(0) as usize))
    }

    /// The stored metric values of one snapshot, in canonical metric
    /// order.
    pub fn interest_metrics(&self, snapshot_id: i64) -> Result<Vec<MetricValue>, ForgeError> {
        let mut statement = self.registry_conn().prepare(
            "SELECT metric, value FROM portfolio_interest_metrics
             WHERE snapshot_id = ?1 ORDER BY metric",
        )?;
        let rows = statement.query_map(params![snapshot_id], |row| {
            Ok(MetricValue {
                metric: row.get(0)?,
                // A stored value can never be negative: the gate
                // refuses one before it is written. The fallback is
                // therefore unreachable, and it reads as zero rather
                // than inventing a negative count.
                value: row.get::<_, i64>(1)?.max(0) as u64,
            })
        })?;
        let mut metrics = Vec::new();
        for row in rows {
            metrics.push(row?);
        }
        metrics.sort_by_key(|value| {
            InterestMetric::parse(&value.metric)
                .ok()
                .map(|metric| metric as usize)
                .unwrap_or(usize::MAX)
        });
        Ok(metrics)
    }

    // --- findings --------------------------------------------------------

    /// Persist one refusal. The detail is redacted again here so a
    /// caller that built it from an untrusted import cannot smuggle a
    /// credential into the audit trail.
    pub fn record_interest_finding(
        &self,
        project_id: &str,
        field: &str,
        code: &str,
        detail: &str,
    ) -> Result<i64, ForgeError> {
        let finding = InterestFinding::new(project_id, field, code, detail);
        let now = Utc::now().to_rfc3339();
        self.registry_conn().execute(
            "INSERT INTO portfolio_interest_findings
                (project_id, field, code, detail, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                finding.project_id,
                finding.field,
                finding.code,
                finding.detail,
                now
            ],
        )?;
        Ok(self.registry_conn().last_insert_rowid())
    }

    /// Read the persisted refusals, newest first.
    pub fn interest_findings(&self, limit: usize) -> Result<Vec<InterestFinding>, ForgeError> {
        let mut statement = self.registry_conn().prepare(
            "SELECT project_id, field, code, detail, created_at
             FROM portfolio_interest_findings ORDER BY id DESC LIMIT ?1",
        )?;
        let rows = statement.query_map(params![limit as i64], |row| {
            Ok(InterestFinding {
                project_id: row.get(0)?,
                field: row.get(1)?,
                code: row.get(2)?,
                detail: row.get(3)?,
                created_at: row.get(4)?,
            })
        })?;
        let mut findings = Vec::new();
        for row in rows {
            findings.push(row?);
        }
        Ok(findings)
    }

    // --- internals -------------------------------------------------------

    /// Open one immediate transaction on the shared connection.
    /// `&self` methods that need several statements use this rather
    /// than `transaction()`, which would need `&mut self`.
    pub(crate) fn interest_transaction(&self) -> Result<rusqlite::Transaction<'_>, ForgeError> {
        Ok(self.registry_conn().unchecked_transaction()?)
    }

    /// Run one snapshot query and hydrate every row's metrics.
    fn interest_collect<P: rusqlite::Params>(
        &self,
        sql: &str,
        params: P,
    ) -> Result<Vec<InterestSnapshot>, ForgeError> {
        let mut statement = self.registry_conn().prepare(sql)?;
        let rows = statement.query_map(params, row_to_interest_snapshot)?;
        let mut snapshots = Vec::new();
        for row in rows {
            snapshots.push(self.interest_hydrate(row?)?);
        }
        Ok(snapshots)
    }

    fn interest_hydrate(&self, snapshot: InterestSnapshot) -> Result<InterestSnapshot, ForgeError> {
        let metrics = self.interest_metrics(snapshot.id)?;
        Ok(InterestSnapshot {
            metrics,
            ..snapshot
        })
    }
}

#[cfg(test)]
pub(crate) mod fixtures {
    use rusqlite::params;

    use crate::portfolio::interest::{
        Coverage, InterestMetric, InterestWrite, MetricValue, PrivacyMode,
    };

    /// A fixed emission time keeps assertions about stored provenance
    /// rather than about the wall clock.
    pub const STAMP: &str = "2026-09-29T00:00:00+00:00";

    /// The `projects`/`operations` shape a registry carried before the
    /// portfolio packages existed. It is written by hand on purpose:
    /// the migration must carry it forward without altering it.
    pub const LEGACY_SCHEMA_SQL: &str = "
        CREATE TABLE projects (
            id TEXT PRIMARY KEY, name TEXT NOT NULL, path TEXT NOT NULL UNIQUE,
            git_remote TEXT, mirror_remotes TEXT NOT NULL DEFAULT '[]', stack TEXT,
            profile TEXT NOT NULL, maturity TEXT, target_maturity TEXT,
            schema_version INTEGER NOT NULL, platform_version TEXT NOT NULL,
            features TEXT NOT NULL DEFAULT '{}', deployment_target TEXT, runtime TEXT,
            last_commit TEXT, quality_status TEXT, agent_status TEXT, docs_status TEXT,
            observed_at TEXT NOT NULL);
        CREATE TABLE operations (
            op_id INTEGER PRIMARY KEY AUTOINCREMENT, kind TEXT NOT NULL,
            project_id TEXT NOT NULL, state TEXT NOT NULL DEFAULT 'pending',
            started_at TEXT NOT NULL, finished_at TEXT, detail TEXT,
            idempotency_key TEXT, request_hash TEXT, queue_id TEXT,
            revision TEXT, build_status TEXT, run_status TEXT,
            container_identity TEXT);
        INSERT INTO projects (id, name, path, profile, schema_version,
            platform_version, observed_at)
        VALUES ('alpha','alpha','/tmp/alpha','rust-web',1,'0.1.0','2026-09-29T00:00:00Z');
        INSERT INTO operations (kind, project_id, state, started_at)
        VALUES ('register','alpha','done','2026-09-29T00:00:00Z');
    ";

    pub fn tmp() -> tempfile::TempDir {
        tempfile::tempdir().expect("tempdir")
    }

    pub fn registry_with_projects(
        path: &std::path::Path,
        ids: &[&str],
    ) -> crate::registry::Registry {
        let registry = crate::registry::Registry::open(path).expect("open registry");
        {
            let conn = rusqlite::Connection::open(path).expect("raw connection");
            for id in ids {
                conn.execute(
                    "INSERT INTO projects
                        (id, name, path, profile, maturity, schema_version,
                         platform_version, features, observed_at)
                     VALUES (?1, ?1, ?2, 'rust-web', 'L2', 1, '0.1.0', '{}', ?3)",
                    params![id, format!("/tmp/{id}"), "2026-09-29T00:00:00Z"],
                )
                .expect("seed project");
            }
        }
        registry
    }

    pub fn write(project: &str) -> InterestWrite {
        InterestWrite {
            project_id: project.to_string(),
            source: "github-analytics".to_string(),
            source_revision: "a1b2c3".to_string(),
            window_start: "2026-09-01T00:00:00Z".to_string(),
            window_end: "2026-09-08T00:00:00Z".to_string(),
            privacy_mode: PrivacyMode::ExactCount,
            coverage: Coverage::Complete,
            replaces_source_revision: None,
            metrics: vec![MetricValue {
                metric: InterestMetric::UniqueVisitors.label().to_string(),
                value: 120,
            }],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::fixtures::{registry_with_projects, tmp, write, LEGACY_SCHEMA_SQL, STAMP};
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn the_interest_migration_is_additive_and_idempotent() {
        let dir = tmp();
        let path = dir.path().join("registry.db");
        let first = registry_with_projects(&path, &["alpha"]);
        first
            .interest_insert_snapshot(&write("alpha"), "ops-admin", STAMP)
            .expect("write");
        drop(first);
        let second = crate::registry::Registry::open(&path).expect("reopen");
        assert_eq!(
            second.interest_snapshots("alpha", 10).expect("rows").len(),
            1
        );
        drop(second);
        let third = crate::registry::Registry::open(&path).expect("reopen again");
        assert_eq!(
            third.interest_snapshots("alpha", 10).expect("rows").len(),
            1
        );
    }

    #[test]
    fn a_pre_change_registry_migrates_forward_with_its_rows_intact() {
        let dir = tmp();
        let path = dir.path().join("registry.db");
        {
            let conn = Connection::open(&path).expect("open");
            conn.execute_batch(LEGACY_SCHEMA_SQL)
                .expect("legacy schema");
        }
        let registry = crate::registry::Registry::open(&path).expect("migrate forward");
        assert_eq!(registry.list().expect("list").len(), 1);
        assert_eq!(
            registry.recent_operations(10).expect("journal").len(),
            1,
            "the pre-change journal row survives untouched"
        );
        let outcome = registry
            .interest_insert_snapshot(&write("alpha"), "ops-admin", STAMP)
            .expect("write after migration");
        assert!(matches!(outcome, SnapshotOutcome::Accepted { .. }));
    }

    #[test]
    fn an_interrupted_interest_migration_rolls_back_and_leaves_the_registry_usable() {
        let dir = tmp();
        let path = dir.path().join("registry.db");
        {
            let conn = Connection::open(&path).expect("raw");
            conn.execute_batch(LEGACY_SCHEMA_SQL)
                .expect("legacy schema");
            // A decoy whose column set makes the interest batch's own
            // `CREATE TABLE IF NOT EXISTS` a silent no-op, so the later
            // index on the real column fails mid-batch.
            conn.execute_batch(
                "CREATE TABLE portfolio_interest_metrics (snapshot_id INTEGER, decoy TEXT);",
            )
            .expect("decoy");
        }
        let err = match crate::registry::Registry::open(&path) {
            Ok(_) => panic!("the decoy must fail the portfolio interest batch"),
            Err(err) => err,
        };
        assert_eq!(err.code(), "registry-error");
        assert!(
            err.to_string()
                .contains("portfolio interest migration failed"),
            "{err}"
        );
        assert!(err.to_string().contains("registry left unchanged"), "{err}");
        {
            let conn = Connection::open(&path).expect("raw");
            for table in [
                "portfolio_interest_snapshots",
                "portfolio_interest_findings",
            ] {
                let present: i64 = conn
                    .query_row(
                        &format!(
                            "SELECT COUNT(*) FROM sqlite_master
                             WHERE type = 'table' AND name = '{table}'"
                        ),
                        [],
                        |row| row.get(0),
                    )
                    .expect("sqlite_master");
                assert_eq!(present, 0, "{table} must not survive a failed batch");
            }
            // The share batch, which ran before the interest batch,
            // committed exactly as it would have on its own.
            let share: i64 = conn
                .query_row("SELECT COUNT(*) FROM portfolio_share_records", [], |row| {
                    row.get(0)
                })
                .expect("portfolio_share_records");
            assert_eq!(share, 0);
            let journal: i64 = conn
                .query_row("SELECT COUNT(*) FROM operations", [], |row| row.get(0))
                .expect("operations");
            assert_eq!(journal, 1);
            conn.execute_batch("DROP TABLE portfolio_interest_metrics")
                .expect("drop decoy");
        }
        let registry = crate::registry::Registry::open(&path).expect("retry succeeds");
        registry
            .interest_insert_snapshot(&write("alpha"), "ops-admin", STAMP)
            .expect("write after the retry");
    }

    #[test]
    fn an_unknown_project_changes_no_interest_state() {
        let dir = tmp();
        let registry = registry_with_projects(&dir.path().join("registry.db"), &["alpha"]);
        let err = registry
            .interest_insert_snapshot(&write("nope"), "ops-admin", STAMP)
            .expect_err("unknown project");
        assert_eq!(err.code(), "unknown-project");
        assert!(registry.interest_findings(10).expect("findings").is_empty());
    }

    #[test]
    fn an_exact_repeat_is_idempotent_and_a_changed_payload_is_a_conflict() {
        let dir = tmp();
        let registry = registry_with_projects(&dir.path().join("registry.db"), &["alpha"]);
        let first = registry
            .interest_insert_snapshot(&write("alpha"), "ops-admin", STAMP)
            .expect("write");
        let first_id = first.snapshot().id;

        let repeat = registry
            .interest_insert_snapshot(&write("alpha"), "ops-admin", STAMP)
            .expect("retry");
        assert!(repeat.is_idempotent_repeat());
        assert_eq!(repeat.snapshot().id, first_id, "no second row");
        assert_eq!(
            registry
                .interest_snapshots("alpha", 10)
                .expect("rows")
                .len(),
            1
        );

        let mut contradicted = write("alpha");
        contradicted.metrics[0].value = 999;
        let err = registry
            .interest_insert_snapshot(&contradicted, "ops-admin", STAMP)
            .expect_err("contradiction");
        assert_eq!(err.code(), "portfolio-interest-conflict");
        assert!(err.to_string().contains("immutable"), "{err}");
        assert_eq!(
            registry
                .interest_snapshots("alpha", 10)
                .expect("rows")
                .len(),
            1,
            "the stored snapshot is untouched"
        );
    }

    #[test]
    fn an_overlapping_window_is_refused_unless_the_source_declares_a_replacement() {
        let dir = tmp();
        let registry = registry_with_projects(&dir.path().join("registry.db"), &["alpha"]);
        registry
            .interest_insert_snapshot(&write("alpha"), "ops-admin", STAMP)
            .expect("first window");

        let mut overlapping = write("alpha");
        overlapping.source_revision = "b2c3d4".to_string();
        overlapping.window_start = "2026-09-07T00:00:00Z".to_string();
        overlapping.window_end = "2026-09-14T00:00:00Z".to_string();
        let err = registry
            .interest_insert_snapshot(&overlapping, "ops-admin", STAMP)
            .expect_err("overlap");
        assert_eq!(err.code(), "portfolio-interest-conflict");
        assert!(
            err.to_string().contains("count the shared days twice"),
            "{err}"
        );
        assert_eq!(
            registry
                .interest_snapshots("alpha", 10)
                .expect("rows")
                .len(),
            1,
            "a refused overlap writes nothing"
        );

        // Adjacent windows are not overlapping: a weekly series is the
        // normal case, not a collision.
        let mut adjacent = write("alpha");
        adjacent.source_revision = "c3d4e5".to_string();
        adjacent.window_start = "2026-09-08T00:00:00Z".to_string();
        adjacent.window_end = "2026-09-15T00:00:00Z".to_string();
        assert!(matches!(
            registry
                .interest_insert_snapshot(&adjacent, "ops-admin", STAMP)
                .expect("adjacent"),
            SnapshotOutcome::Accepted { .. }
        ));
    }

    #[test]
    fn a_declared_replacement_supersedes_the_named_revision_and_keeps_the_history() {
        let dir = tmp();
        let registry = registry_with_projects(&dir.path().join("registry.db"), &["alpha"]);
        registry
            .interest_insert_snapshot(&write("alpha"), "ops-admin", STAMP)
            .expect("first window");

        // A declaration that names a revision which is not the
        // overlapping one is refused rather than silently demoting it.
        let mut wrong_target = write("alpha");
        wrong_target.source_revision = "d4e5f6".to_string();
        wrong_target.replaces_source_revision = Some("not-a-revision".to_string());
        let err = registry
            .interest_insert_snapshot(&wrong_target, "ops-admin", STAMP)
            .expect_err("wrong target");
        assert_eq!(err.code(), "portfolio-interest-conflict");
        assert!(
            err.to_string().contains("supersedes only the revision"),
            "{err}"
        );

        let mut replacement = write("alpha");
        replacement.source_revision = "d4e5f6".to_string();
        replacement.replaces_source_revision = Some("a1b2c3".to_string());
        replacement.metrics[0].value = 150;
        let outcome = registry
            .interest_insert_snapshot(&replacement, "ops-admin", STAMP)
            .expect("replacement");
        match outcome {
            SnapshotOutcome::AcceptedWithSupersession {
                snapshot,
                superseded,
            } => {
                assert_eq!(snapshot.source_revision, "d4e5f6");
                assert_eq!(superseded.source_revision, "a1b2c3");
                assert_eq!(superseded.state, SnapshotState::Superseded);
            }
            other => panic!("expected a supersession, got {other:?}"),
        }
        // History is retained; only the comparison view narrows.
        assert_eq!(
            registry
                .interest_snapshots("alpha", 10)
                .expect("rows")
                .len(),
            2
        );
        let current = registry
            .interest_current_snapshots("alpha")
            .expect("current");
        assert_eq!(current.len(), 1);
        assert_eq!(current[0].source_revision, "d4e5f6");
    }

    #[test]
    fn a_different_source_may_report_the_same_window() {
        let dir = tmp();
        let registry = registry_with_projects(&dir.path().join("registry.db"), &["alpha"]);
        registry
            .interest_insert_snapshot(&write("alpha"), "ops-admin", STAMP)
            .expect("github window");
        let mut other_source = write("alpha");
        other_source.source = "content-analytics".to_string();
        other_source.source_revision = "rev-9".to_string();
        assert!(matches!(
            registry
                .interest_insert_snapshot(&other_source, "ops-admin", STAMP)
                .expect("second source"),
            SnapshotOutcome::Accepted { .. }
        ));
        assert_eq!(
            registry
                .interest_current_snapshots_for_projects(&["alpha".to_string()], None)
                .expect("fleet")
                .len(),
            2
        );
        assert_eq!(
            registry
                .interest_current_snapshots_for_projects(
                    &["alpha".to_string()],
                    Some("content-analytics")
                )
                .expect("narrowed")
                .len(),
            1
        );
    }

    #[test]
    fn a_refused_import_persists_a_finding_without_the_value() {
        let dir = tmp();
        let registry = registry_with_projects(&dir.path().join("registry.db"), &["alpha"]);
        registry
            .record_interest_finding(
                "alpha",
                "metrics.email",
                "interest-identity-refused",
                "metrics `email` is refused because it carries a@b.co",
            )
            .expect("finding");
        let findings = registry.interest_findings(10).expect("findings");
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].code, "interest-identity-refused");
        assert!(
            !findings[0].detail.contains("a@b.co"),
            "the value must not be echoed: {:?}",
            findings[0]
        );
    }

    #[test]
    fn snapshot_counts_distinguish_no_evidence_from_superseded_only() {
        let dir = tmp();
        let registry = registry_with_projects(&dir.path().join("registry.db"), &["alpha"]);
        assert_eq!(
            registry.interest_snapshot_counts("alpha").expect("counts"),
            (0, 0)
        );
        registry
            .interest_insert_snapshot(&write("alpha"), "ops-admin", STAMP)
            .expect("write");
        assert_eq!(
            registry.interest_snapshot_counts("alpha").expect("counts"),
            (1, 1)
        );
        let mut replacement = write("alpha");
        replacement.source_revision = "b2c3d4".to_string();
        replacement.replaces_source_revision = Some("a1b2c3".to_string());
        registry
            .interest_insert_snapshot(&replacement, "ops-admin", STAMP)
            .expect("replacement");
        assert_eq!(
            registry.interest_snapshot_counts("alpha").expect("counts"),
            (2, 1)
        );
    }

    #[test]
    fn metrics_are_stored_as_rows_in_canonical_order() {
        let dir = tmp();
        let registry = registry_with_projects(&dir.path().join("registry.db"), &["alpha"]);
        let mut candidate = write("alpha");
        candidate.metrics = vec![
            MetricValue {
                metric: InterestMetric::PaidInterestEvents.label().to_string(),
                value: 2,
            },
            MetricValue {
                metric: InterestMetric::UniqueVisitors.label().to_string(),
                value: 120,
            },
        ];
        let outcome = registry
            .interest_insert_snapshot(&candidate, "ops-admin", STAMP)
            .expect("write");
        let stored = outcome.snapshot().clone();
        assert_eq!(
            stored
                .metrics
                .iter()
                .map(|m| m.metric.as_str())
                .collect::<Vec<_>>(),
            vec!["unique_visitors", "paid_interest_events"]
        );
        assert_eq!(stored.metric(InterestMetric::UniqueVisitors), Some(120));
        assert_eq!(stored.metric(InterestMetric::PaidInterestEvents), Some(2));
        assert_eq!(stored.actor, "ops-admin");
        assert_eq!(stored.state, SnapshotState::Accepted);
    }
}
