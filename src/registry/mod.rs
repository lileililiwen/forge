//! Persistent project registry backed by SQLite.
//!
//! The registry stores one row per project plus an append-only operation
//! journal. Manifest files and the database can never share one
//! transaction, so `register` writes the database only: a `pending`
//! journal entry is committed first, then the project row plus the
//! terminal journal state in a second transaction. A `pending` entry
//! found at open time belongs to an interrupted run and is reconciled to
//! `failed` — it is never reported as success.

use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use crate::core::manifest::Manifest;
use crate::core::ForgeError;

/// Platform (Forge) version recorded with every registered project.
pub const PLATFORM_VERSION: &str = env!("CARGO_PKG_VERSION");

const SCHEMA_SQL: &str = "
CREATE TABLE IF NOT EXISTS projects (
    id               TEXT PRIMARY KEY,
    name             TEXT NOT NULL,
    path             TEXT NOT NULL UNIQUE,
    git_remote       TEXT,
    mirror_remotes   TEXT NOT NULL DEFAULT '[]',
    stack            TEXT,
    profile          TEXT NOT NULL,
    maturity         TEXT,
    target_maturity  TEXT,
    schema_version   INTEGER NOT NULL,
    platform_version TEXT NOT NULL,
    features         TEXT NOT NULL DEFAULT '{}',
    deployment_target TEXT,
    runtime          TEXT,
    last_commit      TEXT,
    quality_status   TEXT,
    agent_status     TEXT,
    docs_status      TEXT,
    observed_at      TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS operations (
    op_id           INTEGER PRIMARY KEY AUTOINCREMENT,
    kind            TEXT NOT NULL,
    project_id      TEXT NOT NULL,
    state           TEXT NOT NULL DEFAULT 'pending',
    started_at      TEXT NOT NULL,
    finished_at     TEXT,
    detail          TEXT,
    idempotency_key TEXT,
    request_hash    TEXT
);
CREATE UNIQUE INDEX IF NOT EXISTS operations_idempotency_uniq
    ON operations (kind, idempotency_key)
    WHERE idempotency_key IS NOT NULL;
";

/// Apply in-place schema migrations for registries
/// created by an earlier `forge` build. The current
/// schema is created by [`SCHEMA_SQL`]; older schemas
/// need additive `ALTER TABLE` statements to add the
/// `idempotency_key` and `request_hash` columns. Each
/// statement is allowed to fail with
/// `SqliteFailure(..., "duplicate column name: ...")` —
/// that means the column already exists and the
/// migration is a no-op.
fn apply_migrations(conn: &Connection) -> Result<(), ForgeError> {
    let migrations: &[&str] = &[
        "ALTER TABLE operations ADD COLUMN idempotency_key TEXT",
        "ALTER TABLE operations ADD COLUMN request_hash TEXT",
    ];
    for stmt in migrations {
        if let Err(err) = conn.execute(stmt, []) {
            let message = err.to_string();
            // SQLite raises "duplicate column name" when
            // the column is already there. Anything else
            // is a real failure that should bubble up.
            if !message.contains("duplicate column name") {
                return Err(ForgeError::Registry {
                    reason: format!("migration `{stmt}` failed: {message}"),
                });
            }
        }
    }
    // Idempotency unique index may already be present
    // from a fresh install; ignore the duplicate error
    // here as well.
    if let Err(err) = conn.execute(
        "CREATE UNIQUE INDEX IF NOT EXISTS operations_idempotency_uniq
            ON operations (kind, idempotency_key)
            WHERE idempotency_key IS NOT NULL",
        [],
    ) {
        let message = err.to_string();
        if !message.contains("already exists") {
            return Err(ForgeError::Registry {
                reason: format!("idempotency index creation failed: {message}"),
            });
        }
    }
    Ok(())
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
}

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

pub struct Registry {
    conn: Connection,
}

impl Registry {
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
                    idempotency_key, request_hash
             FROM operations ORDER BY op_id",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(OperationEntry {
                op_id: row.get(0)?,
                kind: row.get(1)?,
                project_id: row.get(2)?,
                state: row.get(3)?,
                started_at: row.get(4)?,
                finished_at: row.get(5)?,
                detail: row.get(6)?,
                idempotency_key: row.get(7)?,
                request_hash: row.get(8)?,
            })
        })?;
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
                    idempotency_key, request_hash
             FROM operations WHERE op_id = ?1",
        )?;
        let entry = stmt
            .query_row(params![op_id], |row| {
                Ok(OperationEntry {
                    op_id: row.get(0)?,
                    kind: row.get(1)?,
                    project_id: row.get(2)?,
                    state: row.get(3)?,
                    started_at: row.get(4)?,
                    finished_at: row.get(5)?,
                    detail: row.get(6)?,
                    idempotency_key: row.get(7)?,
                    request_hash: row.get(8)?,
                })
            })
            .optional()?;
        Ok(entry)
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
        // Conflict check first: a key reused with a different fingerprint
        // is refused before any row is written. The unique index is the
        // secondary guard against double insertion under concurrency.
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

    /// Look up an operation by `(kind, idempotency_key)`. Returns
    /// `None` when the key has never been recorded for this kind.
    pub fn operation_by_idempotency(
        &self,
        kind: &str,
        idempotency_key: &str,
    ) -> Result<Option<OperationEntry>, ForgeError> {
        let mut stmt = self.conn.prepare(
            "SELECT op_id, kind, project_id, state, started_at, finished_at, detail,
                    idempotency_key, request_hash
             FROM operations
             WHERE kind = ?1 AND idempotency_key = ?2
             ORDER BY op_id DESC
             LIMIT 1",
        )?;
        let entry = stmt
            .query_row(params![kind, idempotency_key], |row| {
                Ok(OperationEntry {
                    op_id: row.get(0)?,
                    kind: row.get(1)?,
                    project_id: row.get(2)?,
                    state: row.get(3)?,
                    started_at: row.get(4)?,
                    finished_at: row.get(5)?,
                    detail: row.get(6)?,
                    idempotency_key: row.get(7)?,
                    request_hash: row.get(8)?,
                })
            })
            .optional()?;
        Ok(entry)
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

    fn mark_op_failed(&self, op_id: i64, detail: &str) -> Result<(), ForgeError> {
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

    fn insert_project(
        &mut self,
        op_id: i64,
        manifest: &Manifest,
        canonical_path: &str,
        git_remote: Option<String>,
        last_commit: Option<String>,
    ) -> Result<ProjectRecord, ForgeError> {
        // Profile compatibility is validated before any row mutation: an
        // unknown profile or an unsupported capability fails registration
        // with the original record (if any) left unchanged.
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
              quality_status, agent_status, docs_status, observed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12,
                     ?13, ?14, ?15, NULL, NULL, NULL, ?16)
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

    fn by_id(&self, id: &str) -> Result<Option<ProjectRecord>, ForgeError> {
        self.conn
            .query_row(
                "SELECT id, name, path, git_remote, mirror_remotes, stack, profile,
                        maturity, target_maturity, schema_version, platform_version,
                        features, deployment_target, runtime, last_commit,
                        quality_status, agent_status, docs_status, observed_at
                 FROM projects WHERE id = ?1",
                params![id],
                read_record,
            )
            .optional()
            .map_err(ForgeError::from)
    }

    fn by_path(&self, path: &str) -> Result<Option<ProjectRecord>, ForgeError> {
        self.conn
            .query_row(
                "SELECT id, name, path, git_remote, mirror_remotes, stack, profile,
                        maturity, target_maturity, schema_version, platform_version,
                        features, deployment_target, runtime, last_commit,
                        quality_status, agent_status, docs_status, observed_at
                 FROM projects WHERE path = ?1",
                params![path],
                read_record,
            )
            .optional()
            .map_err(ForgeError::from)
    }
}

fn read_record(row: &rusqlite::Row<'_>) -> rusqlite::Result<ProjectRecord> {
    let mirrors_json: String = row.get(4)?;
    let features_json: String = row.get(11)?;
    let path: String = row.get(2)?;
    let available = Path::new(&path).exists();
    Ok(ProjectRecord {
        id: row.get(0)?,
        name: row.get(1)?,
        path: path.clone(),
        git_remote: row.get(3)?,
        mirror_remotes: serde_json::from_str(&mirrors_json).unwrap_or_default(),
        stack: row.get(5)?,
        profile: row.get(6)?,
        maturity: row.get(7)?,
        target_maturity: row.get(8)?,
        schema_version: row.get(9)?,
        platform_version: row.get(10)?,
        features: serde_json::from_str(&features_json).unwrap_or_default(),
        deployment_target: row.get(12)?,
        runtime: row.get(13)?,
        last_commit: row.get(14)?,
        quality_status: row.get(15)?,
        agent_status: row.get(16)?,
        docs_status: row.get(17)?,
        observed_at: row.get(18)?,
        available,
    })
}

/// Best-effort Git capture via argument arrays (never shell). Any failure
/// leaves `unknown` rather than failing registration.
fn capture_git_info(dir: &Path) -> (Option<String>, Option<String>) {
    let remote = Command::new("git")
        .arg("-C")
        .arg(dir)
        .arg("remote")
        .arg("get-url")
        .arg("origin")
        .output()
        .ok()
        .filter(|out| out.status.success())
        .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_string())
        .filter(|s| !s.is_empty());

    let commit = Command::new("git")
        .arg("-C")
        .arg(dir)
        .arg("rev-parse")
        .arg("HEAD")
        .output()
        .ok()
        .filter(|out| out.status.success())
        .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_string())
        .filter(|s| !s.is_empty());

    (remote, commit)
}

