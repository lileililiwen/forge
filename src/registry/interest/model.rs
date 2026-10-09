//! Interest persistence model: the additive schema DDL, the outcome
//! vocabulary, and the shared row decoder. Registry-independent.

use crate::core::ForgeError;
use crate::portfolio::interest::{InterestSnapshot, SnapshotState};

/// Additive interest schema. Every statement is `CREATE ... IF NOT
/// EXISTS`, so a registry written before the interest package migrates
/// forward in place and re-opening is a no-op. SQLite does not enforce
/// foreign keys here, so [`crate::registry::Registry::require_project`]
/// resolves the project against the canonical registry identity
/// itself and returns a typed `unknown-project` rather than relying on
/// the pragma.
pub const PORTFOLIO_INTEREST_SCHEMA_SQL: &str = "
CREATE TABLE IF NOT EXISTS portfolio_interest_snapshots (
    id                       INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id               TEXT NOT NULL,
    source                   TEXT NOT NULL,
    source_revision          TEXT NOT NULL,
    window_start             TEXT NOT NULL,
    window_end               TEXT NOT NULL,
    privacy_mode             TEXT NOT NULL,
    coverage                 TEXT NOT NULL,
    state                    TEXT NOT NULL DEFAULT 'accepted',
    replaces_source_revision TEXT,
    actor                    TEXT NOT NULL,
    received_at              TEXT NOT NULL,
    UNIQUE (project_id, source, source_revision, window_start, window_end)
);
CREATE TABLE IF NOT EXISTS portfolio_interest_metrics (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    snapshot_id INTEGER NOT NULL,
    metric      TEXT NOT NULL,
    value       INTEGER NOT NULL,
    UNIQUE (snapshot_id, metric)
);
CREATE TABLE IF NOT EXISTS portfolio_interest_findings (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id TEXT NOT NULL,
    field      TEXT NOT NULL,
    code       TEXT NOT NULL,
    detail     TEXT NOT NULL,
    created_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS portfolio_interest_snapshots_project_idx
    ON portfolio_interest_snapshots (project_id, window_start);
CREATE INDEX IF NOT EXISTS portfolio_interest_snapshots_source_idx
    ON portfolio_interest_snapshots (project_id, source, window_start);
CREATE INDEX IF NOT EXISTS portfolio_interest_metrics_snapshot_idx
    ON portfolio_interest_metrics (snapshot_id, metric);
CREATE INDEX IF NOT EXISTS portfolio_interest_findings_project_idx
    ON portfolio_interest_findings (project_id, id);
";

pub(crate) fn invalid(reason: String) -> ForgeError {
    ForgeError::PortfolioInterestInvalid { reason }
}

pub(crate) fn conflict(reason: String) -> ForgeError {
    ForgeError::PortfolioInterestConflict { reason }
}

/// What one import attempt did with one snapshot.
///
/// `AlreadyPresent` is the idempotent result and carries the stored
/// snapshot, so a retrying adapter learns what Forge already holds
/// rather than only that it refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SnapshotOutcome {
    Accepted {
        snapshot: InterestSnapshot,
    },
    AlreadyPresent {
        snapshot: InterestSnapshot,
    },
    /// An overlapping same-source window was replaced. The superseded
    /// snapshot is returned so the caller can report both.
    AcceptedWithSupersession {
        snapshot: InterestSnapshot,
        superseded: Box<InterestSnapshot>,
    },
}

impl SnapshotOutcome {
    pub fn snapshot(&self) -> &InterestSnapshot {
        match self {
            SnapshotOutcome::Accepted { snapshot }
            | SnapshotOutcome::AlreadyPresent { snapshot }
            | SnapshotOutcome::AcceptedWithSupersession { snapshot, .. } => snapshot,
        }
    }

    pub fn is_idempotent_repeat(&self) -> bool {
        matches!(self, SnapshotOutcome::AlreadyPresent { .. })
    }
}

/// Decode one `portfolio_interest_snapshots` row. The column order is
/// fixed and shared by every SELECT so the projection stays one
/// mapper.
pub(crate) fn row_to_interest_snapshot(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<InterestSnapshot> {
    let state: String = row.get(8)?;
    Ok(InterestSnapshot {
        id: row.get(0)?,
        project_id: row.get(1)?,
        source: row.get(2)?,
        source_revision: row.get(3)?,
        window_start: row.get(4)?,
        window_end: row.get(5)?,
        privacy_mode: row.get(6)?,
        coverage: row.get(7)?,
        state: SnapshotState::parse(&state).unwrap_or(SnapshotState::Superseded),
        replaces_source_revision: row.get(9)?,
        actor: row.get(10)?,
        received_at: row.get(11)?,
        metrics: Vec::new(),
    })
}
