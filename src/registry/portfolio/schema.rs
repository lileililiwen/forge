//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

/// Additive portfolio schema. Every statement is
/// `CREATE ... IF NOT EXISTS`, so opening a registry written
/// before the portfolio package migrates it forward in place and
/// re-opening is a no-op. `project_id` references the canonical
/// imported identity in `projects`; SQLite does not enforce
/// foreign keys unless `PRAGMA foreign_keys` is enabled, so the
/// methods on this type check the project themselves and return a
/// typed `unknown-project` instead of relying on the pragma.
pub const PORTFOLIO_SCHEMA_SQL: &str = "
CREATE TABLE IF NOT EXISTS portfolio_projects (
    project_id   TEXT PRIMARY KEY,
    lifecycle    TEXT,
    confidence   TEXT,
    next_action  TEXT,
    blocker      TEXT,
    reviewed_at  TEXT
);
CREATE TABLE IF NOT EXISTS portfolio_tags (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    name       TEXT NOT NULL UNIQUE,
    color      TEXT,
    created_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS portfolio_project_tags (
    project_id TEXT NOT NULL,
    tag_id     INTEGER NOT NULL,
    PRIMARY KEY (project_id, tag_id)
);
CREATE TABLE IF NOT EXISTS portfolio_relations (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    from_project  TEXT NOT NULL,
    to_project    TEXT NOT NULL,
    relation_type TEXT NOT NULL,
    note          TEXT,
    created_at    TEXT NOT NULL,
    UNIQUE (from_project, to_project, relation_type)
);
CREATE TABLE IF NOT EXISTS portfolio_goals (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    title       TEXT NOT NULL UNIQUE,
    status      TEXT NOT NULL DEFAULT 'planned',
    description TEXT,
    created_at  TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS portfolio_goal_projects (
    goal_id    INTEGER NOT NULL,
    project_id TEXT NOT NULL,
    PRIMARY KEY (goal_id, project_id)
);
CREATE TABLE IF NOT EXISTS portfolio_reviews (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id TEXT NOT NULL,
    confidence TEXT NOT NULL,
    note       TEXT,
    reviewed_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS portfolio_evidence_snapshots (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id      TEXT NOT NULL,
    source_system   TEXT NOT NULL,
    source_revision TEXT NOT NULL,
    observed_at     TEXT NOT NULL,
    status          TEXT NOT NULL,
    stale_after     TEXT,
    evidence_json   TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS portfolio_project_tags_tag_idx
    ON portfolio_project_tags (tag_id, project_id);
CREATE INDEX IF NOT EXISTS portfolio_relations_from_idx
    ON portfolio_relations (from_project, created_at);
CREATE INDEX IF NOT EXISTS portfolio_relations_to_idx
    ON portfolio_relations (to_project, created_at);
CREATE INDEX IF NOT EXISTS portfolio_goal_projects_project_idx
    ON portfolio_goal_projects (project_id, goal_id);
CREATE INDEX IF NOT EXISTS portfolio_reviews_project_idx
    ON portfolio_reviews (project_id, id);
CREATE INDEX IF NOT EXISTS portfolio_snapshots_project_idx
    ON portfolio_evidence_snapshots (project_id, source_system, id);
";