/// Default registry location: `$FORGE_REGISTRY`, else XDG data home,
/// else `~/.local/share/forge/registry.db`.
pub fn default_registry_path() -> PathBuf {
    if let Ok(path) = std::env::var("FORGE_REGISTRY") {
        if !path.trim().is_empty() {
            return PathBuf::from(path);
        }
    }
    if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
        if !xdg.trim().is_empty() {
            return PathBuf::from(xdg).join("forge").join("registry.db");
        }
    }
    match std::env::var("HOME") {
        Ok(home) => PathBuf::from(home)
            .join(".local")
            .join("share")
            .join("forge")
            .join("registry.db"),
        Err(_) => PathBuf::from("forge-registry.db"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write_manifest(dir: &Path, name: &str, text: &str) -> PathBuf {
        let path = dir.join(name);
        fs::write(&path, text).unwrap();
        path
    }

    fn full_manifest_text(id: &str) -> String {
        format!(
            "schema: 1\nproject:\n  id: {id}\n  name: Test {id}\n  profile: rust-web\n  maturity: L1\nruntime:\n  language: rust\nfeatures:\n  auth: 2.1\n"
        )
    }

    fn open_registry(dir: &TempDir) -> Registry {
        Registry::open(&dir.path().join("registry.db")).unwrap()
    }

    #[test]
    fn persists_identity_and_observations_across_restart() {
        let tmp = TempDir::new().unwrap();
        let proj = tmp.path().join("proj");
        fs::create_dir(&proj).unwrap();
        write_manifest(&proj, "forge.yaml", &full_manifest_text("persist-me"));
        let db_path = tmp.path().join("registry.db");

        let before = {
            let mut reg = Registry::open(&db_path).unwrap();
            reg.register(&proj, None).unwrap()
        };
        assert_eq!(before.schema_version, 1);
        assert!(!before.observed_at.is_empty());
        // Non-git fixture: observations stay unknown rather than failing.
        assert!(before.last_commit.is_none());

        let after = {
            let reg = Registry::open(&db_path).unwrap();
            reg.inspect("persist-me").unwrap()
        };
        assert_eq!(after.id, before.id);
        assert_eq!(after.path, before.path);
        assert_eq!(after.observed_at, before.observed_at);
        assert_eq!(after.features.get("auth").map(String::as_str), Some("2.1"));
    }

    #[test]
    fn rejects_id_and_path_collisions() {
        let tmp = TempDir::new().unwrap();
        let a = tmp.path().join("a");
        let b = tmp.path().join("b");
        fs::create_dir(&a).unwrap();
        fs::create_dir(&b).unwrap();
        write_manifest(&a, "forge.yaml", &full_manifest_text("same-id"));
        write_manifest(&b, "forge.yaml", &full_manifest_text("same-id"));
        let other = tmp.path().join("other");
        fs::create_dir(&other).unwrap();
        write_manifest(&other, "forge.yaml", &full_manifest_text("other-id"));

        let mut reg = open_registry(&tmp);
        let original = reg.register(&a, None).unwrap();

        let err = reg.register(&b, None).expect_err("id reuse must fail");
        assert_eq!(err.code(), "id-collision");
        // Original record unchanged.
        assert_eq!(reg.inspect("same-id").unwrap().path, original.path);

        // Same path under a different id.
        write_manifest(&a, "forge.yaml", &full_manifest_text("other-id"));
        let err = reg.register(&a, None).expect_err("path reuse must fail");
        assert_eq!(err.code(), "path-collision");
    }

    #[test]
    fn reports_unavailable_path_and_unknown_id() {
        let tmp = TempDir::new().unwrap();
        let proj = tmp.path().join("proj");
        fs::create_dir(&proj).unwrap();
        write_manifest(&proj, "forge.yaml", &full_manifest_text("gone-fish"));
        // No runtime section variant for the unknown-health path.
        let norun = tmp.path().join("norun");
        fs::create_dir(&norun).unwrap();
        write_manifest(
            &norun,
            "forge.yaml",
            "schema: 1\nproject:\n  id: no-run\n  name: No Run\n  profile: rust-web\n",
        );

        let mut reg = open_registry(&tmp);
        reg.register(&proj, None).unwrap();
        reg.register(&norun, None).unwrap();
        fs::remove_dir_all(&proj).unwrap();

        let gone = reg.inspect("gone-fish").unwrap();
        assert!(!gone.available);
        assert_eq!(gone.health(), "unavailable");
        let minimal = reg.inspect("no-run").unwrap();
        assert_eq!(minimal.health(), "unknown");

        let err = reg.inspect("no-such-project").expect_err("unknown id");
        assert_eq!(err.code(), "unknown-project");
    }

    #[test]
    fn register_leaves_both_manifests_unchanged_on_conflict() {
        let tmp = TempDir::new().unwrap();
        let proj = tmp.path().join("proj");
        fs::create_dir(&proj).unwrap();
        let forge_text = full_manifest_text("dual-proj");
        write_manifest(&proj, "forge.yaml", &forge_text);
        write_manifest(&proj, "platform.yaml", &forge_text);

        let mut reg = open_registry(&tmp);
        let err = reg.register(&proj, None).expect_err("dual manifests");
        assert_eq!(err.code(), "ambiguous-manifest");
        assert_eq!(
            fs::read_to_string(proj.join("forge.yaml")).unwrap(),
            forge_text
        );

        // Explicit legacy import works.
        let record = reg
            .register(&proj, Some(Path::new("platform.yaml")))
            .unwrap();
        assert_eq!(record.id, "dual-proj");
    }

    #[test]
    fn unsupported_schema_is_never_rewritten() {
        let tmp = TempDir::new().unwrap();
        let proj = tmp.path().join("proj");
        fs::create_dir(&proj).unwrap();
        let text = "schema: 99\nproject:\n  id: future\n  name: Future\n  profile: rust-web\n";
        write_manifest(&proj, "forge.yaml", text);

        let mut reg = open_registry(&tmp);
        let err = reg.register(&proj, None).expect_err("schema 99");
        assert_eq!(err.code(), "unsupported-schema");
        assert_eq!(fs::read_to_string(proj.join("forge.yaml")).unwrap(), text);
        assert!(reg.inspect("future").is_err());
    }

    #[test]
    fn reconcile_marks_interrupted_operations_failed() {
        let tmp = TempDir::new().unwrap();
        let db_path = tmp.path().join("registry.db");
        {
            let reg = Registry::open(&db_path).unwrap();
            reg.conn
                .execute(
                    "INSERT INTO operations (kind, project_id, state, started_at)
                     VALUES ('register', 'ghost', 'pending', '2026-01-01T00:00:00Z')",
                    [],
                )
                .unwrap();
        }
        let reg = Registry::open(&db_path).unwrap();
        let entries = reg.journal_entries().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].state, "failed");
    }
}
