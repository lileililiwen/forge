//! Portfolio share persistence: the allowlist, its approvals and its
//! publication audit.
//!
//! These tables share the registry's SQLite file and its migration
//! lifecycle with the rest of the portfolio domain but never a row
//! with it. Three boundaries are deliberate:
//!
//! - **Nothing is public until it is approved and published.** A
//!   stored [`ShareRecord`] is a draft of what *may* be shown; the
//!   manifest is built from it, hashed, approved by exact hash and
//!   only then handed to a publisher.
//! - **Approvals and attempts are append-only.** An approval pins
//!   the canonical bytes and their hash; a publication attempt pins
//!   the operation key, the revision, the hash and the outcome. A
//!   later attempt never rewrites an earlier row, so the audit trail
//!   can always answer what was public, when, by whom and from which
//!   revision.
//! - **Findings are evidence, not state.** A refused write persists
//!   the field and the rule that refused it. The offending value never
//!   reaches a row.
//!
//! The file is split by concern: this module owns the schema, the
//! share records and their allowlisted surfaces; [`audit`] owns the
//! findings, approvals and publication attempts.

pub mod audit;

use chrono::Utc;
use rusqlite::{params, OptionalExtension};

use crate::core::ForgeError;
use crate::portfolio::share::{
    self, ShareFinding, ShareRecord, ShareState, ShareSurface, ShareWrite,
};

