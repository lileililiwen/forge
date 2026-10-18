//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

/// Platform (Forge) version recorded with every registered project.
pub const PLATFORM_VERSION: &str = env!("CARGO_PKG_VERSION");

pub(super) const SCHEMA_SQL: &str = "
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
    kit_id           TEXT,
    kit_version      TEXT,
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
    request_hash    TEXT,
    queue_id        TEXT,
    revision        TEXT,
    build_status    TEXT,
    run_status      TEXT,
    container_identity TEXT
);
CREATE UNIQUE INDEX IF NOT EXISTS operations_idempotency_uniq
    ON operations (kind, idempotency_key)
    WHERE idempotency_key IS NOT NULL;
CREATE INDEX IF NOT EXISTS operations_queue_project_state_idx
    ON operations (queue_id, project_id, state)
    WHERE queue_id IS NOT NULL;
CREATE TABLE IF NOT EXISTS hosted_ci_project_bindings (
    project_id TEXT PRIMARY KEY,
    repository_id TEXT NOT NULL UNIQUE,
    repository TEXT NOT NULL UNIQUE,
    bound_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE TABLE IF NOT EXISTS hosted_ci_evidence (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    repository TEXT NOT NULL,
    repository_id TEXT NOT NULL,
    run_id TEXT NOT NULL,
    run_attempt INTEGER NOT NULL,
    job TEXT NOT NULL,
    workflow TEXT NOT NULL,
    source_revision TEXT NOT NULL,
    aggregate TEXT NOT NULL,
    observed_at TEXT NOT NULL,
    received_at TEXT NOT NULL,
    digest TEXT NOT NULL,
    envelope_json TEXT NOT NULL,
    UNIQUE (project_id, run_id, run_attempt)
);
CREATE INDEX IF NOT EXISTS hosted_ci_evidence_project_received_idx
    ON hosted_ci_evidence (project_id, received_at DESC);
";
