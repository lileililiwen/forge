//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use crate::core::manifest::Manifest;
use crate::core::ForgeError;
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::Path;
use std::time::Duration;

use super::schema::{PLATFORM_VERSION, SCHEMA_SQL};
use super::store::{apply_migrations, capture_git_info, read_record, row_to_operation_entry};

/// Outcome of [`Registry::reserve_idempotent_operation`]. `Reserved`
/// means a fresh pending operation was inserted; `Reused` means the
/// caller already reserved (or finalized) an operation with the same
/// `(kind, idempotency_key, request_hash)` triple and should look up
/// the prior record instead of running the work again.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReservationOutcome {
    Reserved { op_id: i64 },
    Reused { op_id: i64 },
}
/// One project's most recent publish operation, projected for the web
/// fleet's `published` source. Only the bounded logical fields are
/// carried; the raw `detail` is redacted by the caller before it is
/// serialized. A legacy row may have every phase column `NULL`, which is
/// surfaced as `None` rather than guessed.
#[derive(Debug, Clone)]
pub struct PublishedOperation {
    pub project_id: String,
    pub state: String,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub detail: Option<String>,
    pub queue_id: Option<String>,
    pub revision: Option<String>,
    pub build_status: Option<String>,
    pub run_status: Option<String>,
    pub container_identity: Option<String>,
}
pub struct Registry {
    pub(super) conn: Connection,
}
impl Registry {
    /// Borrow the shared connection. Domain modules that need to open
    /// their own transaction use this so an `&self` method never has
    /// to escalate to `&mut self`.
    pub(crate) fn registry_conn(&self) -> &Connection {
        &self.conn
    }
    /// Open (creating) the registry database and reconcile stale journal
    /// entries left by interrupted runs.
    pub fn open(path: &Path) -> Result<Self, ForgeError> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(|err| ForgeError::Registry {
                    reason: format!(
                        "cannot create registry directory {}: {err}",
                        parent.display()
                    ),
                })?;
            }
        }
        let conn = Connection::open(path).map_err(|err| ForgeError::Registry {
            reason: format!("cannot open registry {}: {err}", path.display()),
        })?;
        conn.busy_timeout(Duration::from_secs(5))?;
        conn.execute_batch(SCHEMA_SQL)?;
        apply_migrations(&conn)?;
        let mut registry = Registry { conn };
        registry.reconcile_journal()?;
        Ok(registry)
    }
    /// Open an **existing** registry strictly read-only: no directory is
    /// created, no schema is applied, no migration runs and no journal
    /// row is reconciled. Read-only projections (the project catalog)
    /// use this so a query can never write a registry byte, table or
    /// journal row. A missing file is a typed error the caller reports
    /// as an unavailable or empty source rather than silently creating
    /// one.
    pub fn open_read_only(path: &Path) -> Result<Self, ForgeError> {
        let conn = Connection::open_with_flags(
            path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(|err| ForgeError::Registry {
            reason: format!("cannot open registry read-only {}: {err}", path.display()),
        })?;
        conn.busy_timeout(Duration::from_secs(5))?;
        Ok(Registry { conn })
    }
    /// Mark every leftover `pending` entry as `failed`. Returns how many
    /// were reconciled.
    pub fn reconcile_journal(&mut self) -> Result<usize, ForgeError> {
        let now = Utc::now().to_rfc3339();
        let count = self.conn.execute(
            "UPDATE operations SET state = 'failed', finished_at = ?1,
             detail = 'interrupted: previous run did not complete; never reported as success'
             WHERE state = 'pending'",
            params![now],
        )?;
        Ok(count)
    }
    pub fn journal_entries(&self) -> Result<Vec<OperationEntry>, ForgeError> {
        let mut stmt = self.conn.prepare(
            "SELECT op_id, kind, project_id, state, started_at, finished_at, detail,
                    idempotency_key, request_hash, queue_id,
                    revision, build_status, run_status, container_identity
             FROM operations ORDER BY op_id",
        )?;
        let rows = stmt.query_map([], row_to_operation_entry)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }
    /// Most recent journal entries, ordered by `op_id` descending and
    /// capped at `limit`. Used by the portal dashboard so a long-running
    /// registry does not pin the renderer to an unbounded history.
    pub fn recent_operations(&self, limit: usize) -> Result<Vec<OperationEntry>, ForgeError> {
        let mut stmt = self.conn.prepare(
            "SELECT op_id, kind, project_id, state, started_at, finished_at, detail,
                    idempotency_key, request_hash, queue_id,
                    revision, build_status, run_status, container_identity
             FROM operations ORDER BY op_id DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit as i64], row_to_operation_entry)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }
    /// Most recent journal entries for a single project, ordered by
    /// `op_id` descending and capped at `limit`. The portal surface
    /// uses this so a project dashboard is not polluted by unrelated
    /// registry-wide operations.
    pub fn operations_for_project(
        &self,
        project_id: &str,
        limit: usize,
    ) -> Result<Vec<OperationEntry>, ForgeError> {
        let mut stmt = self.conn.prepare(
            "SELECT op_id, kind, project_id, state, started_at, finished_at, detail,
                    idempotency_key, request_hash, queue_id,
                    revision, build_status, run_status, container_identity
             FROM operations WHERE project_id = ?1 ORDER BY op_id DESC LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![project_id, limit as i64], row_to_operation_entry)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }
    /// Look up a single journal entry by op_id.
    pub fn operation(&self, op_id: i64) -> Result<Option<OperationEntry>, ForgeError> {
        let mut stmt = self.conn.prepare(
            "SELECT op_id, kind, project_id, state, started_at, finished_at, detail,
                    idempotency_key, request_hash, queue_id,
                    revision, build_status, run_status, container_identity
             FROM operations WHERE op_id = ?1",
        )?;
        let entry = stmt
            .query_row(params![op_id], row_to_operation_entry)
            .optional()?;
        Ok(entry)
    }
    /// Most recent journal entries for one queue, ordered by `op_id`
    /// descending and capped at `limit`. Returns an empty `Vec` when
    /// the queue has never been recorded. The `forge deploy status
    /// --queue <id>` projection uses this so a queue report is
    /// entirely self-contained.
    pub fn operations_for_queue(
        &self,
        queue_id: &str,
        limit: usize,
    ) -> Result<Vec<OperationEntry>, ForgeError> {
        let mut stmt = self.conn.prepare(
            "SELECT op_id, kind, project_id, state, started_at, finished_at, detail,
                    idempotency_key, request_hash, queue_id,
                    revision, build_status, run_status, container_identity
             FROM operations WHERE queue_id = ?1 ORDER BY op_id DESC LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![queue_id, limit as i64], row_to_operation_entry)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }
    /// The most recent publish operation per `project_id`, newest first and
    /// capped at `limit`. One bounded `GROUP BY` over the append-only journal:
    /// the web fleet's `published` source uses this to show projects the
    /// operator shipped but never locally registered. Read-only; a legacy row
    /// with `NULL` phase columns is projected with those fields absent.
    pub fn latest_publishes(&self, limit: usize) -> Result<Vec<PublishedOperation>, ForgeError> {
        let mut stmt = self.conn.prepare(
            "SELECT o.project_id, o.state, o.started_at, o.finished_at, o.detail,
                    o.queue_id, o.revision, o.build_status, o.run_status,
                    o.container_identity
             FROM operations o
             JOIN (
                 SELECT project_id, MAX(op_id) AS max_op
                 FROM operations
                 WHERE kind IN ('publish', 'publish.github')
                 GROUP BY project_id
             ) latest
               ON o.project_id = latest.project_id AND o.op_id = latest.max_op
             ORDER BY o.started_at DESC, o.project_id
             LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit as i64], |row| {
            Ok(PublishedOperation {
                project_id: row.get(0)?,
                state: row.get(1)?,
                started_at: row.get(2)?,
                finished_at: row.get(3)?,
                detail: row.get(4)?,
                queue_id: row.get(5)?,
                revision: row.get(6)?,
                build_status: row.get(7)?,
                run_status: row.get(8)?,
                container_identity: row.get(9)?,
            })
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }
    /// Append one journal entry for a non-registration operation (e.g.
    /// `upgrade` per-project outcomes with `done`/`failed`/`blocked`/
    /// `skipped` states). Journals are append-only evidence; this never
    /// mutates project rows.
    pub fn record_operation(
        &self,
        kind: &str,
        project_id: &str,
        state: &str,
        detail: &str,
    ) -> Result<(), ForgeError> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO operations (kind, project_id, state, started_at, finished_at, detail)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![kind, project_id, state, now, now, detail],
        )?;
        Ok(())
    }
    /// Reserve a `pending` operation with an idempotency key and a
    /// SHA-style request fingerprint. The pair is unique per `kind`, so
    /// a caller that retries with the same key + same fingerprint
    /// receives the original `op_id` (and a previously committed final
    /// state) while a caller that reuses the key with a different
    /// fingerprint is rejected with
    /// [`ForgeError::IdempotencyKeyConflict`].
    ///
    /// `project_id` is the owning project (or a transport-level
    /// synthetic id such as the registry-wide `__api__`). The key is
    /// stored verbatim so a duplicate request with the same key on a
    /// different transport can be detected by the caller.
    pub fn reserve_idempotent_operation(
        &self,
        kind: &str,
        project_id: &str,
        idempotency_key: &str,
        request_hash: &str,
    ) -> Result<ReservationOutcome, ForgeError> {
        let now = Utc::now().to_rfc3339();
        if let Some(existing) = self.operation_by_idempotency(kind, idempotency_key)? {
            if existing.request_hash.as_deref() != Some(request_hash) {
                return Err(ForgeError::IdempotencyKeyConflict {
                    reason: format!(
                        "idempotency key `{idempotency_key}` was previously used for a \
                         different `{kind}` request; refusing to silently re-interpret the \
                         key as a new operation"
                    ),
                });
            }
            return Ok(ReservationOutcome::Reused {
                op_id: existing.op_id,
            });
        }
        self.conn.execute(
            "INSERT INTO operations
                (kind, project_id, state, started_at, idempotency_key, request_hash)
             VALUES (?1, ?2, 'pending', ?3, ?4, ?5)",
            params![kind, project_id, now, idempotency_key, request_hash],
        )?;
        Ok(ReservationOutcome::Reserved {
            op_id: self.conn.last_insert_rowid(),
        })
    }
    /// Finalize a previously reserved operation by writing the terminal
    /// state and the human-readable detail. The transition is recorded
    /// as a single append (no row replacement) so the journal stays
    /// evidence-complete.
    pub fn finalize_operation(
        &self,
        op_id: i64,
        state: &str,
        detail: &str,
    ) -> Result<(), ForgeError> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE operations SET state = ?1, finished_at = ?2, detail = ?3
             WHERE op_id = ?4",
            params![state, now, detail, op_id],
        )?;
        Ok(())
    }
    /// Update one operation row with the additive revision /
    /// build_status / run_status / container_identity fields. Used by
    /// the GitHub push path so the same `op_id` that the idempotency
    /// layer reserved also carries the publish phase evidence.
    /// `None` arguments leave the existing column value intact so a
    /// partial update never blanks an already-recorded field.
    pub fn update_operation_phase(
        &self,
        op_id: i64,
        revision: Option<&str>,
        build_status: Option<&str>,
        run_status: Option<&str>,
        container_identity: Option<&str>,
    ) -> Result<(), ForgeError> {
        self.conn.execute(
            "UPDATE operations
             SET revision        = COALESCE(?1, revision),
                 build_status    = COALESCE(?2, build_status),
                 run_status      = COALESCE(?3, run_status),
                 container_identity = COALESCE(?4, container_identity)
             WHERE op_id = ?5",
            params![
                revision,
                build_status,
                run_status,
                container_identity,
                op_id
            ],
        )?;
        Ok(())
    }
    /// Look up an operation by `(kind, idempotency_key)`. Returns
    /// `None` when the key has never been recorded for this kind.
    pub fn operation_by_idempotency(
        &self,
        kind: &str,
        idempotency_key: &str,
    ) -> Result<Option<OperationEntry>, ForgeError> {
        let mut stmt = self.conn.prepare(
            "SELECT op_id, kind, project_id, state, started_at, finished_at, detail,
                    idempotency_key, request_hash, queue_id,
                    revision, build_status, run_status, container_identity
             FROM operations
             WHERE kind = ?1 AND idempotency_key = ?2
             ORDER BY op_id DESC
             LIMIT 1",
        )?;
        let entry = stmt
            .query_row(params![kind, idempotency_key], row_to_operation_entry)
            .optional()?;
        Ok(entry)
    }
    /// Append one journal entry for a fleet publish slot. Distinct
    /// from `record_operation` because the row is tagged with a
    /// `queue_id` so a `forge deploy status --queue <id>` query can
    /// reconstruct the full state from a single index lookup. The
    /// state transitions are queued → running → terminal; a non-
    /// pending initial write is allowed only for the
    /// `record_terminal_queue_row` helper that the queue loop calls
    /// after the active phase settles.
    pub fn record_queue_operation(
        &self,
        kind: &str,
        project_id: &str,
        queue_id: &str,
        state: &str,
        detail: Option<&str>,
    ) -> Result<i64, ForgeError> {
        let now = Utc::now().to_rfc3339();
        self.conn
            .execute(
                "INSERT INTO operations (kind, project_id, queue_id, state, started_at, finished_at, detail)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![kind, project_id, queue_id, state, now, now, detail],
            )?;
        Ok(self.conn.last_insert_rowid())
    }
    /// Append one publish journal row with the additive revision /
    /// build_status / run_status / container_identity fields. Used by
    /// `cmd_publish_provider` so `forge deploy status` can answer
    /// "what revision is running?" without contacting the provider
    /// again. All phase fields are optional so legacy providers that
    /// never carried phase evidence still get their row persisted.
    pub fn record_publish_phase(
        &self,
        project_id: &str,
        state: &str,
        phase: PublishPhaseEvidence<'_>,
        detail: Option<&str>,
    ) -> Result<i64, ForgeError> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO operations
                (kind, project_id, state, started_at, finished_at, detail,
                 revision, build_status, run_status, container_identity)
             VALUES ('publish', ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                project_id,
                state,
                now,
                now,
                detail,
                phase.revision,
                phase.build_status,
                phase.run_status,
                phase.container_identity
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }
    /// Append one publish journal row inside a fleet queue with
    /// additive revision / build_status / run_status /
    /// container_identity fields. Used by `cmd_publish_fleet` so a
    /// `forge deploy status --queue <id>` query reads every phase
    /// field from the journal.
    pub fn record_queue_publish_phase(
        &self,
        project_id: &str,
        queue_id: &str,
        state: &str,
        phase: PublishPhaseEvidence<'_>,
        detail: Option<&str>,
    ) -> Result<i64, ForgeError> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO operations
                (kind, project_id, queue_id, state, started_at, finished_at, detail,
                 revision, build_status, run_status, container_identity)
             VALUES ('publish', ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                project_id,
                queue_id,
                state,
                now,
                now,
                detail,
                phase.revision,
                phase.build_status,
                phase.run_status,
                phase.container_identity
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }
    /// Update the state of one queued operation. Used by the fleet
    /// loop to move a queued project into `running` (state=
    /// `running`, no `finished_at`) and later into a terminal state.
    /// The transition is single-row; only the `pending`/`running`
    /// initial rows from `record_queue_operation` are eligible so a
    /// stale terminal row is never overwritten.
    pub fn update_queue_operation_state(
        &self,
        op_id: i64,
        new_state: &str,
        detail: Option<&str>,
    ) -> Result<(), ForgeError> {
        let now = Utc::now().to_rfc3339();
        let rows = match new_state {
            "queued" | "running" | "pending" => self.conn.execute(
                "UPDATE operations SET state = ?1, detail = ?2
                 WHERE op_id = ?3 AND state IN ('queued', 'running', 'pending')",
                params![new_state, detail, op_id],
            )?,
            _ => self.conn.execute(
                "UPDATE operations SET state = ?1, finished_at = ?2, detail = ?3
                 WHERE op_id = ?4",
                params![new_state, now, detail, op_id],
            )?,
        };
        if rows == 0 {
            return Err(ForgeError::Registry {
                reason: format!(
                    "queue operation {op_id} not found or already in a different state"
                ),
            });
        }
        Ok(())
    }
    /// Validate the manifest in `dir` (read-only) and persist the project.
    /// Same id + same path re-registers (refreshes observations); any
    /// other id/path reuse is rejected without touching the original row.
    pub fn register(
        &mut self,
        dir: &Path,
        explicit: Option<&Path>,
    ) -> Result<ProjectRecord, ForgeError> {
        let (manifest, _) = Manifest::load_from_dir(dir, explicit)?;
        let canonical = dir
            .canonicalize()
            .map_err(|_| ForgeError::PathUnavailable {
                path: dir.display().to_string(),
            })?;
        let canonical_text = canonical.display().to_string();
        let (git_remote, last_commit) = capture_git_info(&canonical);
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO operations (kind, project_id, state, started_at)
             VALUES ('register', ?1, 'pending', ?2)",
            params![manifest.project.id, now],
        )?;
        let op_id = self.conn.last_insert_rowid();
        let result =
            self.insert_project(op_id, &manifest, &canonical_text, git_remote, last_commit);
        if let Err(err) = &result {
            let _ = self.mark_op_failed(op_id, &err.to_string());
        }
        result
    }
    pub(super) fn mark_op_failed(&self, op_id: i64, detail: &str) -> Result<(), ForgeError> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE operations SET state = 'failed', finished_at = ?1, detail = ?2
             WHERE op_id = ?3",
            params![now, detail, op_id],
        )?;
        Ok(())
    }
    /// Read-only identity pre-check for import adoption: reports the same
    /// id/path collisions as registration without mutating any row, so the
    /// caller can validate before writing a manifest file.
    pub fn check_identity_available(
        &self,
        id: &str,
        canonical_path: &str,
    ) -> Result<(), ForgeError> {
        let owner_of_id: Option<String> = self
            .conn
            .query_row(
                "SELECT path FROM projects WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .optional()?;
        if let Some(owner) = owner_of_id {
            if owner != canonical_path {
                return Err(ForgeError::IdCollision { id: id.to_string() });
            }
        }
        let owner_of_path: Option<String> = self
            .conn
            .query_row(
                "SELECT id FROM projects WHERE path = ?1",
                params![canonical_path],
                |row| row.get(0),
            )
            .optional()?;
        if let Some(owner) = owner_of_path {
            if owner != id {
                return Err(ForgeError::PathCollision {
                    path: canonical_path.to_string(),
                });
            }
        }
        Ok(())
    }
    pub(super) fn insert_project(
        &mut self,
        op_id: i64,
        manifest: &Manifest,
        canonical_path: &str,
        git_remote: Option<String>,
        last_commit: Option<String>,
    ) -> Result<ProjectRecord, ForgeError> {
        let descriptor = crate::profile::inspect_profile(&manifest.project.profile)?;
        let requested: Vec<String> = manifest.features.keys().cloned().collect();
        crate::profile::resolve_profile(&descriptor.id, &requested)?;
        let id = manifest.project.id.clone();
        let owner_of_id: Option<String> = self
            .conn
            .query_row(
                "SELECT path FROM projects WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .optional()?;
        if let Some(owner) = owner_of_id {
            if owner != canonical_path {
                return Err(ForgeError::IdCollision { id });
            }
        }
        let owner_of_path: Option<String> = self
            .conn
            .query_row(
                "SELECT id FROM projects WHERE path = ?1",
                params![canonical_path],
                |row| row.get(0),
            )
            .optional()?;
        if let Some(owner) = owner_of_path {
            if owner != id {
                return Err(ForgeError::PathCollision {
                    path: canonical_path.to_string(),
                });
            }
        }
        let project = &manifest.project;
        let stack = manifest.runtime.as_ref().and_then(|r| r.language.clone());
        let runtime = match manifest.runtime.as_ref() {
            Some(r) => match (&r.language, &r.version) {
                (Some(l), Some(v)) => Some(format!("{l}@{v}")),
                (Some(l), None) => Some(l.clone()),
                (None, Some(v)) => Some(v.clone()),
                (None, None) => None,
            },
            None => None,
        };
        let mirrors: Vec<String> = manifest
            .distribution
            .as_ref()
            .map(|d| d.mirrors.iter().map(|m| m.provider().to_string()).collect())
            .unwrap_or_default();
        let now = Utc::now().to_rfc3339();
        let tx = self.conn.transaction()?;
        tx.execute(
            "INSERT INTO projects
             (id, name, path, git_remote, mirror_remotes, stack, profile,
              maturity, target_maturity, schema_version, platform_version,
              features, deployment_target, runtime, last_commit,
              quality_status, agent_status, docs_status, kit_id, kit_version,
              observed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12,
                     ?13, ?14, ?15, NULL, NULL, NULL, ?16, ?17, ?18)
             ON CONFLICT(id) DO UPDATE SET
               name = excluded.name, path = excluded.path,
               git_remote = excluded.git_remote,
               mirror_remotes = excluded.mirror_remotes, stack = excluded.stack,
               profile = excluded.profile, maturity = excluded.maturity,
               target_maturity = excluded.target_maturity,
               schema_version = excluded.schema_version,
               platform_version = excluded.platform_version,
               features = excluded.features,
               deployment_target = excluded.deployment_target,
               runtime = excluded.runtime, last_commit = excluded.last_commit,
               kit_id = excluded.kit_id, kit_version = excluded.kit_version,
               observed_at = excluded.observed_at",
            params![
                id,
                project.name,
                canonical_path,
                git_remote,
                serde_json::to_string(&mirrors).unwrap_or_else(|_| "[]".to_string()),
                stack,
                project.profile,
                project.maturity.map(|m| m.to_string()),
                project.target_maturity.map(|m| m.to_string()),
                manifest.schema,
                PLATFORM_VERSION,
                serde_json::to_string(&manifest.features).unwrap_or_else(|_| "{}".to_string()),
                manifest.deployment.as_ref().and_then(|d| d.target.clone()),
                runtime,
                last_commit,
                manifest.kit.as_ref().map(|k| k.id.clone()),
                manifest.kit.as_ref().and_then(|k| k.version.clone()),
                now,
            ],
        )?;
        tx.execute(
            "UPDATE operations SET state = 'done', finished_at = ?1 WHERE op_id = ?2",
            params![now, op_id],
        )?;
        tx.commit()?;
        self.inspect(&id)
    }
    /// Look up a project by id, or by filesystem path when the query is
    /// not a registered id but resolves to a registered canonical path.
    pub fn inspect(&self, query: &str) -> Result<ProjectRecord, ForgeError> {
        if let Some(record) = self.by_id(query)? {
            return Ok(record);
        }
        let candidate = Path::new(query);
        if let Ok(canonical) = candidate.canonicalize() {
            if let Some(record) = self.by_path(&canonical.display().to_string())? {
                return Ok(record);
            }
        }
        Err(ForgeError::UnknownProject {
            query: query.to_string(),
        })
    }
    /// All registered projects ordered by id.
    pub fn list(&self) -> Result<Vec<ProjectRecord>, ForgeError> {
        let mut stmt = self.conn.prepare("SELECT id FROM projects ORDER BY id")?;
        let ids: Vec<String> = stmt
            .query_map([], |row| row.get(0))?
            .collect::<Result<_, _>>()?;
        let mut out = Vec::with_capacity(ids.len());
        for id in ids {
            if let Some(record) = self.by_id(&id)? {
                out.push(record);
            }
        }
        Ok(out)
    }
    pub(super) fn by_id(&self, id: &str) -> Result<Option<ProjectRecord>, ForgeError> {
        self.conn
            .query_row(
                "SELECT id, name, path, git_remote, mirror_remotes, stack, profile,
                        maturity, target_maturity, schema_version, platform_version,
                        features, deployment_target, runtime, last_commit,
                        quality_status, agent_status, docs_status, kit_id, kit_version,
                        observed_at
                 FROM projects WHERE id = ?1",
                params![id],
                read_record,
            )
            .optional()
            .map_err(ForgeError::from)
    }
    pub(super) fn by_path(&self, path: &str) -> Result<Option<ProjectRecord>, ForgeError> {
        self.conn
            .query_row(
                "SELECT id, name, path, git_remote, mirror_remotes, stack, profile,
                        maturity, target_maturity, schema_version, platform_version,
                        features, deployment_target, runtime, last_commit,
                        quality_status, agent_status, docs_status, kit_id, kit_version,
                        observed_at
                 FROM projects WHERE path = ?1",
                params![path],
                read_record,
            )
            .optional()
            .map_err(ForgeError::from)
    }
}
/// One journal entry; surfaced for reconciliation evidence.
#[derive(Debug, Clone, Serialize)]
pub struct OperationEntry {
    pub op_id: i64,
    pub kind: String,
    pub project_id: String,
    pub state: String,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub detail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_hash: Option<String>,
    /// Optional fleet run identifier that groups per-project publish
    /// rows under a single invocation. Always `None` for non-fleet
    /// operations (registration, upgrade, gate, …).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub queue_id: Option<String>,
    /// Committed Git revision the publish transferred (full 40-char
    /// hex SHA). `None` for non-publish operations and for legacy
    /// rows persisted before phase evidence existed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revision: Option<String>,
    /// Terminal status of the `build` phase
    /// (`succeeded`/`failed`/`not_started`/`unknown`). `None` for
    /// legacy rows that did not carry phase evidence.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build_status: Option<String>,
    /// Terminal status of the `run` phase (same vocabulary as
    /// `build_status`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_status: Option<String>,
    /// Bounded Compose project / container identity the provider
    /// actually deployed (canonical `forge-<project>-<sha12>`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container_identity: Option<String>,
}
/// Phase evidence carried by a publish journal row. All four fields
/// are optional so legacy providers that did not advertise phase
/// evidence still get their rows persisted with `NULL` values; the
/// `forge deploy status` projection renders the missing values as
/// `unknown` instead of a verified success. The struct groups the
/// phase inputs so callers do not have to pass seven positional
/// arguments to [`Registry::record_publish_phase`] or
/// [`Registry::record_queue_publish_phase`].
#[derive(Debug, Clone, Default)]
pub struct PublishPhaseEvidence<'a> {
    pub revision: Option<&'a str>,
    pub build_status: Option<&'a str>,
    pub run_status: Option<&'a str>,
    pub container_identity: Option<&'a str>,
}
impl<'a> PublishPhaseEvidence<'a> {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn revision(mut self, value: &'a str) -> Self {
        self.revision = Some(value);
        self
    }
    pub fn build_status(mut self, value: &'a str) -> Self {
        self.build_status = Some(value);
        self
    }
    pub fn build_status_opt(mut self, value: Option<&'a str>) -> Self {
        self.build_status = value;
        self
    }
    pub fn run_status(mut self, value: &'a str) -> Self {
        self.run_status = Some(value);
        self
    }
    pub fn run_status_opt(mut self, value: Option<&'a str>) -> Self {
        self.run_status = value;
        self
    }
    pub fn container_identity(mut self, value: &'a str) -> Self {
        self.container_identity = Some(value);
        self
    }
    pub fn container_identity_opt(mut self, value: Option<&'a str>) -> Self {
        self.container_identity = value;
        self
    }
}
/// Persisted project identity plus timestamped observations.
/// `available` is computed at read time, never stored: a missing path
/// reports `unavailable`, a missing observation reports `unknown`.
#[derive(Debug, Clone, Serialize)]
pub struct ProjectRecord {
    pub id: String,
    pub name: String,
    pub path: String,
    pub git_remote: Option<String>,
    pub mirror_remotes: Vec<String>,
    pub stack: Option<String>,
    pub profile: String,
    pub maturity: Option<String>,
    pub target_maturity: Option<String>,
    pub schema_version: i64,
    pub platform_version: String,
    pub features: BTreeMap<String, String>,
    pub deployment_target: Option<String>,
    pub runtime: Option<String>,
    pub last_commit: Option<String>,
    pub quality_status: Option<String>,
    pub agent_status: Option<String>,
    pub docs_status: Option<String>,
    /// Shared-layer kit pinned at generation time
    /// (`scaffold-prewires-shared-layer`). `None` means the project's
    /// manifest declares no kit — a pre-kit project, or one whose profile
    /// predates the contract. Observed additively and never rewritten: a new
    /// kit version does not touch an existing project.
    pub kit_id: Option<String>,
    pub kit_version: Option<String>,
    pub observed_at: String,
    pub available: bool,
}
impl ProjectRecord {
    /// Health for human display: never `healthy` on missing evidence.
    pub fn health(&self) -> &str {
        if !self.available {
            return "unavailable";
        }
        if self.runtime.is_none() {
            return "unknown";
        }
        "ok"
    }
}