/// Additive share schema. Every statement is `CREATE ... IF NOT
/// EXISTS`, so a registry written before the share package migrates
/// forward in place and re-opening is a no-op. SQLite does not
/// enforce foreign keys here, so the methods on this type resolve the
/// project against the canonical registry identity themselves and
/// return a typed `unknown-project` rather than relying on the
/// pragma.
pub const PORTFOLIO_SHARE_SCHEMA_SQL: &str = "
CREATE TABLE IF NOT EXISTS portfolio_share_records (
    project_id      TEXT PRIMARY KEY,
    title           TEXT NOT NULL,
    summary         TEXT NOT NULL,
    category        TEXT NOT NULL,
    source_url      TEXT NOT NULL,
    demo_url        TEXT,
    visibility      TEXT NOT NULL,
    featured        INTEGER NOT NULL DEFAULT 0,
    showcase_status TEXT NOT NULL,
    status_evidence TEXT,
    state           TEXT NOT NULL,
    revision        INTEGER NOT NULL DEFAULT 1,
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS portfolio_share_surfaces (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id TEXT NOT NULL,
    label      TEXT NOT NULL,
    url        TEXT NOT NULL,
    created_at TEXT NOT NULL,
    UNIQUE (project_id, label, url)
);
CREATE TABLE IF NOT EXISTS portfolio_share_findings (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id TEXT NOT NULL,
    field      TEXT NOT NULL,
    code       TEXT NOT NULL,
    detail     TEXT NOT NULL,
    created_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS portfolio_share_approvals (
    revision        INTEGER PRIMARY KEY AUTOINCREMENT,
    manifest_sha256 TEXT NOT NULL,
    manifest_json   TEXT NOT NULL,
    project_count   INTEGER NOT NULL,
    actor           TEXT NOT NULL,
    approved_at     TEXT NOT NULL,
    state           TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS portfolio_share_publications (
    id                INTEGER PRIMARY KEY AUTOINCREMENT,
    operation_key     TEXT NOT NULL UNIQUE,
    manifest_revision INTEGER NOT NULL,
    manifest_sha256   TEXT NOT NULL,
    target            TEXT NOT NULL,
    status            TEXT NOT NULL,
    published_revision TEXT,
    error_code        TEXT,
    actor             TEXT NOT NULL,
    attempted_at      TEXT NOT NULL,
    finished_at       TEXT,
    document_generated_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS portfolio_share_surfaces_project_idx
    ON portfolio_share_surfaces (project_id, label);
CREATE INDEX IF NOT EXISTS portfolio_share_findings_project_idx
    ON portfolio_share_findings (project_id, id);
CREATE INDEX IF NOT EXISTS portfolio_share_publications_status_idx
    ON portfolio_share_publications (status, id);
";

pub(crate) fn invalid(reason: String) -> ForgeError {
    ForgeError::PortfolioShareInvalid { reason }
}

pub(crate) fn conflict(reason: String) -> ForgeError {
    ForgeError::PortfolioShareConflict { reason }
}

impl crate::registry::Registry {
    // --- share records ---------------------------------------------------

    /// Create or update one share record.
    ///
    /// Validation runs to completion before the transaction opens, so
    /// no partially-validated write exists. An accepted write replaces
    /// the whole record *and* its surface allowlist atomically, and
    /// returns the record to `validated`: an edit invalidates any
    /// approval that covered the previous content.
    pub fn share_upsert_record(
        &self,
        project_id: &str,
        write: &ShareWrite,
    ) -> Result<ShareRecord, ForgeError> {
        self.require_project(project_id)?;
        // A refused write is still evidence: the finding is persisted
        // before the typed refusal is returned, so the audit trail can
        // answer why a record never became public.
        let validated = match share::validate_share(write) {
            Ok(validated) => validated,
            Err(reason) => {
                self.record_share_finding(project_id, "record", "share-write-refused", &reason)?;
                // A record whose last edit an admin could not make
                // valid stops being public. Leaving the previous, now
                // unrepresentable content on show would be worse than
                // publishing nothing, so the record moves to `rejected`
                // and the manifest build omits it.
                self.registry_conn().execute(
                    "UPDATE portfolio_share_records SET state = ?1, updated_at = ?2
                     WHERE project_id = ?3",
                    params![
                        ShareState::Rejected.label(),
                        Utc::now().to_rfc3339(),
                        project_id
                    ],
                )?;
                return Err(invalid(reason));
            }
        };
        let now = Utc::now().to_rfc3339();
        let tx = self.share_transaction()?;
        tx.execute(
            "INSERT INTO portfolio_share_records
                (project_id, title, summary, category, source_url, demo_url,
                 visibility, featured, showcase_status, status_evidence, state,
                 revision, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, 1, ?12, ?12)
             ON CONFLICT(project_id) DO UPDATE SET
                title           = excluded.title,
                summary         = excluded.summary,
                category        = excluded.category,
                source_url      = excluded.source_url,
                demo_url        = excluded.demo_url,
                visibility      = excluded.visibility,
                featured        = excluded.featured,
                showcase_status = excluded.showcase_status,
                status_evidence = excluded.status_evidence,
                state           = excluded.state,
                revision        = portfolio_share_records.revision + 1,
                updated_at      = excluded.updated_at",
            params![
                project_id,
                validated.title,
                validated.summary,
                validated.category,
                validated.source_url,
                validated.demo_url,
                validated.visibility.label(),
                i64::from(validated.featured),
                validated.showcase_status.label(),
                validated.status_evidence,
                ShareState::Validated.label(),
                now,
            ],
        )?;
        tx.execute(
            "DELETE FROM portfolio_share_surfaces WHERE project_id = ?1",
            params![project_id],
        )?;
        for surface in &validated.surfaces {
            tx.execute(
                "INSERT INTO portfolio_share_surfaces (project_id, label, url, created_at)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(project_id, label, url) DO NOTHING",
                params![project_id, surface.label, surface.url, now],
            )?;
        }
        tx.commit()?;
        self.share_record(project_id)?
            .ok_or_else(|| invalid("share record disappeared after an accepted write".to_string()))
    }

    /// Withdraw one project's share record. A project without a record
    /// is simply absent from the catalog — Forge never re-discovers it
    /// and never keeps a tombstone that could read as an entry.
    pub fn share_remove_record(&self, project_id: &str) -> Result<bool, ForgeError> {
        self.require_project(project_id)?;
        let tx = self.share_transaction()?;
        tx.execute(
            "DELETE FROM portfolio_share_surfaces WHERE project_id = ?1",
            params![project_id],
        )?;
        let removed = tx.execute(
            "DELETE FROM portfolio_share_records WHERE project_id = ?1",
            params![project_id],
        )?;
        tx.commit()?;
        Ok(removed > 0)
    }

    /// Read one share record with its allowlisted surfaces.
    pub fn share_record(&self, project_id: &str) -> Result<Option<ShareRecord>, ForgeError> {
        let row = self
            .registry_conn()
            .query_row(
                "SELECT project_id, title, summary, category, source_url, demo_url,
                        visibility, featured, showcase_status, status_evidence, state,
                        revision, created_at, updated_at
                 FROM portfolio_share_records WHERE project_id = ?1",
                params![project_id],
                row_to_share_record,
            )
            .optional()?;
        match row {
            Some(record) => {
                let surfaces = self.share_surfaces(project_id)?;
                Ok(Some(ShareRecord { surfaces, ..record }))
            }
            None => Ok(None),
        }
    }

    /// Read every share record, sorted by the stable project id so the
    /// manifest build is order-independent.
    pub fn share_records(&self) -> Result<Vec<ShareRecord>, ForgeError> {
        let mut stmt = self.registry_conn().prepare(
            "SELECT project_id, title, summary, category, source_url, demo_url,
                    visibility, featured, showcase_status, status_evidence, state,
                    revision, created_at, updated_at
             FROM portfolio_share_records
             ORDER BY project_id",
        )?;
        let rows = stmt.query_map([], row_to_share_record)?;
        let mut records = Vec::new();
        for row in rows {
            let record = row?;
            let surfaces = self.share_surfaces(&record.project_id)?;
            records.push(ShareRecord { surfaces, ..record });
        }
        Ok(records)
    }

    /// The allowlisted public surfaces of one record, in canonical
    /// `(label, url)` order.
    pub fn share_surfaces(&self, project_id: &str) -> Result<Vec<ShareSurface>, ForgeError> {
        let mut stmt = self.registry_conn().prepare(
            "SELECT label, url FROM portfolio_share_surfaces
             WHERE project_id = ?1 ORDER BY label, url",
        )?;
        let rows = stmt.query_map(params![project_id], |row| {
            Ok(ShareSurface {
                label: row.get(0)?,
                url: row.get(1)?,
            })
        })?;
        let mut surfaces = Vec::new();
        for row in rows {
            surfaces.push(row?);
        }
        Ok(surfaces)
    }

    // --- findings --------------------------------------------------------

    /// Persist one refusal. The detail is redacted again here so a
    /// caller that built it from user input cannot smuggle a
    /// credential into the audit trail.
    pub fn record_share_finding(
        &self,
        project_id: &str,
        field: &str,
        code: &str,
        detail: &str,
    ) -> Result<i64, ForgeError> {
        let finding = ShareFinding::new(project_id, field, code, detail);
        let now = Utc::now().to_rfc3339();
        self.registry_conn().execute(
            "INSERT INTO portfolio_share_findings
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
    pub fn share_findings(&self, limit: usize) -> Result<Vec<ShareFinding>, ForgeError> {
        let mut stmt = self.registry_conn().prepare(
            "SELECT project_id, field, code, detail, created_at
             FROM portfolio_share_findings ORDER BY id DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit as i64], |row| {
            Ok(ShareFinding {
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
    pub(crate) fn share_transaction(&self) -> Result<rusqlite::Transaction<'_>, ForgeError> {
        Ok(self.registry_conn().unchecked_transaction()?)
    }
}

/// Decode one `portfolio_share_records` row. The column order is fixed
/// and shared by every SELECT so the projection stays one mapper.
pub(crate) fn row_to_share_record(row: &rusqlite::Row<'_>) -> rusqlite::Result<ShareRecord> {
    let state: String = row.get(10)?;
    Ok(ShareRecord {
        project_id: row.get(0)?,
        title: row.get(1)?,
        summary: row.get(2)?,
        category: row.get(3)?,
        source_url: row.get(4)?,
        demo_url: row.get(5)?,
        visibility: row.get(6)?,
        featured: row.get::<_, i64>(7)? != 0,
        showcase_status: row.get(8)?,
        status_evidence: row.get(9)?,
        // An unreadable state is treated as the one that never
        // publishes, rather than guessing a healthy classification.
        state: ShareState::parse(&state).unwrap_or(ShareState::Rejected),
        revision: row.get(11)?,
        created_at: row.get(12)?,
        updated_at: row.get(13)?,
        surfaces: Vec::new(),
    })
}

#[cfg(test)]
pub(crate) mod fixtures {
    use rusqlite::params;

    use crate::portfolio::share::{ShareSurface, ShareWrite, ShowcaseStatus, Visibility};

    /// A fixed emission time keeps reservation assertions about the
    /// manifest hash rather than about the wall clock.
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

    pub fn write(project: &str) -> ShareWrite {
        ShareWrite {
            title: format!("Project {project}"),
            summary: "A public showcase summary.".to_string(),
            category: "platform".to_string(),
            source_url: format!("https://example.com/{project}"),
            demo_url: None,
            visibility: Visibility::Public,
            featured: false,
            showcase_status: ShowcaseStatus::Demo,
            status_evidence: None,
            surfaces: vec![ShareSurface {
                label: "Docs".to_string(),
                url: format!("https://example.com/{project}/docs"),
            }],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::fixtures::{registry_with_projects, tmp, write, LEGACY_SCHEMA_SQL};
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn the_share_migration_is_additive_and_idempotent() {
        let dir = tmp();
        let path = dir.path().join("registry.db");
        let first = registry_with_projects(&path, &["alpha"]);
        first
            .share_upsert_record("alpha", &write("alpha"))
            .expect("write");
        drop(first);
        // Re-opening runs the batch again; every statement is idempotent,
        // so the record and its surface survive untouched.
        let second = crate::registry::Registry::open(&path).expect("reopen");
        assert_eq!(second.share_records().expect("records").len(), 1);
        assert_eq!(second.share_surfaces("alpha").expect("surfaces").len(), 1);
        drop(second);
        let third = crate::registry::Registry::open(&path).expect("reopen again");
        assert_eq!(third.share_records().expect("records").len(), 1);
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
        let record = registry
            .share_upsert_record("alpha", &write("alpha"))
            .expect("write after migration");
        assert_eq!(record.project_id, "alpha");
        assert_eq!(record.surfaces.len(), 1);
    }

    #[test]
    fn an_interrupted_share_migration_rolls_back_and_leaves_the_registry_usable() {
        let dir = tmp();
        let path = dir.path().join("registry.db");
        {
            let conn = Connection::open(&path).expect("raw");
            conn.execute_batch(LEGACY_SCHEMA_SQL)
                .expect("legacy schema");
            // A decoy whose column set makes the share batch's own
            // `CREATE TABLE IF NOT EXISTS` a silent no-op, so the later
            // index on the real column fails mid-batch.
            conn.execute_batch(
                "CREATE TABLE portfolio_share_surfaces (project_id TEXT, decoy TEXT);",
            )
            .expect("decoy");
        }
        let err = match crate::registry::Registry::open(&path) {
            Ok(_) => panic!("the decoy must fail the portfolio share batch"),
            Err(err) => err,
        };
        assert_eq!(err.code(), "registry-error");
        assert!(
            err.to_string().contains("portfolio share migration failed"),
            "{err}"
        );
        assert!(err.to_string().contains("registry left unchanged"), "{err}");
        {
            let conn = Connection::open(&path).expect("raw");
            // Nothing from the share batch survives: not even the tables
            // created before the failing statement.
            for table in [
                "portfolio_share_records",
                "portfolio_share_findings",
                "portfolio_share_approvals",
                "portfolio_share_publications",
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
            // The prior registry is intact and still usable.
            let rows: i64 = conn
                .query_row("SELECT COUNT(*) FROM projects", [], |row| row.get(0))
                .expect("projects");
            assert_eq!(rows, 1);
            let journal: i64 = conn
                .query_row("SELECT COUNT(*) FROM operations", [], |row| row.get(0))
                .expect("operations");
            assert_eq!(journal, 1);
            // The portfolio batch, which ran before the share batch,
            // committed exactly as it would have on its own.
            let portfolio: i64 = conn
                .query_row("SELECT COUNT(*) FROM portfolio_projects", [], |row| {
                    row.get(0)
                })
                .expect("portfolio_projects");
            assert_eq!(portfolio, 0);
            conn.execute_batch("DROP TABLE portfolio_share_surfaces")
                .expect("drop decoy");
        }
        let registry = crate::registry::Registry::open(&path).expect("retry succeeds");
        registry
            .share_upsert_record("alpha", &write("alpha"))
            .expect("write after the retry");
    }

    #[test]
    fn an_unknown_project_changes_no_share_state() {
        let dir = tmp();
        let registry = registry_with_projects(&dir.path().join("registry.db"), &["alpha"]);
        let err = registry
            .share_upsert_record("nope", &write("nope"))
            .expect_err("unknown project");
        assert_eq!(err.code(), "unknown-project");
        assert!(registry.share_records().expect("records").is_empty());
        assert!(registry.share_findings(10).expect("findings").is_empty());
    }

    #[test]
    fn a_refused_write_persists_a_finding_without_the_value() {
        let dir = tmp();
        let registry = registry_with_projects(&dir.path().join("registry.db"), &["alpha"]);
        let mut candidate = write("alpha");
        candidate.source_url = "https://example.com/admin/settings".to_string();
        let err = registry
            .share_upsert_record("alpha", &candidate)
            .expect_err("private surface");
        assert_eq!(err.code(), "portfolio-share-invalid");
        assert!(registry.share_records().expect("records").is_empty());
        let findings = registry.share_findings(10).expect("findings");
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].code, "share-write-refused");
        assert!(findings[0].detail.contains("private"), "{:?}", findings[0]);
        assert!(!findings[0].detail.contains("/admin"));
    }

    #[test]
    fn a_refused_edit_stops_serving_the_previous_record() {
        let dir = tmp();
        let registry = registry_with_projects(&dir.path().join("registry.db"), &["alpha"]);
        registry
            .share_upsert_record("alpha", &write("alpha"))
            .expect("first write");
        let mut broken = write("alpha");
        broken.title = "Alethefy".to_string();
        broken.source_url = "https://example.com/alpha".to_string();
        broken.summary = "token=supersecret".to_string();
        registry
            .share_upsert_record("alpha", &broken)
            .expect_err("credential");
        let record = registry.share_record("alpha").expect("read").expect("row");
        assert_eq!(record.state, ShareState::Rejected);
        let draft = registry.share_manifest_draft(1).expect("draft");
        assert_eq!(draft.project_count(), 0);
        assert!(!draft.approvable());
    }

    #[test]
    fn a_second_edit_bumps_the_record_revision() {
        let dir = tmp();
        let registry = registry_with_projects(&dir.path().join("registry.db"), &["alpha"]);
        let first = registry
            .share_upsert_record("alpha", &write("alpha"))
            .expect("first");
        let second = registry
            .share_upsert_record("alpha", &write("alpha"))
            .expect("second");
        assert_eq!(first.revision, 1);
        assert_eq!(second.revision, 2);
        assert_eq!(second.created_at, first.created_at);
    }

    #[test]
    fn surfaces_are_replaced_not_accumulated() {
        let dir = tmp();
        let registry = registry_with_projects(&dir.path().join("registry.db"), &["alpha"]);
        registry
            .share_upsert_record("alpha", &write("alpha"))
            .expect("first");
        let mut second = write("alpha");
        second.surfaces = vec![ShareSurface {
            label: "Api".to_string(),
            url: "https://example.com/alpha/api-docs".to_string(),
        }];
        let record = registry
            .share_upsert_record("alpha", &second)
            .expect("second");
        assert_eq!(record.surfaces.len(), 1);
        assert_eq!(record.surfaces[0].label, "Api");
    }

    #[test]
    fn removing_a_record_takes_the_project_out_of_the_catalog() {
        let dir = tmp();
        let registry = registry_with_projects(&dir.path().join("registry.db"), &["alpha", "beta"]);
        registry
            .share_upsert_record("alpha", &write("alpha"))
            .expect("alpha");
        registry
            .share_upsert_record("beta", &write("beta"))
            .expect("beta");
        assert!(registry.share_remove_record("alpha").expect("remove"));
        assert!(!registry.share_remove_record("alpha").expect("again"));
        let draft = registry.share_manifest_draft(1).expect("draft");
        assert_eq!(draft.project_count(), 1);
        assert_eq!(draft.body.projects[0].id, "beta");
    }

    #[test]
    fn a_project_without_a_share_record_is_never_listed() {
        let dir = tmp();
        let registry = registry_with_projects(&dir.path().join("registry.db"), &["alpha", "beta"]);
        registry
            .share_upsert_record("alpha", &write("alpha"))
            .expect("alpha");
        assert!(registry.share_record("beta").expect("read").is_none());
        let draft = registry.share_manifest_draft(1).expect("draft");
        assert_eq!(draft.body.projects.len(), 1);
        assert!(!draft.body.canonical_json().contains("beta"));
    }
}
