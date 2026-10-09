//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

pub use super::interest::PORTFOLIO_INTEREST_SCHEMA_SQL;
pub use super::portfolio::PORTFOLIO_SCHEMA_SQL;
pub use super::share::PORTFOLIO_SHARE_SCHEMA_SQL;
use crate::core::ForgeError;
use rusqlite::Connection;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::model::{OperationEntry, ProjectRecord};

/// Apply in-place schema migrations for registries
/// created by an earlier `forge` build. The current
/// schema is created by [`SCHEMA_SQL`]; older schemas
/// need additive `ALTER TABLE` statements to add the
/// `idempotency_key` and `request_hash` columns. Each
/// statement is allowed to fail with
/// `SqliteFailure(..., "duplicate column name: ...")` —
/// that means the column already exists and the
/// migration is a no-op.
pub(super) fn apply_migrations(conn: &Connection) -> Result<(), ForgeError> {
    let migrations: &[&str] = &[
        "ALTER TABLE operations ADD COLUMN idempotency_key TEXT",
        "ALTER TABLE operations ADD COLUMN request_hash TEXT",
        "ALTER TABLE operations ADD COLUMN queue_id TEXT",
        "ALTER TABLE operations ADD COLUMN revision TEXT",
        "ALTER TABLE operations ADD COLUMN build_status TEXT",
        "ALTER TABLE operations ADD COLUMN run_status TEXT",
        "ALTER TABLE operations ADD COLUMN container_identity TEXT",
        // Pinned shared-layer kit, observed additively. No new table: the
        // pinned id and version are two more observed scalars on the existing
        // project record, and a registry created before kits existed reads
        // them as NULL ("this project predates kits").
        "ALTER TABLE projects ADD COLUMN kit_id TEXT",
        "ALTER TABLE projects ADD COLUMN kit_version TEXT",
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
    if let Err(err) = conn.execute(
        "CREATE INDEX IF NOT EXISTS operations_queue_project_state_idx
            ON operations (queue_id, project_id, state)
            WHERE queue_id IS NOT NULL",
        [],
    ) {
        let message = err.to_string();
        if !message.contains("already exists") {
            return Err(ForgeError::Registry {
                reason: format!("queue index creation failed: {message}"),
            });
        }
    }
    apply_portfolio_migration(conn)?;
    apply_portfolio_share_migration(conn)?;
    apply_portfolio_interest_migration(conn)
}

/// Apply the additive portfolio schema inside one explicit
/// transaction.
///
/// `BEGIN` / `COMMIT` are issued explicitly rather than relying on
/// SQLite's implicit batch behaviour, so the rollback guarantee is
/// deterministic: if any statement fails, the batch is rolled back
/// and neither a partial portfolio table nor a half-applied index
/// survives. The pre-existing `projects`/`operations` tables stay
/// exactly as they were and the next open retries the whole batch.
/// Every statement is idempotent, so the batch is a no-op on a
/// registry that already carries the portfolio domain.
pub(super) fn apply_portfolio_migration(conn: &Connection) -> Result<(), ForgeError> {
    if let Err(err) = conn.execute_batch("BEGIN IMMEDIATE") {
        return Err(ForgeError::Registry {
            reason: format!("portfolio migration could not begin: {err}; registry left unchanged"),
        });
    }
    if let Err(err) = conn.execute_batch(PORTFOLIO_SCHEMA_SQL) {
        // Best effort: surface the original failure, not a
        // rollback failure that would hide it.
        let _ = conn.execute_batch("ROLLBACK");
        return Err(ForgeError::Registry {
            reason: format!("portfolio migration failed: {err}; registry left unchanged"),
        });
    }
    if let Err(err) = conn.execute_batch("COMMIT") {
        let _ = conn.execute_batch("ROLLBACK");
        return Err(ForgeError::Registry {
            reason: format!("portfolio migration commit failed: {err}; registry left unchanged"),
        });
    }
    Ok(())
}

/// Apply the additive portfolio share schema inside one explicit
/// transaction.
///
/// The batch is deliberately separate from the portfolio batch so a
/// rollback in either domain leaves the other intact, and it follows
/// the same rule: `BEGIN IMMEDIATE` is issued explicitly, every
/// statement is `CREATE ... IF NOT EXISTS`, and any failure rolls the
/// whole batch back so no partial share table survives. The next open
/// retries it from scratch.
pub(super) fn apply_portfolio_share_migration(conn: &Connection) -> Result<(), ForgeError> {
    if let Err(err) = conn.execute_batch("BEGIN IMMEDIATE") {
        return Err(ForgeError::Registry {
            reason: format!(
                "portfolio share migration could not begin: {err}; registry left unchanged"
            ),
        });
    }
    if let Err(err) = conn.execute_batch(PORTFOLIO_SHARE_SCHEMA_SQL) {
        // Best effort: surface the original failure, not a
        // rollback failure that would hide it.
        let _ = conn.execute_batch("ROLLBACK");
        return Err(ForgeError::Registry {
            reason: format!("portfolio share migration failed: {err}; registry left unchanged"),
        });
    }
    if let Err(err) = conn.execute_batch("COMMIT") {
        let _ = conn.execute_batch("ROLLBACK");
        return Err(ForgeError::Registry {
            reason: format!(
                "portfolio share migration commit failed: {err}; registry left unchanged"
            ),
        });
    }
    Ok(())
}

/// Apply the additive portfolio interest schema inside one explicit
/// transaction.
///
/// The batch is deliberately separate from the portfolio and share
/// batches so a rollback in any domain leaves the others intact, and it
/// follows the same rule: `BEGIN IMMEDIATE` is issued explicitly, every
/// statement is `CREATE ... IF NOT EXISTS`, and any failure rolls the
/// whole batch back so no partial interest table survives. The next
/// open retries it from scratch.
pub(super) fn apply_portfolio_interest_migration(conn: &Connection) -> Result<(), ForgeError> {
    if let Err(err) = conn.execute_batch("BEGIN IMMEDIATE") {
        return Err(ForgeError::Registry {
            reason: format!(
                "portfolio interest migration could not begin: {err}; registry left unchanged"
            ),
        });
    }
    if let Err(err) = conn.execute_batch(PORTFOLIO_INTEREST_SCHEMA_SQL) {
        // Best effort: surface the original failure, not a
        // rollback failure that would hide it.
        let _ = conn.execute_batch("ROLLBACK");
        return Err(ForgeError::Registry {
            reason: format!("portfolio interest migration failed: {err}; registry left unchanged"),
        });
    }
    if let Err(err) = conn.execute_batch("COMMIT") {
        let _ = conn.execute_batch("ROLLBACK");
        return Err(ForgeError::Registry {
            reason: format!(
                "portfolio interest migration commit failed: {err}; registry left unchanged"
            ),
        });
    }
    Ok(())
}

/// Decode one row from any of the operations-table SELECTs into an
/// [`OperationEntry`]. The column order is fixed and shared by every
/// query in this module so the projection code stays a single
/// match-the-field-to-its-index helper.
pub(super) fn row_to_operation_entry(row: &rusqlite::Row<'_>) -> rusqlite::Result<OperationEntry> {
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
        queue_id: row.get(9)?,
        revision: row.get(10)?,
        build_status: row.get(11)?,
        run_status: row.get(12)?,
        container_identity: row.get(13)?,
    })
}

pub(super) fn read_record(row: &rusqlite::Row<'_>) -> rusqlite::Result<ProjectRecord> {
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
        kit_id: row.get(18)?,
        kit_version: row.get(19)?,
        observed_at: row.get(20)?,
        available,
    })
}

/// Best-effort Git capture via argument arrays (never shell). Any failure
/// leaves `unknown` rather than failing registration.
pub(super) fn capture_git_info(dir: &Path) -> (Option<String>, Option<String>) {
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
pub(super) mod tests {
    use super::*;
    use crate::registry::Registry;
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
